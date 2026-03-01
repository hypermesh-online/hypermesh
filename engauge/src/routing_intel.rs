// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Routing intelligence -- transforms aggregated metrics into routing
//! recommendations for BlockMatrix tensor routing and STOQ path scheduling.
//!
//! Consumers implement [`RoutingAdvisor`] or [`PathAdvisor`] trait integration
//! behind feature gates, keeping engauge loosely coupled from infrastructure
//! crates.

use hypermesh_lib::{MatrixPosition, NodeId};

use crate::streaming::aggregator::RegionalAggregate;
use crate::streaming::protocol::MetricsPayload;
use crate::streaming::subscriber::MetricsSubscriber;
use crate::trending::EpochTracker;

/// A routing weight adjustment for a single candidate node.
///
/// Positive values increase routing preference, negative decrease.
/// Zero means no adjustment (neutral).
#[derive(Debug, Clone, PartialEq)]
pub struct TensorWeightModifier {
    /// Node this modifier applies to.
    pub node_id: NodeId,
    /// Multiplicative weight factor (1.0 = neutral, >1 = prefer, <1 = avoid).
    pub weight_factor: f64,
    /// Reason for the adjustment (for diagnostics/logging).
    pub reason: WeightReason,
}

/// Why a routing weight was adjusted.
#[derive(Debug, Clone, PartialEq)]
pub enum WeightReason {
    /// Node has low congestion — prefer it.
    LowCongestion,
    /// Node has high congestion — avoid it.
    HighCongestion,
    /// Node has good throughput.
    HighThroughput,
    /// Node has low throughput.
    LowThroughput,
    /// Node has high latency.
    HighLatency,
    /// Node has low latency.
    LowLatency,
    /// No metrics available — neutral.
    NoData,
}

/// Recommendation for STOQ path scheduler policy adjustments.
#[derive(Debug, Clone, PartialEq)]
pub struct PathPolicyRecommendation {
    /// Whether to enable redundant multi-path.
    pub enable_redundant: bool,
    /// Suggested scheduling strategy.
    pub strategy: SchedulingStrategy,
    /// Congestion level observed (0.0 = clear, 1.0 = saturated).
    pub congestion_level: f64,
}

/// Suggested path scheduling strategy based on observed metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingStrategy {
    /// Network is clear — use bandwidth-weighted scheduling.
    BandwidthWeighted,
    /// Moderate congestion — use lowest-latency scheduling.
    LowestLatency,
    /// High congestion — use redundant multi-path.
    Redundant,
    /// No data — use round-robin default.
    RoundRobin,
}

/// Trait for BlockMatrix's TransactionRouter to optionally consume routing
/// weight adjustments from engauge.
pub trait RoutingAdvisor {
    /// Compute routing weight adjustments for candidate relay nodes.
    fn compute_weight_adjustments(
        &self,
        source: &MatrixPosition,
        destination: &MatrixPosition,
        candidates: &[NodeId],
    ) -> Vec<TensorWeightModifier>;
}

/// Trait for STOQ's PathScheduler to optionally consume path policy
/// recommendations from engauge.
pub trait PathAdvisor {
    /// Recommend path policy adjustments based on observed network state.
    fn recommend_path_policy(&self, aggregate: &RegionalAggregate) -> PathPolicyRecommendation;
}

/// Transforms aggregated network metrics into routing recommendations.
///
/// Consumes data from [`MetricsSubscriber`] and [`EpochTracker`] to produce
/// [`TensorWeightModifier`]s for BlockMatrix pathfinding and
/// [`PathPolicyRecommendation`]s for STOQ path scheduling.
pub struct RoutingIntelligence {
    subscriber: MetricsSubscriber,
    trend_tracker: EpochTracker,
}

/// Congestion thresholds.
const CONGESTION_LOW: f64 = 0.3;
const CONGESTION_HIGH: f64 = 0.7;

/// Throughput thresholds (bps).
const THROUGHPUT_LOW_BPS: f64 = 100_000_000.0; // 100 Mbps
const THROUGHPUT_HIGH_BPS: f64 = 1_000_000_000.0; // 1 Gbps

/// Latency thresholds (microseconds).
const LATENCY_LOW_US: f64 = 5_000.0; // 5ms
const LATENCY_HIGH_US: f64 = 50_000.0; // 50ms

impl RoutingIntelligence {
    /// Create a new routing intelligence engine.
    ///
    /// `max_window_size` controls how many frames the subscriber retains
    /// per source node (default: 30 for ~5 minutes at 10s intervals).
    pub fn new(max_window_size: usize) -> Self {
        Self {
            subscriber: MetricsSubscriber::new(max_window_size),
            trend_tracker: EpochTracker::new(max_window_size),
        }
    }

