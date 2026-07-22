// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Metrics collection for Governor data feed.
//!
//! [`MetricsCollector`] accumulates per-node activity measurements (compute
//! cycles, bandwidth, latency, receipt counts) and derives an
//! [`ActivityScore`] -- a normalized composite used by the Governor to
//! calibrate band pricing and demurrage rates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Configuration baselines
// ---------------------------------------------------------------------------

/// Baseline values used to normalize raw metrics into 0..1 scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsBaseline {
    /// Expected compute cycles per measurement period.
    pub compute_cycles: u64,
    /// Expected bandwidth bytes per measurement period.
    pub bandwidth_bytes: u64,
    /// Target latency in milliseconds (lower is better).
    pub target_latency_ms: f64,
    /// Expected receipt count per measurement period.
    pub receipt_count: u64,
}

impl Default for MetricsBaseline {
    fn default() -> Self {
        Self {
            compute_cycles: 1_000_000,
            bandwidth_bytes: 1_073_741_824, // 1 GiB
            target_latency_ms: 50.0,
            receipt_count: 1_000,
        }
    }
}

// ---------------------------------------------------------------------------
// ActivityScore
// ---------------------------------------------------------------------------

/// Composite activity score derived from raw metrics.
///
/// Each axis is normalized to 0.0..1.0.  The [`composite`](Self::composite)
/// method returns a weighted average.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActivityScore {
    /// Normalized compute utilization (0.0..1.0).
    pub compute_score: f64,
    /// Normalized bandwidth utilization (0.0..1.0).
    pub bandwidth_score: f64,
    /// Normalized latency quality (0.0..1.0, higher is better).
    pub latency_score: f64,
    /// Receipt density -- receipts relative to baseline (0.0..1.0).
    pub receipt_density: f64,
}

impl ActivityScore {
    /// Weighted average: compute 0.3, bandwidth 0.3, latency 0.2, receipts 0.2.
    pub fn composite(&self) -> f64 {
        self.compute_score * 0.3
            + self.bandwidth_score * 0.3
            + self.latency_score * 0.2
            + self.receipt_density * 0.2
    }
}

// ---------------------------------------------------------------------------
// MetricsSnapshot
// ---------------------------------------------------------------------------

/// Point-in-time frozen metrics for Governor consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// When the snapshot was taken.
    pub captured_at: DateTime<Utc>,
    /// Total compute cycles recorded.
    pub compute_cycles: u64,
    /// Total bandwidth bytes recorded.
    pub bandwidth_bytes: u64,
    /// Average latency in milliseconds (NaN-safe: 0.0 when no samples).
    pub avg_latency_ms: f64,
    /// Number of latency samples.
    pub latency_sample_count: usize,
    /// Number of receipts recorded.
    pub receipt_count: u64,
    /// When collection started.
    pub active_since: DateTime<Utc>,
    /// Derived activity score.
    pub activity_score: ActivityScore,
}

// ---------------------------------------------------------------------------
// MetricsCollector
// ---------------------------------------------------------------------------

/// Accumulates per-node capacity metrics and derives activity scores.
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    compute_cycles: u64,
    bandwidth_bytes: u64,
    latency_samples: Vec<f64>,
    receipt_count: u64,
    active_since: DateTime<Utc>,
    baseline: MetricsBaseline,
}

impl MetricsCollector {
    /// Create a new collector with default baselines.
    pub fn new() -> Self {
        Self {
            compute_cycles: 0,
            bandwidth_bytes: 0,
            latency_samples: Vec::new(),
            receipt_count: 0,
            active_since: Utc::now(),
            baseline: MetricsBaseline::default(),
        }
    }

    /// Create a collector with custom baselines.
    pub fn with_baseline(baseline: MetricsBaseline) -> Self {
        Self {
            compute_cycles: 0,
            bandwidth_bytes: 0,
            latency_samples: Vec::new(),
            receipt_count: 0,
            active_since: Utc::now(),
            baseline,
        }
    }

