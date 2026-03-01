// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Per-node capacity metrics (whitepaper section 17).
//!
//! **Capacity only** -- no trust scores, no reputation.
//! A [`CapacityReport`] records what a node has *delivered* in a given epoch,
//! normalized into a single [`CapacityScore`].

use chrono::{DateTime, Utc};
use hypermesh_lib::NodeId;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// CapacityMetrics
// ---------------------------------------------------------------------------

/// Raw capacity measurements for a single node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityMetrics {
    /// Total bytes served to requestors.
    pub bytes_served: u64,
    /// Total compute units delivered (abstract: cycles, FLOPS, etc.).
    pub compute_delivered: u64,
    /// Bytes of storage currently maintained for the network.
    pub storage_maintained_bytes: u64,
    /// Available bandwidth in bits per second.
    pub bandwidth_available_bps: u64,
    /// Uptime ratio over the measurement period (0.0..1.0, stored as millionths).
    ///
    /// Stored as integer millionths (0 = 0%, 1_000_000 = 100%) to keep `Eq`.
    /// Use [`Self::uptime_f64`] for the floating-point value.
    uptime_millionths: u32,
}

impl CapacityMetrics {
    /// Create metrics with an uptime ratio (clamped to 0.0..1.0).
    pub fn new(
        bytes_served: u64,
        compute_delivered: u64,
        storage_maintained_bytes: u64,
        bandwidth_available_bps: u64,
        uptime_ratio: f64,
    ) -> Self {
        let clamped = uptime_ratio.clamp(0.0, 1.0);
        let millionths = (clamped * 1_000_000.0) as u32;
        Self {
            bytes_served,
            compute_delivered,
            storage_maintained_bytes,
            bandwidth_available_bps,
            uptime_millionths: millionths,
        }
    }

    /// Uptime ratio as f64 (0.0..1.0).
    pub fn uptime_f64(&self) -> f64 {
        self.uptime_millionths as f64 / 1_000_000.0
    }
}

// ---------------------------------------------------------------------------
// CapacityScore
// ---------------------------------------------------------------------------

/// Normalized single score derived from [`CapacityMetrics`].
///
/// Weights:
/// - bytes_served: 0.25
/// - compute_delivered: 0.25
/// - storage_maintained: 0.20
/// - bandwidth_available: 0.20
/// - uptime: 0.10
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CapacityScore {
    raw: f64,
}

impl CapacityScore {
    /// Baselines against which raw metrics are normalized.
    const BYTES_BASELINE: f64 = 1_073_741_824.0; // 1 GiB
    const COMPUTE_BASELINE: f64 = 1_000_000.0;
    const STORAGE_BASELINE: f64 = 10_737_418_240.0; // 10 GiB
    const BANDWIDTH_BASELINE: f64 = 1_000_000_000.0; // 1 Gbps

    /// Calculate a score from raw metrics.
    pub fn calculate(metrics: &CapacityMetrics) -> Self {
        let bytes = (metrics.bytes_served as f64 / Self::BYTES_BASELINE).clamp(0.0, 1.0);
        let compute = (metrics.compute_delivered as f64 / Self::COMPUTE_BASELINE).clamp(0.0, 1.0);
        let storage =
            (metrics.storage_maintained_bytes as f64 / Self::STORAGE_BASELINE).clamp(0.0, 1.0);
        let bandwidth =
            (metrics.bandwidth_available_bps as f64 / Self::BANDWIDTH_BASELINE).clamp(0.0, 1.0);
        let uptime = metrics.uptime_f64();

        let raw = bytes * 0.25 + compute * 0.25 + storage * 0.20 + bandwidth * 0.20 + uptime * 0.10;
        Self { raw }
    }

    /// The score value (0.0..1.0).
    pub fn value(&self) -> f64 {
        self.raw
    }
}

// ---------------------------------------------------------------------------
// CapacityReport
// ---------------------------------------------------------------------------

/// A complete capacity report for a node in a given epoch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityReport {
    /// The node being measured.
    pub node_id: NodeId,
    /// Raw capacity measurements.
    pub metrics: CapacityMetrics,
    /// Derived capacity score.
    pub score: CapacityScore,
    /// When the measurement was taken.
    pub measured_at: DateTime<Utc>,
    /// Epoch number (monotonically increasing measurement period).
    pub epoch: u64,
}

