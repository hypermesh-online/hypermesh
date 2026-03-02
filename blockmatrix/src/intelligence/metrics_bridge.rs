// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Metrics bridge for the intelligence layer.
//!
//! Connects the intelligence layer's [`PerformanceMonitor`] to real
//! hardware metrics from the `metrics` module (CPU/memory/network from
//! `/proc`), and provides a unified [`IntelligenceMetricsCollector`]
//! that feeds both runtime resource data and processing statistics into
//! the performance monitor.

use std::sync::Arc;
use std::time::Duration;

use tracing::debug;

use super::performance::PerformanceMonitor;
use crate::metrics::hardware;

/// Collects real hardware metrics and feeds them into the intelligence
/// layer's performance monitor.
pub struct IntelligenceMetricsCollector {
    monitor: Arc<PerformanceMonitor>,
}

impl IntelligenceMetricsCollector {
    /// Create a new collector backed by the given monitor.
    pub fn new(monitor: Arc<PerformanceMonitor>) -> Self {
        Self { monitor }
    }

    /// Sample current hardware metrics and record them.
    ///
    /// Reads CPU and memory from the real `/proc` filesystem (Linux only).
    /// On non-Linux platforms, silently records zero values.
    pub async fn sample_hardware(&self) {
        let cpu_pct = hardware::collect_cpu()
            .map(|c| c.total_usage * 100.0)
            .unwrap_or(0.0);

        let mem_mb = hardware::collect_memory()
            .map(|m| m.used_bytes / (1024 * 1024))
            .unwrap_or(0);

        self.monitor.record_resources(cpu_pct, mem_mb).await;
        debug!(cpu_pct, mem_mb, "Sampled hardware metrics for intelligence layer");
    }

    /// Record an asset processing operation.
    pub async fn record_processing(
        &self,
        component: &str,
        latency: Duration,
        bytes: u64,
        success: bool,
    ) {
        self.monitor
            .record_operation(component, latency, Some(bytes), success)
            .await;
    }

    /// Generate a performance report.
    pub async fn report(&self, period: Duration) -> super::performance::PerformanceReport {
        self.monitor.generate_report(period).await
    }

    /// Get current metrics snapshot.
    pub async fn current_metrics(&self) -> super::performance::PerformanceMetrics {
        self.monitor.get_metrics().await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_collector_sample_hardware() {
        let monitor = Arc::new(PerformanceMonitor::new(true));
        let collector = IntelligenceMetricsCollector::new(monitor);

        // Should not panic even if /proc is unavailable.
        collector.sample_hardware().await;

        let metrics = collector.current_metrics().await;
        // At minimum, the call shouldn't crash and total_operations should be 0.
        assert_eq!(metrics.total_operations, 0);
    }

    #[tokio::test]
    async fn test_collector_record_processing() {
        let monitor = Arc::new(PerformanceMonitor::new(true));
        let collector = IntelligenceMetricsCollector::new(monitor);

        collector
            .record_processing("pipeline", Duration::from_millis(42), 4096, true)
            .await;

        let metrics = collector.current_metrics().await;
        assert_eq!(metrics.total_operations, 1);
        assert_eq!(metrics.total_bytes_processed, 4096);
    }
}
