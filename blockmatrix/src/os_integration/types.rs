// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// OS Integration Types - Unified data structures for cross-platform hardware and eBPF metrics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CPU Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    /// Number of logical CPU cores
    pub cores: usize,

    /// CPU model name (e.g., "Intel Core i7-9700K", "AMD Ryzen 9 5950X")
    pub model: String,

    /// CPU architecture (e.g., "x86_64", "aarch64", "arm")
    pub architecture: String,

    /// Base frequency in MHz (if available)
    pub frequency_mhz: Option<u64>,

    /// Current CPU usage percentage (0-100)
    pub usage_percent: Option<f64>,

    /// Vendor (e.g., "GenuineIntel", "AuthenticAMD", "Apple")
    pub vendor: Option<String>,

    /// Cache sizes in KB (L1, L2, L3)
    pub cache_kb: Option<CacheInfo>,
}

/// CPU Cache Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub l1_kb: Option<u64>,
    pub l2_kb: Option<u64>,
    pub l3_kb: Option<u64>,
}

/// GPU Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// GPU model name (e.g., "NVIDIA GeForce RTX 3080", "AMD Radeon RX 6800 XT")
    pub model: String,

    /// Vendor (e.g., "NVIDIA", "AMD", "Intel", "Apple")
    pub vendor: String,

    /// Total GPU memory in bytes
    pub memory_bytes: Option<u64>,

    /// Available GPU memory in bytes
    pub available_bytes: Option<u64>,

    /// GPU type (discrete, integrated, virtual)
    pub gpu_type: GpuType,

    /// Compute capabilities (CUDA, OpenCL, Vulkan, Metal)
    pub capabilities: Vec<String>,

    /// PCI bus address (e.g., "0000:01:00.0")
    pub pci_address: Option<String>,
}

/// GPU Type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuType {
    Discrete,
    Integrated,
    Virtual,
}

/// Memory Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total physical memory in bytes
    pub total_bytes: u64,

    /// Available memory in bytes
    pub available_bytes: u64,

    /// Used memory in bytes
    pub used_bytes: u64,

    /// Memory usage percentage (0-100)
    pub usage_percent: f64,

    /// Swap/page file total in bytes (if available)
    pub swap_total_bytes: Option<u64>,

    /// Swap/page file used in bytes (if available)
    pub swap_used_bytes: Option<u64>,
}

/// Storage Device Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Device name (e.g., "/dev/sda", "C:", "/dev/disk0")
    pub device: String,

    /// Mount point (e.g., "/", "C:\\", "/home")
    pub mount_point: String,

    /// Filesystem type (e.g., "ext4", "NTFS", "APFS", "ZFS")
    pub filesystem: String,

    /// Total capacity in bytes
    pub total_bytes: u64,

    /// Used space in bytes
    pub used_bytes: u64,

    /// Available space in bytes
    pub available_bytes: u64,

    /// Usage percentage (0-100)
    pub usage_percent: f64,

    /// Storage type (HDD, SSD, NVMe, Network)
    pub storage_type: StorageType,
}

/// Storage Type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageType {
    HDD,
    SSD,
    NVMe,
    Network,
    Unknown,
}

/// Raw device-unique identifiers read from the OS.
///
/// These are the AUTHENTICATION INPUTS to the four-proof State Proof.
/// Every field is optional because sources degrade gracefully — DMI is
/// often root-only, machine-id may be absent in minimal containers, and
/// disk serials require a physical backing device. The composed
/// `DeviceFingerprint` tolerates missing components but requires
/// `min_sources` independent sources to be considered trustworthy.
///
/// NONE of these are addresses — assets get IPv6 addresses, devices do not.
/// The fingerprint derived from these binds the node's genesis proofs to
/// this specific physical machine.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceIdentifiers {
    /// `/etc/machine-id` (or `/var/lib/dbus/machine-id` fallback).
    pub machine_id: Option<String>,
    /// `/sys/class/dmi/id/product_uuid` (root-only on most systems).
    pub product_uuid: Option<String>,
    /// `/sys/class/dmi/id/board_serial` (root-only on most systems).
    pub board_serial: Option<String>,
    /// `/sys/class/dmi/id/product_serial` (root-only on most systems).
    pub product_serial: Option<String>,
    /// Serial of the disk backing the largest mounted filesystem.
    pub primary_disk_serial: Option<String>,
    /// MAC address of the primary (non-loopback, carrier-up) interface.
    pub primary_mac: Option<String>,
}

