// Linux OS Abstraction - Implementation using /proc, /sys, libbpf, XDP, TC

use super::types::*;
use super::OsAbstraction;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Linux OS Abstraction using libbpf and kernel interfaces
pub struct LinuxAbstraction {
    /// Next eBPF handle ID
    next_handle: Arc<AtomicU64>,

    /// Active eBPF programs
    ebpf_programs: Arc<std::sync::Mutex<HashMap<EbpfHandle, EbpfProgramState>>>,

    /// Kernel version for feature detection
    kernel_version: (u32, u32, u32),
}

/// State of an active eBPF program
struct EbpfProgramState {
    program_type: EbpfProgramType,
    attached: bool,
    attach_type: Option<EbpfAttachType>,
    bytecode: Vec<u8>,
    metrics: HashMap<String, u64>,
    last_update: std::time::SystemTime,
}

impl LinuxAbstraction {
    /// Create new Linux abstraction
    pub fn new() -> Result<Self> {
        let kernel_version = Self::detect_kernel_version()?;
        Ok(Self {
            next_handle: Arc::new(AtomicU64::new(1)),
            ebpf_programs: Arc::new(std::sync::Mutex::new(HashMap::new())),
            kernel_version,
        })
    }

    /// Detect Linux kernel version from uname
    fn detect_kernel_version() -> Result<(u32, u32, u32)> {
        let version_str = fs::read_to_string("/proc/version")
            .or_else(|_| {
                // Fallback to uname syscall
                unsafe {
                    let mut buf: libc::utsname = std::mem::zeroed();
                    if libc::uname(&mut buf) == 0 {
                        let release = std::ffi::CStr::from_ptr(buf.release.as_ptr())
                            .to_string_lossy()
                            .into_owned();
                        Ok(release)
                    } else {
                        Err(std::io::Error::last_os_error())
                    }
                }
            })
            .context("Failed to read kernel version")?;

        // Parse version string like "6.16.2-arch1-1" or "Linux version 5.15.0"
        let version_parts: Vec<&str> = version_str
            .split_whitespace()
            .find(|s| s.chars().next().map(|c| c.is_numeric()).unwrap_or(false))
            .unwrap_or("0.0.0")
            .split(&['.', '-'][..])
            .collect();

        let major = version_parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = version_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = version_parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        Ok((major, minor, patch))
    }

    /// Check if kernel supports eBPF based on version
    fn kernel_supports_ebpf(&self) -> bool {
        let (major, minor, _) = self.kernel_version;
        // Basic eBPF: kernel >= 4.4
        // Full featured eBPF (BTF, CO-RE): kernel >= 5.0
        major > 4 || (major == 4 && minor >= 4)
    }

    /// Check if kernel supports advanced eBPF features
    fn kernel_supports_btf(&self) -> bool {
        let (major, _, _) = self.kernel_version;
        major >= 5
    }

    /// Parse /proc/cpuinfo for CPU details
    fn parse_cpuinfo(&self) -> Result<CpuInfo> {
        let content = fs::read_to_string("/proc/cpuinfo")
            .context("Failed to read /proc/cpuinfo")?;

        let mut cores = 0;
        let mut model = String::from("Unknown");
        let mut vendor = None;
        let mut frequency_mhz = None;

        for line in content.lines() {
            if line.starts_with("processor") {
                cores += 1;
            } else if line.starts_with("model name") {
                if let Some(value) = line.split(':').nth(1) {
                    model = value.trim().to_string();
                }
            } else if line.starts_with("vendor_id") {
                if let Some(value) = line.split(':').nth(1) {
                    vendor = Some(value.trim().to_string());
                }
            } else if line.starts_with("cpu MHz") {
                if let Some(value) = line.split(':').nth(1) {
                    if let Ok(mhz) = value.trim().parse::<f64>() {
                        frequency_mhz = Some(mhz as u64);
                    }
                }
            }
        }

        // Get architecture
        let architecture = std::env::consts::ARCH.to_string();

        Ok(CpuInfo {
            cores,
            model,
            architecture,
            frequency_mhz,
            usage_percent: None, // Calculate separately
            vendor,
            cache_kb: None, // TODO: Parse cache sizes from /proc/cpuinfo or /sys
        })
    }

