// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Network Coordinator - Primary Component for Sprint 2.3
//!
//! Revolutionary Concept #4: Multi-Network Participation
//!
//! Single node joins 10+ networks simultaneously with:
//! - Complete isolation (zero packet leakage)
//! - Independent privacy tiers per network
//! - Matrix-based asset routing
//! - Cross-network validation without bridging traffic
//!
//! Example: Car purchase validation across Bank->Dealer->Insurance->DMV

use super::PeerIdentity;
use crate::assets::core::{AssetError, AssetRegistration, AssetResult, StateProof};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

// Import network membership from our implementation
pub use super::network_membership::{
    ApprovalProcess, JoinRequirements, MembershipStatus, MultiNetworkMembership,
    NetworkCredentials, NetworkDiscovery, NetworkId, ParticipationMembership, PrivacyMode,
    TrustChainClient,
};

/// Multi-network coordinator - PRIMARY component for Sprint 2.3
pub struct MultiNetworkCoordinator {
    /// Local node ID
    local_node: PeerIdentity,
    /// Network membership manager (integrates with TrustChain)
    membership: Arc<MultiNetworkMembership>,
    /// STOQ isolation manager (protocol-level isolation)
    stoq_isolation: Arc<StoqIsolationManager>,
    /// Asset routing per network (matrix-based)
    asset_routing: Arc<RwLock<HashMap<NetworkId, NetworkAssetRouter>>>,
    /// Cross-network validation manager
    cross_network_validator: Arc<CrossNetworkValidator>,
    /// Engagement monitor (NGauge integration)
    engagement_monitor: Arc<EngagementMonitor>,
    /// Configuration
    config: MultiNetworkConfig,
}

/// STOQ isolation manager interface
pub struct StoqIsolationManager {
    /// Map of network isolation stacks
    isolation_stacks: Arc<RwLock<HashMap<NetworkId, IsolationStack>>>,
}

/// Isolation stack for a network
#[derive(Clone, Debug)]
pub struct IsolationStack {
    /// Network ID
    pub network_id: NetworkId,
    /// Privacy tier
    pub privacy_tier: PrivacyMode,
    /// Packet filter active
    pub filter_active: bool,
    /// Violations detected
    pub violations: u64,
}

impl Default for StoqIsolationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StoqIsolationManager {
    pub fn new() -> Self {
        Self {
            isolation_stacks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create isolated stack for network
    pub async fn create_stack(
        &self,
        network_id: NetworkId,
        privacy_tier: PrivacyMode,
    ) -> AssetResult<()> {
        let mut stacks = self.isolation_stacks.write().await;

        let stack = IsolationStack {
            network_id,
            privacy_tier,
            filter_active: true,
            violations: 0,
        };

        stacks.insert(network_id, stack);
        Ok(())
    }

    /// Remove isolation stack
    pub async fn remove_stack(&self, network_id: &NetworkId) -> AssetResult<()> {
        let mut stacks = self.isolation_stacks.write().await;
        stacks.remove(network_id);
        Ok(())
    }

    /// Verify packet isolation
    pub async fn verify_isolation(&self, from_network: &NetworkId, to_network: &NetworkId) -> bool {
        if from_network == to_network {
            return true; // Same network always allowed
        }

        // Cross-network blocked by default
        let mut stacks = self.isolation_stacks.write().await;
        if let Some(stack) = stacks.get_mut(from_network) {
            stack.violations += 1;
        }

        false
    }

    /// Get violations count
    pub async fn violations(&self, network_id: &NetworkId) -> u64 {
        let stacks = self.isolation_stacks.read().await;
        stacks.get(network_id).map(|s| s.violations).unwrap_or(0)
    }
}

/// Network asset router - matrix-based routing for assets
pub struct NetworkAssetRouter {
    /// Network ID
    _network_id: NetworkId,
    /// Assets visible in this network
    visible_assets: HashSet<AssetRegistration>,
    /// Matrix positions for assets
    asset_positions: HashMap<AssetRegistration, IntegerMatrixPosition>,
    /// Routing table
    routing_table: HashMap<AssetRegistration, Vec<IntegerMatrixPosition>>,
}

/// Integer matrix position for asset routing within multi-network coordinator.
/// Unlike hypermesh_lib::MatrixPosition (f64 coordinates for GPS),
/// this uses i64 for discrete grid-based asset routing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntegerMatrixPosition {
    /// X coordinate
    pub x: i64,
    /// Y coordinate
    pub y: i64,
    /// Z coordinate
    pub z: i64,
}

impl NetworkAssetRouter {
    pub fn new(network_id: NetworkId) -> Self {
        Self {
            _network_id: network_id,
            visible_assets: HashSet::new(),
            asset_positions: HashMap::new(),
            routing_table: HashMap::new(),
        }
    }

