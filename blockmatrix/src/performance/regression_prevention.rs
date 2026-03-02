// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Performance Regression Detection and Prevention
//!
//! Compares current metric values against a stored baseline and flags
//! regressions that exceed a configurable tolerance.

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tracing::warn;

/// A recorded baseline of performance metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    /// Metric name -> expected value.
    pub metrics: HashMap<String, f64>,
    /// When the baseline was captured.
    pub captured_at: SystemTime,
    /// Descriptive label (e.g. "v0.9 release").
    pub label: String,
}

impl PerformanceBaseline {
    /// Create a new empty baseline.
    pub fn new(label: &str) -> Self {
        Self {
            metrics: HashMap::new(),
            captured_at: SystemTime::now(),
            label: label.to_string(),
        }
    }

    /// Set the expected value for a metric.
    pub fn set(&mut self, metric: &str, value: f64) {
        self.metrics.insert(metric.to_string(), value);
    }

    /// Get the expected value for a metric.
    pub fn get(&self, metric: &str) -> Option<f64> {
        self.metrics.get(metric).copied()
    }
}

/// A regression alert produced by the detector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAlert {
    /// Metric that regressed.
    pub metric: String,
    /// Baseline (expected) value.
    pub baseline: f64,
    /// Current (observed) value.
    pub current: f64,
    /// Regression magnitude (percentage worse).
    pub regression_pct: f64,
    /// Tolerance that was exceeded.
    pub tolerance_pct: f64,
}

/// Detects regressions by comparing current values against a baseline.
pub struct RegressionDetector {
    /// Tolerance per metric (percentage). Defaults to `default_tolerance`.
    tolerances: HashMap<String, f64>,
    /// Fallback tolerance when no per-metric override exists.
    default_tolerance: f64,
}

impl RegressionDetector {
    /// Create a detector with the given default tolerance (percentage).
    pub fn new(default_tolerance_pct: f64) -> Self {
        Self {
            tolerances: HashMap::new(),
            default_tolerance: default_tolerance_pct,
        }
    }

    /// Set a per-metric tolerance override.
    pub fn set_tolerance(&mut self, metric: &str, tolerance_pct: f64) {
        self.tolerances.insert(metric.to_string(), tolerance_pct);
    }

    /// Compare `current` values against `baseline`.
    ///
    /// A metric is flagged as regressed if its current value is
    /// **higher** than the baseline by more than the tolerance percentage.
    /// (For throughput metrics where higher is better, the caller should
    /// invert the sign before calling.)
    pub fn detect(
        &self,
        baseline: &PerformanceBaseline,
        current: &HashMap<String, f64>,
    ) -> Vec<RegressionAlert> {
        let mut alerts = Vec::new();

        for (metric, &baseline_val) in &baseline.metrics {
            let Some(&current_val) = current.get(metric) else {
                continue;
            };

            // Skip if baseline is zero to avoid divide-by-zero.
            if baseline_val.abs() < f64::EPSILON {
                continue;
            }

            let change_pct = ((current_val - baseline_val) / baseline_val) * 100.0;

            let tolerance = self
                .tolerances
                .get(metric)
                .copied()
                .unwrap_or(self.default_tolerance);

            if change_pct > tolerance {
                warn!(
                    metric,
                    baseline_val,
                    current_val,
                    change_pct,
                    tolerance,
                    "Performance regression detected"
                );
                alerts.push(RegressionAlert {
                    metric: metric.clone(),
                    baseline: baseline_val,
                    current: current_val,
                    regression_pct: change_pct,
                    tolerance_pct: tolerance,
                });
            }
        }

        alerts
    }
}

/// High-level regression prevention wrapper.
///
/// Holds a baseline and detector together for convenience.
pub struct RegressionPrevention {
    baseline: PerformanceBaseline,
    detector: RegressionDetector,
}

impl RegressionPrevention {
    /// Create with a baseline and default tolerance.
    pub fn new(baseline: PerformanceBaseline, default_tolerance_pct: f64) -> Self {
        Self {
            baseline,
            detector: RegressionDetector::new(default_tolerance_pct),
        }
    }

    /// Run a regression check with the current metrics.
    pub fn check(&self, current: &HashMap<String, f64>) -> Vec<RegressionAlert> {
        self.detector.detect(&self.baseline, current)
    }

    /// Replace the baseline.
    pub fn update_baseline(&mut self, baseline: PerformanceBaseline) {
        self.baseline = baseline;
    }

    /// Get the underlying detector for per-metric tolerance overrides.
    pub fn detector_mut(&mut self) -> &mut RegressionDetector {
        &mut self.detector
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_regression_within_tolerance() {
        let mut baseline = PerformanceBaseline::new("test");
        baseline.set("latency_ms", 50.0);
        baseline.set("memory_mb", 400.0);

        let detector = RegressionDetector::new(10.0); // 10% tolerance

        let mut current = HashMap::new();
        current.insert("latency_ms".to_string(), 52.0); // 4% increase
        current.insert("memory_mb".to_string(), 410.0); // 2.5% increase

        let alerts = detector.detect(&baseline, &current);
        assert!(alerts.is_empty(), "No alerts expected within tolerance");
    }

    #[test]
    fn test_regression_detected() {
        let mut baseline = PerformanceBaseline::new("test");
        baseline.set("latency_ms", 50.0);

        let detector = RegressionDetector::new(10.0);

        let mut current = HashMap::new();
        current.insert("latency_ms".to_string(), 60.0); // 20% increase

        let alerts = detector.detect(&baseline, &current);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].metric, "latency_ms");
        assert!((alerts[0].regression_pct - 20.0).abs() < 0.1);
    }

    #[test]
    fn test_prevention_check() {
        let mut baseline = PerformanceBaseline::new("v1.0");
        baseline.set("startup_ms", 80.0);
        baseline.set("throughput_gbps", 40.0);

        let prevention = RegressionPrevention::new(baseline, 5.0);

        let mut current = HashMap::new();
        current.insert("startup_ms".to_string(), 90.0); // 12.5% regression
        current.insert("throughput_gbps".to_string(), 41.0); // improved

        let alerts = prevention.check(&current);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].metric, "startup_ms");
    }

    #[test]
    fn test_per_metric_tolerance() {
        let mut baseline = PerformanceBaseline::new("test");
        baseline.set("latency_ms", 50.0);

        let mut detector = RegressionDetector::new(5.0); // strict default
        detector.set_tolerance("latency_ms", 25.0); // relaxed for latency

        let mut current = HashMap::new();
        current.insert("latency_ms".to_string(), 60.0); // 20% -- within 25%

        let alerts = detector.detect(&baseline, &current);
        assert!(alerts.is_empty());
    }
}
