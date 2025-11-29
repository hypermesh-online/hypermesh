//! QUIC client implementation for Nexus transport layer

use crate::{Result, TransportError, TransportConfig, CertificateManager, Connection, TransportMessage};
use nexus_shared::NodeId;
use quinn::{Endpoint, ClientConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error, debug};

/// QUIC client for establishing outbound connections
pub struct QuicClient {
    /// Client configuration
    config: TransportConfig,
    
    /// Certificate manager for TLS
    cert_manager: Arc<CertificateManager>,
    
    /// Quinn endpoint
    endpoint: Option<Endpoint>,
    
    /// Active connections
    connections: Arc<RwLock<std::collections::HashMap<NodeId, Arc<Connection>>>>,
    
    /// Client node ID
    node_id: NodeId,
    
    /// Message sender for incoming messages
    message_sender: mpsc::UnboundedSender<(NodeId, TransportMessage)>,
    
    /// Message receiver for incoming messages
    message_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<(NodeId, TransportMessage)>>>>,
}

impl QuicClient {
    /// Create a new QUIC client
    pub async fn new(
        config: TransportConfig,
        cert_manager: Arc<CertificateManager>
    ) -> Result<Self> {
        config.validate().map_err(|e| TransportError::Configuration { message: e })?;
        
        let node_id = NodeId::random();
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        
        Ok(Self {
            config,
            cert_manager,
            endpoint: None,
            connections: Arc::new(RwLock::new(std::collections::HashMap::new())),
            node_id,
            message_sender,
            message_receiver: Arc::new(RwLock::new(Some(message_receiver))),
        })
    }
    
    /// Initialize the client endpoint
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting QUIC client");
        
        // Create client configuration
        let client_config = self.cert_manager.client_config()
            .map_err(|e| TransportError::Certificate { 
                message: format!("Failed to create client config: {}", e) 
            })?;
            
        let quinn_config = self.config.to_quinn_client_config();
        