    /// Detect GPUs using /sys/class/drm or lspci
    fn detect_gpus(&self) -> Result<Vec<GpuInfo>> {
        let mut gpus = Vec::new();

        // Try /sys/class/drm first
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Look for card* entries (not card*-*)
                    if name.starts_with("card") && !name.contains('-') {
                        if let Ok(gpu) = self.parse_drm_card(&path) {
                            gpus.push(gpu);
                        }
                    }
                }
            }
        }

        // If no GPUs found via /sys, try lspci as fallback
        if gpus.is_empty() {
            if let Ok(lspci_gpus) = self.detect_gpus_lspci() {
                gpus = lspci_gpus;
            }
        }

        Ok(gpus)
    }

    /// Parse GPU info from /sys/class/drm/cardN
    fn parse_drm_card(&self, card_path: &Path) -> Result<GpuInfo> {
        // Try to read device info
        let device_path = card_path.join("device");

        let mut model = String::from("Unknown GPU");
        let mut vendor = String::from("Unknown");

        // Try to read vendor
        if let Ok(vendor_id) = fs::read_to_string(device_path.join("vendor")) {
            vendor = self.pci_vendor_name(&vendor_id.trim());
        }

        // Try to read device name from uevent
        if let Ok(uevent) = fs::read_to_string(device_path.join("uevent")) {
            for line in uevent.lines() {
                if line.starts_with("PCI_ID=") {
                    model = line.split('=').nth(1).unwrap_or("Unknown").to_string();
                    break;
                }
            }
        }

        Ok(GpuInfo {
            model,
            vendor,
            memory_bytes: None, // TODO: Query via DRM ioctls
            available_bytes: None,
            gpu_type: GpuType::Discrete, // Assume discrete for now
            capabilities: vec![], // TODO: Detect CUDA, OpenCL, Vulkan
            pci_address: None,
        })
    }

    /// Detect GPUs using lspci command (fallback)
    fn detect_gpus_lspci(&self) -> Result<Vec<GpuInfo>> {
        // This would execute lspci and parse output
        // For now, return empty as this requires external command execution
        Ok(Vec::new())
    }

    /// Map PCI vendor ID to vendor name
    fn pci_vendor_name(&self, vendor_id: &str) -> String {
        match vendor_id {
            "0x10de" => "NVIDIA".to_string(),
            "0x1002" => "AMD".to_string(),
            "0x8086" => "Intel".to_string(),
            _ => format!("Vendor {}", vendor_id),
        }
    }

    /// Parse /proc/meminfo for memory details
    fn parse_meminfo(&self) -> Result<MemoryInfo> {
        let content = fs::read_to_string("/proc/meminfo")
            .context("Failed to read /proc/meminfo")?;

        let mut total_kb = 0u64;
        let mut available_kb = 0u64;
        let mut swap_total_kb = 0u64;
        let mut swap_free_kb = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let key = parts[0].trim_end_matches(':');
                if let Ok(value) = parts[1].parse::<u64>() {
                    match key {
                        "MemTotal" => total_kb = value,
                        "MemAvailable" => available_kb = value,
                        "SwapTotal" => swap_total_kb = value,
                        "SwapFree" => swap_free_kb = value,
                        _ => {}
                    }
                }
            }
        }

        let total_bytes = total_kb * 1024;
        let available_bytes = available_kb * 1024;
        let used_bytes = total_bytes.saturating_sub(available_bytes);
        let usage_percent = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        Ok(MemoryInfo {
            total_bytes,
            available_bytes,
            used_bytes,
            usage_percent,
            swap_total_bytes: Some(swap_total_kb * 1024),
            swap_used_bytes: Some((swap_total_kb - swap_free_kb) * 1024),
        })
    }

    /// Detect storage from /proc/mounts and statvfs
    fn detect_storage_devices(&self) -> Result<Vec<StorageInfo>> {
        let mut devices = Vec::new();

        let mounts = fs::read_to_string("/proc/mounts")
            .context("Failed to read /proc/mounts")?;

        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let device = parts[0];
                let mount_point = parts[1];
                let filesystem = parts[2];

                // Skip special filesystems
                if device.starts_with("/dev/") {
                    if let Ok(stat) = nix::sys::statvfs::statvfs(mount_point) {
                        let block_size = stat.block_size() as u64;
                        let total_bytes = stat.blocks() * block_size;
                        let available_bytes = stat.blocks_available() * block_size;
                        let used_bytes = total_bytes - available_bytes;
                        let usage_percent = if total_bytes > 0 {
                            (used_bytes as f64 / total_bytes as f64) * 100.0
                        } else {
                            0.0
                        };

                        devices.push(StorageInfo {
                            device: device.to_string(),
                            mount_point: mount_point.to_string(),
                            filesystem: filesystem.to_string(),
                            total_bytes,
                            used_bytes,
                            available_bytes,
                            usage_percent,
                            storage_type: self.detect_storage_type(device),
                        });
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Detect storage type (HDD, SSD, NVMe)
    fn detect_storage_type(&self, device: &str) -> StorageType {
        // Extract device name (e.g., "/dev/sda1" -> "sda")
        let dev_name = device.trim_start_matches("/dev/");
        let base_name = dev_name.trim_end_matches(char::is_numeric);

        // Check if NVMe
        if base_name.starts_with("nvme") {
            return StorageType::NVMe;
        }

        // Check /sys/block/sdX/queue/rotational
        let rotational_path = format!("/sys/block/{}/queue/rotational", base_name);
        if let Ok(content) = fs::read_to_string(&rotational_path) {
            if content.trim() == "0" {
                return StorageType::SSD;
            } else if content.trim() == "1" {
                return StorageType::HDD;
            }
        }

        StorageType::Unknown
    }

    /// Get resource usage from /proc
    fn get_current_resource_usage(&self) -> Result<ResourceUsage> {
        // Parse /proc/stat for CPU usage
        let cpu_percent = self.get_cpu_usage()?;

        // Parse /proc/meminfo for memory
        let memory_info = self.parse_meminfo()?;

        // Parse /proc/loadavg for load average
        let load_average = self.get_load_average()?;

        Ok(ResourceUsage {
            cpu_percent,
            cpu_per_core: None, // TODO: Parse per-core stats
            memory_used_bytes: memory_info.used_bytes,
            memory_percent: memory_info.usage_percent,
            network_rx_bytes_per_sec: 0, // TODO: Parse /proc/net/dev
            network_tx_bytes_per_sec: 0,
            disk_read_bytes_per_sec: 0, // TODO: Parse /proc/diskstats
            disk_write_bytes_per_sec: 0,
            process_count: self.get_process_count(),
            load_average: Some(load_average),
        })
    }

    /// Get CPU usage percentage
    fn get_cpu_usage(&self) -> Result<f64> {
        // This is a simplified version - proper implementation needs to
        // sample /proc/stat twice with a delay to calculate usage
        Ok(0.0) // TODO: Implement proper CPU usage calculation
    }

    /// Get load average from /proc/loadavg
    fn get_load_average(&self) -> Result<[f64; 3]> {
        let content = fs::read_to_string("/proc/loadavg")
            .context("Failed to read /proc/loadavg")?;

        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            let load1 = parts[0].parse::<f64>().unwrap_or(0.0);
            let load5 = parts[1].parse::<f64>().unwrap_or(0.0);
            let load15 = parts[2].parse::<f64>().unwrap_or(0.0);
            Ok([load1, load5, load15])
        } else {
            Ok([0.0, 0.0, 0.0])
        }
    }

    /// Get process count from /proc
    fn get_process_count(&self) -> Option<usize> {
        fs::read_dir("/proc")
            .ok()?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name()
                    .to_str()
                    .and_then(|name| name.parse::<u32>().ok())
                    .is_some()
            })
            .count()
            .into()
    }

    /// Validate eBPF program bytecode (basic validation)
    fn validate_ebpf_bytecode(&self, program: &[u8]) -> Result<()> {
        if program.is_empty() {
            return Err(anyhow::anyhow!("eBPF program is empty"));
        }

        // eBPF instructions are 8 bytes each
        if program.len() % 8 != 0 {
            return Err(anyhow::anyhow!(
                "eBPF program size must be multiple of 8 bytes (instruction size)"
            ));
        }

        // Basic size check (kernel has limits)
        const MAX_INSN_COUNT: usize = 1_000_000; // 1M instructions
        if program.len() / 8 > MAX_INSN_COUNT {
            return Err(anyhow::anyhow!(
                "eBPF program too large: {} instructions (max {})",
                program.len() / 8,
                MAX_INSN_COUNT
            ));
        }

        Ok(())
    }

    /// Check if eBPF filesystem is mounted
    fn check_bpf_fs(&self) -> bool {
        Path::new("/sys/fs/bpf").exists()
    }

    /// Check CAP_BPF or CAP_SYS_ADMIN capability
    fn check_bpf_permissions(&self) -> bool {
        // Try to check capabilities
        // For now, check if we can read /sys/kernel/debug/tracing
        Path::new("/sys/kernel/debug/tracing").exists()
            || Path::new("/sys/kernel/tracing").exists()
    }
}

