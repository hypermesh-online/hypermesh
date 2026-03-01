// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Internal `/proc` filesystem parsing helpers.
//!
//! All parsing is zero-allocation where possible and tolerant of
//! missing or malformed fields (returns 0 on parse failure).

use std::collections::HashMap;
use std::io;

use super::{InterfaceMetrics, MemoryMetrics, NetworkMetrics};

// ---------------------------------------------------------------------------
// /proc/stat
// ---------------------------------------------------------------------------

/// Parsed snapshot from `/proc/stat`.
pub(super) struct ProcStatSnapshot {
    pub user: u64,
    pub system: u64,
    pub idle: u64,
    pub total_all: u64,
    pub total_usage: f64,
    pub per_core_usage: Vec<f64>,
    /// (total_jiffies, idle_jiffies) per core for delta calculation.
    pub per_core_raw: Vec<(u64, u64)>,
}

pub(super) fn parse_proc_stat(content: &str) -> ProcStatSnapshot {
    let mut user = 0u64;
    let mut system = 0u64;
    let mut idle = 0u64;
    let mut total_all = 0u64;
    let mut per_core_usage = Vec::new();
    let mut per_core_raw = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        if parts[0] == "cpu" && parts.len() >= 5 {
            // Aggregate CPU line: cpu user nice system idle iowait irq softirq ...
            user = parts[1].parse().unwrap_or(0);
            let nice: u64 = parts[2].parse().unwrap_or(0);
            system = parts[3].parse().unwrap_or(0);
            idle = parts[4].parse().unwrap_or(0);

            total_all = parts[1..]
                .iter()
                .filter_map(|s| s.parse::<u64>().ok())
                .sum();
            user += nice; // include nice in user time
        } else if parts[0].starts_with("cpu") && parts.len() >= 5 {
            // Per-core line: cpu0 user nice system idle ...
            let core_idle: u64 = parts[4].parse().unwrap_or(0);
            let core_total: u64 = parts[1..]
                .iter()
                .filter_map(|s| s.parse::<u64>().ok())
                .sum();

            let usage = if core_total > 0 {
                1.0 - (core_idle as f64 / core_total as f64)
            } else {
                0.0
            };
            per_core_usage.push(usage.clamp(0.0, 1.0));
            per_core_raw.push((core_total, core_idle));
        }
    }

    let total_usage = if total_all > 0 {
        1.0 - (idle as f64 / total_all as f64)
    } else {
        0.0
    };

    ProcStatSnapshot {
        user,
        system,
        idle,
        total_all,
        total_usage: total_usage.clamp(0.0, 1.0),
        per_core_usage,
        per_core_raw,
    }
}

// ---------------------------------------------------------------------------
// /proc/meminfo
// ---------------------------------------------------------------------------

pub(super) fn parse_meminfo(content: &str) -> io::Result<MemoryMetrics> {
    let mut values: HashMap<String, u64> = HashMap::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let key = parts[0].trim_end_matches(':').to_string();
            if let Ok(val) = parts[1].parse::<u64>() {
                // Values in /proc/meminfo are in kB
                values.insert(key, val * 1024);
            }
        }
    }

    let total_bytes = values.get("MemTotal").copied().unwrap_or(0);
    let available_bytes = values
        .get("MemAvailable")
        .copied()
        .or_else(|| values.get("MemFree").copied())
        .unwrap_or(0);

    let used_bytes = total_bytes.saturating_sub(available_bytes);
    let usage_ratio = if total_bytes > 0 {
        used_bytes as f64 / total_bytes as f64
    } else {
        0.0
    };

    let swap_total = values.get("SwapTotal").copied().unwrap_or(0);
    let swap_free = values.get("SwapFree").copied().unwrap_or(0);

    Ok(MemoryMetrics {
        total_bytes,
        available_bytes,
        used_bytes,
        usage_ratio,
        swap_total_bytes: swap_total,
        swap_free_bytes: swap_free,
    })
}

// ---------------------------------------------------------------------------
// /proc/net/dev
// ---------------------------------------------------------------------------

