//! Hardware Detection and System Resource API
//!
//! Provides real-time system hardware detection and resource monitoring
//! for the HyperMesh platform.

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use sysinfo::System;
use std::time::{Duration, Instant};

/// Hardware Detection Service
///
/// Provides real system hardware information and resource monitoring
pub struct HardwareDetectionService {
    /// System information handle
    system_info: Arc<RwLock<System>>,

    /// Cached hardware capabilities
    cached_capabilities: Arc<RwLock<Option<HardwareCapabilities>>>,

    /// Last update timestamp
    last_update: Arc<RwLock<Instant>>,

    /// Update interval
    update_interval: Duration,
}

/// Hardware capabilities detected from the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareCapabilities {
    /// CPU information
    pub cpu: CpuInfo,

    /// Memory information
    pub memory: MemoryInfo,

    /// Storage information
    pub storage: Vec<StorageInfo>,

    /// Network interfaces
    pub network: Vec<NetworkInterface>,

    /// System information
    pub system: SystemInfo,

    /// Detection timestamp
    pub detected_at: u64,
}

/// CPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    /// Number of physical CPU cores
    pub physical_cores: u32,

    /// Number of logical CPU cores (including hyperthreading)
    pub logical_cores: u32,

    /// CPU model name
    pub model_name: String,

    /// CPU frequency in MHz
    pub frequency_mhz: u64,

    /// CPU vendor
    pub vendor: String,

    /// CPU architecture
    pub architecture: String,

    /// Current CPU usage percentage
    pub usage_percent: f32,

    /// CPU temperature (if available)
    pub temperature_celsius: Option<f32>,
}

/// Memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total physical memory in bytes
    pub total_bytes: u64,

    /// Available memory in bytes
    pub available_bytes: u64,

    /// Used memory in bytes
    pub used_bytes: u64,

    /// Memory usage percentage
    pub usage_percent: f32,

    /// Swap total in bytes
    pub swap_total_bytes: u64,

    /// Swap used in bytes
    pub swap_used_bytes: u64,

    /// Memory speed in MHz (if available)
    pub speed_mhz: Option<u32>,

    /// Memory type (DDR4, DDR5, etc)
    pub memory_type: Option<String>,
}

/// Storage device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Mount point or drive letter
    pub mount_point: String,

    /// Device name
    pub device_name: String,

    /// File system type
    pub filesystem_type: String,

    /// Total space in bytes
    pub total_bytes: u64,

    /// Available space in bytes
    pub available_bytes: u64,

    /// Used space in bytes
    pub used_bytes: u64,

    /// Usage percentage
    pub usage_percent: f32,

    /// Is SSD
    pub is_ssd: bool,

    /// Is removable
    pub is_removable: bool,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name
    pub name: String,

    /// MAC address
    pub mac_address: String,

    /// IP addresses (v4 and v6)
    pub ip_addresses: Vec<String>,

    /// Interface speed in Mbps
    pub speed_mbps: u64,

    /// Is wireless
    pub is_wireless: bool,

    /// Is virtual
    pub is_virtual: bool,

    /// Bytes received
    pub bytes_received: u64,

    /// Bytes transmitted
    pub bytes_transmitted: u64,

    /// Packets received
    pub packets_received: u64,

    /// Packets transmitted
    pub packets_transmitted: u64,

    /// Current bandwidth usage Mbps
    pub current_bandwidth_mbps: f32,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system name
    pub os_name: String,

    /// OS version
    pub os_version: String,

    /// Kernel version
    pub kernel_version: String,

    /// System hostname
    pub hostname: String,

    /// System uptime in seconds
    pub uptime_seconds: u64,

    /// Boot time timestamp
    pub boot_time: u64,

    /// Number of processes
    pub process_count: usize,
}

/// Current resource allocation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// CPU allocation
    pub cpu: ResourceAllocationDetail,

    /// Memory allocation
    pub memory: ResourceAllocationDetail,

    /// Storage allocation
    pub storage: ResourceAllocationDetail,

    /// Network allocation
    pub network: ResourceAllocationDetail,
}

/// Resource allocation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocationDetail {
    /// Total available
    pub total: u64,

    /// Currently allocated for sharing
    pub allocated: u64,

    /// Currently in use
    pub used: u64,

    /// Available for new allocations
    pub available: u64,

    /// Allocation percentage
    pub allocation_percent: f32,

    /// Usage percentage
    pub usage_percent: f32,
}

