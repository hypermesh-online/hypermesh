// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Unified HyperMesh Metrics
//!
//! Combines HyperMesh intelligence metrics (PoS, asset, routing, privacy)
//! with transport-level metrics (packet counts, latency, throughput).
//! This is the single metrics collection point for the eBPF subsystem.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Unified HyperMesh metrics combining intelligence and transport layers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HyperMeshMetrics {
    /// Proof of State validation metrics
    pub pos_metrics: ProofOfStateMetrics,
    /// Asset hash verification metrics
    pub asset_metrics: AssetHashMetrics,
    /// Matrix routing metrics
    pub routing_metrics: MatrixRoutingMetrics,
    /// Privacy tier enforcement metrics
    pub privacy_metrics: PrivacyTierMetrics,
    /// Transport-level packet metrics
    pub transport_metrics: TransportMetrics,
}

/// Proof of State validation metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProofOfStateMetrics {
    /// Total PoS validations attempted
    pub total_validations: u64,
    /// Successful validations
    pub successful: u64,
    /// Failed validations
    pub failed: u64,
    /// Timestamp validation failures
    pub timestamp_failures: u64,
    /// Proof of Stake failures
    pub pos_stake_failures: u64,
    /// Proof of Work failures
    pub pos_work_failures: u64,
    /// Proof of Space failures
    pub pos_space_failures: u64,
    /// Average validation time (microseconds)
    pub avg_validation_us: u64,
}

impl ProofOfStateMetrics {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_validations == 0 {
            return 0.0;
        }
        (self.successful as f64 / self.total_validations as f64) * 100.0
    }

    /// Calculate failure rate
    pub fn failure_rate(&self) -> f64 {
        100.0 - self.success_rate()
    }
}

/// Asset hash verification metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetHashMetrics {
    /// Total asset hash validations
    pub total_validations: u64,
    /// Successful validations
    pub successful: u64,
    /// Hash mismatches
    pub hash_mismatches: u64,
    /// Shard validation failures
    pub shard_failures: u64,
    /// Registry lookup failures
    pub registry_failures: u64,
    /// Total assets validated
    pub unique_assets: u64,
    /// Total shards validated
    pub total_shards: u64,
    /// Average validation time (microseconds)
    pub avg_validation_us: u64,
}

impl AssetHashMetrics {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_validations == 0 {
            return 0.0;
        }
        (self.successful as f64 / self.total_validations as f64) * 100.0
    }
}

/// Matrix routing metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatrixRoutingMetrics {
    /// Total routing validations
    pub total_validations: u64,
    /// Successful validations
    pub successful: u64,
    /// Path validation failures (loops, invalid coordinates)
    pub path_failures: u64,
    /// Topology violations
    pub topology_violations: u64,
    /// Average path length (hops)
    pub avg_path_length: f64,
    /// Average validation time (microseconds)
    pub avg_validation_us: u64,
}

impl MatrixRoutingMetrics {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_validations == 0 {
            return 0.0;
        }
        (self.successful as f64 / self.total_validations as f64) * 100.0
    }
}

/// Privacy mode enforcement metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrivacyTierMetrics {
    /// Connections by privacy mode
    pub anonymous_connections: u64,
    pub private_connections: u64,
    pub public_connections: u64,
    /// Privacy mode violations (access denied)
    pub tier_violations: u64,
    /// Total privacy checks
    pub total_checks: u64,
}

impl PrivacyTierMetrics {
    /// Get total connections
    pub fn total_connections(&self) -> u64 {
        self.anonymous_connections + self.private_connections + self.public_connections
    }

    /// Calculate violation rate
    pub fn violation_rate(&self) -> f64 {
        if self.total_checks == 0 {
            return 0.0;
        }
        (self.tier_violations as f64 / self.total_checks as f64) * 100.0
    }
}

/// Transport-level metrics from eBPF/XDP layer
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransportMetrics {
    /// Total packets processed by XDP
    pub total_packets: u64,
    /// Packets per second (current rate)
    pub packets_per_second: f64,
    /// Bytes per second (current throughput)
    pub bytes_per_second: f64,
    /// Total bytes processed
    pub total_bytes: u64,
    /// Packets dropped at kernel level
    pub kernel_drops: u64,
    /// Packets redirected to AF_XDP
    pub af_xdp_redirects: u64,
    /// Zero-copy operations count
    pub zero_copy_ops: u64,
    /// Standard I/O fallback operations count
    pub memcpy_ops: u64,
    /// Latency: minimum (microseconds)
    pub latency_min_us: u64,
    /// Latency: maximum (microseconds)
    pub latency_max_us: u64,
    /// Latency: average (microseconds)
    pub latency_avg_us: u64,
}

