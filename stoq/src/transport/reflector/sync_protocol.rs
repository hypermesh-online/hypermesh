// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sync protocol state machine for reflector pool synchronization.
//!
//! Manages heartbeat broadcasting, peer health tracking, replication threshold
//! detection for block heights, sync request/response flow, block
//! announcements, and stale-peer pruning.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::RwLock;
use tracing::{debug, info};

use hypermesh_lib::{MatrixPosition, NetworkId};

use super::block_transport::StoqBlockTransport;
use super::message::ReflectorMessage;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration knobs for the sync protocol.
#[derive(Debug, Clone)]
pub struct SyncProtocolConfig {
    /// Interval between heartbeat broadcasts.
    pub heartbeat_interval: Duration,
    /// A reflector is considered stale after this duration without a
    /// heartbeat.
    pub stale_reflector_timeout: Duration,
    /// Fraction of peers that must confirm a block for replication threshold (0.0-1.0).
    pub replication_threshold: f64,
    /// Maximum number of blocks to request in a single sync batch.
    pub max_sync_batch_size: u32,
    /// Delay before retrying a failed sync request.
    pub sync_retry_delay: Duration,
    /// Maximum number of sync retry attempts.
    pub max_sync_retries: u32,
}

impl Default for SyncProtocolConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(5),
            stale_reflector_timeout: Duration::from_secs(30),
            replication_threshold: 0.67,
            max_sync_batch_size: 50,
            sync_retry_delay: Duration::from_secs(2),
            max_sync_retries: 5,
        }
    }
}

// ---------------------------------------------------------------------------
// PeerHealth
// ---------------------------------------------------------------------------

/// Health tracking for a single peer in the reflector pool.
#[derive(Debug)]
pub struct PeerHealth {
    /// Identifier of the tracked peer.
    pub node_id: String,
    /// When the last heartbeat was received (or injected).
    pub last_heartbeat: Instant,
    /// Last reported block height.
    pub block_height: u64,
    /// Last reported health score (0.0-1.0).
    pub health_score: f64,
    /// Peer's Block-MATRIX position.
    pub position: MatrixPosition,
    /// Number of consecutive missed heartbeats.
    pub consecutive_misses: u32,
}

// ---------------------------------------------------------------------------
// ReplicationState
// ---------------------------------------------------------------------------

/// Tracks replication threshold confirmations for block heights.
#[derive(Debug)]
pub struct ReplicationState {
    /// `block_height -> list of confirming node_ids`.
    confirmations: HashMap<u64, Vec<String>>,
    /// Total number of known peers for replication threshold fraction calculation.
    total_peers: usize,
}

impl Default for ReplicationState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicationState {
    /// Create an empty replication threshold state.
    pub fn new() -> Self {
        Self {
            confirmations: HashMap::new(),
            total_peers: 0,
        }
    }

    /// Record a confirmation for the given block height from `node_id`.
    ///
    /// Duplicate confirmations from the same node are silently ignored.
    pub fn record_confirmation(&mut self, block_height: u64, node_id: String) {
        let list = self.confirmations.entry(block_height).or_default();
        if !list.contains(&node_id) {
            list.push(node_id);
        }
    }

    /// Check whether replication threshold has been reached for `block_height`.
    ///
    /// Threshold is met when the number of unique confirmations is
    /// at least `ceil(total_peers * threshold)`. If there are zero
    /// peers, replication threshold is never reached.
    pub fn has_sufficient_peers(&self, block_height: u64, threshold: f64) -> bool {
        if self.total_peers == 0 {
            return false;
        }
        let required = (self.total_peers as f64 * threshold).ceil() as usize;
        let count = self
            .confirmations
            .get(&block_height)
            .map(|v| v.len())
            .unwrap_or(0);
        count >= required
    }

    /// Update the total peer count used for replication threshold calculations.
    pub fn update_peer_count(&mut self, count: usize) {
        self.total_peers = count;
    }

