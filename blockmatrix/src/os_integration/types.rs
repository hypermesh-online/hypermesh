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
    Kprobe { function: String },

    /// Tracepoint - Linux only
    Tracepoint { category: String, name: String },

    /// LSM (Linux Security Module) hook - Linux only
    Lsm { hook: String },

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