impl CapacityReport {
    /// Build a report from raw metrics for a given node and epoch.
    pub fn new(node_id: NodeId, metrics: CapacityMetrics, epoch: u64) -> Self {
        let score = CapacityScore::calculate(&metrics);
        Self {
            node_id,
            metrics,
            score,
            measured_at: Utc::now(),
            epoch,
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node() -> NodeId {
        NodeId::from_public_key(b"capacity-node-001")
    }

    fn full_metrics() -> CapacityMetrics {
        CapacityMetrics::new(
            1_073_741_824,  // 1 GiB served
            1_000_000,      // full compute
            10_737_418_240, // 10 GiB storage
            1_000_000_000,  // 1 Gbps
            1.0,            // 100% uptime
        )
    }

    fn zero_metrics() -> CapacityMetrics {
        CapacityMetrics::new(0, 0, 0, 0, 0.0)
    }

    #[test]
    fn full_metrics_score_is_one() {
        let score = CapacityScore::calculate(&full_metrics());
        assert!(
            (score.value() - 1.0).abs() < 1e-6,
            "full score: {}",
            score.value()
        );
    }

    #[test]
    fn zero_metrics_score_is_zero() {
        let score = CapacityScore::calculate(&zero_metrics());
        assert!(
            (score.value()).abs() < 1e-6,
            "zero score: {}",
            score.value()
        );
    }

    #[test]
    fn partial_metrics_score() {
        let metrics = CapacityMetrics::new(
            536_870_912,   // 0.5 GiB
            500_000,       // half compute
            5_368_709_120, // 5 GiB storage
            500_000_000,   // 500 Mbps
            0.9,
        );
        let score = CapacityScore::calculate(&metrics);
        // bytes: 0.5*0.25=0.125, compute: 0.5*0.25=0.125, storage: 0.5*0.20=0.10,
        // bandwidth: 0.5*0.20=0.10, uptime: 0.9*0.10=0.09 => ~0.54
        assert!(
            score.value() > 0.4 && score.value() < 0.7,
            "score: {}",
            score.value()
        );
    }

    #[test]
    fn uptime_clamped() {
        let m = CapacityMetrics::new(0, 0, 0, 0, 1.5); // over 1.0
        assert!((m.uptime_f64() - 1.0).abs() < 1e-3);

        let m = CapacityMetrics::new(0, 0, 0, 0, -0.5); // under 0.0
        assert!(m.uptime_f64().abs() < 1e-3);
    }

    #[test]
    fn above_baseline_clamped_to_one() {
        let metrics = CapacityMetrics::new(u64::MAX, u64::MAX, u64::MAX, u64::MAX, 1.0);
        let score = CapacityScore::calculate(&metrics);
        assert!(
            (score.value() - 1.0).abs() < 1e-6,
            "over-baseline score: {}",
            score.value()
        );
    }

    #[test]
    fn report_new_calculates_score() {
        let metrics = full_metrics();
        let report = CapacityReport::new(test_node(), metrics.clone(), 42);
        assert_eq!(report.epoch, 42);
        assert_eq!(report.node_id, test_node());
        assert!((report.score.value() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn report_serde_roundtrip() {
        let report = CapacityReport::new(test_node(), full_metrics(), 7);
        let json = serde_json::to_string(&report).expect("test: serialize report");
        let back: CapacityReport = serde_json::from_str(&json).expect("test: deserialize report");
        assert_eq!(back.epoch, 7);
        assert_eq!(back.node_id, test_node());
        assert!((back.score.value() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn capacity_metrics_equality() {
        let a = CapacityMetrics::new(100, 200, 300, 400, 0.95);
        let b = CapacityMetrics::new(100, 200, 300, 400, 0.95);
        assert_eq!(a, b);
    }

    #[test]
    fn weight_distribution_sums_to_one() {
        // 0.25 + 0.25 + 0.20 + 0.20 + 0.10 = 1.0
        let sum = 0.25_f64 + 0.25 + 0.20 + 0.20 + 0.10;
        assert!((sum - 1.0).abs() < 1e-9);
    }
}