    /// Number of unique confirmations for a block height.
    pub fn confirmations_for(&self, block_height: u64) -> usize {
        self.confirmations
            .get(&block_height)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Remove all confirmation entries below `min_height`.
    pub fn cleanup_below(&mut self, min_height: u64) {
        self.confirmations.retain(|&h, _| h >= min_height);
    }
}

// ---------------------------------------------------------------------------
// SyncProtocol
// ---------------------------------------------------------------------------

/// Sync protocol state machine coordinating heartbeats, replication threshold,
/// sync requests, block announcements, and peer health.
pub struct SyncProtocol {
    /// Protocol configuration.
    config: SyncProtocolConfig,
    /// Underlying block transport for sending/receiving messages.
    transport: Arc<StoqBlockTransport>,
    /// Network this protocol manages.
    network_id: NetworkId,
    /// Local node identifier.
    local_node_id: String,
    /// Local node's Block-MATRIX position.
    local_position: MatrixPosition,
    /// Local chain height (updated externally via `set_block_height`).
    local_block_height: AtomicU64,
    /// Per-peer health information.
    health_tracker: Arc<DashMap<String, PeerHealth>>,
    /// Replication tracking for block heights.
    replication_state: Arc<RwLock<ReplicationState>>,
    /// Flag indicating whether a sync operation is currently active.
    _sync_in_progress: RwLock<bool>,
}

impl SyncProtocol {
    /// Create a new sync protocol instance wired to the given transport.
    pub fn new(
        config: SyncProtocolConfig,
        transport: Arc<StoqBlockTransport>,
        network_id: NetworkId,
        local_node_id: String,
        local_position: MatrixPosition,
    ) -> Self {
        info!(
            node = %local_node_id,
            network = %network_id,
            "SyncProtocol created"
        );
        Self {
            config,
            transport,
            network_id,
            local_node_id,
            local_position,
            local_block_height: AtomicU64::new(0),
            health_tracker: Arc::new(DashMap::new()),
            replication_state: Arc::new(RwLock::new(ReplicationState::new())),
            _sync_in_progress: RwLock::new(false),
        }
    }

    // -- Heartbeat -----------------------------------------------------------

    /// Build a [`ReflectorMessage::Heartbeat`] with the local node's
    /// current state.
    pub fn create_heartbeat(&self) -> ReflectorMessage {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        ReflectorMessage::Heartbeat {
            node_id: self.local_node_id.clone(),
            network_id: self.network_id,
            block_height: self.local_block_height.load(Ordering::Relaxed),
            health_score: 1.0, // local node is always healthy
            position: self.local_position,
            timestamp: now_secs,
        }
    }

    /// Broadcast a heartbeat to all connected reflectors.
    ///
    /// Returns the number of reflectors the heartbeat was sent to.
    pub fn send_heartbeat(&self) -> anyhow::Result<usize> {
        let hb = self.create_heartbeat();
        self.transport.broadcast_message(&hb)
    }

    /// Process an incoming heartbeat from a remote peer.
    ///
    /// Updates (or inserts) the peer's entry in the health tracker and
    /// propagates the new health/height to the transport layer.
    pub fn process_heartbeat(
        &self,
        from_node: &str,
        block_height: u64,
        health_score: f64,
        position: MatrixPosition,
    ) {
        let clamped_health = health_score.clamp(0.0, 1.0);
        let now = Instant::now();

        self.health_tracker.insert(
            from_node.to_string(),
            PeerHealth {
                node_id: from_node.to_string(),
                last_heartbeat: now,
                block_height,
                health_score: clamped_health,
                position,
                consecutive_misses: 0,
            },
        );

        self.transport
            .update_reflector_health(from_node, clamped_health, block_height);

        // Keep replication threshold state's peer count in sync.
        self.replication_state
            .write()
            .update_peer_count(self.health_tracker.len());

        debug!(
            from = %from_node,
            height = block_height,
            health = clamped_health,
            "Processed heartbeat"
        );
    }

    // -- Sync flow -----------------------------------------------------------

