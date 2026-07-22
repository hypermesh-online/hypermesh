// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Collective network intelligence aggregation.
//!
//! Builds on [`RegionalAggregator`] to produce network-wide insights
//! including capacity reports, hotspot alerts, coverage gaps, and
//! economic summaries. Privacy-aware: Anonymous nodes contribute only
//! aggregate stats.

use std::collections::HashMap;

use hypermesh_lib::{MatrixPosition, NodeId, PrivacyMode};
use serde::{Deserialize, Serialize};

use crate::streaming::aggregator::RegionalAggregator;
use crate::streaming::protocol::{
    EconomicSnapshot, MetricsFrame, MetricsPayload,
};

// ---------------------------------------------------------------------------
// NetworkInsight
// ---------------------------------------------------------------------------

/// Network-wide insight produced by collective intelligence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkInsight {
    /// Overall capacity report for the network.
    CapacityReport(CapacityReport),
    /// Alert about a congested region.
    HotspotAlert(HotspotAlert),
    /// Alert about an underserved region.
    CoverageGap(CoverageGap),
    /// Network-wide economic summary.
    EconomicSummary(EconomicSummary),
}

/// Network-wide capacity report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityReport {
    /// Total nodes contributing data.
    pub total_nodes: usize,
    /// Total available bandwidth across network (bps).
    pub total_bandwidth_bps: u64,
    /// Average capacity score (0.0 to 1.0).
    pub avg_capacity_score: f64,
    /// Average uptime ratio.
    pub avg_uptime: f64,
    /// Number of nodes with verified spatial positions.
    pub verified_nodes: usize,
}

/// Alert about a congested network region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotAlert {
    /// Approximate center of the congested region.
    pub center: MatrixPosition,
    /// Average congestion ratio in the hotspot (0.0 to 1.0).
    pub congestion_ratio: f64,
    /// Number of nodes in the congested region.
    pub affected_nodes: usize,
    /// Severity: "low", "medium", "high".
    pub severity: String,
}

/// Alert about an underserved region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageGap {
    /// Approximate center of the gap.
    pub center: MatrixPosition,
    /// Radius of the underserved region (matrix units).
    pub radius: f64,
    /// Number of nodes in this region (low is the problem).
    pub node_count: usize,
}

/// Network-wide economic summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicSummary {
    /// Total in-flight value across network (gold-grams).
    pub total_in_flight_grams: f64,
    /// Network-wide settlement rate (per epoch).
    pub settlement_rate: f64,
    /// Total active packets across network.
    pub total_active_packets: u32,
    /// Number of nodes reporting economic data.
    pub reporting_nodes: usize,
}

// ---------------------------------------------------------------------------
// CollectiveIntelligence
// ---------------------------------------------------------------------------

/// Aggregates metrics across multiple nodes to produce network-wide insights.
///
/// Privacy-aware: Anonymous node data is counted but not individually tracked.
/// Private nodes contribute within their federation scope.
/// Public nodes contribute full details.
pub struct CollectiveIntelligence {
    /// Regional aggregator for capacity/congestion/routing metrics.
    aggregator: RegionalAggregator,
    /// Per-node positions (only for tracked nodes).
    node_positions: HashMap<NodeId, MatrixPosition>,
    /// Per-node economic snapshots.
    economic_data: HashMap<NodeId, EconomicSnapshot>,
    /// Anonymous node count (we count them but don't track individually).
    anonymous_node_count: usize,
    /// Congestion threshold for hotspot detection.
    hotspot_threshold: f64,
    /// Minimum node count for coverage gap detection.
    coverage_min_nodes: usize,
}

impl CollectiveIntelligence {
    /// Create a new collective intelligence engine.
    ///
    /// - `window_size`: per-source frame window for the aggregator.
    /// - `hotspot_threshold`: congestion ratio above which a hotspot is reported.
    /// - `coverage_min_nodes`: minimum nodes per region below which a gap is detected.
    pub fn new(window_size: usize, hotspot_threshold: f64, coverage_min_nodes: usize) -> Self {
        Self {
            aggregator: RegionalAggregator::new(window_size),
            node_positions: HashMap::new(),
            economic_data: HashMap::new(),
            anonymous_node_count: 0,
            hotspot_threshold,
            coverage_min_nodes,
        }
    }

    /// Create with sensible defaults.
    pub fn with_defaults() -> Self {
        Self::new(30, 0.7, 2)
    }

