//! STOQ Protocol Message Layer
//!
//! This module provides protocol-level message handling on top of the STOQ transport layer.
//! It bridges the gap between raw QUIC streams and structured application messages.

use std::sync::Arc;
use std::marker::PhantomData;
use std::collections::HashMap;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut, BufMut};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use anyhow::{Result, anyhow};
use tracing::{debug, warn, error, info};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{timeout, Duration};

use crate::transport::{StoqTransport, Connection, Stream};
use crate::transport::certificates::CertificateManager;

/// Protocol message header for STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    /// Message type identifier
    pub message_type: String,
    /// Message version
    pub version: u8,
    /// Message ID for request/response correlation
    pub message_id: u64,
    /// Total message length in bytes
    pub content_length: u32,
    /// Optional authentication token
    pub auth_token: Option<String>,
    /// Timestamp when message was created
    pub timestamp: u64,
    /// Optional compression type
    pub compression: Option<CompressionType>,
}

/// Compression types supported by STOQ protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Lz4,
}

/// Generic STOQ protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqMessage<T> {
    /// Message header
    pub header: MessageHeader,
    /// Message payload
    pub payload: T,
}

/// Message handler trait for processing incoming messages
#[async_trait]
pub trait MessageHandler<T>: Send + Sync {
    /// Handle incoming message and optionally return a response
    async fn handle_message(&self, message: StoqMessage<T>, connection_info: &ConnectionInfo) -> Result<Option<Bytes>>;
}

/// Connection information for message handlers
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Connection ID
    pub connection_id: String,
    /// Remote endpoint address
    pub remote_address: std::net::SocketAddr,
    /// Certificate fingerprint (if available)
    pub cert_fingerprint: Option<String>,
    /// Connection established time
    pub established_at: std::time::SystemTime,
}

/// STOQ protocol handler that manages message routing and processing
pub struct StoqProtocolHandler {
    /// Message handlers by message type
    handlers: Arc<RwLock<HashMap<String, Box<dyn MessageHandler<Bytes> + Send + Sync>>>>,
    /// Active message processing tasks
    active_streams: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// Message ID counter for outgoing messages
    message_id_counter: Arc<std::sync::atomic::AtomicU64>,
    /// Protocol configuration
    config: ProtocolConfig,
    /// Certificate manager for authentication
    cert_manager: Option<Arc<CertificateManager>>,
}

/// Protocol configuration
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Message processing timeout
    pub message_timeout: Duration,
    /// Enable message compression
    pub enable_compression: bool,
    /// Compression threshold (messages larger than this will be compressed)
    pub compression_threshold: usize,
    /// Enable authentication
    pub enable_authentication: bool,
    /// Maximum concurrent streams per connection
    pub max_concurrent_streams: usize,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            max_message_size: 16 * 1024 * 1024, // 16MB
            message_timeout: Duration::from_secs(30),
            enable_compression: true,
            compression_threshold: 1024, // 1KB
            enable_authentication: true,
            max_concurrent_streams: 100,
        }
    }
}

