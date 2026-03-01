// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-path QUIC connection manager.
//!
//! Manages multiple independent QUIC connections as logical "paths"
//! within a single multi-path connection. Each path has its own scope,
//! privacy mode, network membership, and health tracking. The policy
//! engine validates all path additions and sends against scope,
//! federation, and privacy constraints.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use parking_lot::RwLock;
use tracing::{debug, info, warn};

use hypermesh_lib::{BlockchainScope, NetworkId, PrivacyMode};

use crate::network_isolation::NetworkIsolationManager;
use crate::protocol::pos_fast_validator::PosFastValidator;
use crate::transport::connection::Endpoint;

use super::policy::{
    PathPolicy, PathRejectionReason, PathValidation, PosValidationLevel, SendContext,
};
use super::scheduler::{PathCandidate, PathScheduler, PathSelector};

/// Information about a single QUIC path within a multi-path connection.
pub struct PathInfo {
    /// Unique identifier for this path.
    pub path_id: u32,
    /// Remote endpoint this path connects to.
    pub remote_endpoint: Endpoint,
    /// Blockchain scope of this path.
    pub scope: BlockchainScope,
    /// Privacy mode governing this path's behavior.
    pub privacy_mode: PrivacyMode,
    /// Network this path belongs to.
    pub network_id: NetworkId,
    /// Federation chain (nesting hierarchy) for this path.
    pub federation_chain: Vec<NetworkId>,
    /// Gateway node for cross-scope transfers (if any).
    pub gateway_node: Option<String>,
    /// Whether this path connects to a remote node.
    pub is_remote: bool,
    /// Health score stored as f64 bits (use `f64::to_bits`/`from_bits`).
    pub health_score: AtomicU64,
    /// Total bytes sent on this path.
    pub bytes_sent: AtomicU64,
    /// Total bytes received on this path.
    pub bytes_received: AtomicU64,
    /// When this path was created.
    pub created_at: Instant,
    /// Last time this path was active.
    pub last_active: RwLock<Instant>,
}

/// A safe, cloneable snapshot of path information.
#[derive(Debug, Clone)]
pub struct PathSnapshot {
    /// Path identifier.
    pub path_id: u32,
    /// Blockchain scope.
    pub scope: BlockchainScope,
    /// Privacy mode.
    pub privacy_mode: PrivacyMode,
    /// Network membership.
    pub network_id: NetworkId,
    /// Federation chain.
    pub federation_chain: Vec<NetworkId>,
    /// Gateway node (if any).
    pub gateway_node: Option<String>,
    /// Whether the path is remote.
    pub is_remote: bool,
    /// Current health score.
    pub health_score: f64,
    /// Bytes sent.
    pub bytes_sent: u64,
    /// Bytes received.
    pub bytes_received: u64,
    /// Creation timestamp.
    pub created_at: Instant,
}

/// Aggregate metrics for a multi-path connection.
pub struct MultiPathMetrics {
    /// Total bytes sent across all paths.
    pub total_bytes_sent: AtomicU64,
    /// Total bytes received across all paths.
    pub total_bytes_received: AtomicU64,
    /// Number of paths added over the connection lifetime.
    pub paths_added: AtomicU64,
    /// Number of paths removed over the connection lifetime.
    pub paths_removed: AtomicU64,
    /// Number of path additions rejected by policy.
    pub policy_rejections: AtomicU64,
    /// Number of send operations completed.
    pub sends_completed: AtomicU64,
}

impl MultiPathMetrics {
    fn new() -> Self {
        Self {
            total_bytes_sent: AtomicU64::new(0),
            total_bytes_received: AtomicU64::new(0),
            paths_added: AtomicU64::new(0),
            paths_removed: AtomicU64::new(0),
            policy_rejections: AtomicU64::new(0),
            sends_completed: AtomicU64::new(0),
        }
    }
}

