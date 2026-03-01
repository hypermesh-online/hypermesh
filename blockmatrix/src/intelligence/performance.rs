// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Performance Monitoring and Optimization
//!
//! This module provides comprehensive performance monitoring for the intelligence layer,
//! tracking latency, throughput, and resource utilization across all components.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::instrument;

/// Performance monitor for the intelligence layer
pub struct PerformanceMonitor {
    /// Whether monitoring is enabled
    enabled: bool,

    /// Metrics storage
    metrics: Arc<RwLock<PerformanceMetrics>>,

    /// Latency tracking
    latency_tracker: Arc<RwLock<LatencyTracker>>,

    /// Throughput tracking
    throughput_tracker: Arc<RwLock<ThroughputTracker>>,

    /// Resource utilization tracking
    resource_tracker: Arc<RwLock<ResourceTracker>>,
}

/// Performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total operations tracked
    pub total_operations: u64,

    /// Current operations per second
    pub ops_per_second: f64,

    /// Average latency (ms)
    pub avg_latency_ms: u64,

    /// P50 latency (ms)
    pub p50_latency_ms: u64,

    /// P95 latency (ms)
    pub p95_latency_ms: u64,

    /// P99 latency (ms)
    pub p99_latency_ms: u64,

    /// Maximum latency (ms)
    pub max_latency_ms: u64,

    /// Total bytes processed
    pub total_bytes_processed: u64,

    /// Current throughput (MB/s)
    pub throughput_mbps: f64,

    /// CPU utilization (0-100%)
    pub cpu_utilization: f64,

    /// Memory usage (MB)
    pub memory_usage_mb: u64,

    /// Active connections
    pub active_connections: usize,

    /// Component-specific metrics
    pub component_metrics: HashMap<String, ComponentMetrics>,
}

/// Component-specific metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentMetrics {
    /// Operations count
    pub operations: u64,

    /// Average latency (ms)
    pub avg_latency_ms: u64,

    /// Error count
    pub errors: u64,

    /// Success rate (0-1)
    pub success_rate: f64,

    /// Last operation timestamp
    pub last_operation: Option<SystemTime>,
}

/// Latency metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LatencyMetrics {
    /// Minimum latency (ms)
    pub min_ms: u64,

    /// Maximum latency (ms)
    pub max_ms: u64,

    /// Average latency (ms)
    pub avg_ms: u64,

    /// Standard deviation (ms)
    pub std_dev_ms: u64,

    /// Percentiles
    pub percentiles: HashMap<String, u64>,

    /// Histogram buckets
    pub histogram: Vec<(u64, u64)>, // (bucket_ms, count)
}

/// Throughput metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// Current throughput (ops/sec)
    pub current_ops_per_sec: f64,

    /// Peak throughput (ops/sec)
    pub peak_ops_per_sec: f64,

    /// Average throughput (ops/sec)
    pub avg_ops_per_sec: f64,

    /// Current bandwidth (MB/s)
    pub current_bandwidth_mbps: f64,

    /// Peak bandwidth (MB/s)
    pub peak_bandwidth_mbps: f64,

    /// Total operations
    pub total_operations: u64,

    /// Total bytes
    pub total_bytes: u64,
}

/// Performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Report ID
    pub id: String,

    /// Report timestamp
    pub timestamp: SystemTime,

    /// Reporting period
    pub period: Duration,

    /// Overall metrics
    pub metrics: PerformanceMetrics,

    /// Latency analysis
    pub latency: LatencyMetrics,

    /// Throughput analysis
    pub throughput: ThroughputMetrics,

    /// Resource utilization
    pub resources: ResourceUtilization,

    /// Performance score (0-100)
    pub performance_score: u8,

    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Resource utilization metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// CPU cores used
    pub cpu_cores: f64,

    /// CPU percentage (0-100)
    pub cpu_percentage: f64,

    /// Memory used (MB)
    pub memory_mb: u64,

    /// Memory percentage (0-100)
    pub memory_percentage: f64,

    /// Network bandwidth (MB/s)
    pub network_mbps: f64,

    /// Disk I/O (MB/s)
    pub disk_io_mbps: f64,

    /// File descriptors
    pub file_descriptors: usize,

    /// Thread count
    pub thread_count: usize,
}

/// Latency tracker
struct LatencyTracker {
    /// Recent latency samples (circular buffer)
    samples: VecDeque<u64>,

    /// Maximum samples to keep
    max_samples: usize,