    /// Ingest a metrics frame from the streaming layer.
    pub fn ingest(&mut self, frame: crate::streaming::MetricsFrame) {
        self.subscriber.receive(frame);
    }

    /// Access the underlying subscriber for direct queries.
    pub fn subscriber(&self) -> &MetricsSubscriber {
        &self.subscriber
    }

    /// Access the trend tracker for epoch-level analysis.
    pub fn trend_tracker(&self) -> &EpochTracker {
        &self.trend_tracker
    }

    /// Mutable access to the trend tracker for recording epochs.
    pub fn trend_tracker_mut(&mut self) -> &mut EpochTracker {
        &mut self.trend_tracker
    }

    /// Compute a weight modifier for a single candidate based on its
    /// latest metrics in the subscriber window.
    fn weight_for_candidate(&self, candidate: &NodeId) -> TensorWeightModifier {
        let latest = match self.subscriber.latest(candidate) {
            Some(frame) => frame,
            None => {
                return TensorWeightModifier {
                    node_id: *candidate,
                    weight_factor: 1.0,
                    reason: WeightReason::NoData,
                };
            }
        };

        let (weight_factor, reason) = match &latest.payload {
            MetricsPayload::Congestion(c) => weight_from_congestion(c),
            MetricsPayload::Routing(r) => weight_from_routing(r),
            MetricsPayload::Capacity(c) => weight_from_capacity(c),
            MetricsPayload::Economic(e) => weight_from_economic(e),
            MetricsPayload::Verification(v) => weight_from_verification(v),
        };

        TensorWeightModifier {
            node_id: *candidate,
            weight_factor,
            reason,
        }
    }
}

/// Derive weight factor and reason from a congestion snapshot.
fn weight_from_congestion(
    snap: &crate::streaming::protocol::CongestionSnapshot,
) -> (f64, WeightReason) {
    if snap.buffer_fullness_ratio > CONGESTION_HIGH {
        (0.3, WeightReason::HighCongestion)
    } else if snap.buffer_fullness_ratio < CONGESTION_LOW {
        (1.5, WeightReason::LowCongestion)
    } else {
        (1.0, WeightReason::LowCongestion)
    }
}

/// Derive weight factor and reason from a routing snapshot.
fn weight_from_routing(snap: &crate::streaming::protocol::RoutingSnapshot) -> (f64, WeightReason) {
    let latency = snap.avg_latency_us as f64;
    let throughput = snap.throughput_bps as f64;

    if latency > LATENCY_HIGH_US {
        (0.5, WeightReason::HighLatency)
    } else if latency < LATENCY_LOW_US && throughput > THROUGHPUT_HIGH_BPS {
        (1.8, WeightReason::LowLatency)
    } else if throughput < THROUGHPUT_LOW_BPS {
        (0.6, WeightReason::LowThroughput)
    } else {
        (1.0, WeightReason::HighThroughput)
    }
}

/// Derive weight factor and reason from a capacity snapshot.
fn weight_from_capacity(
    snap: &crate::streaming::protocol::CapacitySnapshot,
) -> (f64, WeightReason) {
    let bw_ratio = snap.bandwidth_available_bps as f64 / THROUGHPUT_HIGH_BPS;
    let factor = 0.5 + bw_ratio.clamp(0.0, 1.0); // 0.5-1.5 range
    let reason = if bw_ratio > 0.5 {
        WeightReason::HighThroughput
    } else {
        WeightReason::LowThroughput
    };
    (factor, reason)
}

/// Derive weight factor and reason from an economic snapshot.
///
/// Economic data does not directly affect routing weights.
fn weight_from_economic(
    _snap: &crate::streaming::protocol::EconomicSnapshot,
) -> (f64, WeightReason) {
    (1.0, WeightReason::NoData)
}

/// Derive weight factor and reason from a verification snapshot.
///
/// Nodes with high spatial consistency are preferred for routing.
fn weight_from_verification(
    snap: &crate::streaming::protocol::VerificationSnapshot,
) -> (f64, WeightReason) {
    if snap.consistency_ratio < 0.5 {
        (0.3, WeightReason::NoData)
    } else if snap.consistency_ratio > 0.9 {
        (1.2, WeightReason::NoData)
    } else {
        (1.0, WeightReason::NoData)
    }
}

impl RoutingAdvisor for RoutingIntelligence {
    fn compute_weight_adjustments(
        &self,
        _source: &MatrixPosition,
        _destination: &MatrixPosition,
        candidates: &[NodeId],
    ) -> Vec<TensorWeightModifier> {
        candidates
            .iter()
            .map(|c| self.weight_for_candidate(c))
            .collect()
    }
}