    /// Ingest a metrics frame.
    ///
    /// Anonymous frames are counted but not individually tracked.
    /// Private/Public frames are fully aggregated.
    pub fn ingest(&mut self, frame: MetricsFrame) {
        if frame.privacy_mode == PrivacyMode::ANONYMOUS {
            self.anonymous_node_count += 1;
            return;
        }

        // Track economic data separately.
        if let MetricsPayload::Economic(ref econ) = frame.payload {
            self.economic_data
                .insert(frame.source_node, econ.clone());
        }

        self.aggregator.ingest(frame);
    }

    /// Register a node's matrix position (for spatial analysis).
    pub fn register_position(&mut self, node_id: NodeId, position: MatrixPosition) {
        self.node_positions.insert(node_id, position);
    }

    /// Produce all current network insights.
    pub fn generate_insights(&self) -> Vec<NetworkInsight> {
        let mut insights = Vec::new();

        // 1. Capacity report.
        insights.push(NetworkInsight::CapacityReport(self.capacity_report()));

        // 2. Hotspot detection.
        if let Some(hotspot) = self.detect_hotspot() {
            insights.push(NetworkInsight::HotspotAlert(hotspot));
        }

        // 3. Coverage gap detection.
        if let Some(gap) = self.detect_coverage_gap() {
            insights.push(NetworkInsight::CoverageGap(gap));
        }

        // 4. Economic summary.
        if !self.economic_data.is_empty() {
            insights.push(NetworkInsight::EconomicSummary(self.economic_summary()));
        }

        insights
    }

    /// Generate a capacity report from aggregated data.
    pub fn capacity_report(&self) -> CapacityReport {
        let agg = self.aggregator.aggregate();
        CapacityReport {
            total_nodes: agg.node_count + self.anonymous_node_count,
            total_bandwidth_bps: agg.total_bandwidth_bps,
            avg_capacity_score: agg.avg_capacity_score,
            avg_uptime: 0.0, // Not tracked in aggregate currently.
            verified_nodes: agg.verified_node_count,
        }
    }

    /// Detect congestion hotspots.
    fn detect_hotspot(&self) -> Option<HotspotAlert> {
        let agg = self.aggregator.aggregate();

        if agg.node_count == 0 || agg.avg_buffer_fullness < self.hotspot_threshold {
            return None;
        }

        let severity = if agg.avg_buffer_fullness > 0.9 {
            "high"
        } else if agg.avg_buffer_fullness > 0.8 {
            "medium"
        } else {
            "low"
        };

        // Compute center from known positions.
        let center = self.compute_centroid();

        Some(HotspotAlert {
            center,
            congestion_ratio: agg.avg_buffer_fullness,
            affected_nodes: agg.node_count,
            severity: severity.to_string(),
        })
    }

    /// Detect coverage gaps (regions with too few nodes).
    fn detect_coverage_gap(&self) -> Option<CoverageGap> {
        if self.node_positions.len() < self.coverage_min_nodes {
            let center = self.compute_centroid();
            return Some(CoverageGap {
                center,
                radius: 10.0, // Default radius for gap detection.
                node_count: self.node_positions.len(),
            });
        }
        None
    }

    /// Generate economic summary from tracked nodes.
    fn economic_summary(&self) -> EconomicSummary {
        let mut total_float = 0.0;
        let mut total_rate = 0.0;
        let mut total_packets = 0u32;

        for econ in self.economic_data.values() {
            total_float += econ.in_flight_float_grams;
            total_rate += econ.settlement_rate_per_epoch;
            total_packets += econ.active_packets;
        }

        EconomicSummary {
            total_in_flight_grams: total_float,
            settlement_rate: total_rate,
            total_active_packets: total_packets,
            reporting_nodes: self.economic_data.len(),
        }
    }

