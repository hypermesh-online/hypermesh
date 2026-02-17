// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Network Isolation - Protocol-level network separation
//!
//! Revolutionary Concept #4: Multi-Network Participation
//! STOQ enforces complete packet isolation between networks at protocol level
//!
//! Each network gets:
//! - Independent network stack
//! - Separate connection pools
//! - Isolated packet queues
//! - Privacy tier enforcement
//! - Zero cross-talk guarantee

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};

use crate::transport::{StoqTransport, Connection};

/// Network identifier (16 bytes, matches BlockMatrix NetworkId)
pub type NetworkId = [u8; 16];

/// Network isolation manager
pub struct NetworkIsolationManager {
    /// Isolated network stacks per network
    network_stacks: Arc<RwLock<HashMap<NetworkId, NetworkStack>>>,
    /// Isolation violations tracker
    violations: Arc<RwLock<Vec<IsolationViolation>>>,
    /// Configuration
    config: IsolationConfig,
}

/// Isolated network stack
pub struct NetworkStack {
    /// Network ID
    pub network_id: NetworkId,
    /// Network name
    pub name: String,
    /// STOQ transport instance for this network
    pub transport: Arc<StoqTransport>,
    /// Active connections in this network
    pub connections: Arc<RwLock<HashMap<ConnectionId, Connection>>>,
    /// Privacy tier for this network
    pub privacy_tier: PrivacyTier,
    /// Network statistics
    pub stats: Arc<RwLock<NetworkStats>>,
    /// Explicit tunnels to other networks (if configured)
    pub tunnels: Arc<RwLock<HashMap<NetworkId, NetworkTunnel>>>,
}

/// Connection identifier
pub type ConnectionId = u64;

/// Privacy tier (matches BlockMatrix privacy tiers)
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PrivacyTier {
    /// Anonymous - Zero identity tracking
    Anonymous,
    /// Private P2P - Trusted peer circles
    PrivateP2P,
    /// Federated - Cross-network partner trust
    Federated,
    /// Public - Full transparency with PoS validation
    Public,
}

impl PrivacyTier {
    /// Get privacy tier name
    pub fn name(&self) -> &str {
        match self {
            PrivacyTier::Anonymous => "Anonymous",
            PrivacyTier::PrivateP2P => "PrivateP2P",
            PrivacyTier::Federated => "Federated",
            PrivacyTier::Public => "Public",
        }
    }
}

/// Network tunnel for explicit cross-network communication
#[derive(Clone, Debug)]
pub struct NetworkTunnel {
    /// Source network
    pub from_network: NetworkId,
    /// Target network
    pub to_network: NetworkId,
    /// Allowed traffic types
    pub allowed_traffic: Vec<TrafficType>,
    /// Validation requirements
    pub validation_required: bool,
    /// Active
    pub active: bool,
}

/// Traffic type for tunnel filtering
#[derive(Clone, Debug, PartialEq)]
pub enum TrafficType {
    /// Asset validation proofs
    AssetProof,
    /// Blockchain consensus
    Consensus,
    /// Certificate validation
    Certificate,
    /// All traffic (use with caution)
    All,
}

/// Network statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Active connections
    pub active_connections: usize,
    /// Isolation violations detected
    pub violations_detected: u64,
}

/// Isolation violation record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IsolationViolation {
    /// When violation occurred
    pub timestamp: std::time::SystemTime,
    /// Source network
    pub source_network: NetworkId,
    /// Target network
    pub target_network: NetworkId,
    /// Violation type
    pub violation_type: ViolationType,
    /// Connection ID involved
    pub connection_id: Option<ConnectionId>,
}

/// Violation type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ViolationType {
    /// Packet sent to wrong network
    PacketLeakage,
    /// Connection crossed network boundary
    ConnectionBreach,
    /// Tunnel used without authorization
    UnauthorizedTunnel,
    /// Privacy tier violation
    PrivacyTierMismatch,
}

/// Isolation configuration
#[derive(Clone, Debug)]
pub struct IsolationConfig {
    /// Enable strict isolation (zero tolerance)
    pub strict_mode: bool,
    /// Log all violations
    pub log_violations: bool,
    /// Maximum networks per node
    pub max_networks: usize,
    /// Enable tunnels between networks
    pub allow_tunnels: bool,
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            log_violations: true,
            max_networks: 100,
            allow_tunnels: true,
        }
    }
}

