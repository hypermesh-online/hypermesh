//! eBPF metrics collection for STOQ transport layer
//!
//! Collects kernel-level metrics for transport performance monitoring
//! including packet counts, latency measurements, and connection tracking.

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use parking_lot::RwLock;

// eBPF map types would normally come from aya

/// eBPF metrics collected at kernel level
#[derive(Debug, Default, Clone)]
pub struct EbpfMetrics {
    /// Packet-level metrics
    pub packet_metrics: PacketMetrics,
    /// Connection-level metrics
    pub connection_metrics: ConnectionMetrics,
    /// Latency measurements
    pub latency_metrics: LatencyMetrics,
    /// CPU utilization metrics
    pub cpu_metrics: CpuMetrics,
    /// Memory metrics
    pub memory_metrics: MemoryMetrics,
}

/// Packet-level metrics from eBPF
#[derive(Debug, Default, Clone)]
pub struct PacketMetrics {
    /// Total packets processed
    pub total_packets: u64,
    /// Packets by size distribution
    pub size_distribution: SizeDistribution,
    /// Packets per second (current rate)
    pub packets_per_second: f64,
    /// Bytes per second (current throughput)
    pub bytes_per_second: f64,
    /// Packet drops at kernel level
    pub kernel_drops: u64,
    /// Retransmissions detected
    pub retransmissions: u64,
}

/// Packet size distribution buckets
#[derive(Debug, Default, Clone)]
pub struct SizeDistribution {
    /// Packets < 64 bytes
    pub tiny: u64,
    /// Packets 64-256 bytes
    pub small: u64,
    /// Packets 256-1024 bytes
    pub medium: u64,
    /// Packets 1024-1500 bytes
    pub large: u64,
    /// Packets > 1500 bytes (jumbo)
    pub jumbo: u64,
}

/// Connection-level metrics
#[derive(Debug, Default, Clone)]
pub struct ConnectionMetrics {
    /// Active connections
    pub active_connections: u64,
    /// New connections per second
    pub new_connections_per_sec: f64,
    /// Closed connections
    pub closed_connections: u64,
    /// Failed connections
    pub failed_connections: u64,
    /// Connection state distribution
    pub state_distribution: StateDistribution,
}

/// Connection state distribution
#[derive(Debug, Default, Clone)]
pub struct StateDistribution {
    /// Handshaking
    pub handshaking: u64,
    /// Established
    pub established: u64,
    /// Closing
    pub closing: u64,
    /// Time-wait
    pub time_wait: u64,
}

/// Latency metrics from kernel timestamps
#[derive(Debug, Default, Clone)]
pub struct LatencyMetrics {
    /// Minimum latency (microseconds)
    pub min_us: u64,
    /// Maximum latency (microseconds)
    pub max_us: u64,
    /// Average latency (microseconds)
    pub avg_us: u64,
    /// 50th percentile (microseconds)
    pub p50_us: u64,
    /// 95th percentile (microseconds)
    pub p95_us: u64,
    /// 99th percentile (microseconds)
    pub p99_us: u64,
    /// Latency histogram buckets
    pub histogram: LatencyHistogram,
}

/// Latency histogram buckets
#[derive(Debug, Default, Clone)]
pub struct LatencyHistogram {
    /// < 1ms
    pub under_1ms: u64,
    /// 1-5ms
    pub ms_1_to_5: u64,
    /// 5-10ms
    pub ms_5_to_10: u64,
    /// 10-50ms
    pub ms_10_to_50: u64,
    /// 50-100ms
    pub ms_50_to_100: u64,
    /// > 100ms
    pub over_100ms: u64,
}

/// CPU utilization metrics
#[derive(Debug, Default, Clone)]
pub struct CpuMetrics {
    /// CPU cores in use
    pub cores_active: u32,
    /// Average CPU utilization (percent)
    pub avg_utilization: f64,
    /// Per-core utilization
    pub per_core: Vec<f64>,
    /// Interrupt count
    pub interrupts: u64,
    /// Context switches
    pub context_switches: u64,
}

/// Memory metrics
#[derive(Debug, Default, Clone)]
pub struct MemoryMetrics {
    /// UMEM pages allocated
    pub umem_pages: u64,
    /// UMEM pages in use
    pub umem_pages_used: u64,
    /// Ring buffer utilization (percent)
    pub ring_utilization: f64,
    /// Zero-copy operations
    pub zero_copy_ops: u64,
    /// Memory copy operations (fallback)
    pub memcpy_ops: u64,
}

/// eBPF metrics collector
pub struct EbpfMetricsCollector {
    /// Current metrics
    current: Arc<RwLock<EbpfMetrics>>,
    /// Previous metrics for rate calculations
    previous: Arc<RwLock<EbpfMetrics>>,
    /// Last collection time
    last_collection: Arc<RwLock<Instant>>,
    /// Collection interval
    interval: Duration,
    /// Atomic counters for lock-free updates
    packet_counter: Arc<AtomicU64>,
    byte_counter: Arc<AtomicU64>,
}