/// Multi-path QUIC connection manager.
///
/// Wraps multiple independent QUIC connections as logical paths,
/// enforcing scope/privacy/federation policy on every path addition
/// and cross-network send. Uses a pluggable scheduler to distribute
/// traffic across paths.
pub struct MultiPathConnection {
    peer_id: String,
    paths: Arc<DashMap<u32, PathInfo>>,
    next_path_id: AtomicU32,
    policy: Arc<PathPolicy>,
    selector: Arc<RwLock<PathSelector>>,
    _pos_fast_validator: Option<Arc<PosFastValidator>>,
    _isolation_manager: Option<Arc<NetworkIsolationManager>>,
    metrics: Arc<MultiPathMetrics>,
}

impl MultiPathConnection {
    /// Create a new multi-path connection for the given peer.
    pub fn new(peer_id: String, policy: PathPolicy) -> Self {
        info!("Creating multi-path connection for peer {}", peer_id);
        Self {
            peer_id,
            paths: Arc::new(DashMap::new()),
            next_path_id: AtomicU32::new(0),
            policy: Arc::new(policy),
            selector: Arc::new(RwLock::new(PathSelector::new(PathScheduler::RoundRobin))),
            _pos_fast_validator: None,
            _isolation_manager: None,
            metrics: Arc::new(MultiPathMetrics::new()),
        }
    }

    /// Attach a PoS fast validator for privacy-tier-aware validation.
    pub fn with_pos_validator(mut self, validator: Arc<PosFastValidator>) -> Self {
        self._pos_fast_validator = Some(validator);
        self
    }

    /// Attach a network isolation manager for tunnel verification.
    pub fn with_isolation_manager(mut self, manager: Arc<NetworkIsolationManager>) -> Self {
        self._isolation_manager = Some(manager);
        self
    }

    /// Set the path scheduling strategy.
    pub fn with_scheduler(self, scheduler: PathScheduler) -> Self {
        *self.selector.write() = PathSelector::new(scheduler);
        self
    }

    /// Add a new path to the multi-path connection.
    ///
    /// Validates the path against the policy engine before accepting it.
    /// Returns the assigned path ID on success, or a rejection reason.
    pub fn add_path(
        &self,
        endpoint: Endpoint,
        scope: BlockchainScope,
        privacy_mode: PrivacyMode,
        network_id: NetworkId,
        federation_chain: Vec<NetworkId>,
        gateway_node: Option<String>,
        is_remote: bool,
    ) -> Result<u32, PathRejectionReason> {
        let current_count = self.paths.len();
        let network_count = self.count_paths_for_network(&network_id);

        // Policy validation
        let validation = self.policy.validate_path(
            &scope,
            &privacy_mode,
            &network_id,
            &federation_chain,
            &gateway_node,
            is_remote,
            current_count,
            network_count,
        );

        if let PathValidation::Rejected(reason) = validation {
            warn!("Path rejected for peer {}: {}", self.peer_id, reason);
            self.metrics
                .policy_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(reason);
        }

        // Check PoS validation level requirement
        let pos_level = self.policy.requires_pos_validation(&privacy_mode);
        match pos_level {
            PosValidationLevel::Full => {
                debug!(
                    "Path for peer {} requires full PoS validation (deferred to data flow)",
                    self.peer_id
                );
            }
            PosValidationLevel::Partial => {
                debug!(
                    "Path for peer {} requires partial PoS validation (deferred to data flow)",
                    self.peer_id
                );
            }
            PosValidationLevel::None => {
                debug!(
                    "Path for peer {} skips PoS validation (anonymous mode)",
                    self.peer_id
                );
            }
        }

        // Assign path ID and insert
        let path_id = self.next_path_id.fetch_add(1, Ordering::Relaxed);
        let now = Instant::now();

        let path_info = PathInfo {
            path_id,
            remote_endpoint: endpoint,
            scope,
            privacy_mode,
            network_id,
            federation_chain,
            gateway_node,
            is_remote,
            health_score: AtomicU64::new(1.0_f64.to_bits()),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            created_at: now,
            last_active: RwLock::new(now),
        };

        self.paths.insert(path_id, path_info);
        self.metrics.paths_added.fetch_add(1, Ordering::Relaxed);

        info!(
            "Added path {} for peer {} (scope={:?}, network={}, remote={})",
            path_id, self.peer_id, scope, network_id, is_remote
        );

        Ok(path_id)
    }