impl StoqProtocolHandler {
    /// Create a new protocol handler
    pub fn new(config: ProtocolConfig, cert_manager: Option<Arc<CertificateManager>>) -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            active_streams: Arc::new(Mutex::new(HashMap::new())),
            message_id_counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            config,
            cert_manager,
        }
    }

    /// Register a message handler for a specific message type
    pub async fn register_handler<T>(&self, message_type: String, handler: impl MessageHandler<T> + 'static) 
    where 
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let wrapped_handler = TypedMessageHandlerWrapper::new(handler);
        self.handlers.write().await.insert(message_type, Box::new(wrapped_handler));
    }

    /// Handle incoming connection and process messages
    pub async fn handle_connection(&self, connection: Arc<Connection>, transport: Arc<StoqTransport>) -> Result<()> {
        let connection_info = ConnectionInfo {
            connection_id: connection.id(),
            remote_address: connection.endpoint().to_socket_addr(),
            cert_fingerprint: self.extract_cert_fingerprint(&connection).await,
            established_at: std::time::SystemTime::now(),
        };

        info!("Handling STOQ protocol connection: {}", connection_info.connection_id);

        // Accept and process streams concurrently
        loop {
            match connection.accept_stream().await {
                Ok(stream) => {
                    let handler = self.clone();
                    let conn_info = connection_info.clone();
                    let stream_id = format!("{}::{}", connection_info.connection_id, uuid::Uuid::new_v4());
                    
                    // Check concurrent stream limit
                    {
                        let active_streams = self.active_streams.lock().await;
                        if active_streams.len() >= self.config.max_concurrent_streams {
                            warn!("Maximum concurrent streams reached, dropping new stream");
                            continue;
                        }
                    }
                    
                    let task = tokio::spawn(async move {
                        if let Err(e) = handler.handle_stream(stream, conn_info).await {
                            error!("Error handling stream: {}", e);
                        }
                    });
                    
                    self.active_streams.lock().await.insert(stream_id.clone(), task);
                    
                    // Clean up completed tasks periodically
                    self.cleanup_completed_tasks().await;
                }
                Err(e) => {
                    if connection.is_active() {
                        warn!("Error accepting stream on active connection: {}", e);
                    } else {
                        debug!("Connection closed, stopping stream acceptance");
                        break;
                    }
                }
            }
        }

        info!("Connection handler finished: {}", connection_info.connection_id);
        Ok(())
    }

    /// Handle individual stream messages
    async fn handle_stream(&self, mut stream: Stream, connection_info: ConnectionInfo) -> Result<()> {
        debug!("Processing stream for connection: {}", connection_info.connection_id);

        // Receive message with timeout
        let message_data = match timeout(self.config.message_timeout, stream.receive()).await {
            Ok(Ok(data)) => data,
            Ok(Err(e)) => return Err(anyhow!("Stream receive error: {}", e)),
            Err(_) => return Err(anyhow!("Message receive timeout")),
        };

        // Validate message size
        if message_data.len() > self.config.max_message_size {
            return Err(anyhow!("Message size {} exceeds limit {}", message_data.len(), self.config.max_message_size));
        }

        // Parse message header
        let header = self.parse_message_header(&message_data)?;
        debug!("Received message type: {} (id: {})", header.message_type, header.message_id);

        // Extract payload
        let payload = self.extract_payload(&message_data, &header)?;

        // Validate authentication if enabled
        if self.config.enable_authentication {
            self.validate_authentication(&header, &connection_info)?;
        }

        // Route message to appropriate handler
        let response = self.route_message(header.message_type.clone(), payload, &connection_info).await?;

        // Send response if handler provided one
        if let Some(response_data) = response {
            match timeout(self.config.message_timeout, stream.send(&response_data)).await {
                Ok(Ok(_)) => debug!("Response sent for message: {}", header.message_id),
                Ok(Err(e)) => warn!("Failed to send response: {}", e),
                Err(_) => warn!("Response send timeout for message: {}", header.message_id),
            }
        }

        Ok(())
    }

    /// Route message to appropriate handler
    async fn route_message(&self, message_type: String, payload: Bytes, connection_info: &ConnectionInfo) -> Result<Option<Bytes>> {
        let handlers = self.handlers.read().await;
        
        if let Some(handler) = handlers.get(&message_type) {
            let message = StoqMessage {
                header: MessageHeader {
                    message_type,
                    version: 1,
                    message_id: self.next_message_id(),
                    content_length: payload.len() as u32,
                    auth_token: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    compression: None,
                },
                payload,
            };
            
            handler.handle_message(message, connection_info).await
        } else {
            warn!("No handler registered for message type: {}", message_type);
            Err(anyhow!("Unknown message type: {}", message_type))
        }
    }

    /// Send message over connection
    pub async fn send_message<T>(&self, connection: &Connection, message_type: String, payload: T) -> Result<()> 
    where 
        T: Serialize,
    {
        let serialized_payload = bincode::serialize(&payload)?;
        let compressed_payload = self.compress_if_needed(&serialized_payload)?;
        
        let header = MessageHeader {
            message_type,
            version: 1,
            message_id: self.next_message_id(),
            content_length: compressed_payload.len() as u32,
            auth_token: self.generate_auth_token().await?,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            compression: if compressed_payload.len() < serialized_payload.len() {
                Some(CompressionType::Lz4)
            } else {
                Some(CompressionType::None)
            },
        };

        let message_data = self.encode_message(&header, &compressed_payload)?;
        
        let mut stream = connection.open_stream().await?;
        stream.send(&message_data).await?;
        
        debug!("Sent message type: {} (id: {})", header.message_type, header.message_id);
        Ok(())
    }

    /// Parse message header from received data
    fn parse_message_header(&self, data: &Bytes) -> Result<MessageHeader> {
        if data.len() < 8 {
            return Err(anyhow!("Message too short for header"));
        }

        // First 4 bytes contain header length
        let header_length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        
        if data.len() < 4 + header_length {
            return Err(anyhow!("Incomplete message header"));
        }

        let header_data = &data[4..4 + header_length];
        let header: MessageHeader = bincode::deserialize(header_data)?;
        
        Ok(header)
    }

    /// Extract payload from message data
    fn extract_payload(&self, data: &Bytes, header: &MessageHeader) -> Result<Bytes> {
        let header_data = bincode::serialize(header)?;
        let header_length = header_data.len();
        let payload_start = 4 + header_length; // 4 bytes for header length + header
        
        if data.len() < payload_start + header.content_length as usize {
            return Err(anyhow!("Incomplete message payload"));
        }

        let compressed_payload = data.slice(payload_start..payload_start + header.content_length as usize);
        
        // Decompress if needed
        match header.compression {
            Some(CompressionType::Lz4) => self.decompress_lz4(&compressed_payload),
            Some(CompressionType::Gzip) => self.decompress_gzip(&compressed_payload),
            _ => Ok(compressed_payload),
        }
    }

    /// Encode message with header and payload
    fn encode_message(&self, header: &MessageHeader, payload: &[u8]) -> Result<Bytes> {
        let header_data = bincode::serialize(header)?;
        let header_length = header_data.len() as u32;
        
        let mut message_data = BytesMut::with_capacity(4 + header_data.len() + payload.len());
        message_data.put_u32(header_length);
        message_data.put_slice(&header_data);
        message_data.put_slice(payload);
        
        Ok(message_data.freeze())
    }

    /// Compress data if it exceeds threshold
    fn compress_if_needed(&self, data: &[u8]) -> Result<Vec<u8>> {
        if self.config.enable_compression && data.len() > self.config.compression_threshold {
            self.compress_lz4(data)
        } else {
            Ok(data.to_vec())
        }
    }

    /// Compress data using LZ4
    fn compress_lz4(&self, data: &[u8]) -> Result<Vec<u8>> {
        // For now, return original data. In production, use lz4_flex or similar
        // let compressed = lz4_flex::compress_prepend_size(data);
        Ok(data.to_vec())
    }

    /// Decompress LZ4 data
    fn decompress_lz4(&self, data: &[u8]) -> Result<Bytes> {
        // For now, return original data. In production, use lz4_flex or similar
        // let decompressed = lz4_flex::decompress_size_prepended(data)?;
        Ok(Bytes::copy_from_slice(data))
    }

    /// Decompress Gzip data
    fn decompress_gzip(&self, data: &[u8]) -> Result<Bytes> {
        // Placeholder for gzip decompression
        Ok(Bytes::copy_from_slice(data))
    }

    /// Validate authentication token
    fn validate_authentication(&self, header: &MessageHeader, connection_info: &ConnectionInfo) -> Result<()> {
        if self.config.enable_authentication {
            if header.auth_token.is_none() {
                return Err(anyhow!("Authentication required but no token provided"));
            }
            
            // In production, validate the token against the certificate or other auth mechanism
            if let Some(cert_fingerprint) = &connection_info.cert_fingerprint {
                debug!("Validating auth token for cert: {}", cert_fingerprint);
                // Token validation logic would go here
            }
        }
        Ok(())
    }

    /// Generate authentication token
    async fn generate_auth_token(&self) -> Result<Option<String>> {
        if self.config.enable_authentication {
            // In production, generate proper authentication token
            Ok(Some(format!("token_{}", self.next_message_id())))
        } else {
            Ok(None)
        }
    }

    /// Extract certificate fingerprint from connection
    async fn extract_cert_fingerprint(&self, connection: &Connection) -> Option<String> {
        // In production, extract the actual certificate fingerprint from the QUIC connection
        // For now, generate a placeholder based on connection ID
        Some(format!("cert_{}", connection.id()))
    }

    /// Get next message ID
    fn next_message_id(&self) -> u64 {
        self.message_id_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Clean up completed stream handling tasks
    async fn cleanup_completed_tasks(&self) {
        let mut active_streams = self.active_streams.lock().await;
        let mut completed_tasks = Vec::new();
        
        for (stream_id, task) in active_streams.iter() {
            if task.is_finished() {
                completed_tasks.push(stream_id.clone());
            }
        }
        
        for stream_id in completed_tasks {
            if let Some(task) = active_streams.remove(&stream_id) {
                let _ = task.await; // Ensure task is properly cleaned up
            }
        }
    }
}

