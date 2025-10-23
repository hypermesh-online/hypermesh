//! Real-time Performance Monitoring System
//!
//! This module provides continuous, real-time performance monitoring for STOQ transport,
//! replacing all hardcoded performance claims with actual measured values.

use std::sync::Arc;
use std::time::{Instant, Duration};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::collections::VecDeque;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use tokio::time::interval;
use tracing::{info, warn, error, debug};

/// Performance monitoring system with real measurements
pub struct PerformanceMonitor {
    // Real measured metrics
    throughput_samples: Arc<RwLock<ThroughputTracker>>,
    latency_samples: Arc<RwLock<LatencyTracker>>,
    connection_metrics: Arc<RwLock<ConnectionMetrics>>,
    packet_metrics: Arc<RwLock<PacketMetrics>>,

    // Performance thresholds for alerting
    throughput_threshold: f64, // Gbps
    latency_threshold: f64,    // ms

    // Monitoring state
    monitoring_active: Arc<AtomicBool>,
    start_time: Instant,
}

/// Tracks real throughput measurements
#[derive(Debug)]
struct ThroughputTracker {
    samples: VecDeque<ThroughputSample>,
    max_samples: usize,
    current_bytes: AtomicU64,
    last_reset: Instant,
}

#[derive(Debug, Clone)]
struct ThroughputSample {
    timestamp: Instant,
    bytes_per_sec: f64,
    gbps: f64,
}

impl ThroughputTracker {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
            current_bytes: AtomicU64::new(0),
            last_reset: Instant::now(),
        }
    }

    fn record_bytes(&self, bytes: usize) {
        self.current_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    fn calculate_throughput(&mut self) -> f64 {
        let now = Instant::now();
        let duration = now.duration_since(self.last_reset);

        if duration.as_secs_f64() < 0.1 {
            return 0.0; // Not enough time has passed
        }

        let bytes = self.current_bytes.swap(0, Ordering::Relaxed);
        let bytes_per_sec = bytes as f64 / duration.as_secs_f64();
        let gbps = (bytes_per_sec * 8.0) / 1_000_000_000.0;

        let sample = ThroughputSample {
            timestamp: now,
            bytes_per_sec,
            gbps,
        };

        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);

        self.last_reset = now;
        gbps
    }

    fn get_statistics(&self) -> ThroughputStats {
        if self.samples.is_empty() {
            return ThroughputStats::default();
        }

        let gbps_values: Vec<f64> = self.samples.iter().map(|s| s.gbps).collect();
        let sum: f64 = gbps_values.iter().sum();
        let count = gbps_values.len() as f64;

        let mut sorted = gbps_values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        ThroughputStats {
            current_gbps: self.samples.back().map(|s| s.gbps).unwrap_or(0.0),
            average_gbps: sum / count,
            peak_gbps: sorted.last().copied().unwrap_or(0.0),
            p50_gbps: percentile(&sorted, 50.0),
            p95_gbps: percentile(&sorted, 95.0),
            p99_gbps: percentile(&sorted, 99.0),
            sample_count: self.samples.len(),
        }
    }
}

/// Tracks real latency measurements
#[derive(Debug)]
struct LatencyTracker {
    samples: VecDeque<LatencySample>,
    max_samples: usize,
}

#[derive(Debug, Clone)]
struct LatencySample {
    timestamp: Instant,
    latency_ms: f64,
}

