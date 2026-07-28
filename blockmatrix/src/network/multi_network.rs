// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Network Coordinator
//!
//! Central coordinator for multi-network participation, enabling a single node
//! to simultaneously connect to multiple network types with complete isolation.
//!
//! CRITICAL: Networks CANNOT transition between types. Only independent
//! connect/disconnect operations are allowed. Each network maintains its type
//! throughout its lifetime.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::isolation::{DefaultIsolationManager, IsolationManager};
use super::trust::{
    AnonymousNetworkHandler, AssetResponse, FederatedNetworkHandler,
    NetworkConfig as TrustNetworkConfig, NetworkConnection, NetworkHandler, NetworkId, NetworkType,
    P2PNetworkHandler, PublicNetworkHandler, StateProof,
};
use crate::assets::core::AssetRegistration;

/// Configuration for joining a network
#[derive(Debug, Clone)]
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
}

impl NetworkConfig {
    pub fn anonymous() -> Self {
        Self {
            peer_addresses: vec![],
            federation_gateway: None,
            dns_name: None,
            proof_of_state: None,
            stoq_port: None,
        }
    }

    pub fn p2p(peers: Vec<String>) -> Self {
        Self {
            peer_addresses: peers,
            federation_gateway: None,
            dns_name: None,
            proof_of_state: None,
            stoq_port: None,
        }
    }

    pub fn federated(gateway: String) -> Self {
        Self {
            peer_addresses: vec![],
            federation_gateway: Some(gateway),
            dns_name: None,
            proof_of_state: None,
            stoq_port: None,
        }
    }

    pub fn public(dns_name: String, proof: StateProof) -> Self {
        Self {
            peer_addresses: vec![],
            federation_gateway: None,
            dns_name: Some(dns_name),
            proof_of_state: Some(proof),
            stoq_port: None,
        }
    }

    /// Convert to trust module's NetworkConfig
    fn to_trust_config(&self, network_type: NetworkType) -> TrustNetworkConfig {
        TrustNetworkConfig {
            network_type,
            peer_addresses: self.peer_addresses.clone(),
            federation_gateway: self.federation_gateway.clone(),
            dns_name: self.dns_name.clone(),
            proof_of_state: self.proof_of_state.clone(),
        }
    }
}

/// Asset visibility control
pub struct AssetVisibilityControl {
    /// Asset ID -> Allowed network IDs
    visibility_map: HashMap<AssetRegistration, Vec<NetworkId>>,

    /// Default visibility policy for new assets
    default_policy: VisibilityPolicy,
}

impl Default for AssetVisibilityControl {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetVisibilityControl {
    pub fn new() -> Self {
        Self {
            visibility_map: HashMap::new(),
            default_policy: VisibilityPolicy::Private, // Private by default
        }
    }

    pub fn set_visibility(&mut self, asset_id: AssetRegistration, networks: Vec<NetworkId>) {
        self.visibility_map.insert(asset_id, networks);
    }

    pub fn is_visible_to(&self, asset_id: &AssetRegistration, network_id: &NetworkId) -> bool {
        self.visibility_map
            .get(asset_id)
            .map(|networks| networks.contains(network_id))
            .unwrap_or(false)
    }

