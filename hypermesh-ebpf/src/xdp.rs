// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Unified XDP (eXpress Data Path) Management
//!
//! Provides kernel-level packet classification, filtering, and routing
//! for the HyperMesh node. Merges HyperMesh intelligence validation
//! (PoS, asset hash, routing) with XDP program attachment and management.
//!
//! This is THE single XDP manager for the entire HyperMesh stack.
//! STOQ and blockmatrix are consumers via the `HyperMeshEbpf` orchestrator.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::capabilities::NicCapabilities;
use crate::policy_maps::PolicyManager;
use crate::hypermesh_headers::*;
use crate::validation::{ProofOfStateValidator, FastValidationResult};
use hypermesh_lib::MatrixPosition;

// -----------------------------------------------------------------------
// Kernel-side PoS configuration
// -----------------------------------------------------------------------

/// Configuration for kernel-side PoS structural validation.
///
/// Synced to the `pos_config_map` BPF array map (index 0).
///
/// These checks are non-cryptographic -- they reject obviously invalid
/// packets at wire speed (wrong algorithm byte, insufficient PoW
/// difficulty, stale cache entries).  Full asymmetric crypto
/// verification (FALCON-1024, Ed25519, ECDSA) MUST remain in
/// userspace because the BPF instruction set has no such helpers.
///
/// Serialization layout (24 bytes, little-endian, matches C `struct pos_config`):
///   `[0..4]`   min_difficulty        (u32 LE)
///   `[4..12]`  max_timestamp_skew_ns (u64 LE)
///   `[12..20]` validation_ttl_ns     (u64 LE)
///   `[20..24]` enabled               (u32 LE, 1 or 0)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelPosConfig {
    /// Minimum leading zero bits for PoW difficulty (0 = disabled)
    pub min_difficulty: u32,
    /// Maximum clock skew tolerance in nanoseconds (0 = disabled).
    /// Stored in the BPF map for future use; current kernel code
    /// uses `validation_ttl_ns` for staleness enforcement.
    pub max_timestamp_skew_ns: u64,
    /// How long a cached PoS validation is considered valid (ns).
    /// 0 means cached entries never expire (infinite TTL).
    pub validation_ttl_ns: u64,
    /// Whether kernel-side PoS structural checks are enabled.
    /// When false, the XDP program falls back to cache-only lookup.
    pub enabled: bool,
}

impl Default for KernelPosConfig {
    fn default() -> Self {
        Self {
            min_difficulty: 8, // Match userspace default (first byte must be 0x00)
            max_timestamp_skew_ns: 5 * 60 * 1_000_000_000, // 5 minutes
            validation_ttl_ns: 60 * 60 * 1_000_000_000,    // 1 hour
            enabled: true,
        }
    }
}

impl KernelPosConfig {
    /// Serialize to 24 bytes matching the C `struct pos_config` layout.
    ///
    /// Layout (all little-endian):
    ///   `[0..4]`   min_difficulty        u32
    ///   `[4..12]`  max_timestamp_skew_ns u64
    ///   `[12..20]` validation_ttl_ns     u64
    ///   `[20..24]` enabled               u32
    pub fn to_bytes(&self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        buf[0..4].copy_from_slice(&self.min_difficulty.to_le_bytes());
        buf[4..12].copy_from_slice(&self.max_timestamp_skew_ns.to_le_bytes());
        buf[12..20].copy_from_slice(&self.validation_ttl_ns.to_le_bytes());
        buf[20..24].copy_from_slice(&(self.enabled as u32).to_le_bytes());
        buf
    }

    /// Deserialize from 24 bytes (C `struct pos_config` layout).
    ///
    /// Returns `None` if the slice is too short.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 24 {
            return None;
        }
        Some(Self {
            min_difficulty: u32::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3],
            ]),
            max_timestamp_skew_ns: u64::from_le_bytes([
                bytes[4], bytes[5], bytes[6], bytes[7],
                bytes[8], bytes[9], bytes[10], bytes[11],
            ]),
            validation_ttl_ns: u64::from_le_bytes([
                bytes[12], bytes[13], bytes[14], bytes[15],
                bytes[16], bytes[17], bytes[18], bytes[19],
            ]),
            enabled: u32::from_le_bytes([
                bytes[20], bytes[21], bytes[22], bytes[23],
            ]) != 0,
        })
    }
}