impl LatencyTracker {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    fn record_latency(&mut self, duration: Duration) {
        let latency_ms = duration.as_secs_f64() * 1000.0;
        let sample = LatencySample {
            timestamp: Instant::now(),
            latency_ms,
        };

        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    fn get_statistics(&self) -> LatencyStats {
        if self.samples.is_empty() {
            return LatencyStats::default();
        }

        let latencies: Vec<f64> = self.samples.iter().map(|s| s.latency_ms).collect();
        let sum: f64 = latencies.iter().sum();
        let count = latencies.len() as f64;

        let mut sorted = latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        LatencyStats {
            current_ms: self.samples.back().map(|s| s.latency_ms).unwrap_or(0.0),
            average_ms: sum / count,
            min_ms: sorted.first().copied().unwrap_or(0.0),
            max_ms: sorted.last().copied().unwrap_or(0.0),
            p50_ms: percentile(&sorted, 50.0),
            p95_ms: percentile(&sorted, 95.0),
            p99_ms: percentile(&sorted, 99.0),
            sample_count: self.samples.len(),
        }
    }
}

/// Connection performance metrics
#[derive(Debug, Clone, Default)]
struct ConnectionMetrics {
    active_connections: u64,
    total_connections: u64,
    failed_connections: u64,
    connection_rate: f64,
    average_connect_time_ms: f64,
}

/// Packet processing metrics
#[derive(Debug, Clone, Default)]
struct PacketMetrics {
    packets_sent: u64,
    packets_received: u64,
    packets_dropped: u64,
    packet_rate: f64,
    average_packet_size: f64,
}

/// Real throughput statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThroughputStats {
    pub current_gbps: f64,
    pub average_gbps: f64,
    pub peak_gbps: f64,
    pub p50_gbps: f64,
    pub p95_gbps: f64,
    pub p99_gbps: f64,
    pub sample_count: usize,
}

/// Real latency statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LatencyStats {
    pub current_ms: f64,
    pub average_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub sample_count: usize,
}

/// Complete performance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub timestamp: String,
    pub uptime_seconds: f64,
    pub throughput: ThroughputStats,
    pub latency: LatencyStats,
    pub connections: ConnectionSnapshot,
    pub packets: PacketSnapshot,
    pub performance_tier: NetworkTier,
    pub health_status: HealthStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSnapshot {
    pub active: u64,
    pub total: u64,
    pub failed: u64,
    pub success_rate: f64,
    pub connections_per_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketSnapshot {
    pub sent: u64,
    pub received: u64,
    pub dropped: u64,
    pub loss_rate: f64,
    pub packets_per_sec: f64,
}

/// Network tier based on actual measured performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkTier {
    Slow { mbps: f64 },           // < 100 Mbps
    Home { mbps: f64 },           // 100 Mbps - 1 Gbps
    Standard { gbps: f64 },       // 1 - 2.5 Gbps
    Performance { gbps: f64 },    // 2.5 - 10 Gbps
    Enterprise { gbps: f64 },     // 10 - 25 Gbps
    DataCenter { gbps: f64 },     // 25+ Gbps
}

impl NetworkTier {
    pub fn from_gbps(gbps: f64) -> Self {
        let mbps = gbps * 1000.0;
        match gbps {
            g if g >= 25.0 => NetworkTier::DataCenter { gbps: g },
            g if g >= 10.0 => NetworkTier::Enterprise { gbps: g },
            g if g >= 2.5 => NetworkTier::Performance { gbps: g },
            g if g >= 1.0 => NetworkTier::Standard { gbps: g },
            g if mbps >= 100.0 => NetworkTier::Home { mbps },
            _ => NetworkTier::Slow { mbps },
        }
    }
}

/// Health status based on performance thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy { message: String },
    Warning { message: String },
    Critical { message: String },
}

impl PerformanceMonitor {
    pub fn new(throughput_threshold: f64, latency_threshold: f64) -> Self {
        Self {
            throughput_samples: Arc::new(RwLock::new(ThroughputTracker::new(1000))),
            latency_samples: Arc::new(RwLock::new(LatencyTracker::new(1000))),
            connection_metrics: Arc::new(RwLock::new(ConnectionMetrics::default())),
            packet_metrics: Arc::new(RwLock::new(PacketMetrics::default())),
            throughput_threshold,
            latency_threshold,
            monitoring_active: Arc::new(AtomicBool::new(false)),
            start_time: Instant::now(),
        }
    }

