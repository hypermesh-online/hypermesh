// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Engauge intelligence bridge.
//!
//! Connects blockmatrix subsystems to engauge's swarm analytics and routing
//! intelligence when the `intelligence` feature is enabled. Provides:
//!
//! - [`feed_swarm_analytics`]: pushes [`SwarmDemandTracker`] data into
//!   engauge's [`SwarmAnalytics`] for popularity tracking and replication.
//! - [`compute_propagation_weights`]: converts engauge [`TensorWeightModifier`]s
//!   into [`PropagationWeight`]s for the block propagator.
//! - [`apply_path_recommendation`]: translates engauge [`PathPolicyRecommendation`]
//!   into a strategy name string suitable for STOQ's `PathSelector::apply_recommendation`.

#[cfg(feature = "intelligence")]
use std::sync::Arc;

#[cfg(feature = "intelligence")]
use engauge::{
    PathPolicyRecommendation, RoutingAdvisor, RoutingIntelligence, SwarmAnalytics,
    TensorWeightModifier,
};
use hypermesh_lib::{ContentHash, MatrixPosition, NodeId};

use crate::blockchain::propagation::PropagationWeight;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::SwarmDemandTracker;

/// Feed demand data from the [`SwarmDemandTracker`] into engauge's
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

/// Convert engauge [`TensorWeightModifier`]s into [`PropagationWeight`]s
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

/// Translate an engauge [`PathPolicyRecommendation`] into the
/// `(enable_redundant, strategy_name)` pair expected by
/// STOQ's `PathSelector::apply_recommendation`.
#[cfg(feature = "intelligence")]
pub fn apply_path_recommendation(
    rec: &PathPolicyRecommendation,
) -> (bool, &'static str) {
    use engauge::routing_intel::SchedulingStrategy;
    let strategy_name = match rec.strategy {
        SchedulingStrategy::BandwidthWeighted => "BandwidthWeighted",
        SchedulingStrategy::LowestLatency => "LowestLatency",
        SchedulingStrategy::Redundant => "Redundant",
        SchedulingStrategy::RoundRobin => "RoundRobin",
    };
    (rec.enable_redundant, strategy_name)
}

// ---------------------------------------------------------------------------
// EngaugeBridge — periodic feeder + serialized demand summary
// ---------------------------------------------------------------------------

/// High-level bridge that owns references to the swarm demand tracker and
/// analytics, providing periodic feed and serialised demand summaries for
/// cross-node transmission.
#[cfg(feature = "intelligence")]
pub struct EngaugeBridge {
    tracker: Arc<SwarmDemandTracker>,
    analytics: Arc<std::sync::Mutex<SwarmAnalytics>>,
    position: MatrixPosition,
}

#[cfg(feature = "intelligence")]
impl EngaugeBridge {
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
                    .expect("engauge bridge: analytics lock poisoned");
                feed_swarm_analytics(&self.tracker, &mut analytics, self.position).await;
            }
            tokio::time::sleep(interval).await;
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
    use engauge::PathPolicyRecommendation;
    use engauge::routing_intel::SchedulingStrategy;

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
        let mods = vec![engauge::TensorWeightModifier {
            node_id: NodeId::from_public_key(b"unknown"),
            weight_factor: 1.5,
            reason: engauge::routing_intel::WeightReason::LowCongestion,
        }];

        let coords = std::collections::HashMap::new();
        let weights = compute_propagation_weights(&mods, &coords);
        assert!(weights.is_empty());
    }

    #[test]
    fn compute_weights_maps_known_nodes() {
        let node_id = NodeId::from_public_key(b"known-node");
        let coord = MatrixCoordinate::new(5, 10, 15).expect("test: valid coord");

        let mods = vec![engauge::TensorWeightModifier {
            node_id,
            weight_factor: 0.3,
            reason: engauge::routing_intel::WeightReason::HighCongestion,
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
}