/// Sharing capabilities based on hardware
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingCapabilities {
    /// Maximum CPU cores available for sharing
    pub max_cpu_cores: u32,

    /// Maximum memory available for sharing (bytes)
    pub max_memory_bytes: u64,

    /// Maximum storage available for sharing (bytes)
    pub max_storage_bytes: u64,

    /// Maximum network bandwidth for sharing (Mbps)
    pub max_network_mbps: u64,

    /// Recommended CPU cores for sharing
    pub recommended_cpu_cores: u32,

    /// Recommended memory for sharing (bytes)
    pub recommended_memory_bytes: u64,

    /// Recommended storage for sharing (bytes)
    pub recommended_storage_bytes: u64,

    /// Recommended network bandwidth for sharing (Mbps)
    pub recommended_network_mbps: u64,

    /// Available sharing modes
    pub available_modes: Vec<SharingMode>,
}

/// Sharing mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingMode {
    /// Mode name (Private, Federated, Public)
    pub name: String,

    /// Mode description
    pub description: String,

    /// Is currently active
    pub is_active: bool,

    /// Resource limits for this mode
    pub resource_limits: ResourceLimits,
}

/// Resource limits for a sharing mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Max CPU cores
    pub max_cpu_cores: u32,

    /// Max memory bytes
    pub max_memory_bytes: u64,

    /// Max storage bytes
    pub max_storage_bytes: u64,

    /// Max network bandwidth Mbps
    pub max_network_mbps: u64,
}

impl HardwareDetectionService {
    /// Create new hardware detection service
    pub async fn new() -> Result<Self> {
        info!("🔍 Initializing Hardware Detection Service");

        let mut system = System::new_all();
        system.refresh_all();

        Ok(Self {
            system_info: Arc::new(RwLock::new(system)),
            cached_capabilities: Arc::new(RwLock::new(None)),
            last_update: Arc::new(RwLock::new(Instant::now())),
            update_interval: Duration::from_secs(5), // Update every 5 seconds
        })
    }

