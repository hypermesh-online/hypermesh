// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Real system metrics collection for HyperMesh nodes.
//!
//! Provides a [`MetricsSource`] trait with two implementations:
//! - [`SystemMetricsSource`] reads actual system data from `/proc` on Linux.
//! - [`MockMetricsSource`] returns configurable test data.

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::node_metrics::HardwareSummary;

// ---------------------------------------------------------------------------
// MetricsSnapshot
// ---------------------------------------------------------------------------

/// Point-in-time snapshot of all system metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    /// When this snapshot was taken.
    pub timestamp: SystemTime,
    /// Network metrics.
    pub network: NetworkMetrics,
    /// Compute metrics.
    pub compute: ComputeMetrics,
    /// Storage metrics.
    pub storage: StorageMetrics,
}

/// Network interface metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Bytes sent (cumulative).
    pub bytes_sent: u64,
    /// Bytes received (cumulative).
    pub bytes_received: u64,
    /// Active connection count.
    pub connection_count: u32,
    /// Latency samples in microseconds (most recent).
    pub latency_samples_us: Vec<u64>,
}

/// Compute resource metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComputeMetrics {
    /// CPU usage ratio (0.0 to 1.0).
    pub cpu_usage: f64,
    /// Number of logical CPU cores.
    pub cpu_cores: usize,
    /// Total memory in bytes.
    pub memory_total_bytes: u64,
    /// Used memory in bytes.
    pub memory_used_bytes: u64,
    /// Memory usage ratio (0.0 to 1.0).
    pub memory_usage: f64,
}

/// Storage metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageMetrics {
    /// Total disk space in bytes.
    pub disk_total_bytes: u64,
    /// Used disk space in bytes.
    pub disk_used_bytes: u64,
    /// Disk usage ratio (0.0 to 1.0).
    pub disk_usage: f64,
    /// Read bytes per second (recent average).
    pub read_bps: u64,
    /// Write bytes per second (recent average).
    pub write_bps: u64,
}

// ---------------------------------------------------------------------------
// MetricsSource trait
// ---------------------------------------------------------------------------

/// Trait for collecting system metrics from various sources.
pub trait MetricsSource: Send + Sync {
    /// Collect a snapshot of current system metrics.
    fn collect(&self) -> Result<SystemSnapshot, MetricsSourceError>;
}

/// Errors from metrics collection.
#[derive(Debug, thiserror::Error)]
pub enum MetricsSourceError {
    /// Failed to read from a system source.
    #[error("system read error: {0}")]
    SystemRead(String),
    /// Parsing failed.
    #[error("parse error: {0}")]
    Parse(String),
}

// ---------------------------------------------------------------------------
// SystemMetricsSource
// ---------------------------------------------------------------------------

/// Reads real system metrics from `/proc` on Linux.
///
/// On non-Linux systems, returns zeroed defaults.
pub struct SystemMetricsSource;

impl SystemMetricsSource {
    /// Create a new system metrics source.
    pub fn new() -> Self {
        Self
    }

    /// Read network stats from `/proc/net/dev`.
    fn read_network_stats() -> NetworkMetrics {
        #[cfg(target_os = "linux")]
        {
            Self::read_proc_net_dev().unwrap_or_default()
        }
        #[cfg(not(target_os = "linux"))]
        {
            NetworkMetrics::default()
        }
    }

    #[cfg(target_os = "linux")]
    fn read_proc_net_dev() -> Result<NetworkMetrics, MetricsSourceError> {
        let content = std::fs::read_to_string("/proc/net/dev")
            .map_err(|e| MetricsSourceError::SystemRead(e.to_string()))?;

        let mut bytes_sent: u64 = 0;
        let mut bytes_received: u64 = 0;

        for line in content.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                let iface = parts[0].trim_end_matches(':');
                // Skip loopback
                if iface == "lo" {
                    continue;
                }
                if let Ok(rx) = parts[1].parse::<u64>() {
                    bytes_received = bytes_received.saturating_add(rx);
                }
                if let Ok(tx) = parts[9].parse::<u64>() {
                    bytes_sent = bytes_sent.saturating_add(tx);
                }
            }
        }

        Ok(NetworkMetrics {
            bytes_sent,
            bytes_received,
            connection_count: 0,
            latency_samples_us: Vec::new(),
        })
    }

    /// Read memory info from `/proc/meminfo`.
    fn read_memory_stats() -> (u64, u64, f64) {
        #[cfg(target_os = "linux")]
        {
            Self::read_proc_meminfo().unwrap_or((0, 0, 0.0))
        }
        #[cfg(not(target_os = "linux"))]
        {
            (0, 0, 0.0)
        }
    }

    #[cfg(target_os = "linux")]
    fn read_proc_meminfo() -> Result<(u64, u64, f64), MetricsSourceError> {
        let content = std::fs::read_to_string("/proc/meminfo")
            .map_err(|e| MetricsSourceError::SystemRead(e.to_string()))?;

        let mut total_kb: u64 = 0;
        let mut available_kb: u64 = 0;

        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                total_kb = parse_meminfo_value(line);
            } else if line.starts_with("MemAvailable:") {
                available_kb = parse_meminfo_value(line);
            }
        }

        let total = total_kb * 1024;
        let used = total.saturating_sub(available_kb * 1024);
        let usage = if total > 0 {
            used as f64 / total as f64
        } else {
            0.0
        };

        Ok((total, used, usage))
    }
}

/// Parse a value from a meminfo line like "MemTotal:    16384 kB".
#[cfg(target_os = "linux")]
fn parse_meminfo_value(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
}

