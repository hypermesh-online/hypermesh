// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Performance Monitoring, Regression Detection, and Production Readiness
//!
//! Provides:
//! - **Monitoring Integration**: Metric collection, alerting, and export.
//! - **Regression Prevention**: Baseline comparison with configurable tolerance.
//! - **Production Readiness**: Validates that performance targets are met.

pub mod monitoring_integration;
pub mod regression_prevention;

pub use monitoring_integration::{
    AlertingManager, DashboardManager, MetricSample, MetricsExporter, MonitoringConfig,
    MonitoringIntegration,
};

pub use regression_prevention::{
    PerformanceBaseline, RegressionAlert, RegressionDetector, RegressionPrevention,
};

/// Performance optimization error types.
#[derive(Debug, thiserror::Error)]
pub enum PerformanceError {
    #[error("Optimization failed: {message}")]
    OptimizationFailed { message: String },

    #[error("Performance target not met: {target} = {actual}, expected {expected}")]
    TargetNotMet {
        target: String,
        actual: f64,
        expected: f64,
    },

    #[error("Monitoring overhead exceeded: {actual}% > {limit}%")]
    MonitoringOverheadExceeded { actual: f64, limit: f64 },

    #[error("Performance regression detected: {metric} degraded by {percentage}%")]
    RegressionDetected { metric: String, percentage: f64 },
}

/// Result type for performance operations.
pub type PerformanceResult<T> = Result<T, PerformanceError>;

/// Performance targets for the system.
#[derive(Debug, Clone)]
pub struct PerformanceTargets {
    pub latency_target_ms: f64,
    pub memory_target_mb: f64,
    pub throughput_target_ops: f64,
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            latency_target_ms: 50.0,
            memory_target_mb: 450.0,
            throughput_target_ops: 1000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_targets_defaults() {
        let targets = PerformanceTargets::default();
        assert!(targets.latency_target_ms < 100.0);
        assert!(targets.memory_target_mb < 500.0);
    }

    #[test]
    fn test_regression_detection_integration() {
        let mut baseline = PerformanceBaseline::new("v1");
        baseline.set("latency_ms", 50.0);
        baseline.set("memory_mb", 400.0);

        let prevention = RegressionPrevention::new(baseline, 10.0);

        let mut current = std::collections::HashMap::new();
        current.insert("latency_ms".to_string(), 52.0);
        current.insert("memory_mb".to_string(), 410.0);

        let alerts = prevention.check(&current);
        assert!(alerts.is_empty(), "Within tolerance -- no alerts");
    }
}
