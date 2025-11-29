//! Hardware configuration and detection types

use serde::{Deserialize, Serialize};

/// Hardware configuration detected/configured by user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfiguration {
    pub cpu_info: CpuInfo,
    pub gpu_info: Vec<GpuInfo>,
    pub memory_info: MemoryInfo,
    pub storage_info: Vec<StorageInfo>,
    pub network_info: NetworkInfo,
    pub verification_status: VerificationStatus,
}

/// CPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model: String,
    pub cores: u32,
    pub threads: u32,
    pub base_frequency: u64,
    pub max_frequency: u64,
    pub cache_l1: u64,
    pub cache_l2: u64,
    pub cache_l3: u64,
    pub architecture: String,
    pub instruction_sets: Vec<String>,
}

/// GPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub model: String,
    pub vendor: String,
    pub memory_usage: u64,
    pub compute_units: u32,
    pub base_clock: u64,
    pub memory_clock: u64,
    pub memory_bus_width: u32,
    pub compute_capability: Option<String>,
    pub supported_apis: Vec<String>,
}

/// Memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_capacity: u64,
    pub available_capacity: u64,
    pub memory_type: String,
    pub speed: u64,
    pub modules: Vec<MemoryModule>,
}

/// Memory module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryModule {
    pub size: u64,
    pub speed: u64,
    pub latency: String,
    pub manufacturer: String,
}

/// Storage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub device_type: StorageType,
    pub capacity: u64,
    pub available: u64,
    pub interface: String,
    pub read_speed: u64,
    pub write_speed: u64,
    pub manufacturer: String,
    pub model: String,
}

/// Storage device types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    HDD,
    SSD,
    NVMe,
    Optane,
    Network,
}

/// Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
    pub bandwidth_upload: u64,
    pub bandwidth_download: u64,
    pub latency: u64,
    pub is_metered: bool,
    pub location: NetworkLocation,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub interface_type: String,
    pub speed: u64,
    pub mac_address: String,
    pub ip_addresses: Vec<String>,
}

/// Network location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLocation {
    pub country: String,
    pub region: String,
    pub city: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: String,
}

/// Hardware verification status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Failed(String),
    Expired,
}