impl TransportMetrics {
    /// Get throughput in Gbps
    pub fn throughput_gbps(&self) -> f64 {
        self.bytes_per_second * 8.0 / 1_000_000_000.0
    }

    /// Get packet rate in millions of packets per second
    pub fn packet_rate_mpps(&self) -> f64 {
        self.packets_per_second / 1_000_000.0
    }
}

/// HyperMesh metrics collector
pub struct HyperMeshMetricsCollector {
    /// Atomic counters for lock-free updates
    pos_validations: Arc<AtomicU64>,
    pos_successful: Arc<AtomicU64>,
    pos_failed: Arc<AtomicU64>,
    pos_timestamp_failures: Arc<AtomicU64>,

    asset_validations: Arc<AtomicU64>,
    asset_successful: Arc<AtomicU64>,
    asset_hash_mismatches: Arc<AtomicU64>,
    asset_shard_failures: Arc<AtomicU64>,

    routing_validations: Arc<AtomicU64>,
    routing_successful: Arc<AtomicU64>,
    routing_path_failures: Arc<AtomicU64>,

    privacy_anonymous: Arc<AtomicU64>,
    privacy_private: Arc<AtomicU64>,
    privacy_public: Arc<AtomicU64>,
    privacy_violations: Arc<AtomicU64>,

    // Transport-level counters
    transport_packets: Arc<AtomicU64>,
    transport_bytes: Arc<AtomicU64>,
    transport_drops: Arc<AtomicU64>,
    transport_redirects: Arc<AtomicU64>,
    transport_zero_copy: Arc<AtomicU64>,
    transport_memcpy: Arc<AtomicU64>,

    /// Last collection timestamp for rate calculations
    last_collection: Arc<RwLock<Instant>>,
    /// Previous packet count for rate calculation
    prev_packets: Arc<AtomicU64>,
    /// Previous byte count for rate calculation
    prev_bytes: Arc<AtomicU64>,

    /// Current metrics snapshot
    current: Arc<RwLock<HyperMeshMetrics>>,
}

impl HyperMeshMetricsCollector {
    /// Create new metrics collector
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            pos_validations: Arc::new(AtomicU64::new(0)),
            pos_successful: Arc::new(AtomicU64::new(0)),
            pos_failed: Arc::new(AtomicU64::new(0)),
            pos_timestamp_failures: Arc::new(AtomicU64::new(0)),

            asset_validations: Arc::new(AtomicU64::new(0)),
            asset_successful: Arc::new(AtomicU64::new(0)),
            asset_hash_mismatches: Arc::new(AtomicU64::new(0)),
            asset_shard_failures: Arc::new(AtomicU64::new(0)),

            routing_validations: Arc::new(AtomicU64::new(0)),
            routing_successful: Arc::new(AtomicU64::new(0)),
            routing_path_failures: Arc::new(AtomicU64::new(0)),

            privacy_anonymous: Arc::new(AtomicU64::new(0)),
            privacy_private: Arc::new(AtomicU64::new(0)),
            privacy_public: Arc::new(AtomicU64::new(0)),
            privacy_violations: Arc::new(AtomicU64::new(0)),

            transport_packets: Arc::new(AtomicU64::new(0)),
            transport_bytes: Arc::new(AtomicU64::new(0)),
            transport_drops: Arc::new(AtomicU64::new(0)),
            transport_redirects: Arc::new(AtomicU64::new(0)),
            transport_zero_copy: Arc::new(AtomicU64::new(0)),
            transport_memcpy: Arc::new(AtomicU64::new(0)),

            last_collection: Arc::new(RwLock::new(Instant::now())),
            prev_packets: Arc::new(AtomicU64::new(0)),
            prev_bytes: Arc::new(AtomicU64::new(0)),

