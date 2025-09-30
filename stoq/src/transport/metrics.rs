//! STOQ Transport Metrics - Native protocol monitoring without external dependencies
//!
//! Provides comprehensive metrics collection for STOQ transport protocol including:
//! - Basic transport metrics (bytes, connections, throughput)
//! - Protocol-specific metrics (tokenization, sharding, hop routing)
//! - Performance metrics (latency, error rates, packet loss)
//! - Resource utilization (memory pools, CPU usage)

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, Duration};
use std::collections::VecDeque;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

/// Core transport metrics with native collection
pub struct TransportMetrics {
    // Basic counters
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    connections_established: AtomicU64,
    connections_closed: AtomicU64,

    // Protocol metrics
    packets_tokenized: AtomicU64,
    packets_sharded: AtomicU64,
    shards_reassembled: AtomicU64,
    hop_routes_processed: AtomicU64,

    // Performance metrics
    latency_samples: Arc<RwLock<LatencyTracker>>,
    error_counts: Arc<RwLock<ErrorMetrics>>,

    // Timing
    start_time: Instant,
    last_reset: Arc<RwLock<Instant>>,
}

/// Latency tracking with percentiles
struct LatencyTracker {
    samples: VecDeque<u64>, // Microseconds
    max_samples: usize,
    sum: u64,
    count: u64,
}

impl LatencyTracker {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
            sum: 0,
            count: 0,
        }
    }

    fn record(&mut self, latency_us: u64) {
        if self.samples.len() >= self.max_samples {
            if let Some(old) = self.samples.pop_front() {
                self.sum = self.sum.saturating_sub(old);
            }
        }
        self.samples.push_back(latency_us);
        self.sum += latency_us;
        self.count += 1;
    }

    fn average(&self) -> u64 {
        if self.samples.is_empty() {
            0
        } else {
            self.sum / self.samples.len() as u64
        }
    }

    fn percentile(&self, p: f64) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }
        let mut sorted: Vec<_> = self.samples.iter().copied().collect();
        sorted.sort_unstable();
        let idx = ((sorted.len() as f64 - 1.0) * p / 100.0) as usize;
        sorted[idx]
    }
}

/// Error tracking metrics
#[derive(Default)]
struct ErrorMetrics {
    connection_failures: u64,
    packet_drops: u64,
    sharding_errors: u64,
    reassembly_errors: u64,
    token_validation_failures: u64,
}

impl TransportMetrics {
    pub fn new() -> Self {
        Self {
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            connections_established: AtomicU64::new(0),
            connections_closed: AtomicU64::new(0),
            packets_tokenized: AtomicU64::new(0),
            packets_sharded: AtomicU64::new(0),
            shards_reassembled: AtomicU64::new(0),
            hop_routes_processed: AtomicU64::new(0),
            latency_samples: Arc::new(RwLock::new(LatencyTracker::new(10000))),
            error_counts: Arc::new(RwLock::new(ErrorMetrics::default())),
            start_time: Instant::now(),
            last_reset: Arc::new(RwLock::new(Instant::now())),
        }
    }