    /// Remove a path from the connection.
    ///
    /// Returns `true` if the path existed and was removed.
    pub fn remove_path(&self, path_id: u32) -> bool {
        if self.paths.remove(&path_id).is_some() {
            self.metrics.paths_removed.fetch_add(1, Ordering::Relaxed);
            debug!("Removed path {} for peer {}", path_id, self.peer_id);
            true
        } else {
            false
        }
    }

    /// Total number of active paths.
    pub fn path_count(&self) -> usize {
        self.paths.len()
    }

    /// List path IDs belonging to the given network.
    pub fn paths_for_network(&self, network_id: &NetworkId) -> Vec<u32> {
        self.paths
            .iter()
            .filter(|entry| entry.value().network_id == *network_id)
            .map(|entry| *entry.key())
            .collect()
    }

    /// List path IDs matching the given blockchain scope.
    pub fn paths_for_scope(&self, scope: &BlockchainScope) -> Vec<u32> {
        self.paths
            .iter()
            .filter(|entry| entry.value().scope == *scope)
            .map(|entry| *entry.key())
            .collect()
    }

    /// Get a snapshot of a specific path (non-reference, safe to use).
    pub fn get_path(&self, path_id: u32) -> Option<PathSnapshot> {
        self.paths.get(&path_id).map(|entry| {
            let info = entry.value();
            PathSnapshot {
                path_id: info.path_id,
                scope: info.scope,
                privacy_mode: info.privacy_mode,
                network_id: info.network_id,
                federation_chain: info.federation_chain.clone(),
                gateway_node: info.gateway_node.clone(),
                is_remote: info.is_remote,
                health_score: f64::from_bits(info.health_score.load(Ordering::Relaxed)),
                bytes_sent: info.bytes_sent.load(Ordering::Relaxed),
                bytes_received: info.bytes_received.load(Ordering::Relaxed),
                created_at: info.created_at,
            }
        })
    }

    /// Select path(s) for sending, optionally filtered by target network.
    ///
    /// Returns the selected path ID(s) based on the current scheduling
    /// strategy. Returns an error if no suitable paths are available.
    pub fn select_path(
        &self,
        target_network: Option<&NetworkId>,
    ) -> Result<Vec<u32>, PathRejectionReason> {
        let candidates: Vec<PathCandidate> = self
            .paths
            .iter()
            .filter(|entry| {
                if let Some(net) = target_network {
                    entry.value().network_id == *net
                } else {
                    true
                }
            })
            .map(|entry| {
                let info = entry.value();
                let health = f64::from_bits(info.health_score.load(Ordering::Relaxed));
                let sent = info.bytes_sent.load(Ordering::Relaxed);
                // Estimate bandwidth from bytes sent (heuristic:
                // assume 1 second of data for simplicity).
                let bw_estimate = if sent > 0 {
                    (sent as f64) * 8.0
                } else {
                    1_000_000_000.0 // Default 1 Gbps
                };

                PathCandidate {
                    path_id: info.path_id,
                    bandwidth_estimate_bps: bw_estimate,
                    rtt_ms: 10.0, // Default; would come from AdaptiveConnection
                    health_score: health,
                    bytes_sent: sent,
                }
            })
            .collect();

        let selector = self.selector.read();
        let selected = selector.select_all(&candidates);

        if selected.is_empty() {
            return Err(PathRejectionReason::MaxPathsExceeded);
        }

        Ok(selected)
    }

