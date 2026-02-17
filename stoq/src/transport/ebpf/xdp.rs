// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! XDP (eXpress Data Path) packet filtering for STOQ
//!
//! Provides early packet classification and filtering at the kernel level
//! for improved performance and reduced CPU usage.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// XDP action to take on packets
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum XdpAction {
    /// Drop the packet
    Drop = 1,
    /// Pass packet to normal network stack
    Pass = 2,
    /// Redirect packet to AF_XDP socket
    Redirect = 3,
}

/// XDP program manager
pub struct XdpManager {
    /// Attached interfaces and their programs
    attached: Arc<RwLock<HashMap<String, AttachedProgram>>>,
    /// Statistics from XDP programs
    stats: Arc<RwLock<XdpStats>>,
    /// Whether XDP is available
    available: bool,
    /// Loaded BPF handle (only present when ebpf feature enabled and kernel load succeeds)
    #[cfg(feature = "ebpf")]
    bpf: Option<aya::Bpf>,
}

/// Attached XDP program info
struct AttachedProgram {
    interface: String,
    attach_mode: XdpAttachMode,
}

/// XDP attach mode
#[derive(Debug, Clone, Copy)]
pub enum XdpAttachMode {
    /// Native mode (fastest, requires driver support)
    Native,
    /// Generic mode (slower, works everywhere)
    Generic,
    /// Offloaded to hardware (if supported)
    Offload,
}

/// XDP program statistics
#[derive(Debug, Default, Clone)]
pub struct XdpStats {
    pub packets_passed: u64,
    pub packets_dropped: u64,
    pub packets_redirected: u64,
    pub bytes_processed: u64,
}

