//! STOQ Monitoring - Built-in monitoring without external dependencies
//!
//! This module provides comprehensive monitoring for STOQ transport protocol
//! that can be easily integrated with dashboard UIs like Nexus.

use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use stoq::transport::{StoqTransport, ProtocolMetrics, IntervalMetrics};

/// Comprehensive monitoring interface for STOQ
pub struct StoqMonitor {
    transport: Arc<StoqTransport>,
    collection_interval: Duration,
    last_collection: Instant,
    historical_metrics: Vec<MetricsSnapshot>,
    max_history: usize,
}

impl StoqMonitor {
    /// Create a new STOQ monitor
    pub fn new(transport: Arc<StoqTransport>) -> Self {
        Self::with_config(transport, Duration::from_secs(10), 100)
    }

    /// Create monitor with custom configuration
    pub fn with_config(
        transport: Arc<StoqTransport>,
        collection_interval: Duration,
        max_history: usize,
    ) -> Self {
        Self {
            transport,
            collection_interval,
            last_collection: Instant::now(),
            historical_metrics: Vec::with_capacity(max_history),
            max_history,
        }
    }

    /// Collect current metrics snapshot
    pub fn collect(&mut self) -> MetricsSnapshot {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_collection);

        // Get comprehensive metrics from transport
        let transport_stats = self.transport.stats();
        let protocol_metrics = self.transport.get_protocol_metrics();
        let interval_metrics = self.transport.get_interval_metrics();
        let (peak_gbps, zero_copy_ops, pool_hits, frame_batches) = self.transport.performance_stats();
        let pool_stats = self.transport.pool_stats();

        let snapshot = MetricsSnapshot {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            collection_interval_secs: elapsed.as_secs_f64(),

            // Transport metrics
            bytes_sent: transport_stats.bytes_sent,
            bytes_received: transport_stats.bytes_received,
            active_connections: transport_stats.active_connections,
            total_connections: transport_stats.total_connections,
            throughput_gbps: transport_stats.throughput_gbps,
            avg_latency_us: transport_stats.avg_latency_us,

            // Protocol metrics
            packets_tokenized: protocol_metrics.packets_tokenized,
            packets_sharded: protocol_metrics.packets_sharded,
            shards_reassembled: protocol_metrics.shards_reassembled,
            hop_routes_processed: protocol_metrics.hop_routes_processed,

            // Latency percentiles
            p50_latency_us: protocol_metrics.p50_latency_us,
            p95_latency_us: protocol_metrics.p95_latency_us,
            p99_latency_us: protocol_metrics.p99_latency_us,

            // Error metrics
            connection_failures: protocol_metrics.connection_failures,
            packet_drops: protocol_metrics.packet_drops,
            sharding_errors: protocol_metrics.sharding_errors,
            reassembly_errors: protocol_metrics.reassembly_errors,
            token_validation_failures: protocol_metrics.token_validation_failures,

            // Performance metrics
            peak_throughput_gbps: peak_gbps,
            zero_copy_operations: zero_copy_ops,
            memory_pool_hits: pool_hits,
            frame_batches_sent: frame_batches,

            // Rate metrics
            packets_per_sec: interval_metrics.packets_per_sec,
            connections_per_sec: interval_metrics.connections_per_sec,

            // Connection pool stats
            connection_pools: pool_stats.len(),
            total_pooled_connections: pool_stats.iter().map(|(_, count)| count).sum(),
        };

        // Store in history
        if self.historical_metrics.len() >= self.max_history {
            self.historical_metrics.remove(0);
        }
        self.historical_metrics.push(snapshot.clone());

        // Reset interval metrics for next collection
        if elapsed >= self.collection_interval {
            self.transport.reset_interval_metrics();
            self.last_collection = now;
        }