    /// Record bytes sent on a specific path.
    pub fn record_send(&self, path_id: u32, bytes: u64) {
        if let Some(entry) = self.paths.get(&path_id) {
            entry.value().bytes_sent.fetch_add(bytes, Ordering::Relaxed);
            *entry.value().last_active.write() = Instant::now();
        }
        self.metrics
            .total_bytes_sent
            .fetch_add(bytes, Ordering::Relaxed);
        self.metrics.sends_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record bytes received on a specific path.
    pub fn record_recv(&self, path_id: u32, bytes: u64) {
        if let Some(entry) = self.paths.get(&path_id) {
            entry
                .value()
                .bytes_received
                .fetch_add(bytes, Ordering::Relaxed);
            *entry.value().last_active.write() = Instant::now();
        }
        self.metrics
            .total_bytes_received
            .fetch_add(bytes, Ordering::Relaxed);
    }

    /// Update the health score for a path.
    pub fn update_health(&self, path_id: u32, score: f64) {
        let clamped = score.clamp(0.0, 1.0);
        if let Some(entry) = self.paths.get(&path_id) {
            entry
                .value()
                .health_score
                .store(clamped.to_bits(), Ordering::Relaxed);
            debug!("Updated health for path {}: {:.2}", path_id, clamped);
        }
    }

    /// Validate whether sending from a specific path to a target
    /// network is permitted by the federation/scope policy.
    pub fn validate_send_to_network(
        &self,
        from_path_id: u32,
        target_network: &NetworkId,
    ) -> PathValidation {
        let path = match self.paths.get(&from_path_id) {
            Some(entry) => entry,
            None => {
                return PathValidation::Rejected(PathRejectionReason::TunnelNotConfigured {
                    from: NetworkId([0u8; 16]),
                    to: *target_network,
                });
            }
        };

        let ctx = SendContext {
            network_id: path.value().network_id,
            scope: path.value().scope,
            federation_chain: path.value().federation_chain.clone(),
        };

        self.policy.validate_send(&ctx, target_network)
    }

    /// Get the aggregate metrics for this multi-path connection.
    pub fn metrics(&self) -> &MultiPathMetrics {
        &self.metrics
    }

    /// Get the peer identifier.
    pub fn peer_id(&self) -> &str {
        &self.peer_id
    }

    /// List all active path IDs.
    pub fn active_paths(&self) -> Vec<u32> {
        self.paths.iter().map(|entry| *entry.key()).collect()
    }

    /// Count paths belonging to a specific network.
    fn count_paths_for_network(&self, network_id: &NetworkId) -> usize {
        self.paths
            .iter()
            .filter(|entry| entry.value().network_id == *network_id)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    fn test_endpoint() -> Endpoint {
        Endpoint::new(Ipv6Addr::LOCALHOST, 9292)
    }

    fn network(id: u8) -> NetworkId {
        NetworkId([id; 16])
    }

    #[test]
    fn test_add_remove_paths() {
        let conn = MultiPathConnection::new("peer-1".to_string(), PathPolicy::default());

        let p1 = conn
            .add_path(
                test_endpoint(),
                BlockchainScope::Network,
                PrivacyMode::PUBLIC,
                network(1),
                vec![],
                None,
                false,
            )
            .expect("test: path 1 should be added");

        let p2 = conn
            .add_path(
                test_endpoint(),
                BlockchainScope::Network,
                PrivacyMode::PUBLIC,
                network(2),
                vec![],
                None,
                false,
            )
            .expect("test: path 2 should be added");

        let p3 = conn
            .add_path(
                test_endpoint(),
                BlockchainScope::Network,
                PrivacyMode::PRIVATE,
                network(3),
                vec![],
                None,
                false,
            )
            .expect("test: path 3 should be added");

        assert_eq!(conn.path_count(), 3);

        // Remove one path
        assert!(conn.remove_path(p2));
        assert_eq!(conn.path_count(), 2);

        // Verify remaining paths
        assert!(conn.get_path(p1).is_some());
        assert!(conn.get_path(p2).is_none());
        assert!(conn.get_path(p3).is_some());

        // Verify metrics
        assert_eq!(conn.metrics().paths_added.load(Ordering::Relaxed), 3);
        assert_eq!(conn.metrics().paths_removed.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_policy_rejection_propagated() {
        let conn = MultiPathConnection::new("peer-2".to_string(), PathPolicy::default());

        // Device scope + remote should be rejected by default policy
        let result = conn.add_path(
            test_endpoint(),
            BlockchainScope::Device,
            PrivacyMode::PUBLIC,
            network(1),
            vec![],
            None,
            true, // remote
        );

        assert!(result.is_err());
        if let Err(reason) = result {
            assert!(
                matches!(reason, PathRejectionReason::DeviceScopeRemoteNotAllowed),
                "Expected DeviceScopeRemoteNotAllowed, got: {reason}"
            );
        }

        // Verify rejection was counted
        assert_eq!(conn.metrics().policy_rejections.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_paths_for_network() {
        let conn = MultiPathConnection::new("peer-3".to_string(), PathPolicy::default());

        let net_a = network(1);
        let net_b = network(2);

        // Add 2 paths to network A
        conn.add_path(
            test_endpoint(),
            BlockchainScope::Network,
            PrivacyMode::PUBLIC,
            net_a,
            vec![],
            None,
            false,
        )
        .expect("test: add path to net A");

        conn.add_path(
            test_endpoint(),
            BlockchainScope::Network,
            PrivacyMode::PUBLIC,
            net_a,
            vec![],
            None,
            false,
        )
        .expect("test: add second path to net A");

        // Add 1 path to network B
        conn.add_path(
            test_endpoint(),
            BlockchainScope::Network,
            PrivacyMode::PRIVATE,
            net_b,
            vec![],
            None,
            false,
        )
        .expect("test: add path to net B");

        assert_eq!(conn.paths_for_network(&net_a).len(), 2);
        assert_eq!(conn.paths_for_network(&net_b).len(), 1);
        assert_eq!(conn.paths_for_network(&network(99)).len(), 0);
    }

    #[test]
    fn test_record_send_recv_metrics() {
        let conn = MultiPathConnection::new("peer-4".to_string(), PathPolicy::default());

        let path_id = conn
            .add_path(
                test_endpoint(),
                BlockchainScope::Network,
                PrivacyMode::PUBLIC,
                network(1),
                vec![],
                None,
                false,
            )
            .expect("test: add path");

        // Record sends and receives
        conn.record_send(path_id, 1000);
        conn.record_send(path_id, 2000);
        conn.record_recv(path_id, 500);

        // Verify path-level metrics
        let snapshot = conn.get_path(path_id).expect("test: path should exist");
        assert_eq!(snapshot.bytes_sent, 3000);
        assert_eq!(snapshot.bytes_received, 500);

        // Verify aggregate metrics
        assert_eq!(
            conn.metrics().total_bytes_sent.load(Ordering::Relaxed),
            3000
        );
        assert_eq!(
            conn.metrics().total_bytes_received.load(Ordering::Relaxed),
            500
        );
        assert_eq!(conn.metrics().sends_completed.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_validate_send_cross_federation() {
        let conn = MultiPathConnection::new("peer-5".to_string(), PathPolicy::default());

        let net_a = network(1);
        let net_b = network(2);
        let net_c = network(3);

        // Add path with federation chain [A, B]
        let path_id = conn
            .add_path(
                test_endpoint(),
                BlockchainScope::Network,
                PrivacyMode::PUBLIC,
                net_a,
                vec![net_a, net_b], // federation chain
                None,
                false,
            )
            .expect("test: add path");

        // Send to same network: allowed
        let result = conn.validate_send_to_network(path_id, &net_a);
        assert!(
            result.is_allowed(),
            "Send to same network should be allowed"
        );

        // Send to network in federation chain: allowed
        let result = conn.validate_send_to_network(path_id, &net_b);
        assert!(
            result.is_allowed(),
            "Send to federation chain member should be allowed"
        );

        // Send to network outside federation: rejected
        let result = conn.validate_send_to_network(path_id, &net_c);
        assert!(
            matches!(
                result,
                PathValidation::Rejected(PathRejectionReason::FederationBoundaryViolation { .. })
            ),
            "Send outside federation should be rejected"
        );
    }
}
