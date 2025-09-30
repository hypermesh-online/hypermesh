//! eBPF Transport Acceleration for STOQ
//!
//! Provides kernel-level optimizations for the STOQ transport layer including:
//! - XDP packet filtering for early classification
//! - AF_XDP zero-copy sockets
//! - Transport-level metrics collection in kernel
//! - Connection tracking and load balancing

#[cfg(feature = "ebpf")]
pub mod xdp;
#[cfg(feature = "ebpf")]
pub mod af_xdp;
#[cfg(feature = "ebpf")]
pub mod metrics;
#[cfg(feature = "ebpf")]
pub mod loader;

use anyhow::{Result, anyhow};
use std::sync::Arc;
use parking_lot::RwLock;

/// eBPF capability detection result
#[derive(Debug, Clone)]
pub struct EbpfCapabilities {
    /// XDP support available
    pub xdp_available: bool,
    /// AF_XDP support available
    pub af_xdp_available: bool,
    /// Kernel version string
    pub kernel_version: String,
    /// CAP_NET_ADMIN available
    pub has_cap_net_admin: bool,
    /// BPF filesystem mounted
    pub bpf_fs_mounted: bool,
}

impl EbpfCapabilities {
    /// Detect eBPF capabilities on current system
    pub fn detect() -> Self {
        let kernel_version = Self::get_kernel_version();
        let has_cap_net_admin = Self::check_cap_net_admin();
        let bpf_fs_mounted = Self::check_bpf_fs();

        // XDP requires kernel 4.8+, AF_XDP requires kernel 4.18+
        let (major, minor) = Self::parse_kernel_version(&kernel_version);
        let xdp_available = major > 4 || (major == 4 && minor >= 8);
        let af_xdp_available = major > 4 || (major == 4 && minor >= 18);

        Self {
            xdp_available: xdp_available && has_cap_net_admin && bpf_fs_mounted,
            af_xdp_available: af_xdp_available && has_cap_net_admin && bpf_fs_mounted,
            kernel_version,
            has_cap_net_admin,
            bpf_fs_mounted,
        }
    }

