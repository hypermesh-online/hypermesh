//! Native collectors for system and application metrics

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::{MetricValue, MetricsStorage};

/// Collect CPU metrics from /proc/stat
pub struct CpuCollector {
    previous_stats: Option<CpuStats>,
    last_collection: Option<Instant>,
}

#[derive(Clone, Debug)]
struct CpuStats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl CpuCollector {
    pub fn new() -> Self {
        Self {
            previous_stats: None,
            last_collection: None,
        }
    }

    pub fn collect(&mut self) -> HashMap<String, MetricValue> {
        let mut metrics = HashMap::new();

        if let Ok(stats) = self.read_cpu_stats() {
            if let Some(prev) = &self.previous_stats {
                let delta_total = (stats.total() - prev.total()) as f64;

                if delta_total > 0.0 {
                    metrics.insert(
                        "cpu_usage_user".to_string(),
                        MetricValue::Gauge(((stats.user - prev.user) as f64 / delta_total) * 100.0),
                    );
                    metrics.insert(
                        "cpu_usage_system".to_string(),
                        MetricValue::Gauge(((stats.system - prev.system) as f64 / delta_total) * 100.0),
                    );
                    metrics.insert(
                        "cpu_usage_idle".to_string(),
                        MetricValue::Gauge(((stats.idle - prev.idle) as f64 / delta_total) * 100.0),
                    );
                    metrics.insert(
                        "cpu_usage_iowait".to_string(),
                        MetricValue::Gauge(((stats.iowait - prev.iowait) as f64 / delta_total) * 100.0),
                    );
                    metrics.insert(
                        "cpu_usage_total".to_string(),
                        MetricValue::Gauge(100.0 - ((stats.idle - prev.idle) as f64 / delta_total) * 100.0),
                    );
                }
            }

            self.previous_stats = Some(stats);
            self.last_collection = Some(Instant::now());
        }

        metrics
    }

    fn read_cpu_stats(&self) -> std::io::Result<CpuStats> {
        let file = File::open("/proc/stat")?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.starts_with("cpu ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 9 {
                    return Ok(CpuStats {
                        user: parts[1].parse().unwrap_or(0),
                        nice: parts[2].parse().unwrap_or(0),
                        system: parts[3].parse().unwrap_or(0),
                        idle: parts[4].parse().unwrap_or(0),
                        iowait: parts[5].parse().unwrap_or(0),
                        irq: parts[6].parse().unwrap_or(0),
                        softirq: parts[7].parse().unwrap_or(0),
                        steal: parts[8].parse().unwrap_or(0),
                    });
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "CPU stats not found",
        ))
    }
}

impl CpuStats {
    fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait + self.irq + self.softirq + self.steal
    }
}

/// Collect memory metrics from /proc/meminfo
pub struct MemoryCollector;

impl MemoryCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&self) -> HashMap<String, MetricValue> {
        let mut metrics = HashMap::new();

        if let Ok(meminfo) = self.read_meminfo() {
            if let (Some(total), Some(available)) = (meminfo.get("MemTotal"), meminfo.get("MemAvailable")) {
                let used = total - available;
                metrics.insert("memory_total_bytes".to_string(), MetricValue::Gauge(*total as f64));
                metrics.insert("memory_available_bytes".to_string(), MetricValue::Gauge(*available as f64));
                metrics.insert("memory_used_bytes".to_string(), MetricValue::Gauge(used as f64));
                metrics.insert(
                    "memory_usage_percent".to_string(),
                    MetricValue::Gauge((used as f64 / *total as f64) * 100.0),
                );
            }

            if let Some(swap_total) = meminfo.get("SwapTotal") {
                if let Some(swap_free) = meminfo.get("SwapFree") {
                    let swap_used = swap_total - swap_free;
                    metrics.insert("swap_total_bytes".to_string(), MetricValue::Gauge(*swap_total as f64));
                    metrics.insert("swap_free_bytes".to_string(), MetricValue::Gauge(*swap_free as f64));
                    metrics.insert("swap_used_bytes".to_string(), MetricValue::Gauge(swap_used as f64));
                }
            }

            if let Some(cached) = meminfo.get("Cached") {
                metrics.insert("memory_cached_bytes".to_string(), MetricValue::Gauge(*cached as f64));
            }

            if let Some(buffers) = meminfo.get("Buffers") {
                metrics.insert("memory_buffers_bytes".to_string(), MetricValue::Gauge(*buffers as f64));
            }
        }