    /// Compute centroid of known node positions.
    fn compute_centroid(&self) -> MatrixPosition {
        if self.node_positions.is_empty() {
            return MatrixPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            };
        }
        let count = self.node_positions.len() as f64;
        let sx: f64 = self.node_positions.values().map(|p| p.x).sum();
        let sy: f64 = self.node_positions.values().map(|p| p.y).sum();
        let sz: f64 = self.node_positions.values().map(|p| p.z).sum();
        MatrixPosition {
            x: sx / count,
            y: sy / count,
            z: sz / count,
        }
    }

    /// Number of tracked source nodes (excluding anonymous).
    pub fn tracked_node_count(&self) -> usize {
        self.aggregator.source_count()
    }

    /// Number of anonymous nodes counted.
    pub fn anonymous_node_count(&self) -> usize {
        self.anonymous_node_count
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::protocol::{
        CapacitySnapshot, CongestionSnapshot, MetricsPayload,
    };

    fn test_node(name: &str) -> NodeId {
        NodeId::from_public_key(name.as_bytes())
    }

    fn capacity_frame(node: &str, bandwidth: u64) -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(node),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: 1024,
                compute_delivered: 500,
                storage_maintained_bytes: 2048,
                bandwidth_available_bps: bandwidth,
                uptime_ratio: 0.99,
            }),
            sequence: 0,
        }
    }

    fn congestion_frame(node: &str, fullness: f64) -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(node),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Congestion(CongestionSnapshot {
                buffer_fullness_ratio: fullness,
                queue_depth: 10,
                dropped_packets_epoch: 0,
                avg_queue_wait_us: 50,
            }),
            sequence: 0,
        }
    }

    fn economic_frame(node: &str, float_grams: f64) -> MetricsFrame {
        MetricsFrame {
            source_node: test_node(node),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::PUBLIC,
            payload: MetricsPayload::Economic(EconomicSnapshot {
                in_flight_float_grams: float_grams,
                settlement_rate_per_epoch: 5.0,
                active_packets: 3,
                ..Default::default()
            }),
            sequence: 0,
        }
    }

    #[test]
    fn capacity_report_aggregates_nodes() {
        let mut intel = CollectiveIntelligence::with_defaults();

        intel.ingest(capacity_frame("node-a", 1_000_000_000));
        intel.ingest(capacity_frame("node-b", 2_000_000_000));

        let report = intel.capacity_report();
        assert_eq!(report.total_nodes, 2);
        assert_eq!(report.total_bandwidth_bps, 3_000_000_000);
    }

    #[test]
    fn anonymous_nodes_counted_not_tracked() {
        let mut intel = CollectiveIntelligence::with_defaults();

        let anon_frame = MetricsFrame {
            source_node: test_node("anon-1"),
            timestamp_us: 1_000_000,
            privacy_mode: PrivacyMode::ANONYMOUS,
            payload: MetricsPayload::Capacity(CapacitySnapshot {
                bytes_served: 1024,
                compute_delivered: 500,
                storage_maintained_bytes: 2048,
                bandwidth_available_bps: 1_000_000,
                uptime_ratio: 0.99,
            }),
            sequence: 0,
        };

        intel.ingest(anon_frame);
        intel.ingest(capacity_frame("public-1", 1_000_000));

        assert_eq!(intel.anonymous_node_count(), 1);
        assert_eq!(intel.tracked_node_count(), 1);

        let report = intel.capacity_report();
        assert_eq!(report.total_nodes, 2, "anonymous + tracked");
    }

    #[test]
    fn hotspot_detection_when_congested() {
        let mut intel = CollectiveIntelligence::new(30, 0.7, 2);

        intel.ingest(congestion_frame("hot-a", 0.85));
        intel.ingest(congestion_frame("hot-b", 0.90));

        let insights = intel.generate_insights();
        let hotspots: Vec<_> = insights
            .iter()
            .filter(|i| matches!(i, NetworkInsight::HotspotAlert(_)))
            .collect();

        assert_eq!(hotspots.len(), 1, "should detect hotspot");
        if let NetworkInsight::HotspotAlert(alert) = &hotspots[0] {
            assert!(alert.congestion_ratio > 0.7);
            assert_eq!(alert.affected_nodes, 2);
        }
    }

    #[test]
    fn economic_summary_from_public_nodes() {
        let mut intel = CollectiveIntelligence::with_defaults();

        intel.ingest(economic_frame("eco-a", 100.0));
        intel.ingest(economic_frame("eco-b", 50.0));

        let insights = intel.generate_insights();
        let summaries: Vec<_> = insights
            .iter()
            .filter(|i| matches!(i, NetworkInsight::EconomicSummary(_)))
            .collect();

        assert_eq!(summaries.len(), 1);
        if let NetworkInsight::EconomicSummary(summary) = &summaries[0] {
            assert!((summary.total_in_flight_grams - 150.0).abs() < 1e-9);
            assert_eq!(summary.reporting_nodes, 2);
        }
    }
}