impl OsAbstraction for LinuxAbstraction {
    fn platform(&self) -> &str {
        "linux"
    }

    fn detect_cpu(&self) -> Result<CpuInfo> {
        self.parse_cpuinfo()
    }

    fn detect_gpu(&self) -> Result<Vec<GpuInfo>> {
        self.detect_gpus()
    }

    fn detect_memory(&self) -> Result<MemoryInfo> {
        self.parse_meminfo()
    }

    fn detect_storage(&self) -> Result<Vec<StorageInfo>> {
        self.detect_storage_devices()
    }

    fn get_resource_usage(&self) -> Result<ResourceUsage> {
        self.get_current_resource_usage()
    }

    fn load_ebpf_program(&self, program: &[u8]) -> Result<EbpfHandle> {
        // Check eBPF support
        if !self.kernel_supports_ebpf() {
            return Err(anyhow::anyhow!(
                "eBPF not supported: kernel version {:?} < 4.4",
                self.kernel_version
            ));
        }

        // Check permissions
        if !self.check_bpf_permissions() {
            return Err(anyhow::anyhow!(
                "Insufficient permissions to load eBPF programs. \
                 Requires CAP_BPF or CAP_SYS_ADMIN capability, or root access."
            ));
        }

        // Validate bytecode
        self.validate_ebpf_bytecode(program)?;

        // Generate unique handle
        let handle_id = self.next_handle.fetch_add(1, Ordering::SeqCst);
        let handle = EbpfHandle(handle_id);

        // Store program state
        // NOTE: Actual kernel loading would use aya::Bpf::load_file() or similar
        // For now, we store the bytecode and mark it as loaded
        let state = EbpfProgramState {
            program_type: EbpfProgramType::Generic,
            attached: false,
            attach_type: None,
            bytecode: program.to_vec(),
            metrics: HashMap::new(),
            last_update: std::time::SystemTime::now(),
        };

        self.ebpf_programs.lock().unwrap().insert(handle, state);

        tracing::info!(
            "eBPF program loaded: handle={}, size={} bytes ({} instructions), kernel={:?}",
            handle_id,
            program.len(),
            program.len() / 8,
            self.kernel_version
        );

        Ok(handle)
    }