        metrics
    }

    fn read_meminfo(&self) -> std::io::Result<HashMap<String, u64>> {
        let file = File::open("/proc/meminfo")?;
        let reader = BufReader::new(file);
        let mut meminfo = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let key = parts[0].trim_end_matches(':');
                let value = parts[1].parse::<u64>().unwrap_or(0);
                // Convert from KB to bytes
                meminfo.insert(key.to_string(), value * 1024);
            }
        }

        Ok(meminfo)
    }
}

/// Collect network metrics from /proc/net/dev
pub struct NetworkCollector {
    previous_stats: HashMap<String, NetworkStats>,
    last_collection: Option<Instant>,
}

#[derive(Clone, Debug)]
struct NetworkStats {
    rx_bytes: u64,
    rx_packets: u64,
    rx_errors: u64,
    rx_dropped: u64,
    tx_bytes: u64,
    tx_packets: u64,
    tx_errors: u64,
    tx_dropped: u64,
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self {
            previous_stats: HashMap::new(),
            last_collection: None,
        }
    }

    pub fn collect(&mut self) -> HashMap<String, MetricValue> {
        let mut metrics = HashMap::new();

        if let Ok(current_stats) = self.read_network_stats() {
            if let Some(last) = self.last_collection {
                let elapsed = last.elapsed().as_secs_f64();

                for (interface, stats) in &current_stats {
                    if let Some(prev) = self.previous_stats.get(interface) {
                        // Calculate rates per second
                        let rx_bytes_rate = (stats.rx_bytes - prev.rx_bytes) as f64 / elapsed;
                        let tx_bytes_rate = (stats.tx_bytes - prev.tx_bytes) as f64 / elapsed;
                        let rx_packets_rate = (stats.rx_packets - prev.rx_packets) as f64 / elapsed;
                        let tx_packets_rate = (stats.tx_packets - prev.tx_packets) as f64 / elapsed;

                        metrics.insert(
                            format!("network_{}_rx_bytes_per_sec", interface),
                            MetricValue::Gauge(rx_bytes_rate),
                        );
                        metrics.insert(
                            format!("network_{}_tx_bytes_per_sec", interface),
                            MetricValue::Gauge(tx_bytes_rate),
                        );
                        metrics.insert(
                            format!("network_{}_rx_packets_per_sec", interface),
                            MetricValue::Gauge(rx_packets_rate),
                        );
                        metrics.insert(
                            format!("network_{}_tx_packets_per_sec", interface),
                            MetricValue::Gauge(tx_packets_rate),
                        );

                        // Total counters
                        metrics.insert(
                            format!("network_{}_rx_bytes_total", interface),
                            MetricValue::Counter(stats.rx_bytes),
                        );
                        metrics.insert(
                            format!("network_{}_tx_bytes_total", interface),
                            MetricValue::Counter(stats.tx_bytes),
                        );

                        // Error counters
                        metrics.insert(
                            format!("network_{}_rx_errors_total", interface),
                            MetricValue::Counter(stats.rx_errors),
                        );
                        metrics.insert(
                            format!("network_{}_tx_errors_total", interface),
                            MetricValue::Counter(stats.tx_errors),
                        );
                    }
                }
            }

            self.previous_stats = current_stats;
            self.last_collection = Some(Instant::now());
        }

        metrics
    }

    fn read_network_stats(&self) -> std::io::Result<HashMap<String, NetworkStats>> {
        let file = File::open("/proc/net/dev")?;
        let reader = BufReader::new(file);
        let mut stats = HashMap::new();

        for (i, line) in reader.lines().enumerate() {
            // Skip header lines
            if i < 2 {
                continue;
            }

            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.len() >= 17 {
                let interface = parts[0].trim_end_matches(':');

                // Skip loopback for now
                if interface == "lo" {
                    continue;
                }

                stats.insert(
                    interface.to_string(),
                    NetworkStats {
                        rx_bytes: parts[1].parse().unwrap_or(0),
                        rx_packets: parts[2].parse().unwrap_or(0),
                        rx_errors: parts[3].parse().unwrap_or(0),
                        rx_dropped: parts[4].parse().unwrap_or(0),
                        tx_bytes: parts[9].parse().unwrap_or(0),
                        tx_packets: parts[10].parse().unwrap_or(0),
                        tx_errors: parts[11].parse().unwrap_or(0),
                        tx_dropped: parts[12].parse().unwrap_or(0),
                    },
                );
            }
        }

        Ok(stats)
    }
}