impl DeviceIdentifiers {
    /// Count of independent identifier sources actually present.
    ///
    /// machine-id, DMI (any of uuid/board/product serial counts once),
    /// disk serial, and MAC are treated as four independent source classes.
    pub fn source_count(&self) -> usize {
        let mut n = 0;
        if self.machine_id.as_ref().is_some_and(|s| !s.is_empty()) {
            n += 1;
        }
        let has_dmi = [&self.product_uuid, &self.board_serial, &self.product_serial]
            .iter()
            .any(|f| f.as_ref().is_some_and(|s| !s.is_empty()));
        if has_dmi {
            n += 1;
        }
        if self
            .primary_disk_serial
            .as_ref()
            .is_some_and(|s| !s.is_empty())
        {
            n += 1;
        }
        if self.primary_mac.as_ref().is_some_and(|s| !s.is_empty()) {
            n += 1;
        }
        n
    }
}

/// Primary network interface identity (MAC + carrier state).
///
/// Replaces the historic hardcoded loopback `::1` network asset. The MAC
/// is a device-unique fingerprint component; carrier state selects a live
/// interface over dead ones.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NicInfo {
    /// Interface name (e.g. "eth0", "wlan0", "enp3s0").
    pub name: String,
    /// Hardware MAC address (e.g. "aa:bb:cc:dd:ee:ff").
    pub mac: String,
    /// Whether the link reports carrier (physically connected).
    pub carrier: bool,
    /// Whether this is the loopback interface.
    pub is_loopback: bool,
}

/// Composed, device-bound fingerprint = `BLAKE3(machine_id || product_uuid
/// || board_serial || disk_serial || mac)`.
///
/// This is an AUTHENTICATION INPUT to the four proofs, NOT an address.
/// It is folded into all four genesis proofs so a copied identity directory
/// run on a different physical machine fails the continuity gate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceFingerprint {
    /// 32-byte BLAKE3 digest of the concatenated available identifiers.
    pub digest: [u8; 32],
    /// Number of independent identifier sources that contributed.
    pub source_count: usize,
    /// The raw identifiers used (for audit + continuity re-derivation).
    pub identifiers: DeviceIdentifiers,
}

impl DeviceFingerprint {
    /// Compose a fingerprint from raw identifiers.
    ///
    /// Components are hashed in a FIXED order with domain-separating labels
    /// so the digest is deterministic and stable across boots on the same
    /// machine. Missing components are simply skipped (tolerant).
    pub fn compose(ids: DeviceIdentifiers) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"hypermesh-device-fingerprint-v1");
        for (label, value) in [
            (b"machine_id".as_slice(), ids.machine_id.as_deref()),
            (b"product_uuid".as_slice(), ids.product_uuid.as_deref()),
            (b"board_serial".as_slice(), ids.board_serial.as_deref()),
            (b"product_serial".as_slice(), ids.product_serial.as_deref()),
            (b"disk_serial".as_slice(), ids.primary_disk_serial.as_deref()),
            (b"mac".as_slice(), ids.primary_mac.as_deref()),
        ] {
            if let Some(v) = value {
                if !v.is_empty() {
                    hasher.update(label);
                    hasher.update(b"=");
                    hasher.update(v.as_bytes());
                    hasher.update(b";");
                }
            }
        }
        let digest = *hasher.finalize().as_bytes();
        let source_count = ids.source_count();
        Self {
            digest,
            source_count,
            identifiers: ids,
        }
    }

    /// Hex-encoded digest (used as the recoverable device binding token).
    pub fn hex(&self) -> String {
        blake3::Hash::from(self.digest).to_hex().to_string()
    }

    /// True if at least `min_sources` independent sources contributed.
    pub fn has_min_sources(&self, min_sources: usize) -> bool {
        self.source_count >= min_sources
    }
}

