// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// macOS OS Abstraction - Implementation using sysctl, system_profiler, vm_stat, and df
//
// Hardware detection uses sysctl and system_profiler command-line tools parsed
// from their text output. All command invocations gracefully handle failures
// (command not found, permission errors, unexpected output format) by falling
// back to sensible defaults or empty collections.
//
// eBPF/BPF is NOT supported on macOS for HyperMesh purposes (the kernel BPF
// is packet-filter only, not the full eBPF instruction set Linux provides).
// All eBPF trait methods return Err(not supported).

use super::types::*;
use super::OsAbstraction;
use anyhow::{Context, Result};
use std::process::Command;

/// macOS OS Abstraction using sysctl, system_profiler, vm_stat, and df
///
/// Provides hardware detection for CPU, GPU, memory, and storage on macOS.
/// eBPF is not supported; all eBPF methods return errors.
pub struct MacOsAbstraction {
    _private: (),
}

impl MacOsAbstraction {
    /// Create new macOS abstraction
    pub fn new() -> Result<Self> {
        Ok(Self { _private: () })
    }
}

// ---------------------------------------------------------------------------
// Command helpers
// ---------------------------------------------------------------------------

/// Run a command and return its stdout as a trimmed String.
/// Returns Err if the command cannot be found or exits with non-zero status.
fn run_command(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute `{cmd}`"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("`{cmd}` exited with {}: {stderr}", output.status);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Read a single sysctl value (e.g. `hw.ncpu`).
fn sysctl_value(key: &str) -> Result<String> {
    let raw = run_command("sysctl", &["-n", key])?;
    Ok(raw.trim().to_string())
}

/// Read a sysctl value and parse it as u64.
fn sysctl_u64(key: &str) -> Result<u64> {
    sysctl_value(key)?
        .parse::<u64>()
        .with_context(|| format!("failed to parse sysctl `{key}` as u64"))
}

// ---------------------------------------------------------------------------
// CPU detection
// ---------------------------------------------------------------------------

/// Detect CPU information via sysctl on macOS.
fn detect_cpu_macos() -> Result<CpuInfo> {
    let cores = sysctl_u64("hw.ncpu").unwrap_or(num_cpus::get() as u64) as usize;

    let model = sysctl_value("machdep.cpu.brand_string")
        .unwrap_or_else(|_| "Unknown macOS CPU".to_string());

    let frequency_mhz = sysctl_u64("hw.cpufrequency").map(|hz| hz / 1_000_000).ok();

    let vendor = sysctl_value("machdep.cpu.vendor").ok();

    let cache_kb = build_cache_info_macos();

    Ok(CpuInfo {
        cores,
        model,
        architecture: std::env::consts::ARCH.to_string(),
        frequency_mhz,
        usage_percent: None,
        vendor,
        cache_kb,
    })
}

/// Attempt to read L1/L2/L3 cache sizes from sysctl on macOS.
fn build_cache_info_macos() -> Option<CacheInfo> {
    let l1 = sysctl_u64("hw.l1dcachesize").map(|b| b / 1024).ok();
    let l2 = sysctl_u64("hw.l2cachesize").map(|b| b / 1024).ok();
    let l3 = sysctl_u64("hw.l3cachesize").map(|b| b / 1024).ok();

    if l1.is_some() || l2.is_some() || l3.is_some() {
        Some(CacheInfo {
            l1_kb: l1,
            l2_kb: l2,
            l3_kb: l3,
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// GPU detection
// ---------------------------------------------------------------------------

/// Detect GPUs via `system_profiler SPDisplaysDataType`.
/// Returns an empty vec on any failure.
fn detect_gpu_macos() -> Vec<GpuInfo> {
    let output = match run_command("system_profiler", &["SPDisplaysDataType"]) {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    parse_system_profiler_gpus(&output)
}

/// Parse system_profiler display output into GpuInfo entries.
fn parse_system_profiler_gpus(output: &str) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let mut current_model: Option<String> = None;
    let mut current_vendor = String::from("Unknown");
    let mut current_vram: Option<u64> = None;
    let mut current_type = GpuType::Discrete;
    let mut has_chipset_model = false;

    for line in output.lines() {
        let trimmed = line.trim();

        // Skip empty lines and top-level section headers (no leading whitespace)
        // GPU entries in system_profiler are indented with spaces
        let leading_spaces = line.len() - line.trim_start().len();

        // A new GPU device block starts with an indented line ending in ':'
        // that is NOT a key-value pair. Top-level headers (0-1 spaces) are
        // section labels like "Graphics/Displays:" and should be skipped.
        if trimmed.ends_with(':') && !trimmed.contains(": ") && leading_spaces >= 4 {
            // Flush previous GPU if we had one with a chipset model
            if let Some(model) = current_model.take() {
                if has_chipset_model {
                    gpus.push(build_gpu_info(
                        model,
                        current_vendor.clone(),
                        current_vram.take(),
                        current_type.clone(),
                    ));
                }
            }
            current_model = Some(trimmed.trim_end_matches(':').to_string());
            current_vendor = String::from("Unknown");
            current_vram = None;
            current_type = GpuType::Discrete;
            has_chipset_model = false;
        }

        if let Some((key, value)) = trimmed.split_once(": ") {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Vendor" => current_vendor = value.to_string(),
                "Chipset Model" => {
                    current_model = Some(value.to_string());
                    has_chipset_model = true;
                }
                "VRAM (Total)" | "VRAM (Dynamic, Max)" => {
                    current_vram = parse_vram_string(value);
                }
                "Type" => {
                    current_type = if value.contains("Integrated") {
                        GpuType::Integrated
                    } else {
                        GpuType::Discrete
                    };
                }
                _ => {}
            }
        }
    }

    // Flush last entry
    if let Some(model) = current_model.take() {
        if has_chipset_model {
            gpus.push(build_gpu_info(
                model,
                current_vendor,
                current_vram,
                current_type,
            ));
        }
    }

    gpus
}

/// Parse a VRAM string like "1536 MB" or "8 GB" into bytes.
fn parse_vram_string(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let amount: u64 = parts[0].parse().ok()?;
    let multiplier: u64 = match parts[1].to_uppercase().as_str() {
        "MB" => 1024 * 1024,
        "GB" => 1024 * 1024 * 1024,
        "TB" => 1024 * 1024 * 1024 * 1024,
        _ => 1,
    };
    Some(amount * multiplier)
}

/// Build a `GpuInfo` from parsed fields.
fn build_gpu_info(
    model: String,
    vendor: String,
    memory_bytes: Option<u64>,
    gpu_type: GpuType,
) -> GpuInfo {
    let mut capabilities = Vec::new();
    // macOS always supports Metal on modern hardware
    capabilities.push("Metal".to_string());
    let vendor_lower = vendor.to_lowercase();
    if vendor_lower.contains("amd") || vendor_lower.contains("intel") {
        capabilities.push("OpenCL".to_string());
    }

    GpuInfo {
        model,
        vendor,
        memory_bytes,
        available_bytes: None,
        gpu_type,
        capabilities,
        pci_address: None,
    }
}

// ---------------------------------------------------------------------------
// Memory detection
// ---------------------------------------------------------------------------

/// Detect memory via `sysctl hw.memsize` and `vm_stat`.
fn detect_memory_macos() -> Result<MemoryInfo> {
    let total_bytes = sysctl_u64("hw.memsize").context("failed to read hw.memsize")?;

    // Try vm_stat for usage breakdown
    let (used_bytes, available_bytes) = parse_vm_stat(total_bytes).unwrap_or((0, total_bytes));

    let usage_percent = if total_bytes > 0 {
        (used_bytes as f64 / total_bytes as f64) * 100.0
    } else {
        0.0
    };

    // Swap info
    let (swap_total_bytes, swap_used_bytes) = parse_swap_usage().unwrap_or((None, None));

    Ok(MemoryInfo {
        total_bytes,
        available_bytes,
        used_bytes,
        usage_percent,
        swap_total_bytes,
        swap_used_bytes,
    })
}

/// Parse `vm_stat` output to determine used and available bytes.
/// Returns `(used_bytes, available_bytes)`.
fn parse_vm_stat(total_bytes: u64) -> Result<(u64, u64)> {
    let output = run_command("vm_stat", &[])?;

    // First line contains page size: "Mach Virtual Memory Statistics: (page size of 16384 bytes)"
    let page_size = parse_page_size(&output).unwrap_or(4096);

    let mut free_pages: u64 = 0;
    let mut inactive_pages: u64 = 0;
    let mut speculative_pages: u64 = 0;

    for line in output.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_end_matches('.');
            if let Ok(pages) = value.parse::<u64>() {
                match key {
                    "Pages free" => free_pages = pages,
                    "Pages inactive" => inactive_pages = pages,
                    "Pages speculative" => speculative_pages = pages,
                    _ => {}
                }
            }
        }
    }

    let available_bytes = (free_pages + inactive_pages + speculative_pages) * page_size;
    let used_bytes = total_bytes.saturating_sub(available_bytes);

    Ok((used_bytes, available_bytes))
}

/// Extract page size from vm_stat header line.
fn parse_page_size(output: &str) -> Option<u64> {
    // "Mach Virtual Memory Statistics: (page size of 16384 bytes)"
    let first = output.lines().next()?;
    let start = first.find("page size of ")? + "page size of ".len();
    let rest = &first[start..];
    let end = rest.find(' ')?;
    rest[..end].parse::<u64>().ok()
}

/// Parse swap usage from `sysctl vm.swapusage` output.
/// Returns `(Option<total_bytes>, Option<used_bytes>)`.
fn parse_swap_usage() -> Option<(Option<u64>, Option<u64>)> {
    let output = run_command("sysctl", &["-n", "vm.swapusage"]).ok()?;
    // Format: "total = 2048.00M  used = 512.00M  free = 1536.00M"
    let mut total: Option<u64> = None;
    let mut used: Option<u64> = None;

    let parts: Vec<&str> = output.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "=" {
            if let Some(val_str) = parts.get(i + 1) {
                if let Some(bytes) = parse_swap_value(val_str) {
                    // Determine if this is total, used, or free based on preceding word
                    if let Some(label) = parts.get(i.wrapping_sub(1)) {
                        match *label {
                            "total" => total = Some(bytes),
                            "used" => used = Some(bytes),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Some((total, used))
}

/// Parse a swap value like "2048.00M" into bytes.
fn parse_swap_value(s: &str) -> Option<u64> {
    let (num_str, suffix) = if let Some(n) = s.strip_suffix('M') {
        (n, 1024u64 * 1024)
    } else if let Some(n) = s.strip_suffix('G') {
        (n, 1024u64 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix('K') {
        (n, 1024u64)
    } else {
        return s.parse::<u64>().ok();
    };

    let val: f64 = num_str.parse().ok()?;
    Some((val * suffix as f64) as u64)
}

// ---------------------------------------------------------------------------
// Storage detection
// ---------------------------------------------------------------------------

/// Detect storage via `df -k`.
fn detect_storage_macos() -> Result<Vec<StorageInfo>> {
    let output = run_command("df", &["-k"])?;
    Ok(parse_df_output(&output))
}

/// Parse `df -k` output into StorageInfo entries.
/// Filters to real devices (paths starting with `/dev/`).
fn parse_df_output(output: &str) -> Vec<StorageInfo> {
    let mut devices = Vec::new();

    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }

        let device = parts[0];
        if !device.starts_with("/dev/") {
            continue;
        }

        let total_kb: u64 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let used_kb: u64 = parts[2].parse().unwrap_or(0);
        let available_kb: u64 = parts[3].parse().unwrap_or(0);
        // mount point is the last field
        let mount_point = parts[parts.len() - 1];

        let total_bytes = total_kb * 1024;
        let used_bytes = used_kb * 1024;
        let available_bytes = available_kb * 1024;
        let usage_percent = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        let filesystem = detect_macos_filesystem(device);
        let storage_type = detect_macos_storage_type(device);

        devices.push(StorageInfo {
            device: device.to_string(),
            mount_point: mount_point.to_string(),
            filesystem,
            total_bytes,
            used_bytes,
            available_bytes,
            usage_percent,
            storage_type,
        });
    }

    devices
}

/// Determine filesystem type for a macOS device.
/// Falls back to "APFS" (default on modern macOS).
fn detect_macos_filesystem(_device: &str) -> String {
    "APFS".to_string()
}

/// Determine storage type for a macOS device.
fn detect_macos_storage_type(device: &str) -> StorageType {
    if device.contains("nvme") {
        StorageType::NVMe
    } else if device.contains("disk") {
        // Modern Macs (2015+) use NVMe SSDs; safely assume SSD
        StorageType::SSD
    } else {
        StorageType::Unknown
    }
}

// ---------------------------------------------------------------------------
// Resource usage
// ---------------------------------------------------------------------------

/// Get current resource usage on macOS.
fn get_resource_usage_macos() -> Result<ResourceUsage> {
    let memory = detect_memory_macos().unwrap_or_default();

    let load_average = parse_load_average_macos();

    Ok(ResourceUsage {
        cpu_usage_percent: 0.0, // Proper calculation requires two samples over time
        memory_usage_percent: memory.usage_percent,
        load_average,
        network_rx_bytes_per_sec: None,
        network_tx_bytes_per_sec: None,
        disk_read_bytes_per_sec: None,
        disk_write_bytes_per_sec: None,
        process_count: None,
    })
}

/// Read load average via `sysctl vm.loadavg`.
fn parse_load_average_macos() -> Option<[f64; 3]> {
    // vm.loadavg returns something like "{ 1.23 0.45 0.67 }"
    let raw = sysctl_value("vm.loadavg").ok()?;
    let cleaned = raw.trim_matches(|c: char| c == '{' || c == '}' || c.is_whitespace());
    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    if parts.len() >= 3 {
        let l1 = parts[0].parse::<f64>().ok()?;
        let l5 = parts[1].parse::<f64>().ok()?;
        let l15 = parts[2].parse::<f64>().ok()?;
        Some([l1, l5, l15])
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// OsAbstraction trait implementation
// ---------------------------------------------------------------------------

impl OsAbstraction for MacOsAbstraction {
    fn platform(&self) -> &str {
        "macos"
    }

    fn detect_cpu(&self) -> Result<CpuInfo> {
        detect_cpu_macos()
    }

    fn detect_gpu(&self) -> Result<Vec<GpuInfo>> {
        Ok(detect_gpu_macos())
    }

    fn detect_memory(&self) -> Result<MemoryInfo> {
        detect_memory_macos()
    }

    fn detect_storage(&self) -> Result<Vec<StorageInfo>> {
        detect_storage_macos()
    }

    fn get_resource_usage(&self) -> Result<ResourceUsage> {
        get_resource_usage_macos()
    }

    fn load_ebpf_program(&self, _program: &[u8]) -> Result<EbpfHandle> {
        Err(anyhow::anyhow!(
            "eBPF is not supported on macOS. HyperMesh eBPF requires Linux kernel >= 4.4"
        ))
    }

    fn attach_ebpf_monitor(&self, _handle: EbpfHandle, _attach_type: EbpfAttachType) -> Result<()> {
        Err(anyhow::anyhow!(
            "eBPF is not supported on macOS. HyperMesh eBPF requires Linux kernel >= 4.4"
        ))
    }

    fn read_ebpf_metrics(&self, _handle: EbpfHandle) -> Result<EbpfMetrics> {
        Err(anyhow::anyhow!(
            "eBPF is not supported on macOS. HyperMesh eBPF requires Linux kernel >= 4.4"
        ))
    }

    fn unload_ebpf_program(&self, _handle: EbpfHandle) -> Result<()> {
        Err(anyhow::anyhow!(
            "eBPF is not supported on macOS. HyperMesh eBPF requires Linux kernel >= 4.4"
        ))
    }

    fn is_ebpf_supported(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_abstraction_creation() {
        let macos = MacOsAbstraction::new().expect("test: should create macOS abstraction");
        assert_eq!(macos.platform(), "macos");
    }

    #[test]
    fn test_macos_ebpf_not_supported() {
        let macos = MacOsAbstraction::new().expect("test: should create macOS abstraction");
        assert!(!macos.is_ebpf_supported());
    }

    #[test]
    fn test_macos_ebpf_load_returns_error() {
        let macos = MacOsAbstraction::new().expect("test: should create macOS abstraction");
        let result = macos.load_ebpf_program(&[0x95, 0, 0, 0, 0, 0, 0, 0]);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not supported"),
            "error should mention not supported: {msg}"
        );
    }

    #[test]
    fn test_macos_ebpf_attach_returns_error() {
        let macos = MacOsAbstraction::new().expect("test: should create macOS abstraction");
        let result = macos.attach_ebpf_monitor(EbpfHandle(1), EbpfAttachType::BpfFilter);
        assert!(result.is_err());
    }

    #[test]
    fn test_macos_ebpf_read_returns_error() {
        let macos = MacOsAbstraction::new().expect("test: should create macOS abstraction");
        let result = macos.read_ebpf_metrics(EbpfHandle(1));
        assert!(result.is_err());
    }

    #[test]
    fn test_macos_ebpf_unload_returns_error() {
        let macos = MacOsAbstraction::new().expect("test: should create macOS abstraction");
        let result = macos.unload_ebpf_program(EbpfHandle(1));
        assert!(result.is_err());
    }

    // --- CPU detection on macOS ---

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_cpu_detection_live() {
        let macos = MacOsAbstraction::new().expect("test: macOS abstraction");
        let cpu = macos
            .detect_cpu()
            .expect("test: CPU detection should succeed");
        assert!(cpu.cores > 0, "should detect at least one CPU core");
        assert!(!cpu.model.is_empty(), "should have a CPU model string");
    }

    // --- Memory detection on macOS ---

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_memory_detection_live() {
        let macos = MacOsAbstraction::new().expect("test: macOS abstraction");
        let mem = macos
            .detect_memory()
            .expect("test: memory detection should succeed");
        assert!(mem.total_bytes > 0, "should detect non-zero total memory");
        assert!(mem.usage_percent >= 0.0 && mem.usage_percent <= 100.0);
    }

    // --- Storage detection on macOS ---

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_storage_detection_live() {
        let macos = MacOsAbstraction::new().expect("test: macOS abstraction");
        let storage = macos
            .detect_storage()
            .expect("test: storage detection should succeed");
        assert!(
            !storage.is_empty(),
            "should detect at least one storage device"
        );
    }

    // --- Parser unit tests (run on any platform) ---

    #[test]
    fn test_parse_vram_string_mb() {
        assert_eq!(parse_vram_string("1536 MB"), Some(1536 * 1024 * 1024));
    }

    #[test]
    fn test_parse_vram_string_gb() {
        assert_eq!(parse_vram_string("8 GB"), Some(8 * 1024 * 1024 * 1024));
    }

    #[test]
    fn test_parse_vram_string_invalid() {
        assert_eq!(parse_vram_string("invalid"), None);
        assert_eq!(parse_vram_string(""), None);
    }

    #[test]
    fn test_parse_df_output_filters_devs() {
        let sample = "\
Filesystem  1024-blocks      Used Available Capacity  Mounted on
/dev/disk1s1  499963904 124567890 375396014    25%    /
devfs               380       380         0   100%    /dev
/dev/disk1s4  499963904    204800 375396014     0%    /private/var/vm";

        let devices = parse_df_output(sample);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].device, "/dev/disk1s1");
        assert_eq!(devices[0].mount_point, "/");
        assert!(devices[0].total_bytes > 0);
        assert_eq!(devices[1].device, "/dev/disk1s4");
    }

    #[test]
    fn test_parse_df_output_empty() {
        let sample = "Filesystem  1024-blocks  Used  Available  Capacity  Mounted on\n";
        let devices = parse_df_output(sample);
        assert!(devices.is_empty());
    }

    #[test]
    fn test_parse_system_profiler_gpus_basic() {
        let sample = "\
Graphics/Displays:

    Apple M1 Pro:

      Chipset Model: Apple M1 Pro
      Type: GPU
      Bus: Built-In
      Total Number of Cores: 16
      Vendor: Apple (0x106b)
      Metal Support: Metal 3
";
        let gpus = parse_system_profiler_gpus(sample);
        assert_eq!(gpus.len(), 1);
        assert!(gpus[0].model.contains("Apple M1 Pro"));
    }

    #[test]
    fn test_parse_system_profiler_gpus_empty() {
        let gpus = parse_system_profiler_gpus("");
        assert!(gpus.is_empty());
    }

    #[test]
    fn test_parse_page_size() {
        let header =
            "Mach Virtual Memory Statistics: (page size of 16384 bytes)\nPages free: 100\n";
        assert_eq!(parse_page_size(header), Some(16384));
    }

    #[test]
    fn test_parse_page_size_4096() {
        let header = "Mach Virtual Memory Statistics: (page size of 4096 bytes)\n";
        assert_eq!(parse_page_size(header), Some(4096));
    }

    #[test]
    fn test_parse_page_size_invalid() {
        assert_eq!(parse_page_size("no page size here"), None);
    }

    #[test]
    fn test_detect_macos_storage_type() {
        assert_eq!(detect_macos_storage_type("/dev/nvme0n1"), StorageType::NVMe);
        assert_eq!(detect_macos_storage_type("/dev/disk1s1"), StorageType::SSD);
        assert_eq!(
            detect_macos_storage_type("/dev/other"),
            StorageType::Unknown
        );
    }

    #[test]
    fn test_parse_swap_value() {
        assert_eq!(parse_swap_value("2048.00M"), Some(2048 * 1024 * 1024));
        assert_eq!(parse_swap_value("1.00G"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_swap_value("512.00K"), Some(512 * 1024));
        assert!(parse_swap_value("invalid").is_none());
    }
}
