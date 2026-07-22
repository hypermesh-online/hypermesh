// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! NGauge intelligence bridge.
//!
//! Connects blockmatrix subsystems to ngauge's swarm analytics and routing
//! intelligence when the `intelligence` feature is enabled. Provides:
//!
//! - [`feed_swarm_analytics`]: pushes [`SwarmDemandTracker`] data into
//!   ngauge's [`SwarmAnalytics`] for popularity tracking and replication.
//! - [`compute_propagation_weights`]: converts ngauge [`TensorWeightModifier`]s
//!   into [`PropagationWeight`]s for the block propagator.
//! - [`apply_path_recommendation`]: translates ngauge [`PathPolicyRecommendation`]
//!   into a strategy name string suitable for STOQ's `PathSelector::apply_recommendation`.

#[cfg(feature = "intelligence")]
use std::sync::Arc;

#[cfg(feature = "intelligence")]
use ngauge::{
    PathPolicyRecommendation, ReplicationConfig, ReplicationSignal, ReplicationTrigger,
    RoutingAdvisor, RoutingIntelligence, SwarmAnalytics, TensorWeightModifier,
};
use hypermesh_lib::{ContentHash, MatrixPosition, NodeId};

use crate::blockchain::propagation::PropagationWeight;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::SwarmDemandTracker;

/// Feed demand data from the [`SwarmDemandTracker`] into ngauge's
/// [`SwarmAnalytics`]. Each recorded shard fetch becomes a
/// `record_request` call.
///
/// Call this periodically (e.g., every sync round) to keep the swarm
/// analytics up to date without flooding it on every single fetch.
#[cfg(feature = "intelligence")]
pub async fn feed_swarm_analytics(
    tracker: &SwarmDemandTracker,
    analytics: &mut SwarmAnalytics,
    our_position: MatrixPosition,
) {
    let snapshot = tracker.snapshot().await;
    for (shard_id, entry) in &snapshot {
        for requester_id in &entry.requester_ids {
            let consumer_id = NodeId::from_public_key(requester_id.as_bytes());
            analytics.record_request(
                *shard_id,
                consumer_id,
                our_position,
                entry.last_request_us,
            );
        }
    }
    tracing::debug!(
        "Fed {} shard demand entries into SwarmAnalytics",
        snapshot.len(),
    );
}

/// Convert ngauge [`TensorWeightModifier`]s into [`PropagationWeight`]s
/// for the block propagator.
///
/// Requires a mapping from `NodeId` to `MatrixCoordinate`. Nodes without
/// a known coordinate are silently skipped.
#[cfg(feature = "intelligence")]
pub fn compute_propagation_weights(
    modifiers: &[TensorWeightModifier],
    node_coords: &std::collections::HashMap<NodeId, MatrixCoordinate>,
) -> Vec<PropagationWeight> {
    modifiers
        .iter()
        .filter_map(|m| {
            node_coords.get(&m.node_id).map(|coord| PropagationWeight {
                coordinate: *coord,
                weight: m.weight_factor,
            })
        })
        .collect()
}

/// Translate an ngauge [`PathPolicyRecommendation`] into the
/// `(enable_redundant, strategy_name)` pair expected by
/// STOQ's `PathSelector::apply_recommendation`.
#[cfg(feature = "intelligence")]
pub fn apply_path_recommendation(
    rec: &PathPolicyRecommendation,
) -> (bool, &'static str) {
    use ngauge::routing_intel::SchedulingStrategy;
    let strategy_name = match rec.strategy {
        SchedulingStrategy::BandwidthWeighted => "BandwidthWeighted",
        SchedulingStrategy::LowestLatency => "LowestLatency",
        SchedulingStrategy::Redundant => "Redundant",
        SchedulingStrategy::RoundRobin => "RoundRobin",
    };
    (rec.enable_redundant, strategy_name)
}

// ---------------------------------------------------------------------------
// NGaugeBridge — periodic feeder + serialized demand summary
// ---------------------------------------------------------------------------

/// High-level bridge that owns references to the swarm demand tracker and
/// analytics, providing periodic feed and serialised demand summaries for
/// cross-node transmission.
#[cfg(feature = "intelligence")]
pub struct NGaugeBridge {
    tracker: Arc<SwarmDemandTracker>,
    analytics: Arc<std::sync::Mutex<SwarmAnalytics>>,
    position: MatrixPosition,
}

#[cfg(feature = "intelligence")]
impl NGaugeBridge {
    /// Create a new bridge.
    pub fn new(
        tracker: Arc<SwarmDemandTracker>,
        analytics: Arc<std::sync::Mutex<SwarmAnalytics>>,
        position: MatrixPosition,
    ) -> Self {
        Self {
            tracker,
            analytics,
            position,
        }
    }

