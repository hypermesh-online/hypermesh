//! Transport wrapper for unified QUIC client/server interface

use crate::{Result, RuntimeError};
use nexus_transport::{QuicClient, QuicServer, Connection, TransportConfig};
use nexus_shared::NodeId;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Unified QUIC transport wrapper
pub struct QuicTransport {
    client: Arc<RwLock<Option<QuicClient>>>,
    server: Arc<RwLock<Option<QuicServer>>>,
    config: TransportConfig,
}

impl QuicTransport {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            server: Arc::new(RwLock::new(None)),
            config,
        }
    }
    
    pub async fn start_client(&self, cert_manager: Arc<nexus_transport::CertificateManager>) -> Result<()> {
        let mut client = QuicClient::new(self.config.clone(), cert_manager).await
            .map_err(|e| RuntimeError::Transport { message: e.to_string() })?;
        
        client.start().await
            .map_err(|e| RuntimeError::Transport { message: e.to_string() })?;
            
        *self.client.write().await = Some(client);
        Ok(())
    }
    
    pub async fn start_server(&self, cert_manager: Arc<nexus_transport::CertificateManager>) -> Result<SocketAddr> {
        let mut server = QuicServer::new(self.config.clone(), cert_manager).await
            .map_err(|e| RuntimeError::Transport { message: e.to_string() })?;
        
        let addr = server.start().await
            .map_err(|e| RuntimeError::Transport { message: e.to_string() })?;
            
        *self.server.write().await = Some(server);
        Ok(addr)
    }
    
    pub async fn connect(&self, addr: SocketAddr, server_name: &str) -> Result<NodeId> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| RuntimeError::Transport {
            message: "Client not started".to_string()
        })?;
        
        client.connect(addr, server_name).await
            .map_err(|e| RuntimeError::Transport { message: e.to_string() })
    }
    
    pub async fn get_connection(&self, node_id: NodeId) -> Option<Arc<Connection>> {
        if let Some(client) = self.client.read().await.as_ref() {
            client.get_connection(node_id).await
        } else {
            None
        }
    }
}

impl std::fmt::Debug for QuicTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuicTransport")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}