    /// Start continuous monitoring
    pub async fn start_monitoring(&self) {
        if self.monitoring_active.swap(true, Ordering::SeqCst) {
            warn!("Performance monitoring already active");
            return;
        }

        info!("Starting real-time performance monitoring");

        let monitor = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));

            while monitor.monitoring_active.load(Ordering::Relaxed) {
                interval.tick().await;

                // Calculate current throughput
                let gbps = monitor.throughput_samples.write().calculate_throughput();

                // Get performance snapshot
                let snapshot = monitor.get_snapshot();

                // Log real performance (not fantasy metrics)
                debug!(
                    "Performance: {:.3} Gbps (avg: {:.3}), Latency: {:.2} ms (p95: {:.2}), Connections: {}",
                    snapshot.throughput.current_gbps,
                    snapshot.throughput.average_gbps,
                    snapshot.latency.current_ms,
                    snapshot.latency.p95_ms,
                    snapshot.connections.active
                );

                // Check health status
                match &snapshot.health_status {
                    HealthStatus::Critical { message } => {
                        error!("CRITICAL: {}", message);
                    }
                    HealthStatus::Warning { message } => {
                        warn!("WARNING: {}", message);
                    }
                    HealthStatus::Healthy { .. } => {}
                }

                // Detect performance regression
                if gbps > 0.0 && gbps < monitor.throughput_threshold * 0.8 {
                    warn!(
                        "Performance degradation detected: {:.3} Gbps (threshold: {:.3} Gbps)",
                        gbps, monitor.throughput_threshold
                    );
                }
            }
        });
    }

    /// Stop monitoring
    pub fn stop_monitoring(&self) {
        self.monitoring_active.store(false, Ordering::SeqCst);
        info!("Performance monitoring stopped");
    }

    /// Record bytes transferred
    pub fn record_bytes(&self, bytes: usize) {
        self.throughput_samples.read().record_bytes(bytes);
    }

    /// Record latency measurement
    pub fn record_latency(&self, duration: Duration) {
        self.latency_samples.write().record_latency(duration);
    }

    /// Record connection event
    pub fn record_connection(&self, success: bool, connect_time: Option<Duration>) {
        let mut metrics = self.connection_metrics.write();
        if success {
            metrics.active_connections += 1;
            metrics.total_connections += 1;
            if let Some(time) = connect_time {
                metrics.average_connect_time_ms = time.as_secs_f64() * 1000.0;
            }
        } else {
            metrics.failed_connections += 1;
        }
    }

    /// Record packet event
    pub fn record_packet(&self, sent: bool, size: usize, dropped: bool) {
        let mut metrics = self.packet_metrics.write();
        if sent {
            metrics.packets_sent += 1;
        } else {
            metrics.packets_received += 1;
        }
        if dropped {
            metrics.packets_dropped += 1;
        }
        metrics.average_packet_size = size as f64;
    }

    /// Get current performance snapshot
    pub fn get_snapshot(&self) -> PerformanceSnapshot {
        let throughput = self.throughput_samples.read().get_statistics();
        let latency = self.latency_samples.read().get_statistics();
        let conn_metrics = self.connection_metrics.read().clone();
        let packet_metrics = self.packet_metrics.read().clone();

        let uptime = self.start_time.elapsed().as_secs_f64();

        let connections = ConnectionSnapshot {
            active: conn_metrics.active_connections,
            total: conn_metrics.total_connections,
            failed: conn_metrics.failed_connections,
            success_rate: if conn_metrics.total_connections > 0 {
                (conn_metrics.total_connections - conn_metrics.failed_connections) as f64
                    / conn_metrics.total_connections as f64
            } else {
                1.0
            },
            connections_per_sec: conn_metrics.connection_rate,
        };

        let packets = PacketSnapshot {
            sent: packet_metrics.packets_sent,
            received: packet_metrics.packets_received,
            dropped: packet_metrics.packets_dropped,
            loss_rate: if packet_metrics.packets_sent > 0 {
                packet_metrics.packets_dropped as f64 / packet_metrics.packets_sent as f64
            } else {
                0.0
            },
            packets_per_sec: packet_metrics.packet_rate,
        };

        let performance_tier = NetworkTier::from_gbps(throughput.current_gbps);

        let health_status = self.determine_health(&throughput, &latency, &connections, &packets);

        PerformanceSnapshot {
            timestamp: chrono::Utc::now().to_rfc3339(),
            uptime_seconds: uptime,
            throughput,
            latency,
            connections,
            packets,
            performance_tier,
            health_status,
        }
    }

    /// Determine system health based on real metrics
    fn determine_health(
        &self,
        throughput: &ThroughputStats,
        latency: &LatencyStats,
        connections: &ConnectionSnapshot,
        packets: &PacketSnapshot,
    ) -> HealthStatus {
        let mut issues = Vec::new();

        // Check throughput
        if throughput.average_gbps < self.throughput_threshold * 0.5 {
            issues.push(format!(
                "Throughput critically low: {:.3} Gbps (threshold: {:.3} Gbps)",
                throughput.average_gbps, self.throughput_threshold
            ));
        } else if throughput.average_gbps < self.throughput_threshold * 0.8 {
            issues.push(format!(
                "Throughput below target: {:.3} Gbps",
                throughput.average_gbps
            ));
        }

        // Check latency
        if latency.p95_ms > self.latency_threshold * 2.0 {
            issues.push(format!(
                "Latency critically high: {:.2} ms (p95)",
                latency.p95_ms
            ));
        } else if latency.p95_ms > self.latency_threshold {
            issues.push(format!("Latency elevated: {:.2} ms (p95)", latency.p95_ms));
        }

        // Check connection health
        if connections.success_rate < 0.9 {
            issues.push(format!(
                "High connection failure rate: {:.1}%",
                (1.0 - connections.success_rate) * 100.0
            ));
        }

        // Check packet loss
        if packets.loss_rate > 0.05 {
            issues.push(format!("High packet loss: {:.1}%", packets.loss_rate * 100.0));
        }

        match issues.len() {
            0 => HealthStatus::Healthy {
                message: format!(
                    "System operating normally at {:.3} Gbps",
                    throughput.current_gbps
                ),
            },
            1..=2 => HealthStatus::Warning {
                message: issues.join(", "),
            },
            _ => HealthStatus::Critical {
                message: format!("Multiple issues detected: {}", issues.join("; ")),
            },
        }
    }

    /// Export performance data for analysis
    pub fn export_metrics(&self) -> String {
        let snapshot = self.get_snapshot();
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|e| {
            format!("Failed to export metrics: {}", e)
        })
    }
}

