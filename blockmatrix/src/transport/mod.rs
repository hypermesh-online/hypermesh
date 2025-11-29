//! HyperMesh Transport Layer
//!
//! This module provides STOQ protocol implementation for HyperMesh
//! distributed communication and transport.

pub mod types;
pub mod config;
pub mod auth;
pub mod error;
pub mod monitoring;
pub mod pool;
pub mod benches;
pub mod tests;

// Re-export main types
pub use types::NodeId;

pub use config::{
    TransportConfig, HyperMeshTransportConfig,
};

pub use auth::{
    AuthManager, NodeAuthenticator,
};

pub use error::{
    TransportError,
};

pub use monitoring::{
    TransportMetrics, TransportMonitor,
};

pub use pool::{
    StoqConnection, Endpoint, ConnectionPool,
};

// Main transport types
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Result;

/// HyperMesh endpoint representation
#[derive(Debug, Clone)]
pub struct HyperMeshEndpoint {
    /// IPv6 address
    pub address: std::net::Ipv6Addr,
    /// Port number
    pub port: u16,
    /// Optional server name for SNI
    pub server_name: Option<String>,
    /// Optional node ID for authentication
    pub node_id: Option<String>,
}

/// HyperMesh connection
#[derive(Clone)]
pub struct HyperMeshConnection {
    /// Connection ID
    pub id: String,
    /// Remote endpoint
    pub endpoint: HyperMeshEndpoint,
    /// Connection status
    pub active: Arc<parking_lot::RwLock<bool>>,
}

/// HyperMesh transport statistics
#[derive(Debug, Clone)]
pub struct HyperMeshTransportStats {
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Active connections
    pub active_connections: usize,
}

/// HyperMesh transport trait for consensus integration
#[async_trait]
pub trait HyperMeshTransportTrait: Send + Sync {
    /// Connect to a remote endpoint
    async fn connect(&self, endpoint: &HyperMeshEndpoint) -> Result<HyperMeshConnection>;

    /// Send message to a node
    async fn send(&self, node_id: &NodeId, message: &[u8]) -> Result<()>;

    /// Receive message from any node
    async fn receive(&self) -> Result<(NodeId, Vec<u8>)>;

    /// Get transport statistics
    fn get_stats(&self) -> HyperMeshTransportStats;
}

/// HyperMesh transport implementation
pub struct HyperMeshTransport {
    config: TransportConfig,
}

impl HyperMeshTransport {
    /// Create new transport instance
    pub fn new(config: TransportConfig) -> Self {
        Self { config }
    }

    /// Create new transport instance with HyperMeshTransportConfig
    pub async fn new_async(config: HyperMeshTransportConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        0 // Stub implementation
    }

    /// Get transport statistics
    pub async fn stats(&self) -> HyperMeshTransportStats {
        HyperMeshTransportStats {
            bytes_sent: 0,
            bytes_received: 0,
            active_connections: 0,
        }
    }

    /// Connect to a node
    pub async fn connect_to_node(&self, _node_id: NodeId, _endpoint: &stoq::Endpoint) -> Result<()> {
        Ok(()) // Stub implementation
    }

    /// Send data to a connection
    pub async fn send_data(&self, _conn_id: &str, _data: &[u8]) -> Result<()> {
        Ok(()) // Stub implementation
    }

    /// Listen for connections
    pub async fn listen(&self, _endpoint: &stoq::Endpoint) -> Result<()> {
        Ok(()) // Stub implementation
    }

    /// Accept a connection
    pub async fn accept(&self) -> Result<(String, NodeId)> {
        Ok((String::new(), NodeId::new("".to_string()))) // Stub implementation
    }

    /// Close a connection
    pub async fn close_connection(&self, _conn_id: &str) -> Result<()> {
        Ok(()) // Stub implementation
    }

    /// Send message to a node
    pub async fn send_message(&self, _node_id: &NodeId, _message: Vec<u8>) -> Result<()> {
        Ok(()) // Stub implementation
    }

    /// Receive message
    pub async fn receive_message(&self) -> Result<(NodeId, Vec<u8>)> {
        Ok((NodeId::new("".to_string()), Vec::new())) // Stub implementation
    }
}

// Re-export as aliases for compatibility
pub use HyperMeshConnection as Connection;
pub use HyperMeshEndpoint as HyperEndpoint;
pub use HyperMeshTransportStats as TransportStats;
pub use HyperMeshTransport as HyperTransport;