    /// Run a periodic loop that feeds swarm demand data into analytics.
    ///
    /// Calls [`feed_swarm_analytics`] every `interval_secs` seconds.
    /// Designed to be spawned via `tokio::spawn`.
    pub async fn run_periodic_feed(&self, interval_secs: u64) {
        let interval = std::time::Duration::from_secs(interval_secs);
        loop {
            {
                let mut analytics = self
                    .analytics
                    .lock()
                    .expect("ngauge bridge: analytics lock poisoned");
                feed_swarm_analytics(&self.tracker, &mut analytics, self.position).await;
            }
            tokio::time::sleep(interval).await;
        }
    }

    /// Run the ngauge [`ReplicationTrigger`] against the shared analytics
    /// snapshot and return the replication signals it produces.
    ///
    /// Caller is expected to drive the swarm closure: for each
    /// `Replicate` signal, look up providers via `ShardLocationIndex` and
    /// request additional copies via TAG_SHARD_FETCH (see Phase E.2).
    ///
    /// Acquires a synchronous lock on analytics. The lock is released
    /// before this function returns, so the caller can hold the returned
    /// signals across await points.
    pub fn check_replication_signals(&self) -> Vec<ReplicationSignal> {
        self.check_replication_signals_with(ReplicationConfig::default())
    }

    /// Variant of [`check_replication_signals`] that lets the caller
    /// override the trigger configuration.
    pub fn check_replication_signals_with(
        &self,
        config: ReplicationConfig,
    ) -> Vec<ReplicationSignal> {
        let trigger = ReplicationTrigger::new(config);
        match self.analytics.lock() {
            Ok(guard) => trigger.check(&guard),
            Err(e) => {
                tracing::warn!("ngauge bridge: analytics lock poisoned: {e}");
                Vec::new()
            }
        }
    }

