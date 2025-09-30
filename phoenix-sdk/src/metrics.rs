//! Phoenix Metrics Collection
//!
//! Real-time performance monitoring and metrics collection.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::net::SocketAddr;
use parking_lot::RwLock;
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use stoq::TransportStats;

use crate::{
    config::{PerformanceTier, SecurityLevel},
    connection::ConnectionState,
};

/// Phoenix metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoenixMetrics {
    /// Application name
    pub app_name: String,
    /// Uptime duration
    pub uptime: Duration,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Number of active connections
    pub active_connections: usize,
    /// Total connections established
    pub total_connections: u64,
    /// Current throughput in Gbps
    pub throughput_gbps: f64,
    /// Average latency in microseconds
    pub avg_latency_us: u64,
    /// Performance tier
    pub performance_tier: PerformanceTier,
    /// Security level
    pub security_level: SecurityLevel,
}

/// Live metrics for real-time monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMetrics {
    /// Current throughput (bytes/sec)
    pub throughput_bps: u64,
    /// Packets per second
    pub packets_per_second: u64,
    /// Connection rate (connections/sec)
    pub connection_rate: f64,
    /// Error rate (errors/sec)
    pub error_rate: f64,
    /// CPU usage percentage
    pub cpu_usage: f32,
    /// Memory usage in MB
    pub memory_usage_mb: u64,
    /// Network utilization percentage
    pub network_utilization: f32,
}

/// Connection-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    /// Connection ID
    pub connection_id: String,
    /// Remote address
    pub remote_addr: SocketAddr,
    /// Connection state
    pub state: ConnectionState,
    /// Bytes sent on this connection
    pub bytes_sent: u64,
    /// Bytes received on this connection
    pub bytes_received: u64,
    /// Round-trip time
    pub round_trip_time: Duration,
    /// Congestion window size
    pub congestion_window: u64,
}

/// Metrics collector for Phoenix SDK
pub struct MetricsCollector {
    app_name: String,
    start_time: Instant,
    state: Arc<RwLock<MetricsState>>,
    connection_metrics: Arc<DashMap<String, ConnectionMetricsData>>,
    samples: Arc<RwLock<MetricsSamples>>,
}

#[derive(Debug, Default)]
struct MetricsState {
    total_bytes_sent: u64,
    total_bytes_received: u64,
    total_connections: u64,
    accepted_connections: u64,
    failed_connections: u64,
    total_errors: u64,
    last_transport_stats: Option<TransportStats>,
}

#[derive(Debug)]
struct ConnectionMetricsData {
    bytes_sent: u64,
    bytes_received: u64,
    last_activity: Instant,
    rtt_samples: Vec<Duration>,
}