impl Clone for PerformanceMonitor {
    fn clone(&self) -> Self {
        Self {
            throughput_samples: self.throughput_samples.clone(),
            latency_samples: self.latency_samples.clone(),
            connection_metrics: self.connection_metrics.clone(),
            packet_metrics: self.packet_metrics.clone(),
            throughput_threshold: self.throughput_threshold,
            latency_threshold: self.latency_threshold,
            monitoring_active: self.monitoring_active.clone(),
            start_time: self.start_time,
        }
    }
}

/// Calculate percentile from sorted array
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p / 100.0) as usize;
    sorted[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_monitoring() {
        let monitor = PerformanceMonitor::new(1.0, 10.0);

        // Record some measurements
        monitor.record_bytes(1_000_000);
        monitor.record_latency(Duration::from_millis(5));
        monitor.record_connection(true, Some(Duration::from_millis(2)));
        monitor.record_packet(true, 1500, false);

        // Get snapshot
        let snapshot = monitor.get_snapshot();
        assert!(snapshot.uptime_seconds >= 0.0);

        // Check health determination
        match snapshot.health_status {
            HealthStatus::Healthy { .. } => {}
            _ => panic!("Expected healthy status for initial state"),
        }
    }

    #[test]
    fn test_network_tier_classification() {
        assert!(matches!(NetworkTier::from_gbps(0.05), NetworkTier::Slow { .. }));
        assert!(matches!(NetworkTier::from_gbps(0.5), NetworkTier::Home { .. }));
        assert!(matches!(NetworkTier::from_gbps(1.5), NetworkTier::Standard { .. }));
        assert!(matches!(NetworkTier::from_gbps(5.0), NetworkTier::Performance { .. }));
        assert!(matches!(NetworkTier::from_gbps(15.0), NetworkTier::Enterprise { .. }));
        assert!(matches!(NetworkTier::from_gbps(30.0), NetworkTier::DataCenter { .. }));
    }
}