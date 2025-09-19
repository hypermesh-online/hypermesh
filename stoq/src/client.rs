//! STOQ Protocol Client with Message Support
//!
//! This module provides a high-level client interface that integrates
//! the STOQ transport layer with structured message sending and receiving.

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;
use bytes::Bytes;
use serde::{Serialize, de::DeserializeOwned};
use tracing::{info, debug, warn};
use tokio::sync::{RwLock, Mutex};

use crate::transport::{StoqTransport, Connection, Endpoint, TransportConfig};
use crate::protocol::{StoqProtocolHandler, StoqMessage, ProtocolConfig, MessageHandler, ConnectionInfo};

/// High-level STOQ client with integrated message support
pub struct StoqClient {
    /// Underlying transport layer
    transport: Arc<StoqTransport>,
    /// Protocol message handler
    protocol_handler: Arc<StoqProtocolHandler>,
    /// Active connections to different endpoints
    connections: Arc<RwLock<HashMap<String, Arc<Connection>>>>,
    /// Client configuration
    config: StoqClientConfig,
}

/// STOQ client configuration
#[derive(Debug, Clone)]
pub struct StoqClientConfig {
    /// Transport layer configuration
    pub transport: TransportConfig,
    /// Protocol layer configuration
    pub protocol: ProtocolConfig,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Default message timeout
    pub message_timeout: Duration,
    /// Enable connection pooling
    pub enable_connection_pooling: bool,
    /// Maximum connections to maintain
    pub max_connections: usize,
}

impl Default for StoqClientConfig {
    fn default() -> Self {
        let mut transport_config = TransportConfig::default();
        transport_config.port = 0; // Use ephemeral port for client
        
        Self {
            transport: transport_config,
            protocol: ProtocolConfig::default(),
            connection_timeout: Duration::from_secs(10),
            message_timeout: Duration::from_secs(30),
            enable_connection_pooling: true,
            max_connections: 100,
        }
    }
}

impl StoqClient {
    /// Create a new STOQ client
    pub async fn new(config: StoqClientConfig) -> Result<Self> {
        info!("Initializing STOQ client");

        // Initialize transport layer
        let transport = Arc::new(StoqTransport::new(config.transport.clone()).await?);
        
        // Initialize protocol handler
        let cert_manager = transport.cert_manager.clone();
        let protocol_handler = Arc::new(StoqProtocolHandler::new(
            config.protocol.clone(),
            Some(cert_manager),
        ));

        let client = Self {
            transport,
            protocol_handler,
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
        };

        info!("STOQ client initialized successfully");
        Ok(client)
    }

    /// Connect to a remote endpoint
    pub async fn connect(&self, endpoint: &Endpoint) -> Result<Arc<Connection>> {
        let endpoint_key = format!("{}:{}", endpoint.address, endpoint.port);
        
        // Check if we have an existing connection
        if self.config.enable_connection_pooling {
            let connections = self.connections.read().await;
            if let Some(existing_conn) = connections.get(&endpoint_key) {
                if existing_conn.is_active() {
                    debug!("Reusing existing connection to {}", endpoint_key);
                    return Ok(existing_conn.clone());
                }
            }
        }

        debug!("Creating new connection to {}", endpoint_key);
        let connection = self.transport.connect(endpoint).await?;
        
        // Store connection if pooling is enabled
        if self.config.enable_connection_pooling {
            let mut connections = self.connections.write().await;
            if connections.len() >= self.config.max_connections {
                // Remove oldest connection (simple eviction)
                if let Some((old_key, old_conn)) = connections.iter().next() {
                    let old_key = old_key.clone();
                    old_conn.close();
                    connections.remove(&old_key);
                }
            }
            connections.insert(endpoint_key.clone(), connection.clone());
        }

        info!("Connected to {}", endpoint_key);
        Ok(connection)
    }

    /// Send a message to an endpoint
    pub async fn send_message<T>(&self, endpoint: &Endpoint, message_type: String, payload: T) -> Result<()>
    where
        T: Serialize,
    {
        let connection = self.connect(endpoint).await?;
        self.protocol_handler.send_message(&connection, message_type, payload).await?;
        Ok(())
    }

    /// Send a message and wait for a response
    pub async fn send_message_with_response<T, R>(&self, endpoint: &Endpoint, message_type: String, payload: T) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let connection = self.connect(endpoint).await?;
        
        // Send the message
        self.protocol_handler.send_message(&connection, message_type, payload).await?;
        
        // Wait for response on the same connection
        let mut stream = connection.accept_stream().await?;
        let response_data = tokio::time::timeout(
            self.config.message_timeout,
            stream.receive()
        ).await??;
        