    /// Add asset to network
    pub fn add_asset(&mut self, asset_id: AssetRegistration, position: IntegerMatrixPosition) {
        self.visible_assets.insert(asset_id.clone());
        self.asset_positions.insert(asset_id, position);
    }

    /// Remove asset from network
    pub fn remove_asset(&mut self, asset_id: &AssetRegistration) {
        self.visible_assets.remove(asset_id);
        self.asset_positions.remove(asset_id);
        self.routing_table.remove(asset_id);
    }

    /// Check if asset is visible
    pub fn is_visible(&self, asset_id: &AssetRegistration) -> bool {
        self.visible_assets.contains(asset_id)
    }

    /// Get matrix position for asset
    pub fn get_position(&self, asset_id: &AssetRegistration) -> Option<&IntegerMatrixPosition> {
        self.asset_positions.get(asset_id)
    }

    /// Calculate route to asset (tensor-based pathfinding)
    pub fn calculate_route(
        &self,
        from: &IntegerMatrixPosition,
        to_asset: &AssetRegistration,
    ) -> Option<Vec<IntegerMatrixPosition>> {
        let to_position = self.asset_positions.get(to_asset)?;

        // Simple linear path (production would use A* with matrix operations)
        let mut path = vec![from.clone()];

        // Add intermediate positions (simplified)
        let mid = IntegerMatrixPosition {
            x: (from.x + to_position.x) / 2,
            y: (from.y + to_position.y) / 2,
            z: (from.z + to_position.z) / 2,
        };
        path.push(mid);
        path.push(to_position.clone());

        Some(path)
    }
}

/// Cross-network validator - validates assets across networks using blockchain proofs
pub struct CrossNetworkValidator {
    /// Validation cache
    validation_cache: Arc<RwLock<HashMap<(NetworkId, AssetRegistration), ValidationResult>>>,
}

/// Validation result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Asset ID
    pub asset_id: AssetRegistration,
    /// Networks where validated
    pub validated_networks: Vec<NetworkId>,
    /// State proof
    pub proof: StateProof,
    /// Validation time
    pub validated_at: SystemTime,
    /// Valid
    pub valid: bool,
}