    fn get_kernel_version() -> String {
        std::fs::read_to_string("/proc/version")
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    fn parse_kernel_version(version: &str) -> (u32, u32) {
        // Parse version like "Linux version 5.10.0-..."
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
        // Check if we have CAP_NET_ADMIN capability
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;

            Command::new("capsh")
                .args(&["--print"])
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

/// eBPF transport acceleration manager
pub struct EbpfTransport {
    /// Detected capabilities
    pub capabilities: EbpfCapabilities,
    /// XDP program manager (if available)
    #[cfg(feature = "ebpf")]
    xdp_manager: Option<Arc<RwLock<xdp::XdpManager>>>,
    /// AF_XDP socket manager (if available)
    #[cfg(feature = "ebpf")]
    af_xdp_manager: Option<Arc<RwLock<af_xdp::AfXdpManager>>>,
    /// eBPF metrics collector
    #[cfg(feature = "ebpf")]
    metrics_collector: Option<Arc<metrics::EbpfMetricsCollector>>,
    /// Whether eBPF is enabled
    enabled: bool,
}

impl EbpfTransport {
    /// Create new eBPF transport with capability detection
    pub fn new() -> Result<Self> {
        let capabilities = EbpfCapabilities::detect();

        tracing::info!("eBPF capabilities detected: {:?}", capabilities);

        if !capabilities.has_cap_net_admin {
            tracing::warn!("CAP_NET_ADMIN not available, eBPF features disabled");
            tracing::warn!("To enable eBPF, run with: sudo setcap cap_net_admin+ep <binary>");
        }

        #[cfg(not(feature = "ebpf"))]
        {
            tracing::info!("eBPF feature not compiled in, using standard transport");
            return Ok(Self {
                capabilities,
                enabled: false,
            });
        }

        #[cfg(feature = "ebpf")]
        {
            let mut xdp_manager = None;
            let mut af_xdp_manager = None;
            let mut metrics_collector = None;

            // Initialize XDP if available
            if capabilities.xdp_available {
                match xdp::XdpManager::new() {
                    Ok(mgr) => {
                        xdp_manager = Some(Arc::new(RwLock::new(mgr)));
                        tracing::info!("XDP packet filtering initialized");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize XDP: {}", e);
                    }
                }
            }

            // Initialize AF_XDP if available
            if capabilities.af_xdp_available {
                match af_xdp::AfXdpManager::new() {
                    Ok(mgr) => {
                        af_xdp_manager = Some(Arc::new(RwLock::new(mgr)));
                        tracing::info!("AF_XDP zero-copy sockets initialized");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize AF_XDP: {}", e);
                    }
                }
            }

            // Initialize metrics collector if any eBPF feature is available
            if xdp_manager.is_some() || af_xdp_manager.is_some() {
                match metrics::EbpfMetricsCollector::new() {
                    Ok(collector) => {
                        metrics_collector = Some(Arc::new(collector));
                        tracing::info!("eBPF metrics collection initialized");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize eBPF metrics: {}", e);
                    }
                }
            }

            let enabled = xdp_manager.is_some() || af_xdp_manager.is_some();

            Ok(Self {
                capabilities,
                xdp_manager,
                af_xdp_manager,
                metrics_collector,
                enabled,
            })
        }
    }

    /// Check if eBPF acceleration is available
    pub fn is_available(&self) -> bool {
        self.enabled
    }

    /// Get current eBPF metrics
    #[cfg(feature = "ebpf")]
    pub fn get_metrics(&self) -> Option<metrics::EbpfMetrics> {
        self.metrics_collector.as_ref().map(|c| c.collect())
    }

    #[cfg(not(feature = "ebpf"))]
    pub fn get_metrics(&self) -> Option<()> {
        None
    }

    /// Attach XDP program to interface
    #[cfg(feature = "ebpf")]
    pub fn attach_xdp(&self, interface: &str) -> Result<()> {
        if let Some(xdp) = &self.xdp_manager {
            xdp.write().attach(interface)?;
            tracing::info!("XDP program attached to interface {}", interface);
            Ok(())
        } else {
            Err(anyhow!("XDP not available"))
        }
    }

    #[cfg(not(feature = "ebpf"))]
    pub fn attach_xdp(&self, _interface: &str) -> Result<()> {
        Err(anyhow!("eBPF feature not compiled"))
    }

    /// Create AF_XDP socket for zero-copy
    #[cfg(feature = "ebpf")]
    pub fn create_af_xdp_socket(&self, interface: &str, queue_id: u32) -> Result<af_xdp::AfXdpSocket> {
        if let Some(mgr) = &self.af_xdp_manager {
            mgr.write().create_socket(interface, queue_id)
        } else {
            Err(anyhow!("AF_XDP not available"))
        }
    }

    #[cfg(not(feature = "ebpf"))]
    pub fn create_af_xdp_socket(&self, _interface: &str, _queue_id: u32) -> Result<()> {
        Err(anyhow!("eBPF feature not compiled"))
    }

    /// Cleanup and detach all eBPF programs
    pub fn cleanup(&self) -> Result<()> {
        #[cfg(feature = "ebpf")]
        {
            if let Some(xdp) = &self.xdp_manager {
                xdp.write().detach_all()?;
            }

            if let Some(af_xdp) = &self.af_xdp_manager {
                af_xdp.write().close_all()?;
            }

            tracing::info!("eBPF cleanup completed");
        }

        Ok(())
    }
}

impl Drop for EbpfTransport {
    fn drop(&mut self) {
        if let Err(e) = self.cleanup() {
            tracing::error!("Failed to cleanup eBPF: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_detection() {
        let caps = EbpfCapabilities::detect();
        println!("Detected capabilities: {:?}", caps);

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
    }
}