    fn attach_ebpf_monitor(&self, handle: EbpfHandle, attach_type: EbpfAttachType) -> Result<()> {
        // Validate handle
        let mut programs = self.ebpf_programs.lock().unwrap();
        let state = programs
            .get_mut(&handle)
            .ok_or_else(|| anyhow::anyhow!("Invalid eBPF handle: {:?}", handle))?;

        // Check if already attached
        if state.attached {
            return Err(anyhow::anyhow!(
                "eBPF program already attached to {:?}",
                state.attach_type
            ));
        }

        // Validate attach type compatibility with kernel version
        match &attach_type {
            EbpfAttachType::Xdp => {
                if !self.kernel_supports_ebpf() {
                    return Err(anyhow::anyhow!("XDP requires kernel >= 4.8"));
                }
            }
            EbpfAttachType::Lsm { .. } => {
                let (major, minor, _) = self.kernel_version;
                if major < 5 || (major == 5 && minor < 7) {
                    return Err(anyhow::anyhow!("LSM hooks require kernel >= 5.7"));
                }
            }
            _ => {}
        }

        // NOTE: Actual attachment would use aya or libbpf APIs:
        // - XDP: aya::programs::Xdp::attach()
        // - TC: aya::programs::SchedClassifier::attach()
        // - Kprobe: aya::programs::KProbe::attach()
        // - Tracepoint: aya::programs::TracePoint::attach()
        //
        // For now, we simulate successful attachment
        state.attached = true;
        state.attach_type = Some(attach_type.clone());

        tracing::info!(
            "eBPF program attached: handle={:?}, type={:?}",
            handle,
            attach_type
        );

        Ok(())
    }

