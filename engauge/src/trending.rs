// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Multi-epoch capacity aggregation and trending.
//!
//! [`EpochTracker`] records per-node [`CapacityReport`] snapshots across epochs,
//! maintains a bounded ring buffer of history, and derives [`CapacityTrend`] and
//! [`AggregatedCapacity`] from that history.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::capacity::CapacityReport;

// ---------------------------------------------------------------------------
// EpochRecord
// ---------------------------------------------------------------------------

/// A single epoch's capacity data for a node.
#[derive(Debug, Clone)]
pub struct EpochRecord {
    /// Monotonically increasing epoch identifier (auto-assigned by tracker).
    pub epoch_id: u64,
    /// When this record was created.
    pub timestamp: Instant,
    /// The full capacity report for this epoch.
    pub report: CapacityReport,
}

// ---------------------------------------------------------------------------
// TrendDirection
// ---------------------------------------------------------------------------

/// Direction of capacity trend across observed epochs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    /// Latest score exceeds mean by more than 5%.
    Rising,
    /// Latest score is below mean by more than 5%.
    Falling,
    /// Latest score is within 5% of the mean.
    Stable,
}

// ---------------------------------------------------------------------------
// CapacityTrend
// ---------------------------------------------------------------------------

/// Aggregated trend analysis across multiple epochs for a single node.
#[derive(Debug, Clone)]
pub struct CapacityTrend {
    /// Overall direction of the trend.
    pub direction: TrendDirection,
    /// Arithmetic mean of scores across the lookback window.
    pub mean_score: f64,
    /// Rate of change per epoch (simple: `(last - first) / (n - 1)`).
    pub score_velocity: f64,
    /// Standard deviation of scores across the lookback window.
    pub volatility: f64,
    /// Number of epochs in the analysis window.
    pub epoch_count: usize,
    /// Most recent score value.
    pub latest_score: f64,
}

// ---------------------------------------------------------------------------
// AggregatedCapacity
// ---------------------------------------------------------------------------

/// Exponentially-decay-weighted moving average of [`CapacityScore`] component
/// weights across epochs (recent epochs contribute more).
#[derive(Debug, Clone)]
pub struct AggregatedCapacity {
    /// Weighted mean of the composite capacity score.
    pub weighted_score: f64,
    /// Number of epochs in the aggregation window.
    pub epoch_count: usize,
}

// ---------------------------------------------------------------------------
// EpochTracker
// ---------------------------------------------------------------------------

/// Multi-epoch capacity tracker.
///
/// Maintains a per-node ring buffer of [`EpochRecord`]s (bounded by
/// `max_epochs`) and provides trend and aggregation queries.
pub struct EpochTracker {
    /// Node ID -> epoch history (bounded ring buffer).
    histories: HashMap<String, VecDeque<EpochRecord>>,
    /// Maximum epochs retained per node.
    max_epochs: usize,
    /// Auto-incrementing epoch counter per node.
    next_epoch_ids: HashMap<String, u64>,
}

impl EpochTracker {
    /// Create a tracker that retains at most `max_epochs` per node.
    pub fn new(max_epochs: usize) -> Self {
        let max_epochs = if max_epochs == 0 { 1 } else { max_epochs };
        Self {
            histories: HashMap::new(),
            max_epochs,
            next_epoch_ids: HashMap::new(),
        }
    }

    /// Record a capacity report for a node, auto-assigning epoch ID.
    ///
    /// Oldest epochs are pruned when the buffer exceeds `max_epochs`.
    pub fn record_epoch(&mut self, node_id: &str, report: CapacityReport) {
        let epoch_id = self.next_epoch_ids.entry(node_id.to_string()).or_insert(0);
        let current_id = *epoch_id;
        *epoch_id += 1;

        let record = EpochRecord {
            epoch_id: current_id,
            timestamp: Instant::now(),
            report,
        };

        let history = self.histories.entry(node_id.to_string()).or_default();

        history.push_back(record);

        // Prune oldest if over capacity.
        while history.len() > self.max_epochs {
            history.pop_front();
        }
    }

    /// Compute a trend from the last `lookback` epochs for a node.
    ///
    /// Returns `None` if the node has no recorded epochs.
    pub fn get_trend(&self, node_id: &str, lookback: usize) -> Option<CapacityTrend> {
        let history = self.histories.get(node_id)?;
        if history.is_empty() {
            return None;
        }

        let scores = self.tail_scores(history, lookback);
        let n = scores.len();

        let mean_score = scores.iter().sum::<f64>() / n as f64;
        let latest_score = *scores.last().expect("scores is non-empty");

        let score_velocity = if n > 1 {
            (latest_score - scores[0]) / (n - 1) as f64
        } else {
            0.0
        };

        let volatility = std_deviation(&scores, mean_score);

        let direction = if latest_score > mean_score * 1.05 {
            TrendDirection::Rising
        } else if latest_score < mean_score * 0.95 {
            TrendDirection::Falling
        } else {
            TrendDirection::Stable
        };

        Some(CapacityTrend {
            direction,
            mean_score,
            score_velocity,
            volatility,
            epoch_count: n,
            latest_score,
        })
    }

