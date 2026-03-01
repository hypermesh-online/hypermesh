// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// BSD OS Abstraction - Implementation using sysctl, pciconf, and df
//
// Targets FreeBSD, OpenBSD, and NetBSD. Hardware detection uses sysctl
// and standard Unix tools. All command invocations gracefully handle
// failures by returning sensible defaults or empty collections.
//
// eBPF is NOT supported on BSD for HyperMesh purposes (BSD bpf(4) is
// the classic packet filter, not the extended eBPF instruction set
// that Linux provides). All eBPF trait methods return Err(not supported).

use super::types::*;
use super::OsAbstraction;
use anyhow::{Context, Result};
use std::process::Command;

/// BSD OS Abstraction using sysctl, pciconf, and df
///
/// Provides hardware detection for CPU, GPU, memory, and storage on
/// FreeBSD, OpenBSD, and NetBSD. eBPF is not supported; all eBPF
/// methods return errors.
pub struct BsdAbstraction {
    _private: (),
}

impl BsdAbstraction {
    /// Create new BSD abstraction
    pub fn new() -> Result<Self> {
        Ok(Self { _private: () })
    }
}

// ---------------------------------------------------------------------------
// Command helpers
// ---------------------------------------------------------------------------

/// Run a command and return its stdout as a trimmed String.
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

/// Read a single sysctl value.
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

/// Detect CPU information via sysctl on BSD.
fn detect_cpu_bsd() -> Result<CpuInfo> {
    let cores = sysctl_u64("hw.ncpu").unwrap_or(num_cpus::get() as u64) as usize;

    // hw.model is available on FreeBSD/OpenBSD/NetBSD
    let model = sysctl_value("hw.model").unwrap_or_else(|_| "Unknown BSD CPU".to_string());

    // FreeBSD: hw.clockrate (MHz integer)
    // OpenBSD: hw.cpuspeed (MHz integer)
    let frequency_mhz = sysctl_u64("hw.clockrate")
        .or_else(|_| sysctl_u64("hw.cpuspeed"))
        .ok();

    let vendor = extract_vendor_from_model(&model);

    Ok(CpuInfo {
        cores,
        model,
        architecture: std::env::consts::ARCH.to_string(),
        frequency_mhz,
        usage_percent: None,
        vendor,
        cache_kb: None,
    })
}

