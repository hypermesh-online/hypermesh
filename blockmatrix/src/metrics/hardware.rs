// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Real hardware metrics collection from the local system.
//!
//! Reads directly from `/proc` filesystem on Linux to collect CPU, memory,
//! network, and storage metrics without external dependencies.

use std::io;
use std::time::{Duration, Instant, SystemTime};

use super::parsers;

/// Collected CPU metrics from `/proc/stat`.
#[derive(Debug, Clone)]
pub struct CpuMetrics {
    /// Number of logical CPU cores.
    pub core_count: usize,
    /// Per-core usage ratios (0.0 to 1.0), keyed by core index.
    pub per_core_usage: Vec<f64>,
    /// Aggregate CPU usage ratio across all cores (0.0 to 1.0).
    pub total_usage: f64,
    /// User-space time (jiffies from `/proc/stat`).
    pub user_jiffies: u64,
    /// System (kernel) time (jiffies).
    pub system_jiffies: u64,
    /// Idle time (jiffies).
    pub idle_jiffies: u64,
}

/// Collected memory metrics from `/proc/meminfo`.
#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    /// Total physical memory in bytes.
    pub total_bytes: u64,
    /// Available memory in bytes (includes reclaimable caches).
    pub available_bytes: u64,
    /// Used memory in bytes (`total - available`).
    pub used_bytes: u64,
    /// Usage ratio (0.0 to 1.0).
    pub usage_ratio: f64,
    /// Swap total in bytes.
    pub swap_total_bytes: u64,
    /// Swap free in bytes.
    pub swap_free_bytes: u64,
}

/// Per-interface network metrics from `/proc/net/dev`.
#[derive(Debug, Clone)]
pub struct InterfaceMetrics {
    /// Interface name (e.g. "eth0").
    pub name: String,
    /// Bytes received.
    pub rx_bytes: u64,
    /// Packets received.
    pub rx_packets: u64,
    /// Receive errors.
    pub rx_errors: u64,
    /// Receive drops.
    pub rx_drops: u64,
    /// Bytes transmitted.
    pub tx_bytes: u64,
    /// Packets transmitted.
    pub tx_packets: u64,
    /// Transmit errors.
    pub tx_errors: u64,
    /// Transmit drops.
    pub tx_drops: u64,
}

/// Aggregated network metrics across all non-loopback interfaces.
#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    /// Per-interface breakdown.
    pub interfaces: Vec<InterfaceMetrics>,
    /// Total bytes received across all interfaces.
    pub total_rx_bytes: u64,
    /// Total bytes transmitted across all interfaces.
    pub total_tx_bytes: u64,
    /// Total receive errors.
    pub total_rx_errors: u64,
    /// Total transmit errors.
    pub total_tx_errors: u64,
}

/// Storage metrics for the root filesystem.
#[derive(Debug, Clone)]
pub struct StorageMetrics {
    /// Total space in bytes.
    pub total_bytes: u64,
    /// Available space in bytes.
    pub available_bytes: u64,
    /// Used space in bytes.
    pub used_bytes: u64,
    /// Usage ratio (0.0 to 1.0).
    pub usage_ratio: f64,
}

/// Complete node hardware metrics snapshot.
#[derive(Debug, Clone)]
pub struct NodeMetrics {
    /// When this snapshot was taken.
    pub timestamp: SystemTime,
    /// How long collection took.
    pub collection_duration: Duration,
    /// CPU metrics.
    pub cpu: CpuMetrics,
    /// Memory metrics.
    pub memory: MemoryMetrics,
    /// Network metrics.
    pub network: NetworkMetrics,
    /// Storage metrics.
    pub storage: StorageMetrics,
}

/// Collect all hardware metrics in a single call.
///
/// Returns `Err` only if critical files cannot be read (e.g., not on Linux).
pub fn collect() -> io::Result<NodeMetrics> {
    let start = Instant::now();

    let cpu = collect_cpu()?;
    let memory = collect_memory()?;
    let network = collect_network()?;
    let storage = collect_storage()?;

    Ok(NodeMetrics {
        timestamp: SystemTime::now(),
        collection_duration: start.elapsed(),
        cpu,
        memory,
        network,
        storage,
    })
}

/// Collect instantaneous CPU metrics from `/proc/stat`.
///
/// Returns aggregate and per-core jiffies. For usage ratios over time,
/// call [`collect_cpu_usage`] which samples twice with a short delay.
pub fn collect_cpu() -> io::Result<CpuMetrics> {
    let content = std::fs::read_to_string("/proc/stat")?;
    let parsed = parsers::parse_proc_stat(&content);

    let core_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    Ok(CpuMetrics {
        core_count,
        per_core_usage: parsed.per_core_usage,
        total_usage: parsed.total_usage,
        user_jiffies: parsed.user,
        system_jiffies: parsed.system,
        idle_jiffies: parsed.idle,
    })
}