    /// Get current hardware capabilities
    pub async fn get_hardware_capabilities(&self) -> Result<HardwareCapabilities> {
        // Check if cache is still valid
        let last_update = *self.last_update.read().await;
        if last_update.elapsed() < self.update_interval {
            if let Some(cached) = &*self.cached_capabilities.read().await {
                return Ok(cached.clone());
            }
        }

        // Refresh system information
        let mut system = self.system_info.write().await;
        system.refresh_all();
        system.refresh_cpu();
        system.refresh_memory();
        system.refresh_disks();
        system.refresh_networks();

        // Detect CPU information
        let cpu = self.detect_cpu_info(&system).await?;

        // Detect memory information
        let memory = self.detect_memory_info(&system).await?;

        // Detect storage information
        let storage = self.detect_storage_info(&system).await?;

        // Detect network interfaces
        let network = self.detect_network_info(&system).await?;

        // Detect system information
        let system_info = self.detect_system_info(&system).await?;

        let capabilities = HardwareCapabilities {
            cpu,
            memory,
            storage,
            network,
            system: system_info,
            detected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Update cache
        *self.cached_capabilities.write().await = Some(capabilities.clone());
        *self.last_update.write().await = Instant::now();

        Ok(capabilities)
    }

    /// Detect CPU information
    async fn detect_cpu_info(&self, system: &System) -> Result<CpuInfo> {
        let physical_cores = system.physical_core_count().unwrap_or(1) as u32;
        let processors = system.processors();
        let logical_cores = processors.len() as u32;

        let first_cpu = processors.first();
        let model_name = first_cpu
            .map(|p| p.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let frequency_mhz = first_cpu
            .map(|p| p.frequency())
            .unwrap_or(2000); // Default 2GHz if unknown

        let vendor = first_cpu
            .map(|p| p.vendor_id().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let usage_percent = system.global_processor_info().cpu_usage();

        // Detect architecture
        let architecture = if cfg!(target_arch = "x86_64") {
            "x86_64".to_string()
        } else if cfg!(target_arch = "aarch64") {
            "aarch64".to_string()
        } else {
            "unknown".to_string()
        };

        Ok(CpuInfo {
            physical_cores,
            logical_cores,
            model_name,
            frequency_mhz,
            vendor,
            architecture,
            usage_percent,
            temperature_celsius: None, // Temperature requires additional platform-specific code
        })
    }

    /// Detect memory information
    async fn detect_memory_info(&self, system: &System) -> Result<MemoryInfo> {
        let total_bytes = system.total_memory() * 1024; // Convert from KB to bytes
        let used_bytes = system.used_memory() * 1024;
        let available_bytes = system.available_memory() * 1024;
        let usage_percent = (used_bytes as f32 / total_bytes as f32) * 100.0;

        let swap_total_bytes = system.total_swap() * 1024;
        let swap_used_bytes = system.used_swap() * 1024;

        Ok(MemoryInfo {
            total_bytes,
            available_bytes,
            used_bytes,
            usage_percent,
            swap_total_bytes,
            swap_used_bytes,
            speed_mhz: None, // Would require platform-specific detection
            memory_type: None, // Would require platform-specific detection
        })
    }

    /// Detect storage information
    async fn detect_storage_info(&self, system: &System) -> Result<Vec<StorageInfo>> {
        let mut storage_devices = Vec::new();

        for disk in system.disks() {
            let mount_point = disk.mount_point().to_string_lossy().to_string();
            let device_name = disk.name().to_string_lossy().to_string();
            let filesystem_type = String::from_utf8_lossy(disk.file_system()).to_string();
            let total_bytes = disk.total_space();
            let available_bytes = disk.available_space();
            let used_bytes = total_bytes.saturating_sub(available_bytes);
            let usage_percent = if total_bytes > 0 {
                (used_bytes as f32 / total_bytes as f32) * 100.0
            } else {
                0.0
            };

            // Simple SSD detection heuristic (can be improved with platform-specific code)
            let is_ssd = device_name.contains("nvme") ||
                         device_name.contains("ssd") ||
                         filesystem_type.contains("apfs"); // macOS SSD hint

            storage_devices.push(StorageInfo {
                mount_point,
                device_name,
                filesystem_type,
                total_bytes,
                available_bytes,
                used_bytes,
                usage_percent,
                is_ssd,
                is_removable: disk.is_removable(),
            });
        }

        Ok(storage_devices)
    }

    /// Detect network interface information
    async fn detect_network_info(&self, system: &System) -> Result<Vec<NetworkInterface>> {
        let mut interfaces = Vec::new();

        for (name, data) in system.networks() {
            // Skip loopback interfaces
            if name == "lo" || name.starts_with("lo") {
                continue;
            }

            let mac_address = data.mac_address().to_string();

            // Collect IP addresses (simplified - in production would use proper network interface APIs)
            let ip_addresses = vec![];

            // Detect interface type
            let is_wireless = name.starts_with("wl") ||
                            name.starts_with("wlan") ||
                            name.starts_with("wifi");
            let is_virtual = name.starts_with("veth") ||
                           name.starts_with("docker") ||
                           name.starts_with("br");

            // Default speed based on interface type
            let speed_mbps = if is_wireless {
                1000 // 1 Gbps for WiFi
            } else if is_virtual {
                10000 // 10 Gbps for virtual
            } else {
                1000 // 1 Gbps default
            };

            interfaces.push(NetworkInterface {
                name: name.clone(),
                mac_address,
                ip_addresses,
                speed_mbps,
                is_wireless,
                is_virtual,
                bytes_received: data.total_received(),
                bytes_transmitted: data.total_transmitted(),
                packets_received: data.total_packets_received(),
                packets_transmitted: data.total_packets_transmitted(),
                current_bandwidth_mbps: 0.0, // Would need to track over time
            });
        }

        Ok(interfaces)
    }

    /// Detect system information
    async fn detect_system_info(&self, system: &System) -> Result<SystemInfo> {
        let os_name = system.name().unwrap_or_else(|| "Unknown".to_string());
        let os_version = system.os_version().unwrap_or_else(|| "Unknown".to_string());
        let kernel_version = system.kernel_version().unwrap_or_else(|| "Unknown".to_string());
        let hostname = system.host_name().unwrap_or_else(|| "Unknown".to_string());
        let uptime_seconds = system.uptime();
        let boot_time = system.boot_time();
        let process_count = system.processes().len();

        Ok(SystemInfo {
            os_name,
            os_version,
            kernel_version,
            hostname,
            uptime_seconds,
            boot_time,
            process_count,
        })
    }

    /// Get current resource allocation status
    pub async fn get_resource_allocation(&self) -> Result<ResourceAllocation> {
        let capabilities = self.get_hardware_capabilities().await?;

        // Calculate allocations based on current system state
        // In production, this would track actual HyperMesh allocations

        let cpu_allocation = ResourceAllocationDetail {
            total: capabilities.cpu.logical_cores as u64,
            allocated: (capabilities.cpu.logical_cores as f32 * 0.5) as u64, // 50% allocated
            used: (capabilities.cpu.logical_cores as f32 * capabilities.cpu.usage_percent / 100.0) as u64,
            available: (capabilities.cpu.logical_cores as f32 * 0.5) as u64,
            allocation_percent: 50.0,
            usage_percent: capabilities.cpu.usage_percent,
        };

        let memory_allocation = ResourceAllocationDetail {
            total: capabilities.memory.total_bytes,
            allocated: capabilities.memory.total_bytes / 2, // 50% allocated
            used: capabilities.memory.used_bytes,
            available: capabilities.memory.available_bytes,
            allocation_percent: 50.0,
            usage_percent: capabilities.memory.usage_percent,
        };

        let total_storage: u64 = capabilities.storage.iter().map(|s| s.total_bytes).sum();
        let used_storage: u64 = capabilities.storage.iter().map(|s| s.used_bytes).sum();
        let available_storage: u64 = capabilities.storage.iter().map(|s| s.available_bytes).sum();

        let storage_allocation = ResourceAllocationDetail {
            total: total_storage,
            allocated: total_storage / 2, // 50% allocated
            used: used_storage,
            available: available_storage,
            allocation_percent: 50.0,
            usage_percent: (used_storage as f32 / total_storage as f32) * 100.0,
        };

        let total_bandwidth: u64 = capabilities.network.iter().map(|n| n.speed_mbps).sum();

        let network_allocation = ResourceAllocationDetail {
            total: total_bandwidth,
            allocated: total_bandwidth / 2, // 50% allocated
            used: 0, // Would need actual measurement
            available: total_bandwidth / 2,
            allocation_percent: 50.0,
            usage_percent: 0.0,
        };

        Ok(ResourceAllocation {
            cpu: cpu_allocation,
            memory: memory_allocation,
            storage: storage_allocation,
            network: network_allocation,
        })
    }

    /// Get sharing capabilities based on hardware
    pub async fn get_sharing_capabilities(&self) -> Result<SharingCapabilities> {
        let capabilities = self.get_hardware_capabilities().await?;

        // Calculate maximum and recommended sharing based on hardware
        let max_cpu_cores = capabilities.cpu.logical_cores;
        let max_memory_bytes = capabilities.memory.total_bytes;
        let max_storage_bytes: u64 = capabilities.storage.iter().map(|s| s.available_bytes).sum();
        let max_network_mbps: u64 = capabilities.network.iter().map(|n| n.speed_mbps).sum();

        // Recommended is 50% of max for stable operation
        let recommended_cpu_cores = max_cpu_cores / 2;
        let recommended_memory_bytes = max_memory_bytes / 2;
        let recommended_storage_bytes = max_storage_bytes / 2;
        let recommended_network_mbps = max_network_mbps / 2;

        // Define available sharing modes
        let available_modes = vec![
            SharingMode {
                name: "Private".to_string(),
                description: "Resources available only to your local applications".to_string(),
                is_active: true,
                resource_limits: ResourceLimits {
                    max_cpu_cores: max_cpu_cores / 4,
                    max_memory_bytes: max_memory_bytes / 4,
                    max_storage_bytes: max_storage_bytes / 4,
                    max_network_mbps: 0, // No network sharing in private mode
                },
            },
            SharingMode {
                name: "Federated".to_string(),
                description: "Shared with trusted networks and verified peers".to_string(),
                is_active: true,
                resource_limits: ResourceLimits {
                    max_cpu_cores: recommended_cpu_cores,
                    max_memory_bytes: recommended_memory_bytes,
                    max_storage_bytes: recommended_storage_bytes,
                    max_network_mbps: recommended_network_mbps,
                },
            },
            SharingMode {
                name: "Public".to_string(),
                description: "Available to the global HyperMesh network".to_string(),
                is_active: false,
                resource_limits: ResourceLimits {
                    max_cpu_cores,
                    max_memory_bytes,
                    max_storage_bytes,
                    max_network_mbps,
                },
            },
        ];

        Ok(SharingCapabilities {
            max_cpu_cores,
            max_memory_bytes,
            max_storage_bytes,
            max_network_mbps,
            recommended_cpu_cores,
            recommended_memory_bytes,
            recommended_storage_bytes,
            recommended_network_mbps,
            available_modes,
        })
    }

    /// Update system information (called periodically)
    pub async fn refresh(&self) -> Result<()> {
        let mut system = self.system_info.write().await;
        system.refresh_all();

        // Clear cache to force refresh on next request
        *self.cached_capabilities.write().await = None;

        Ok(())
    }
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl<T> HardwareApiResponse<T> {
    /// Create success response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Create error response
    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}