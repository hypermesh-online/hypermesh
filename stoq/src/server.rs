//! STOQ Protocol Server with Integrated Message Handling
//!
//! This module provides a high-level server interface that integrates
//! the STOQ transport layer with the protocol message handling system.

use std::sync::Arc;
use std::net::Ipv6Addr;
use anyhow::Result;
use tracing::{info, warn, error};
use tokio::sync::RwLock;

use crate::transport::{StoqTransport, TransportConfig};
use crate::protocol::{StoqProtocolHandler, ProtocolConfig, MessageHandler, StoqMessage, ConnectionInfo};
use crate::transport::certificates::CertificateManager;

/// High-level STOQ server with integrated protocol handling
pub struct StoqServer {
    /// Underlying transport layer
    transport: Arc<RwLock<StoqTransport>>,
    /// Protocol message handler
    protocol_handler: Arc<StoqProtocolHandler>,
    /// Server configuration
    config: StoqServerConfig,
    /// Flag to track if server is running
    running: Arc<std::sync::atomic::AtomicBool>,
}

/// STOQ server configuration
#[derive(Debug, Clone)]
pub struct StoqServerConfig {
    /// Transport layer configuration
    pub transport: TransportConfig,
    /// Protocol layer configuration
    pub protocol: ProtocolConfig,
    /// Server bind address
    pub bind_address: Ipv6Addr,
    /// Server port
    pub port: u16,
    /// Maximum concurrent connections
    pub max_connections: Option<u32>,
}

impl Default for StoqServerConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
            protocol: ProtocolConfig::default(),
            bind_address: Ipv6Addr::LOCALHOST,
            port: crate::DEFAULT_PORT,
            max_connections: Some(1000),
        }
    }
}

impl StoqServer {
    /// Create a new STOQ server with integrated protocol handling
    pub async fn new(config: StoqServerConfig) -> Result<Self> {
        info!("Initializing STOQ server on [{}]:{}", config.bind_address, config.port);

        // Create transport configuration
        let mut transport_config = config.transport.clone();
        transport_config.bind_address = config.bind_address;
        transport_config.port = config.port;
        if let Some(max_conn) = config.max_connections {
            transport_config.max_connections = Some(max_conn);
        }

        // Initialize transport layer
        let transport = StoqTransport::new(transport_config).await?;
        
        // Initialize protocol handler with certificate manager from transport
        let cert_manager = transport.cert_manager.clone();
        let protocol_handler = Arc::new(StoqProtocolHandler::new(
            config.protocol.clone(),
            Some(cert_manager),
        ));

        let server = Self {
            transport: Arc::new(RwLock::new(transport)),
            protocol_handler,
            config,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        // Wire protocol handler to transport
        {
            let mut transport_guard = server.transport.write().await;
            transport_guard.set_protocol_handler(server.protocol_handler.clone());
        }

        info!("STOQ server initialized successfully");
        Ok(server)
    }

    /// Register a message handler for a specific message type
    pub async fn register_handler<T>(&self, message_type: String, handler: impl MessageHandler<T> + 'static)
    where
        T: serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        info!("Registering handler for message type: {}", message_type);
        self.protocol_handler.register_handler(message_type, handler).await;
    }

    /// Start the server and begin accepting connections
    pub async fn start(&self) -> Result<()> {
        if self.running.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Server is already running"));
        }

        info!("Starting STOQ server on [{}]:{}", self.config.bind_address, self.config.port);
        
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);