    /// Compute an exponentially-decay-weighted aggregate of capacity scores
    /// over the last `lookback` epochs.
    ///
    /// Decay factor: weight_i = decay^(n - 1 - i), where `decay = 0.9` and
    /// `i = 0` is the oldest epoch in the window. More recent epochs have
    /// weight closer to 1.0.
    pub fn aggregate(&self, node_id: &str, lookback: usize) -> Option<AggregatedCapacity> {
        let history = self.histories.get(node_id)?;
        if history.is_empty() {
            return None;
        }

        let scores = self.tail_scores(history, lookback);
        let n = scores.len();

        const DECAY: f64 = 0.9;

        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;

        for (i, &score) in scores.iter().enumerate() {
            let weight = DECAY.powi((n - 1 - i) as i32);
            weighted_sum += score * weight;
            weight_total += weight;
        }

        let weighted_score = if weight_total > 0.0 {
            weighted_sum / weight_total
        } else {
            0.0
        };

        Some(AggregatedCapacity {
            weighted_score,
            epoch_count: n,
        })
    }

    /// Number of distinct nodes being tracked.
    pub fn node_count(&self) -> usize {
        self.histories.len()
    }

    /// Number of epochs recorded for a specific node (0 if unknown).
    pub fn epoch_count(&self, node_id: &str) -> usize {
        self.histories.get(node_id).map_or(0, |h| h.len())
    }

    // -- internal helpers ---------------------------------------------------

    /// Extract the last `lookback` score values from a history deque.
    fn tail_scores(&self, history: &VecDeque<EpochRecord>, lookback: usize) -> Vec<f64> {
        let n = history.len();
        let start = n.saturating_sub(lookback);
        history
            .iter()
            .skip(start)
            .map(|r| r.report.score.value())
            .collect()
    }
}