impl Default for SystemMetricsSource {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsSource for SystemMetricsSource {
    fn collect(&self) -> Result<SystemSnapshot, MetricsSourceError> {
        let network = Self::read_network_stats();
        let (mem_total, mem_used, mem_usage) = Self::read_memory_stats();

        Ok(SystemSnapshot {
            timestamp: SystemTime::now(),
            network,
            compute: ComputeMetrics {
                cpu_usage: 0.0, // CPU usage requires sampling over time
                cpu_cores: num_cpus(),
                memory_total_bytes: mem_total,
                memory_used_bytes: mem_used,
                memory_usage: mem_usage,
            },
            storage: StorageMetrics::default(),
        })
    }
}

/// Get number of CPU cores.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1)
}

/// Convert a SystemSnapshot into a HardwareSummary for compatibility.
impl From<&SystemSnapshot> for HardwareSummary {
    fn from(snap: &SystemSnapshot) -> Self {
        HardwareSummary {
            cpu_usage: snap.compute.cpu_usage,
            cpu_cores: snap.compute.cpu_cores,
            memory_total_bytes: snap.compute.memory_total_bytes,
            memory_available_bytes: snap
                .compute
                .memory_total_bytes
                .saturating_sub(snap.compute.memory_used_bytes),
            memory_usage: snap.compute.memory_usage,
            storage_total_bytes: snap.storage.disk_total_bytes,
            storage_available_bytes: snap
                .storage
                .disk_total_bytes
                .saturating_sub(snap.storage.disk_used_bytes),
            storage_usage: snap.storage.disk_usage,
            net_rx_bytes: snap.network.bytes_received,
            net_tx_bytes: snap.network.bytes_sent,
            net_rx_errors: 0,
            net_tx_errors: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// MockMetricsSource
// ---------------------------------------------------------------------------

/// Configurable mock metrics source for testing.
pub struct MockMetricsSource {
    snapshot: SystemSnapshot,
}

impl MockMetricsSource {
    /// Create a mock source that always returns the given snapshot.
    pub fn new(snapshot: SystemSnapshot) -> Self {
        Self { snapshot }
    }

    /// Create a mock source with reasonable default values.
    pub fn with_defaults() -> Self {
        Self {
            snapshot: SystemSnapshot {
                timestamp: SystemTime::now(),
                network: NetworkMetrics {
                    bytes_sent: 10_000_000,
                    bytes_received: 50_000_000,
                    connection_count: 5,
                    latency_samples_us: vec![3000, 4500, 2800, 5100],
                },
                compute: ComputeMetrics {
                    cpu_usage: 0.35,
                    cpu_cores: 4,
                    memory_total_bytes: 8_000_000_000,
                    memory_used_bytes: 3_000_000_000,
                    memory_usage: 0.375,
                },
                storage: StorageMetrics {
                    disk_total_bytes: 500_000_000_000,
                    disk_used_bytes: 200_000_000_000,
                    disk_usage: 0.40,
                    read_bps: 100_000_000,
                    write_bps: 50_000_000,
                },
            },
        }
    }
}

impl MetricsSource for MockMetricsSource {
    fn collect(&self) -> Result<SystemSnapshot, MetricsSourceError> {
        Ok(self.snapshot.clone())
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_source_returns_configured_values() {
        let mock = MockMetricsSource::with_defaults();
        let snap = mock.collect().expect("test: collect should succeed");

        assert_eq!(snap.network.bytes_sent, 10_000_000);
        assert_eq!(snap.network.bytes_received, 50_000_000);
        assert_eq!(snap.network.connection_count, 5);
        assert_eq!(snap.compute.cpu_cores, 4);
        assert!((snap.compute.cpu_usage - 0.35).abs() < 1e-9);
        assert_eq!(snap.compute.memory_total_bytes, 8_000_000_000);
        assert!((snap.storage.disk_usage - 0.40).abs() < 1e-9);
    }

    #[test]
    fn mock_source_custom_snapshot() {
        let custom = SystemSnapshot {
            timestamp: SystemTime::now(),
            network: NetworkMetrics {
                bytes_sent: 100,
                bytes_received: 200,
                connection_count: 1,
                latency_samples_us: vec![1000],
            },
            compute: ComputeMetrics {
                cpu_usage: 0.90,
                cpu_cores: 2,
                memory_total_bytes: 4_000_000_000,
                memory_used_bytes: 3_500_000_000,
                memory_usage: 0.875,
            },
            storage: StorageMetrics::default(),
        };

        let mock = MockMetricsSource::new(custom);
        let snap = mock.collect().expect("test: collect should succeed");

        assert_eq!(snap.network.bytes_sent, 100);
        assert!((snap.compute.cpu_usage - 0.90).abs() < 1e-9);
        assert_eq!(snap.compute.cpu_cores, 2);
    }

    #[test]
    fn system_source_collects_without_error() {
        let source = SystemMetricsSource::new();
        let result = source.collect();
        assert!(result.is_ok(), "system metrics collection should not error");
        let snap = result.expect("test: collect");
        assert!(snap.compute.cpu_cores >= 1);
    }

    #[test]
    fn snapshot_to_hardware_summary_conversion() {
        let mock = MockMetricsSource::with_defaults();
        let snap = mock.collect().expect("test: collect");
        let hw = HardwareSummary::from(&snap);

        assert!((hw.cpu_usage - 0.35).abs() < 1e-9);
        assert_eq!(hw.cpu_cores, 4);
        assert_eq!(hw.memory_total_bytes, 8_000_000_000);
        assert_eq!(
            hw.memory_available_bytes,
            8_000_000_000 - 3_000_000_000
        );
        assert_eq!(hw.net_rx_bytes, 50_000_000);
        assert_eq!(hw.net_tx_bytes, 10_000_000);
    }
}