    /// Histogram buckets
    histogram: HashMap<u64, u64>,

    /// Total sum for average calculation
    total_sum: u64,

    /// Sample count
    sample_count: u64,
}

impl LatencyTracker {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
            histogram: HashMap::new(),
            total_sum: 0,
            sample_count: 0,
        }
    }

    fn record(&mut self, latency_ms: u64) {
        // Add to samples
        if self.samples.len() >= self.max_samples {
            if let Some(old) = self.samples.pop_front() {
                self.total_sum = self.total_sum.saturating_sub(old);
            }
        }
        self.samples.push_back(latency_ms);
        self.total_sum += latency_ms;
        self.sample_count += 1;

        // Update histogram
        let bucket = (latency_ms / 10) * 10; // 10ms buckets
        *self.histogram.entry(bucket).or_insert(0) += 1;
    }

    fn calculate_percentile(&self, percentile: f64) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }

        let mut sorted: Vec<u64> = self.samples.iter().copied().collect();
        sorted.sort_unstable();

        let index = ((percentile / 100.0) * (sorted.len() as f64 - 1.0)) as usize;
        sorted[index]
    }

    fn get_metrics(&self) -> LatencyMetrics {
        if self.samples.is_empty() {
            return LatencyMetrics::default();
        }

        let avg = if self.sample_count > 0 {
            self.total_sum / self.sample_count
        } else {
            0
        };

        let min = *self.samples.iter().min().unwrap_or(&0);
        let max = *self.samples.iter().max().unwrap_or(&0);

        // Calculate standard deviation
        let variance: f64 = self
            .samples
            .iter()
            .map(|&x| {
                let diff = x as f64 - avg as f64;
                diff * diff
            })
            .sum::<f64>()
            / self.samples.len() as f64;

        let std_dev = variance.sqrt() as u64;

        let mut percentiles = HashMap::new();
        percentiles.insert("p50".to_string(), self.calculate_percentile(50.0));
        percentiles.insert("p75".to_string(), self.calculate_percentile(75.0));
        percentiles.insert("p90".to_string(), self.calculate_percentile(90.0));
        percentiles.insert("p95".to_string(), self.calculate_percentile(95.0));
        percentiles.insert("p99".to_string(), self.calculate_percentile(99.0));

        let histogram: Vec<(u64, u64)> = self.histogram.iter().map(|(&k, &v)| (k, v)).collect();

        LatencyMetrics {
            min_ms: min,
            max_ms: max,
            avg_ms: avg,
            std_dev_ms: std_dev,
            percentiles,
            histogram,
        }
    }
}

/// Throughput tracker
struct ThroughputTracker {
    /// Operation timestamps (circular buffer)
    operation_times: VecDeque<Instant>,

    /// Bytes processed with timestamps
    bytes_processed: VecDeque<(Instant, u64)>,

    /// Window size for rate calculation
    window: Duration,

    /// Total operations
    total_ops: u64,

    /// Total bytes
    total_bytes: u64,

    /// Peak ops/sec
    peak_ops_per_sec: f64,

    /// Peak bandwidth
    peak_bandwidth_mbps: f64,
}

impl ThroughputTracker {
    fn new(window: Duration) -> Self {
        Self {
            operation_times: VecDeque::with_capacity(10000),
            bytes_processed: VecDeque::with_capacity(10000),
            window,
            total_ops: 0,
            total_bytes: 0,
            peak_ops_per_sec: 0.0,
            peak_bandwidth_mbps: 0.0,
        }
    }

    fn record_operation(&mut self, bytes: Option<u64>) {
        let now = Instant::now();

        // Clean old entries
        let cutoff = now - self.window;
        while let Some(front) = self.operation_times.front() {
            if *front < cutoff {
                self.operation_times.pop_front();
            } else {
                break;
            }
        }

        while let Some((time, _)) = self.bytes_processed.front() {
            if *time < cutoff {
                self.bytes_processed.pop_front();
            } else {
                break;
            }
        }

        // Add new entries
        self.operation_times.push_back(now);
        self.total_ops += 1;

        if let Some(bytes) = bytes {
            self.bytes_processed.push_back((now, bytes));
            self.total_bytes += bytes;
        }

        // Update peaks
        let current_ops = self.calculate_ops_per_sec();
        if current_ops > self.peak_ops_per_sec {
            self.peak_ops_per_sec = current_ops;
        }

        let current_bandwidth = self.calculate_bandwidth_mbps();
        if current_bandwidth > self.peak_bandwidth_mbps {
            self.peak_bandwidth_mbps = current_bandwidth;
        }
    }

