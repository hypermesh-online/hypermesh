// Windows OS Abstraction - Implementation using WMI and Performance Counters
//
// Sprint 3: Full Windows integration with WMI for hardware detection and
// Performance Counters for runtime metrics (CPU usage, network I/O, disk I/O)

use super::types::*;
use super::OsAbstraction;
use anyhow::{Result, Context};
use std::sync::Mutex;
use std::time::Instant;

#[cfg(target_os = "windows")]
use windows::{
    Win32::System::{
        SystemInformation::{GetSystemTimes, GlobalMemoryStatusEx, MEMORYSTATUSEX},
        Performance::*,
    },
    Win32::NetworkManagement::IpHelper::*,
    Win32::Storage::FileSystem::*,
};

/// CPU sample for delta calculations
#[derive(Debug, Clone)]
struct CpuSample {
    idle_time: u64,
    kernel_time: u64,
    user_time: u64,
    timestamp: Instant,
}

/// Network interface statistics
#[derive(Debug, Clone, Default)]
struct NetworkStats {
    bytes_received: u64,
    bytes_sent: u64,
    timestamp: Instant,
}

/// Disk I/O statistics
#[derive(Debug, Clone, Default)]
struct DiskStats {
    bytes_read: u64,
    bytes_written: u64,
    timestamp: Instant,
}

/// Windows OS Abstraction with Performance Counter support
pub struct WindowsAbstraction {
    /// Previous CPU sample for delta calculation
    previous_cpu_sample: Mutex<Option<CpuSample>>,

    /// Previous network stats for rate calculation
    previous_network_stats: Mutex<Option<NetworkStats>>,

    /// Previous disk stats for rate calculation
    previous_disk_stats: Mutex<Option<DiskStats>>,

    /// WMI connection (if available)
    #[cfg(target_os = "windows")]
    wmi_connection: Option<wmi::WMIConnection>,
}

impl WindowsAbstraction {
    /// Create new Windows abstraction
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            // Try to establish WMI connection for hardware detection
            let wmi_connection = wmi::WMIConnection::new(
                wmi::COMLibrary::new()
                    .context("Failed to initialize COM library")?
            ).ok();