    /// Send a sync request to a specific peer.
    pub fn request_sync(&self, target_node: &str, from_height: u64) -> anyhow::Result<()> {
        let msg = ReflectorMessage::SyncRequest {
            network_id: self.network_id,
            from_height,
            max_blocks: self.config.max_sync_batch_size,
            requesting_node: self.local_node_id.clone(),
        };
        self.transport.send_message(&msg, target_node)
    }

    /// Process an incoming sync response.
    ///
    /// Currently logs the response for diagnostics. Block fetching
    /// based on the returned hashes will be wired in a future sprint
    /// when the STOQ stream consumer is integrated.
    pub fn process_sync_response(
        &self,
        from_node: &str,
        block_hashes: &[[u8; 32]],
        peer_height: u64,
    ) {
        info!(
            from = %from_node,
            hashes = block_hashes.len(),
            peer_height = peer_height,
            "Received sync response"
        );
    }

    // -- Block announcements -------------------------------------------------

    /// Broadcast a block announcement to all connected reflectors.
    ///
    /// Returns the number of reflectors the announcement was sent to.
    pub fn announce_block(&self, block_height: u64, block_hash: [u8; 32]) -> anyhow::Result<usize> {
        let msg = ReflectorMessage::BlockAnnounce {
            network_id: self.network_id,
            block_height,
            block_hash,
            announcing_node: self.local_node_id.clone(),
        };
        self.transport.broadcast_message(&msg)
    }

    // -- Replication ----------------------------------------------------------

    /// Record a replication threshold confirmation from a peer.
    pub fn record_confirmation(&self, block_height: u64, from_node: &str) {
        self.replication_state
            .write()
            .record_confirmation(block_height, from_node.to_string());
    }

    /// Check whether replication threshold has been reached for a block height.
    pub fn has_sufficient_peers(&self, block_height: u64) -> bool {
        self.replication_state
            .read()
            .has_sufficient_peers(block_height, self.config.replication_threshold)
    }

    // -- Stale peer management -----------------------------------------------

    /// Remove peers whose last heartbeat exceeds the stale timeout.
    ///
    /// Returns the number of peers pruned.
    pub fn prune_stale(&self) -> usize {
        let threshold = self.config.stale_reflector_timeout;
        let now = Instant::now();

        let stale_ids: Vec<String> = self
            .health_tracker
            .iter()
            .filter(|entry| now.duration_since(entry.value().last_heartbeat) > threshold)
            .map(|entry| entry.key().clone())
            .collect();

        let count = stale_ids.len();
        for id in &stale_ids {
            self.health_tracker.remove(id);
            self.transport.disconnect_reflector(id);
            debug!(node = %id, "Pruned stale reflector");
        }

        if count > 0 {
            self.replication_state
                .write()
                .update_peer_count(self.health_tracker.len());
            info!(pruned = count, "Stale reflectors removed");
        }
        count
    }

    // -- Queries -------------------------------------------------------------

    /// Update the local block height.
    pub fn set_block_height(&self, height: u64) {
        self.local_block_height.store(height, Ordering::Relaxed);
    }

    /// Current local block height.
    pub fn block_height(&self) -> u64 {
        self.local_block_height.load(Ordering::Relaxed)
    }

    /// Number of tracked peers (healthy or stale).
    pub fn peer_count(&self) -> usize {
        self.health_tracker.len()
    }

    /// List identifiers of peers that are not stale.
    pub fn healthy_peers(&self) -> Vec<String> {
        let threshold = self.config.stale_reflector_timeout;
        let now = Instant::now();

        self.health_tracker
            .iter()
            .filter(|e| now.duration_since(e.value().last_heartbeat) <= threshold)
            .map(|e| e.key().clone())
            .collect()
    }

    /// Check whether any tracked peer has a higher block height than
    /// the local node, indicating that a sync is needed.
    pub fn needs_sync(&self) -> bool {
        let local = self.local_block_height.load(Ordering::Relaxed);
        self.health_tracker
            .iter()
            .any(|e| e.value().block_height > local)
    }

