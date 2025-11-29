//! Nexus Transport - QUIC-based secure transport layer
//! 
//! This module provides the foundational transport layer for Nexus using QUIC over IPv6.
//! Key features include:
//! - Certificate-based authentication
//! - Zero-round-trip connection resumption
//! - Connection migration support
//! - Built-in flow control and congestion control
//! - Multiplexed streams within connections

pub mod client;
pub mod server;
pub mod config;
pub mod error;
pub mod certificate;
pub mod stream;
pub mod connection;

pub use client::QuicClient;
pub use server::QuicServer;
pub use config::TransportConfig;
pub use error::{TransportError, Result};
pub use certificate::{CertificateManager, generate_self_signed_cert};
pub use stream::{QuicStream, StreamType};
pub use connection::{Connection, ConnectionInfo};

use nexus_shared::{NodeId, NexusError};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

/// Protocol version for transport layer
pub const TRANSPORT_PROTOCOL_VERSION: u32 = 1;

/// Maximum transmission unit for QUIC packets
pub const MAX_MTU: u16 = 1400;

/// Default keep-alive interval
pub const DEFAULT_KEEP_ALIVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// Maximum message size (10MB)
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Message types for transport protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    /// Handshake and authentication
    Handshake,
    /// Application data
    Data,
    /// Control messages (ping, pong, etc.)
    Control,
    /// Stream management
    Stream,
}

/// Transport message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportMessage {
    /// Message type
    pub message_type: MessageType,
    /// Source node ID
    pub source: NodeId,
    /// Destination node ID (optional for broadcast)
    pub destination: Option<NodeId>,
    /// Message payload
    pub payload: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
    /// Message sequence number
    pub sequence: u64,
}

impl TransportMessage {
    pub fn new(
        message_type: MessageType,
        source: NodeId,
        destination: Option<NodeId>,
        payload: Vec<u8>,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        Self {
            message_type,
            source,
            destination,
            payload,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64,
            sequence: 0, // Will be set by connection
        }
    }
    
    /// Serialize message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| {
            TransportError::Serialization {
                message: format!("Failed to serialize transport message: {}", e),
            }
        })
    }
    
    /// Deserialize message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).map_err(|e| {
            TransportError::Serialization {
                message: format!("Failed to deserialize transport message: {}", e),
            }
        })
    }
}

/// Transport layer builder
pub struct TransportBuilder {
    config: TransportConfig,
    certificate_manager: Option<Arc<CertificateManager>>,
}

impl TransportBuilder {
    pub fn new() -> Self {
        Self {
            config: TransportConfig::default(),
            certificate_manager: None,
        }
    }
    
    pub fn with_config(mut self, config: TransportConfig) -> Self {
        self.config = config;
        self
    }
    
    pub fn with_certificate_manager(mut self, cert_manager: Arc<CertificateManager>) -> Self {
        self.certificate_manager = Some(cert_manager);
        self
    }
    
    pub async fn build_server(self) -> Result<QuicServer> {
        let cert_manager = self.certificate_manager
            .ok_or_else(|| TransportError::Configuration {
                message: "Certificate manager is required for server".to_string(),
            })?;
            
        QuicServer::new(self.config, cert_manager).await
    }
    
    pub async fn build_client(self) -> Result<QuicClient> {
        let cert_manager = self.certificate_manager
            .ok_or_else(|| TransportError::Configuration {
                message: "Certificate manager is required for client".to_string(),
            })?;
            
        QuicClient::new(self.config, cert_manager).await
    }
}

impl Default for TransportBuilder {
    fn default() -> Self {
        Self::new()
    }
}