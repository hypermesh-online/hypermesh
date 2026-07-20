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
use super::validation::{
    asset_hash_entry_to_bytes, policy_to_bytes, pos_validation_to_bytes,
};

use super::types::*;

/// Read CLOCK_MONOTONIC in nanoseconds via `clock_gettime`, sharing the
/// exact kernel clock base used by `bpf_ktime_get_ns()`. This makes the
/// XDP program's `now - last_validated > ttl` TTL comparison meaningful
/// across the userspace/kernel boundary (a process-relative `Instant`
/// would NOT share the kernel base and would corrupt the TTL math).
/// Falls back to 0 on error.
#[cfg(feature = "kernel-attach")]
fn monotonic_now_ns() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid, fully-owned timespec; clock_gettime only
    // writes into it and returns 0 on success.
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    if rc != 0 {
        return 0;
    }
    (ts.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(ts.tv_nsec as u64)
}

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
    /// Structural four-proof pre-validator for the userspace PoS path.
    ///
    /// Used by the packet-validation methods in `validation.rs` (the
    /// documented §5.4 userspace pre-validation). Kernel-attach does the
    /// same structural checks in-kernel; this drives the userspace fallback
    /// and the deep-validation companion.
    pub(super) pos_validator: ProofOfStateValidator,
    /// Hardware offload policy
    pub(crate) offload_policy: OffloadPolicy,
    /// Loaded BPF handle (only present when kernel-attach feature enabled).
    ///
    /// Held behind an `Arc<Mutex<..>>` so per-event map writes
    /// (`update_asset_hash_map`, `update_pos_header_map`, …) can take
    /// `&self` — the orchestrator drives them through a shared `&self`
    /// config API and a `RwLock` read guard, so `&mut self` is not
    /// available at the point of the kernel write.
    #[cfg(feature = "kernel-attach")]
    bpf: Arc<parking_lot::Mutex<Option<aya::Bpf>>>,
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
            bpf: Arc::new(parking_lot::Mutex::new(None)),
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

        // Push all currently-known policies into the kernel `policy_map`
        // (real aya writes under kernel-attach; no-op otherwise). This is
        // the actual population — the former PolicyManager::sync_to_kernel
        // "Would sync..." stub only validated byte formatting and never
        // held a BPF handle. When the BPF program is not loaded (userspace
        // fallback) sync_policies_to_bpf is a graceful no-op.
        self.sync_policies_to_bpf()?;

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
                        *self.bpf.lock() = Some(bpf);
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
            if self.bpf.lock().take().is_some() {
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
            let mut guard = self.bpf.lock();
            if let Some(ref mut bpf) = *guard {
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
            let mut guard = self.bpf.lock();
            if let Some(ref mut bpf) = *guard {
                use aya::maps::PerCpuArray;

                // The kernel `struct xdp_stats` is still 4x u64 on the wire,
                // but `bytes_processed` (values[3]) is intentionally never
                // written in-kernel (the kernel cannot see the encrypted QUIC
                // payload), so it is not aggregated or surfaced.
                if let Some(map) = bpf.map_mut("stats_map") {
                    if let Ok(stats_array) = PerCpuArray::<_, [u64; 4]>::try_from(map) {
                        if let Ok(per_cpu_values) = stats_array.get(&0, 0) {
                            let mut aggregated = XdpStats::default();
                            for values in per_cpu_values.iter() {
                                aggregated.packets_passed += values[0];
                                aggregated.packets_dropped += values[1];
                                aggregated.packets_redirected += values[2];
                            }
                            *self.stats.write() = aggregated;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Sync all policies from the `PolicyManager` into the kernel `policy_map`.
    ///
    /// The kernel `policy_map` is keyed by the 16-byte IPv6 source address
    /// (matching the C program's per-source lookup) with a 16-byte
    /// `policy_value` (4x u32 LE). Connection IDs stored in the
    /// `PolicyManager` are the low 8 bytes of a `NetworkId` /
    /// source-derived key; here we zero-extend the u64 key to a 16-byte
    /// address slot so the same policy applies to every source in that
    /// network until a per-source override is written via
    /// [`Self::update_policy_for_source`].
    ///
    /// When `kernel-attach` is not enabled this is a no-op returning `Ok(())`.
    #[cfg(feature = "kernel-attach")]
    pub fn sync_policies_to_bpf(&self) -> Result<()> {
        let mut guard = self.bpf.lock();
        let Some(ref mut bpf) = *guard else {
            return Ok(());
        };
        use aya::maps::HashMap as BpfHashMap;

        if let Some(map) = bpf.map_mut("policy_map") {
            match BpfHashMap::<_, [u8; 16], [u8; 16]>::try_from(map) {
                Ok(mut bpf_map) => {
                    let policies = self.policy_manager.get_all_policies();
                    for (conn_id, policy) in &policies {
                        // Zero-extend the u64 policy key into a 16-byte
                        // source-address slot (low 8 bytes = conn_id LE).
                        let mut key = [0u8; 16];
                        key[..8].copy_from_slice(&conn_id.to_le_bytes());
                        let bytes = policy_to_bytes(policy);
                        bpf_map.insert(&key, &bytes, 0).map_err(|e| {
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
        Ok(())
    }

    #[cfg(not(feature = "kernel-attach"))]
    pub fn sync_policies_to_bpf(&self) -> Result<()> {
        tracing::debug!(
            "kernel-attach not enabled; policy sync is userspace-only ({} policies)",
            self.policy_manager.policy_count()
        );
        Ok(())
    }

    /// Write a per-source `policy_value` into the kernel `policy_map`.
    ///
    /// P5 unification: called when userspace authenticates a peer so the
    /// kernel gate keys on the SAME IPv6 source P1 authorized. Under
    /// `kernel-attach` this is a real aya map write; otherwise a no-op.
    #[allow(unused_variables)]
    pub fn update_policy_for_source(
        &self,
        src_ip: [u8; 16],
        policy: &crate::policy_maps::ValidationPolicy,
    ) -> Result<()> {
        #[cfg(feature = "kernel-attach")]
        {
            let mut guard = self.bpf.lock();
            if let Some(ref mut bpf) = *guard {
                use aya::maps::HashMap as BpfHashMap;
                if let Some(map) = bpf.map_mut("policy_map") {
                    match BpfHashMap::<_, [u8; 16], [u8; 16]>::try_from(map) {
                        Ok(mut m) => {
                            let bytes = policy_to_bytes(policy);
                            m.insert(&src_ip, &bytes, 0).map_err(|e| {
                                anyhow!("Failed to write policy_map for source: {}", e)
                            })?;
                            tracing::debug!(
                                "policy_map[src] requires_pos={} written",
                                policy.requires_pos
                            );
                        }
                        Err(e) => tracing::warn!("policy_map access failed: {}", e),
                    }
                }
            }
        }
        Ok(())
    }

    /// Write an asset-hash registry entry into the kernel `asset_hash_map`.
    ///
    /// Keyed by the 32-byte BLAKE3 content hash — the SAME content address
    /// P1's userspace gate registers — so the kernel can reject transfers
    /// referencing unregistered assets at wire speed. Real aya write under
    /// `kernel-attach`, no-op otherwise.
    #[allow(unused_variables)]
    pub fn update_asset_hash_map(
        &self,
        content_hash: [u8; 32],
        shard_count: u32,
        registered: bool,
    ) -> Result<()> {
        #[cfg(feature = "kernel-attach")]
        {
            let mut guard = self.bpf.lock();
            if let Some(ref mut bpf) = *guard {
                use aya::maps::HashMap as BpfHashMap;
                if let Some(map) = bpf.map_mut("asset_hash_map") {
                    match BpfHashMap::<_, [u8; 32], [u8; 40]>::try_from(map) {
                        Ok(mut m) => {
                            let value = asset_hash_entry_to_bytes(
                                &content_hash,
                                shard_count,
                                registered,
                            );
                            m.insert(&content_hash, &value, 0).map_err(|e| {
                                anyhow!("Failed to write asset_hash_map: {}", e)
                            })?;
                            tracing::debug!(
                                "asset_hash_map[{}] registered={}",
                                hex::encode(&content_hash[..8]),
                                registered
                            );
                        }
                        Err(e) => tracing::warn!("asset_hash_map access failed: {}", e),
                    }
                }
            }
        }
        Ok(())
    }

    /// Write a PoS validation cache entry into the kernel `pos_header_map`.
    ///
    /// Keyed by the 16-byte IPv6 source address of an authenticated peer —
    /// the SAME fact P1 authenticates. `validated=1` means userspace
    /// completed cryptographic verification; the kernel then admits that
    /// source's HyperMesh traffic (subject to TTL). Real aya write under
    /// `kernel-attach`, no-op otherwise.
    #[allow(unused_variables)]
    pub fn update_pos_header_map(
        &self,
        src_ip: [u8; 16],
        validated: bool,
        algorithm: u8,
    ) -> Result<()> {
        #[cfg(feature = "kernel-attach")]
        {
            let mut guard = self.bpf.lock();
            if let Some(ref mut bpf) = *guard {
                use aya::maps::HashMap as BpfHashMap;
                if let Some(map) = bpf.map_mut("pos_header_map") {
                    match BpfHashMap::<_, [u8; 16], [u8; 24]>::try_from(map) {
                        Ok(mut m) => {
                            // last_validated is set to the kernel monotonic
                            // clock at write time; userspace cannot read
                            // bpf_ktime_get_ns() directly, so we approximate
                            // with CLOCK_MONOTONIC which shares the same base.
                            let now_ns = monotonic_now_ns();
                            let value =
                                pos_validation_to_bytes(algorithm, validated, now_ns);
                            m.insert(&src_ip, &value, 0).map_err(|e| {
                                anyhow!("Failed to write pos_header_map: {}", e)
                            })?;
                            tracing::debug!(
                                "pos_header_map[src] validated={}",
                                validated
                            );
                        }
                        Err(e) => tracing::warn!("pos_header_map access failed: {}", e),
                    }
                }
            }
        }
        Ok(())
    }

    /// Write a matrix-routing forwarding rule into the kernel `routing_map`.
    ///
    /// Keyed by the raw 12-byte matrix position (3x f32 LE, exactly as it
    /// appears in the on-wire MATRIX extension header). The value carries
    /// the egress ifindex and an active flag driving the XDP_TX delegation
    /// branch (P5 step 6). Real aya write under `kernel-attach`, no-op
    /// otherwise.
    #[allow(unused_variables)]
    pub fn update_routing_map(
        &self,
        position_bytes: [u8; 12],
        egress_ifindex: u32,
        active: bool,
    ) -> Result<()> {
        #[cfg(feature = "kernel-attach")]
        {
            let mut guard = self.bpf.lock();
            if let Some(ref mut bpf) = *guard {
                use aya::maps::HashMap as BpfHashMap;
                if let Some(map) = bpf.map_mut("routing_map") {
                    match BpfHashMap::<_, [u8; 12], [u8; 8]>::try_from(map) {
                        Ok(mut m) => {
                            let mut value = [0u8; 8];
                            value[0..4].copy_from_slice(&egress_ifindex.to_le_bytes());
                            value[4..8]
                                .copy_from_slice(&(active as u32).to_le_bytes());
                            m.insert(&position_bytes, &value, 0).map_err(|e| {
                                anyhow!("Failed to write routing_map: {}", e)
                            })?;
                            tracing::debug!(
                                "routing_map[pos] egress_if={} active={}",
                                egress_ifindex,
                                active
                            );
                        }
                        Err(e) => tracing::warn!("routing_map access failed: {}", e),
                    }
                }
            }
        }
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
            let mut guard = self.bpf.lock();
            if let Some(ref mut bpf) = *guard {
                use aya::maps::Array;

                if let Some(map) = bpf.map_mut("pos_config_map") {
                    match Array::<_, [u8; KernelPosConfig::SIZE]>::try_from(map) {
                        Ok(mut array) => {
                            let bytes = config.to_bytes();
                            array
                                .set(0, &bytes, 0)
                                .map_err(|e| anyhow!("Failed to write pos_config_map: {}", e))?;
                            tracing::info!(
                                "Kernel PoS config synced: ttl={}ns, enabled={}",
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
                 (ttl={}ns, enabled={})",
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