/// Wrapper to adapt typed message handlers to the generic interface
struct TypedMessageHandlerWrapper<T> {
    handler: Box<dyn MessageHandler<T> + Send + Sync>,
    _phantom: PhantomData<T>,
}

impl<T> TypedMessageHandlerWrapper<T> 
where 
    T: DeserializeOwned + Send + Sync + 'static,
{
    fn new(handler: impl MessageHandler<T> + 'static) -> Self {
        Self {
            handler: Box::new(handler),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<T> MessageHandler<Bytes> for TypedMessageHandlerWrapper<T>
where 
    T: DeserializeOwned + Send + Sync + 'static,
{
    async fn handle_message(&self, message: StoqMessage<Bytes>, connection_info: &ConnectionInfo) -> Result<Option<Bytes>> {
        // Deserialize the payload to the expected type
        let typed_payload: T = bincode::deserialize(&message.payload)?;
        
        let typed_message = StoqMessage {
            header: message.header,
            payload: typed_payload,
        };
        
        self.handler.handle_message(typed_message, connection_info).await
    }
}

impl Clone for StoqProtocolHandler {
    fn clone(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
            active_streams: self.active_streams.clone(),
            message_id_counter: self.message_id_counter.clone(),
            config: self.config.clone(),
            cert_manager: self.cert_manager.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    struct TestMessageHandler;

    #[async_trait]
    impl MessageHandler<Value> for TestMessageHandler {
        async fn handle_message(&self, message: StoqMessage<Value>, _connection_info: &ConnectionInfo) -> Result<Option<Bytes>> {
            println!("Received test message: {:?}", message.payload);
            Ok(Some(Bytes::from("test response")))
        }
    }

    #[tokio::test]
    async fn test_protocol_handler_creation() {
        let config = ProtocolConfig::default();
        let handler = StoqProtocolHandler::new(config, None);
        assert_eq!(handler.next_message_id(), 1);
    }

    #[tokio::test]
    async fn test_message_encoding_decoding() {
        let handler = StoqProtocolHandler::new(ProtocolConfig::default(), None);
        
        let header = MessageHeader {
            message_type: "test".to_string(),
            version: 1,
            message_id: 123,
            content_length: 4,
            auth_token: None,
            timestamp: 1234567890,
            compression: Some(CompressionType::None),
        };
        
        let payload = b"test";
        let encoded = handler.encode_message(&header, payload).unwrap();
        let parsed_header = handler.parse_message_header(&encoded).unwrap();
        
        assert_eq!(parsed_header.message_type, "test");
        assert_eq!(parsed_header.message_id, 123);
    }

    #[tokio::test]
    async fn test_handler_registration() {
        let handler = StoqProtocolHandler::new(ProtocolConfig::default(), None);
        handler.register_handler("test".to_string(), TestMessageHandler).await;
        
        let handlers = handler.handlers.read().await;
        assert!(handlers.contains_key("test"));
    }
}