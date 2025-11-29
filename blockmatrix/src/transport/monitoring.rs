//! Transport Monitoring and Metrics for HyperMesh

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use tracing::{debug, info};

/// Transport monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStats {
    /// Total authenticated nodes
    pub authenticated_nodes: usize,
    /// Total failed authentication attempts
    pub failed_authentications: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total connections established
    pub connections_established: u64,
    /// Total connections accepted
    pub connections_accepted: u64,
    /// Total connections closed
    pub connections_closed: u64,
    /// Average latency in microseconds
    pub avg_latency_us: u64,
    /// Maximum latency in microseconds
    pub max_latency_us: u64,
    /// Monitoring start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Monitoring uptime in seconds
    pub uptime_seconds: u64,
}

/// Transport metrics (alias for MonitoringStats for compatibility)
pub type TransportMetrics = MonitoringStats;

/// Transport monitor for collecting metrics
pub struct TransportMonitor {
    authenticated_nodes: AtomicUsize,
    failed_authentications: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    connections_established: AtomicU64,
    connections_accepted: AtomicU64,
    connections_closed: AtomicU64,
    latency_samples: Arc<RwLock<Vec<u64>>>,
    start_time: Instant,
    start_time_utc: chrono::DateTime<chrono::Utc>,
}

impl TransportMonitor {
    /// Create a new transport monitor
    pub fn new() -> Self {
        Self {
            authenticated_nodes: AtomicUsize::new(0),
            failed_authentications: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            connections_established: AtomicU64::new(0),
            connections_accepted: AtomicU64::new(0),
            connections_closed: AtomicU64::new(0),
            latency_samples: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            start_time: Instant::now(),
            start_time_utc: chrono::Utc::now(),
        }
    }

    /// Record an authenticated node
    pub fn record_authenticated_node(&self) {
        self.authenticated_nodes.fetch_add(1, Ordering::Relaxed);
        debug!("Authenticated node recorded");
    }

    /// Record a failed authentication
    pub fn record_failed_authentication(&self) {
        self.failed_authentications.fetch_add(1, Ordering::Relaxed);
        debug!("Failed authentication recorded");
    }

    /// Record bytes sent
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record bytes received
    pub fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a connection establishment
    pub fn record_connection_established(&self) {
        self.connections_established.fetch_add(1, Ordering::Relaxed);
        debug!("Connection establishment recorded");
    }

    /// Record a connection acceptance
    pub fn record_connection_accepted(&self) {
        self.connections_accepted.fetch_add(1, Ordering::Relaxed);
        debug!("Connection acceptance recorded");
    }

    /// Record a connection closure
    pub fn record_connection_closed(&self) {
        self.connections_closed.fetch_add(1, Ordering::Relaxed);
        debug!("Connection closure recorded");
    }

    /// Record a latency sample in microseconds
    pub fn record_latency(&self, latency_us: u64) {
        let mut samples = self.latency_samples.write();
        samples.push(latency_us);

        // Keep only last 1000 samples
        if samples.len() > 1000 {
            samples.remove(0);
        }
    }

    /// Get current monitoring statistics
    pub fn get_stats(&self) -> MonitoringStats {
        let samples = self.latency_samples.read();

        let (avg_latency, max_latency) = if !samples.is_empty() {
            let sum: u64 = samples.iter().sum();
            let avg = sum / samples.len() as u64;
            let max = *samples.iter().max().unwrap_or(&0);
            (avg, max)
        } else {
            (0, 0)
        };

        MonitoringStats {
            authenticated_nodes: self.authenticated_nodes.load(Ordering::Relaxed),
            failed_authentications: self.failed_authentications.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            connections_established: self.connections_established.load(Ordering::Relaxed),
            connections_accepted: self.connections_accepted.load(Ordering::Relaxed),
            connections_closed: self.connections_closed.load(Ordering::Relaxed),
            avg_latency_us: avg_latency,
            max_latency_us: max_latency,
            start_time: self.start_time_utc,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.authenticated_nodes.store(0, Ordering::Relaxed);
        self.failed_authentications.store(0, Ordering::Relaxed);
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.connections_established.store(0, Ordering::Relaxed);
        self.connections_accepted.store(0, Ordering::Relaxed);
        self.connections_closed.store(0, Ordering::Relaxed);
        self.latency_samples.write().clear();
        info!("Transport monitoring statistics reset");
    }

    /// Get throughput in Gbps
    pub fn get_throughput_gbps(&self) -> f64 {
        let uptime = self.start_time.elapsed().as_secs_f64();
        if uptime > 0.0 {
            let total_bytes = self.bytes_sent.load(Ordering::Relaxed) +
                             self.bytes_received.load(Ordering::Relaxed);
            (total_bytes as f64 * 8.0) / (uptime * 1_000_000_000.0)
        } else {
            0.0
        }
    }

    /// Shutdown monitoring (placeholder for future cleanup)
    pub async fn shutdown(&self) {
        info!("Shutting down transport monitoring");
        // Future: Could persist statistics to disk
    }
}

impl Default for TransportMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_counters() {
        let monitor = TransportMonitor::new();

        monitor.record_authenticated_node();
        monitor.record_connection_established();
        monitor.record_bytes_sent(1024);
        monitor.record_bytes_received(2048);

        let stats = monitor.get_stats();
        assert_eq!(stats.authenticated_nodes, 1);
        assert_eq!(stats.connections_established, 1);
        assert_eq!(stats.bytes_sent, 1024);
        assert_eq!(stats.bytes_received, 2048);
    }

    #[test]
    fn test_latency_tracking() {
        let monitor = TransportMonitor::new();

        monitor.record_latency(100);
        monitor.record_latency(200);
        monitor.record_latency(300);

        let stats = monitor.get_stats();
        assert_eq!(stats.avg_latency_us, 200);
        assert_eq!(stats.max_latency_us, 300);
    }

    #[test]
    fn test_reset() {
        let monitor = TransportMonitor::new();

        monitor.record_authenticated_node();
        monitor.record_connection_established();
        monitor.reset();

        let stats = monitor.get_stats();
        assert_eq!(stats.authenticated_nodes, 0);
        assert_eq!(stats.connections_established, 0);
    }
}