    /// Record compute cycles.
    pub fn record_compute(&mut self, cycles: u64) {
        self.compute_cycles = self.compute_cycles.saturating_add(cycles);
    }

    /// Record bandwidth consumption.
    pub fn record_bandwidth(&mut self, bytes: u64) {
        self.bandwidth_bytes = self.bandwidth_bytes.saturating_add(bytes);
    }

    /// Record a latency sample in milliseconds.
    pub fn record_latency(&mut self, ms: f64) {
        self.latency_samples.push(ms);
    }

    /// Record that a content receipt was produced.
    pub fn record_receipt(&mut self) {
        self.receipt_count = self.receipt_count.saturating_add(1);
    }

    /// Calculate the current activity score from accumulated data.
    pub fn activity_score(&self) -> ActivityScore {
        let compute_score = normalize(self.compute_cycles, self.baseline.compute_cycles);
        let bandwidth_score = normalize(self.bandwidth_bytes, self.baseline.bandwidth_bytes);
        let latency_score = self.latency_quality();
        let receipt_density = normalize(self.receipt_count, self.baseline.receipt_count);

        ActivityScore {
            compute_score,
            bandwidth_score,
            latency_score,
            receipt_density,
        }
    }

    /// Freeze current state into an immutable snapshot.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let avg_latency_ms = if self.latency_samples.is_empty() {
            0.0
        } else {
            let sum: f64 = self.latency_samples.iter().sum();
            sum / self.latency_samples.len() as f64
        };

        MetricsSnapshot {
            captured_at: Utc::now(),
            compute_cycles: self.compute_cycles,
            bandwidth_bytes: self.bandwidth_bytes,
            avg_latency_ms,
            latency_sample_count: self.latency_samples.len(),
            receipt_count: self.receipt_count,
            active_since: self.active_since,
            activity_score: self.activity_score(),
        }
    }

    /// Reset all counters (e.g. at the start of a new measurement window).
    pub fn reset(&mut self) {
        self.compute_cycles = 0;
        self.bandwidth_bytes = 0;
        self.latency_samples.clear();
        self.receipt_count = 0;
        self.active_since = Utc::now();
    }

    // -- internal helpers ---------------------------------------------------

    /// Latency quality: 1.0 when avg is at or below target, decays toward 0.
    fn latency_quality(&self) -> f64 {
        if self.latency_samples.is_empty() {
            return 0.5; // no data -> neutral
        }
        let sum: f64 = self.latency_samples.iter().sum();
        let avg = sum / self.latency_samples.len() as f64;
        if avg <= 0.0 {
            return 1.0;
        }
        // ratio = target / avg.  Clamp to [0, 1].
        let ratio = self.baseline.target_latency_ms / avg;
        ratio.clamp(0.0, 1.0)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Normalize a value against a baseline, clamped to 0.0..1.0.
fn normalize(value: u64, baseline: u64) -> f64 {
    if baseline == 0 {
        return 0.0;
    }
    let ratio = value as f64 / baseline as f64;
    ratio.clamp(0.0, 1.0)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_collector_zero_state() {
        let c = MetricsCollector::new();
        assert_eq!(c.compute_cycles, 0);
        assert_eq!(c.bandwidth_bytes, 0);
        assert!(c.latency_samples.is_empty());
        assert_eq!(c.receipt_count, 0);
    }

    #[test]
    fn record_compute_accumulates() {
        let mut c = MetricsCollector::new();
        c.record_compute(100);
        c.record_compute(200);
        assert_eq!(c.compute_cycles, 300);
    }

    #[test]
    fn record_bandwidth_accumulates() {
        let mut c = MetricsCollector::new();
        c.record_bandwidth(1024);
        c.record_bandwidth(2048);
        assert_eq!(c.bandwidth_bytes, 3072);
    }

    #[test]
    fn record_latency_collects_samples() {
        let mut c = MetricsCollector::new();
        c.record_latency(10.0);
        c.record_latency(20.0);
        c.record_latency(30.0);
        assert_eq!(c.latency_samples.len(), 3);
    }

    #[test]
    fn record_receipt_counts() {
        let mut c = MetricsCollector::new();
        c.record_receipt();
        c.record_receipt();
        c.record_receipt();
        assert_eq!(c.receipt_count, 3);
    }

    #[test]
    fn activity_score_zero_metrics() {
        let c = MetricsCollector::new();
        let score = c.activity_score();
        assert_eq!(score.compute_score, 0.0);
        assert_eq!(score.bandwidth_score, 0.0);
        // No latency data -> neutral 0.5
        assert!((score.latency_score - 0.5).abs() < 1e-9);
        assert_eq!(score.receipt_density, 0.0);
    }

    #[test]
    fn activity_score_full_baseline() {
        let baseline = MetricsBaseline::default();
        let mut c = MetricsCollector::with_baseline(baseline.clone());
        c.record_compute(baseline.compute_cycles);
        c.record_bandwidth(baseline.bandwidth_bytes);
        c.record_latency(baseline.target_latency_ms);
        for _ in 0..baseline.receipt_count {
            c.record_receipt();
        }

        let score = c.activity_score();
        assert!((score.compute_score - 1.0).abs() < 1e-9);
        assert!((score.bandwidth_score - 1.0).abs() < 1e-9);
        assert!((score.latency_score - 1.0).abs() < 1e-9);
        assert!((score.receipt_density - 1.0).abs() < 1e-9);
        assert!((score.composite() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn composite_weight_check() {
        // Manually set known scores and verify weighted average
        let score = ActivityScore {
            compute_score: 1.0,
            bandwidth_score: 0.0,
            latency_score: 0.5,
            receipt_density: 0.5,
        };
        // 1.0*0.3 + 0.0*0.3 + 0.5*0.2 + 0.5*0.2 = 0.3 + 0 + 0.1 + 0.1 = 0.5
        assert!((score.composite() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn latency_score_better_than_target() {
        let mut c = MetricsCollector::new();
        // Very low latency (better than 50ms target) -> clamped to 1.0
        c.record_latency(10.0);
        let score = c.activity_score();
        assert!((score.latency_score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn latency_score_worse_than_target() {
        let mut c = MetricsCollector::new();
        // 200ms latency vs 50ms target -> 50/200 = 0.25
        c.record_latency(200.0);
        let score = c.activity_score();
        assert!((score.latency_score - 0.25).abs() < 1e-9);
    }

    #[test]
    fn snapshot_captures_state() {
        let mut c = MetricsCollector::new();
        c.record_compute(500);
        c.record_bandwidth(1024);
        c.record_latency(25.0);
        c.record_latency(75.0);
        c.record_receipt();

        let snap = c.snapshot();
        assert_eq!(snap.compute_cycles, 500);
        assert_eq!(snap.bandwidth_bytes, 1024);
        assert!((snap.avg_latency_ms - 50.0).abs() < 1e-9);
        assert_eq!(snap.latency_sample_count, 2);
        assert_eq!(snap.receipt_count, 1);
    }

    #[test]
    fn reset_clears_all() {
        let mut c = MetricsCollector::new();
        c.record_compute(999);
        c.record_bandwidth(999);
        c.record_latency(999.0);
        c.record_receipt();
        c.reset();

        assert_eq!(c.compute_cycles, 0);
        assert_eq!(c.bandwidth_bytes, 0);
        assert!(c.latency_samples.is_empty());
        assert_eq!(c.receipt_count, 0);
    }

    #[test]
    fn normalize_clamps_above_baseline() {
        // Recording more than baseline should clamp to 1.0
        let mut c = MetricsCollector::new();
        c.record_compute(10_000_000); // 10x baseline
        let score = c.activity_score();
        assert!((score.compute_score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn snapshot_serde_roundtrip() {
        let mut c = MetricsCollector::new();
        c.record_compute(42);
        let snap = c.snapshot();
        let json = serde_json::to_string(&snap).expect("test: serialize snapshot");
        let back: MetricsSnapshot =
            serde_json::from_str(&json).expect("test: deserialize snapshot");
        assert_eq!(back.compute_cycles, 42);
    }
}