impl EbpfMetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Result<Self> {
        Ok(Self {
            current: Arc::new(RwLock::new(EbpfMetrics::default())),
            previous: Arc::new(RwLock::new(EbpfMetrics::default())),
            last_collection: Arc::new(RwLock::new(Instant::now())),
            interval: Duration::from_secs(1),
            packet_counter: Arc::new(AtomicU64::new(0)),
            byte_counter: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Collect current metrics from eBPF maps
    pub fn collect(&self) -> EbpfMetrics {
        let now = Instant::now();
        let mut metrics = self.current.write();

        // Calculate rates
        let elapsed = now.duration_since(*self.last_collection.read()).as_secs_f64();
        if elapsed > 0.0 {
            let packets = self.packet_counter.load(Ordering::Relaxed);
            let bytes = self.byte_counter.load(Ordering::Relaxed);

            let prev = self.previous.read();
            let packet_diff = packets.saturating_sub(prev.packet_metrics.total_packets);
            let byte_diff = bytes.saturating_sub(
                (prev.packet_metrics.bytes_per_second * elapsed) as u64
            );

            metrics.packet_metrics.packets_per_second = packet_diff as f64 / elapsed;
            metrics.packet_metrics.bytes_per_second = byte_diff as f64 / elapsed;
        }

        // Update previous metrics
        *self.previous.write() = metrics.clone();
        *self.last_collection.write() = now;

        metrics.clone()
    }

    /// Update packet counter (called from XDP program via map)
    pub fn update_packet_count(&self, count: u64) {
        self.packet_counter.store(count, Ordering::Relaxed);
    }

    /// Update byte counter
    pub fn update_byte_count(&self, bytes: u64) {
        self.byte_counter.store(bytes, Ordering::Relaxed);
    }

    /// Update latency measurement
    pub fn record_latency(&self, latency_us: u64) {
        let mut metrics = self.current.write();
        let latency = &mut metrics.latency_metrics;

        // Update min/max
        if latency.min_us == 0 || latency_us < latency.min_us {
            latency.min_us = latency_us;
        }
        if latency_us > latency.max_us {
            latency.max_us = latency_us;
        }

        // Update histogram
        match latency_us {
            0..=999 => latency.histogram.under_1ms += 1,
            1000..=4999 => latency.histogram.ms_1_to_5 += 1,
            5000..=9999 => latency.histogram.ms_5_to_10 += 1,
            10000..=49999 => latency.histogram.ms_10_to_50 += 1,
            50000..=99999 => latency.histogram.ms_50_to_100 += 1,
            _ => latency.histogram.over_100ms += 1,
        }
    }

    /// Update packet size distribution
    pub fn record_packet_size(&self, size: usize) {
        let mut metrics = self.current.write();
        let dist = &mut metrics.packet_metrics.size_distribution;

        match size {
            0..=63 => dist.tiny += 1,
            64..=255 => dist.small += 1,
            256..=1023 => dist.medium += 1,
            1024..=1500 => dist.large += 1,
            _ => dist.jumbo += 1,
        }

        metrics.packet_metrics.total_packets += 1;
    }

    /// Update connection metrics
    pub fn update_connection_count(&self, active: u64, new_rate: f64) {
        let mut metrics = self.current.write();
        metrics.connection_metrics.active_connections = active;
        metrics.connection_metrics.new_connections_per_sec = new_rate;
    }

    /// Update CPU metrics
    pub fn update_cpu_metrics(&self, cores: u32, utilization: f64, per_core: Vec<f64>) {
        let mut metrics = self.current.write();
        metrics.cpu_metrics.cores_active = cores;
        metrics.cpu_metrics.avg_utilization = utilization;
        metrics.cpu_metrics.per_core = per_core;
    }

    /// Update memory metrics
    pub fn update_memory_metrics(&self, umem_pages: u64, umem_used: u64, ring_util: f64) {
        let mut metrics = self.current.write();
        metrics.memory_metrics.umem_pages = umem_pages;
        metrics.memory_metrics.umem_pages_used = umem_used;
        metrics.memory_metrics.ring_utilization = ring_util;
    }

    /// Record zero-copy operation
    pub fn record_zero_copy(&self) {
        self.current.write().memory_metrics.zero_copy_ops += 1;
    }

    /// Record memory copy operation (fallback)
    pub fn record_memcpy(&self) {
        self.current.write().memory_metrics.memcpy_ops += 1;
    }

    /// Get throughput in Gbps
    pub fn get_throughput_gbps(&self) -> f64 {
        let metrics = self.current.read();
        metrics.packet_metrics.bytes_per_second * 8.0 / 1_000_000_000.0
    }

    /// Get packet rate in millions of packets per second
    pub fn get_packet_rate_mpps(&self) -> f64 {
        let metrics = self.current.read();
        metrics.packet_metrics.packets_per_second / 1_000_000.0
    }

    /// Calculate percentiles from histogram
    pub fn calculate_latency_percentiles(&self) {
        let mut metrics = self.current.write();
        let histogram = &metrics.latency_metrics.histogram;

        let total = histogram.under_1ms
            + histogram.ms_1_to_5
            + histogram.ms_5_to_10
            + histogram.ms_10_to_50
            + histogram.ms_50_to_100
            + histogram.over_100ms;

        if total == 0 {
            return;
        }

        // Calculate percentiles based on histogram buckets
        let p50_target = total / 2;
        let p95_target = (total * 95) / 100;
        let p99_target = (total * 99) / 100;

        let mut cumulative = 0;
        let buckets = [
            (histogram.under_1ms, 500),       // 0.5ms average for < 1ms
            (histogram.ms_1_to_5, 3000),      // 3ms average for 1-5ms
            (histogram.ms_5_to_10, 7500),     // 7.5ms average for 5-10ms
            (histogram.ms_10_to_50, 30000),   // 30ms average for 10-50ms
            (histogram.ms_50_to_100, 75000),  // 75ms average for 50-100ms
            (histogram.over_100ms, 150000),   // 150ms average for > 100ms
        ];

        for (count, avg_us) in buckets {
            cumulative += count;
            if cumulative >= p50_target && metrics.latency_metrics.p50_us == 0 {
                metrics.latency_metrics.p50_us = avg_us;
            }
            if cumulative >= p95_target && metrics.latency_metrics.p95_us == 0 {
                metrics.latency_metrics.p95_us = avg_us;
            }
            if cumulative >= p99_target && metrics.latency_metrics.p99_us == 0 {
                metrics.latency_metrics.p99_us = avg_us;
                break;
            }
        }

        // Calculate average
        if total > 0 {
            let weighted_sum: u64 = buckets
                .iter()
                .map(|(count, avg_us)| count * avg_us)
                .sum();
            metrics.latency_metrics.avg_us = weighted_sum / total;
        }
    }
}

/// Format metrics for logging
impl std::fmt::Display for EbpfMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "eBPF Metrics:\n")?;
        write!(f, "  Packets: {} total, {:.2} pps\n",
            self.packet_metrics.total_packets,
            self.packet_metrics.packets_per_second)?;
        write!(f, "  Throughput: {:.2} Gbps\n",
            self.packet_metrics.bytes_per_second * 8.0 / 1_000_000_000.0)?;
        write!(f, "  Connections: {} active, {:.2} new/sec\n",
            self.connection_metrics.active_connections,
            self.connection_metrics.new_connections_per_sec)?;
        write!(f, "  Latency: min={} µs, avg={} µs, p99={} µs\n",
            self.latency_metrics.min_us,
            self.latency_metrics.avg_us,
            self.latency_metrics.p99_us)?;
        write!(f, "  CPU: {:.1}% avg, {} cores\n",
            self.cpu_metrics.avg_utilization,
            self.cpu_metrics.cores_active)?;
        write!(f, "  Memory: {} zero-copy, {} memcpy ops\n",
            self.memory_metrics.zero_copy_ops,
            self.memory_metrics.memcpy_ops)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = EbpfMetricsCollector::new();
        assert!(collector.is_ok());
    }

    #[test]
    fn test_latency_recording() {
        let collector = EbpfMetricsCollector::new().unwrap();

        collector.record_latency(500);    // 0.5ms
        collector.record_latency(1500);   // 1.5ms
        collector.record_latency(50000);  // 50ms

        let metrics = collector.collect();
        assert_eq!(metrics.latency_metrics.min_us, 500);
        assert_eq!(metrics.latency_metrics.max_us, 50000);
        assert_eq!(metrics.latency_metrics.histogram.under_1ms, 1);
        assert_eq!(metrics.latency_metrics.histogram.ms_1_to_5, 1);
        assert_eq!(metrics.latency_metrics.histogram.ms_50_to_100, 1);
    }

    #[test]
    fn test_packet_size_distribution() {
        let collector = EbpfMetricsCollector::new().unwrap();

        collector.record_packet_size(32);    // tiny
        collector.record_packet_size(128);   // small
        collector.record_packet_size(512);   // medium
        collector.record_packet_size(1400);  // large
        collector.record_packet_size(9000);  // jumbo

        let metrics = collector.collect();
        assert_eq!(metrics.packet_metrics.size_distribution.tiny, 1);
        assert_eq!(metrics.packet_metrics.size_distribution.small, 1);
        assert_eq!(metrics.packet_metrics.size_distribution.medium, 1);
        assert_eq!(metrics.packet_metrics.size_distribution.large, 1);
        assert_eq!(metrics.packet_metrics.size_distribution.jumbo, 1);
        assert_eq!(metrics.packet_metrics.total_packets, 5);
    }
}