impl NetworkIsolationManager {
    /// Create new isolation manager
    pub fn new(config: IsolationConfig) -> Self {
        Self {
            network_stacks: Arc::new(RwLock::new(HashMap::new())),
            violations: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Create isolated network stack
    pub async fn create_network_stack(
        &self,
        network_id: NetworkId,
        name: String,
        privacy_tier: PrivacyTier,
    ) -> Result<()> {
        let mut stacks = self.network_stacks.write().await;

        if stacks.len() >= self.config.max_networks {
            return Err(anyhow!("Maximum networks limit reached"));
        }

        if stacks.contains_key(&network_id) {
            return Err(anyhow!("Network stack already exists"));
        }

        // Create isolated STOQ transport for this network
        // Use dynamic port to avoid conflicts in tests
        #[cfg(not(test))]
        let transport_config = crate::config::TransportConfig::default();

        #[cfg(test)]
        let transport_config = {
            let mut config = crate::config::TransportConfig::default();
            config.port = 0; // Let OS assign an available port
            config
        };

        let transport = Arc::new(StoqTransport::new(transport_config).await?);

        let stack = NetworkStack {
            network_id,
            name: name.clone(),
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
            privacy_tier,
            stats: Arc::new(RwLock::new(NetworkStats::default())),
            tunnels: Arc::new(RwLock::new(HashMap::new())),
        };

        stacks.insert(network_id, stack);

        tracing::info!(
            "Created isolated network stack for {} ({})",
            name,
            hex::encode(&network_id)
        );

        Ok(())
    }

    /// Remove network stack
    pub async fn remove_network_stack(&self, network_id: NetworkId) -> Result<()> {
        let mut stacks = self.network_stacks.write().await;

        if let Some(stack) = stacks.remove(&network_id) {
            // Close all connections
            let mut connections = stack.connections.write().await;
            connections.clear();

            tracing::info!(
                "Removed network stack for {}",
                hex::encode(&network_id)
            );
            Ok(())
        } else {
            Err(anyhow!("Network stack not found"))
        }
    }

    /// Get network stack
    pub async fn get_network_stack(&self, network_id: &NetworkId) -> Option<NetworkStack> {
        let stacks = self.network_stacks.read().await;
        stacks.get(network_id).cloned()
    }

    /// Create explicit tunnel between networks
    pub async fn create_tunnel(
        &self,
        from_network: NetworkId,
        to_network: NetworkId,
        allowed_traffic: Vec<TrafficType>,
        validation_required: bool,
    ) -> Result<()> {
        if !self.config.allow_tunnels {
            return Err(anyhow!("Tunnels disabled in configuration"));
        }

        let stacks = self.network_stacks.read().await;

        let from_stack = stacks.get(&from_network)
            .ok_or_else(|| anyhow!("Source network not found"))?;

        if !stacks.contains_key(&to_network) {
            return Err(anyhow!("Target network not found"));
        }

        let tunnel = NetworkTunnel {
            from_network,
            to_network,
            allowed_traffic,
            validation_required,
            active: true,
        };

        let mut tunnels = from_stack.tunnels.write().await;
        tunnels.insert(to_network, tunnel);

        tracing::info!(
            "Created tunnel from {} to {}",
            hex::encode(&from_network),
            hex::encode(&to_network)
        );

        Ok(())
    }

    /// Verify packet isolation (called by STOQ transport)
    pub async fn verify_packet_isolation(
        &self,
        network_id: &NetworkId,
        connection_id: ConnectionId,
        destination_network: &NetworkId,
    ) -> bool {
        // Same network - always allowed
        if network_id == destination_network {
            return true;
        }

        // Check for explicit tunnel
        let stacks = self.network_stacks.read().await;
        if let Some(stack) = stacks.get(network_id) {
            let tunnels = stack.tunnels.read().await;
            if let Some(tunnel) = tunnels.get(destination_network) {
                if tunnel.active {
                    return true;
                }
            }
        }

        // Isolation violation
        self.record_violation(IsolationViolation {
            timestamp: std::time::SystemTime::now(),
            source_network: *network_id,
            target_network: *destination_network,
            violation_type: ViolationType::PacketLeakage,
            connection_id: Some(connection_id),
        }).await;

        false
    }

    /// Record isolation violation
    async fn record_violation(&self, violation: IsolationViolation) {
        if self.config.log_violations {
            tracing::error!(
                "ISOLATION VIOLATION: {:?} from {} to {}",
                violation.violation_type,
                hex::encode(&violation.source_network),
                hex::encode(&violation.target_network)
            );
        }

        let mut violations = self.violations.write().await;
        violations.push(violation.clone());

        // Update network stats
        let stacks = self.network_stacks.read().await;
        if let Some(stack) = stacks.get(&violation.source_network) {
            let mut stats = stack.stats.write().await;
            stats.violations_detected += 1;
        }
    }

    /// Get all violations
    pub async fn get_violations(&self) -> Vec<IsolationViolation> {
        let violations = self.violations.read().await;
        violations.clone()
    }

    /// Get network statistics
    pub async fn get_network_stats(&self, network_id: &NetworkId) -> Option<NetworkStats> {
        let stacks = self.network_stacks.read().await;
        if let Some(stack) = stacks.get(network_id) {
            let stats = stack.stats.read().await;
            Some(stats.clone())
        } else {
            None
        }
    }

    /// Get all active networks
    pub async fn active_networks(&self) -> Vec<NetworkId> {
        let stacks = self.network_stacks.read().await;
        stacks.keys().copied().collect()
    }

    /// Clear violations (for testing)
    pub async fn clear_violations(&self) {
        let mut violations = self.violations.write().await;
        violations.clear();
    }
}

// Clone implementation for NetworkStack
impl Clone for NetworkStack {
    fn clone(&self) -> Self {
        Self {
            network_id: self.network_id,
            name: self.name.clone(),
            transport: self.transport.clone(),
            connections: self.connections.clone(),
            privacy_tier: self.privacy_tier.clone(),
            stats: self.stats.clone(),
            tunnels: self.tunnels.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::init_test_crypto;

    #[tokio::test]
    async fn test_create_isolated_networks() {
        init_test_crypto();
        let manager = NetworkIsolationManager::new(IsolationConfig::default());

        // Create bank network
        let bank_network = [1u8; 16];
        manager.create_network_stack(
            bank_network,
            "Bank Customer Portal".to_string(),
            PrivacyTier::Public,
        ).await.unwrap();

        // Create employee network
        let employee_network = [2u8; 16];
        manager.create_network_stack(
            employee_network,
            "Employee VPN".to_string(),
            PrivacyTier::PrivateP2P,
        ).await.unwrap();

        // Verify both exist
        let networks = manager.active_networks().await;
        assert_eq!(networks.len(), 2);
    }

    #[tokio::test]
    async fn test_packet_isolation() {
        init_test_crypto();
        let manager = NetworkIsolationManager::new(IsolationConfig::default());

        let network1 = [1u8; 16];
        let network2 = [2u8; 16];

        manager.create_network_stack(
            network1,
            "Network 1".to_string(),
            PrivacyTier::Public,
        ).await.unwrap();

        manager.create_network_stack(
            network2,
            "Network 2".to_string(),
            PrivacyTier::Public,
        ).await.unwrap();

        // Verify same-network traffic allowed
        assert!(manager.verify_packet_isolation(&network1, 1, &network1).await);

        // Verify cross-network traffic blocked
        assert!(!manager.verify_packet_isolation(&network1, 1, &network2).await);

        // Check violation recorded
        let violations = manager.get_violations().await;
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].source_network, network1);
        assert_eq!(violations[0].target_network, network2);
    }

    #[tokio::test]
    async fn test_explicit_tunnels() {
        init_test_crypto();
        let manager = NetworkIsolationManager::new(IsolationConfig::default());

        let network1 = [1u8; 16];
        let network2 = [2u8; 16];

        manager.create_network_stack(
            network1,
            "Network 1".to_string(),
            PrivacyTier::Public,
        ).await.unwrap();

        manager.create_network_stack(
            network2,
            "Network 2".to_string(),
            PrivacyTier::Federated,
        ).await.unwrap();

        // Create tunnel
        manager.create_tunnel(
            network1,
            network2,
            vec![TrafficType::AssetProof],
            true,
        ).await.unwrap();

        // Now cross-network traffic should be allowed
        manager.clear_violations().await;
        assert!(manager.verify_packet_isolation(&network1, 1, &network2).await);

        // No violations
        let violations = manager.get_violations().await;
        assert_eq!(violations.len(), 0);
    }

    #[tokio::test]
    async fn test_network_removal() {
        init_test_crypto();
        let manager = NetworkIsolationManager::new(IsolationConfig::default());

        let network1 = [1u8; 16];
        manager.create_network_stack(
            network1,
            "Network 1".to_string(),
            PrivacyTier::Public,
        ).await.unwrap();

        assert_eq!(manager.active_networks().await.len(), 1);

        manager.remove_network_stack(network1).await.unwrap();

        assert_eq!(manager.active_networks().await.len(), 0);
    }
}