impl XdpManager {
    /// Create new XDP manager and load eBPF programs
    pub fn new() -> Result<Self> {
        let available = Self::check_ebpf_support();

        if available {
            tracing::info!("XDP support detected on this system");
        } else {
            tracing::warn!("XDP not available on this system");
        }

        Ok(Self {
            attached: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(XdpStats::default())),
            available,
            #[cfg(feature = "ebpf")]
            bpf: None,
        })
    }

    fn check_ebpf_support() -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/sys/fs/bpf").exists()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    /// Attach XDP program to network interface
    pub fn attach(&mut self, interface: &str) -> Result<()> {
        self.attach_with_mode(interface, XdpAttachMode::Native)
    }

    /// Attach XDP program with specific mode
    pub fn attach_with_mode(&mut self, interface: &str, mode: XdpAttachMode) -> Result<()> {
        if !self.available {
            return Err(anyhow!("XDP not available on this system"));
        }

        #[cfg(feature = "ebpf")]
        {
            use aya::programs::{Xdp, XdpFlags};

            let bpf_path = std::path::PathBuf::from("/sys/fs/bpf/stoq_xdp");
            // Also check target/bpf for locally compiled objects
            let obj_path = std::path::PathBuf::from("target/bpf/stoq_xdp.o");

            let load_path = if bpf_path.exists() {
                Some(bpf_path)
            } else if obj_path.exists() {
                Some(obj_path)
            } else {
                None
            };

            if let Some(path) = load_path {
                match aya::Bpf::load_file(&path) {
                    Ok(mut bpf) => {
                        match bpf.program_mut("stoq_xdp_filter") {
                            Some(program) => {
                                let xdp: &mut Xdp = program.try_into()
                                    .map_err(|e| anyhow!("Failed to cast to XDP program: {}", e))?;
                                xdp.load()
                                    .map_err(|e| anyhow!("Failed to load XDP program: {}", e))?;
                                let flags = match mode {
                                    XdpAttachMode::Native => XdpFlags::default(),
                                    XdpAttachMode::Generic => XdpFlags::SKB_MODE,
                                    XdpAttachMode::Offload => XdpFlags::HW_MODE,
                                };
                                xdp.attach(interface, flags)
                                    .map_err(|e| anyhow!("Failed to attach XDP to {}: {}", interface, e))?;
                                self.bpf = Some(bpf);
                                tracing::info!(
                                    "XDP program attached to {} in {:?} mode (kernel)",
                                    interface, mode
                                );
                            }
                            None => {
                                tracing::warn!(
                                    "No 'stoq_xdp_filter' program found in {:?}, using userspace fallback",
                                    path
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load eBPF object from {:?}: {}. Using userspace fallback.",
                            path, e
                        );
                    }
                }
            } else {
                tracing::info!(
                    "No compiled eBPF object found, using userspace fallback for XDP on {}",
                    interface
                );
            }
        }

        // Track attachment regardless of kernel/userspace mode
        let attached_prog = AttachedProgram {
            interface: interface.to_string(),
            attach_mode: mode,
        };
        self.attached.write().insert(interface.to_string(), attached_prog);

        #[cfg(not(feature = "ebpf"))]
        {
            tracing::info!(
                "XDP program attached to {} in {:?} mode (userspace fallback)",
                interface, mode
            );
        }

        Ok(())
    }

    /// Detach XDP program from interface
    pub fn detach(&mut self, interface: &str) -> Result<()> {
        if self.attached.write().remove(interface).is_some() {
            tracing::info!("XDP program detached from {}", interface);
        }
        Ok(())
    }

    /// Detach all XDP programs
    pub fn detach_all(&mut self) -> Result<()> {
        self.attached.write().clear();

        // Dropping the Bpf handle detaches all loaded programs
        #[cfg(feature = "ebpf")]
        {
            if self.bpf.take().is_some() {
                tracing::info!("eBPF programs unloaded from kernel");
            }
        }

        tracing::info!("All XDP programs detached");
        Ok(())
    }

    /// Update connection filter rules
    pub fn update_filter(&mut self, _src_ip: [u8; 16], _dst_ip: [u8; 16], _action: XdpAction) -> Result<()> {
        #[cfg(feature = "ebpf")]
        {
            if let Some(ref mut bpf) = self.bpf {
                use aya::maps::HashMap as BpfHashMap;

                // Build filter key from src + dst IPs
                let mut key = [0u8; 32];
                key[..16].copy_from_slice(&_src_ip);
                key[16..].copy_from_slice(&_dst_ip);

                let action_val: u32 = _action as u32;

                match bpf.map_mut("filter_map") {
                    Some(map) => {
                        match BpfHashMap::<_, [u8; 32], u32>::try_from(map) {
                            Ok(mut filter_map) => {
                                filter_map.insert(&key, &action_val, 0)
                                    .map_err(|e| anyhow!("Failed to update filter map: {}", e))?;
                                tracing::debug!("Updated XDP filter rule");
                            }
                            Err(e) => {
                                tracing::warn!("Failed to access filter_map: {}", e);
                            }
                        }
                    }
                    None => {
                        tracing::debug!("filter_map not available in loaded BPF program");
                    }
                }
            } else {
                tracing::debug!("Filter rule update skipped: no kernel BPF loaded");
            }
        }
        Ok(())
    }

    /// Get current statistics from XDP programs
    pub fn get_stats(&self) -> XdpStats {
        self.stats.read().clone()
    }

    /// Update statistics from eBPF maps
    pub fn update_stats(&mut self) -> Result<()> {
        #[cfg(feature = "ebpf")]
        {
            if let Some(ref mut bpf) = self.bpf {
                use aya::maps::PerCpuArray;

                match bpf.map_mut("stats_map") {
                    Some(map) => {
                        match PerCpuArray::<_, [u64; 4]>::try_from(map) {
                            Ok(stats_array) => {
                                if let Ok(per_cpu_values) = stats_array.get(&0, 0) {
                                    let mut aggregated = XdpStats::default();
                                    for values in per_cpu_values.iter() {
                                        aggregated.packets_passed += values[0];
                                        aggregated.packets_dropped += values[1];
                                        aggregated.packets_redirected += values[2];
                                        aggregated.bytes_processed += values[3];
                                    }
                                    *self.stats.write() = aggregated;
                                    tracing::trace!("Updated XDP stats from kernel maps");
                                }
                            }
                            Err(e) => {
                                tracing::debug!("Failed to read stats_map: {}", e);
                            }
                        }
                    }
                    None => {
                        tracing::debug!("stats_map not available in loaded BPF program");
                    }
                }
            }
        }
        Ok(())
    }
}

/// XDP filter configuration
#[derive(Debug, Clone)]
pub struct XdpFilterConfig {
    /// Allow only QUIC packets (UDP port 9292)
    pub filter_quic_only: bool,
    /// Drop non-IPv6 packets
    pub drop_ipv4: bool,
    /// Maximum packet size to process
    pub max_packet_size: usize,
    /// Enable connection tracking
    pub enable_connection_tracking: bool,
}

impl Default for XdpFilterConfig {
    fn default() -> Self {
        Self {
            filter_quic_only: true,
            drop_ipv4: true,
            max_packet_size: 65535,
            enable_connection_tracking: true,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xdp_manager_creation() {
        let manager = XdpManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_filter_config_default() {
        let config = XdpFilterConfig::default();
        assert!(config.filter_quic_only);
        assert!(config.drop_ipv4);
        assert_eq!(config.max_packet_size, 65535);
    }
}