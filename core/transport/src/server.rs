//! QUIC server implementation for Nexus transport layer

use crate::{Result, TransportError, TransportConfig, CertificateManager, Connection, TransportMessage};
use nexus_shared::NodeId;
use quinn::{Endpoint, ServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error, debug};

/// QUIC server for accepting incoming connections
pub struct QuicServer {
    /// Server configuration
    config: TransportConfig,
    
    /// Certificate manager for TLS
    cert_manager: Arc<CertificateManager>,
    
    /// Quinn endpoint
    endpoint: Option<Endpoint>,
    
    /// Active connections
    connections: Arc<RwLock<std::collections::HashMap<NodeId, Arc<Connection>>>>,
    
    /// Server node ID
    node_id: NodeId,
    
    /// Message sender for incoming messages
    message_sender: mpsc::UnboundedSender<(NodeId, TransportMessage)>,
    
    /// Message receiver for incoming messages
    message_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<(NodeId, TransportMessage)>>>>,
    
    /// Shutdown signal
    shutdown_sender: Option<mpsc::Sender<()>>,
}

impl QuicServer {
    /// Create a new QUIC server
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
            shutdown_sender: None,
        })
    }
    
    /// Start the server and begin accepting connections
    pub async fn start(&mut self) -> Result<SocketAddr> {
        info!("Starting QUIC server on {}", self.config.socket_addr());
        
        // Create server configuration
        let server_config = self.cert_manager.server_config()
            .map_err(|e| TransportError::Certificate { 
                message: format!("Failed to create server config: {}", e) 
            })?;
            
        let quinn_config = self.config.to_quinn_server_config(server_config);
        
        // Create endpoint
        let endpoint = Endpoint::server(quinn_config, self.config.socket_addr())
            .map_err(|e| TransportError::Endpoint { 
                message: format!("Failed to create endpoint: {}", e) 
            })?;
            
        let local_addr = endpoint.local_addr()
            .map_err(|e| TransportError::Endpoint { 
                message: format!("Failed to get local address: {}", e) 
            })?;
            
        info!("QUIC server listening on {}", local_addr);
        
        // Start connection handling task
        let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(1);
        self.shutdown_sender = Some(shutdown_sender);
        
        let connections = Arc::clone(&self.connections);
        let message_sender = self.message_sender.clone();
        let node_id = self.node_id;
        
        let endpoint_clone = endpoint.clone();
        
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    conn = endpoint_clone.accept() => {
                        let Some(conn) = conn else { break; };
                        debug!("Received incoming connection");
                        
                        let connections = Arc::clone(&connections);
                        let message_sender = message_sender.clone();
                        
                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_incoming_connection(
                                conn, 
                                connections,
                                message_sender,
                                node_id
                            ).await {
                                error!("Failed to handle incoming connection: {}", e);
                            }
                        });
                    }
                    _ = shutdown_receiver.recv() => {
                        info!("Shutting down connection handler");
                        break;
                    }
                }
            }
        });
        
        self.endpoint = Some(endpoint);
        Ok(local_addr)
    }
    
    /// Handle an incoming connection
    async fn handle_incoming_connection(
        connecting: quinn::Connecting,
        connections: Arc<RwLock<std::collections::HashMap<NodeId, Arc<Connection>>>>,
        message_sender: mpsc::UnboundedSender<(NodeId, TransportMessage)>,
        local_node_id: NodeId,
    ) -> Result<()> {
        let quinn_connection = connecting.await
            .map_err(|e| TransportError::Connection { 
                message: format!("Connection failed: {}", e) 
            })?;
            
        let remote_addr = quinn_connection.remote_address();
        info!("New connection established from {}", remote_addr);
        
        // Create connection wrapper
        let connection = Arc::new(Connection::new(
            quinn_connection,
            local_node_id,
            None, // Will be set after handshake
        ).await?);
        
        // Perform handshake to get remote node ID
        let remote_node_id = connection.handshake().await?;
        connection.set_remote_node_id(remote_node_id).await;
        
        // Store connection
        connections.write().await.insert(remote_node_id, Arc::clone(&connection));
        
        // Handle connection messages
        let conn_message_sender = message_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = connection.handle_messages(conn_message_sender).await {
                error!("Connection message handling failed: {}", e);
            }
            
            // Clean up connection when it closes
            connections.write().await.remove(&remote_node_id);
            info!("Connection closed for node {}", remote_node_id);
        });
        
        Ok(())
    }
    
    /// Stop the server
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping QUIC server");
        
        // Send shutdown signal
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(()).await;
        }
        
        // Close endpoint
        if let Some(endpoint) = self.endpoint.take() {
            endpoint.close(0u32.into(), b"server shutdown");
            endpoint.wait_idle().await;
        }
        
        // Close all connections
        let mut connections = self.connections.write().await;
        for (node_id, connection) in connections.drain() {
            debug!("Closing connection to {}", node_id);
            connection.close().await;
        }
        
        info!("QUIC server stopped");
        Ok(())
    }
    
    /// Send a message to a specific peer
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
    
    /// Get server node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }
    
    /// Get server local address
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.endpoint.as_ref().and_then(|e| e.local_addr().ok())
    }
    
    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.endpoint.is_some()
    }
    
    /// Get connection statistics
    pub async fn connection_stats(&self) -> ConnectionStats {
        let connections = self.connections.read().await;
        let total_connections = connections.len();
        
        let mut total_bytes_sent = 0;
        let mut total_bytes_received = 0;
        let mut total_messages_sent = 0;
        let mut total_messages_received = 0;
        
        for connection in connections.values() {
            let stats = connection.stats().await;
            total_bytes_sent += stats.bytes_sent;
            total_bytes_received += stats.bytes_received;
            total_messages_sent += stats.messages_sent;
            total_messages_received += stats.messages_received;
        }
        
        ConnectionStats {
            total_connections,
            total_bytes_sent,
            total_bytes_received,
            total_messages_sent,
            total_messages_received,
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total_connections: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
}

impl std::fmt::Debug for QuicServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuicServer")
            .field("node_id", &self.node_id)
            .field("is_running", &self.is_running())
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl Drop for QuicServer {
    fn drop(&mut self) {
        if self.endpoint.is_some() {
            warn!("QuicServer dropped without proper shutdown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_server_creation() {
        let cert_manager = Arc::new(
            CertificateManager::new_self_signed(
                "test-server".to_string(),
                365,
                Duration::from_secs(3600),
            ).await.unwrap()
        );
        
        let mut config = TransportConfig::default();
        config.port = 0; // Use any available port
        
        let server = QuicServer::new(config, cert_manager).await;
        assert!(server.is_ok());
        
        let server = server.unwrap();
        assert!(!server.is_running());
        assert_eq!(server.connection_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_server_start_stop() {
        let cert_manager = Arc::new(
            CertificateManager::new_self_signed(
                "test-server".to_string(),
                365,
                Duration::from_secs(3600),
            ).await.unwrap()
        );
        
        let mut config = TransportConfig::default();
        config.port = 0;
        
        let mut server = QuicServer::new(config, cert_manager).await.unwrap();
        
        let addr = server.start().await.unwrap();
        assert!(addr.port() != 0);
        assert!(server.is_running());
        assert_eq!(server.local_addr().unwrap(), addr);
        
        server.stop().await.unwrap();
        assert!(!server.is_running());
    }
}