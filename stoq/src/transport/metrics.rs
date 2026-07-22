// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport Metrics - Native protocol monitoring without external dependencies
//!
//! Provides comprehensive metrics collection for STOQ transport protocol including:
//! - Basic transport metrics (bytes, connections, throughput)
//! - Protocol-specific metrics (tokenization, sharding, hop routing)
//! - Performance metrics (latency, error rates, packet loss)
//! - Resource utilization (memory pools, CPU usage)

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

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

/// Latency tracking with percentiles and jitter computation.
struct LatencyTracker {
    samples: VecDeque<u64>, // Microseconds
    max_samples: usize,
    sum: u64,
    count: u64,
    /// Sum of absolute differences between consecutive samples (for jitter).
    jitter_sum: u64,
    /// Number of jitter samples (consecutive pairs).
    jitter_count: u64,
    /// Most recent sample (for computing inter-sample difference).
    last_sample: Option<u64>,
}

impl LatencyTracker {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
            sum: 0,
            count: 0,
            jitter_sum: 0,
            jitter_count: 0,
            last_sample: None,
        }
    }

    fn record(&mut self, latency_us: u64) {
        // Compute jitter as absolute difference between consecutive RTTs
        if let Some(prev) = self.last_sample {
            let diff = if latency_us > prev {
                latency_us - prev
            } else {
                prev - latency_us
            };
            self.jitter_sum += diff;
            self.jitter_count += 1;
        }
        self.last_sample = Some(latency_us);

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

    /// Mean jitter (average absolute inter-sample RTT difference) in microseconds.
    fn jitter(&self) -> u64 {
        if self.jitter_count == 0 {
            0
        } else {
            self.jitter_sum / self.jitter_count
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

    /// Min latency in the current window.
    fn min(&self) -> u64 {
        self.samples.iter().copied().min().unwrap_or(0)
    }

    /// Max latency in the current window.
    fn max(&self) -> u64 {
        self.samples.iter().copied().max().unwrap_or(0)
    }

    fn sample_count(&self) -> u64 {
        self.count
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
    retransmissions: u64,
}

impl Default for TransportMetrics {
    fn default() -> Self {
        Self::new()
    }
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
        self.bytes_received
            .fetch_add(bytes as u64, Ordering::Relaxed);
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
        self.packets_sharded
            .fetch_add(shard_count as u64, Ordering::Relaxed);
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

    /// Record a QUIC retransmission event.
    pub fn record_retransmission(&self) {
        self.error_counts.write().retransmissions += 1;
    }

    /// Get a rich transport snapshot including jitter and loss metrics.
    ///
    /// This is the primary method for feeding transport data to ngauge.
    pub fn get_transport_snapshot(&self, active_connections: usize) -> TransportSnapshot {
        let bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        let bytes_received = self.bytes_received.load(Ordering::Relaxed);
        let total_connections = self.connections_established.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();

        let throughput_bps = if elapsed_secs > 0.0 {
            ((bytes_sent + bytes_received) as f64 * 8.0) / elapsed_secs
        } else {
            0.0
        };

        let latency = self.latency_samples.read();
        let errors = self.error_counts.read();

        let total_packets = self.packets_tokenized.load(Ordering::Relaxed);
        let loss_ratio = if total_packets > 0 {
            errors.packet_drops as f64 / total_packets as f64
        } else {
            0.0
        };

        TransportSnapshot {
            bytes_sent,
            bytes_received,
            active_connections,
            total_connections,
            throughput_bps,
            avg_latency_us: latency.average(),
            min_latency_us: latency.min(),
            max_latency_us: latency.max(),
            p50_latency_us: latency.percentile(50.0),
            p95_latency_us: latency.percentile(95.0),
            p99_latency_us: latency.percentile(99.0),
            jitter_us: latency.jitter(),
            latency_sample_count: latency.sample_count(),
            packet_drops: errors.packet_drops,
            retransmissions: errors.retransmissions,
            loss_ratio,
            uptime: elapsed,
        }
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
            throughput_gbps: ((bytes_sent + bytes_received) as f64 * 8.0)
                / (elapsed * 1_000_000_000.0),
            packets_per_sec: self.packets_tokenized.load(Ordering::Relaxed) as f64 / elapsed,
            connections_per_sec: self.connections_established.load(Ordering::Relaxed) as f64
                / elapsed,
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

/// Rich transport metrics snapshot for feeding to ngauge.
///
/// Contains measured values from actual QUIC connections including
/// latency percentiles, jitter, and loss ratios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportSnapshot {
    /// Total bytes sent since transport start.
    pub bytes_sent: u64,
    /// Total bytes received since transport start.
    pub bytes_received: u64,
    /// Current number of active connections.
    pub active_connections: usize,
    /// Total connections established since start.
    pub total_connections: u64,
    /// Current throughput in bits per second.
    pub throughput_bps: f64,
    /// Average RTT in microseconds.
    pub avg_latency_us: u64,
    /// Minimum RTT in microseconds (within sample window).
    pub min_latency_us: u64,
    /// Maximum RTT in microseconds (within sample window).
    pub max_latency_us: u64,
    /// 50th percentile RTT in microseconds.
    pub p50_latency_us: u64,
    /// 95th percentile RTT in microseconds.
    pub p95_latency_us: u64,
    /// 99th percentile RTT in microseconds.
    pub p99_latency_us: u64,
    /// Mean jitter (inter-sample RTT variance) in microseconds.
    pub jitter_us: u64,
    /// Number of latency samples recorded.
    pub latency_sample_count: u64,
    /// Total packets dropped.
    pub packet_drops: u64,
    /// Total retransmissions.
    pub retransmissions: u64,
    /// Loss ratio (drops / total packets), 0.0 to 1.0.
    pub loss_ratio: f64,
    /// Time since transport started.
    #[serde(skip)]
    pub uptime: Duration,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_metrics_are_zero() {
        let m = TransportMetrics::new();
        let snap = m.get_transport_snapshot(0);
        assert_eq!(snap.bytes_sent, 0);
        assert_eq!(snap.bytes_received, 0);
        assert_eq!(snap.active_connections, 0);
        assert_eq!(snap.avg_latency_us, 0);
        assert_eq!(snap.jitter_us, 0);
        assert_eq!(snap.packet_drops, 0);
        assert_eq!(snap.retransmissions, 0);
        assert!(snap.loss_ratio.abs() < 1e-9);
    }

    #[test]
    fn bytes_tracking() {
        let m = TransportMetrics::new();
        m.record_bytes_sent(1024);
        m.record_bytes_sent(2048);
        m.record_bytes_received(512);

        let snap = m.get_transport_snapshot(0);
        assert_eq!(snap.bytes_sent, 3072);
        assert_eq!(snap.bytes_received, 512);
    }

    #[test]
    fn latency_percentiles() {
        let m = TransportMetrics::new();
        // Record 100 latency samples from 1us to 100us
        for i in 1..=100u64 {
            m.record_latency(Duration::from_micros(i));
        }

        let snap = m.get_transport_snapshot(0);
        assert_eq!(snap.avg_latency_us, 50); // (1+100)/2 = 50.5, integer = 50
        assert_eq!(snap.min_latency_us, 1);
        assert_eq!(snap.max_latency_us, 100);
        assert!(snap.p50_latency_us >= 45 && snap.p50_latency_us <= 55);
        assert!(snap.p95_latency_us >= 90);
        assert!(snap.p99_latency_us >= 95);
        assert_eq!(snap.latency_sample_count, 100);
    }

    #[test]
    fn jitter_computation() {
        let m = TransportMetrics::new();
        // Record alternating latencies to create jitter
        m.record_latency(Duration::from_micros(100));
        m.record_latency(Duration::from_micros(200));
        m.record_latency(Duration::from_micros(100));
        m.record_latency(Duration::from_micros(200));

        let snap = m.get_transport_snapshot(0);
        // Jitter = mean of |200-100|, |100-200|, |200-100| = 100
        assert_eq!(snap.jitter_us, 100);
    }

    #[test]
    fn zero_jitter_for_constant_latency() {
        let m = TransportMetrics::new();
        m.record_latency(Duration::from_micros(50));
        m.record_latency(Duration::from_micros(50));
        m.record_latency(Duration::from_micros(50));

        let snap = m.get_transport_snapshot(0);
        assert_eq!(snap.jitter_us, 0);
    }

    #[test]
    fn loss_ratio_computation() {
        let m = TransportMetrics::new();
        // Simulate 100 packets tokenized, 5 dropped
        for _ in 0..100 {
            m.record_packet_tokenized();
        }
        for _ in 0..5 {
            m.record_packet_drop();
        }

        let snap = m.get_transport_snapshot(2);
        assert!((snap.loss_ratio - 0.05).abs() < 1e-9);
        assert_eq!(snap.packet_drops, 5);
        assert_eq!(snap.active_connections, 2);
    }

    #[test]
    fn retransmission_tracking() {
        let m = TransportMetrics::new();
        m.record_retransmission();
        m.record_retransmission();
        m.record_retransmission();

        let snap = m.get_transport_snapshot(0);
        assert_eq!(snap.retransmissions, 3);
    }

    #[test]
    fn protocol_metrics_include_all_errors() {
        let m = TransportMetrics::new();
        m.record_connection_failure();
        m.record_sharding_error();
        m.record_reassembly_error();
        m.record_token_validation_failure();

        let proto = m.get_protocol_metrics();
        assert_eq!(proto.connection_failures, 1);
        assert_eq!(proto.sharding_errors, 1);
        assert_eq!(proto.reassembly_errors, 1);
        assert_eq!(proto.token_validation_failures, 1);
    }

    #[test]
    fn transport_stats_backward_compat() {
        let m = TransportMetrics::new();
        m.record_bytes_sent(1000);
        m.record_latency(Duration::from_micros(500));

        let stats = m.get_stats(3);
        assert_eq!(stats.bytes_sent, 1000);
        assert_eq!(stats.active_connections, 3);
        assert_eq!(stats.avg_latency_us, 500);
    }

    #[test]
    fn snapshot_serialization() {
        let m = TransportMetrics::new();
        m.record_bytes_sent(42);
        m.record_latency(Duration::from_micros(100));

        let snap = m.get_transport_snapshot(1);
        let json = serde_json::to_string(&snap).expect("test: serialize snapshot");
        let back: TransportSnapshot =
            serde_json::from_str(&json).expect("test: deserialize snapshot");
        assert_eq!(back.bytes_sent, 42);
        assert_eq!(back.avg_latency_us, 100);
    }
}