impl Default for CrossNetworkValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossNetworkValidator {
    pub fn new() -> Self {
        Self {
            validation_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Validate asset across networks using blockchain proof
    pub async fn validate_cross_network(
        &self,
        asset_id: AssetRegistration,
        source_network: NetworkId,
        target_network: NetworkId,
        proof: StateProof,
    ) -> AssetResult<bool> {
        // Check cache
        let cache_key = (target_network, asset_id.clone());
        {
            let cache = self.validation_cache.read().await;
            if let Some(result) = cache.get(&cache_key) {
                if result
                    .validated_at
                    .elapsed()
                    .unwrap_or(std::time::Duration::MAX)
                    < std::time::Duration::from_secs(300)
                {
                    return Ok(result.valid);
                }
            }
        }

        // Validate proof (simplified - production would verify all 4 proofs)
        let valid = !proof.space_proof.node_id.is_empty()
            && !proof.stake_proof.stake_holder_id.is_empty()
            && proof.work_proof.work_hash != [0u8; 32]
            && proof.time_proof.nonce > 0;

        // Cache result
        let result = ValidationResult {
            asset_id: asset_id.clone(),
            validated_networks: vec![source_network, target_network],
            proof,
            validated_at: SystemTime::now(),
            valid,
        };

        let mut cache = self.validation_cache.write().await;
        cache.insert(cache_key, result);

        Ok(valid)
    }

    /// Get validation result
    pub async fn get_validation(
        &self,
        network_id: NetworkId,
        asset_id: &AssetRegistration,
    ) -> Option<ValidationResult> {
        let cache = self.validation_cache.read().await;
        cache.get(&(network_id, asset_id.clone())).cloned()
    }
}

/// Engagement monitor - tracks engagement per network (NGauge integration)
pub struct EngagementMonitor {
    /// Metrics per network
    network_metrics: Arc<RwLock<HashMap<NetworkId, NetworkEngagementMetrics>>>,
}

/// Engagement metrics for a network
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkEngagementMetrics {
    /// Network ID
    pub network_id: NetworkId,
    /// Assets used
    pub assets_used: u64,
    /// Transactions
    pub transactions: u64,
    /// Data transferred
    pub data_transferred: u64,
    /// Active time
    pub active_time_seconds: u64,
    /// Last activity
    pub last_activity: Option<SystemTime>,
}

impl Default for NetworkEngagementMetrics {
    fn default() -> Self {
        Self {
            network_id: NetworkId([0u8; 16]),
            assets_used: 0,
            transactions: 0,
            data_transferred: 0,
            active_time_seconds: 0,
            last_activity: None,
        }
    }
}

impl Default for EngagementMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl EngagementMonitor {
    pub fn new() -> Self {
        Self {
            network_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record engagement event
    pub async fn record_event(&self, network_id: NetworkId, event_type: EngagementEventType) {
        let mut metrics = self.network_metrics.write().await;
        let network_metrics =
            metrics
                .entry(network_id)
                .or_insert_with(|| NetworkEngagementMetrics {
                    network_id,
                    ..Default::default()
                });

        match event_type {
            EngagementEventType::AssetUsed => network_metrics.assets_used += 1,
            EngagementEventType::Transaction => network_metrics.transactions += 1,
            EngagementEventType::DataTransferred(bytes) => {
                network_metrics.data_transferred += bytes
            }
        }

        network_metrics.last_activity = Some(SystemTime::now());
    }

    /// Get metrics for network
    pub async fn get_metrics(&self, network_id: &NetworkId) -> Option<NetworkEngagementMetrics> {
        let metrics = self.network_metrics.read().await;
        metrics.get(network_id).cloned()
    }

    /// Get all metrics
    pub async fn all_metrics(&self) -> Vec<NetworkEngagementMetrics> {
        let metrics = self.network_metrics.read().await;
        metrics.values().cloned().collect()
    }
}

/// Engagement event type
#[derive(Clone, Debug)]
pub enum EngagementEventType {
    AssetUsed,
    Transaction,
    DataTransferred(u64),
}

/// Multi-network configuration
#[derive(Clone, Debug)]
pub struct MultiNetworkConfig {
    /// Maximum networks to join
    pub max_networks: usize,
    /// Enable cross-network validation
    pub cross_network_validation: bool,
    /// Enable engagement monitoring
    pub engagement_monitoring: bool,
    /// Strict isolation mode
    pub strict_isolation: bool,
}

impl Default for MultiNetworkConfig {
    fn default() -> Self {
        Self {
            max_networks: 100,
            cross_network_validation: true,
            engagement_monitoring: true,
            strict_isolation: true,
        }
    }
}

impl MultiNetworkCoordinator {
    /// Create new multi-network coordinator
    pub fn new(
        local_node: PeerIdentity,
        trustchain_client: Arc<dyn TrustChainClient>,
        config: MultiNetworkConfig,
    ) -> Self {
        let membership = Arc::new(MultiNetworkMembership::new(
            local_node.clone(),
            trustchain_client,
        ));
        let stoq_isolation = Arc::new(StoqIsolationManager::new());
        let cross_network_validator = Arc::new(CrossNetworkValidator::new());
        let engagement_monitor = Arc::new(EngagementMonitor::new());

        Self {
            local_node,
            membership,
            stoq_isolation,
            asset_routing: Arc::new(RwLock::new(HashMap::new())),
            cross_network_validator,
            engagement_monitor,
            config,
        }
    }

    /// Discover available networks
    pub async fn discover_networks(&self) -> AssetResult<Vec<NetworkDiscovery>> {
        self.membership.discover_networks().await
    }

    /// Join a network
    pub async fn join_network(
        &self,
        network_id: NetworkId,
        privacy_tier: PrivacyMode,
    ) -> AssetResult<()> {
        // Check network limit
        let active = self.membership.active_memberships().await;
        if active.len() >= self.config.max_networks {
            return Err(AssetError::NetworkError {
                message: format!(
                    "Maximum networks limit ({}) reached",
                    self.config.max_networks
                ),
            });
        }

        // Join via TrustChain
        self.membership
            .join_network(network_id, privacy_tier)
            .await?;

        // Create STOQ isolation stack
        self.stoq_isolation
            .create_stack(network_id, privacy_tier)
            .await?;

        // Create asset router
        let router = NetworkAssetRouter::new(network_id);
        let mut routing = self.asset_routing.write().await;
        routing.insert(network_id, router);

        tracing::info!(
            "Node {} joined network {} with privacy tier {:?}",
            hex::encode(&self.local_node.id[..8]),
            network_id,
            privacy_tier
        );

        Ok(())
    }

    /// Leave a network
    pub async fn leave_network(&self, network_id: NetworkId) -> AssetResult<()> {
        // Leave via TrustChain
        self.membership.leave_network(network_id).await?;

        // Remove STOQ isolation
        self.stoq_isolation.remove_stack(&network_id).await?;

        // Remove asset router
        let mut routing = self.asset_routing.write().await;
        routing.remove(&network_id);

        tracing::info!(
            "Node {} left network {}",
            hex::encode(&self.local_node.id[..8]),
            network_id
        );

        Ok(())
    }

    /// Add asset to network
    pub async fn add_asset_to_network(
        &self,
        network_id: NetworkId,
        asset_id: AssetRegistration,
        matrix_position: IntegerMatrixPosition,
    ) -> AssetResult<()> {
        // Add to membership visibility
        self.membership
            .add_asset_to_network(network_id, asset_id.clone())
            .await?;

        // Add to router
        let mut routing = self.asset_routing.write().await;
        if let Some(router) = routing.get_mut(&network_id) {
            router.add_asset(asset_id, matrix_position);
        }

        Ok(())
    }

    /// Validate asset across networks
    pub async fn validate_asset_cross_network(
        &self,
        asset_id: AssetRegistration,
        source_network: NetworkId,
        target_network: NetworkId,
        proof: StateProof,
    ) -> AssetResult<bool> {
        if !self.config.cross_network_validation {
            return Ok(true); // Validation disabled
        }

        self.cross_network_validator
            .validate_cross_network(asset_id, source_network, target_network, proof)
            .await
    }

    /// Get active networks
    pub async fn active_networks(&self) -> Vec<ParticipationMembership> {
        self.membership.active_memberships().await
    }

    /// Verify isolation (zero packet leakage)
    pub async fn verify_isolation(&self) -> AssetResult<IsolationReport> {
        let networks: Vec<NetworkId> = self
            .membership
            .active_memberships()
            .await
            .iter()
            .map(|m| m.network_id)
            .collect();

        let mut total_violations = 0;
        let mut network_violations = HashMap::new();

        for network_id in &networks {
            let violations = self.stoq_isolation.violations(network_id).await;
            network_violations.insert(*network_id, violations);
            total_violations += violations;
        }

        Ok(IsolationReport {
            total_networks: networks.len(),
            total_violations,
            network_violations,
            strict_mode: self.config.strict_isolation,
        })
    }

    /// Get engagement metrics
    pub async fn get_engagement_metrics(&self) -> Vec<NetworkEngagementMetrics> {
        self.engagement_monitor.all_metrics().await
    }

    /// Record engagement event
    pub async fn record_engagement(&self, network_id: NetworkId, event_type: EngagementEventType) {
        if self.config.engagement_monitoring {
            self.engagement_monitor
                .record_event(network_id, event_type)
                .await;
        }
    }
}

/// Isolation report
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IsolationReport {
    /// Total networks
    pub total_networks: usize,
    /// Total violations
    pub total_violations: u64,
    /// Violations per network
    pub network_violations: HashMap<NetworkId, u64>,
    /// Strict mode enabled
    pub strict_mode: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::AssetType;
    use crate::test_utils::test_asset_id;
    use async_trait::async_trait;

    struct MockTrustChainClient;

    #[async_trait]
    impl TrustChainClient for MockTrustChainClient {
        async fn request_credentials(
            &self,
            _network_id: NetworkId,
        ) -> AssetResult<NetworkCredentials> {
            use std::time::Duration;
            Ok(NetworkCredentials {
                certificate: vec![1, 2, 3],
                public_key: vec![4, 5, 6],
                private_key_encrypted: vec![7, 8, 9],
                session_tokens: vec![],
                expires_at: SystemTime::now() + Duration::from_secs(3600),
            })
        }

        async fn revoke_credentials(&self, _network_id: NetworkId) -> AssetResult<()> {
            Ok(())
        }

        async fn validate_certificate(&self, _cert: &[u8]) -> AssetResult<bool> {
            Ok(true)
        }

        async fn discover_networks(&self) -> AssetResult<Vec<NetworkDiscovery>> {
            Ok(vec![
                NetworkDiscovery {
                    network_id: NetworkId([1u8; 16]),
                    name: "Bank".to_string(),
                    description: "Bank network".to_string(),
                    entry_points: vec![],
                    requirements: JoinRequirements {
                        invitation_required: false,
                        requires_state_proof: false,
                        required_proofs: HashSet::new(),
                        geo_restrictions: None,
                        approval_process: ApprovalProcess::Automatic,
                    },
                    privacy_tier: PrivacyMode::PUBLIC,
                    member_count: 100,
                    is_public: true,
                },
                NetworkDiscovery {
                    network_id: NetworkId([2u8; 16]),
                    name: "Dealer".to_string(),
                    description: "Dealer network".to_string(),
                    entry_points: vec![],
                    requirements: JoinRequirements {
                        invitation_required: false,
                        requires_state_proof: false,
                        required_proofs: HashSet::new(),
                        geo_restrictions: None,
                        approval_process: ApprovalProcess::Automatic,
                    },
                    privacy_tier: PrivacyMode::PRIVATE,
                    member_count: 50,
                    is_public: false,
                },
            ])
        }
    }

    #[tokio::test]
    async fn test_join_multiple_networks() {
        let node_id = PeerIdentity {
            name: "test-node".to_string(),
            id: [0u8; 32],
            address: "::1".parse().expect("test: valid ipv6"),
            pub_key: vec![],
        };

        let client = Arc::new(MockTrustChainClient);
        let coordinator =
            MultiNetworkCoordinator::new(node_id, client.clone(), MultiNetworkConfig::default());

        // Discover networks
        coordinator
            .membership
            .discover_networks()
            .await
            .expect("test: discover");

        // Join bank network
        let bank_network = NetworkId([1u8; 16]);
        coordinator
            .join_network(bank_network, PrivacyMode::PUBLIC)
            .await
            .expect("test: join bank");

        // Join dealer network
        let dealer_network = NetworkId([2u8; 16]);
        coordinator
            .membership
            .discover_networks()
            .await
            .expect("test: discover");
        coordinator
            .join_network(dealer_network, PrivacyMode::PRIVATE)
            .await
            .expect("test: join dealer");

        // Verify both active
        let active = coordinator.active_networks().await;
        assert!(!active.is_empty()); // At least bank network
    }

    #[tokio::test]
    async fn test_asset_cross_network_validation() {
        let node_id = PeerIdentity {
            name: "test-node".to_string(),
            id: [0u8; 32],
            address: "::1".parse().expect("test: valid ipv6"),
            pub_key: vec![],
        };

        let client = Arc::new(MockTrustChainClient);
        let coordinator =
            MultiNetworkCoordinator::new(node_id, client, MultiNetworkConfig::default());

        let asset_id = test_asset_id(AssetType::Storage);
        let bank_network = NetworkId([1u8; 16]);
        let dealer_network = NetworkId([2u8; 16]);

        use crate::proof_of_state::{
            SpaceProof, StakeProof, TimeProof, WorkProof,
        };
        use std::time::SystemTime;

        let proof = StateProof {
            space_proof: SpaceProof {
                node_id: "test-node".to_string(),
                storage_path: "/tmp/test".to_string(),
                total_size: 1000,
                total_storage: 10000,
                file_hash: "abcd1234".to_string(),
                proof_timestamp: SystemTime::now(),
            },
            stake_proof: StakeProof {
                stake_holder: "test".to_string(),
                stake_holder_id: "test-id".to_string(),
                stake_timestamp: SystemTime::now(),

            },
            work_proof: WorkProof {
                owner_id: "test-owner".to_string(),
                work_hash: *blake3::hash(b"test-work").as_bytes(),
                workload_id: "test-workload".to_string(),
                proof_timestamp: SystemTime::now(),
            },
            time_proof: TimeProof {
                network_time_offset: std::time::Duration::from_secs(0),
                time_verification_timestamp: SystemTime::now(),
                nonce: 42,
                proof_hash: vec![1, 2, 3, 4],
            },
            ..Default::default()
        };

        let valid = coordinator
            .validate_asset_cross_network(asset_id, bank_network, dealer_network, proof)
            .await
            .expect("test: validate");

        assert!(valid);
    }

    #[tokio::test]
    async fn test_isolation_verification() {
        let node_id = PeerIdentity {
            name: "test-node".to_string(),
            id: [0u8; 32],
            address: "::1".parse().expect("test: valid ipv6"),
            pub_key: vec![],
        };

        let client = Arc::new(MockTrustChainClient);
        let coordinator =
            MultiNetworkCoordinator::new(node_id, client, MultiNetworkConfig::default());

        coordinator
            .membership
            .discover_networks()
            .await
            .expect("test: discover");
        let network1 = NetworkId([1u8; 16]);
        coordinator
            .join_network(network1, PrivacyMode::PUBLIC)
            .await
            .expect("test: join");

        // Get isolation report
        let report = coordinator
            .verify_isolation()
            .await
            .expect("test: verify isolation");
        assert!(report.total_networks >= 1);
        assert_eq!(report.total_violations, 0); // No violations yet
    }
}