    fn calculate_ops_per_sec(&self) -> f64 {
        if self.operation_times.is_empty() {
            return 0.0;
        }

        let count = self.operation_times.len() as f64;
        count / self.window.as_secs_f64()
    }

    fn calculate_bandwidth_mbps(&self) -> f64 {
        if self.bytes_processed.is_empty() {
            return 0.0;
        }

        let total_bytes: u64 = self.bytes_processed.iter().map(|(_, bytes)| bytes).sum();

        (total_bytes as f64 / 1_048_576.0) / self.window.as_secs_f64()
    }

    fn get_metrics(&self) -> ThroughputMetrics {
        ThroughputMetrics {
            current_ops_per_sec: self.calculate_ops_per_sec(),
            peak_ops_per_sec: self.peak_ops_per_sec,
            avg_ops_per_sec: if self.total_ops > 0 {
                self.total_ops as f64 / self.window.as_secs_f64()
            } else {
                0.0
            },
            current_bandwidth_mbps: self.calculate_bandwidth_mbps(),
            peak_bandwidth_mbps: self.peak_bandwidth_mbps,
            total_operations: self.total_ops,
            total_bytes: self.total_bytes,
        }
    }
}

/// Resource tracker
struct ResourceTracker {
    /// CPU samples
    cpu_samples: VecDeque<f64>,

    /// Memory samples
    memory_samples: VecDeque<u64>,

    /// Maximum samples
    max_samples: usize,
}

impl ResourceTracker {
    fn new(max_samples: usize) -> Self {
        Self {
            cpu_samples: VecDeque::with_capacity(max_samples),
            memory_samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    fn record_cpu(&mut self, percentage: f64) {
        if self.cpu_samples.len() >= self.max_samples {
            self.cpu_samples.pop_front();
        }
        self.cpu_samples.push_back(percentage);
    }

    fn record_memory(&mut self, mb: u64) {
        if self.memory_samples.len() >= self.max_samples {
            self.memory_samples.pop_front();
        }
        self.memory_samples.push_back(mb);
    }

    fn get_utilization(&self) -> ResourceUtilization {
        let cpu_percentage = if !self.cpu_samples.is_empty() {
            self.cpu_samples.iter().sum::<f64>() / self.cpu_samples.len() as f64
        } else {
            0.0
        };

        let memory_mb = if !self.memory_samples.is_empty() {
            self.memory_samples.iter().sum::<u64>() / self.memory_samples.len() as u64
        } else {
            0
        };

        ResourceUtilization {
            cpu_cores: cpu_percentage / 100.0 * num_cpus::get() as f64,
            cpu_percentage,
            memory_mb,
            memory_percentage: 0.0, // Would need system memory info
            network_mbps: 0.0,      // Placeholder
            disk_io_mbps: 0.0,      // Placeholder
            file_descriptors: 0,    // Placeholder
            thread_count: 0,        // Placeholder
        }
    }
}

impl PerformanceMonitor {
    /// Create new performance monitor
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            latency_tracker: Arc::new(RwLock::new(LatencyTracker::new(10000))),
            throughput_tracker: Arc::new(RwLock::new(ThroughputTracker::new(Duration::from_secs(
                60,
            )))),
            resource_tracker: Arc::new(RwLock::new(ResourceTracker::new(1000))),
        }
    }

    /// Record an operation
    #[instrument(skip(self))]
    pub async fn record_operation(
        &self,
        component: &str,
        latency: Duration,
        bytes: Option<u64>,
        success: bool,
    ) {
        if !self.enabled {
            return;
        }

        let latency_ms = latency.as_millis() as u64;

        // Update latency tracker
        self.latency_tracker.write().await.record(latency_ms);

        // Update throughput tracker
        self.throughput_tracker
            .write()
            .await
            .record_operation(bytes);

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.total_operations += 1;

        if let Some(bytes) = bytes {
            metrics.total_bytes_processed += bytes;
        }

        // Update component metrics
        let component_metrics = metrics
            .component_metrics
            .entry(component.to_string())
            .or_insert_with(ComponentMetrics::default);

        component_metrics.operations += 1;
        component_metrics.last_operation = Some(SystemTime::now());

        if !success {
            component_metrics.errors += 1;
        }

        component_metrics.success_rate = (component_metrics.operations - component_metrics.errors)
            as f64
            / component_metrics.operations as f64;

        // Update average latency for component
        if component_metrics.operations == 1 {
            component_metrics.avg_latency_ms = latency_ms;
        } else {
            component_metrics.avg_latency_ms = (component_metrics.avg_latency_ms
                * (component_metrics.operations - 1)
                + latency_ms)
                / component_metrics.operations;
        }
    }