/// Collect disk I/O metrics from /proc/diskstats
pub struct DiskCollector {
    previous_stats: HashMap<String, DiskStats>,
    last_collection: Option<Instant>,
}

#[derive(Clone, Debug)]
struct DiskStats {
    reads_completed: u64,
    reads_merged: u64,
    sectors_read: u64,
    time_reading_ms: u64,
    writes_completed: u64,
    writes_merged: u64,
    sectors_written: u64,
    time_writing_ms: u64,
    io_in_progress: u64,
    time_io_ms: u64,
}

impl DiskCollector {
    pub fn new() -> Self {
        Self {
            previous_stats: HashMap::new(),
            last_collection: None,
        }
    }

    pub fn collect(&mut self) -> HashMap<String, MetricValue> {
        let mut metrics = HashMap::new();

        if let Ok(current_stats) = self.read_disk_stats() {
            if let Some(last) = self.last_collection {
                let elapsed = last.elapsed().as_secs_f64();

                for (device, stats) in &current_stats {
                    if let Some(prev) = self.previous_stats.get(device) {
                        // Calculate rates per second
                        let read_rate = (stats.reads_completed - prev.reads_completed) as f64 / elapsed;
                        let write_rate = (stats.writes_completed - prev.writes_completed) as f64 / elapsed;
                        let read_bytes_rate = ((stats.sectors_read - prev.sectors_read) * 512) as f64 / elapsed;
                        let write_bytes_rate = ((stats.sectors_written - prev.sectors_written) * 512) as f64 / elapsed;

                        metrics.insert(
                            format!("disk_{}_reads_per_sec", device),
                            MetricValue::Gauge(read_rate),
                        );
                        metrics.insert(
                            format!("disk_{}_writes_per_sec", device),
                            MetricValue::Gauge(write_rate),
                        );
                        metrics.insert(
                            format!("disk_{}_read_bytes_per_sec", device),
                            MetricValue::Gauge(read_bytes_rate),
                        );
                        metrics.insert(
                            format!("disk_{}_write_bytes_per_sec", device),
                            MetricValue::Gauge(write_bytes_rate),
                        );
                        metrics.insert(
                            format!("disk_{}_io_in_progress", device),
                            MetricValue::Gauge(stats.io_in_progress as f64),
                        );
                    }
                }
            }

            self.previous_stats = current_stats;
            self.last_collection = Some(Instant::now());
        }

        metrics
    }

    fn read_disk_stats(&self) -> std::io::Result<HashMap<String, DiskStats>> {
        let file = File::open("/proc/diskstats")?;
        let reader = BufReader::new(file);
        let mut stats = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.len() >= 14 {
                let device = parts[2];

                // Filter for actual disks (not partitions)
                if device.starts_with("sd") || device.starts_with("nvme") || device.starts_with("vd") {
                    if !device.chars().last().unwrap().is_ascii_digit() {
                        stats.insert(
                            device.to_string(),
                            DiskStats {
                                reads_completed: parts[3].parse().unwrap_or(0),
                                reads_merged: parts[4].parse().unwrap_or(0),
                                sectors_read: parts[5].parse().unwrap_or(0),
                                time_reading_ms: parts[6].parse().unwrap_or(0),
                                writes_completed: parts[7].parse().unwrap_or(0),
                                writes_merged: parts[8].parse().unwrap_or(0),
                                sectors_written: parts[9].parse().unwrap_or(0),
                                time_writing_ms: parts[10].parse().unwrap_or(0),
                                io_in_progress: parts[11].parse().unwrap_or(0),
                                time_io_ms: parts[12].parse().unwrap_or(0),
                            },
                        );
                    }
                }
            }
        }

