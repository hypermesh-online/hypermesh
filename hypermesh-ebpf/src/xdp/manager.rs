// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! XDP manager: program attachment, kernel map operations, and lifecycle.
//!
//! Packet validation methods are in `validation.rs`.

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::capabilities::NicCapabilities;
use crate::policy_maps::PolicyManager;
use crate::validation::ProofOfStateValidator;

#[cfg(feature = "kernel-attach")]
use super::validation::policy_to_bytes;

use super::types::*;

// -----------------------------------------------------------------------
// Unified XDP Manager
// -----------------------------------------------------------------------

/// Unified XDP manager for the HyperMesh eBPF subsystem.
///
/// Combines XDP program attachment/detachment with HyperMesh-specific
/// packet validation (PoS headers, asset hashes, matrix routing).
pub struct XdpManager {
    /// Attached interfaces and their programs
    pub(super) attached: Arc<RwLock<HashMap<String, AttachedProgram>>>,
    /// Per-interface XDP statistics
    stats: Arc<RwLock<XdpStats>>,
    /// Whether XDP kernel support is detected
    available: bool,
    /// Policy manager for validation decisions
    pub(super) policy_manager: PolicyManager,
    /// Proof of State fast validator
    pub(super) pos_validator: ProofOfStateValidator,
    /// Hardware offload policy
    pub(crate) offload_policy: OffloadPolicy,
    /// Loaded BPF handle (only present when kernel-attach feature enabled)
    #[cfg(feature = "kernel-attach")]
    bpf: Option<aya::Bpf>,
}