        // Create endpoint with client configuration
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())
            .map_err(|e| TransportError::Endpoint { 
                message: format!("Failed to create client endpoint: {}", e) 
            })?;
            
        endpoint.set_default_client_config(quinn_config);
        
        info!("QUIC client started");
        self.endpoint = Some(endpoint);
        Ok(())
    }
    
    /// Connect to a remote server
    pub async fn connect(&self, remote_addr: SocketAddr, server_name: &str) -> Result<NodeId> {
        let endpoint = self.endpoint.as_ref()
            .ok_or_else(|| TransportError::Endpoint { 
                message: "Client not started".to_string() 
            })?;
            
        info!("Connecting to {} ({})", remote_addr, server_name);
        
        // Establish QUIC connection
        let new_conn = endpoint
            .connect(remote_addr, server_name)
            .map_err(|e| TransportError::Connection { 
                message: format!("Failed to initiate connection: {}", e) 
            })?
            .await
            .map_err(|e| TransportError::Connection { 
                message: format!("Connection failed: {}", e) 
            })?;
            
        info!("QUIC connection established to {}", remote_addr);
        
        // Create connection wrapper
        let connection = Arc::new(Connection::new(
            new_conn,
            self.node_id,
            None, // Will be set after handshake
        ).await?);
        
        // Perform handshake to get remote node ID
        let remote_node_id = connection.handshake().await?;
        connection.set_remote_node_id(remote_node_id).await;
        
        // Store connection
        self.connections.write().await.insert(remote_node_id, Arc::clone(&connection));
        
        // Handle connection messages
        let connections = Arc::clone(&self.connections);
        let message_sender = self.message_sender.clone();
        
        tokio::spawn(async move {
            if let Err(e) = connection.handle_messages(message_sender).await {
                error!("Connection message handling failed: {}", e);
            }
            
            // Clean up connection when it closes
            connections.write().await.remove(&remote_node_id);
            info!("Connection closed for node {}", remote_node_id);
        });
        
        info!("Successfully connected to node {}", remote_node_id);
        Ok(remote_node_id)
    }
    
    /// Connect with retry logic
    pub async fn connect_with_retry(
        &self,
        remote_addr: SocketAddr,
        server_name: &str,
        max_retries: usize,
        retry_delay: std::time::Duration,
    ) -> Result<NodeId> {
        let mut attempts = 0;
        let mut last_error = None;
        
        while attempts < max_retries {
            match self.connect(remote_addr, server_name).await {
                Ok(node_id) => return Ok(node_id),
                Err(e) => {
                    attempts += 1;
                    last_error = Some(e);
                    
                    if attempts < max_retries {
                        warn!(
                            "Connection attempt {} failed, retrying in {:?}: {}",
                            attempts, retry_delay, last_error.as_ref().unwrap()
                        );
                        tokio::time::sleep(retry_delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| TransportError::Connection {
            message: "All connection attempts failed".to_string(),
        }))
    }
    
    /// Disconnect from a specific peer
    pub async fn disconnect(&self, node_id: NodeId) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(connection) = connections.remove(&node_id) {
            info!("Disconnecting from node {}", node_id);
            connection.close().await;
        }
        
        Ok(())
    }
    
    /// Send a message to a connected peer
    pub async fn send_message(
        &self,
        target: NodeId,
        message: TransportMessage,
    ) -> Result<()> {
        let connections = self.connections.read().await;
        let connection = connections.get(&target)
            .ok_or_else(|| TransportError::Connection { 
                message: format!("No connection to node {}", target) 
            })?;
            
        connection.send_message(message).await
    }
    
    /// Send request and wait for response
    pub async fn send_request(
        &self,
        target: NodeId,
        request: TransportMessage,
        timeout: std::time::Duration,
    ) -> Result<TransportMessage> {
        let connections = self.connections.read().await;
        let connection = connections.get(&target)
            .ok_or_else(|| TransportError::Connection { 
                message: format!("No connection to node {}", target) 
            })?;
            
        connection.send_request(request, timeout).await
    }
    
    /// Broadcast a message to all connected peers
    pub async fn broadcast_message(&self, message: TransportMessage) -> Result<usize> {
        let connections = self.connections.read().await;
        let mut sent_count = 0;
        
        for (node_id, connection) in connections.iter() {
            match connection.send_message(message.clone()).await {
                Ok(()) => {
                    sent_count += 1;
                    debug!("Broadcast message sent to {}", node_id);
                }
                Err(e) => {
                    warn!("Failed to send broadcast message to {}: {}", node_id, e);
                }
            }
        }
        
        info!("Broadcast message sent to {} peers", sent_count);
        Ok(sent_count)
    }
    
    /// Stop the client and close all connections
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping QUIC client");
        
        // Close all connections
        let mut connections = self.connections.write().await;
        for (node_id, connection) in connections.drain() {
            debug!("Closing connection to {}", node_id);
            connection.close().await;
        }
        
        // Close endpoint
        if let Some(endpoint) = self.endpoint.take() {
            endpoint.close(0u32.into(), b"client shutdown");
            endpoint.wait_idle().await;
        }
        
        info!("QUIC client stopped");
        Ok(())
    }
    
    /// Get message receiver for incoming messages
    pub async fn take_message_receiver(
        &self
    ) -> Option<mpsc::UnboundedReceiver<(NodeId, TransportMessage)>> {
        self.message_receiver.write().await.take()
    }
    
    /// Get list of connected peers
    pub async fn connected_peers(&self) -> Vec<NodeId> {
        self.connections.read().await.keys().cloned().collect()
    }
    
    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
    
    /// Check if connected to a specific peer
    pub async fn is_connected(&self, node_id: NodeId) -> bool {
        self.connections.read().await.contains_key(&node_id)
    }
    
    /// Get client node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }
    
    /// Check if client is started
    pub fn is_started(&self) -> bool {
        self.endpoint.is_some()
    }
    
    /// Get connection to a specific peer
    pub async fn get_connection(&self, node_id: NodeId) -> Option<Arc<Connection>> {
        self.connections.read().await.get(&node_id).cloned()
    }
    
    /// Ping a connected peer
    pub async fn ping(&self, target: NodeId) -> Result<std::time::Duration> {
        let start = std::time::Instant::now();
        
        let ping_message = TransportMessage::new(
            crate::MessageType::Control,
            self.node_id,
            Some(target),
            b"ping".to_vec(),
        );
        
        let response = self.send_request(
            target,
            ping_message,
            std::time::Duration::from_secs(5),
        ).await?;
        
        // Verify it's a pong response
        if response.payload != b"pong" {
            return Err(TransportError::Connection {
                message: "Invalid ping response".to_string(),
            });
        }
        
        Ok(start.elapsed())
    }
}

impl std::fmt::Debug for QuicClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuicClient")
            .field("node_id", &self.node_id)
            .field("is_started", &self.is_started())
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl Drop for QuicClient {
    fn drop(&mut self) {
        if self.endpoint.is_some() {
            warn!("QuicClient dropped without proper shutdown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_client_creation() {
        let cert_manager = Arc::new(
            CertificateManager::new_self_signed(
                "test-client".to_string(),
                365,
                Duration::from_secs(3600),
            ).await.unwrap()
        );
        
        let config = TransportConfig::default();
        let client = QuicClient::new(config, cert_manager).await;
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert!(!client.is_started());
        assert_eq!(client.connection_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_client_start_stop() {
        let cert_manager = Arc::new(
            CertificateManager::new_self_signed(
                "test-client".to_string(),
                365,
                Duration::from_secs(3600),
            ).await.unwrap()
        );
        
        let config = TransportConfig::default();
        let mut client = QuicClient::new(config, cert_manager).await.unwrap();
        
        client.start().await.unwrap();
        assert!(client.is_started());
        
        client.stop().await.unwrap();
        assert!(!client.is_started());
    }
    
    #[tokio::test]
    async fn test_connection_management() {
        let cert_manager = Arc::new(
            CertificateManager::new_self_signed(
                "test-client".to_string(),
                365,
                Duration::from_secs(3600),
            ).await.unwrap()
        );
        
        let config = TransportConfig::default();
        let mut client = QuicClient::new(config, cert_manager).await.unwrap();
        client.start().await.unwrap();
        
        let node_id = NodeId::random();
        assert!(!client.is_connected(node_id).await);
        
        // Test would require actual server to connect to
        // This demonstrates the API structure
        
        client.stop().await.unwrap();
    }
}