        snapshot
    }

    /// Get historical metrics
    pub fn history(&self) -> &[MetricsSnapshot] {
        &self.historical_metrics
    }

    /// Get metrics summary for dashboard display
    pub fn summary(&mut self) -> MetricsSummary {
        let current = self.collect();

        // Calculate trends from history
        let trend_window = 10; // Last 10 snapshots
        let trend_start = self.historical_metrics.len().saturating_sub(trend_window);
        let recent_history = &self.historical_metrics[trend_start..];

        let throughput_trend = if recent_history.len() > 1 {
            let first = recent_history.first().unwrap().throughput_gbps;
            let last = recent_history.last().unwrap().throughput_gbps;
            ((last - first) / first * 100.0).round() as i32
        } else {
            0
        };

        let error_rate = if current.total_connections > 0 {
            (current.connection_failures as f64 / current.total_connections as f64 * 100.0)
        } else {
            0.0
        };

        MetricsSummary {
            current_throughput_gbps: current.throughput_gbps,
            peak_throughput_gbps: current.peak_throughput_gbps,
            throughput_trend_percent: throughput_trend,
            active_connections: current.active_connections,
            total_connections: current.total_connections,
            avg_latency_ms: (current.avg_latency_us as f64) / 1000.0,
            p99_latency_ms: (current.p99_latency_us as f64) / 1000.0,
            packets_per_sec: current.packets_per_sec,
            error_rate_percent: error_rate,
            memory_efficiency_percent: if current.memory_pool_hits + current.packet_drops > 0 {
                (current.memory_pool_hits as f64 /
                 (current.memory_pool_hits + current.packet_drops) as f64 * 100.0)
            } else {
                100.0
            },
        }
    }

    /// Export metrics in JSON format for external systems
    pub fn export_json(&mut self) -> String {
        let snapshot = self.collect();
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get health status
    pub fn health_status(&mut self) -> HealthStatus {
        let metrics = self.collect();

        let status = if metrics.connection_failures > metrics.total_connections / 10 {
            HealthLevel::Critical
        } else if metrics.packet_drops > 0 || metrics.throughput_gbps < 1.0 {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };

        let mut issues = Vec::new();
        if metrics.connection_failures > 0 {
            issues.push(format!("{} connection failures", metrics.connection_failures));
        }
        if metrics.packet_drops > 0 {
            issues.push(format!("{} packet drops", metrics.packet_drops));
        }
        if metrics.throughput_gbps < 1.0 {
            issues.push(format!("Low throughput: {:.2} Gbps", metrics.throughput_gbps));
        }
        if metrics.p99_latency_us > 10000 {
            issues.push(format!("High P99 latency: {}ms", metrics.p99_latency_us / 1000));
        }

        HealthStatus {
            level: status,
            throughput_gbps: metrics.throughput_gbps,
            active_connections: metrics.active_connections,
            error_count: metrics.connection_failures + metrics.packet_drops +
                        metrics.sharding_errors + metrics.reassembly_errors,
            issues,
        }
    }
}

/// Complete metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub collection_interval_secs: f64,

    // Transport metrics
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub active_connections: usize,
    pub total_connections: u64,
    pub throughput_gbps: f64,
    pub avg_latency_us: u64,

    // Protocol metrics
    pub packets_tokenized: u64,
    pub packets_sharded: u64,
    pub shards_reassembled: u64,
    pub hop_routes_processed: u64,

    // Latency percentiles
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,

    // Error metrics
    pub connection_failures: u64,
    pub packet_drops: u64,
    pub sharding_errors: u64,
    pub reassembly_errors: u64,
    pub token_validation_failures: u64,

    // Performance metrics
    pub peak_throughput_gbps: f64,
    pub zero_copy_operations: u64,
    pub memory_pool_hits: u64,
    pub frame_batches_sent: u64,

    // Rate metrics
    pub packets_per_sec: f64,
    pub connections_per_sec: f64,

    // Pool stats
    pub connection_pools: usize,
    pub total_pooled_connections: usize,
}

/// Summarized metrics for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub current_throughput_gbps: f64,
    pub peak_throughput_gbps: f64,
    pub throughput_trend_percent: i32,
    pub active_connections: usize,
    pub total_connections: u64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub packets_per_sec: f64,
    pub error_rate_percent: f64,
    pub memory_efficiency_percent: f64,
}

/// Health status levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthLevel {
    Healthy,
    Warning,
    Critical,
}

/// Health status report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub level: HealthLevel,
    pub throughput_gbps: f64,
    pub active_connections: usize,
    pub error_count: u64,
    pub issues: Vec<String>,
}

/// Monitoring API for integration with external systems
pub trait MonitoringAPI: Send + Sync {
    /// Get current metrics snapshot
    fn get_metrics(&mut self) -> MetricsSnapshot;

    /// Get metrics summary for dashboard
    fn get_summary(&mut self) -> MetricsSummary;

    /// Get health status
    fn get_health(&mut self) -> HealthStatus;

    /// Get historical metrics
    fn get_history(&self) -> Vec<MetricsSnapshot>;

    /// Export metrics as JSON
    fn export_json(&mut self) -> String;
}

impl MonitoringAPI for StoqMonitor {
    fn get_metrics(&mut self) -> MetricsSnapshot {
        self.collect()
    }

    fn get_summary(&mut self) -> MetricsSummary {
        self.summary()
    }

    fn get_health(&mut self) -> HealthStatus {
        self.health_status()
    }

    fn get_history(&self) -> Vec<MetricsSnapshot> {
        self.historical_metrics.clone()
    }

    fn export_json(&mut self) -> String {
        self.export_json()
    }
}