impl XdpManager {
    /// Create a new XDP manager
    pub fn new(policy_manager: PolicyManager) -> Result<Self> {
        let available = Self::check_xdp_support();

        if available {
            tracing::info!("XDP support detected on this system");
        } else {
            tracing::debug!("XDP not available, userspace validation active");
        }

        Ok(Self {
            attached: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(XdpStats::default())),
            available,
            policy_manager,
            pos_validator: ProofOfStateValidator::default(),
            offload_policy: OffloadPolicy::Disabled,
            #[cfg(feature = "kernel-attach")]
            bpf: None,
        })
    }

    fn check_xdp_support() -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/sys/fs/bpf").exists()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    /// Whether XDP kernel attachment is available
    pub fn is_available(&self) -> bool {
        self.available
    }

    // -------------------------------------------------------------------
    // XDP program attachment
    // -------------------------------------------------------------------

    /// Attach the HyperMesh XDP program to a network interface.
    ///
    /// With `kernel-attach` feature: loads the unified `hypermesh_xdp` program
    /// into the kernel and attaches to the interface via aya.
    /// Without: tracks attachment state for userspace fallback.
    #[cfg(feature = "kernel-attach")]
    pub fn attach(&mut self, interface: &str) -> Result<()> {
        self.attach_with_mode(interface, XdpAttachMode::Generic)
    }

    #[cfg(not(feature = "kernel-attach"))]
    pub fn attach(&mut self, interface: &str) -> Result<()> {
        self.attach_with_mode(interface, XdpAttachMode::Generic)
    }

    /// Set the hardware offload policy
    pub fn set_offload_policy(&mut self, policy: OffloadPolicy) {
        self.offload_policy = policy;
    }

    /// Attach with a specific XDP mode (Native/Generic/Offload)
    ///
    /// When mode is `Offload`, detects NIC capabilities and resolves the
    /// effective mode based on the current `OffloadPolicy`:
    /// - NIC supports offload: proceed with HW_MODE
    /// - NIC does not support offload + `Required`: return error
    /// - NIC does not support offload + `Opportunistic`/`Disabled`: fall back to Native
    pub fn attach_with_mode(&mut self, interface: &str, mode: XdpAttachMode) -> Result<()> {
        let effective_mode = match mode {
            XdpAttachMode::Offload => {
                let nic = NicCapabilities::detect(interface);
                if nic.supports_xdp_offload {
                    tracing::info!(
                        "NIC {} (driver: {}) supports XDP offload",
                        interface,
                        nic.driver_name
                    );
                    XdpAttachMode::Offload
                } else {
                    match self.offload_policy {
                        OffloadPolicy::Required => {
                            return Err(anyhow!(
                                "NIC {} (driver: {}) does not support XDP offload and policy is Required",
                                interface,
                                if nic.driver_name.is_empty() { "unknown" } else { &nic.driver_name }
                            ));
                        }
                        _ => {
                            tracing::info!(
                                "NIC {} (driver: {}) does not support XDP offload, falling back to native",
                                interface,
                                if nic.driver_name.is_empty() { "unknown" } else { &nic.driver_name }
                            );
                            XdpAttachMode::Native
                        }
                    }
                }
            }
            other => other,
        };

        #[cfg(feature = "kernel-attach")]
        {
            if self.available {
                self.try_kernel_attach(interface, effective_mode)?;
            }
        }

        // Track attachment regardless of kernel/userspace mode
        let prog = AttachedProgram {
            _interface: interface.to_string(),
            _attach_mode: effective_mode,
        };
        self.attached.write().insert(interface.to_string(), prog);

        // Sync policies to kernel maps if attached
        self.policy_manager.sync_to_kernel()?;

        tracing::info!(
            "XDP manager attached to {} ({} mode)",
            interface,
            if self.available {
                "kernel"
            } else {
                "userspace"
            }
        );

        Ok(())
    }

    #[cfg(feature = "kernel-attach")]
    fn try_kernel_attach(&mut self, interface: &str, mode: XdpAttachMode) -> Result<()> {
        use aya::programs::{Xdp, XdpFlags};

        let bpf_paths = [
            std::path::PathBuf::from("/sys/fs/bpf/hypermesh_xdp"),
            std::path::PathBuf::from("target/bpf/hypermesh_xdp.o"),
            std::path::PathBuf::from("target/bpf/stoq_xdp.o"),
        ];

        let load_path = bpf_paths.iter().find(|p| p.exists());

        if let Some(path) = load_path {
            match aya::Bpf::load_file(path) {
                Ok(mut bpf) => {
                    // Try hypermesh program name first, then stoq
                    let prog_name = if bpf.program("hypermesh_xdp_filter").is_some() {
                        "hypermesh_xdp_filter"
                    } else {
                        "stoq_xdp_filter"
                    };

                    if let Some(program) = bpf.program_mut(prog_name) {
                        let xdp: &mut Xdp = program
                            .try_into()
                            .map_err(|e| anyhow!("Not an XDP program: {}", e))?;
                        xdp.load()
                            .map_err(|e| anyhow!("Failed to load XDP: {}", e))?;
                        let flags = match mode {
                            XdpAttachMode::Native => XdpFlags::default(),
                            XdpAttachMode::Generic => XdpFlags::SKB_MODE,
                            XdpAttachMode::Offload => XdpFlags::HW_MODE,
                        };
                        xdp.attach(interface, flags)
                            .map_err(|e| anyhow!("Failed to attach XDP to {}: {}", interface, e))?;
                        self.bpf = Some(bpf);
                        tracing::info!(
                            "XDP program '{}' attached to {} from {:?}",
                            prog_name,
                            interface,
                            path
                        );
                    } else {
                        tracing::warn!(
                            "No XDP filter program found in {:?}, using userspace",
                            path
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load eBPF from {:?}: {}. Userspace fallback.",
                        path,
                        e
                    );
                }
            }
        } else {
            tracing::info!(
                "No compiled eBPF object found, userspace fallback for {}",
                interface
            );
        }

        Ok(())
    }

    /// Detach XDP program from a specific interface
    pub fn detach(&mut self, interface: &str) -> Result<()> {
        if self.attached.write().remove(interface).is_some() {
            tracing::info!("XDP program detached from {}", interface);
        }
        Ok(())
    }

    /// Detach all XDP programs
    pub fn detach_all(&mut self) -> Result<()> {
        self.attached.write().clear();

        #[cfg(feature = "kernel-attach")]
        {
            if self.bpf.take().is_some() {
                tracing::info!("eBPF programs unloaded from kernel");
            }
        }

        tracing::info!("All XDP programs detached");
        Ok(())
    }

    // -------------------------------------------------------------------
    // Kernel map operations
    // -------------------------------------------------------------------

    /// Update XDP filter rules in kernel map
    #[allow(unused_variables)]
    pub fn update_filter(
        &mut self,
        src_ip: [u8; 16],
        dst_ip: [u8; 16],
        action: XdpAction,
    ) -> Result<()> {
        #[cfg(feature = "kernel-attach")]
        {
            if let Some(ref mut bpf) = self.bpf {
                use aya::maps::HashMap as BpfHashMap;

                let mut key = [0u8; 32];
                key[..16].copy_from_slice(&src_ip);
                key[16..].copy_from_slice(&dst_ip);

                let action_val: u32 = action as u32;

                if let Some(map) = bpf.map_mut("filter_map") {
                    match BpfHashMap::<_, [u8; 32], u32>::try_from(map) {
                        Ok(mut filter_map) => {
                            filter_map
                                .insert(&key, &action_val, 0)
                                .map_err(|e| anyhow!("Failed to update filter map: {}", e))?;
                            tracing::debug!("Updated XDP filter rule");
                        }
                        Err(e) => {
                            tracing::warn!("Failed to access filter_map: {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Read current statistics from XDP maps
    pub fn get_stats(&self) -> XdpStats {
        self.stats.read().clone()
    }

    /// Update stats by reading from kernel eBPF maps
    pub fn update_stats(&mut self) -> Result<()> {
        #[cfg(feature = "kernel-attach")]
        {
            if let Some(ref mut bpf) = self.bpf {
                use aya::maps::PerCpuArray;

                if let Some(map) = bpf.map_mut("stats_map") {
                    if let Ok(stats_array) = PerCpuArray::<_, [u64; 4]>::try_from(map) {
                        if let Ok(per_cpu_values) = stats_array.get(&0, 0) {
                            let mut aggregated = XdpStats::default();
                            for values in per_cpu_values.iter() {
                                aggregated.packets_passed += values[0];
                                aggregated.packets_dropped += values[1];
                                aggregated.packets_redirected += values[2];
                                aggregated.bytes_processed += values[3];
                            }
                            *self.stats.write() = aggregated;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Sync all policies from the PolicyManager into the kernel BPF "policy_map".
    ///
    /// Each policy is serialized as a 24-byte `#[repr(C)]` blob (matching
    /// `ValidationPolicy` layout) keyed by connection ID (u64).
    ///
    /// When `kernel-attach` is not enabled this is a no-op that returns `Ok(())`.
    #[cfg(feature = "kernel-attach")]
    pub fn sync_policies_to_bpf(&mut self) -> Result<()> {
        if let Some(ref mut bpf) = self.bpf {
            use aya::maps::HashMap as BpfHashMap;

            if let Some(map) = bpf.map_mut("policy_map") {
                match BpfHashMap::<_, u64, [u8; 24]>::try_from(map) {
                    Ok(mut bpf_map) => {
                        let policies = self.policy_manager.get_all_policies();
                        for (conn_id, policy) in &policies {
                            let bytes = policy_to_bytes(policy);
                            bpf_map.insert(conn_id, &bytes, 0).map_err(|e| {
                                anyhow!("Failed to insert policy for conn {}: {}", conn_id, e)
                            })?;
                        }
                        tracing::debug!("Synced {} policies to kernel BPF map", policies.len());
                    }
                    Err(e) => {
                        tracing::warn!("Failed to access policy_map: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    #[cfg(not(feature = "kernel-attach"))]
    pub fn sync_policies_to_bpf(&mut self) -> Result<()> {
        tracing::debug!(
            "kernel-attach not enabled; policy sync is userspace-only ({} policies)",
            self.policy_manager.policy_count()
        );
        Ok(())
    }

    /// Get the interface name this manager is attached to (first attached)
    pub fn interface(&self) -> Option<String> {
        self.attached.read().keys().next().cloned()
    }

    /// Get policy manager reference
    pub fn policy_manager(&self) -> &PolicyManager {
        &self.policy_manager
    }

    // -------------------------------------------------------------------
    // Kernel PoS config
    // -------------------------------------------------------------------

    /// Set kernel-side PoS validation configuration.
    ///
    /// With `kernel-attach`: serializes the config to a 24-byte blob and
    /// writes it to the `pos_config_map` BPF array map at index 0.
    ///
    /// Without `kernel-attach`: logs the config and returns Ok.
    #[allow(unused_variables)]
    pub fn set_kernel_pos_config(&mut self, config: &KernelPosConfig) -> Result<()> {
        #[cfg(feature = "kernel-attach")]
        {
            if let Some(ref mut bpf) = self.bpf {
                use aya::maps::Array;

                if let Some(map) = bpf.map_mut("pos_config_map") {
                    match Array::<_, [u8; 24]>::try_from(map) {
                        Ok(mut array) => {
                            let bytes = config.to_bytes();
                            array
                                .set(0, &bytes, 0)
                                .map_err(|e| anyhow!("Failed to write pos_config_map: {}", e))?;
                            tracing::info!(
                                "Kernel PoS config synced: difficulty={}, ttl={}ns, enabled={}",
                                config.min_difficulty,
                                config.validation_ttl_ns,
                                config.enabled
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to access pos_config_map: {}", e);
                        }
                    }
                }
            }
        }

        #[cfg(not(feature = "kernel-attach"))]
        {
            tracing::debug!(
                "kernel-attach not enabled; kernel PoS config stored locally \
                 (difficulty={}, ttl={}ns, enabled={})",
                config.min_difficulty,
                config.validation_ttl_ns,
                config.enabled
            );
        }

        Ok(())
    }
}

impl Drop for XdpManager {
    fn drop(&mut self) {
        let _ = self.detach_all();
    }
}