impl PathAdvisor for RoutingIntelligence {
    fn recommend_path_policy(&self, aggregate: &RegionalAggregate) -> PathPolicyRecommendation {
        if aggregate.node_count == 0 {
            return PathPolicyRecommendation {
                enable_redundant: false,
                strategy: SchedulingStrategy::RoundRobin,
                congestion_level: 0.0,
            };
        }

        let congestion = aggregate.avg_buffer_fullness;

        if congestion > CONGESTION_HIGH {
            PathPolicyRecommendation {
                enable_redundant: true,
                strategy: SchedulingStrategy::Redundant,
                congestion_level: congestion,
            }
        } else if congestion > CONGESTION_LOW {
            PathPolicyRecommendation {
                enable_redundant: false,
                strategy: SchedulingStrategy::LowestLatency,
                congestion_level: congestion,
            }
        } else {
            PathPolicyRecommendation {
                enable_redundant: false,
                strategy: SchedulingStrategy::BandwidthWeighted,
                congestion_level: congestion,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::protocol::*;
    use hypermesh_lib::PrivacyMode;

    fn make_frame(node: &str, payload: MetricsPayload) -> MetricsFrame {
        MetricsFrame {
            source_node: NodeId::from_public_key(node.as_bytes()),
            timestamp_us: 1000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload,
            sequence: 1,
        }
    }

    fn make_congestion_frame(node: &str, fullness: f64) -> MetricsFrame {
        make_frame(
            node,
            MetricsPayload::Congestion(CongestionSnapshot {
                buffer_fullness_ratio: fullness,
                queue_depth: 10,
                dropped_packets_epoch: 0,
                avg_queue_wait_us: 100,
            }),
        )
    }

    fn make_routing_frame(node: &str, latency_us: u64, throughput_bps: u64) -> MetricsFrame {
        make_frame(
            node,
            MetricsPayload::Routing(RoutingSnapshot {
                avg_latency_us: latency_us,
                throughput_bps,
                path_count: 3,
                active_connections: 10,
            }),
        )
    }

    fn make_capacity_frame(node: &str, bandwidth_bps: u64) -> MetricsFrame {
        make_frame(
            node,
            MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: 1000,
                compute_delivered: 500,
                storage_maintained_bytes: 10000,
                bandwidth_available_bps: bandwidth_bps,
                uptime_ratio: 0.99,
            }),
        )
    }

    fn origin() -> MatrixPosition {
        MatrixPosition {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
    fn dest() -> MatrixPosition {
        MatrixPosition {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }
    }

    #[test]
    fn no_data_returns_neutral_weight() {
        let intel = RoutingIntelligence::new(30);
        let candidates = vec![NodeId::from_public_key(b"unknown-node")];

        let weights = intel.compute_weight_adjustments(&origin(), &dest(), &candidates);
        assert_eq!(weights.len(), 1);
        assert!((weights[0].weight_factor - 1.0).abs() < 1e-6);
        assert_eq!(weights[0].reason, WeightReason::NoData);
    }

    #[test]
    fn high_congestion_lowers_weight() {
        let mut intel = RoutingIntelligence::new(30);
        intel.ingest(make_congestion_frame("congested", 0.9));

        let candidates = vec![NodeId::from_public_key(b"congested")];
        let weights = intel.compute_weight_adjustments(&origin(), &dest(), &candidates);

        assert_eq!(weights.len(), 1);
        assert!(
            weights[0].weight_factor < 0.5,
            "high congestion should lower weight"
        );
        assert_eq!(weights[0].reason, WeightReason::HighCongestion);
    }

    #[test]
    fn low_congestion_raises_weight() {
        let mut intel = RoutingIntelligence::new(30);
        intel.ingest(make_congestion_frame("clear", 0.1));

        let candidates = vec![NodeId::from_public_key(b"clear")];
        let weights = intel.compute_weight_adjustments(&origin(), &dest(), &candidates);

        assert!(
            weights[0].weight_factor > 1.0,
            "low congestion should raise weight"
        );
        assert_eq!(weights[0].reason, WeightReason::LowCongestion);
    }

    #[test]
    fn high_latency_lowers_weight() {
        let mut intel = RoutingIntelligence::new(30);
        intel.ingest(make_routing_frame("slow", 100_000, 500_000_000));

        let candidates = vec![NodeId::from_public_key(b"slow")];
        let weights = intel.compute_weight_adjustments(&origin(), &dest(), &candidates);

        assert!(
            weights[0].weight_factor < 1.0,
            "high latency should lower weight"
        );
        assert_eq!(weights[0].reason, WeightReason::HighLatency);
    }

    #[test]
    fn low_latency_high_throughput_raises_weight() {
        let mut intel = RoutingIntelligence::new(30);
        intel.ingest(make_routing_frame("fast", 1_000, 2_000_000_000));

        let candidates = vec![NodeId::from_public_key(b"fast")];
        let weights = intel.compute_weight_adjustments(&origin(), &dest(), &candidates);

        assert!(
            weights[0].weight_factor > 1.5,
            "low latency + high throughput should raise weight"
        );
        assert_eq!(weights[0].reason, WeightReason::LowLatency);
    }

    #[test]
    fn capacity_bandwidth_affects_weight() {
        let mut intel = RoutingIntelligence::new(30);
        intel.ingest(make_capacity_frame("high-bw", 2_000_000_000)); // 2 Gbps
        intel.ingest(make_capacity_frame("low-bw", 10_000_000)); // 10 Mbps

        let candidates = vec![NodeId::from_public_key(b"high-bw"), NodeId::from_public_key(b"low-bw")];
        let weights = intel.compute_weight_adjustments(&origin(), &dest(), &candidates);

        assert!(
            weights[0].weight_factor > weights[1].weight_factor,
            "high bandwidth node should have higher weight"
        );
    }

    #[test]
    fn multiple_candidates_independent() {
        let mut intel = RoutingIntelligence::new(30);
        intel.ingest(make_congestion_frame("a", 0.1));
        intel.ingest(make_congestion_frame("b", 0.9));

        let candidates = vec![NodeId::from_public_key(b"a"), NodeId::from_public_key(b"b"), NodeId::from_public_key(b"c")];
        let weights = intel.compute_weight_adjustments(&origin(), &dest(), &candidates);

        assert_eq!(weights.len(), 3);
        assert!(weights[0].weight_factor > 1.0, "a should be preferred");
        assert!(weights[1].weight_factor < 0.5, "b should be penalized");
        assert!(
            (weights[2].weight_factor - 1.0).abs() < 1e-6,
            "c should be neutral"
        );
    }

    #[test]
    fn empty_aggregate_recommends_round_robin() {
        let intel = RoutingIntelligence::new(30);
        let agg = RegionalAggregate {
            node_count: 0,
            avg_buffer_fullness: 0.0,
            avg_latency_us: 0.0,
            avg_throughput_bps: 0.0,
            total_bandwidth_bps: 0,
            avg_capacity_score: 0.0,
            verified_node_count: 0,
            avg_consistency_ratio: 0.0,
        };

        let rec = intel.recommend_path_policy(&agg);
        assert_eq!(rec.strategy, SchedulingStrategy::RoundRobin);
        assert!(!rec.enable_redundant);
    }

    #[test]
    fn high_congestion_recommends_redundant() {
        let intel = RoutingIntelligence::new(30);
        let agg = RegionalAggregate {
            node_count: 5,
            avg_buffer_fullness: 0.85,
            avg_latency_us: 10_000.0,
            avg_throughput_bps: 500_000_000.0,
            total_bandwidth_bps: 2_500_000_000,
            avg_capacity_score: 0.5,
            verified_node_count: 0,
            avg_consistency_ratio: 0.0,
        };

        let rec = intel.recommend_path_policy(&agg);
        assert_eq!(rec.strategy, SchedulingStrategy::Redundant);
        assert!(rec.enable_redundant);
        assert!(rec.congestion_level > 0.7);
    }

    #[test]
    fn moderate_congestion_recommends_lowest_latency() {
        let intel = RoutingIntelligence::new(30);
        let agg = RegionalAggregate {
            node_count: 3,
            avg_buffer_fullness: 0.5,
            avg_latency_us: 8_000.0,
            avg_throughput_bps: 800_000_000.0,
            total_bandwidth_bps: 2_400_000_000,
            avg_capacity_score: 0.7,
            verified_node_count: 0,
            avg_consistency_ratio: 0.0,
        };

        let rec = intel.recommend_path_policy(&agg);
        assert_eq!(rec.strategy, SchedulingStrategy::LowestLatency);
        assert!(!rec.enable_redundant);
    }

    #[test]
    fn low_congestion_recommends_bandwidth_weighted() {
        let intel = RoutingIntelligence::new(30);
        let agg = RegionalAggregate {
            node_count: 4,
            avg_buffer_fullness: 0.1,
            avg_latency_us: 2_000.0,
            avg_throughput_bps: 1_500_000_000.0,
            total_bandwidth_bps: 6_000_000_000,
            avg_capacity_score: 0.9,
            verified_node_count: 0,
            avg_consistency_ratio: 0.0,
        };

        let rec = intel.recommend_path_policy(&agg);
        assert_eq!(rec.strategy, SchedulingStrategy::BandwidthWeighted);
        assert!(!rec.enable_redundant);
    }
}
