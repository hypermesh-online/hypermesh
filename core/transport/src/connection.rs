//! Connection management and message handling

use crate::{Result, TransportError, TransportMessage, MessageType};
use nexus_shared::NodeId;
use quinn::{SendStream, RecvStream};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, oneshot, Mutex};
use tracing::{info, warn, error, debug, trace};
use std::collections::HashMap;
use std::time::Duration;

/// Connection wrapper for QUIC connections
pub struct Connection {
    /// Quinn connection
    quinn_connection: quinn::Connection,
    
    /// Local node ID
    local_node_id: NodeId,
    
    /// Remote node ID (set after handshake)
    remote_node_id: Arc<RwLock<Option<NodeId>>>,
    
    /// Connection statistics
    stats: Arc<RwLock<ConnectionStats>>,
    
    /// Pending requests (for request-response pattern)
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<TransportMessage>>>>,
    
    /// Request sequence number
    request_sequence: Arc<std::sync::atomic::AtomicU64>,
    
    /// Message handlers
    message_handlers: Arc<RwLock<Vec<mpsc::UnboundedSender<(NodeId, TransportMessage)>>>>,
}

impl Connection {
    /// Create a new connection wrapper
    pub async fn new(
        connection: quinn::Connection,
        local_node_id: NodeId,
        remote_node_id: Option<NodeId>,
    ) -> Result<Self> {
        Ok(Self {
            quinn_connection: connection,
            local_node_id,
            remote_node_id: Arc::new(RwLock::new(remote_node_id)),
            stats: Arc::new(RwLock::new(ConnectionStats::new())),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            request_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            message_handlers: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Perform handshake to exchange node IDs
    pub async fn handshake(&self) -> Result<NodeId> {
        debug!("Performing handshake");
        
        // Open bidirectional stream for handshake
        let (mut send_stream, mut recv_stream) = self.quinn_connection
            .open_bi()
            .await
            .map_err(|e| TransportError::Stream { 
                message: format!("Failed to open handshake stream: {}", e) 
            })?;
        
        // Send our node ID
        let handshake_message = TransportMessage::new(
            MessageType::Handshake,
            self.local_node_id,
            None,
            self.local_node_id.as_bytes().to_vec(),
        );
        
        let message_bytes = handshake_message.to_bytes()?;
        Self::write_message(&mut send_stream, &message_bytes).await?;
        send_stream.finish().await
            .map_err(|e| TransportError::Stream { 
                message: format!("Failed to finish handshake send: {}", e) 
            })?;
        
        // Receive remote node ID
        let response_bytes = Self::read_message(&mut recv_stream).await?;
        let response_message = TransportMessage::from_bytes(&response_bytes)?;
        
        if response_message.message_type != MessageType::Handshake {
            return Err(TransportError::Authentication { 
                reason: "Invalid handshake response".to_string() 
            });
        }
        
        let remote_node_id = NodeId::new(
            response_message.payload
                .try_into()
                .map_err(|_| TransportError::Authentication { 
                    reason: "Invalid node ID in handshake".to_string() 
                })?
        );
        
        info!("Handshake completed with node {}", remote_node_id);
        Ok(remote_node_id)
    }
    
    /// Set remote node ID
    pub async fn set_remote_node_id(&self, node_id: NodeId) {
        *self.remote_node_id.write().await = Some(node_id);
    }
    
    /// Get remote node ID
    pub async fn remote_node_id(&self) -> Option<NodeId> {
        *self.remote_node_id.read().await
    }
    
    /// Send a message
    pub async fn send_message(&self, message: TransportMessage) -> Result<()> {
        let mut send_stream = self.quinn_connection
            .open_uni()
            .await
            .map_err(|e| TransportError::Stream { 
                message: format!("Failed to open send stream: {}", e) 
            })?;
        
        let message_bytes = message.to_bytes()?;
        Self::write_message(&mut send_stream, &message_bytes).await?;
        
        send_stream.finish().await
            .map_err(|e| TransportError::Stream { 
                message: format!("Failed to finish send stream: {}", e) 
            })?;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.messages_sent += 1;
        stats.bytes_sent += message_bytes.len() as u64;
        
        trace!("Message sent: {} bytes", message_bytes.len());
        Ok(())
    }
    
    /// Send a request and wait for response
    pub async fn send_request(
        &self,
        request: TransportMessage,
        timeout: Duration,
    ) -> Result<TransportMessage> {
        let sequence = self.request_sequence
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        let (response_sender, response_receiver) = oneshot::channel();
        
        // Store pending request
        self.pending_requests.lock().await.insert(sequence, response_sender);
        
        // Modify request to include sequence number
        let mut request_with_seq = request;
        request_with_seq.sequence = sequence;
        
        // Send request
        self.send_message(request_with_seq).await?;
        
        // Wait for response with timeout
        let response = tokio::time::timeout(timeout, response_receiver)
            .await
            .map_err(|_| TransportError::Timeout { 
                duration_ms: timeout.as_millis() as u64 
            })?
            .map_err(|_| TransportError::Connection { 
                message: "Request cancelled".to_string() 
            })?;
        
        Ok(response)
    }
    
    /// Handle incoming messages
    pub async fn handle_messages(
        &self,
        message_sender: mpsc::UnboundedSender<(NodeId, TransportMessage)>,
    ) -> Result<()> {
        // Add message handler
        self.message_handlers.write().await.push(message_sender);
        
        loop {
            // Accept incoming streams
            match self.quinn_connection.accept_uni().await {
                Ok(recv_stream) => {
                    let handlers = Arc::clone(&self.message_handlers);
                    let stats = Arc::clone(&self.stats);
                    let pending_requests = Arc::clone(&self.pending_requests);
                    let remote_node_id = self.remote_node_id().await;
                    
                    tokio::spawn(async move {
                        if let Some(remote_id) = remote_node_id {
                            if let Err(e) = Self::handle_incoming_stream(
                                recv_stream,
                                remote_id,
                                handlers,
                                stats,
                                pending_requests,
                            ).await {
                                error!("Failed to handle incoming stream: {}", e);
                            }
                        }
                    });
                }
                Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                    debug!("Connection closed by application");
                    break;
                }
                Err(e) => {
                    error!("Failed to accept stream: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle an incoming stream
    async fn handle_incoming_stream(
        mut recv_stream: RecvStream,
        remote_node_id: NodeId,
        handlers: Arc<RwLock<Vec<mpsc::UnboundedSender<(NodeId, TransportMessage)>>>>,
        stats: Arc<RwLock<ConnectionStats>>,
        pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<TransportMessage>>>>,
    ) -> Result<()> {
        let message_bytes = Self::read_message(&mut recv_stream).await?;
        let message = TransportMessage::from_bytes(&message_bytes)?;
        
        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.messages_received += 1;
            stats_guard.bytes_received += message_bytes.len() as u64;
        }
        
        // Handle control messages
        if message.message_type == MessageType::Control {
            if message.payload == b"ping" {
                // Send pong response
                // This would require opening a new stream - simplified for now
                debug!("Received ping from {}", remote_node_id);
                return Ok(());
            }
        }
        
        // Check if this is a response to a pending request
        if message.sequence != 0 {
            let mut pending = pending_requests.lock().await;
            if let Some(sender) = pending.remove(&message.sequence) {
                let _ = sender.send(message);
                return Ok(());
            }
        }
        
        // Send to message handlers
        let handlers_guard = handlers.read().await;
        for handler in handlers_guard.iter() {
            if let Err(e) = handler.send((remote_node_id, message.clone())) {
                warn!("Failed to send message to handler: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Write a message to a stream
    async fn write_message(stream: &mut SendStream, message: &[u8]) -> Result<()> {
        // Write message length first
        let len = message.len() as u32;
        stream.write_all(&len.to_be_bytes()).await
            .map_err(|e| TransportError::Stream { 
                message: format!("Failed to write message length: {}", e) 
            })?;
        
        // Write message data
        stream.write_all(message).await
            .map_err(|e| TransportError::Stream { 
                message: format!("Failed to write message data: {}", e) 
            })?;
        
        Ok(())
    }
    
    /// Read a message from a stream
    async fn read_message(stream: &mut RecvStream) -> Result<Vec<u8>> {
        // Read message length first
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await
            .map_err(|e| TransportError::Stream { 
                message: format!("Failed to read message length: {}", e) 
            })?;
        
        let len = u32::from_be_bytes(len_bytes) as usize;
        
        // Validate message length
        if len > crate::MAX_MESSAGE_SIZE {
            return Err(TransportError::Stream { 
                message: format!("Message too large: {} bytes", len) 
            });
        }
        
        // Read message data
        let mut message = vec![0u8; len];
        stream.read_exact(&mut message).await
            .map_err(|e| TransportError::Stream { 
                message: format!("Failed to read message data: {}", e) 
            })?;
        
        Ok(message)
    }
    
    /// Close the connection
    pub async fn close(&self) {
        self.quinn_connection.close(0u32.into(), b"connection closed");
    }
    
    /// Get connection statistics
    pub async fn stats(&self) -> ConnectionStats {
        self.stats.read().await.clone()
    }
    
    /// Get connection info
    pub async fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            local_node_id: self.local_node_id,
            remote_node_id: self.remote_node_id().await,
            remote_address: self.quinn_connection.remote_address(),
            stats: self.stats().await,
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connection_time: std::time::SystemTime,
}

impl ConnectionStats {
    pub fn new() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connection_time: std::time::SystemTime::now(),
        }
    }
}

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub local_node_id: NodeId,
    pub remote_node_id: Option<NodeId>,
    pub remote_address: std::net::SocketAddr,
    pub stats: ConnectionStats,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("local_node_id", &self.local_node_id)
            .field("remote_address", &self.quinn_connection.remote_address())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_connection_stats() {
        let stats = ConnectionStats::new();
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
    }
    
    #[tokio::test]
    async fn test_message_serialization() {
        let message = TransportMessage::new(
            MessageType::Data,
            NodeId::random(),
            None,
            b"test".to_vec(),
        );
        
        let bytes = message.to_bytes().unwrap();
        let deserialized = TransportMessage::from_bytes(&bytes).unwrap();
        
        assert_eq!(message.payload, deserialized.payload);
        assert_eq!(message.source, deserialized.source);
    }
}