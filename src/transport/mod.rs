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
    TransportConfig,
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
}

// Re-export as aliases for compatibility
pub use HyperMeshConnection as Connection;
pub use HyperMeshEndpoint as HyperEndpoint;
pub use HyperMeshTransportStats as TransportStats;
pub use HyperMeshTransport as HyperTransport;