impl Default for EpochTracker {
    fn default() -> Self {
        Self::new(30)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Population standard deviation of a slice around a precomputed mean.
fn std_deviation(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capacity::CapacityMetrics;
    use hypermesh_lib::NodeId;

    /// Build a CapacityReport with a score derived from the given fraction
    /// of the baseline (0.0 = zero metrics, 1.0 = full baseline).
    fn report_with_fraction(fraction: f64) -> CapacityReport {
        let bytes = (1_073_741_824_f64 * fraction) as u64;
        let compute = (1_000_000_f64 * fraction) as u64;
        let storage = (10_737_418_240_f64 * fraction) as u64;
        let bandwidth = (1_000_000_000_f64 * fraction) as u64;
        let uptime = fraction.clamp(0.0, 1.0);

        let metrics = CapacityMetrics::new(bytes, compute, storage, bandwidth, uptime);
        CapacityReport::new(NodeId::from("trend-node"), metrics, 0)
    }

    #[test]
    fn test_empty_tracker() {
        let tracker = EpochTracker::new(10);
        assert_eq!(tracker.node_count(), 0);
        assert_eq!(tracker.epoch_count("nonexistent"), 0);
        assert!(tracker.get_trend("nonexistent", 5).is_none());
        assert!(tracker.aggregate("nonexistent", 5).is_none());
    }

    #[test]
    fn test_single_epoch() {
        let mut tracker = EpochTracker::new(10);
        tracker.record_epoch("node-a", report_with_fraction(0.5));

        assert_eq!(tracker.node_count(), 1);
        assert_eq!(tracker.epoch_count("node-a"), 1);

        let trend = tracker
            .get_trend("node-a", 10)
            .expect("test: trend should exist for node-a");
        assert_eq!(trend.epoch_count, 1);
        assert_eq!(trend.direction, TrendDirection::Stable);
        assert!((trend.score_velocity).abs() < 1e-9);
        assert!((trend.volatility).abs() < 1e-9);
    }

    #[test]
    fn test_rising_trend() {
        let mut tracker = EpochTracker::new(30);

        // Insert 10 epochs with increasing capacity fractions.
        for i in 0..10 {
            let fraction = 0.1 + (i as f64) * 0.08; // 0.1 -> 0.82
            tracker.record_epoch("riser", report_with_fraction(fraction));
        }

        let trend = tracker
            .get_trend("riser", 10)
            .expect("test: trend should exist for riser");
        assert_eq!(trend.epoch_count, 10);
        assert_eq!(
            trend.direction,
            TrendDirection::Rising,
            "expected Rising, got {:?} (latest={}, mean={})",
            trend.direction,
            trend.latest_score,
            trend.mean_score,
        );
        assert!(
            trend.score_velocity > 0.0,
            "velocity should be positive for rising trend"
        );
    }

    #[test]
    fn test_falling_trend() {
        let mut tracker = EpochTracker::new(30);

        // Insert 10 epochs with decreasing capacity fractions.
        for i in 0..10 {
            let fraction = 0.9 - (i as f64) * 0.08; // 0.9 -> 0.18
            tracker.record_epoch("faller", report_with_fraction(fraction));
        }

        let trend = tracker
            .get_trend("faller", 10)
            .expect("test: trend should exist for faller");
        assert_eq!(trend.epoch_count, 10);
        assert_eq!(
            trend.direction,
            TrendDirection::Falling,
            "expected Falling, got {:?} (latest={}, mean={})",
            trend.direction,
            trend.latest_score,
            trend.mean_score,
        );
        assert!(
            trend.score_velocity < 0.0,
            "velocity should be negative for falling trend"
        );
    }

    #[test]
    fn test_epoch_pruning() {
        let mut tracker = EpochTracker::new(5);

        // Insert 8 epochs; only 5 should remain.
        for i in 0..8 {
            let fraction = 0.5 + (i as f64) * 0.01;
            tracker.record_epoch("pruned", report_with_fraction(fraction));
        }

        assert_eq!(tracker.epoch_count("pruned"), 5);

        // The trend should only cover the retained 5 epochs, not all 8.
        let trend = tracker
            .get_trend("pruned", 100)
            .expect("test: trend should exist for pruned");
        assert_eq!(trend.epoch_count, 5);
    }

    #[test]
    fn test_aggregation_weights() {
        let mut tracker = EpochTracker::new(30);

        // Insert 5 low-capacity epochs then 1 high-capacity epoch.
        for _ in 0..5 {
            tracker.record_epoch("weighted", report_with_fraction(0.1));
        }
        tracker.record_epoch("weighted", report_with_fraction(0.9));

        let agg = tracker
            .aggregate("weighted", 6)
            .expect("test: aggregate should exist for weighted");
        assert_eq!(agg.epoch_count, 6);

        // Weighted average should be pulled toward the recent high epoch
        // (decay = 0.9 means the last epoch has weight 1.0, the first has
        // weight 0.9^5 = 0.59049).  Simple (unweighted) mean of 5*low + 1*high
        // would be dominated by lows.  Weighted mean should be higher.
        let low_score = report_with_fraction(0.1).score.value();
        let unweighted_mean = (low_score * 5.0 + report_with_fraction(0.9).score.value()) / 6.0;

        assert!(
            agg.weighted_score > unweighted_mean,
            "weighted_score ({}) should exceed unweighted mean ({}) \
             because recent high epoch has more weight",
            agg.weighted_score,
            unweighted_mean,
        );
    }

    #[test]
    fn test_multiple_nodes() {
        let mut tracker = EpochTracker::new(10);

        tracker.record_epoch("alpha", report_with_fraction(0.3));
        tracker.record_epoch("alpha", report_with_fraction(0.4));
        tracker.record_epoch("beta", report_with_fraction(0.8));

        assert_eq!(tracker.node_count(), 2);
        assert_eq!(tracker.epoch_count("alpha"), 2);
        assert_eq!(tracker.epoch_count("beta"), 1);
    }

    #[test]
    fn test_default_tracker() {
        let tracker = EpochTracker::default();
        assert_eq!(tracker.node_count(), 0);
        // Default max_epochs is 30 -- insert 35 and verify pruning.
        let mut tracker = EpochTracker::default();
        for i in 0..35 {
            tracker.record_epoch(
                "default-test",
                report_with_fraction(0.5 + (i as f64) * 0.001),
            );
        }
        assert_eq!(tracker.epoch_count("default-test"), 30);
    }

    #[test]
    fn test_lookback_smaller_than_history() {
        let mut tracker = EpochTracker::new(20);

        // Insert 10 low then 5 high.
        for _ in 0..10 {
            tracker.record_epoch("partial", report_with_fraction(0.1));
        }
        for _ in 0..5 {
            tracker.record_epoch("partial", report_with_fraction(0.9));
        }

        // Lookback of 5 should only see the 5 high epochs.
        let trend = tracker
            .get_trend("partial", 5)
            .expect("test: trend should exist for partial");
        assert_eq!(trend.epoch_count, 5);

        // All 5 are high, so direction should be Stable (all same value).
        assert_eq!(
            trend.direction,
            TrendDirection::Stable,
            "5 identical high epochs should be Stable, got {:?}",
            trend.direction,
        );
    }
}
