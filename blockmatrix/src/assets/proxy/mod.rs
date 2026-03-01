// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Remote Proxy/NAT System for HyperMesh
//!
//! CRITICAL IMPLEMENTATION: Complete NAT-like addressing system for memory/resources
//! with global proxy addresses, federated trust integration, and quantum-resistant security.
//!
//! This module implements the highest priority missing component from the Caesar Asset Roadmap.

pub mod forwarding;
pub mod manager;
pub mod nat_translation;
pub mod proxy_selector;
pub mod remote_memory_transport;
pub mod routing;
pub mod scope_routing;
pub mod security;
pub mod sharding;
pub mod trust_integration;

pub use forwarding::{ForwardingMode, ForwardingRule, ForwardingRuleType, ProxyForwarder};
pub use manager::RemoteProxyManager;
pub use nat_translation::{GlobalAddress, MemoryPermissions, NATTranslator, PrivacyConfig};
pub use proxy_selector::{ProxyNode, ProxySelector, ProxySelectorConfig, TrustLevel};
pub use remote_memory_transport::{
    MappedMemoryRegion, MemoryOperationType, OperationResult, RemoteMemoryTransport,
    TransportConfig, TransportMetrics,
};
pub use routing::{ProxyRoute, ProxyRouter, RouteTable};
pub use scope_routing::{
    GatewayNodeInfo, ScopeAwareRoute, ScopeAwareRouter, ScopeRoutingConfig, ScopeRoutingError,
    ScopeRoutingStats,
};
pub use security::{FalconSigner, KyberEncryption, QuantumSecurity};
pub use sharding::{EncryptedShard, ShardManager, ShardedDataAccess};
pub use trust_integration::{CertificateValidator, TrustChainIntegration};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export ProxyAddress from core
pub use crate::assets::core::{
    AssetError, AssetRegistration, AssetResult, PrivacyMode, ProxyAddress,
};

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
    /// Whether authentication is required for proxy nodes
    pub require_authentication: bool,
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
        port_ranges.insert(
            "memory".to_string(),
            PortRange {
                start: 8000,
                end: 8999,
            },
        );
        port_ranges.insert(
            "cpu".to_string(),
            PortRange {
                start: 9000,
                end: 9999,
            },
        );
        port_ranges.insert(
            "gpu".to_string(),
            PortRange {
                start: 10000,
                end: 10999,
            },
        );
        port_ranges.insert(
            "storage".to_string(),
            PortRange {
                start: 11000,
                end: 11999,
            },
        );
        port_ranges.insert(
            "network".to_string(),
            PortRange {
                start: 12000,
                end: 12999,
            },
        );

        Self {
            network_prefix: [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad], // HyperMesh IPv6 prefix
            hypermesh_network_id: Some("hypermesh-main".to_string()),
            port_ranges: port_ranges.clone(),
            default_port_range: port_ranges.get("memory").cloned(),
            require_authentication: true,
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