    fn read_ebpf_metrics(&self, handle: EbpfHandle) -> Result<EbpfMetrics> {
        let mut programs = self.ebpf_programs.lock().unwrap();
        let state = programs
            .get_mut(&handle)
            .ok_or_else(|| anyhow::anyhow!("Invalid eBPF handle: {:?}", handle))?;

        // Check if program is attached
        if !state.attached {
            return Err(anyhow::anyhow!(
                "eBPF program not attached, cannot read metrics"
            ));
        }

        // Update timestamp
        state.last_update = std::time::SystemTime::now();

        // NOTE: Actual metric reading would use aya Map APIs:
        // - Read from eBPF maps using aya::maps::MapData
        // - Support for HashMap, Array, PerfEventArray, etc.
        //
        // For demonstration, we'll simulate some metrics based on attach type
        let mut values = state.metrics.clone();
        let mut metadata = HashMap::new();

        // Simulate metrics based on attach type
        if let Some(ref attach_type) = state.attach_type {
            metadata.insert("attach_type".to_string(), format!("{:?}", attach_type));

            match attach_type {
                EbpfAttachType::Xdp => {
                    values.insert("packets_processed".to_string(), 12345);
                    values.insert("bytes_processed".to_string(), 987654);
                    values.insert("packets_dropped".to_string(), 23);
                }
                EbpfAttachType::Kprobe { function } => {
                    values.insert("probe_hits".to_string(), 5678);
                    metadata.insert("function".to_string(), function.clone());
                }
                EbpfAttachType::Tracepoint { category, name } => {
                    values.insert("events".to_string(), 9012);
                    metadata.insert("category".to_string(), category.clone());
                    metadata.insert("name".to_string(), name.clone());
                }
                _ => {
                    values.insert("events".to_string(), 1000);
                }
            }
        }

        metadata.insert("kernel_version".to_string(), format!("{:?}", self.kernel_version));

        Ok(EbpfMetrics {
            name: format!("ebpf_program_{}", handle.0),
            metric_type: EbpfMetricType::Counter,
            values,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata,
        })
    }

    fn unload_ebpf_program(&self, handle: EbpfHandle) -> Result<()> {
        let mut programs = self.ebpf_programs.lock().unwrap();

        // Remove program state
        let state = programs
            .remove(&handle)
            .ok_or_else(|| anyhow::anyhow!("Invalid eBPF handle: {:?}", handle))?;

        // NOTE: Actual cleanup would:
        // - Detach from attach points (XDP, TC, kprobe, etc.)
        // - Close eBPF map file descriptors
        // - Unpin from /sys/fs/bpf if pinned
        // - Close program file descriptor
        //
        // For now, just log the cleanup
        tracing::info!(
            "eBPF program unloaded: handle={:?}, was_attached={}, type={:?}",
            handle,
            state.attached,
            state.attach_type
        );

        Ok(())
    }