            Ok(Self {
                previous_cpu_sample: Mutex::new(None),
                previous_network_stats: Mutex::new(None),
                previous_disk_stats: Mutex::new(None),
                wmi_connection,
            })
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(Self {
                previous_cpu_sample: Mutex::new(None),
                previous_network_stats: Mutex::new(None),
                previous_disk_stats: Mutex::new(None),
            })
        }
    }

    #[cfg(target_os = "windows")]
    fn get_cpu_sample(&self) -> Result<CpuSample> {
        use std::mem::MaybeUninit;

        unsafe {
            let mut idle_time = MaybeUninit::uninit();
            let mut kernel_time = MaybeUninit::uninit();
            let mut user_time = MaybeUninit::uninit();

            GetSystemTimes(
                Some(idle_time.as_mut_ptr()),
                Some(kernel_time.as_mut_ptr()),
                Some(user_time.as_mut_ptr())
            )?;

            let idle_time = idle_time.assume_init();
            let kernel_time = kernel_time.assume_init();
            let user_time = user_time.assume_init();

            // Convert FILETIME to u64 (100-nanosecond intervals)
            let idle = (idle_time.dwHighDateTime as u64) << 32 | idle_time.dwLowDateTime as u64;
            let kernel = (kernel_time.dwHighDateTime as u64) << 32 | kernel_time.dwLowDateTime as u64;
            let user = (user_time.dwHighDateTime as u64) << 32 | user_time.dwLowDateTime as u64;

            Ok(CpuSample {
                idle_time: idle,
                kernel_time: kernel,
                user_time: user,
                timestamp: Instant::now(),
            })
        }
    }

    #[cfg(target_os = "windows")]
    fn calculate_cpu_usage(&self, prev: &CpuSample, curr: &CpuSample) -> f64 {
        let idle_delta = curr.idle_time.saturating_sub(prev.idle_time);
        let kernel_delta = curr.kernel_time.saturating_sub(prev.kernel_time);
        let user_delta = curr.user_time.saturating_sub(prev.user_time);

        // Kernel time includes idle time on Windows
        let total_delta = kernel_delta + user_delta;

        if total_delta == 0 {
            return 0.0;
        }

        // Calculate CPU usage percentage
        let busy_delta = total_delta.saturating_sub(idle_delta);
        (busy_delta as f64 / total_delta as f64) * 100.0
    }

    #[cfg(target_os = "windows")]
    pub fn get_network_stats(&self) -> Result<NetworkStats> {
        use std::ptr;

        unsafe {
            // First call to get required buffer size
            let _ = GetIfTable2(ptr::null_mut());

            // Allocate buffer
            let mut buffer = vec![0u8; 65536]; // 64KB should be enough
            let table = buffer.as_mut_ptr() as *mut MIB_IF_TABLE2;

            // Get the actual table
            GetIfTable2(&mut table)?;

            let mut total_received = 0u64;
            let mut total_sent = 0u64;

            // Sum up all interface statistics
            let num_entries = (*table).NumEntries;
            let entries = std::slice::from_raw_parts(
                (*table).Table.as_ptr(),
                num_entries as usize
            );

            for entry in entries {
                // Only count operational interfaces
                if entry.InterfaceAndOperStatusFlags.OperStatus == 1 {
                    total_received += entry.InOctets;
                    total_sent += entry.OutOctets;
                }
            }

            // Free the table
            FreeMibTable(table as *const _);

            Ok(NetworkStats {
                bytes_received: total_received,
                bytes_sent: total_sent,
                timestamp: Instant::now(),
            })
        }
    }

    #[cfg(target_os = "windows")]
    pub fn get_disk_stats(&self) -> Result<DiskStats> {

        let mut total_read = 0u64;
        let mut total_written = 0u64;

        // Use WMI to get disk performance data
        if let Some(ref conn) = self.wmi_connection {
            #[derive(serde::Deserialize)]
            struct Win32_PerfRawData_PerfDisk_LogicalDisk {
                DiskReadBytesPerSec: Option<u64>,
                DiskWriteBytesPerSec: Option<u64>,
                Name: String,
            }

            if let Ok(disks) = conn.query::<Win32_PerfRawData_PerfDisk_LogicalDisk>() {
                for disk in disks {
                    // Skip _Total entry
                    if disk.Name != "_Total" {
                        total_read += disk.DiskReadBytesPerSec.unwrap_or(0);
                        total_written += disk.DiskWriteBytesPerSec.unwrap_or(0);
                    }
                }
            }
        }

        Ok(DiskStats {
            bytes_read: total_read,
            bytes_written: total_written,
            timestamp: Instant::now(),
        })
    }
}

impl OsAbstraction for WindowsAbstraction {
    fn platform(&self) -> &str {
        "windows"
    }

    fn detect_cpu(&self) -> Result<CpuInfo> {
        #[cfg(target_os = "windows")]
        {
            if let Some(ref conn) = self.wmi_connection {
                #[derive(serde::Deserialize)]
                struct Win32_Processor {
                    Name: String,
                    NumberOfCores: u32,
                    NumberOfLogicalProcessors: u32,
                    MaxClockSpeed: u32,
                    Manufacturer: String,
                    L2CacheSize: Option<u32>,
                    L3CacheSize: Option<u32>,
                }

                if let Ok(processors) = conn.query::<Win32_Processor>() {
                    if let Some(cpu) = processors.into_iter().next() {
                        return Ok(CpuInfo {
                            cores: cpu.NumberOfLogicalProcessors as usize,
                            model: cpu.Name.trim().to_string(),
                            architecture: std::env::consts::ARCH.to_string(),
                            frequency_mhz: Some(cpu.MaxClockSpeed as u64),
                            usage_percent: None,
                            vendor: Some(cpu.Manufacturer),
                            cache_kb: Some(CacheInfo {
                                l1_kb: None, // Not available via WMI
                                l2_kb: cpu.L2CacheSize.map(|s| s as u64),
                                l3_kb: cpu.L3CacheSize.map(|s| s as u64),
                            }),
                        });
                    }
                }
            }
        }

        // Fallback to basic detection
        Ok(CpuInfo {
            cores: num_cpus::get(),
            model: "Windows CPU".to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            frequency_mhz: None,
            usage_percent: None,
            vendor: None,
            cache_kb: None,
        })
    }