pub(super) fn parse_net_dev(content: &str) -> NetworkMetrics {
    let mut interfaces = Vec::new();
    let mut total_rx_bytes = 0u64;
    let mut total_tx_bytes = 0u64;
    let mut total_rx_errors = 0u64;
    let mut total_tx_errors = 0u64;

    for line in content.lines().skip(2) {
        // Skip header lines
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Format: iface: rx_bytes rx_packets rx_errs rx_drop ... tx_bytes tx_packets ...
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 11 {
            continue;
        }

        let name = parts[0].trim_end_matches(':').to_string();
        // Skip loopback
        if name == "lo" {
            continue;
        }

        let iface = InterfaceMetrics {
            name,
            rx_bytes: parts[1].parse().unwrap_or(0),
            rx_packets: parts[2].parse().unwrap_or(0),
            rx_errors: parts[3].parse().unwrap_or(0),
            rx_drops: parts[4].parse().unwrap_or(0),
            tx_bytes: parts[9].parse().unwrap_or(0),
            tx_packets: parts[10].parse().unwrap_or(0),
            tx_errors: parts[11].parse().unwrap_or(0),
            tx_drops: if parts.len() > 12 {
                parts[12].parse().unwrap_or(0)
            } else {
                0
            },
        };

        total_rx_bytes += iface.rx_bytes;
        total_tx_bytes += iface.tx_bytes;
        total_rx_errors += iface.rx_errors;
        total_tx_errors += iface.tx_errors;
        interfaces.push(iface);
    }

    NetworkMetrics {
        interfaces,
        total_rx_bytes,
        total_tx_bytes,
        total_rx_errors,
        total_tx_errors,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_PROC_STAT: &str = "\
cpu  10000 500 3000 50000 200 100 50 0 0 0
cpu0 2500 125 750 12500 50 25 12 0 0 0
cpu1 2500 125 750 12500 50 25 13 0 0 0
cpu2 2500 125 750 12500 50 25 12 0 0 0
cpu3 2500 125 750 12500 50 25 13 0 0 0
intr 1234567
ctxt 9876543
";

    const SAMPLE_MEMINFO: &str = "\
MemTotal:       16384000 kB
MemFree:         2048000 kB
MemAvailable:    8192000 kB
Buffers:          512000 kB
Cached:          4096000 kB
SwapTotal:       4096000 kB
SwapFree:        4096000 kB
";

    const SAMPLE_NET_DEV: &str = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 1234567   12345    0    0    0     0          0         0  1234567   12345    0    0    0     0       0          0
  eth0: 9876543   98765   10    5    0     0          0       100  5432100   54321    3    1    0     0       0          0
wlan0:  1111111   11111    1    0    0     0          0         0   222222    2222    0    0    0     0       0          0
";

    #[test]
    fn parse_proc_stat_aggregate_values() {
        let snap = parse_proc_stat(SAMPLE_PROC_STAT);
        // user includes nice: 10000 + 500 = 10500
        assert_eq!(snap.user, 10500);
        assert_eq!(snap.system, 3000);
        assert_eq!(snap.idle, 50000);
        assert!(snap.total_all > 0);
        assert!(snap.total_usage >= 0.0);
        assert!(snap.total_usage <= 1.0);
    }

    #[test]
    fn parse_proc_stat_per_core() {
        let snap = parse_proc_stat(SAMPLE_PROC_STAT);
        assert_eq!(snap.per_core_usage.len(), 4);
        for usage in &snap.per_core_usage {
            assert!(*usage >= 0.0);
            assert!(*usage <= 1.0);
        }
    }

    #[test]
    fn parse_meminfo_values() {
        let mem = parse_meminfo(SAMPLE_MEMINFO).expect("test: parse meminfo");
        assert_eq!(mem.total_bytes, 16_384_000 * 1024);
        assert_eq!(mem.available_bytes, 8_192_000 * 1024);
        assert_eq!(mem.used_bytes, mem.total_bytes - mem.available_bytes);
        assert!(mem.usage_ratio > 0.0);
        assert!(mem.usage_ratio < 1.0);
        assert_eq!(mem.swap_total_bytes, 4_096_000 * 1024);
        assert_eq!(mem.swap_free_bytes, 4_096_000 * 1024);
    }

    #[test]
    fn parse_net_dev_skips_loopback() {
        let net = parse_net_dev(SAMPLE_NET_DEV);
        assert_eq!(net.interfaces.len(), 2, "should skip loopback");
        let names: Vec<&str> = net.interfaces.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"eth0"));
        assert!(names.contains(&"wlan0"));
        assert!(!names.contains(&"lo"));
    }

    #[test]
    fn parse_net_dev_totals() {
        let net = parse_net_dev(SAMPLE_NET_DEV);
        assert_eq!(net.total_rx_bytes, 9876543 + 1111111);
        assert_eq!(net.total_tx_bytes, 5432100 + 222222);
        assert_eq!(net.total_rx_errors, 10 + 1);
        assert_eq!(net.total_tx_errors, 3);
    }

    #[test]
    fn parse_net_dev_interface_detail() {
        let net = parse_net_dev(SAMPLE_NET_DEV);
        let eth0 = net
            .interfaces
            .iter()
            .find(|i| i.name == "eth0")
            .expect("test: eth0 should exist");
        assert_eq!(eth0.rx_bytes, 9876543);
        assert_eq!(eth0.rx_packets, 98765);
        assert_eq!(eth0.rx_errors, 10);
        assert_eq!(eth0.rx_drops, 5);
        assert_eq!(eth0.tx_bytes, 5432100);
        assert_eq!(eth0.tx_packets, 54321);
        assert_eq!(eth0.tx_errors, 3);
    }

    #[test]
    fn parse_empty_proc_stat() {
        let snap = parse_proc_stat("");
        assert_eq!(snap.user, 0);
        assert_eq!(snap.idle, 0);
        assert!(snap.per_core_usage.is_empty());
    }

    #[test]
    fn parse_empty_meminfo() {
        let mem = parse_meminfo("").expect("test: parse empty meminfo");
        assert_eq!(mem.total_bytes, 0);
        assert_eq!(mem.available_bytes, 0);
        assert_eq!(mem.usage_ratio, 0.0);
    }

    #[test]
    fn parse_empty_net_dev() {
        let net = parse_net_dev("");
        assert!(net.interfaces.is_empty());
        assert_eq!(net.total_rx_bytes, 0);
    }
}
