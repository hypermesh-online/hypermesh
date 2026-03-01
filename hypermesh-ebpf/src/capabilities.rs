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
    /// NIC capabilities for a specific interface (populated on demand)
    pub nic_capabilities: Option<NicCapabilities>,
}

/// Hardware NIC capabilities for XDP offload
#[derive(Debug, Clone, Default)]
pub struct NicCapabilities {
    /// Network interface name
    pub interface: String,
    /// Driver name (e.g., "mlx5_core", "nfp", "ixgbe", "virtio_net")
    pub driver_name: String,
    /// Whether the NIC driver supports XDP offload (HW mode)
    pub supports_xdp_offload: bool,
    /// Whether the NIC supports hardware timestamps
    pub supports_hw_timestamp: bool,
    /// Number of hardware RX queues (for multi-queue AF_XDP)
    pub rx_queue_count: u32,
    /// Number of hardware TX queues
    pub tx_queue_count: u32,
}

impl NicCapabilities {
    /// Detect NIC capabilities for a specific interface
    pub fn detect(interface: &str) -> Self {
        let driver_name = Self::read_driver_name(interface);
        let supports_xdp_offload = Self::check_offload_support(&driver_name);
        let supports_hw_timestamp = Self::check_hw_timestamp(&driver_name);
        let (rx_queues, tx_queues) = Self::read_queue_counts(interface);

        Self {
            interface: interface.to_string(),
            driver_name,
            supports_xdp_offload,
            supports_hw_timestamp,
            rx_queue_count: rx_queues,
            tx_queue_count: tx_queues,
        }
    }

    /// Read driver name from /sys/class/net/{iface}/device/driver
    fn read_driver_name(interface: &str) -> String {
        // Read the symlink at /sys/class/net/{interface}/device/driver
        // The symlink target basename is the driver name
        let path = format!("/sys/class/net/{interface}/device/driver");
        std::fs::read_link(&path)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default()
    }

    /// Check if driver supports XDP hardware offload
    fn check_offload_support(driver_name: &str) -> bool {
        // Known XDP-offload-capable drivers:
        // - mlx5_core: Mellanox ConnectX-5/6/7 (most complete XDP HW offload)
        // - nfp: Netronome Agilio SmartNICs (programmable, partial eBPF)
        // - bnxt_en: Broadcom NetXtreme-E (limited HW offload)
        matches!(driver_name, "mlx5_core" | "nfp" | "bnxt_en")
    }

    /// Check if driver supports hardware timestamps
    fn check_hw_timestamp(driver_name: &str) -> bool {
        matches!(
            driver_name,
            "mlx5_core" | "nfp" | "bnxt_en" | "ixgbe" | "i40e" | "ice"
        )
    }

    /// Read RX/TX queue counts from /sys/class/net/{iface}/queues/
    fn read_queue_counts(interface: &str) -> (u32, u32) {
        let queues_path = format!("/sys/class/net/{interface}/queues");
        let (mut rx, mut tx) = (0u32, 0u32);

        if let Ok(entries) = std::fs::read_dir(&queues_path) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("rx-") {
                    rx += 1;
                } else if name_str.starts_with("tx-") {
                    tx += 1;
                }
            }
        }

        // Default to 1 if we can't detect (most NICs have at least 1)
        (rx.max(1), tx.max(1))
    }
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
            nic_capabilities: None,
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

    /// Detect NIC capabilities for a specific interface
    pub fn detect_nic(&mut self, interface: &str) {
        self.nic_capabilities = Some(NicCapabilities::detect(interface));
    }

    fn read_kernel_version() -> String {
        std::fs::read_to_string("/proc/version").unwrap_or_else(|_| "Unknown".to_string())
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

    // -------------------------------------------------------------------
    // NIC capability tests
    // -------------------------------------------------------------------

    #[test]
    fn test_nic_capabilities_default() {
        let nic = NicCapabilities::default();
        assert!(nic.interface.is_empty());
        assert!(nic.driver_name.is_empty());
        assert!(!nic.supports_xdp_offload);
        assert!(!nic.supports_hw_timestamp);
        assert_eq!(nic.rx_queue_count, 0);
        assert_eq!(nic.tx_queue_count, 0);
    }

    #[test]
    fn test_nic_capabilities_detect_lo() {
        // loopback is available on all Linux systems
        let nic = NicCapabilities::detect("lo");
        assert_eq!(nic.interface, "lo");
        // loopback has no PCI device driver, so driver_name is empty
        assert!(!nic.supports_xdp_offload);
        assert!(!nic.supports_hw_timestamp);
        // Queue counts default to at least 1
        assert!(nic.rx_queue_count >= 1);
        assert!(nic.tx_queue_count >= 1);
    }

    #[test]
    fn test_known_offload_drivers() {
        assert!(NicCapabilities::check_offload_support("mlx5_core"));
        assert!(NicCapabilities::check_offload_support("nfp"));
        assert!(NicCapabilities::check_offload_support("bnxt_en"));
    }

    #[test]
    fn test_non_offload_drivers() {
        assert!(!NicCapabilities::check_offload_support("virtio_net"));
        assert!(!NicCapabilities::check_offload_support("e1000"));
        assert!(!NicCapabilities::check_offload_support("ixgbe"));
        assert!(!NicCapabilities::check_offload_support("veth"));
        assert!(!NicCapabilities::check_offload_support(""));
    }

    #[test]
    fn test_hw_timestamp_drivers() {
        assert!(NicCapabilities::check_hw_timestamp("mlx5_core"));
        assert!(NicCapabilities::check_hw_timestamp("nfp"));
        assert!(NicCapabilities::check_hw_timestamp("bnxt_en"));
        assert!(NicCapabilities::check_hw_timestamp("ixgbe"));
        assert!(NicCapabilities::check_hw_timestamp("i40e"));
        assert!(NicCapabilities::check_hw_timestamp("ice"));
        assert!(!NicCapabilities::check_hw_timestamp("virtio_net"));
        assert!(!NicCapabilities::check_hw_timestamp("e1000"));
    }

    #[test]
    fn test_queue_count_defaults() {
        // Non-existent interface should return (1, 1) defaults
        let (rx, tx) = NicCapabilities::read_queue_counts("nonexistent_iface_xyz");
        assert_eq!(rx, 1);
        assert_eq!(tx, 1);
    }

    #[test]
    fn test_detect_nic_populates_capabilities() {
        let mut caps = EbpfCapabilities::detect();
        assert!(caps.nic_capabilities.is_none());
        caps.detect_nic("lo");
        assert!(caps.nic_capabilities.is_some());
        let nic = caps
            .nic_capabilities
            .expect("test: nic capabilities should be populated");
        assert_eq!(nic.interface, "lo");
    }
}