/// Try to infer CPU vendor from model string.
fn extract_vendor_from_model(model: &str) -> Option<String> {
    let lower = model.to_lowercase();
    if lower.contains("intel") {
        Some("GenuineIntel".to_string())
    } else if lower.contains("amd") {
        Some("AuthenticAMD".to_string())
    } else if lower.contains("arm") || lower.contains("aarch") {
        Some("ARM".to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// GPU detection
// ---------------------------------------------------------------------------

/// Detect GPUs via `pciconf -lv` on FreeBSD, or `pcidump -v` on OpenBSD.
/// Returns an empty vec on any failure.
fn detect_gpu_bsd() -> Vec<GpuInfo> {
    // Try FreeBSD pciconf first, then OpenBSD pcidump
    if let Ok(output) = run_command("pciconf", &["-lv"]) {
        return parse_pciconf_gpus(&output);
    }
    if let Ok(output) = run_command("pcidump", &["-v"]) {
        return parse_pciconf_gpus(&output);
    }
    Vec::new()
}

/// Parse pciconf/pcidump output looking for VGA or display devices.
fn parse_pciconf_gpus(output: &str) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let mut in_vga_block = false;
    let mut current_vendor = String::from("Unknown");
    let mut current_device = String::from("Unknown GPU");
    let mut current_pci_addr: Option<String> = None;

    for line in output.lines() {
        let trimmed = line.trim();

        // Device header lines contain '@pci' (e.g. "vgapci0@pci0:1:0:0:")
        // Only match VGA/Display on these header lines, not on key-value lines
        let is_device_header = trimmed.contains("@pci") || trimmed.contains("@isa");
        let is_vga_header = is_device_header
            && (trimmed.contains("vgapci")
                || trimmed.contains("VGA")
                || trimmed.contains("Display"));

        if is_vga_header {
            // Flush previous if needed
            if in_vga_block {
                gpus.push(build_gpu_entry(
                    &current_device,
                    &current_vendor,
                    current_pci_addr.take(),
                ));
            }
            in_vga_block = true;
            current_vendor = String::from("Unknown");
            current_device = String::from("Unknown GPU");
            if let Some(addr) = extract_pci_address(trimmed) {
                current_pci_addr = Some(addr);
            }
        } else if is_device_header && in_vga_block {
            // A new non-VGA device header ends the current VGA block
            gpus.push(build_gpu_entry(
                &current_device,
                &current_vendor,
                current_pci_addr.take(),
            ));
            in_vga_block = false;
        }

        if in_vga_block && !is_device_header {
            if let Some((key, value)) = parse_pci_kv(trimmed) {
                match key {
                    "vendor" => current_vendor = value,
                    "device" => current_device = value,
                    _ => {}
                }
            }
        }
    }

    // Flush last entry
    if in_vga_block {
        gpus.push(build_gpu_entry(
            &current_device,
            &current_vendor,
            current_pci_addr,
        ));
    }

    gpus
}

/// Parse a "key = value" line from pciconf output.
fn parse_pci_kv(line: &str) -> Option<(&str, String)> {
    // pciconf uses "    vendor     = 'NVIDIA Corporation'"
    if let Some((key, value)) = line.split_once('=') {
        let key = key.trim();
        let value = value
            .trim()
            .trim_matches('\'')
            .trim_matches('"')
            .to_string();
        return Some((key, value));
    }
    None
}

/// Extract PCI address from a pciconf device line.
fn extract_pci_address(line: &str) -> Option<String> {
    if let Some(at_idx) = line.find('@') {
        let after = &line[at_idx + 1..];
        let end = after.find(':').unwrap_or(after.len());
        let addr = after[..end].trim_end_matches(':');
        if !addr.is_empty() {
            return Some(addr.to_string());
        }
    }
    None
}

/// Build a GpuInfo entry from parsed PCI fields.
fn build_gpu_entry(device: &str, vendor: &str, pci_address: Option<String>) -> GpuInfo {
    let gpu_type = if vendor.to_lowercase().contains("intel") {
        GpuType::Integrated
    } else {
        GpuType::Discrete
    };

    let mut capabilities = Vec::new();
    let vendor_lower = vendor.to_lowercase();
    if vendor_lower.contains("nvidia") || vendor_lower.contains("amd") {
        capabilities.push("OpenCL".to_string());
        capabilities.push("Vulkan".to_string());
    } else if vendor_lower.contains("intel") {
        capabilities.push("OpenCL".to_string());
    }

    GpuInfo {
        model: device.to_string(),
        vendor: vendor.to_string(),
        memory_bytes: None,
        available_bytes: None,
        gpu_type,
        capabilities,
        pci_address,
    }
}

// ---------------------------------------------------------------------------
// Memory detection
// ---------------------------------------------------------------------------

/// Detect memory via sysctl on BSD.
fn detect_memory_bsd() -> Result<MemoryInfo> {
    // hw.physmem is available on FreeBSD, OpenBSD, NetBSD
    let total_bytes = sysctl_u64("hw.physmem")
        .or_else(|_| sysctl_u64("hw.realmem"))
        .context("failed to read hw.physmem or hw.realmem")?;

    // hw.usermem gives user-available memory on some BSDs
    let available_bytes = sysctl_u64("hw.usermem").unwrap_or(total_bytes);

    let used_bytes = total_bytes.saturating_sub(available_bytes);
    let usage_percent = if total_bytes > 0 {
        (used_bytes as f64 / total_bytes as f64) * 100.0
    } else {
        0.0
    };

    // Swap info varies across BSDs
    let (swap_total, swap_used) = detect_swap_bsd();

    Ok(MemoryInfo {
        total_bytes,
        available_bytes,
        used_bytes,
        usage_percent,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_used,
    })
}

/// Detect swap usage on BSD via `swapctl -s` or `swapinfo`.
fn detect_swap_bsd() -> (Option<u64>, Option<u64>) {
    // Try swapinfo first (FreeBSD)
    if let Ok(output) = run_command("swapinfo", &["-k"]) {
        if let Some((total, used)) = parse_swapinfo_output(&output) {
            return (Some(total), Some(used));
        }
    }
    // Try swapctl (OpenBSD/NetBSD)
    if let Ok(output) = run_command("swapctl", &["-s"]) {
        if let Some((total, used)) = parse_swap_output(&output) {
            return (Some(total), Some(used));
        }
    }
    (None, None)
}

/// Parse `swapctl -s` output.
fn parse_swap_output(output: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = output.split_whitespace().collect();
    let mut total: Option<u64> = None;
    let mut used: Option<u64> = None;

    for (i, part) in parts.iter().enumerate() {
        if let Ok(val) = part.parse::<u64>() {
            if total.is_none() {
                total = Some(val * 1024); // 1k-blocks to bytes
            } else if used.is_none() && parts.get(i + 1).is_some_and(|p| p.starts_with("used")) {
                used = Some(val * 1024);
            }
        }
    }

    Some((total.unwrap_or(0), used.unwrap_or(0)))
}

/// Parse `swapinfo -k` output (FreeBSD).
fn parse_swapinfo_output(output: &str) -> Option<(u64, u64)> {
    let mut total: u64 = 0;
    let mut used: u64 = 0;

    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            total += parts[1].parse::<u64>().unwrap_or(0) * 1024;
            used += parts[2].parse::<u64>().unwrap_or(0) * 1024;
        }
    }

    if total > 0 {
        Some((total, used))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Storage detection
// ---------------------------------------------------------------------------

/// Detect storage via `df -k` on BSD.
fn detect_storage_bsd() -> Result<Vec<StorageInfo>> {
    let output = run_command("df", &["-k"])?;
    Ok(parse_df_output_bsd(&output))
}

/// Parse `df -k` output into StorageInfo entries.
fn parse_df_output_bsd(output: &str) -> Vec<StorageInfo> {
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
        let mount_point = parts[parts.len() - 1];

        let total_bytes = total_kb * 1024;
        let used_bytes = used_kb * 1024;
        let available_bytes = available_kb * 1024;
        let usage_percent = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        let filesystem = detect_bsd_filesystem(device);
        let storage_type = detect_bsd_storage_type(device);

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

/// Determine filesystem type for a BSD device.
fn detect_bsd_filesystem(device: &str) -> String {
    if device.contains("zfs") || device.contains("zpool") {
        "ZFS".to_string()
    } else {
        "UFS".to_string()
    }
}

/// Determine storage type for a BSD device.
fn detect_bsd_storage_type(device: &str) -> StorageType {
    if device.contains("nvme") || device.contains("nvd") {
        StorageType::NVMe
    } else if device.contains("ada") {
        StorageType::SSD
    } else if device.contains("da") {
        StorageType::HDD
    } else {
        StorageType::Unknown
    }
}

// ---------------------------------------------------------------------------
// Resource usage
// ---------------------------------------------------------------------------

/// Get current resource usage on BSD.
fn get_resource_usage_bsd() -> Result<ResourceUsage> {
    let memory = detect_memory_bsd().unwrap_or_default();

    let load_average = parse_load_average_bsd();

    Ok(ResourceUsage {
        cpu_usage_percent: 0.0, // Proper calculation requires two kern.cp_time samples
        memory_usage_percent: memory.usage_percent,
        load_average,
        network_rx_bytes_per_sec: None,
        network_tx_bytes_per_sec: None,
        disk_read_bytes_per_sec: None,
        disk_write_bytes_per_sec: None,
        process_count: None,
    })
}

/// Read load average via `sysctl vm.loadavg` on BSD.
fn parse_load_average_bsd() -> Option<[f64; 3]> {
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

impl OsAbstraction for BsdAbstraction {
    fn platform(&self) -> &str {
        "bsd"
    }

    fn detect_cpu(&self) -> Result<CpuInfo> {
        detect_cpu_bsd()
    }

    fn detect_gpu(&self) -> Result<Vec<GpuInfo>> {
        Ok(detect_gpu_bsd())
    }

    fn detect_memory(&self) -> Result<MemoryInfo> {
        detect_memory_bsd()
    }

    fn detect_storage(&self) -> Result<Vec<StorageInfo>> {
        detect_storage_bsd()
    }

    fn get_resource_usage(&self) -> Result<ResourceUsage> {
        get_resource_usage_bsd()
    }

    fn load_ebpf_program(&self, _program: &[u8]) -> Result<EbpfHandle> {
        Err(anyhow::anyhow!(
            "eBPF is not supported on BSD. HyperMesh eBPF requires Linux kernel >= 4.4"
        ))
    }

    fn attach_ebpf_monitor(&self, _handle: EbpfHandle, _attach_type: EbpfAttachType) -> Result<()> {
        Err(anyhow::anyhow!(
            "eBPF is not supported on BSD. HyperMesh eBPF requires Linux kernel >= 4.4"
        ))
    }

    fn read_ebpf_metrics(&self, _handle: EbpfHandle) -> Result<EbpfMetrics> {
        Err(anyhow::anyhow!(
            "eBPF is not supported on BSD. HyperMesh eBPF requires Linux kernel >= 4.4"
        ))
    }

    fn unload_ebpf_program(&self, _handle: EbpfHandle) -> Result<()> {
        Err(anyhow::anyhow!(
            "eBPF is not supported on BSD. HyperMesh eBPF requires Linux kernel >= 4.4"
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
    fn test_bsd_abstraction_creation() {
        let bsd = BsdAbstraction::new().expect("test: should create BSD abstraction");
        assert_eq!(bsd.platform(), "bsd");
    }

    #[test]
    fn test_bsd_ebpf_not_supported() {
        let bsd = BsdAbstraction::new().expect("test: should create BSD abstraction");
        assert!(!bsd.is_ebpf_supported());
    }

    #[test]
    fn test_bsd_ebpf_load_returns_error() {
        let bsd = BsdAbstraction::new().expect("test: should create BSD abstraction");
        let result = bsd.load_ebpf_program(&[0x95, 0, 0, 0, 0, 0, 0, 0]);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not supported"),
            "error should mention not supported: {msg}"
        );
    }

    #[test]
    fn test_bsd_ebpf_attach_returns_error() {
        let bsd = BsdAbstraction::new().expect("test: should create BSD abstraction");
        let result = bsd.attach_ebpf_monitor(EbpfHandle(1), EbpfAttachType::BpfFilter);
        assert!(result.is_err());
    }

    #[test]
    fn test_bsd_ebpf_read_returns_error() {
        let bsd = BsdAbstraction::new().expect("test: should create BSD abstraction");
        let result = bsd.read_ebpf_metrics(EbpfHandle(1));
        assert!(result.is_err());
    }

    #[test]
    fn test_bsd_ebpf_unload_returns_error() {
        let bsd = BsdAbstraction::new().expect("test: should create BSD abstraction");
        let result = bsd.unload_ebpf_program(EbpfHandle(1));
        assert!(result.is_err());
    }

    // --- CPU detection on BSD ---

    #[test]
    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    fn test_bsd_cpu_detection_live() {
        let bsd = BsdAbstraction::new().expect("test: BSD abstraction");
        let cpu = bsd
            .detect_cpu()
            .expect("test: CPU detection should succeed");
        assert!(cpu.cores > 0, "should detect at least one CPU core");
        assert!(!cpu.model.is_empty(), "should have a CPU model string");
    }

    // --- Memory detection on BSD ---

    #[test]
    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    fn test_bsd_memory_detection_live() {
        let bsd = BsdAbstraction::new().expect("test: BSD abstraction");
        let mem = bsd
            .detect_memory()
            .expect("test: memory detection should succeed");
        assert!(mem.total_bytes > 0, "should detect non-zero total memory");
    }

    // --- Storage detection on BSD ---

    #[test]
    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    fn test_bsd_storage_detection_live() {
        let bsd = BsdAbstraction::new().expect("test: BSD abstraction");
        let storage = bsd
            .detect_storage()
            .expect("test: storage detection should succeed");
        assert!(
            !storage.is_empty(),
            "should detect at least one storage device"
        );
    }

    // --- Parser unit tests (run on any platform) ---

    #[test]
    fn test_extract_vendor_from_model_intel() {
        assert_eq!(
            extract_vendor_from_model("Intel(R) Core(TM) i7-9700K"),
            Some("GenuineIntel".to_string())
        );
    }

    #[test]
    fn test_extract_vendor_from_model_amd() {
        assert_eq!(
            extract_vendor_from_model("AMD Ryzen 9 5950X"),
            Some("AuthenticAMD".to_string())
        );
    }

    #[test]
    fn test_extract_vendor_from_model_unknown() {
        assert_eq!(extract_vendor_from_model("SomeOtherCPU"), None);
    }

    #[test]
    fn test_parse_df_output_bsd_filters_devs() {
        let sample = "\
Filesystem  1024-blocks      Used Available Capacity  Mounted on
/dev/ada0p2   499963904 124567890 375396014    25%    /
devfs                 1         1         0   100%    /dev
/dev/ada1p1   100000000  50000000  50000000    50%    /data";

        let devices = parse_df_output_bsd(sample);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].device, "/dev/ada0p2");
        assert_eq!(devices[0].mount_point, "/");
        assert!(devices[0].total_bytes > 0);
        assert_eq!(devices[1].device, "/dev/ada1p1");
        assert_eq!(devices[1].mount_point, "/data");
    }

    #[test]
    fn test_parse_df_output_bsd_empty() {
        let sample = "Filesystem  1024-blocks  Used  Available  Capacity  Mounted on\n";
        let devices = parse_df_output_bsd(sample);
        assert!(devices.is_empty());
    }

    #[test]
    fn test_detect_bsd_storage_type() {
        assert_eq!(detect_bsd_storage_type("/dev/nvme0n1"), StorageType::NVMe);
        assert_eq!(detect_bsd_storage_type("/dev/nvd0"), StorageType::NVMe);
        assert_eq!(detect_bsd_storage_type("/dev/ada0p2"), StorageType::SSD);
        assert_eq!(detect_bsd_storage_type("/dev/da0"), StorageType::HDD);
        assert_eq!(detect_bsd_storage_type("/dev/other"), StorageType::Unknown);
    }

    #[test]
    fn test_detect_bsd_filesystem() {
        assert_eq!(detect_bsd_filesystem("/dev/ada0p2"), "UFS");
        assert_eq!(detect_bsd_filesystem("zfs/data"), "ZFS");
        assert_eq!(detect_bsd_filesystem("zpool/root"), "ZFS");
    }

    #[test]
    fn test_parse_pciconf_gpus_basic() {
        let sample = "\
vgapci0@pci0:1:0:0:    class=0x030000 rev=0xa1 hdr=0x00 vendor=0x10de device=0x2204
    vendor     = 'NVIDIA Corporation'
    device     = 'GA102 [GeForce RTX 3090]'
    class      = display
    subclass   = VGA

none0@pci0:0:0:0:    class=0x060000
    vendor     = 'Intel Corporation'
";
        let gpus = parse_pciconf_gpus(sample);
        assert_eq!(gpus.len(), 1);
        assert!(gpus[0].model.contains("RTX 3090"));
        assert!(gpus[0].vendor.contains("NVIDIA"));
    }

    #[test]
    fn test_parse_pciconf_gpus_empty() {
        let gpus = parse_pciconf_gpus("");
        assert!(gpus.is_empty());
    }

    #[test]
    fn test_parse_swapinfo_output() {
        let sample = "\
Device          1K-blocks     Used    Avail Capacity
/dev/ada0p3       2097152   524288  1572864    25%";
        let result = parse_swapinfo_output(sample);
        assert!(result.is_some());
        let (total, used) = result.expect("test: should parse swapinfo");
        assert_eq!(total, 2097152 * 1024);
        assert_eq!(used, 524288 * 1024);
    }

    #[test]
    fn test_parse_swapinfo_output_empty() {
        let sample = "Device  1K-blocks  Used  Avail  Capacity\n";
        let result = parse_swapinfo_output(sample);
        assert!(result.is_none());
    }
}