#[derive(Debug, Default)]
struct MetricsSamples {
    throughput_samples: Vec<(Instant, u64)>,
    latency_samples: Vec<(Instant, Duration)>,
    error_samples: Vec<(Instant, String)>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
            start_time: Instant::now(),
            state: Arc::new(RwLock::new(MetricsState::default())),
            connection_metrics: Arc::new(DashMap::new()),
            samples: Arc::new(RwLock::new(MetricsSamples::default())),
        }
    }

    /// Record data send operation
    pub fn record_send(&self, bytes: usize, duration: Duration) {
        let mut state = self.state.write();
        state.total_bytes_sent += bytes as u64;

        // Record throughput sample
        let mut samples = self.samples.write();
        samples.throughput_samples.push((Instant::now(), bytes as u64));
        samples.latency_samples.push((Instant::now(), duration));

        // Keep only recent samples (last 60 seconds)
        let cutoff = Instant::now() - Duration::from_secs(60);
        samples.throughput_samples.retain(|(t, _)| *t > cutoff);
        samples.latency_samples.retain(|(t, _)| *t > cutoff);
    }

    /// Record data receive operation
    pub fn record_receive(&self, bytes: usize, duration: Duration) {
        let mut state = self.state.write();
        state.total_bytes_received += bytes as u64;

        // Record samples
        let mut samples = self.samples.write();
        samples.throughput_samples.push((Instant::now(), bytes as u64));
        samples.latency_samples.push((Instant::now(), duration));

        // Keep only recent samples
        let cutoff = Instant::now() - Duration::from_secs(60);
        samples.throughput_samples.retain(|(t, _)| *t > cutoff);
        samples.latency_samples.retain(|(t, _)| *t > cutoff);
    }

    /// Update transport statistics
    pub fn update_transport_stats(&self, stats: TransportStats) {
        let mut state = self.state.write();
        state.last_transport_stats = Some(stats);
    }

    /// Increment accepted connections counter
    pub fn increment_accepted_connections(&self) {
        let mut state = self.state.write();
        state.accepted_connections += 1;
        state.total_connections += 1;
    }

    /// Record connection error
    pub fn record_error(&self, error: String) {
        let mut state = self.state.write();
        state.total_errors += 1;

        let mut samples = self.samples.write();
        samples.error_samples.push((Instant::now(), error));

        // Keep only recent errors
        let cutoff = Instant::now() - Duration::from_secs(60);
        samples.error_samples.retain(|(t, _)| *t > cutoff);
    }

    /// Get connection bytes sent
    pub fn get_connection_bytes_sent(&self, conn_id: &str) -> u64 {
        self.connection_metrics
            .get(conn_id)
            .map(|m| m.bytes_sent)
            .unwrap_or(0)
    }

    /// Get connection bytes received
    pub fn get_connection_bytes_received(&self, conn_id: &str) -> u64 {
        self.connection_metrics
            .get(conn_id)
            .map(|m| m.bytes_received)
            .unwrap_or(0)
    }

    /// Get current metrics snapshot
    pub fn get_snapshot(&self) -> MetricsSnapshot {
        let state = self.state.read();
        let samples = self.samples.read();

        // Calculate current throughput
        let now = Instant::now();
        let recent_window = Duration::from_secs(5);
        let recent_bytes: u64 = samples
            .throughput_samples
            .iter()
            .filter(|(t, _)| now.duration_since(*t) < recent_window)
            .map(|(_, b)| b)
            .sum();

        let throughput_bps = (recent_bytes as f64 / recent_window.as_secs_f64()) as u64;
        let throughput_gbps = throughput_bps as f64 / 1_000_000_000.0;

        // Calculate average latency
        let avg_latency_us = if !samples.latency_samples.is_empty() {
            let total: u64 = samples
                .latency_samples
                .iter()
                .map(|(_, d)| d.as_micros() as u64)
                .sum();
            total / samples.latency_samples.len() as u64
        } else {
            0
        };

        // Calculate error rate
        let error_rate = samples.error_samples.len() as f64 / 60.0;

        MetricsSnapshot {
            timestamp: Instant::now(),
            uptime: self.start_time.elapsed(),
            total_bytes_sent: state.total_bytes_sent,
            total_bytes_received: state.total_bytes_received,
            total_connections: state.total_connections,
            active_connections: self.connection_metrics.len(),
            throughput_gbps,
            avg_latency_us,
            error_rate,
        }
    }

    /// Get live metrics for dashboard
    pub fn get_live_metrics(&self) -> LiveMetrics {
        let snapshot = self.get_snapshot();
        let system_info = self.get_system_info();

        LiveMetrics {
            throughput_bps: (snapshot.throughput_gbps * 1_000_000_000.0) as u64,
            packets_per_second: 0, // Would need packet-level tracking
            connection_rate: snapshot.total_connections as f64 / snapshot.uptime.as_secs_f64(),
            error_rate: snapshot.error_rate,
            cpu_usage: system_info.cpu_usage,
            memory_usage_mb: system_info.memory_mb,
            network_utilization: (snapshot.throughput_gbps / 40.0 * 100.0) as f32, // Assuming 40 Gbps max
        }
    }

    fn get_system_info(&self) -> SystemInfo {
        use sysinfo::{System, SystemExt, ProcessExt};

        let mut system = System::new_all();
        system.refresh_all();

        let pid = sysinfo::get_current_pid().unwrap();
        let process = system.process(pid);

        SystemInfo {
            cpu_usage: process.map(|p| p.cpu_usage()).unwrap_or(0.0),
            memory_mb: process.map(|p| p.memory() / 1024).unwrap_or(0),
        }
    }
}

#[derive(Debug)]
struct SystemInfo {
    cpu_usage: f32,
    memory_mb: u64,
}

/// Metrics snapshot for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp of snapshot
    pub timestamp: Instant,
    /// Application uptime
    pub uptime: Duration,
    /// Total bytes sent
    pub total_bytes_sent: u64,
    /// Total bytes received
    pub total_bytes_received: u64,
    /// Total connections
    pub total_connections: u64,
    /// Active connections
    pub active_connections: usize,
    /// Throughput in Gbps
    pub throughput_gbps: f64,
    /// Average latency in microseconds
    pub avg_latency_us: u64,
    /// Error rate per second
    pub error_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new("test-app");

        // Record some operations
        collector.record_send(1024, Duration::from_millis(10));
        collector.record_receive(2048, Duration::from_millis(15));

        let snapshot = collector.get_snapshot();
        assert_eq!(snapshot.total_bytes_sent, 1024);
        assert_eq!(snapshot.total_bytes_received, 2048);
    }

    #[test]
    fn test_connection_metrics() {
        let metrics = ConnectionMetrics {
            connection_id: "test-123".to_string(),
            remote_addr: "127.0.0.1:8080".parse().unwrap(),
            state: ConnectionState::Connected,
            bytes_sent: 1024,
            bytes_received: 2048,
            round_trip_time: Duration::from_millis(10),
            congestion_window: 65536,
        };

        assert_eq!(metrics.connection_id, "test-123");
        assert_eq!(metrics.bytes_sent, 1024);
    }
}