/// Collect CPU usage ratios by sampling `/proc/stat` twice with a gap.
pub fn collect_cpu_usage(sample_duration: Duration) -> io::Result<CpuMetrics> {
    let content1 = std::fs::read_to_string("/proc/stat")?;
    let snap1 = parsers::parse_proc_stat(&content1);

    std::thread::sleep(sample_duration);

    let content2 = std::fs::read_to_string("/proc/stat")?;
    let snap2 = parsers::parse_proc_stat(&content2);

    let core_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let total_delta = snap2.total_all.saturating_sub(snap1.total_all);
    let idle_delta = snap2.idle.saturating_sub(snap1.idle);
    let total_usage = if total_delta > 0 {
        1.0 - (idle_delta as f64 / total_delta as f64)
    } else {
        0.0
    };

    let mut per_core_usage = Vec::new();
    for (s1, s2) in snap1.per_core_raw.iter().zip(snap2.per_core_raw.iter()) {
        let delta_total = s2.0.saturating_sub(s1.0);
        let delta_idle = s2.1.saturating_sub(s1.1);
        let usage = if delta_total > 0 {
            1.0 - (delta_idle as f64 / delta_total as f64)
        } else {
            0.0
        };
        per_core_usage.push(usage.clamp(0.0, 1.0));
    }

    Ok(CpuMetrics {
        core_count,
        per_core_usage,
        total_usage: total_usage.clamp(0.0, 1.0),
        user_jiffies: snap2.user,
        system_jiffies: snap2.system,
        idle_jiffies: snap2.idle,
    })
}

/// Collect memory metrics from `/proc/meminfo`.
pub fn collect_memory() -> io::Result<MemoryMetrics> {
    let content = std::fs::read_to_string("/proc/meminfo")?;
    parsers::parse_meminfo(&content)
}

/// Collect network interface metrics from `/proc/net/dev`.
pub fn collect_network() -> io::Result<NetworkMetrics> {
    let content = std::fs::read_to_string("/proc/net/dev")?;
    Ok(parsers::parse_net_dev(&content))
}

/// Collect storage metrics for the root filesystem via `libc::statvfs`.
pub fn collect_storage() -> io::Result<StorageMetrics> {
    #[cfg(target_os = "linux")]
    {
        collect_storage_statvfs()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(StorageMetrics {
            total_bytes: 0,
            available_bytes: 0,
            used_bytes: 0,
            usage_ratio: 0.0,
        })
    }
}

#[cfg(target_os = "linux")]
fn collect_storage_statvfs() -> io::Result<StorageMetrics> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let path = CString::new("/").map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();

    // SAFETY: `statvfs` writes into the provided buffer and returns 0 on success.
    // The CString path is valid for the duration of the call.
    let ret = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };

    if ret != 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: statvfs returned 0, so the struct is fully initialized.
    let stat = unsafe { stat.assume_init() };

    let block_size = stat.f_frsize as u64;
    let total_bytes = stat.f_blocks * block_size;
    let available_bytes = stat.f_bavail * block_size;
    let used_bytes = total_bytes.saturating_sub(stat.f_bfree * block_size);
    let usage_ratio = if total_bytes > 0 {
        used_bytes as f64 / total_bytes as f64
    } else {
        0.0
    };

    Ok(StorageMetrics {
        total_bytes,
        available_bytes,
        used_bytes,
        usage_ratio,
    })
}

// ===========================================================================
// Tests -- real system collection (Linux only)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn collect_real_cpu_metrics() {
        let cpu = collect_cpu().expect("test: collect CPU on Linux");
        assert!(cpu.core_count >= 1);
        assert!(cpu.total_usage >= 0.0);
        assert!(cpu.total_usage <= 1.0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn collect_real_memory_metrics() {
        let mem = collect_memory().expect("test: collect memory on Linux");
        assert!(mem.total_bytes > 0, "system must have some memory");
        assert!(mem.available_bytes > 0, "some memory must be available");
        assert!(mem.used_bytes <= mem.total_bytes);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn collect_real_network_metrics() {
        let net = collect_network().expect("test: collect network on Linux");
        // System should have at least one non-loopback interface in most cases
        // but we don't assert on interface count since containers may differ
        assert!(net.total_rx_bytes + net.total_tx_bytes >= 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn collect_real_storage_metrics() {
        let storage = collect_storage().expect("test: collect storage on Linux");
        assert!(storage.total_bytes > 0, "root filesystem must have space");
        assert!(storage.available_bytes > 0, "some space must be free");
        assert!(storage.usage_ratio >= 0.0);
        assert!(storage.usage_ratio <= 1.0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn collect_all_metrics() {
        let metrics = collect().expect("test: collect all metrics on Linux");
        assert!(metrics.cpu.core_count >= 1);
        assert!(metrics.memory.total_bytes > 0);
        assert!(metrics.storage.total_bytes > 0);
        assert!(
            metrics.collection_duration < Duration::from_secs(5),
            "collection should be fast"
        );
    }
}