            current: Arc::new(RwLock::new(HyperMeshMetrics::default())),
        })
    }

    /// Record Proof of State validation
    pub fn record_pos_validation(&self, success: bool) {
        self.pos_validations.fetch_add(1, Ordering::Relaxed);
        if success {
            self.pos_successful.fetch_add(1, Ordering::Relaxed);
        } else {
            self.pos_failed.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record Proof of State timestamp failure
    pub fn record_pos_timestamp_failure(&self) {
        self.pos_timestamp_failures.fetch_add(1, Ordering::Relaxed);
        self.pos_failed.fetch_add(1, Ordering::Relaxed);
        self.pos_validations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record asset hash validation
    pub fn record_asset_validation(&self, success: bool, hash_mismatch: bool) {
        self.asset_validations.fetch_add(1, Ordering::Relaxed);
        if success {
            self.asset_successful.fetch_add(1, Ordering::Relaxed);
        } else if hash_mismatch {
            self.asset_hash_mismatches.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record asset shard failure
    pub fn record_asset_shard_failure(&self) {
        self.asset_shard_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Record matrix routing validation
    pub fn record_routing_validation(&self, success: bool, path_failure: bool) {
        self.routing_validations.fetch_add(1, Ordering::Relaxed);
        if success {
            self.routing_successful.fetch_add(1, Ordering::Relaxed);
        } else if path_failure {
            self.routing_path_failures.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record privacy mode connection (u8 from eBPF map: 0=Anonymous, 1|2=Private, 3=Public)
    pub fn record_privacy_connection(&self, mode: u8) {
        match mode {
            0 => self.privacy_anonymous.fetch_add(1, Ordering::Relaxed),
            1 | 2 => self.privacy_private.fetch_add(1, Ordering::Relaxed),
            3 => self.privacy_public.fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };
    }

    /// Record privacy tier violation
    pub fn record_privacy_violation(&self) {
        self.privacy_violations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a transport packet processed by XDP
    pub fn record_transport_packet(&self, size: u64) {
        self.transport_packets.fetch_add(1, Ordering::Relaxed);
        self.transport_bytes.fetch_add(size, Ordering::Relaxed);
    }

    /// Record a kernel-level packet drop
    pub fn record_transport_drop(&self) {
        self.transport_drops.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an AF_XDP redirect
    pub fn record_transport_redirect(&self) {
        self.transport_redirects.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a zero-copy operation
    pub fn record_zero_copy(&self) {
        self.transport_zero_copy.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a memcpy fallback operation
    pub fn record_memcpy(&self) {
        self.transport_memcpy.fetch_add(1, Ordering::Relaxed);
    }

    /// Get throughput in Gbps based on current metrics
    pub fn throughput_gbps(&self) -> f64 {
        let metrics = self.current.read();
        metrics.transport_metrics.throughput_gbps()
    }

    /// Collect current metrics snapshot
    pub fn collect(&self) -> HyperMeshMetrics {
        let mut metrics = self.current.write();

        // Update PoS metrics
        metrics.pos_metrics.total_validations = self.pos_validations.load(Ordering::Relaxed);
        metrics.pos_metrics.successful = self.pos_successful.load(Ordering::Relaxed);
        metrics.pos_metrics.failed = self.pos_failed.load(Ordering::Relaxed);
        metrics.pos_metrics.timestamp_failures =
            self.pos_timestamp_failures.load(Ordering::Relaxed);

        // Update asset metrics
        metrics.asset_metrics.total_validations = self.asset_validations.load(Ordering::Relaxed);
        metrics.asset_metrics.successful = self.asset_successful.load(Ordering::Relaxed);
        metrics.asset_metrics.hash_mismatches = self.asset_hash_mismatches.load(Ordering::Relaxed);
        metrics.asset_metrics.shard_failures = self.asset_shard_failures.load(Ordering::Relaxed);

        // Update routing metrics
        metrics.routing_metrics.total_validations =
            self.routing_validations.load(Ordering::Relaxed);
        metrics.routing_metrics.successful = self.routing_successful.load(Ordering::Relaxed);
        metrics.routing_metrics.path_failures = self.routing_path_failures.load(Ordering::Relaxed);

        // Update privacy metrics
        metrics.privacy_metrics.anonymous_connections =
            self.privacy_anonymous.load(Ordering::Relaxed);
        metrics.privacy_metrics.private_connections = self.privacy_private.load(Ordering::Relaxed);
        metrics.privacy_metrics.public_connections = self.privacy_public.load(Ordering::Relaxed);
        metrics.privacy_metrics.tier_violations = self.privacy_violations.load(Ordering::Relaxed);

        // Update transport metrics
        let now = Instant::now();
        let packets = self.transport_packets.load(Ordering::Relaxed);
        let bytes = self.transport_bytes.load(Ordering::Relaxed);

        let elapsed = now
            .duration_since(*self.last_collection.read())
            .as_secs_f64();
        if elapsed > 0.0 {
            let prev_p = self.prev_packets.load(Ordering::Relaxed);
            let prev_b = self.prev_bytes.load(Ordering::Relaxed);
            let pkt_diff = packets.saturating_sub(prev_p);
            let byte_diff = bytes.saturating_sub(prev_b);
            metrics.transport_metrics.packets_per_second = pkt_diff as f64 / elapsed;
            metrics.transport_metrics.bytes_per_second = byte_diff as f64 / elapsed;
        }

        metrics.transport_metrics.total_packets = packets;
        metrics.transport_metrics.total_bytes = bytes;
        metrics.transport_metrics.kernel_drops = self.transport_drops.load(Ordering::Relaxed);
        metrics.transport_metrics.af_xdp_redirects =
            self.transport_redirects.load(Ordering::Relaxed);
        metrics.transport_metrics.zero_copy_ops = self.transport_zero_copy.load(Ordering::Relaxed);
        metrics.transport_metrics.memcpy_ops = self.transport_memcpy.load(Ordering::Relaxed);

        self.prev_packets.store(packets, Ordering::Relaxed);
        self.prev_bytes.store(bytes, Ordering::Relaxed);
        *self.last_collection.write() = now;

        metrics.clone()
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.pos_validations.store(0, Ordering::Relaxed);
        self.pos_successful.store(0, Ordering::Relaxed);
        self.pos_failed.store(0, Ordering::Relaxed);
        self.pos_timestamp_failures.store(0, Ordering::Relaxed);

        self.asset_validations.store(0, Ordering::Relaxed);
        self.asset_successful.store(0, Ordering::Relaxed);
        self.asset_hash_mismatches.store(0, Ordering::Relaxed);
        self.asset_shard_failures.store(0, Ordering::Relaxed);

        self.routing_validations.store(0, Ordering::Relaxed);
        self.routing_successful.store(0, Ordering::Relaxed);
        self.routing_path_failures.store(0, Ordering::Relaxed);

        self.privacy_anonymous.store(0, Ordering::Relaxed);
        self.privacy_private.store(0, Ordering::Relaxed);
        self.privacy_public.store(0, Ordering::Relaxed);
        self.privacy_violations.store(0, Ordering::Relaxed);

        self.transport_packets.store(0, Ordering::Relaxed);
        self.transport_bytes.store(0, Ordering::Relaxed);
        self.transport_drops.store(0, Ordering::Relaxed);
        self.transport_redirects.store(0, Ordering::Relaxed);
        self.transport_zero_copy.store(0, Ordering::Relaxed);
        self.transport_memcpy.store(0, Ordering::Relaxed);
        self.prev_packets.store(0, Ordering::Relaxed);
        self.prev_bytes.store(0, Ordering::Relaxed);
        *self.last_collection.write() = Instant::now();

        *self.current.write() = HyperMeshMetrics::default();

        tracing::info!("HyperMesh metrics reset");
    }
}

impl Default for HyperMeshMetricsCollector {
    fn default() -> Self {
        Self::new().expect("Failed to create HyperMeshMetricsCollector")
    }
}

/// Format metrics for display
impl std::fmt::Display for HyperMeshMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "HyperMesh Intelligence Metrics:")?;
        writeln!(f, "  Proof of State:")?;
        writeln!(
            f,
            "    Total: {}, Success: {}, Failed: {}",
            self.pos_metrics.total_validations,
            self.pos_metrics.successful,
            self.pos_metrics.failed
        )?;
        writeln!(
            f,
            "    Success Rate: {:.2}%",
            self.pos_metrics.success_rate()
        )?;

        writeln!(f, "  Asset Hash:")?;
        writeln!(
            f,
            "    Total: {}, Success: {}, Mismatches: {}",
            self.asset_metrics.total_validations,
            self.asset_metrics.successful,
            self.asset_metrics.hash_mismatches
        )?;
        writeln!(
            f,
            "    Success Rate: {:.2}%",
            self.asset_metrics.success_rate()
        )?;

        writeln!(f, "  Matrix Routing:")?;
        writeln!(
            f,
            "    Total: {}, Success: {}, Path Failures: {}",
            self.routing_metrics.total_validations,
            self.routing_metrics.successful,
            self.routing_metrics.path_failures
        )?;
        writeln!(
            f,
            "    Success Rate: {:.2}%",
            self.routing_metrics.success_rate()
        )?;

        writeln!(f, "  Privacy Modes:")?;
        writeln!(
            f,
            "    Anonymous: {}, Private: {}, Public: {}",
            self.privacy_metrics.anonymous_connections,
            self.privacy_metrics.private_connections,
            self.privacy_metrics.public_connections
        )?;
        writeln!(
            f,
            "    Violations: {}",
            self.privacy_metrics.tier_violations
        )?;

        writeln!(f, "  Transport:")?;
        writeln!(
            f,
            "    Packets: {}, {:.2} pps, {:.2} Gbps",
            self.transport_metrics.total_packets,
            self.transport_metrics.packets_per_second,
            self.transport_metrics.throughput_gbps()
        )?;
        writeln!(
            f,
            "    Drops: {}, AF_XDP Redirects: {}",
            self.transport_metrics.kernel_drops, self.transport_metrics.af_xdp_redirects
        )?;
        writeln!(
            f,
            "    Zero-copy: {}, Memcpy: {}",
            self.transport_metrics.zero_copy_ops, self.transport_metrics.memcpy_ops
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = HyperMeshMetricsCollector::new().expect("test: create metrics collector");

        collector.record_pos_validation(true);
        collector.record_pos_validation(false);
        collector.record_asset_validation(true, false);
        collector.record_routing_validation(true, false);
        collector.record_privacy_connection(1);

        let metrics = collector.collect();

        assert_eq!(metrics.pos_metrics.total_validations, 2);
        assert_eq!(metrics.pos_metrics.successful, 1);
        assert_eq!(metrics.pos_metrics.failed, 1);
        assert_eq!(metrics.asset_metrics.total_validations, 1);
        assert_eq!(metrics.routing_metrics.total_validations, 1);
        assert_eq!(metrics.privacy_metrics.private_connections, 1);
    }

    #[test]
    fn test_success_rates() {
        let mut metrics = HyperMeshMetrics::default();
        metrics.pos_metrics.total_validations = 100;
        metrics.pos_metrics.successful = 95;
        metrics.pos_metrics.failed = 5;

        assert_eq!(metrics.pos_metrics.success_rate(), 95.0);
        assert_eq!(metrics.pos_metrics.failure_rate(), 5.0);
    }

    #[test]
    fn test_metrics_reset() {
        let collector = HyperMeshMetricsCollector::new().expect("test: create metrics collector");

        collector.record_pos_validation(true);
        collector.record_asset_validation(true, false);

        let metrics = collector.collect();
        assert!(metrics.pos_metrics.total_validations > 0);

        collector.reset();

        let metrics = collector.collect();
        assert_eq!(metrics.pos_metrics.total_validations, 0);
        assert_eq!(metrics.asset_metrics.total_validations, 0);
    }

    #[test]
    fn test_transport_metrics() {
        let collector = HyperMeshMetricsCollector::new().expect("test: create metrics collector");

        collector.record_transport_packet(1500);
        collector.record_transport_packet(800);
        collector.record_transport_drop();
        collector.record_transport_redirect();
        collector.record_zero_copy();
        collector.record_memcpy();

        let metrics = collector.collect();
        assert_eq!(metrics.transport_metrics.total_packets, 2);
        assert_eq!(metrics.transport_metrics.total_bytes, 2300);
        assert_eq!(metrics.transport_metrics.kernel_drops, 1);
        assert_eq!(metrics.transport_metrics.af_xdp_redirects, 1);
        assert_eq!(metrics.transport_metrics.zero_copy_ops, 1);
        assert_eq!(metrics.transport_metrics.memcpy_ops, 1);
    }

    #[test]
    fn test_transport_metrics_reset() {
        let collector = HyperMeshMetricsCollector::new().expect("test: create metrics collector");

        collector.record_transport_packet(1000);
        collector.record_transport_drop();

        let metrics = collector.collect();
        assert_eq!(metrics.transport_metrics.total_packets, 1);

        collector.reset();

        let metrics = collector.collect();
        assert_eq!(metrics.transport_metrics.total_packets, 0);
        assert_eq!(metrics.transport_metrics.kernel_drops, 0);
    }
}