    /// Record resource utilization
    pub async fn record_resources(&self, cpu_percentage: f64, memory_mb: u64) {
        if !self.enabled {
            return;
        }

        let mut tracker = self.resource_tracker.write().await;
        tracker.record_cpu(cpu_percentage);
        tracker.record_memory(memory_mb);

        let mut metrics = self.metrics.write().await;
        metrics.cpu_utilization = cpu_percentage;
        metrics.memory_usage_mb = memory_mb;
    }

    /// Generate performance report
    pub async fn generate_report(&self, period: Duration) -> PerformanceReport {
        let metrics = self.metrics.read().await.clone();
        let latency = self.latency_tracker.read().await.get_metrics();
        let throughput = self.throughput_tracker.read().await.get_metrics();
        let resources = self.resource_tracker.read().await.get_utilization();

        // Calculate performance score (0-100)
        let mut score = 100u8;

        // Deduct for high latency
        if latency.percentiles.get("p99").copied().unwrap_or(0) > 1000 {
            score = score.saturating_sub(20);
        } else if latency.percentiles.get("p95").copied().unwrap_or(0) > 500 {
            score = score.saturating_sub(10);
        }

        // Deduct for low throughput
        if throughput.current_ops_per_sec < 10.0 {
            score = score.saturating_sub(15);
        }

        // Deduct for high resource usage
        if resources.cpu_percentage > 80.0 {
            score = score.saturating_sub(10);
        }
        if resources.memory_mb > 4096 {
            score = score.saturating_sub(10);
        }

        // Generate recommendations
        let mut recommendations = Vec::new();

        if latency.percentiles.get("p99").copied().unwrap_or(0) > 1000 {
            recommendations.push(
                "High P99 latency detected - consider optimizing slow operations".to_string(),
            );
        }

        if throughput.current_ops_per_sec < throughput.avg_ops_per_sec * 0.5 {
            recommendations
                .push("Throughput degradation detected - investigate bottlenecks".to_string());
        }

        if resources.cpu_percentage > 80.0 {
            recommendations.push("High CPU usage - consider scaling horizontally".to_string());
        }

        PerformanceReport {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            period,
            metrics,
            latency,
            throughput,
            resources,
            performance_score: score,
            recommendations,
        }
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        *self.metrics.write().await = PerformanceMetrics::default();
        *self.latency_tracker.write().await = LatencyTracker::new(10000);
        *self.throughput_tracker.write().await = ThroughputTracker::new(Duration::from_secs(60));
        *self.resource_tracker.write().await = ResourceTracker::new(1000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new(true);

        // Record some operations
        for i in 0..10 {
            monitor
                .record_operation(
                    "test_component",
                    Duration::from_millis(50 + i * 10),
                    Some(1024 * (i + 1)),
                    true,
                )
                .await;
        }

        // Record resources
        monitor.record_resources(45.0, 2048).await;

        // Get metrics
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.total_operations, 10);
        assert_eq!(metrics.component_metrics["test_component"].operations, 10);

        // Generate report
        let report = monitor.generate_report(Duration::from_secs(60)).await;
        assert!(report.performance_score > 0);
        assert_eq!(report.metrics.total_operations, 10);
    }

    #[tokio::test]
    async fn test_latency_tracker() {
        let mut tracker = LatencyTracker::new(100);

        for i in 1..=100 {
            tracker.record(i as u64);
        }

        let metrics = tracker.get_metrics();
        assert_eq!(metrics.min_ms, 1);
        assert_eq!(metrics.max_ms, 100);
        assert_eq!(metrics.avg_ms, 50);
        assert!(metrics.percentiles.contains_key("p50"));
        assert!(metrics.percentiles.contains_key("p99"));
    }

    #[tokio::test]
    async fn test_throughput_tracker() {
        let mut tracker = ThroughputTracker::new(Duration::from_secs(1));

        for _ in 0..10 {
            tracker.record_operation(Some(1024));
        }

        let metrics = tracker.get_metrics();
        assert_eq!(metrics.total_operations, 10);
        assert_eq!(metrics.total_bytes, 10240);
        assert!(metrics.current_ops_per_sec >= 0.0);
    }
}