/// Real-time Resource Usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage (0-100)
    pub cpu_usage_percent: f64,

    /// Memory usage percentage (0-100)
    pub memory_usage_percent: f64,

    /// System load average (1min, 5min, 15min) - Unix-like systems
    pub load_average: Option<[f64; 3]>,

    /// Network receive bytes per second (if available)
    pub network_rx_bytes_per_sec: Option<u64>,

    /// Network transmit bytes per second (if available)
    pub network_tx_bytes_per_sec: Option<u64>,

    /// Disk read bytes per second (if available)
    pub disk_read_bytes_per_sec: Option<u64>,

    /// Disk write bytes per second (if available)
    pub disk_write_bytes_per_sec: Option<u64>,

    /// Number of running processes
    pub process_count: Option<usize>,
}

/// eBPF Program Handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EbpfHandle(pub u64);

/// eBPF Attach Type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EbpfAttachType {
    /// XDP (eXpress Data Path) - Linux only
    Xdp,

    /// TC (Traffic Control) - Linux only
    TcIngress,
    TcEgress,

    /// Kprobe (kernel probe) - Linux only
    Kprobe {
        function: String,
    },

    /// Tracepoint - Linux only
    Tracepoint {
        category: String,
        name: String,
    },

    /// LSM (Linux Security Module) hook - Linux only
    Lsm {
        hook: String,
    },

    /// Windows eBPF hooks
    WindowsNetworkBind,
    WindowsNetworkConnect,

    /// BSD/macOS BPF filter
    BpfFilter,
}

/// eBPF Metrics collected by eBPF programs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbpfMetrics {
    /// Metric name
    pub name: String,

    /// Metric type (counter, gauge, histogram)
    pub metric_type: EbpfMetricType,

    /// Metric values (key-value pairs from eBPF maps)
    pub values: HashMap<String, u64>,

    /// Timestamp when metrics were collected
    pub timestamp_ms: u64,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// eBPF Metric Type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EbpfMetricType {
    /// Monotonically increasing counter
    Counter,

    /// Gauge (can go up or down)
    Gauge,

    /// Histogram buckets
    Histogram,
}

/// eBPF Program Type (determines what the program can do)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EbpfProgramType {
    /// XDP program for packet processing
    Xdp,

    /// TC program for traffic control
    Tc,

    /// Kprobe for kernel function tracing
    Kprobe,

    /// Tracepoint for kernel event tracing
    Tracepoint,

    /// LSM for security policy enforcement
    Lsm,

    /// Generic eBPF program (Windows, BSD, macOS)
    Generic,
}

impl Default for CpuInfo {
    fn default() -> Self {
        Self {
            cores: 1,
            model: "Unknown".to_string(),
            architecture: "unknown".to_string(),
            frequency_mhz: None,
            usage_percent: None,
            vendor: None,
            cache_kb: None,
        }
    }
}

impl Default for MemoryInfo {
    fn default() -> Self {
        Self {
            total_bytes: 0,
            available_bytes: 0,
            used_bytes: 0,
            usage_percent: 0.0,
            swap_total_bytes: None,
            swap_used_bytes: None,
        }
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            load_average: None,
            network_rx_bytes_per_sec: None,
            network_tx_bytes_per_sec: None,
            disk_read_bytes_per_sec: None,
            disk_write_bytes_per_sec: None,
            process_count: None,
        }
    }
}