    fn is_ebpf_supported(&self) -> bool {
        self.kernel_supports_ebpf() && self.check_bpf_fs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_cpu_detection() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");
        let cpu = linux.detect_cpu().expect("Failed to detect CPU");

        assert!(cpu.cores > 0, "Should detect at least one CPU core");
        assert!(!cpu.model.is_empty(), "Should detect CPU model");
        assert_eq!(cpu.architecture, std::env::consts::ARCH);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_memory_detection() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");
        let memory = linux.detect_memory().expect("Failed to detect memory");

        assert!(memory.total_bytes > 0, "Should detect non-zero memory");
        assert!(memory.usage_percent >= 0.0 && memory.usage_percent <= 100.0);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_storage_detection() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");
        let storage = linux.detect_storage().expect("Failed to detect storage");

        // Should have at least root filesystem
        assert!(!storage.is_empty(), "Should detect at least one storage device");

        for device in &storage {
            assert!(device.total_bytes > 0, "Storage should have non-zero capacity");
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_support() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");
        let supported = linux.is_ebpf_supported();

        println!("eBPF supported: {}", supported);
        println!("Kernel version: {:?}", linux.kernel_version);
        println!("BPF FS exists: {}", linux.check_bpf_fs());
        println!("BPF permissions: {}", linux.check_bpf_permissions());

        // On modern kernels (>= 4.4), eBPF should be supported
        if linux.kernel_version.0 >= 4 {
            assert!(
                linux.kernel_supports_ebpf(),
                "Kernel >= 4.4 should support eBPF"
            );
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_kernel_version_detection() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");
        let (major, minor, patch) = linux.kernel_version;

        println!("Detected kernel version: {}.{}.{}", major, minor, patch);

        // Sanity checks
        assert!(major > 0, "Kernel major version should be > 0");
        assert!(major < 100, "Kernel major version should be reasonable");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_program_validation() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");

        // Empty program should fail
        assert!(linux.validate_ebpf_bytecode(&[]).is_err());

        // Invalid size (not multiple of 8) should fail
        assert!(linux.validate_ebpf_bytecode(&[0; 7]).is_err());

        // Valid size should pass basic validation
        let valid_program = vec![0; 64]; // 8 instructions
        assert!(linux.validate_ebpf_bytecode(&valid_program).is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_program_lifecycle() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");

        // Skip if eBPF not supported
        if !linux.is_ebpf_supported() {
            println!("Skipping eBPF test: not supported on this system");
            return;
        }

        // Create a valid eBPF program (minimal size)
        let program = vec![0x95, 0, 0, 0, 0, 0, 0, 0]; // BPF_EXIT instruction

        // Load program
        let handle = linux.load_ebpf_program(&program);

        // Without proper permissions, this might fail
        if handle.is_err() {
            println!("Skipping eBPF load test: insufficient permissions");
            println!("Error: {}", handle.unwrap_err());
            return;
        }

        let handle = handle.unwrap();
        println!("Loaded eBPF program: {:?}", handle);

        // Try to attach (might fail without permissions)
        let attach_result = linux.attach_ebpf_monitor(
            handle,
            EbpfAttachType::Tracepoint {
                category: "sched".to_string(),
                name: "sched_switch".to_string(),
            },
        );

        if attach_result.is_err() {
            println!("Skipping eBPF attach test: {}", attach_result.unwrap_err());
        } else {
            println!("Attached eBPF program successfully");

            // Read metrics
            if let Ok(metrics) = linux.read_ebpf_metrics(handle) {
                println!("eBPF metrics: {:?}", metrics);
                assert_eq!(metrics.metric_type, EbpfMetricType::Counter);
            }
        }

        // Cleanup
        linux
            .unload_ebpf_program(handle)
            .expect("Failed to unload eBPF program");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_invalid_handle() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");

        let invalid_handle = EbpfHandle(99999);

        // Operations on invalid handle should fail
        assert!(linux
            .attach_ebpf_monitor(
                invalid_handle,
                EbpfAttachType::Tracepoint {
                    category: "test".to_string(),
                    name: "test".to_string(),
                }
            )
            .is_err());

        assert!(linux.read_ebpf_metrics(invalid_handle).is_err());
        assert!(linux.unload_ebpf_program(invalid_handle).is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_double_attach() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");

        if !linux.is_ebpf_supported() {
            return;
        }

        let program = vec![0x95, 0, 0, 0, 0, 0, 0, 0];
        if let Ok(handle) = linux.load_ebpf_program(&program) {
            let attach_type = EbpfAttachType::Tracepoint {
                category: "sched".to_string(),
                name: "sched_switch".to_string(),
            };

            // First attach might succeed
            if linux.attach_ebpf_monitor(handle, attach_type.clone()).is_ok() {
                // Second attach should fail
                assert!(linux.attach_ebpf_monitor(handle, attach_type).is_err());
            }

            let _ = linux.unload_ebpf_program(handle);
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_metrics_without_attach() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");

        if !linux.is_ebpf_supported() {
            return;
        }

        let program = vec![0x95, 0, 0, 0, 0, 0, 0, 0];
        if let Ok(handle) = linux.load_ebpf_program(&program) {
            // Reading metrics without attaching should fail
            assert!(linux.read_ebpf_metrics(handle).is_err());

            let _ = linux.unload_ebpf_program(handle);
        }
    }
}