    pub fn get_visible_networks(&self, asset_id: &AssetRegistration) -> Vec<NetworkId> {
        self.visibility_map
            .get(asset_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn remove_visibility(&mut self, asset_id: &AssetRegistration) {
        self.visibility_map.remove(asset_id);
    }

    pub fn set_default_policy(&mut self, policy: VisibilityPolicy) {
        self.default_policy = policy;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VisibilityPolicy {
    Private,     // Not visible by default
    AllNetworks, // Visible to all connected networks
    Explicit,    // Must be explicitly configured
}

/// Handler-based coordinator for isolated network *connections*.
///
/// Distinct concern from `assets::multi_node::MultiNetworkCoordinator`
/// (the live participation coordinator: membership + asset routing +
/// cross-network validation + engagement). This type bootstraps
/// `NetworkConnection`s through per-type `NetworkHandler`s and enforces
/// packet isolation. The two previously collided on the name
/// `MultiNetworkCoordinator`; this one is renamed to reflect its actual
/// responsibility.
///
/// NOTE: currently unreferenced outside this module's own tests — a
/// candidate for removal in a dead-code pass (deferred here to preserve
/// the exact `blockmatrix --lib` test baseline).
pub struct NetworkConnectionCoordinator {
    /// Network handlers by type
    handlers: HashMap<NetworkType, Arc<dyn NetworkHandler>>,

    /// Active network connections (isolated)
    connections: Arc<RwLock<HashMap<NetworkId, NetworkConnection>>>,

    /// Isolation manager prevents cross-network leakage
    isolation: Arc<dyn IsolationManager>,

    /// Asset visibility controller
    asset_visibility: Arc<RwLock<AssetVisibilityControl>>,
}

impl NetworkConnectionCoordinator {
    pub fn new(isolation: Arc<dyn IsolationManager>) -> Self {
        let mut handlers = HashMap::new();

        // Register all 4 network type handlers
        handlers.insert(
            NetworkType::Anonymous,
            Arc::new(AnonymousNetworkHandler::new()) as Arc<dyn NetworkHandler>,
        );

        // P2P handler will be created on-demand with peer addresses
        // Federated handler will be created with gateway URL
        // Public handler uses standard configuration
        handlers.insert(
            NetworkType::Public,
            Arc::new(PublicNetworkHandler::new()) as Arc<dyn NetworkHandler>,
        );

        Self {
            handlers,
            connections: Arc::new(RwLock::new(HashMap::new())),
            isolation,
            asset_visibility: Arc::new(RwLock::new(AssetVisibilityControl::new())),
        }
    }

    /// Create with default isolation manager
    pub fn new_default() -> Self {
        Self::new(Arc::new(DefaultIsolationManager::new()))
    }

    /// Join a network with specified configuration
    ///
    /// CRITICAL: Once joined, a network's type CANNOT be changed. Networks are immutable.
    /// To change network type, you must leave_network() and join_network() with new type.
    pub async fn join_network(
        &mut self,
        mut network_type: NetworkType,
        config: NetworkConfig,
    ) -> Result<NetworkId> {
        // Create handler for P2P and Federated on-demand
        match &network_type {
            NetworkType::P2P => {
                if !self.handlers.contains_key(&network_type) {
                    let handler = P2PNetworkHandler::new();
                    self.handlers.insert(
                        NetworkType::P2P,
                        Arc::new(handler) as Arc<dyn NetworkHandler>,
                    );
                }
            }
            NetworkType::Federated {
                gateway_url: _gateway_url,
            } => {
                // Update network_type with gateway from config if needed
                if config.federation_gateway.is_some() {
                    network_type = NetworkType::Federated {
                        gateway_url: config.federation_gateway.clone().expect("federation gateway checked above"),
                    };
                }

                // Create new handler for this specific federation
                let handler = FederatedNetworkHandler::new();
                self.handlers.insert(
                    network_type.clone(),
                    Arc::new(handler) as Arc<dyn NetworkHandler>,
                );
            }
            _ => {}
        }

        // Get appropriate handler
        let handler = self
            .handlers
            .get(&network_type)
            .ok_or_else(|| anyhow!("Unknown network type: {network_type:?}"))?;

        // Convert to trust config
        let trust_config = config.to_trust_config(network_type.clone());

        // Bootstrap with network-specific logic
        let connection = handler.bootstrap(trust_config).await?;

        // Store isolated connection
        let network_id = connection.network_id;
        self.connections
            .write()
            .await
            .insert(network_id, connection);

        // Configure isolation for this network
        self.isolation
            .configure_network(network_id, network_type.clone())
            .await?;

        info!("Joined network: {:?} with ID: {}", network_type, network_id);
        Ok(network_id)
    }

    /// Leave a network gracefully
    pub async fn leave_network(&self, network_id: NetworkId) -> Result<()> {
        // Remove connection
        let mut connections = self.connections.write().await;
        let connection = connections
            .remove(&network_id)
            .ok_or_else(|| anyhow!("Network not found: {network_id}"))?;

        // Get handler for disconnection
        let handler = self.handlers.get(&connection.network_type).ok_or_else(|| {
            anyhow!(
                "Handler not found for network type: {:?}",
                connection.network_type
            )
        })?;

        // Disconnect gracefully
        handler.disconnect().await?;

        // Remove isolation configuration
        self.isolation.remove_network(network_id).await?;

        info!("Left network: {}", network_id);
        Ok(())
    }

    /// Get list of active networks
    pub async fn active_networks(&self) -> Vec<NetworkId> {
        self.connections.read().await.keys().cloned().collect()
    }

    /// Check if connected to specific network
    pub async fn is_connected(&self, network_id: NetworkId) -> bool {
        self.connections.read().await.contains_key(&network_id)
    }

    /// Get network type for a connected network
    pub async fn get_network_type(&self, network_id: &NetworkId) -> Option<NetworkType> {
        self.connections
            .read()
            .await
            .get(network_id)
            .map(|conn| conn.network_type.clone())
    }

    /// Set asset visibility for specific networks
    pub async fn set_asset_visibility(
        &self,
        asset_id: AssetRegistration,
        networks: Vec<NetworkId>,
    ) -> Result<()> {
        // Verify all networks are connected
        let connections = self.connections.read().await;
        for network_id in &networks {
            if !connections.contains_key(network_id) {
                return Err(anyhow!("Network {network_id} not connected"));
            }
        }

        self.asset_visibility
            .write()
            .await
            .set_visibility(asset_id, networks);
        Ok(())
    }

    /// Handle asset request with network-specific authorization
    ///
    /// Authorization is determined by the coordinator's visibility controls.
    /// If `set_asset_visibility` granted access to this network, the request
    /// is authorized. Handler-level peer validation is a separate concern
    /// for actual peer-to-peer data transfer.
    pub async fn handle_asset_request(
        &self,
        network_id: NetworkId,
        asset_id: AssetRegistration,
    ) -> Result<AssetResponse> {
        // Check if asset is visible to this network
        let visibility = self.asset_visibility.read().await;
        if !visibility.is_visible_to(&asset_id, &network_id) {
            return Ok(AssetResponse {
                asset_id: format!("{asset_id:?}"),
                data: None,
                authorized: false,
                metadata: HashMap::from([(
                    "error".to_string(),
                    "Asset not visible to network".to_string(),
                )]),
            });
        }

        // Get network connection to populate metadata
        let connections = self.connections.read().await;
        let connection = connections
            .get(&network_id)
            .ok_or_else(|| anyhow!("Network not connected"))?;

        // Visibility confirmed -- asset is authorized for this network
        Ok(AssetResponse {
            asset_id: format!("{asset_id:?}"),
            data: None,
            authorized: true,
            metadata: HashMap::from([(
                "network".to_string(),
                connection.network_type.name().to_string(),
            )]),
        })
    }

    /// Get statistics about connected networks
    pub async fn get_network_stats(&self) -> NetworkStats {
        let connections = self.connections.read().await;
        let mut stats = NetworkStats::default();

        for connection in connections.values() {
            match connection.network_type {
                NetworkType::Anonymous => stats.anonymous_count += 1,
                NetworkType::P2P => stats.p2p_count += 1,
                NetworkType::Federated { .. } => stats.federated_count += 1,
                NetworkType::Public => stats.public_count += 1,
            }
        }

        stats.total_networks = connections.len();
        stats
    }

    /// List all connected networks with their types
    pub async fn list_networks(&self) -> Vec<(NetworkId, NetworkType)> {
        self.connections
            .read()
            .await
            .iter()
            .map(|(id, conn)| (*id, conn.network_type.clone()))
            .collect()
    }

    /// Verify network type cannot be changed (enforcement guard)
    ///
    /// This method exists to enforce the architectural constraint that networks
    /// are immutable once created. Attempting to change network type will fail.
    pub async fn verify_network_immutability(&self, network_id: &NetworkId) -> Result<()> {
        let connections = self.connections.read().await;
        if connections.contains_key(network_id) {
            return Err(anyhow!(
                "Network {network_id} already exists. Networks are immutable. \
                Use leave_network() then join_network() to change type."
            ));
        }
        Ok(())
    }
}

/// Network statistics
#[derive(Debug, Default, Clone)]
pub struct NetworkStats {
    pub total_networks: usize,
    pub anonymous_count: usize,
    pub p2p_count: usize,
    pub federated_count: usize,
    pub public_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::{AssetCategory, AssetData, BaseSystemType, NetworkScope};
    fn create_test_asset_id() -> AssetRegistration {
        let asset_data = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };

        AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Storage),
        )
    }

    #[tokio::test]
    async fn test_join_multiple_networks() {
        let mut coordinator = NetworkConnectionCoordinator::new_default();

        // Join Anonymous network
        let anon_id = coordinator
            .join_network(NetworkType::Anonymous, NetworkConfig::anonymous())
            .await
            .expect("test: expected success");

        // Join Public network
        let pub_id = coordinator
            .join_network(
                NetworkType::Public,
                NetworkConfig::public(
                    "test.node".to_string(),
                    StateProof::default(),
                ),
            )
            .await
            .expect("test: expected success");

        // Verify both networks are active
        let active = coordinator.active_networks().await;
        assert_eq!(active.len(), 2);
        assert!(active.contains(&anon_id));
        assert!(active.contains(&pub_id));

        // Check network types
        assert_eq!(
            coordinator.get_network_type(&anon_id).await.expect("test: async operation"),
            NetworkType::Anonymous
        );
        assert_eq!(
            coordinator.get_network_type(&pub_id).await.expect("test: async operation"),
            NetworkType::Public
        );
    }

    #[tokio::test]
    async fn test_asset_visibility() {
        let mut coordinator = NetworkConnectionCoordinator::new_default();

        // Join two networks
        let network1 = coordinator
            .join_network(NetworkType::Anonymous, NetworkConfig::anonymous())
            .await
            .expect("test: expected success");

        let network2 = coordinator
            .join_network(
                NetworkType::Public,
                NetworkConfig::public(
                    "test.node".to_string(),
                    StateProof::default(),
                ),
            )
            .await
            .expect("test: expected success");

        // Create test asset
        let asset_id = create_test_asset_id();

        // Set visibility to only network1
        coordinator
            .set_asset_visibility(asset_id.clone(), vec![network1])
            .await
            .expect("test: expected success");

        // Test access from network1 (should be authorized)
        let response1 = coordinator
            .handle_asset_request(network1, asset_id.clone())
            .await
            .expect("test: expected success");
        assert!(response1.authorized);

        // Test access from network2 (should be denied)
        let response2 = coordinator
            .handle_asset_request(network2, asset_id)
            .await
            .expect("test: expected success");
        assert!(!response2.authorized);
    }

    #[tokio::test]
    async fn test_leave_network() {
        let mut coordinator = NetworkConnectionCoordinator::new_default();

        // Join network
        let network_id = coordinator
            .join_network(NetworkType::Anonymous, NetworkConfig::anonymous())
            .await
            .expect("test: expected success");

        // Verify it's connected
        assert!(coordinator.is_connected(network_id).await);

        // Leave network
        coordinator.leave_network(network_id).await.expect("test: async operation");

        // Verify it's disconnected
        assert!(!coordinator.is_connected(network_id).await);
    }

    #[tokio::test]
    async fn test_network_stats() {
        let mut coordinator = NetworkConnectionCoordinator::new_default();

        // Join various networks
        coordinator
            .join_network(NetworkType::Anonymous, NetworkConfig::anonymous())
            .await
            .expect("test: expected success");

        coordinator
            .join_network(NetworkType::Anonymous, NetworkConfig::anonymous())
            .await
            .expect("test: expected success");

        coordinator
            .join_network(
                NetworkType::Public,
                NetworkConfig::public(
                    "test.node".to_string(),
                    StateProof::default(),
                ),
            )
            .await
            .expect("test: expected success");

        // Get stats
        let stats = coordinator.get_network_stats().await;
        assert_eq!(stats.total_networks, 3);
        assert_eq!(stats.anonymous_count, 2);
        assert_eq!(stats.public_count, 1);
        assert_eq!(stats.p2p_count, 0);
        assert_eq!(stats.federated_count, 0);
    }
}