// -----------------------------------------------------------------------
// Packet decision types (the three execution paths)
// -----------------------------------------------------------------------

/// Decision for an incoming packet. Represents the three HyperMesh execution paths:
/// 1. Pass (local execution)
/// 2. Redirect (zero-copy AF_XDP to STOQ)
/// 3. Forward (delegate to another matrix node)
/// 4. Drop (invalid)
#[derive(Debug, Clone, PartialEq)]
pub enum PacketDecision {
    /// XDP_PASS - deliver to local userspace for processing
    Pass,
    /// XDP_REDIRECT - zero-copy transfer to AF_XDP socket for STOQ
    Redirect { socket_index: u32 },
    /// XDP_TX / forward - delegate to another matrix node
    Forward { next_hop: MatrixPosition },
    /// XDP_DROP - packet is invalid, discard
    Drop { reason: String },
}

/// Legacy filter action (kept for backward compatibility with existing tests)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// Pass packet to userspace
    Pass,
    /// Drop packet at kernel level
    Drop,
    /// Redirect to AF_XDP socket for zero-copy
    Redirect,
}

// -----------------------------------------------------------------------
// XDP attach mode and stats
// -----------------------------------------------------------------------

/// XDP attach mode
#[derive(Debug, Clone, Copy)]
pub enum XdpAttachMode {
    /// Native mode (fastest, requires driver support)
    Native,
    /// Generic/SKB mode (slower, works everywhere)
    Generic,
    /// Offloaded to NIC hardware (if supported)
    Offload,
}

/// Policy for handling XDP hardware offload
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffloadPolicy {
    /// Never attempt hardware offload (default)
    Disabled,
    /// Try hardware offload, fall back to native XDP if unavailable
    Opportunistic,
    /// Require hardware offload, fail if NIC doesn't support it
    Required,
}

impl Default for OffloadPolicy {
    fn default() -> Self {
        Self::Disabled
    }
}

