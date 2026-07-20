// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network Configuration Types
//!
//! Provides configuration types for different network participation modes,
//! supporting all four network types with their specific requirements.
//!
//! CRITICAL: Networks are IMMUTABLE once created. Network types CANNOT transition.
//! Only independent connect/disconnect operations are allowed.

use super::trust::StateProof;
use serde::{Deserialize, Serialize};

/// Configuration for joining a network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Peer addresses for P2P mode
    pub peer_addresses: Vec<String>,

    /// Federation gateway URL
    pub federation_gateway: Option<String>,

    /// DNS name for public network
    pub dns_name: Option<String>,

    /// Proof of State for public network
    pub proof_of_state: Option<StateProof>,

    /// Custom STOQ port
    pub stoq_port: Option<u16>,

    /// Network-specific metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl NetworkConfig {
    /// Create configuration for anonymous network
    pub fn anonymous() -> Self {
        Self {
            peer_addresses: vec![],
            federation_gateway: None,
            dns_name: None,
            proof_of_state: None,
            stoq_port: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create configuration for P2P network
    pub fn p2p(peers: Vec<String>) -> Self {
        Self {
            peer_addresses: peers,
            federation_gateway: None,
            dns_name: None,
            proof_of_state: None,
            stoq_port: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create configuration for federated network
    pub fn federated(gateway: String) -> Self {
        Self {
            peer_addresses: vec![],
            federation_gateway: Some(gateway),
            dns_name: None,
            proof_of_state: None,
            stoq_port: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create configuration for public network
    pub fn public(dns_name: String, proof: StateProof) -> Self {
        Self {
            peer_addresses: vec![],
            federation_gateway: None,
            dns_name: Some(dns_name),
            proof_of_state: Some(proof),
            stoq_port: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set custom STOQ port
    pub fn with_stoq_port(mut self, port: u16) -> Self {
        self.stoq_port = Some(port);
        self
    }

    /// Add metadata entry
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Anonymous requires nothing
        if self.is_anonymous() {
            return Ok(());
        }

        // P2P requires peer addresses (and nothing else set)
        if self.is_p2p() {
            return Ok(());
        }

        // Federated requires gateway (and nothing else set)
        if self.is_federated() {
            return Ok(());
        }

        // Public requires both DNS and proof
        if self.is_public() {
            return Ok(());
        }

        // Partial public: proof_of_state without dns_name or vice versa
        if self.proof_of_state.is_some() && self.dns_name.is_none() {
            return Err("Public configuration requires DNS name".to_string());
        }
        if self.dns_name.is_some() && self.proof_of_state.is_none() {
            return Err("Public configuration requires Proof of State".to_string());
        }

        // Config doesn't match any known type
        Err("Configuration does not match any valid network type".to_string())
    }

    /// Check if this is anonymous configuration
    pub fn is_anonymous(&self) -> bool {
        self.peer_addresses.is_empty()
            && self.federation_gateway.is_none()
            && self.dns_name.is_none()
            && self.proof_of_state.is_none()
    }

    /// Check if this is P2P configuration
    pub fn is_p2p(&self) -> bool {
        !self.peer_addresses.is_empty()
            && self.federation_gateway.is_none()
            && self.dns_name.is_none()
            && self.proof_of_state.is_none()
    }

    /// Check if this is federated configuration
    pub fn is_federated(&self) -> bool {
        self.federation_gateway.is_some()
            && self.peer_addresses.is_empty()
            && self.dns_name.is_none()
            && self.proof_of_state.is_none()
    }

    /// Check if this is public configuration
    pub fn is_public(&self) -> bool {
        self.dns_name.is_some() && self.proof_of_state.is_some()
    }
}

/// Node configuration for multi-network participation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Whether to disable public network entirely
    pub disable_public: bool,

    /// Whether to disable anonymous network
    pub disable_anonymous: bool,

    /// Default federation gateway
    pub default_federation_gateway: Option<String>,

    /// Default P2P peers
    pub default_p2p_peers: Vec<String>,

    /// Maximum networks to join simultaneously
    pub max_networks: usize,

    /// Resource limits per network type
    pub resource_limits: ResourceLimits,

    /// Asset visibility policy
    pub asset_visibility_policy: AssetVisibilityPolicy,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            disable_public: false,
            disable_anonymous: false,
            default_federation_gateway: None,
            default_p2p_peers: vec![],
            max_networks: 10,
            resource_limits: ResourceLimits::default(),
            asset_visibility_policy: AssetVisibilityPolicy::Explicit,
        }
    }
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU percentage per network type
    pub cpu_per_network: f32,

    /// Memory MB per network
    pub memory_mb_per_network: u64,

    /// Storage GB per network
    pub storage_gb_per_network: u64,

    /// Bandwidth Mbps per network
    pub bandwidth_mbps_per_network: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_per_network: 25.0,           // 25% CPU per network
            memory_mb_per_network: 2048,     // 2GB RAM per network
            storage_gb_per_network: 50,      // 50GB storage per network
            bandwidth_mbps_per_network: 100, // 100 Mbps per network
        }
    }
}

/// Asset visibility policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AssetVisibilityPolicy {
    /// Assets are private by default
    Private,
    /// Assets visible to all networks by default
    AllNetworks,
    /// Each asset must be explicitly configured
    Explicit,
    /// Assets visible only to same network type
    SameTypeOnly,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        // Anonymous - valid
        let anon = NetworkConfig::anonymous();
        assert!(anon.validate().is_ok());
        assert!(anon.is_anonymous());

        // P2P - valid with peers
        let p2p = NetworkConfig::p2p(vec!["peer1.local".to_string()]);
        assert!(p2p.validate().is_ok());
        assert!(p2p.is_p2p());

        // Federated - valid with gateway
        let fed = NetworkConfig::federated("gateway.fed".to_string());
        assert!(fed.validate().is_ok());
        assert!(fed.is_federated());

        // Public - invalid without DNS
        let mut public = NetworkConfig {
            peer_addresses: vec![],
            federation_gateway: None,
            dns_name: None,
            proof_of_state: Some(StateProof::default()),
            stoq_port: None,
            metadata: std::collections::HashMap::new(),
        };
        assert!(public.validate().is_err());

        // Public - valid with DNS and proof
        public.dns_name = Some("node.hypermesh.online".to_string());
        assert!(public.validate().is_ok());
        assert!(public.is_public());
    }

    #[test]
    fn test_config_builder() {
        let config = NetworkConfig::p2p(vec!["peer1".to_string()])
            .with_stoq_port(8443)
            .with_metadata("region".to_string(), "us-west".to_string());

        assert_eq!(config.stoq_port, Some(8443));
        assert_eq!(config.metadata.get("region"), Some(&"us-west".to_string()));
    }
}
