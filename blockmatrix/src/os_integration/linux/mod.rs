// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Linux OS Abstraction - Implementation using /proc, /sys, libbpf, XDP, TC
//
// NOTE ON eBPF: The eBPF methods on this trait (load_ebpf_program,
// attach_ebpf_monitor, read_ebpf_metrics, unload_ebpf_program) are
// OS-level abstractions with SIMULATED bytecode management.  They do NOT
// delegate to the real hypermesh-ebpf orchestrator (HyperMeshEbpf).
//
// For production HyperMesh eBPF operations (packet validation, privacy
// tier enforcement, asset hash registration, PoS validation), use
// `crate::security::ebpf::EBPFSecurityManager` which delegates to the
// unified `hypermesh_ebpf::HyperMeshEbpf` crate.
//
// This simulation exists to satisfy the OsAbstraction trait interface
// for eBPF capability detection and basic program lifecycle testing
// without requiring the full HyperMesh eBPF stack.

#![allow(unsafe_code)]

mod ebpf;
mod hardware;

use super::types::*;
use super::OsAbstraction;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::sync::atomic::AtomicU64;
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
    _program_type: EbpfProgramType,
    attached: bool,
    attach_type: Option<EbpfAttachType>,
    _bytecode: Vec<u8>,
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
            .or_else(|_| unsafe {
                let mut buf: libc::utsname = std::mem::zeroed();
                if libc::uname(&mut buf) == 0 {
                    let release = std::ffi::CStr::from_ptr(buf.release.as_ptr())
                        .to_string_lossy()
                        .into_owned();
                    Ok(release)
                } else {
                    Err(std::io::Error::last_os_error())
                }
            })
            .context("Failed to read kernel version")?;

        let version_parts: Vec<&str> = version_str
            .split_whitespace()
            .find(|s| s.chars().next().map(|c| c.is_numeric()).unwrap_or(false))
            .unwrap_or("0.0.0")
            .split(&['.', '-'][..])
            .collect();

        let major = version_parts
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let minor = version_parts
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let patch = version_parts
            .get(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Ok((major, minor, patch))
    }

    /// Check if kernel supports eBPF based on version
    pub(crate) fn kernel_supports_ebpf(&self) -> bool {
        let (major, minor, _) = self.kernel_version;
        major > 4 || (major == 4 && minor >= 4)
    }

    /// Check if kernel supports advanced eBPF features
    fn _kernel_supports_btf(&self) -> bool {
        let (major, _, _) = self.kernel_version;
        major >= 5
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
        self.load_ebpf_program_impl(program)
    }

    fn attach_ebpf_monitor(&self, handle: EbpfHandle, attach_type: EbpfAttachType) -> Result<()> {
        self.attach_ebpf_monitor_impl(handle, attach_type)
    }

    fn read_ebpf_metrics(&self, handle: EbpfHandle) -> Result<EbpfMetrics> {
        self.read_ebpf_metrics_impl(handle)
    }

    fn unload_ebpf_program(&self, handle: EbpfHandle) -> Result<()> {
        self.unload_ebpf_program_impl(handle)
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

        assert!(
            !storage.is_empty(),
            "Should detect at least one storage device"
        );

        for device in &storage {
            assert!(
                device.total_bytes > 0,
                "Storage should have non-zero capacity"
            );
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_support() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");
        let supported = linux.is_ebpf_supported();

        println!("eBPF supported: {supported}");
        println!("Kernel version: {:?}", linux.kernel_version);
        println!("BPF FS exists: {}", linux.check_bpf_fs());
        println!("BPF permissions: {}", linux.check_bpf_permissions());

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

        println!("Detected kernel version: {major}.{minor}.{patch}");

        assert!(major > 0, "Kernel major version should be > 0");
        assert!(major < 100, "Kernel major version should be reasonable");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_program_validation() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");

        assert!(linux.validate_ebpf_bytecode(&[]).is_err());
        assert!(linux.validate_ebpf_bytecode(&[0; 7]).is_err());

        let valid_program = vec![0; 64];
        assert!(linux.validate_ebpf_bytecode(&valid_program).is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_program_lifecycle() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");

        if !linux.is_ebpf_supported() {
            println!("Skipping eBPF test: not supported on this system");
            return;
        }

        let program = vec![0x95, 0, 0, 0, 0, 0, 0, 0];

        let handle = linux.load_ebpf_program(&program);

        if handle.is_err() {
            println!("Skipping eBPF load test: insufficient permissions");
            println!("Error: {}", handle.unwrap_err());
            return;
        }

        let handle = handle.expect("test: load ebpf program");
        println!("Loaded eBPF program: {handle:?}");

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

            if let Ok(metrics) = linux.read_ebpf_metrics(handle) {
                println!("eBPF metrics: {metrics:?}");
                assert_eq!(metrics.metric_type, EbpfMetricType::Counter);
            }
        }

        linux
            .unload_ebpf_program(handle)
            .expect("Failed to unload eBPF program");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_invalid_handle() {
        let linux = LinuxAbstraction::new().expect("Failed to create Linux abstraction");

        let invalid_handle = EbpfHandle(99999);

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

            if linux
                .attach_ebpf_monitor(handle, attach_type.clone())
                .is_ok()
            {
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
            assert!(linux.read_ebpf_metrics(handle).is_err());

            let _ = linux.unload_ebpf_program(handle);
        }
    }
}
