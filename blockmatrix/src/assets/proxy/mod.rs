//! Remote Proxy/NAT System for HyperMesh
//!
//! CRITICAL IMPLEMENTATION: Complete NAT-like addressing system for memory/resources
//! with global proxy addresses, federated trust integration, and quantum-resistant security.
//!
//! This module implements the highest priority missing component from the Caesar Asset Roadmap.

pub mod manager;
pub mod routing;
pub mod forwarding;
pub mod trust_integration;
pub mod security;
pub mod sharding;
pub mod nat_translation;
pub mod remote_memory_transport;

pub use manager::{RemoteProxyManager, ForwardingRuleType};
pub use routing::{ProxyRouter, ProxyRoute, RouteTable};
pub use forwarding::{ProxyForwarder, ForwardingRule, ForwardingMode};
pub use trust_integration::{TrustChainIntegration, CertificateValidator};
pub use security::{QuantumSecurity, FalconSigner, KyberEncryption};
pub use sharding::{ShardedDataAccess, ShardManager, EncryptedShard};
pub use nat_translation::{NATTranslator, GlobalAddress, MemoryPermissions};
pub use remote_memory_transport::{
    RemoteMemoryTransport, TransportConfig, MappedMemoryRegion,
    MemoryOperationType, OperationResult, TransportMetrics,
};

use std::collections::HashMap;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

// Re-export ProxyAddress from core
pub use crate::assets::core::{ProxyAddress, AssetId, AssetResult, AssetError, PrivacyLevel};

/// Global proxy network configuration
#[derive(Clone, Debug)]
pub struct ProxyNetworkConfig {
    /// HyperMesh network prefix (IPv6-like)
    pub network_prefix: [u8; 8],
    /// HyperMesh network ID
    pub hypermesh_network_id: Option<String>,
    /// Default port ranges for different services
    pub port_ranges: HashMap<String, PortRange>,
    /// Default port range (for backwards compatibility)
    pub default_port_range: Option<PortRange>,
    /// Trust-based proxy selection enabled
    pub trust_based_selection: bool,
    /// Minimum trust score required
    pub min_trust_score: f32,
    /// Quantum security enabled
    pub quantum_security_enabled: bool,
    /// Sharded data access enabled
    pub sharded_access_enabled: bool,
}

/// Port range specification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

impl Default for ProxyNetworkConfig {
    fn default() -> Self {
        let mut port_ranges = HashMap::new();
        port_ranges.insert("memory".to_string(), PortRange { start: 8000, end: 8999 });
        port_ranges.insert("cpu".to_string(), PortRange { start: 9000, end: 9999 });
        port_ranges.insert("gpu".to_string(), PortRange { start: 10000, end: 10999 });
        port_ranges.insert("storage".to_string(), PortRange { start: 11000, end: 11999 });
        port_ranges.insert("network".to_string(), PortRange { start: 12000, end: 12999 });
        
        Self {
            network_prefix: [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad], // HyperMesh IPv6 prefix
            hypermesh_network_id: Some("hypermesh-main".to_string()),
            port_ranges: port_ranges.clone(),
            default_port_range: port_ranges.get("memory").cloned(),
            trust_based_selection: true,
            min_trust_score: 0.5,
            quantum_security_enabled: true,
            sharded_access_enabled: true,
        }
    }
}

/// Proxy system statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxySystemStats {
    /// Total active proxy nodes
    pub active_proxy_nodes: u64,
    /// Total proxy mappings
    pub total_mappings: u64,
    /// Total forwarded requests
    pub forwarded_requests: u64,
    /// Total NAT translations
    pub nat_translations: u64,
    /// Total bytes transferred through proxy
    pub total_bytes_transferred: u64,
    /// Average response time in milliseconds
    pub average_response_time_ms: f64,
    /// Quantum security validations
    pub quantum_validations: u64,
    /// Trust score validations
    pub trust_validations: u64,
    /// Sharded access requests
    pub sharded_requests: u64,
}

/// Proxy system errors
#[derive(Debug, thiserror::Error)]
pub enum ProxySystemError {
    #[error("Proxy node not found: {node_id}")]
    ProxyNodeNotFound { node_id: String },
    
    #[error("NAT translation failed for address: {address}")]
    NATTranslationFailed { address: String },
    
    #[error("Trust validation failed: {reason}")]
    TrustValidationFailed { reason: String },
    
    #[error("Quantum security validation failed: {reason}")]
    QuantumSecurityFailed { reason: String },
    
    #[error("Forwarding failed: {reason}")]
    ForwardingFailed { reason: String },
    
    #[error("Sharded access failed: {reason}")]
    ShardedAccessFailed { reason: String },
    
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },
}