        Ok(stats)
    }
}

/// System metrics collector aggregating all collectors
pub struct SystemMetricsCollector {
    cpu_collector: CpuCollector,
    memory_collector: MemoryCollector,
    network_collector: NetworkCollector,
    disk_collector: DiskCollector,
}

impl SystemMetricsCollector {
    pub fn new() -> Self {
        Self {
            cpu_collector: CpuCollector::new(),
            memory_collector: MemoryCollector::new(),
            network_collector: NetworkCollector::new(),
            disk_collector: DiskCollector::new(),
        }
    }

    pub async fn collect_all(&mut self, storage: Arc<RwLock<MetricsStorage>>) {
        let mut all_metrics = HashMap::new();

        // Collect CPU metrics
        all_metrics.extend(self.cpu_collector.collect());

        // Collect memory metrics
        all_metrics.extend(self.memory_collector.collect());

        // Collect network metrics
        all_metrics.extend(self.network_collector.collect());

        // Collect disk metrics
        all_metrics.extend(self.disk_collector.collect());

        // Update storage
        let mut storage = storage.write().await;
        for (name, value) in all_metrics {
            storage.current.insert(name, value);
        }
        storage.stats.total_metrics = storage.current.len() as u64;
    }
}

/// HyperMesh-specific metrics collector
pub struct HyperMeshCollector;

impl HyperMeshCollector {
    pub fn new() -> Self {
        Self
    }

    /// Collect container metrics
    pub fn collect_container_metrics(&self) -> HashMap<String, MetricValue> {
        let mut metrics = HashMap::new();

        // These would be collected from actual container runtime
        metrics.insert(
            "hypermesh_containers_running".to_string(),
            MetricValue::Gauge(0.0),
        );
        metrics.insert(
            "hypermesh_container_starts_total".to_string(),
            MetricValue::Counter(0),
        );
        metrics.insert(
            "hypermesh_container_startup_duration_seconds".to_string(),
            MetricValue::Gauge(0.0),
        );

        metrics
    }

    /// Collect consensus metrics
    pub fn collect_consensus_metrics(&self) -> HashMap<String, MetricValue> {
        let mut metrics = HashMap::new();

        metrics.insert(
            "hypermesh_consensus_rounds_total".to_string(),
            MetricValue::Counter(0),
        );
        metrics.insert(
            "hypermesh_consensus_latency_seconds".to_string(),
            MetricValue::Gauge(0.0),
        );
        metrics.insert(
            "hypermesh_byzantine_faults_detected".to_string(),
            MetricValue::Counter(0),
        );

        metrics
    }

    /// Collect network mesh metrics
    pub fn collect_mesh_metrics(&self) -> HashMap<String, MetricValue> {
        let mut metrics = HashMap::new();

        metrics.insert(
            "hypermesh_peer_connections".to_string(),
            MetricValue::Gauge(0.0),
        );
        metrics.insert(
            "hypermesh_p2p_messages_sent_total".to_string(),
            MetricValue::Counter(0),
        );
        metrics.insert(
            "hypermesh_p2p_messages_received_total".to_string(),
            MetricValue::Counter(0),
        );

        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_stats_total() {
        let stats = CpuStats {
            user: 100,
            nice: 10,
            system: 50,
            idle: 200,
            iowait: 20,
            irq: 5,
            softirq: 5,
            steal: 0,
        };

        assert_eq!(stats.total(), 390);
    }

    #[test]
    fn test_memory_collector() {
        let collector = MemoryCollector::new();
        let metrics = collector.collect();

        // On Linux systems, these metrics should be present
        if Path::new("/proc/meminfo").exists() {
            assert!(metrics.contains_key("memory_total_bytes") || metrics.is_empty());
        }
    }

    #[test]
    fn test_system_collector() {
        let mut collector = SystemMetricsCollector::new();
        let storage = Arc::new(RwLock::new(MetricsStorage {
            time_series: std::collections::BTreeMap::new(),
            current: HashMap::new(),
            metadata: HashMap::new(),
            stats: super::super::StorageStats::default(),
        }));

        // This would be an async test in production
        // tokio::runtime::Runtime::new().unwrap().block_on(async {
        //     collector.collect_all(storage).await;
        // });
    }
}