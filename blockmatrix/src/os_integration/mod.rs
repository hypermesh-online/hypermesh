// OS Integration Layer - Cross-platform abstraction for hardware detection and eBPF monitoring
//
// This module provides a unified interface for interacting with operating system APIs
// across Linux, Windows, BSD, and macOS. It enables HyperMesh to detect hardware
// resources and monitor system performance via eBPF without OS-specific code in the core.

pub mod types;
pub mod linux;
pub mod windows;
pub mod bsd;
pub mod macos;

// Re-export types publicly
pub use types::*;
use anyhow::Result;

/// OS Abstraction trait providing unified interface for hardware detection and eBPF monitoring
///
/// Platform implementations:
/// - Linux: LinuxAbstraction (libbpf, XDP, TC, LSM hooks)
/// - Windows: WindowsAbstraction (WMI, eBpf-for-windows)
/// - BSD: BsdAbstraction (sysctl, pciconf, bpf(4))
/// - macOS: MacOsAbstraction (IOKit, system_profiler, native BPF)
pub trait OsAbstraction: Send + Sync {
    /// Get platform identifier (linux, windows, bsd, macos)
    fn platform(&self) -> &str;

    /// Hardware Detection Methods

    /// Detect CPU information (cores, frequency, architecture)
    /// - Linux: Parse /proc/cpuinfo
    /// - Windows: Query Win32_Processor via WMI
    /// - BSD: Use sysctl hw.ncpu, hw.model
    /// - macOS: Use sysctl or system_profiler SPHardwareDataType
    fn detect_cpu(&self) -> Result<CpuInfo>;

    /// Detect GPU information (models, memory, capabilities)
    /// - Linux: Parse lspci output or /sys/class/drm
    /// - Windows: Query Win32_VideoController via WMI
    /// - BSD: Use pciconf -lv
    /// - macOS: Use system_profiler SPDisplaysDataType
    fn detect_gpu(&self) -> Result<Vec<GpuInfo>>;

    /// Detect memory information (total, available, usage)
    /// - Linux: Parse /proc/meminfo
    /// - Windows: Use GlobalMemoryStatusEx API
    /// - BSD: Use sysctl hw.physmem, hw.usermem
    /// - macOS: Use sysctl hw.memsize or vm_stat
    fn detect_memory(&self) -> Result<MemoryInfo>;

    /// Detect storage devices (capacity, usage, type)
    /// - Linux: Parse /proc/mounts, use statvfs
    /// - Windows: Query Win32_LogicalDisk via WMI
    /// - BSD: Use df, mount output
    /// - macOS: Use diskutil list, df
    fn detect_storage(&self) -> Result<Vec<StorageInfo>>;

    /// Get current resource usage (real-time metrics)
    /// - Linux: Parse /proc/stat, /proc/meminfo, /proc/net/dev
    /// - Windows: Use Performance Counters API
    /// - BSD: Use sysctl kern.cp_time, kqueue
    /// - macOS: Use host_statistics, vm_stat
    fn get_resource_usage(&self) -> Result<ResourceUsage>;

    /// eBPF Integration Methods

    /// Load eBPF program into kernel
    /// - Linux: Use libbpf bpf_object__open/load
    /// - Windows: Use eBpf-for-windows API
    /// - BSD: Use bpf(4) kernel interface
    /// - macOS: Use native BPF via bpf(4)
    fn load_ebpf_program(&self, program: &[u8]) -> Result<EbpfHandle>;

    /// Attach eBPF program to monitoring point
    /// - Linux: Attach to XDP, TC, kprobe, tracepoint
    /// - Windows: Attach to eBpf-for-windows hooks
    /// - BSD: Attach to bpf filter
    /// - macOS: Attach to BPF filter
    fn attach_ebpf_monitor(&self, handle: EbpfHandle, attach_type: EbpfAttachType) -> Result<()>;

    /// Read metrics collected by eBPF program
    /// - All platforms: Read from eBPF maps
    fn read_ebpf_metrics(&self, handle: EbpfHandle) -> Result<EbpfMetrics>;

    /// Detach and unload eBPF program
    fn unload_ebpf_program(&self, handle: EbpfHandle) -> Result<()>;

    /// Check if eBPF is supported on this system
    /// - Linux: Check kernel version >= 4.4
    /// - Windows: Check if eBpf-for-windows is installed
    /// - BSD: Check for bpf(4) support
    /// - macOS: Check for BPF support
    fn is_ebpf_supported(&self) -> bool;
}

/// Factory function to create appropriate OS abstraction for current platform
pub fn create_os_abstraction() -> Result<Box<dyn OsAbstraction>> {
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::LinuxAbstraction::new()?))
    }

    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsAbstraction::new()?))
    }

    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    {
        Ok(Box::new(bsd::BsdAbstraction::new()?))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOsAbstraction::new()?))
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "windows",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "macos"
    )))]
    {
        Err(anyhow::anyhow!("Unsupported operating system"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_os_abstraction() {
        let os = create_os_abstraction();
        assert!(os.is_ok(), "Should create OS abstraction for current platform");

        let os = os.unwrap();
        let platform = os.platform();

        // Verify platform detection matches current OS
        #[cfg(target_os = "linux")]
        assert_eq!(platform, "linux");

        #[cfg(target_os = "windows")]
        assert_eq!(platform, "windows");

        #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
        assert_eq!(platform, "bsd");

        #[cfg(target_os = "macos")]
        assert_eq!(platform, "macos");
    }

    #[test]
    fn test_hardware_detection() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Test CPU detection
        let cpu = os.detect_cpu();
        assert!(cpu.is_ok(), "CPU detection should succeed");
        let cpu = cpu.unwrap();
        assert!(cpu.cores > 0, "Should detect at least one CPU core");

        // Test memory detection
        let memory = os.detect_memory();
        assert!(memory.is_ok(), "Memory detection should succeed");
        let memory = memory.unwrap();
        assert!(memory.total_bytes > 0, "Should detect non-zero memory");
    }

    #[test]
    fn test_ebpf_support() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Just check if eBPF support detection works, don't require it to be true
        let supported = os.is_ebpf_supported();

        #[cfg(target_os = "linux")]
        {
            // On Linux, eBPF should be supported on kernel 4.4+
            // We don't assert true because CI might run on old kernel
            println!("Linux eBPF support: {}", supported);
        }
    }
}