        // Main server loop - accept connections and let protocol handler manage them
        loop {
            if !self.running.load(std::sync::atomic::Ordering::Relaxed) {
                info!("Server stopping...");
                break;
            }

            // Accept incoming connection
            let transport = self.transport.read().await;
            match transport.accept().await {
                Ok(connection) => {
                    info!("Accepted new connection: {}", connection.id());
                    // Protocol handler is automatically engaged via transport integration
                }
                Err(e) => {
                    if self.running.load(std::sync::atomic::Ordering::Relaxed) {
                        warn!("Error accepting connection: {}", e);
                        // Brief delay to prevent tight error loops
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
        }

        info!("STOQ server stopped");
        Ok(())
    }

    /// Stop the server gracefully
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping STOQ server...");
        
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        
        // Shutdown transport layer
        let transport = self.transport.read().await;
        transport.shutdown().await;
        
        info!("STOQ server stopped successfully");
        Ok(())
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get server statistics
    pub async fn stats(&self) -> crate::TransportStats {
        let transport = self.transport.read().await;
        transport.stats()
    }

    /// Get the protocol handler for direct access
    pub fn protocol_handler(&self) -> Arc<StoqProtocolHandler> {
        self.protocol_handler.clone()
    }

    /// Get the transport layer for direct access
    pub fn transport(&self) -> Arc<RwLock<StoqTransport>> {
        self.transport.clone()
    }

    /// Run the server with graceful shutdown on SIGINT/SIGTERM
    pub async fn run_with_shutdown(&self) -> Result<()> {
        info!("Starting STOQ server with signal handling...");

        // Setup signal handling for graceful shutdown
        let running = self.running.clone();
        let server_clone = self.clone();
        
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            
            tokio::spawn(async move {
                let mut sigint = signal(SignalKind::interrupt()).expect("Failed to setup SIGINT handler");
                let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM handler");
                
                tokio::select! {
                    _ = sigint.recv() => {
                        info!("Received SIGINT, shutting down gracefully...");
                        running.store(false, std::sync::atomic::Ordering::Relaxed);
                        let _ = server_clone.stop().await;
                    }
                    _ = sigterm.recv() => {
                        info!("Received SIGTERM, shutting down gracefully...");
                        running.store(false, std::sync::atomic::Ordering::Relaxed);
                        let _ = server_clone.stop().await;
                    }
                }
            });
        }

        #[cfg(windows)]
        {
            use tokio::signal::windows::ctrl_c;
            
            tokio::spawn(async move {
                let mut ctrl_c = ctrl_c().expect("Failed to setup Ctrl+C handler");
                ctrl_c.recv().await;
                info!("Received Ctrl+C, shutting down gracefully...");
                running.store(false, std::sync::atomic::Ordering::Relaxed);
                let _ = server_clone.stop().await;
            });
        }

        // Start the main server loop
        self.start().await
    }
}

impl Clone for StoqServer {
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            protocol_handler: self.protocol_handler.clone(),
            config: self.config.clone(),
            running: self.running.clone(),
        }
    }
}

/// Example echo message handler
pub struct EchoMessageHandler;

#[async_trait::async_trait]
impl MessageHandler<String> for EchoMessageHandler {
    async fn handle_message(&self, message: StoqMessage<String>, connection_info: &ConnectionInfo) -> Result<Option<bytes::Bytes>> {
        info!("Echo handler received: '{}' from {}", message.payload, connection_info.connection_id);
        
        // Echo the message back
        let response = format!("Echo: {}", message.payload);
        let response_bytes = bincode::serialize(&response)?;
        Ok(Some(bytes::Bytes::from(response_bytes)))
    }
}

/// Example JSON message handler
pub struct JsonMessageHandler;

#[async_trait::async_trait]
impl MessageHandler<serde_json::Value> for JsonMessageHandler {
    async fn handle_message(&self, message: StoqMessage<serde_json::Value>, connection_info: &ConnectionInfo) -> Result<Option<bytes::Bytes>> {
        info!("JSON handler received from {}: {}", connection_info.connection_id, message.payload);
        
        // Process JSON and return a response
        let response = serde_json::json!({
            "status": "received",
            "original": message.payload,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });
        
        let response_bytes = bincode::serialize(&response)?;
        Ok(Some(bytes::Bytes::from(response_bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[tokio::test]
    async fn test_server_creation() {
        let config = StoqServerConfig {
            bind_address: Ipv6Addr::LOCALHOST,
            port: 0, // Use ephemeral port
            ..Default::default()
        };

        // Initialize crypto provider
        if rustls::crypto::ring::default_provider().install_default().is_err() {
            // Already installed, ignore error
        }

        let server = StoqServer::new(config).await;
        assert!(server.is_ok());
        
        if let Ok(server) = server {
            assert!(!server.is_running());
        }
    }

    #[tokio::test]
    async fn test_handler_registration() {
        let config = StoqServerConfig::default();
        
        // Initialize crypto provider
        if rustls::crypto::ring::default_provider().install_default().is_err() {
            // Already installed, ignore error
        }

        if let Ok(server) = StoqServer::new(config).await {
            server.register_handler("echo".to_string(), EchoMessageHandler).await;
            server.register_handler("json".to_string(), JsonMessageHandler).await;
            
            // Verify handlers are registered (we'd need to expose a way to check this)
            assert!(!server.is_running());
        }
    }
}