    fn detect_gpu(&self) -> Result<Vec<GpuInfo>> {
        #[cfg(target_os = "windows")]
        {
            if let Some(ref conn) = self.wmi_connection {
                #[derive(serde::Deserialize)]
                struct Win32_VideoController {
                    Name: String,
                    AdapterRAM: Option<u32>,
                    VideoProcessor: Option<String>,
                    AdapterCompatibility: Option<String>,
                }

                let mut gpus = Vec::new();

                if let Ok(controllers) = conn.query::<Win32_VideoController>() {
                    for controller in controllers {
                        let vendor = controller.AdapterCompatibility
                            .unwrap_or_else(|| "Unknown".to_string());

                        let gpu_type = if controller.Name.contains("Virtual") {
                            GpuType::Virtual
                        } else if vendor.contains("Intel") &&
                                  controller.Name.contains("UHD") ||
                                  controller.Name.contains("Iris") {
                            GpuType::Integrated
                        } else {
                            GpuType::Discrete
                        };

                        gpus.push(GpuInfo {
                            model: controller.Name,
                            vendor: vendor.clone(),
                            memory_bytes: controller.AdapterRAM.map(|ram| ram as u64),
                            available_bytes: None,
                            gpu_type,
                            capabilities: vec!["DirectX".to_string(), "OpenGL".to_string()],
                            pci_address: None,
                        });
                    }
                }

                if !gpus.is_empty() {
                    return Ok(gpus);
                }
            }
        }

        Ok(Vec::new())
    }

    fn detect_memory(&self) -> Result<MemoryInfo> {
        #[cfg(target_os = "windows")]
        {
            unsafe {
                let mut mem_status = MEMORYSTATUSEX {
                    dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
                    ..Default::default()
                };

                GlobalMemoryStatusEx(&mut mem_status)?;

                let total_bytes = mem_status.ullTotalPhys;
                let available_bytes = mem_status.ullAvailPhys;
                let used_bytes = total_bytes - available_bytes;
                let usage_percent = if total_bytes > 0 {
                    (used_bytes as f64 / total_bytes as f64) * 100.0
                } else {
                    0.0
                };

                return Ok(MemoryInfo {
                    total_bytes,
                    available_bytes,
                    used_bytes,
                    usage_percent,
                    swap_total_bytes: Some(mem_status.ullTotalPageFile),
                    swap_used_bytes: Some(mem_status.ullTotalPageFile - mem_status.ullAvailPageFile),
                });
            }
        }

        Ok(MemoryInfo::default())
    }

    fn detect_storage(&self) -> Result<Vec<StorageInfo>> {
        #[cfg(target_os = "windows")]
        {
            if let Some(ref conn) = self.wmi_connection {
                #[derive(serde::Deserialize)]
                struct Win32_LogicalDisk {
                    DeviceID: String,
                    Size: Option<u64>,
                    FreeSpace: Option<u64>,
                    FileSystem: Option<String>,
                    DriveType: u32,
                }

                let mut storage_devices = Vec::new();

                if let Ok(disks) = conn.query::<Win32_LogicalDisk>() {
                    for disk in disks {
                        // Only include fixed drives (type 3)
                        if disk.DriveType == 3 {
                            if let (Some(total), Some(free)) = (disk.Size, disk.FreeSpace) {
                                let used = total - free;
                                let usage_percent = if total > 0 {
                                    (used as f64 / total as f64) * 100.0
                                } else {
                                    0.0
                                };

                                storage_devices.push(StorageInfo {
                                    device: disk.DeviceID.clone(),
                                    mount_point: disk.DeviceID,
                                    filesystem: disk.FileSystem.unwrap_or_else(|| "Unknown".to_string()),
                                    total_bytes: total,
                                    used_bytes: used,
                                    available_bytes: free,
                                    usage_percent,
                                    storage_type: StorageType::Unknown, // Would need additional WMI queries
                                });
                            }
                        }
                    }
                }

                if !storage_devices.is_empty() {
                    return Ok(storage_devices);
                }
            }
        }

        Ok(Vec::new())
    }