    /// Reference to the protocol configuration.
    pub fn config(&self) -> &SyncProtocolConfig {
        &self.config
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_network() -> NetworkId {
        NetworkId([0x02; 16])
    }

    fn test_position(x: f64) -> MatrixPosition {
        MatrixPosition { x, y: 0.0, z: 0.0 }
    }

    fn make_transport() -> Arc<StoqBlockTransport> {
        Arc::new(StoqBlockTransport::new(
            test_network(),
            hypermesh_lib::PrivacyMode::PUBLIC,
            "local".to_string(),
            test_position(0.0),
        ))
    }

    fn make_protocol(transport: Arc<StoqBlockTransport>) -> SyncProtocol {
        SyncProtocol::new(
            SyncProtocolConfig::default(),
            transport,
            test_network(),
            "local".to_string(),
            test_position(0.0),
        )
    }

    #[test]
    fn test_heartbeat_creation() {
        let t = make_transport();
        let p = make_protocol(t);
        p.set_block_height(42);

        let hb = p.create_heartbeat();
        match hb {
            ReflectorMessage::Heartbeat {
                node_id,
                network_id,
                block_height,
                health_score,
                ..
            } => {
                assert_eq!(node_id, "local");
                assert_eq!(network_id, test_network());
                assert_eq!(block_height, 42);
                assert!((health_score - 1.0).abs() < f64::EPSILON);
            }
            _ => unreachable!("test: expected Heartbeat"),
        }
    }

    #[test]
    fn test_send_heartbeat_broadcast() {
        let t = make_transport();
        t.connect_reflector("r1".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect r1");
        t.connect_reflector("r2".into(), "[::1]:9002".into(), test_position(2.0))
            .expect("test: connect r2");

        let p = make_protocol(t);
        let count = p.send_heartbeat().expect("test: send heartbeat");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_process_heartbeat_updates_health() {
        let t = make_transport();
        let p = make_protocol(t.clone());

        t.connect_reflector("peer-1".into(), "[::1]:9001".into(), test_position(5.0))
            .expect("test: connect");

        p.process_heartbeat("peer-1", 100, 0.85, test_position(5.0));

        assert_eq!(p.peer_count(), 1);

        let entry = p
            .health_tracker
            .get("peer-1")
            .expect("test: peer should exist");
        assert_eq!(entry.value().block_height, 100);
        assert!((entry.value().health_score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_replication_threshold_detection() {
        let t = make_transport();
        let p = make_protocol(t.clone());

        // Register 3 peers via heartbeats
        t.connect_reflector("p1".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect p1");
        t.connect_reflector("p2".into(), "[::1]:9002".into(), test_position(2.0))
            .expect("test: connect p2");
        t.connect_reflector("p3".into(), "[::1]:9003".into(), test_position(3.0))
            .expect("test: connect p3");

        p.process_heartbeat("p1", 10, 1.0, test_position(1.0));
        p.process_heartbeat("p2", 10, 1.0, test_position(2.0));
        p.process_heartbeat("p3", 10, 1.0, test_position(3.0));

        // 2 out of 3 confirm block 10 -> 66.7% which meets ceil(3*0.67) = 3? No.
        // ceil(3 * 0.67) = ceil(2.01) = 3 -- need all 3? Let's verify:
        // Actually 3 * 0.67 = 2.01, ceil(2.01) = 3. So we need 3.
        // With threshold 0.67 and 3 peers, required = ceil(2.01) = 3.
        // Let's confirm with 2 confirmations first (should NOT meet threshold)
        p.record_confirmation(10, "p1");
        p.record_confirmation(10, "p2");
        assert!(!p.has_sufficient_peers(10), "2/3 should not meet threshold at 0.67");

        // 3 out of 3 -> should meet threshold
        p.record_confirmation(10, "p3");
        assert!(p.has_sufficient_peers(10), "3/3 should meet threshold at 0.67");
    }

    #[test]
    fn test_replication_threshold_below_threshold() {
        let t = make_transport();
        let p = make_protocol(t.clone());

        // 4 peers, threshold 0.67 -> required = ceil(4 * 0.67) = ceil(2.68) = 3
        for i in 1..=4 {
            let id = format!("p{i}");
            t.connect_reflector(id.clone(), format!("[::1]:900{i}"), test_position(i as f64))
                .expect("test: connect");
            p.process_heartbeat(&id, 50, 1.0, test_position(i as f64));
        }

        // Only 1 confirmation -- well below threshold
        p.record_confirmation(50, "p1");
        assert!(!p.has_sufficient_peers(50));

        // 2 confirmations -- still below (need 3)
        p.record_confirmation(50, "p2");
        assert!(!p.has_sufficient_peers(50));

        // 3 confirmations -- meets threshold
        p.record_confirmation(50, "p3");
        assert!(p.has_sufficient_peers(50));
    }

    #[test]
    fn test_prune_stale() {
        let config = SyncProtocolConfig {
            stale_reflector_timeout: Duration::from_millis(1),
            ..SyncProtocolConfig::default()
        };

        let t = make_transport();
        t.connect_reflector("stale-peer".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect");

        let p = SyncProtocol::new(
            config,
            t.clone(),
            test_network(),
            "local".to_string(),
            test_position(0.0),
        );

        // Insert a peer with an old heartbeat by using the protocol's
        // process_heartbeat and then sleeping past the timeout.
        p.process_heartbeat("stale-peer", 10, 0.5, test_position(1.0));
        assert_eq!(p.peer_count(), 1);

        // Sleep just past the 1ms timeout.
        std::thread::sleep(Duration::from_millis(10));

        let pruned = p.prune_stale();
        assert_eq!(pruned, 1);
        assert_eq!(p.peer_count(), 0);
        assert_eq!(t.reflector_count(), 0);
    }

    #[test]
    fn test_needs_sync() {
        let t = make_transport();
        let p = make_protocol(t.clone());
        p.set_block_height(50);

        t.connect_reflector("ahead".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect");
        p.process_heartbeat("ahead", 100, 1.0, test_position(1.0));

        assert!(
            p.needs_sync(),
            "peer at 100 vs local at 50 should need sync"
        );

        // Bring local up to date
        p.set_block_height(100);
        assert!(!p.needs_sync(), "same height should not need sync");
    }

    #[test]
    fn test_announce_block() {
        let t = make_transport();
        t.connect_reflector("r1".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect r1");
        t.connect_reflector("r2".into(), "[::1]:9002".into(), test_position(2.0))
            .expect("test: connect r2");

        let p = make_protocol(t.clone());
        let count = p.announce_block(99, [0xFF; 32]).expect("test: announce");
        assert_eq!(count, 2);

        let outbox = t.drain_outbox();
        assert_eq!(outbox.len(), 2);
    }

    #[test]
    fn test_healthy_peers_excludes_stale() {
        let config = SyncProtocolConfig {
            stale_reflector_timeout: Duration::from_secs(60),
            ..SyncProtocolConfig::default()
        };

        let t = make_transport();
        t.connect_reflector("fresh".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect");

        let p = SyncProtocol::new(
            config,
            t,
            test_network(),
            "local".to_string(),
            test_position(0.0),
        );

        p.process_heartbeat("fresh", 10, 1.0, test_position(1.0));
        let healthy = p.healthy_peers();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0], "fresh");
    }

    #[test]
    fn test_replication_threshold_cleanup_below() {
        let mut qs = ReplicationState::new();
        qs.update_peer_count(3);

        qs.record_confirmation(5, "a".to_string());
        qs.record_confirmation(10, "b".to_string());
        qs.record_confirmation(15, "c".to_string());

        qs.cleanup_below(10);
        assert_eq!(qs.confirmations_for(5), 0);
        assert_eq!(qs.confirmations_for(10), 1);
        assert_eq!(qs.confirmations_for(15), 1);
    }

    #[test]
    fn test_duplicate_confirmation_ignored() {
        let mut qs = ReplicationState::new();
        qs.update_peer_count(3);

        qs.record_confirmation(10, "node-a".to_string());
        qs.record_confirmation(10, "node-a".to_string());
        assert_eq!(qs.confirmations_for(10), 1);
    }
}