/// XDP program statistics aggregated from kernel maps
#[derive(Debug, Default, Clone)]
pub struct XdpStats {
    pub packets_passed: u64,
    pub packets_dropped: u64,
    pub packets_redirected: u64,
    pub bytes_processed: u64,
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
    /// Enable connection tracking in kernel map
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

// -----------------------------------------------------------------------
// Attached program tracking
// -----------------------------------------------------------------------

struct AttachedProgram {
    _interface: String,
    _attach_mode: XdpAttachMode,
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
    attached: Arc<RwLock<HashMap<String, AttachedProgram>>>,
    /// Per-interface XDP statistics
    stats: Arc<RwLock<XdpStats>>,
    /// Whether XDP kernel support is detected
    available: bool,
    /// Policy manager for validation decisions
    policy_manager: PolicyManager,
    /// Proof of State fast validator
    pos_validator: ProofOfStateValidator,
    /// Hardware offload policy
    offload_policy: OffloadPolicy,
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
    pub fn attach_with_mode(
        &mut self,
        interface: &str,
        mode: XdpAttachMode,
    ) -> Result<()> {
        let effective_mode = match mode {
            XdpAttachMode::Offload => {
                let nic = NicCapabilities::detect(interface);
                if nic.supports_xdp_offload {
                    tracing::info!(
                        "NIC {} (driver: {}) supports XDP offload",
                        interface, nic.driver_name
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
            if self.available { "kernel" } else { "userspace" }
        );

        Ok(())
    }

    #[cfg(feature = "kernel-attach")]
    fn try_kernel_attach(
        &mut self,
        interface: &str,
        mode: XdpAttachMode,
    ) -> Result<()> {
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
                        let xdp: &mut Xdp = program.try_into()
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
                            prog_name, interface, path
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
                        path, e
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
    // Packet validation (userspace path)
    // -------------------------------------------------------------------

    /// Validate a packet and return a decision (the three execution paths).
    ///
    /// This is the userspace validation path. With kernel-attach, the XDP
    /// program handles this at kernel level; this function serves as fallback.
    ///
    /// Enforces all policy flags:
    /// - `max_packet_size`: Drop oversized packets
    /// - `requires_pos`: Parse and validate PoS header from packet
    /// - `validate_asset_hash`: Check asset hash header in packet
    /// - `check_matrix_routing`: Verify matrix routing header in packet
    pub fn validate_packet(
        &self,
        connection_id: u64,
        packet_data: &[u8],
    ) -> PacketDecision {
        let policy = self.policy_manager.get_policy(connection_id);

        // Check packet size
        if packet_data.len() > policy.max_packet_size as usize {
            return PacketDecision::Drop {
                reason: format!(
                    "Packet too large: {} > {}",
                    packet_data.len(),
                    policy.max_packet_size
                ),
            };
        }

        // Enforce PoS validation when required by policy
        if policy.requires_pos {
            if packet_data.len() < ProofOfStateHeader::SIZE {
                return PacketDecision::Drop {
                    reason: format!(
                        "Packet too short for PoS header: {} < {}",
                        packet_data.len(),
                        ProofOfStateHeader::SIZE
                    ),
                };
            }

            match ProofOfStateHeader::from_bytes(packet_data) {
                Some(header) => {
                    let result = self.pos_validator.validate_fast(&header);
                    if !result.all_ok() {
                        return PacketDecision::Drop {
                            reason: format!(
                                "PoS validation failed: timestamp={}, stake={}, work={}, space={}",
                                result.timestamp_ok, result.stake_ok,
                                result.work_ok, result.space_ok
                            ),
                        };
                    }
                }
                None => {
                    return PacketDecision::Drop {
                        reason: "Failed to parse PoS header".to_string(),
                    };
                }
            }
        }

        // Enforce asset hash validation when required by policy
        if policy.validate_asset_hash {
            // Asset hash header follows PoS header (or starts at offset 0
            // if PoS is not required).
            let offset = if policy.requires_pos {
                ProofOfStateHeader::SIZE
            } else {
                0
            };

            if packet_data.len() < offset + AssetHashHeader::SIZE {
                return PacketDecision::Drop {
                    reason: format!(
                        "Packet too short for asset hash header at offset {}: {} < {}",
                        offset,
                        packet_data.len(),
                        offset + AssetHashHeader::SIZE
                    ),
                };
            }

            match AssetHashHeader::from_bytes(&packet_data[offset..]) {
                Some(header) => {
                    if !header.validate_shard_indices() {
                        return PacketDecision::Drop {
                            reason: format!(
                                "Invalid shard indices: {}/{}",
                                header.shard_index, header.shard_count
                            ),
                        };
                    }
                }
                None => {
                    return PacketDecision::Drop {
                        reason: "Failed to parse asset hash header".to_string(),
                    };
                }
            }
        }

        // Enforce matrix routing validation when required by policy
        if policy.check_matrix_routing {
            // Routing header follows PoS + asset hash headers
            let mut offset = 0;
            if policy.requires_pos {
                offset += ProofOfStateHeader::SIZE;
            }
            if policy.validate_asset_hash {
                offset += AssetHashHeader::SIZE;
            }

            if packet_data.len() < offset + MatrixRoutingHeader::MIN_SIZE {
                return PacketDecision::Drop {
                    reason: format!(
                        "Packet too short for routing header at offset {}: {} < {}",
                        offset,
                        packet_data.len(),
                        offset + MatrixRoutingHeader::MIN_SIZE
                    ),
                };
            }

            match MatrixRoutingHeader::from_bytes(&packet_data[offset..]) {
                Some(routing) => {
                    // Use u16::MAX as matrix size bound (permissive)
                    if !routing.validate_path(u16::MAX) {
                        return PacketDecision::Drop {
                            reason: "Matrix routing path validation failed".to_string(),
                        };
                    }
                }
                None => {
                    return PacketDecision::Drop {
                        reason: "Failed to parse matrix routing header".to_string(),
                    };
                }
            }
        }

        // Default: pass to userspace for processing
        PacketDecision::Pass
    }

    /// Validate a packet returning legacy FilterAction for backward compatibility
    pub fn validate_packet_userspace(
        &self,
        connection_id: u64,
        packet_data: &[u8],
    ) -> FilterAction {
        match self.validate_packet(connection_id, packet_data) {
            PacketDecision::Pass => FilterAction::Pass,
            PacketDecision::Redirect { .. } => FilterAction::Redirect,
            PacketDecision::Forward { .. } => FilterAction::Pass,
            PacketDecision::Drop { .. } => FilterAction::Drop,
        }
    }

    /// Validate Proof of State extension header using the enhanced four-proof
    /// validator. Returns true only if all four proofs pass fast validation.
    pub fn validate_proof_of_state(&self, proof: &ProofOfStateHeader) -> bool {
        let result = self.pos_validator.validate_fast(proof);
        if !result.all_ok() {
            tracing::warn!(
                "Proof of State fast validation failed: timestamp={}, stake={}, work={}, space={}",
                result.timestamp_ok, result.stake_ok, result.work_ok, result.space_ok
            );
            return false;
        }
        true
    }

    /// Validate Proof of State with detailed per-proof results.
    pub fn validate_proof_of_state_detailed(
        &self,
        proof: &ProofOfStateHeader,
    ) -> FastValidationResult {
        self.pos_validator.validate_fast(proof)
    }

    /// Validate Asset Hash extension header
    pub fn validate_asset_hash(
        &self,
        header: &AssetHashHeader,
        _payload: &[u8],
    ) -> bool {
        if !header.validate_shard_indices() {
            tracing::warn!("Invalid shard indices in asset hash header");
            return false;
        }
        true
    }

    /// Validate Matrix Routing extension header
    pub fn validate_matrix_routing(
        &self,
        routing: &MatrixRoutingHeader,
        matrix_size: u16,
    ) -> bool {
        if !routing.validate_path(matrix_size) {
            tracing::warn!("Invalid matrix routing path");
            return false;
        }
        true
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
                            filter_map.insert(&key, &action_val, 0)
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
                            bpf_map.insert(conn_id, &bytes, 0)
                                .map_err(|e| anyhow!(
                                    "Failed to insert policy for conn {}: {}",
                                    conn_id, e
                                ))?;
                        }
                        tracing::debug!(
                            "Synced {} policies to kernel BPF map",
                            policies.len()
                        );
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
                            array.set(0, &bytes, 0)
                                .map_err(|e| anyhow!(
                                    "Failed to write pos_config_map: {}", e
                                ))?;
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

/// Serialize a `ValidationPolicy` to a 24-byte `#[repr(C)]` byte array
/// suitable for writing into a BPF hash map.
///
/// Layout (24 bytes):
///   [0]     requires_pos (bool as u8)
///   [1]     validate_asset_hash (bool as u8)
///   [2]     check_matrix_routing (bool as u8)
///   [3]     privacy_tier (u8)
///   [4..8]  max_packet_size (u32 little-endian)
///   [8..12] rate_limit_per_sec (u32 little-endian)
///   [12..20] _reserved (8 bytes)
///   [20..24] padding (zeros)
#[cfg(any(feature = "kernel-attach", test))]
fn policy_to_bytes(policy: &crate::policy_maps::ValidationPolicy) -> [u8; 24] {
    let mut buf = [0u8; 24];
    buf[0] = policy.requires_pos as u8;
    buf[1] = policy.validate_asset_hash as u8;
    buf[2] = policy.check_matrix_routing as u8;
    buf[3] = policy.privacy_tier;
    buf[4..8].copy_from_slice(&policy.max_packet_size.to_le_bytes());
    buf[8..12].copy_from_slice(&policy.rate_limit_per_sec.to_le_bytes());
    // bytes 12..24 remain zero (reserved + padding)
    buf
}

/// XDP action to take on packets (matches kernel XDP_* constants)
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

impl Drop for XdpManager {
    fn drop(&mut self) {
        let _ = self.detach_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy_maps::ValidationPolicy;
    use crate::validation::ALG_FALCON_1024;

    /// Build a valid `who` field: FALCON-1024 algorithm indicator + 8 non-zero prefix bytes.
    fn valid_who() -> [u8; 32] {
        let mut who = [0xABu8; 32];
        who[0] = ALG_FALCON_1024;
        who
    }

    /// Build a valid `what` field: first byte zero (8 leading zero bits meets default difficulty).
    fn valid_what() -> [u8; 32] {
        let mut what = [0xFFu8; 32];
        what[0] = 0x00;
        what
    }

    /// Build a valid `where_` field: IPv6 global unicast prefix (0x20).
    fn valid_where() -> [u8; 16] {
        let mut w = [0x01u8; 16];
        w[0] = 0x20;
        w
    }

    fn now_micros() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("test: get system time")
            .as_micros() as u64
    }

    #[test]
    fn test_xdp_manager_creation() {
        let policy = PolicyManager::new().expect("test: create policy manager");
        let manager = XdpManager::new(policy);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_filter_config_default() {
        let config = XdpFilterConfig::default();
        assert!(config.filter_quic_only);
        assert!(config.drop_ipv4);
        assert_eq!(config.max_packet_size, 65535);
    }

    #[test]
    fn test_packet_decision_pass() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_default_policy(ValidationPolicy::permissive());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = vec![0u8; 1500];
        let decision = mgr.validate_packet(123, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_packet_decision_drop_oversized() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let large_packet = vec![0u8; 70000];
        let decision = mgr.validate_packet(123, &large_packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("too large"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_userspace_validation_compat() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_default_policy(ValidationPolicy::permissive());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = vec![0u8; 1500];
        let action = mgr.validate_packet_userspace(123, &packet);
        assert_eq!(action, FilterAction::Pass);
    }

    #[test]
    fn test_proof_of_state_validation_valid() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let valid_proof = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        assert!(mgr.validate_proof_of_state(&valid_proof));
    }

    #[test]
    fn test_proof_of_state_validation_future_timestamp() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let future_proof = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros() + 10 * 60 * 1_000_000, // 10 min in future
            where_: valid_where(),
        };
        assert!(!mgr.validate_proof_of_state(&future_proof));
    }

    #[test]
    fn test_proof_of_state_detailed_results() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let proof = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };

        let result = mgr.validate_proof_of_state_detailed(&proof);
        assert!(result.all_ok());
        assert!(result.timestamp_ok);
        assert!(result.stake_ok);
        assert!(result.work_ok);
        assert!(result.space_ok);
    }

    #[test]
    fn test_sync_policies_to_bpf_no_kernel() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_policy(1, ValidationPolicy::strict());
        pm.set_policy(2, ValidationPolicy::permissive());
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Without kernel-attach, this should succeed as a no-op
        assert!(mgr.sync_policies_to_bpf().is_ok());
    }

    #[test]
    fn test_policy_to_bytes_roundtrip() {
        let policy = ValidationPolicy::strict();
        let bytes = policy_to_bytes(&policy);
        assert_eq!(bytes[0], 1); // requires_pos = true
        assert_eq!(bytes[1], 1); // validate_asset_hash = true
        assert_eq!(bytes[2], 1); // check_matrix_routing = true
        assert_eq!(bytes[3], 2); // privacy_tier = 2
        let max_pkt = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(max_pkt, 9000);
        let rate = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        assert_eq!(rate, 100);
    }

    // -------------------------------------------------------------------
    // Policy enforcement tests (validate_packet with policy flags)
    // -------------------------------------------------------------------

    /// Build a valid PoS header as raw bytes.
    fn valid_pos_bytes() -> Vec<u8> {
        let header = ProofOfStateHeader {
            who: valid_who(),
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        header.to_bytes()
    }

    /// Build a valid asset hash header as raw bytes.
    fn valid_asset_hash_bytes() -> Vec<u8> {
        let header = AssetHashHeader {
            asset_id: [0x01; 32],
            hash: [0x02; 32],
            shard_count: 10,
            shard_index: 3,
        };
        header.to_bytes()
    }

    /// Build a valid matrix routing header as raw bytes.
    fn valid_routing_bytes() -> Vec<u8> {
        let header = MatrixRoutingHeader {
            source: MatrixCoordinate { x: 0, y: 0, z: 0 },
            destination: MatrixCoordinate { x: 5, y: 5, z: 0 },
            path: vec![],
        };
        header.to_bytes()
    }

    #[test]
    fn test_policy_pos_required_drops_short_packet() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.requires_pos = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Too short for PoS header (88 bytes needed)
        let packet = vec![0u8; 50];
        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("too short for PoS"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_policy_pos_required_passes_valid_packet() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.requires_pos = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = valid_pos_bytes();
        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_policy_pos_required_drops_invalid_proof() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.requires_pos = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Build a PoS header with invalid who (bad algorithm indicator)
        let mut bad_who = [0xFF; 32];
        bad_who[0] = 0x99; // invalid algorithm
        let header = ProofOfStateHeader {
            who: bad_who,
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        let packet = header.to_bytes();
        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("PoS validation failed"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_policy_asset_hash_required_drops_short_packet() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.validate_asset_hash = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Too short for asset hash header (72 bytes needed, 0 offset)
        let packet = vec![0u8; 50];
        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("too short for asset hash"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_policy_asset_hash_required_passes_valid() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.validate_asset_hash = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = valid_asset_hash_bytes();
        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_policy_asset_hash_drops_invalid_shard() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.validate_asset_hash = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let bad_header = AssetHashHeader {
            asset_id: [0x01; 32],
            hash: [0x02; 32],
            shard_count: 10,
            shard_index: 10, // >= shard_count
        };
        let packet = bad_header.to_bytes();
        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("Invalid shard indices"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_policy_routing_required_drops_short_packet() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.check_matrix_routing = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = vec![0u8; 5];
        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("too short for routing"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_policy_routing_required_passes_valid() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.check_matrix_routing = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let packet = valid_routing_bytes();
        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_policy_all_flags_combined() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_default_policy(ValidationPolicy::strict());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Build combined packet: PoS (88) + AssetHash (72) + Routing (12+)
        let mut packet = valid_pos_bytes();
        packet.extend_from_slice(&valid_asset_hash_bytes());
        packet.extend_from_slice(&valid_routing_bytes());

        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_policy_all_flags_drops_when_pos_invalid() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_default_policy(ValidationPolicy::strict());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Build combined packet with bad PoS
        let mut bad_who = [0xFF; 32];
        bad_who[0] = 0x99;
        let bad_pos = ProofOfStateHeader {
            who: bad_who,
            what: valid_what(),
            when: now_micros(),
            where_: valid_where(),
        };
        let mut packet = bad_pos.to_bytes();
        packet.extend_from_slice(&valid_asset_hash_bytes());
        packet.extend_from_slice(&valid_routing_bytes());

        let decision = mgr.validate_packet(0, &packet);
        match decision {
            PacketDecision::Drop { reason } => {
                assert!(reason.contains("PoS validation failed"));
            }
            other => unreachable!("test: expected Drop, got {:?}", other),
        }
    }

    #[test]
    fn test_permissive_policy_skips_all_checks() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        pm.set_default_policy(ValidationPolicy::permissive());
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // Small garbage packet passes with permissive policy
        let packet = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    #[test]
    fn test_pos_offset_for_asset_hash_check() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut policy = ValidationPolicy::permissive();
        policy.requires_pos = true;
        policy.validate_asset_hash = true;
        pm.set_default_policy(policy);
        let mgr = XdpManager::new(pm).expect("test: create xdp manager");

        // PoS header (88 bytes) + Asset hash header (72 bytes) = 160 bytes needed
        let mut packet = valid_pos_bytes();
        packet.extend_from_slice(&valid_asset_hash_bytes());
        let decision = mgr.validate_packet(0, &packet);
        assert_eq!(decision, PacketDecision::Pass);
    }

    // -------------------------------------------------------------------
    // Offload policy tests
    // -------------------------------------------------------------------

    #[test]
    fn test_offload_policy_default() {
        assert_eq!(OffloadPolicy::default(), OffloadPolicy::Disabled);
    }

    #[test]
    fn test_offload_policy_set() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");
        assert_eq!(mgr.offload_policy, OffloadPolicy::Disabled);

        mgr.set_offload_policy(OffloadPolicy::Opportunistic);
        assert_eq!(mgr.offload_policy, OffloadPolicy::Opportunistic);

        mgr.set_offload_policy(OffloadPolicy::Required);
        assert_eq!(mgr.offload_policy, OffloadPolicy::Required);
    }

    #[test]
    fn test_attach_offload_falls_back_to_native() {
        // loopback does not support XDP offload, so Offload mode should
        // fall back to Native when policy is Disabled (default)
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let result = mgr.attach_with_mode("lo", XdpAttachMode::Offload);
        assert!(result.is_ok(), "test: offload on lo should fall back, not error");

        // Verify attachment was tracked
        let attached = mgr.attached.read();
        assert!(attached.contains_key("lo"));
    }

    #[test]
    fn test_attach_offload_required_fails_on_unsupported_nic() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");
        mgr.set_offload_policy(OffloadPolicy::Required);

        // loopback does not support XDP offload
        let result = mgr.attach_with_mode("lo", XdpAttachMode::Offload);
        assert!(result.is_err(), "test: required offload on lo should fail");

        let err_msg = result.expect_err("test: should be error").to_string();
        assert!(
            err_msg.contains("does not support XDP offload"),
            "test: error should mention offload unsupported, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_attach_offload_opportunistic_falls_back() {
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");
        mgr.set_offload_policy(OffloadPolicy::Opportunistic);

        // loopback does not support offload - should succeed with fallback
        let result = mgr.attach_with_mode("lo", XdpAttachMode::Offload);
        assert!(result.is_ok(), "test: opportunistic offload on lo should fall back");
    }

    #[test]
    fn test_attach_native_mode_unaffected_by_offload_policy() {
        // Native and Generic modes should not be affected by offload policy
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");
        mgr.set_offload_policy(OffloadPolicy::Required);

        let result = mgr.attach_with_mode("lo", XdpAttachMode::Native);
        assert!(result.is_ok(), "test: native mode should not check offload");

        let _ = mgr.detach("lo");

        let result = mgr.attach_with_mode("lo", XdpAttachMode::Generic);
        assert!(result.is_ok(), "test: generic mode should not check offload");
    }

    // -------------------------------------------------------------------
    // KernelPosConfig tests
    // -------------------------------------------------------------------

    #[test]
    fn test_kernel_pos_config_default() {
        let cfg = KernelPosConfig::default();
        assert_eq!(cfg.min_difficulty, 8);
        assert_eq!(cfg.max_timestamp_skew_ns, 5 * 60 * 1_000_000_000);
        assert_eq!(cfg.validation_ttl_ns, 60 * 60 * 1_000_000_000);
        assert!(cfg.enabled);
    }

    #[test]
    fn test_set_kernel_pos_config_no_xdp() {
        // Calling set_kernel_pos_config without XDP attached should succeed
        let pm = PolicyManager::new().expect("test: create policy manager");
        let mut mgr = XdpManager::new(pm).expect("test: create xdp manager");

        let cfg = KernelPosConfig::default();
        let result = mgr.set_kernel_pos_config(&cfg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kernel_pos_config_serialization() {
        let cfg = KernelPosConfig {
            min_difficulty: 16,
            max_timestamp_skew_ns: 300_000_000_000, // 5 min in ns
            validation_ttl_ns: 3_600_000_000_000,   // 1 hour in ns
            enabled: true,
        };

        let bytes = cfg.to_bytes();
        assert_eq!(bytes.len(), 24);

        // Verify field layout
        let difficulty = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(difficulty, 16);

        let skew = u64::from_le_bytes([
            bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11],
        ]);
        assert_eq!(skew, 300_000_000_000);

        let ttl = u64::from_le_bytes([
            bytes[12], bytes[13], bytes[14], bytes[15],
            bytes[16], bytes[17], bytes[18], bytes[19],
        ]);
        assert_eq!(ttl, 3_600_000_000_000);

        let enabled = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        assert_eq!(enabled, 1);
    }

    #[test]
    fn test_kernel_pos_config_serialization_roundtrip() {
        let original = KernelPosConfig {
            min_difficulty: 24,
            max_timestamp_skew_ns: 42_000_000,
            validation_ttl_ns: 99_000_000_000,
            enabled: false,
        };

        let bytes = original.to_bytes();
        let decoded = KernelPosConfig::from_bytes(&bytes)
            .expect("test: from_bytes should succeed with 24 bytes");

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_kernel_pos_config_from_bytes_too_short() {
        let short = [0u8; 23];
        assert!(KernelPosConfig::from_bytes(&short).is_none());
    }

    #[test]
    fn test_kernel_pos_config_disabled_serialization() {
        let cfg = KernelPosConfig {
            min_difficulty: 0,
            max_timestamp_skew_ns: 0,
            validation_ttl_ns: 0,
            enabled: false,
        };
        let bytes = cfg.to_bytes();
        // All bytes should be zero
        assert_eq!(bytes, [0u8; 24]);
    }
}