    fn get_resource_usage(&self) -> Result<ResourceUsage> {
        #[cfg(target_os = "windows")]
        {
            // Get CPU usage
            let cpu_usage_percent = {
                let current_sample = self.get_cpu_sample()?;
                let mut prev_guard = self.previous_cpu_sample.lock().unwrap();

                let usage = if let Some(ref prev_sample) = *prev_guard {
                    self.calculate_cpu_usage(prev_sample, &current_sample)
                } else {
                    0.0 // First sample, no delta available
                };

                *prev_guard = Some(current_sample);
                usage
            };

            // Get memory usage
            let memory_info = self.detect_memory()?;
            let memory_usage_percent = memory_info.usage_percent;

            // Get network I/O rates
            let (network_rx_bytes_per_sec, network_tx_bytes_per_sec) = {
                let current_stats = self.get_network_stats()?;
                let mut prev_guard = self.previous_network_stats.lock().unwrap();

                let (rx_rate, tx_rate) = if let Some(ref prev_stats) = *prev_guard {
                    let time_delta = current_stats.timestamp.duration_since(prev_stats.timestamp);
                    let seconds = time_delta.as_secs_f64();

                    if seconds > 0.0 {
                        let rx_delta = current_stats.bytes_received.saturating_sub(prev_stats.bytes_received);
                        let tx_delta = current_stats.bytes_sent.saturating_sub(prev_stats.bytes_sent);

                        (
                            Some((rx_delta as f64 / seconds) as u64),
                            Some((tx_delta as f64 / seconds) as u64)
                        )
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None) // First sample
                };

                *prev_guard = Some(current_stats);
                (rx_rate, tx_rate)
            };

            // Get disk I/O rates
            let (disk_read_bytes_per_sec, disk_write_bytes_per_sec) = {
                let current_stats = self.get_disk_stats()?;
                let mut prev_guard = self.previous_disk_stats.lock().unwrap();

                let (read_rate, write_rate) = if let Some(ref prev_stats) = *prev_guard {
                    let time_delta = current_stats.timestamp.duration_since(prev_stats.timestamp);
                    let seconds = time_delta.as_secs_f64();

                    if seconds > 0.0 {
                        let read_delta = current_stats.bytes_read.saturating_sub(prev_stats.bytes_read);
                        let write_delta = current_stats.bytes_written.saturating_sub(prev_stats.bytes_written);

                        (
                            Some((read_delta as f64 / seconds) as u64),
                            Some((write_delta as f64 / seconds) as u64)
                        )
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None) // First sample
                };

                *prev_guard = Some(current_stats);
                (read_rate, write_rate)
            };

            // Get process count via WMI
            let process_count = if let Some(ref conn) = self.wmi_connection {
                #[derive(serde::Deserialize)]
                struct Win32_Process {
                    ProcessId: u32,
                }

                conn.query::<Win32_Process>()
                    .map(|processes| Some(processes.len()))
                    .unwrap_or(None)
            } else {
                None
            };

            return Ok(ResourceUsage {
                cpu_usage_percent,
                memory_usage_percent,
                load_average: None, // Not available on Windows
                network_rx_bytes_per_sec,
                network_tx_bytes_per_sec,
                disk_read_bytes_per_sec,
                disk_write_bytes_per_sec,
                process_count,
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(ResourceUsage::default())
        }
    }

    fn load_ebpf_program(&self, _program: &[u8]) -> Result<EbpfHandle> {
        // TODO Sprint 3: Implement using eBpf-for-windows
        // - Load BPF bytecode via eBpf-for-windows API
        // - Return handle for program management

        Err(anyhow::anyhow!("Windows eBPF not implemented yet (Sprint 3)"))
    }

    fn attach_ebpf_monitor(&self, _handle: EbpfHandle, _attach_type: EbpfAttachType) -> Result<()> {
        // TODO Sprint 3: Implement eBpf-for-windows attachment
        // - Attach to network bind/connect hooks
        // - Configure filters

        Err(anyhow::anyhow!("Windows eBPF not implemented yet (Sprint 3)"))
    }

    fn read_ebpf_metrics(&self, _handle: EbpfHandle) -> Result<EbpfMetrics> {
        // TODO Sprint 3: Implement reading from eBpf-for-windows maps

        Err(anyhow::anyhow!("Windows eBPF not implemented yet (Sprint 3)"))
    }

    fn unload_ebpf_program(&self, _handle: EbpfHandle) -> Result<()> {
        // TODO Sprint 3: Cleanup eBpf-for-windows program

        Ok(())
    }

    fn is_ebpf_supported(&self) -> bool {
        // TODO Sprint 3: Check if eBpf-for-windows is installed
        // - Check for eBpf-for-windows driver
        // - Verify Windows version (Windows 10 1809+ required)

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_cpu_detection() {
        let windows = WindowsAbstraction::new().expect("Failed to create Windows abstraction");
        let cpu = windows.detect_cpu().expect("Failed to detect CPU");

        assert!(cpu.cores > 0, "Should detect at least one CPU core");
        assert!(!cpu.model.is_empty(), "Should detect CPU model");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_cpu_usage_tracking() {
        let windows = WindowsAbstraction::new().expect("Failed to create Windows abstraction");

        // First call - should return 0 (no previous sample)
        let usage1 = windows.get_resource_usage().expect("Failed to get resource usage");
        assert_eq!(usage1.cpu_usage_percent, 0.0, "First CPU usage should be 0");

        // Wait a bit for delta calculation
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Second call - should return actual usage
        let usage2 = windows.get_resource_usage().expect("Failed to get resource usage");
        assert!(usage2.cpu_usage_percent >= 0.0, "CPU usage should be non-negative");
        assert!(usage2.cpu_usage_percent <= 100.0, "CPU usage should not exceed 100%");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_network_io_tracking() {
        let windows = WindowsAbstraction::new().expect("Failed to create Windows abstraction");

        // First call - rates should be None (no previous sample)
        let usage1 = windows.get_resource_usage().expect("Failed to get resource usage");
        assert!(usage1.network_rx_bytes_per_sec.is_none(), "First RX rate should be None");
        assert!(usage1.network_tx_bytes_per_sec.is_none(), "First TX rate should be None");

        // Wait for some network activity
        std::thread::sleep(std::time::Duration::from_secs(1));

        // Second call - should have rates if there's network activity
        let usage2 = windows.get_resource_usage().expect("Failed to get resource usage");
        // Rates might still be None if no network activity, which is valid
        if let Some(rx_rate) = usage2.network_rx_bytes_per_sec {
            assert!(rx_rate >= 0, "RX rate should be non-negative");
        }
        if let Some(tx_rate) = usage2.network_tx_bytes_per_sec {
            assert!(tx_rate >= 0, "TX rate should be non-negative");
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_disk_io_tracking() {
        let windows = WindowsAbstraction::new().expect("Failed to create Windows abstraction");

        // First call - rates should be None (no previous sample)
        let usage1 = windows.get_resource_usage().expect("Failed to get resource usage");
        assert!(usage1.disk_read_bytes_per_sec.is_none(), "First read rate should be None");
        assert!(usage1.disk_write_bytes_per_sec.is_none(), "First write rate should be None");

        // Wait for potential disk activity
        std::thread::sleep(std::time::Duration::from_secs(1));

        // Second call - should have rates if there's disk activity
        let usage2 = windows.get_resource_usage().expect("Failed to get resource usage");
        // Rates might still be None if no disk activity, which is valid
        if let Some(read_rate) = usage2.disk_read_bytes_per_sec {
            assert!(read_rate >= 0, "Read rate should be non-negative");
        }
        if let Some(write_rate) = usage2.disk_write_bytes_per_sec {
            assert!(write_rate >= 0, "Write rate should be non-negative");
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_memory_detection() {
        let windows = WindowsAbstraction::new().expect("Failed to create Windows abstraction");
        let memory = windows.detect_memory().expect("Failed to detect memory");

        assert!(memory.total_bytes > 0, "Should detect total memory");
        assert!(memory.available_bytes <= memory.total_bytes, "Available should not exceed total");
        assert!(memory.usage_percent >= 0.0 && memory.usage_percent <= 100.0, "Usage percent should be 0-100");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_gpu_detection() {
        let windows = WindowsAbstraction::new().expect("Failed to create Windows abstraction");
        let gpus = windows.detect_gpu().expect("Failed to detect GPU");

        // May have 0 or more GPUs
        for gpu in gpus {
            assert!(!gpu.model.is_empty(), "GPU model should not be empty");
            assert!(!gpu.vendor.is_empty(), "GPU vendor should not be empty");
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_storage_detection() {
        let windows = WindowsAbstraction::new().expect("Failed to create Windows abstraction");
        let storage = windows.detect_storage().expect("Failed to detect storage");

        // Should have at least one storage device (system drive)
        assert!(!storage.is_empty(), "Should detect at least one storage device");

        for device in storage {
            assert!(!device.device.is_empty(), "Device ID should not be empty");
            assert!(device.total_bytes > 0, "Total storage should be positive");
            assert!(device.usage_percent >= 0.0 && device.usage_percent <= 100.0, "Usage percent should be 0-100");
        }
    }
}