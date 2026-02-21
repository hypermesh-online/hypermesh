// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Platform Info - Unified cross-platform system information
//
// Provides a single `PlatformInfo` struct that gathers OS name, version,
// architecture, hostname, CPU count, total memory, and eBPF support from
// the appropriate `OsAbstraction` implementation. Works on Linux, macOS,
// BSD, and Windows without caller needing to know which platform is active.

use super::{create_os_abstraction, OsAbstraction};
use anyhow::{Context, Result};
use std::fmt;

/// Unified platform information gathered from the OS abstraction layer.
///
/// Use `PlatformInfo::detect()` to auto-detect the current platform.
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    /// Operating system name (e.g. "linux", "macos", "bsd", "windows")
    pub os_name: String,

    /// Operating system version string (e.g. "6.18.7-arch1-1", "14.4.1")
    pub os_version: String,

    /// CPU architecture (e.g. "x86_64", "aarch64")
    pub arch: String,

    /// Machine hostname
    pub hostname: String,

    /// Number of logical CPU cores
    pub cpu_count: usize,

    /// Total physical memory in bytes
    pub total_memory_bytes: u64,

    /// Whether eBPF is supported on this platform
    pub ebpf_supported: bool,
}

impl PlatformInfo {
    /// Detect platform information using the OS abstraction layer.
    ///
    /// Creates the appropriate `OsAbstraction` for the current OS, then
    /// queries it for CPU, memory, and eBPF support. Hostname and OS
    /// version are read from standard sources. All detection steps
    /// gracefully handle failures with sensible defaults.
    pub fn detect() -> Result<Self> {
        let os = create_os_abstraction()
            .context("failed to create OS abstraction for platform detection")?;

        Self::from_abstraction(os.as_ref())
    }

    /// Build PlatformInfo from an existing OsAbstraction instance.
    ///
    /// Useful when you already hold an `OsAbstraction` and want to avoid
    /// creating a second one.
    pub fn from_abstraction(os: &dyn OsAbstraction) -> Result<Self> {
        let os_name = os.platform().to_string();
        let os_version = detect_os_version();
        let arch = std::env::consts::ARCH.to_string();
        let hostname = detect_hostname();

        let cpu_count = os
            .detect_cpu()
            .map(|c| c.cores)
            .unwrap_or_else(|_| num_cpus::get());

        let total_memory_bytes = os
            .detect_memory()
            .map(|m| m.total_bytes)
            .unwrap_or(0);

        let ebpf_supported = os.is_ebpf_supported();

        Ok(Self {
            os_name,
            os_version,
            arch,
            hostname,
            cpu_count,
            total_memory_bytes,
            ebpf_supported,
        })
    }

    /// Format total memory as a human-readable string (e.g. "16.0 GB").
    pub fn memory_display(&self) -> String {
        format_bytes(self.total_memory_bytes)
    }
}

impl fmt::Display for PlatformInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Platform: {} ({})", self.os_name, self.arch)?;
        writeln!(f, "Version:  {}", self.os_version)?;
        writeln!(f, "Hostname: {}", self.hostname)?;
        writeln!(f, "CPUs:     {}", self.cpu_count)?;
        writeln!(f, "Memory:   {}", self.memory_display())?;
        write!(f, "eBPF:     {}", if self.ebpf_supported { "supported" } else { "not supported" })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Detect OS version string in a platform-independent way.
fn detect_os_version() -> String {
    // Try /proc/version on Linux
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/version") {
            if let Some(ver) = content.split_whitespace().nth(2) {
                return ver.to_string();
            }
        }
    }

    // Use uname on Unix-like systems (macOS, BSD, Linux fallback)
    #[cfg(unix)]
    {
        if let Ok(output) = std::process::Command::new("uname").arg("-r").output() {
            if output.status.success() {
                let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !ver.is_empty() {
                    return ver;
                }
            }
        }
    }

    "unknown".to_string()
}

/// Detect machine hostname.
fn detect_hostname() -> String {
    if let Ok(output) = std::process::Command::new("hostname").output() {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }

    "unknown".to_string()
}