        let response: R = bincode::deserialize(&response_data)?;
        Ok(response)
    }

    /// Send raw data over a connection
    pub async fn send_raw_data(&self, endpoint: &Endpoint, data: &[u8]) -> Result<()> {
        let connection = self.connect(endpoint).await?;
        self.transport.send(&connection, data).await?;
        Ok(())
    }

    /// Receive raw data from a connection
    pub async fn receive_raw_data(&self, endpoint: &Endpoint) -> Result<Bytes> {
        let connection = self.connect(endpoint).await?;
        self.transport.receive(&connection).await
    }

    /// Register a message handler for receiving messages (useful for bidirectional communication)
    pub async fn register_handler<T>(&self, message_type: String, handler: impl MessageHandler<T> + 'static)
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        info!("Registering client handler for message type: {}", message_type);
        self.protocol_handler.register_handler(message_type, handler).await;
    }

    /// Start listening for incoming messages on existing connections
    pub async fn start_listening(&self) -> Result<()> {
        info!("Starting client message listening...");
        
        // This would typically be used for bidirectional communication
        // where the client also acts as a server for certain message types
        
        // For each active connection, start protocol handling
        let connections = self.connections.read().await;
        for (endpoint_key, connection) in connections.iter() {
            if connection.is_active() {
                let handler = self.protocol_handler.clone();
                let conn_clone = connection.clone();
                let transport_clone = self.transport.clone();
                let endpoint_key_clone = endpoint_key.clone();
                
                tokio::spawn(async move {
                    debug!("Starting protocol handling for connection: {}", endpoint_key_clone);
                    if let Err(e) = handler.handle_connection(conn_clone, transport_clone).await {
                        warn!("Protocol handler error for {}: {}", endpoint_key_clone, e);
                    }
                });
            }
        }
        
        Ok(())
    }

    /// Close a specific connection
    pub async fn close_connection(&self, endpoint: &Endpoint) -> Result<()> {
        let endpoint_key = format!("{}:{}", endpoint.address, endpoint.port);
        let mut connections = self.connections.write().await;
        
        if let Some(connection) = connections.remove(&endpoint_key) {
            connection.close();
            info!("Closed connection to {}", endpoint_key);
        }
        
        Ok(())
    }

    /// Close all connections and shutdown the client
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down STOQ client...");
        
        // Close all connections
        let mut connections = self.connections.write().await;
        for (endpoint_key, connection) in connections.drain() {
            connection.close();
            debug!("Closed connection to {}", endpoint_key);
        }
        
        // Shutdown transport
        self.transport.shutdown().await;
        
        info!("STOQ client shutdown complete");
        Ok(())
    }

    /// Get client statistics
    pub fn stats(&self) -> crate::TransportStats {
        self.transport.stats()
    }

    /// Get active connection count
    pub async fn active_connections(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Get the protocol handler for direct access
    pub fn protocol_handler(&self) -> Arc<StoqProtocolHandler> {
        self.protocol_handler.clone()
    }

    /// Get the transport layer for direct access
    pub fn transport(&self) -> Arc<StoqTransport> {
        self.transport.clone()
    }
}

/// Convenience methods for common message patterns
impl StoqClient {
    /// Send a string message
    pub async fn send_string(&self, endpoint: &Endpoint, message: String) -> Result<()> {
        self.send_message(endpoint, "string".to_string(), message).await
    }

    /// Send a JSON message
    pub async fn send_json<T>(&self, endpoint: &Endpoint, payload: T) -> Result<()>
    where
        T: Serialize,
    {
        let json_value = serde_json::to_value(payload)?;
        self.send_message(endpoint, "json".to_string(), json_value).await
    }

    /// Send a binary message
    pub async fn send_binary(&self, endpoint: &Endpoint, data: Vec<u8>) -> Result<()> {
        self.send_message(endpoint, "binary".to_string(), data).await
    }

    /// Send string and wait for string response
    pub async fn echo_string(&self, endpoint: &Endpoint, message: String) -> Result<String> {
        self.send_message_with_response(endpoint, "echo".to_string(), message).await
    }

    /// Send JSON and wait for JSON response
    pub async fn request_json<T, R>(&self, endpoint: &Endpoint, request: T) -> Result<R>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let json_request = serde_json::to_value(request)?;
        let json_response: serde_json::Value = self.send_message_with_response(
            endpoint, 
            "json".to_string(), 
            json_request
        ).await?;
        
        let response: R = serde_json::from_value(json_response)?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[tokio::test]
    async fn test_client_creation() {
        // Initialize crypto provider
        if rustls::crypto::ring::default_provider().install_default().is_err() {
            // Already installed, ignore error
        }

        let config = StoqClientConfig::default();
        let client = StoqClient::new(config).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_client_connection_pooling() {
        // Initialize crypto provider
        if rustls::crypto::ring::default_provider().install_default().is_err() {
            // Already installed, ignore error
        }

        let config = StoqClientConfig {
            enable_connection_pooling: true,
            max_connections: 5,
            ..Default::default()
        };

        if let Ok(client) = StoqClient::new(config).await {
            assert_eq!(client.active_connections().await, 0);
            
            // Note: Actual connection tests would require a running server
            // In integration tests, we would set up a test server
        }
    }
}