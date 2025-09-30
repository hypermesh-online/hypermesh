//! Metrics collection and management

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::debug;

/// Metrics collection system
pub struct Metrics {
    /// Component metrics
    components: Arc<RwLock<HashMap<String, ComponentMetrics>>>,
    /// Global counters
    counters: Arc<RwLock<HashMap<String, u64>>>,
    /// Timing metrics
    timings: Arc<RwLock<HashMap<String, Vec<u64>>>>,
}

/// Metrics snapshot
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp of snapshot
    pub timestamp: SystemTime,
    /// Component-specific metrics
    pub components: HashMap<String, ComponentMetrics>,
    /// Global counters
    pub counters: HashMap<String, u64>,
    /// Timing statistics
    pub timing_stats: HashMap<String, TimingStats>,
}

/// Component-specific metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentMetrics {
    /// Component name
    pub name: String,
    /// Total operations
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Success rate
    pub success_rate: f64,
    /// Average operation time (ms)
    pub avg_operation_time_ms: f64,
    /// Last operation time
    pub last_operation: Option<SystemTime>,
    /// Additional metrics specific to component
    pub additional_metrics: HashMap<String, f64>,
}

/// Timing statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimingStats {
    /// Minimum time (ms)
    pub min_ms: u64,
    /// Maximum time (ms)
    pub max_ms: u64,
    /// Average time (ms)
    pub avg_ms: f64,
    /// Median time (ms)
    pub median_ms: u64,
    /// 95th percentile (ms)
    pub p95_ms: u64,
    /// 99th percentile (ms)
    pub p99_ms: u64,
    /// Sample count
    pub count: usize,
}

impl Metrics {
    /// Create new metrics collection
    pub fn new() -> Self {
        Self {
            components: Arc::new(RwLock::new(HashMap::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            timings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record certificate issuance
    pub async fn record_cert_issuance(&self, duration_ms: u64, success: bool) {
        self.record_component_operation("ca", duration_ms, success).await;

        // Record specific CA metrics
        let mut components = self.components.write().await;
        let ca_metrics = components.entry("ca".to_string())
            .or_insert_with(|| ComponentMetrics::new("ca"));

        ca_metrics.additional_metrics
            .entry("avg_issuance_time_ms".to_string())
            .and_modify(|avg| {
                *avg = (*avg + duration_ms as f64) / 2.0;
            })
            .or_insert(duration_ms as f64);
    }

    /// Record DNS resolution
    pub async fn record_dns_resolution(&self, duration_ms: u64, success: bool) {
        self.record_component_operation("dns", duration_ms, success).await;
    }

    /// Record CT log entry
    pub async fn record_ct_log_entry(&self, duration_ms: u64, success: bool) {
        self.record_component_operation("ct", duration_ms, success).await;
    }

    /// Record consensus validation
    pub async fn record_consensus_validation(&self, duration_ms: u64, success: bool) {
        self.record_component_operation("consensus", duration_ms, success).await;
    }

    /// Record component operation
    async fn record_component_operation(&self, component: &str, duration_ms: u64, success: bool) {
        let mut components = self.components.write().await;
        let metrics = components.entry(component.to_string())
            .or_insert_with(|| ComponentMetrics::new(component));

        metrics.total_operations += 1;
        if success {
            metrics.successful_operations += 1;
        } else {
            metrics.failed_operations += 1;
        }

        // Update success rate
        metrics.success_rate = metrics.successful_operations as f64 / metrics.total_operations as f64;

        // Update average time
        let current_avg = metrics.avg_operation_time_ms;
        let new_avg = (current_avg * (metrics.total_operations - 1) as f64 + duration_ms as f64)
            / metrics.total_operations as f64;
        metrics.avg_operation_time_ms = new_avg;

        metrics.last_operation = Some(SystemTime::now());

        // Record timing
        let mut timings = self.timings.write().await;
        timings.entry(component.to_string())
            .or_insert_with(Vec::new)
            .push(duration_ms);

        debug!("Recorded {} operation: {}ms, success: {}", component, duration_ms, success);
    }

    /// Increment counter
    pub async fn increment_counter(&self, name: &str) {
        let mut counters = self.counters.write().await;
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }

    /// Get counter value
    pub async fn get_counter(&self, name: &str) -> u64 {
        let counters = self.counters.read().await;
        counters.get(name).copied().unwrap_or(0)
    }

    /// Collect metrics (periodic collection)
    pub async fn collect(&self) {
        debug!("Collecting metrics");
        // Cleanup old timing data (keep last 1000 samples per metric)
        let mut timings = self.timings.write().await;
        for samples in timings.values_mut() {
            if samples.len() > 1000 {
                let start = samples.len() - 1000;
                *samples = samples[start..].to_vec();
            }
        }
    }

    /// Create metrics snapshot
    pub async fn snapshot(&self) -> MetricsSnapshot {
        let components = self.components.read().await.clone();
        let counters = self.counters.read().await.clone();

        // Calculate timing statistics
        let timings = self.timings.read().await;
        let mut timing_stats = HashMap::new();

        for (name, samples) in timings.iter() {
            if !samples.is_empty() {
                let mut sorted = samples.clone();
                sorted.sort_unstable();

                let min = *sorted.first().unwrap();
                let max = *sorted.last().unwrap();
                let avg = samples.iter().sum::<u64>() as f64 / samples.len() as f64;
                let median = sorted[samples.len() / 2];
                let p95_idx = ((samples.len() as f64 * 0.95) as usize).min(samples.len() - 1);
                let p99_idx = ((samples.len() as f64 * 0.99) as usize).min(samples.len() - 1);
                let p95 = sorted[p95_idx];
                let p99 = sorted[p99_idx];

                timing_stats.insert(name.clone(), TimingStats {
                    min_ms: min,
                    max_ms: max,
                    avg_ms: avg,
                    median_ms: median,
                    p95_ms: p95,
                    p99_ms: p99,
                    count: samples.len(),
                });
            }
        }

        MetricsSnapshot {
            timestamp: SystemTime::now(),
            components,
            counters,
            timing_stats,
        }
    }
}

impl ComponentMetrics {
    /// Create new component metrics
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            success_rate: 0.0,
            avg_operation_time_ms: 0.0,
            last_operation: None,
            additional_metrics: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_recording() {
        let metrics = Metrics::new();

        // Record some operations
        metrics.record_cert_issuance(35, true).await;
        metrics.record_cert_issuance(40, true).await;
        metrics.record_cert_issuance(30, false).await;

        let snapshot = metrics.snapshot().await;
        let ca_metrics = snapshot.components.get("ca").unwrap();

        assert_eq!(ca_metrics.total_operations, 3);
        assert_eq!(ca_metrics.successful_operations, 2);
        assert_eq!(ca_metrics.failed_operations, 1);
        assert!((ca_metrics.success_rate - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_timing_statistics() {
        let metrics = Metrics::new();

        // Record multiple timings
        for i in 1..=100 {
            metrics.record_dns_resolution(i, true).await;
        }

        let snapshot = metrics.snapshot().await;
        let timing = snapshot.timing_stats.get("dns").unwrap();

        assert_eq!(timing.min_ms, 1);
        assert_eq!(timing.max_ms, 100);
        assert_eq!(timing.median_ms, 50);
        assert_eq!(timing.count, 100);
    }

    #[tokio::test]
    async fn test_counters() {
        let metrics = Metrics::new();

        metrics.increment_counter("test_counter").await;
        metrics.increment_counter("test_counter").await;

        assert_eq!(metrics.get_counter("test_counter").await, 2);
        assert_eq!(metrics.get_counter("non_existent").await, 0);
    }
}