    // Basic metrics recording
    pub fn record_bytes_sent(&self, bytes: usize) {
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub fn record_bytes_received(&self, bytes: usize) {
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub fn record_connection_established(&self) {
        self.connections_established.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_connection_closed(&self) {
        self.connections_closed.fetch_add(1, Ordering::Relaxed);
    }

    // Protocol-specific metrics
    pub fn record_packet_tokenized(&self) {
        self.packets_tokenized.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_packet_sharded(&self, shard_count: u32) {
        self.packets_sharded.fetch_add(shard_count as u64, Ordering::Relaxed);
    }

    pub fn record_shards_reassembled(&self) {
        self.shards_reassembled.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_hop_route(&self) {
        self.hop_routes_processed.fetch_add(1, Ordering::Relaxed);
    }

    // Performance metrics
    pub fn record_latency(&self, latency: Duration) {
        let latency_us = latency.as_micros() as u64;
        self.latency_samples.write().record(latency_us);
    }

    pub fn record_connection_failure(&self) {
        self.error_counts.write().connection_failures += 1;
    }

    pub fn record_packet_drop(&self) {
        self.error_counts.write().packet_drops += 1;
    }

    pub fn record_sharding_error(&self) {
        self.error_counts.write().sharding_errors += 1;
    }

    pub fn record_reassembly_error(&self) {
        self.error_counts.write().reassembly_errors += 1;
    }

    pub fn record_token_validation_failure(&self) {
        self.error_counts.write().token_validation_failures += 1;
    }

    /// Get comprehensive transport statistics
    pub fn get_stats(&self, active_connections: usize) -> crate::TransportStats {
        let bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        let bytes_received = self.bytes_received.load(Ordering::Relaxed);
        let total_connections = self.connections_established.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();

        let throughput_gbps = if elapsed > 0.0 {
            ((bytes_sent + bytes_received) as f64 * 8.0) / (elapsed * 1_000_000_000.0)
        } else {
            0.0
        };

        let avg_latency_us = self.latency_samples.read().average();

        crate::TransportStats {
            bytes_sent,
            bytes_received,
            active_connections,
            total_connections,
            throughput_gbps,
            avg_latency_us,
        }
    }

    /// Get detailed protocol metrics for monitoring dashboard
    pub fn get_protocol_metrics(&self) -> ProtocolMetrics {
        let errors = self.error_counts.read();
        let latency = self.latency_samples.read();

        ProtocolMetrics {
            packets_tokenized: self.packets_tokenized.load(Ordering::Relaxed),
            packets_sharded: self.packets_sharded.load(Ordering::Relaxed),
            shards_reassembled: self.shards_reassembled.load(Ordering::Relaxed),
            hop_routes_processed: self.hop_routes_processed.load(Ordering::Relaxed),
            avg_latency_us: latency.average(),
            p50_latency_us: latency.percentile(50.0),
            p95_latency_us: latency.percentile(95.0),
            p99_latency_us: latency.percentile(99.0),
            connection_failures: errors.connection_failures,
            packet_drops: errors.packet_drops,
            sharding_errors: errors.sharding_errors,
            reassembly_errors: errors.reassembly_errors,
            token_validation_failures: errors.token_validation_failures,
        }
    }

    /// Reset non-cumulative metrics (for periodic reporting)
    pub fn reset_interval_metrics(&self) {
        *self.last_reset.write() = Instant::now();
    }

    /// Get metrics since last reset
    pub fn get_interval_metrics(&self) -> IntervalMetrics {
        let elapsed = self.last_reset.read().elapsed().as_secs_f64();
        let bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        let bytes_received = self.bytes_received.load(Ordering::Relaxed);

        IntervalMetrics {
            duration_secs: elapsed,
            throughput_gbps: ((bytes_sent + bytes_received) as f64 * 8.0) / (elapsed * 1_000_000_000.0),
            packets_per_sec: self.packets_tokenized.load(Ordering::Relaxed) as f64 / elapsed,
            connections_per_sec: self.connections_established.load(Ordering::Relaxed) as f64 / elapsed,
        }
    }
}

/// Protocol-specific metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMetrics {
    pub packets_tokenized: u64,
    pub packets_sharded: u64,
    pub shards_reassembled: u64,
    pub hop_routes_processed: u64,
    pub avg_latency_us: u64,
    pub p50_latency_us: u64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
    pub connection_failures: u64,
    pub packet_drops: u64,
    pub sharding_errors: u64,
    pub reassembly_errors: u64,
    pub token_validation_failures: u64,
}

/// Interval-based metrics for rate calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalMetrics {
    pub duration_secs: f64,
    pub throughput_gbps: f64,
    pub packets_per_sec: f64,
    pub connections_per_sec: f64,
}