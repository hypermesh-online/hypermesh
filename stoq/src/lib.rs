//! STOQ Protocol - Pure QUIC over IPv6 Transport
//!
//! This crate provides a high-performance, pure transport protocol
//! built on QUIC over IPv6. STOQ focuses exclusively on transport layer responsibilities:
//! packet delivery, connection management, flow control, and congestion control.
//!
//! STOQ is designed as a pure transport protocol without application-layer features.

#![warn(missing_docs)]

pub mod transport;
pub mod config;
pub mod extensions;
pub mod protocol;

// ARCHITECTURE ENFORCEMENT: STOQ is pure transport - no routing, chunking, or edge features
// These belong in application layers that use STOQ as transport

use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Result;
use serde::{Serialize, Deserialize};

// Re-export pure transport types and protocol extensions
pub use transport::{StoqTransport, TransportConfig, Connection, Endpoint, Stream, NetworkTier};
pub use transport::falcon::{
    FalconEngine, FalconTransport, FalconVariant, FalconPublicKey,
    FalconPrivateKey, FalconSignature
};
pub use config::StoqConfig;
pub use extensions::{
    StoqProtocolExtension, DefaultStoqExtensions, PacketToken, PacketShard,
    HopInfo, SeedInfo, SeedNode, SeedPriority, StoqPacket
};

/// STOQ Protocol version
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Maximum transmission unit for STOQ
pub const STOQ_MTU: usize = 1400;

/// Default port for STOQ protocol
pub const DEFAULT_PORT: u16 = 9292;

/// Pure transport trait - focused only on packet delivery
#[async_trait]
pub trait Transport: Send + Sync {
    /// Connect to a remote endpoint
    async fn connect(&self, endpoint: &Endpoint) -> Result<Connection>;
    
    /// Accept incoming connections
    async fn accept(&self) -> Result<Connection>;
    
    /// Get transport statistics
    fn stats(&self) -> TransportStats;
    
    /// Close all connections and shutdown
    async fn shutdown(&self);
}

/// Listener trait for accepting connections
#[async_trait]
pub trait Listener: Send + Sync {
    /// Accept an incoming connection
    async fn accept(&self) -> Result<Connection>;
    
    /// Get the local address
    fn local_addr(&self) -> Result<SocketAddr>;
}

// REMOVED: Router, Chunker, EdgeNetwork traits
// These are application-layer concerns, not transport responsibilities

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

// REMOVED: Application-layer statistics and content types
// Transport layer only tracks transport-specific metrics

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

/// Pure STOQ transport implementation
pub struct Stoq {
    transport: Arc<StoqTransport>,
    config: StoqConfig,
}

impl Stoq {
    /// Create a new STOQ transport instance
    pub async fn new(config: StoqConfig) -> Result<Self> {
        let transport = Arc::new(StoqTransport::new(config.transport.clone()).await?);
        
        Ok(Self {
            transport,
            config,
        })
    }
    
    /// Get the transport layer
    pub fn transport(&self) -> Arc<StoqTransport> {
        self.transport.clone()
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
        // Initialize crypto provider
        if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
            // Already installed, ignore error
        }
        
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