/// Format a byte count as a human-readable string.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_info_detect() {
        let info = PlatformInfo::detect().expect("test: platform detection should succeed");

        assert!(!info.os_name.is_empty(), "os_name should not be empty");
        assert!(!info.arch.is_empty(), "arch should not be empty");
        assert!(info.cpu_count > 0, "should detect at least one CPU");
        #[cfg(target_os = "linux")]
        assert!(info.total_memory_bytes > 0, "should detect non-zero memory on Linux");
    }

    #[test]
    fn test_platform_info_display() {
        let info = PlatformInfo {
            os_name: "linux".to_string(),
            os_version: "6.18.7-arch1-1".to_string(),
            arch: "x86_64".to_string(),
            hostname: "testhost".to_string(),
            cpu_count: 8,
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            ebpf_supported: true,
        };

        let display = format!("{info}");
        assert!(display.contains("linux"));
        assert!(display.contains("x86_64"));
        assert!(display.contains("6.18.7"));
        assert!(display.contains("testhost"));
        assert!(display.contains("8"));
        assert!(display.contains("16.0 GB"));
        assert!(display.contains("supported"));
    }

    #[test]
    fn test_platform_info_display_no_ebpf() {
        let info = PlatformInfo {
            os_name: "macos".to_string(),
            os_version: "14.4.1".to_string(),
            arch: "aarch64".to_string(),
            hostname: "macbook".to_string(),
            cpu_count: 10,
            total_memory_bytes: 32 * 1024 * 1024 * 1024,
            ebpf_supported: false,
        };

        let display = format!("{info}");
        assert!(display.contains("not supported"));
    }

    #[test]
    fn test_format_bytes_tb() {
        assert_eq!(format_bytes(2 * 1024 * 1024 * 1024 * 1024), "2.0 TB");
    }

    #[test]
    fn test_format_bytes_gb() {
        assert_eq!(format_bytes(16 * 1024 * 1024 * 1024), "16.0 GB");
    }

    #[test]
    fn test_format_bytes_mb() {
        assert_eq!(format_bytes(512 * 1024 * 1024), "512.0 MB");
    }

    #[test]
    fn test_format_bytes_kb() {
        assert_eq!(format_bytes(64 * 1024), "64.0 KB");
    }

    #[test]
    fn test_format_bytes_small() {
        assert_eq!(format_bytes(42), "42 B");
    }

    #[test]
    fn test_format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn test_memory_display() {
        let info = PlatformInfo {
            os_name: "linux".to_string(),
            os_version: "6.0.0".to_string(),
            arch: "x86_64".to_string(),
            hostname: "test".to_string(),
            cpu_count: 4,
            total_memory_bytes: 8 * 1024 * 1024 * 1024,
            ebpf_supported: false,
        };
        assert_eq!(info.memory_display(), "8.0 GB");
    }

    #[test]
    fn test_detect_hostname() {
        let hostname = detect_hostname();
        assert!(!hostname.is_empty(), "hostname should not be empty");
        assert_ne!(hostname, "unknown", "hostname should be detected");
    }

    #[test]
    fn test_detect_os_version() {
        let version = detect_os_version();
        assert!(!version.is_empty(), "os version should not be empty");
        #[cfg(target_os = "linux")]
        assert_ne!(version, "unknown", "Linux should detect OS version");
    }

    #[test]
    fn test_platform_info_from_abstraction() {
        let os = create_os_abstraction().expect("test: should create OS abstraction");
        let info = PlatformInfo::from_abstraction(os.as_ref())
            .expect("test: from_abstraction should succeed");

        assert!(!info.os_name.is_empty());
        assert!(info.cpu_count > 0);
    }

    #[test]
    fn test_factory_returns_correct_platform() {
        let os = create_os_abstraction().expect("test: should create OS abstraction");

        #[cfg(target_os = "linux")]
        assert_eq!(os.platform(), "linux");

        #[cfg(target_os = "macos")]
        assert_eq!(os.platform(), "macos");

        #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
        assert_eq!(os.platform(), "bsd");

        #[cfg(target_os = "windows")]
        assert_eq!(os.platform(), "windows");
    }
}
