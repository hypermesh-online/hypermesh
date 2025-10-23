//! STOQ Protocol - Standalone QUIC over IPv6 with CDN capabilities
//! 
//! This crate provides a high-performance, standalone transport protocol
//! built on QUIC over IPv6 with advanced CDN and edge network capabilities.
//! 
//! STOQ is designed to work independently without Nexus or HyperMesh dependencies.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod transport;
pub mod routing;
pub mod chunking;
pub mod edge;
pub mod config;

use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Result;
use bytes::Bytes;
use serde::{Serialize, Deserialize};

// Re-export key types
pub use transport::{StoqTransport, Connection, Endpoint};
pub use routing::{StoqRouter, Route, NodeId, NodeMetrics, RoutingMatrix};
pub use chunking::{ChunkEngine, Chunk, ChunkId};
pub use edge::{StoqEdgeNetwork, EdgeNode, EdgeCache};
pub use routing::GeoLocation;
pub use config::StoqConfig;

/// STOQ Protocol version
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Maximum transmission unit for STOQ
pub const STOQ_MTU: usize = 1400;

/// Default port for STOQ protocol
pub const DEFAULT_PORT: u16 = 9292;

/// Core STOQ transport trait for standalone operation
#[async_trait]
pub trait Transport: Send + Sync {
    /// Connect to a remote endpoint
    async fn connect(&self, endpoint: &Endpoint) -> Result<Connection>;
    
    /// Listen for incoming connections
    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn Listener>>;
    
    /// Send data over a connection
    async fn send(&self, conn: &Connection, data: &[u8]) -> Result<()>;
    
    /// Receive data from a connection
    async fn receive(&self, conn: &Connection) -> Result<Bytes>;
    
    /// Get transport statistics
    fn stats(&self) -> TransportStats;
}

/// Listener trait for accepting connections
#[async_trait]
pub trait Listener: Send + Sync {
    /// Accept an incoming connection
    async fn accept(&self) -> Result<Connection>;
    
    /// Get the local address
    fn local_addr(&self) -> Result<SocketAddr>;
}

/// Router trait for CDN routing decisions
#[async_trait]
pub trait Router: Send + Sync {
    /// Find optimal route between nodes
    async fn find_route(&self, src: NodeId, dst: NodeId) -> Result<Route>;
    
    /// Update node metrics
    async fn update_metrics(&self, metrics: NodeMetrics) -> Result<()>;
    
    /// Get the current routing matrix
    fn routing_matrix(&self) -> &RoutingMatrix;
    
    /// Calculate route cost
    fn calculate_cost(&self, route: &Route) -> f64;
}

/// Chunker trait for data chunking and deduplication
#[async_trait]
pub trait Chunker: Send + Sync {
    /// Chunk data into smaller pieces
    fn chunk(&self, data: &[u8]) -> Result<Vec<Chunk>>;
    
    /// Reassemble chunks into original data
    fn reassemble(&self, chunks: Vec<Chunk>) -> Result<Bytes>;
    
    /// Check for duplicate chunks
    fn deduplicate(&self, chunks: &[Chunk]) -> Vec<ChunkId>;
    
    /// Get chunk statistics
    fn stats(&self) -> ChunkStats;
}

/// Edge network trait for CDN edge management
#[async_trait]
pub trait EdgeNetwork: Send + Sync {
    /// Register an edge node
    async fn register_edge(&self, node: EdgeNode) -> Result<()>;
    
    /// Find nearest edge node to a location
    async fn find_nearest(&self, location: GeoLocation) -> Result<EdgeNode>;
    
    /// Cache content at edge nodes
    async fn cache_content(&self, content: Content) -> Result<()>;
    
    /// Invalidate cached content
    async fn invalidate_cache(&self, content_id: ContentId) -> Result<()>;
    
    /// Get edge network statistics
    fn stats(&self) -> EdgeStats;
}

/// Transport statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportStats {
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Active connections
    pub active_connections: usize,
    /// Total connections established
    pub total_connections: u64,
    /// Current throughput in Gbps
    pub throughput_gbps: f64,
    /// Average latency in microseconds
    pub avg_latency_us: u64,
}

/// Chunk statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkStats {
    /// Total chunks created
    pub total_chunks: u64,
    /// Duplicate chunks found
    pub duplicate_chunks: u64,
    /// Deduplication ratio
    pub dedup_ratio: f64,
    /// Average chunk size
    pub avg_chunk_size: usize,
}

/// Edge network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeStats {
    /// Number of edge nodes
    pub edge_nodes: usize,
    /// Total cache size
    pub total_cache_size: u64,
    /// Cache hit ratio
    pub cache_hit_ratio: f64,
    /// Average response time
    pub avg_response_time_ms: u64,
}

/// Content to be cached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Unique content identifier
    pub id: ContentId,
    /// Content data
    pub data: Bytes,
    /// Content type
    pub content_type: String,
    /// TTL in seconds
    pub ttl: u64,
}

/// Content identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentId(pub String);

/// STOQ builder for creating configured instances
pub struct StoqBuilder {
    config: StoqConfig,
}

impl StoqBuilder {
    /// Create a new builder with default config
    pub fn new() -> Self {
        Self {
            config: StoqConfig::default(),
        }
    }
    
    /// Set custom configuration
    pub fn with_config(mut self, config: StoqConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Build the STOQ instance
    pub async fn build(self) -> Result<Stoq> {
        Stoq::new(self.config).await
    }
}

/// Main STOQ protocol implementation
pub struct Stoq {
    transport: Arc<StoqTransport>,
    router: Arc<StoqRouter>,
    chunker: Arc<ChunkEngine>,
    edge: Arc<StoqEdgeNetwork>,
    config: StoqConfig,
}

impl Stoq {
    /// Create a new STOQ instance with configuration
    pub async fn new(config: StoqConfig) -> Result<Self> {
        let transport = Arc::new(StoqTransport::new(config.transport.clone()).await?);
        let router = Arc::new(StoqRouter::new(config.routing.clone())?);
        let chunker = Arc::new(ChunkEngine::new(config.chunking.clone())?);
        let edge = Arc::new(StoqEdgeNetwork::new(config.edge.clone()).await?);
        
        Ok(Self {
            transport,
            router,
            chunker,
            edge,
            config,
        })
    }
    
    /// Get the transport layer
    pub fn transport(&self) -> Arc<StoqTransport> {
        self.transport.clone()
    }
    
    /// Get the router
    pub fn router(&self) -> Arc<StoqRouter> {
        self.router.clone()
    }
    
    /// Get the chunker
    pub fn chunker(&self) -> Arc<ChunkEngine> {
        self.chunker.clone()
    }
    
    /// Get the edge network
    pub fn edge(&self) -> Arc<StoqEdgeNetwork> {
        self.edge.clone()
    }
    
    /// Get current configuration
    pub fn config(&self) -> &StoqConfig {
        &self.config
    }
}

impl Default for StoqBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_stoq_builder() {
        let stoq = StoqBuilder::new()
            .build()
            .await;
        assert!(stoq.is_ok());
    }
    
    #[test]
    fn test_protocol_version() {
        assert_eq!(PROTOCOL_VERSION, "1.0.0");
    }
    
    #[test]
    fn test_default_values() {
        assert_eq!(DEFAULT_PORT, 9292);
        assert_eq!(STOQ_MTU, 1400);
    }
}