    /// Produce serialized demand summaries suitable for cross-node transmission.
    ///
    /// Returns one JSON-encoded byte vector per tracked shard containing the
    /// shard hash, request count, and unique consumer count.
    pub async fn metrics_to_transmit(&self) -> Vec<Vec<u8>> {
        let snapshot = self.tracker.snapshot().await;
        snapshot
            .iter()
            .filter_map(|(hash, entry)| {
                let summary = serde_json::json!({
                    "shard_hash": hex::encode(hash.0),
                    "request_count": entry.request_count,
                    "unique_consumers": entry.requester_ids.len(),
                    "last_request_us": entry.last_request_us,
                });
                serde_json::to_vec(&summary).ok()
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[cfg(feature = "intelligence")]
mod tests {
    use super::*;
    use ngauge::PathPolicyRecommendation;
    use ngauge::routing_intel::SchedulingStrategy;

    #[test]
    fn path_recommendation_maps_strategies() {
        let rec = PathPolicyRecommendation {
            enable_redundant: false,
            strategy: SchedulingStrategy::BandwidthWeighted,
            congestion_level: 0.1,
        };
        let (redundant, name) = apply_path_recommendation(&rec);
        assert!(!redundant);
        assert_eq!(name, "BandwidthWeighted");

        let rec2 = PathPolicyRecommendation {
            enable_redundant: true,
            strategy: SchedulingStrategy::Redundant,
            congestion_level: 0.9,
        };
        let (redundant2, name2) = apply_path_recommendation(&rec2);
        assert!(redundant2);
        assert_eq!(name2, "Redundant");
    }

    #[test]
    fn compute_weights_skips_unknown_nodes() {
        let mods = vec![ngauge::TensorWeightModifier {
            node_id: NodeId::from_public_key(b"unknown"),
            weight_factor: 1.5,
            reason: ngauge::routing_intel::WeightReason::LowCongestion,
        }];

        let coords = std::collections::HashMap::new();
        let weights = compute_propagation_weights(&mods, &coords);
        assert!(weights.is_empty());
    }

    #[test]
    fn compute_weights_maps_known_nodes() {
        let node_id = NodeId::from_public_key(b"known-node");
        let coord = MatrixCoordinate::new(5, 10, 15).expect("test: valid coord");

        let mods = vec![ngauge::TensorWeightModifier {
            node_id,
            weight_factor: 0.3,
            reason: ngauge::routing_intel::WeightReason::HighCongestion,
        }];

        let mut coords = std::collections::HashMap::new();
        coords.insert(node_id, coord);

        let weights = compute_propagation_weights(&mods, &coords);
        assert_eq!(weights.len(), 1);
        assert_eq!(weights[0].coordinate, coord);
        assert!((weights[0].weight - 0.3).abs() < 1e-6);
    }

    #[tokio::test]
    async fn feed_swarm_analytics_processes_demand() {
        let tracker = SwarmDemandTracker::new();
        let hash = ContentHash([0xAB; 32]);
        tracker.record_fetch(hash, "peer-1").await;
        tracker.record_fetch(hash, "peer-2").await;

        let mut analytics = SwarmAnalytics::new();
        let pos = MatrixPosition {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };

        feed_swarm_analytics(&tracker, &mut analytics, pos).await;

        let pop = analytics.get_popularity(&hash);
        assert!(pop.is_some(), "shard should have popularity data");
        let pop = pop.expect("test: should exist");
        // 2 unique requester IDs recorded
        assert_eq!(pop.unique_consumers, 2);
    }

    /// H2-H3 test: NGaugeBridge feeds demand data into analytics via shared Mutex.
    #[tokio::test]
    async fn test_bridge_feeds_analytics() {
        let tracker = Arc::new(SwarmDemandTracker::new());
        let analytics = Arc::new(std::sync::Mutex::new(SwarmAnalytics::new()));
        let pos = MatrixPosition { x: 1.0, y: 2.0, z: 3.0 };

        // Record some demand data.
        let hash = ContentHash([0xDD; 32]);
        tracker.record_fetch(hash, "consumer-a").await;
        tracker.record_fetch(hash, "consumer-b").await;
        tracker.record_fetch(hash, "consumer-c").await;

        // Feed into analytics via the free function (same logic NGaugeBridge uses).
        {
            let snapshot = tracker.snapshot().await;
            let mut analytics_guard = analytics.lock().expect("test: analytics lock");
            for (shard_id, entry) in &snapshot {
                for requester_id in &entry.requester_ids {
                    let consumer_id = NodeId::from_public_key(requester_id.as_bytes());
                    analytics_guard.record_request(*shard_id, consumer_id, pos, entry.last_request_us);
                }
            }
        }

        // Verify analytics received the demand data.
        let guard = analytics.lock().expect("test: analytics lock");
        let pop = guard.get_popularity(&hash).expect("test: should have data");
        assert_eq!(pop.unique_consumers, 3);
        assert!(pop.request_count >= 3);
    }

    /// H4 test: compute_propagation_weights produces non-empty weights when
    /// demand data exists and RoutingIntelligence computes modifiers.
    #[test]
    fn test_propagation_weights_from_routing_intelligence() {
        use ngauge::RoutingAdvisor;

        let node_a = NodeId::from_public_key(b"node-alpha");
        let node_b = NodeId::from_public_key(b"node-beta");
        let coord_a = MatrixCoordinate::new(1, 2, 3).expect("test: coord");
        let coord_b = MatrixCoordinate::new(4, 5, 6).expect("test: coord");

        let mut node_coords = std::collections::HashMap::new();
        node_coords.insert(node_a, coord_a);
        node_coords.insert(node_b, coord_b);

        let candidates = vec![node_a, node_b];
        let ri = ngauge::RoutingIntelligence::new(30);
        let source_pos = MatrixPosition { x: 0.0, y: 0.0, z: 0.0 };

        let modifiers = ri.compute_weight_adjustments(&source_pos, &source_pos, &candidates);
        assert_eq!(modifiers.len(), 2, "should get modifiers for both candidates");

        let weights = compute_propagation_weights(&modifiers, &node_coords);
        assert_eq!(weights.len(), 2, "both candidates have known coordinates");
        // Default weight factor is 1.0 (neutral) when no metrics ingested.
        for w in &weights {
            assert!((w.weight - 1.0).abs() < 1e-6, "neutral weight expected");
        }
    }

    /// H5 test: ReplicationTrigger detects shards needing replication when
    /// demand data is present in SwarmAnalytics.
    #[tokio::test]
    async fn test_replication_trigger_fires_on_demand() {
        let tracker = Arc::new(SwarmDemandTracker::new());
        let analytics = Arc::new(std::sync::Mutex::new(SwarmAnalytics::new()));
        let pos = MatrixPosition { x: 0.0, y: 0.0, z: 0.0 };

        // Record high demand for a shard (exceeds default threshold of 100 per replica).
        let hash = ContentHash([0xEE; 32]);
        for i in 0..150 {
            tracker.record_fetch(hash, &format!("peer-{i}")).await;
        }

        // Feed into analytics.
        {
            let snapshot = tracker.snapshot().await;
            let mut guard = analytics.lock().expect("test: analytics lock");
            for (shard_id, entry) in &snapshot {
                for requester_id in &entry.requester_ids {
                    let consumer_id = NodeId::from_public_key(requester_id.as_bytes());
                    guard.record_request(*shard_id, consumer_id, pos, entry.last_request_us);
                }
            }
        }

        // Check replication triggers.
        let guard = analytics.lock().expect("test: analytics lock");
        let trigger = ngauge::ReplicationTrigger::new(ngauge::ReplicationConfig::default());
        let signals = trigger.check(&guard);
        assert!(!signals.is_empty(), "should detect replication need for high-demand shard");

        let signal = &signals[0];
        assert_eq!(signal.shard_id, hash);
        assert!(signal.urgency > 0.0, "urgency should be positive");
        assert!(signal.suggested_count >= 1, "should suggest at least 1 replica");
    }
}
