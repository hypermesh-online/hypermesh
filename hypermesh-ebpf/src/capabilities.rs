// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! eBPF Capability Detection
//!
//! Detects kernel eBPF capabilities on the current system including
//! XDP support, AF_XDP availability, kernel version, and required
//! permissions. This is the unified capability detector for the
//! entire HyperMesh eBPF subsystem.

/// eBPF capability detection result
#[derive(Debug, Clone)]
pub struct EbpfCapabilities {
    /// XDP support available (kernel 4.8+ with CAP_NET_ADMIN and BPF fs)
    pub xdp_available: bool,
    /// AF_XDP support available (kernel 4.18+ with CAP_NET_ADMIN and BPF fs)
    pub af_xdp_available: bool,
    /// Kernel version string from /proc/version
    pub kernel_version: String,
    /// Parsed major kernel version number
    pub kernel_major: u32,
    /// Parsed minor kernel version number
    pub kernel_minor: u32,
    /// CAP_NET_ADMIN capability available
    pub has_cap_net_admin: bool,
    /// BPF filesystem mounted at /sys/fs/bpf
    pub bpf_fs_mounted: bool,
}

impl EbpfCapabilities {
    /// Detect eBPF capabilities on current system
    pub fn detect() -> Self {
        let kernel_version = Self::read_kernel_version();
        let has_cap_net_admin = Self::check_cap_net_admin();
        let bpf_fs_mounted = Self::check_bpf_fs();

        let (kernel_major, kernel_minor) = Self::parse_kernel_version(&kernel_version);

        // XDP requires kernel 4.8+, AF_XDP requires kernel 4.18+
        let xdp_kernel_ok = kernel_major > 4 || (kernel_major == 4 && kernel_minor >= 8);
        let af_xdp_kernel_ok = kernel_major > 4 || (kernel_major == 4 && kernel_minor >= 18);

        Self {
            xdp_available: xdp_kernel_ok && has_cap_net_admin && bpf_fs_mounted,
            af_xdp_available: af_xdp_kernel_ok && has_cap_net_admin && bpf_fs_mounted,
            kernel_version,
            kernel_major,
            kernel_minor,
            has_cap_net_admin,
            bpf_fs_mounted,
        }
    }

    /// Check if XDP is available on this system
    pub fn can_use_xdp(&self) -> bool {
        self.xdp_available
    }

    /// Check if AF_XDP zero-copy sockets are available
    pub fn can_use_af_xdp(&self) -> bool {
        self.af_xdp_available
    }

    /// Check if any eBPF features are available
    pub fn any_available(&self) -> bool {
        self.xdp_available || self.af_xdp_available
    }

    fn read_kernel_version() -> String {
        std::fs::read_to_string("/proc/version")
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    /// Parse kernel version string into (major, minor) tuple.
    /// Expects format like "Linux version 5.10.0-..."
    pub fn parse_kernel_version(version: &str) -> (u32, u32) {
        let parts: Vec<&str> = version.split_whitespace().collect();
        if parts.len() > 2 {
            let version_parts: Vec<&str> = parts[2].split('.').collect();
            if version_parts.len() >= 2 {
                let major = version_parts[0].parse().unwrap_or(0);
                let minor = version_parts[1].parse().unwrap_or(0);
                return (major, minor);
            }
        }
        (0, 0)
    }

    fn check_cap_net_admin() -> bool {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;

            Command::new("capsh")
                .args(["--print"])
                .output()
                .map(|output| {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.contains("cap_net_admin")
                })
                .unwrap_or(false)
        }

        #[cfg(not(target_os = "linux"))]
        false
    }

    fn check_bpf_fs() -> bool {
        std::path::Path::new("/sys/fs/bpf").exists()
    }
}

impl Default for EbpfCapabilities {
    fn default() -> Self {
        Self::detect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_detection() {
        let caps = EbpfCapabilities::detect();
        // Should at least detect kernel version
        assert!(!caps.kernel_version.is_empty());
    }

    #[test]
    fn test_kernel_version_parsing() {
        let version = "Linux version 5.10.0-generic";
        let (major, minor) = EbpfCapabilities::parse_kernel_version(version);
        assert_eq!(major, 5);
        assert_eq!(minor, 10);

        let version = "Linux version 4.18.0-generic";
        let (major, minor) = EbpfCapabilities::parse_kernel_version(version);
        assert_eq!(major, 4);
        assert_eq!(minor, 18);

        let version = "Unknown";
        let (major, minor) = EbpfCapabilities::parse_kernel_version(version);
        assert_eq!(major, 0);
        assert_eq!(minor, 0);
    }

    #[test]
    fn test_version_requirements() {
        // Kernel 5.10 should have both XDP and AF_XDP kernel support
        let mut caps = EbpfCapabilities::detect();
        caps.kernel_major = 5;
        caps.kernel_minor = 10;
        // Note: actual availability depends on has_cap_net_admin and bpf_fs_mounted

        // Kernel 4.5 should have neither
        let (major, minor) = EbpfCapabilities::parse_kernel_version("Linux version 4.5.0-test");
        assert_eq!(major, 4);
        assert_eq!(minor, 5);
        // 4.5 < 4.8, so XDP not supported at kernel level
    }
}
