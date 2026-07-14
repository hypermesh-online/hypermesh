// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh eBPF Intelligence Layer
//!
//! THE single source of truth for all eBPF code in the HyperMesh workspace.
//! Every packet entering a HyperMesh node goes through this crate.
//!
//! Three execution paths:
//! 1. **Zero-copy transfer** (AF_XDP -> STOQ) via XDP_REDIRECT
//! 2. **Delegate** to another matrix node via XDP_TX/forward
//! 3. **Local execution** via XDP_PASS to userspace
//!
//! STOQ and blockmatrix are CONSUMERS of this crate, not implementors.
//!
//! ## Modules
//!
//! - `capabilities` - System capability detection (XDP, AF_XDP, kernel version)
//! - `xdp` - Unified XDP program management and the allowlist datapath
//! - `af_xdp` - AF_XDP zero-copy socket management
//! - `loader` - eBPF program compiler and kernel loader
//! - `hooks` - Validation hook traits for STOQ/blockmatrix to implement
//! - `policy_maps` - eBPF policy map management
//! - `validation` - signing-algorithm indicator constants
//! - `metrics` - Unified intelligence + transport metrics

pub mod af_xdp;
pub mod capabilities;
pub mod hooks;
pub mod loader;
pub mod metrics;
pub mod policy_maps;
pub mod queue_balancer;
pub mod validation;
pub mod xdp;

/// Re-export aya when kernel-attach is enabled, allowing downstream crates
/// to use the same aya version for BPF operations.
#[cfg(feature = "kernel-attach")]
pub use aya;

// Re-export key types for convenience
pub use af_xdp::{AfXdpManager, AfXdpSocket, AfXdpStats, RingConfig, UmemConfig};
pub use capabilities::{EbpfCapabilities, NicCapabilities};
pub use hooks::{
    CertificateValidator, ExtensionValidator, PacketValidator, PassThroughValidator,
    ValidationHooks,
};
pub use loader::{EbpfLoader, ProgramType};
pub use metrics::{HyperMeshMetrics, HyperMeshMetricsCollector, TransportMetrics};
pub use policy_maps::{PolicyManager, ValidationPolicy};
pub use queue_balancer::{
    FlowHashBalancer, LeastLoadedBalancer, MultiQueueManager, PacketHint, QueueBalancer,
    QueueMetrics, RoundRobinBalancer,
};
pub use validation::{ALG_ECDSA, ALG_ED25519, ALG_FALCON_1024};
pub use xdp::{
    FilterAction, KernelPosConfig, OffloadPolicy, PacketDecision, XdpAttachMode, XdpFilterConfig,
    XdpManager, XdpStats,
};

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use hypermesh_lib::{ContentHash, MatrixPosition, NetworkId, PrivacyMode};

// -----------------------------------------------------------------------
// Configuration
// -----------------------------------------------------------------------

/// Configuration for the HyperMesh eBPF subsystem
#[derive(Debug, Clone, Default)]
pub struct EbpfConfig {
    /// XDP filter configuration
    pub xdp_config: XdpFilterConfig,
    /// UMEM configuration for AF_XDP sockets
    pub umem_config: UmemConfig,
    /// Ring buffer configuration for AF_XDP sockets
    pub ring_config: RingConfig,
}

// -----------------------------------------------------------------------
// Error type
// -----------------------------------------------------------------------

/// eBPF subsystem error
#[derive(Debug, thiserror::Error)]
pub enum EbpfError {
    #[error("XDP error: {0}")]
    Xdp(String),
    #[error("AF_XDP error: {0}")]
    AfXdp(String),
    #[error("Policy error: {0}")]
    Policy(String),
    #[error("Loader error: {0}")]
    Loader(String),
    #[error("Capability error: {0}")]
    Capability(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// -----------------------------------------------------------------------
// Shard metadata for asset hash registration
// -----------------------------------------------------------------------

/// Metadata for a registered shard
#[derive(Debug, Clone)]
pub struct ShardMetadata {
    /// Shard index within the asset
    pub shard_index: u32,
    /// Total shard count for this asset
    pub shard_count: u32,
    /// Matrix position where shard is stored
    pub position: MatrixPosition,
}

/// Discretized matrix position key for HashMap lookups.
///
/// Converts floating-point MatrixPosition coordinates to integer keys
/// by truncating to i64, enabling use as HashMap keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MatrixPositionKey(pub i64, pub i64, pub i64);

impl From<&MatrixPosition> for MatrixPositionKey {
    fn from(pos: &MatrixPosition) -> Self {
        Self(pos.x as i64, pos.y as i64, pos.z as i64)
    }
}

impl From<MatrixPosition> for MatrixPositionKey {
    fn from(pos: MatrixPosition) -> Self {
        Self::from(&pos)
    }
}

/// Encode a `MatrixPosition` as the raw 12-byte wire form (3x f32 LE) used
/// as the `routing_map` key and carried in the on-wire MATRIX extension
/// header. The f64 coordinates are narrowed to f32 to match the header
/// layout exactly (the kernel keys on these raw bytes to avoid float math).
pub fn matrix_position_to_wire_bytes(pos: &MatrixPosition) -> [u8; 12] {
    let mut buf = [0u8; 12];
    buf[0..4].copy_from_slice(&(pos.x as f32).to_le_bytes());
    buf[4..8].copy_from_slice(&(pos.y as f32).to_le_bytes());
    buf[8..12].copy_from_slice(&(pos.z as f32).to_le_bytes());
    buf
}

// -----------------------------------------------------------------------
// HyperMeshEbpf - the unified orchestrator
// -----------------------------------------------------------------------

/// HyperMesh eBPF orchestrator - the single entry point for all eBPF operations.
///
/// Combines XDP management, AF_XDP sockets, policy enforcement, validation,
/// and metrics collection into one unified interface.
///
/// STOQ calls `attach_xdp()`, `create_af_xdp_socket()` for transport.
/// BlockMatrix calls `set_privacy_tier()`, `register_asset_hash()` for policy.
/// Both call `metrics()` for observability.
pub struct HyperMeshEbpf {
    /// System capability detection
    caps: EbpfCapabilities,
    /// Unified XDP manager (packet filtering + validation)
    xdp_manager: Option<XdpManager>,
    /// AF_XDP socket manager (zero-copy)
    af_xdp_manager: AfXdpManager,
    /// Policy manager (shared with XDP manager)
    policy_manager: PolicyManager,
    /// Metrics collector (intelligence + transport)
    metrics_collector: HyperMeshMetricsCollector,
    /// Validation hooks (for STOQ/blockmatrix to register validators)
    validation_hooks: ValidationHooks,
    /// eBPF program loader
    loader: EbpfLoader,
    /// Routing rules: destination matrix position -> next-hop matrix position
    routing_rules: Arc<RwLock<HashMap<MatrixPositionKey, MatrixPosition>>>,
    /// Registered asset hashes with shard metadata
    asset_hashes: Arc<RwLock<HashMap<[u8; 32], ShardMetadata>>>,
    /// PoS validation status per content hash (true = valid)
    pos_validations: Arc<RwLock<HashMap<[u8; 32], bool>>>,
}

impl HyperMeshEbpf {
    /// Create a new HyperMesh eBPF orchestrator with default configuration
    pub fn new(config: EbpfConfig) -> Result<Self, EbpfError> {
        let caps = EbpfCapabilities::detect();
        let policy_manager = PolicyManager::new().map_err(|e| EbpfError::Policy(e.to_string()))?;

        let af_xdp_manager = AfXdpManager::with_config(config.umem_config, config.ring_config)
            .map_err(|e| EbpfError::AfXdp(e.to_string()))?;

        let metrics_collector = HyperMeshMetricsCollector::new().map_err(EbpfError::Other)?;

        let loader = EbpfLoader::new();

        tracing::info!(
            "HyperMesh eBPF initialized (XDP: {}, AF_XDP: {})",
            caps.xdp_available,
            caps.af_xdp_available
        );

        Ok(Self {
            caps,
            xdp_manager: None,
            af_xdp_manager,
            policy_manager,
            metrics_collector,
            validation_hooks: ValidationHooks::new(),
            loader,
            routing_rules: Arc::new(RwLock::new(HashMap::new())),
            asset_hashes: Arc::new(RwLock::new(HashMap::new())),
            pos_validations: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get detected system capabilities
    pub fn capabilities(&self) -> &EbpfCapabilities {
        &self.caps
    }

    /// Detect NIC capabilities for an interface
    pub fn detect_nic_capabilities(&mut self, interface: &str) -> NicCapabilities {
        self.caps.detect_nic(interface);
        self.caps.nic_capabilities.clone().unwrap_or_default()
    }

    /// Set the hardware offload policy
    pub fn set_offload_policy(&mut self, policy: OffloadPolicy) {
        if let Some(ref mut xdp) = self.xdp_manager {
            xdp.set_offload_policy(policy);
        }
    }

    // -------------------------------------------------------------------
    // Configuration (called by blockmatrix/trustchain)
    // -------------------------------------------------------------------

    /// Set privacy tier policy for a network
    #[allow(unused_variables)]
    pub fn set_privacy_tier(&self, network: NetworkId, tier: PrivacyMode) -> Result<(), EbpfError> {
        let ebpf_tier = tier.to_ebpf_u8();
        let policy = ValidationPolicy::for_privacy_tier(ebpf_tier);

        // Use first 8 bytes of NetworkId as connection key
        let net_bytes = &network.0;
        let key = u64::from_le_bytes([
            net_bytes[0],
            net_bytes[1],
            net_bytes[2],
            net_bytes[3],
            net_bytes[4],
            net_bytes[5],
            net_bytes[6],
            net_bytes[7],
        ]);

        self.policy_manager.set_policy(key, policy);
        tracing::debug!("Privacy tier set for network: ebpf_u8={}", ebpf_tier);

        // Push the updated policy set into the kernel `policy_map` so the
        // per-source gate reflects the network's privacy tier. No-op when
        // the XDP program is not attached (userspace-only tier).
        if let Some(ref xdp) = self.xdp_manager {
            xdp.sync_policies_to_bpf()
                .map_err(|e| EbpfError::Xdp(e.to_string()))?;
        }

        Ok(())
    }

    /// Mark an authenticated peer's source address as PoS-validated in the
    /// kernel maps.
    ///
    /// P5 unification: call this from the SAME event that registers an
    /// authenticated peer in userspace (`register_authenticated_peer`).
    /// It writes:
    ///   - `pos_header_map[src_ip].validated = 1` (the kernel fast-path
    ///     admits this source's HyperMesh traffic), and
    ///   - `policy_map[src_ip].requires_pos = 1` (so the gate is armed for
    ///     that source),
    /// keyed on the SAME IPv6 source address P1 authenticated. Removing a
    /// peer (`valid = false`) marks the source invalid. No-op (graceful)
    /// when the XDP program is not attached.
    ///
    /// `algorithm` is the signing algorithm indicator (0x01 FALCON, 0x02
    /// Ed25519, 0x03 ECDSA); HyperMesh peers use FALCON-1024.
    pub fn set_peer_pos_validated(
        &self,
        src_ip: [u8; 16],
        valid: bool,
        algorithm: u8,
    ) -> Result<(), EbpfError> {
        if let Some(ref xdp) = self.xdp_manager {
            xdp.update_pos_header_map(src_ip, valid, algorithm, 0)
                .map_err(|e| EbpfError::Xdp(e.to_string()))?;
            // Arm the per-source policy: authenticated peers require PoS.
            let policy = ValidationPolicy::for_privacy_tier(3);
            xdp.update_policy_for_source(src_ip, &policy)
                .map_err(|e| EbpfError::Xdp(e.to_string()))?;
        } else {
            tracing::debug!(
                "No XDP manager attached; peer PoS validation ({}) not synced to kernel",
                valid
            );
        }
        Ok(())
    }

    /// Set a routing rule for matrix topology forwarding.
    ///
    /// Stores the destination -> next-hop mapping so that `validate_packet()`
    /// can return `Forward` decisions when a packet's destination matches.
    /// In production with kernel-attach, this would also update a
    /// BPF_MAP_TYPE_HASH map for kernel-level forwarding.
    pub fn set_routing_rule(
        &self,
        dest: MatrixPosition,
        next_hop: MatrixPosition,
    ) -> Result<(), EbpfError> {
        let key = MatrixPositionKey::from(&dest);
        self.routing_rules.write().insert(key, next_hop);

        tracing::debug!(
            "Routing rule set: dest=({},{},{}) -> next_hop=({},{},{})",
            dest.x,
            dest.y,
            dest.z,
            next_hop.x,
            next_hop.y,
            next_hop.z
        );

        // P5 step 6: mirror the routing rule into the kernel `routing_map`
        // so the XDP_TX delegation branch can forward matching packets. The
        // key is the raw 12-byte matrix position (3x f32 LE), matching the
        // on-wire MATRIX extension header exactly. `egress_ifindex` is left
        // 0 here (skeleton) — the concrete next-hop ifindex is resolved by
        // the ShardLocationIndex feed (see `set_matrix_route_active`), and
        // the live cross-device rewrite is deferred to P8. No-op when the
        // XDP program is not attached.
        if let Some(ref xdp) = self.xdp_manager {
            let pos_bytes = matrix_position_to_wire_bytes(&dest);
            xdp.update_routing_map(pos_bytes, 0, true)
                .map_err(|e| EbpfError::Xdp(e.to_string()))?;
        }

        Ok(())
    }

    /// Activate (or deactivate) a kernel forwarding rule for a matrix
    /// position with a concrete egress interface.
    ///
    /// P5 step 6: this is the entry point the `ShardLocationIndex` feed
    /// uses to install next-hop forwarding — when the swarm provider index
    /// learns that shards for a destination live behind interface
    /// `egress_ifindex`, it activates the `routing_map` entry so the XDP
    /// program delegates matching traffic via XDP_TX. No-op (graceful) when
    /// the XDP program is not attached.
    pub fn set_matrix_route_active(
        &self,
        dest: MatrixPosition,
        egress_ifindex: u32,
        active: bool,
    ) -> Result<(), EbpfError> {
        if let Some(ref xdp) = self.xdp_manager {
            let pos_bytes = matrix_position_to_wire_bytes(&dest);
            xdp.update_routing_map(pos_bytes, egress_ifindex, active)
                .map_err(|e| EbpfError::Xdp(e.to_string()))?;
        }
        Ok(())
    }

    /// Register an asset hash for validation.
    ///
    /// Stores the hash -> shard metadata mapping for userspace validation.
    /// In production with kernel-attach, this would also insert into a BPF
    /// hash map so the XDP program can validate asset hashes at kernel level.
    pub fn register_asset_hash(
        &self,
        hash: ContentHash,
        metadata: ShardMetadata,
    ) -> Result<(), EbpfError> {
        self.asset_hashes.write().insert(hash.0, metadata.clone());

        tracing::debug!(
            "Asset hash registered: shard {}/{}",
            metadata.shard_index,
            metadata.shard_count
        );

        // P5 unification: mirror the userspace registration straight into
        // the kernel `asset_hash_map`, keyed by the SAME 32-byte content
        // hash the userspace gate uses. `registered=true` because this
        // asset is on the blockchain by the time it is registered here.
        // No-op (graceful) when the XDP program is not attached.
        if let Some(ref xdp) = self.xdp_manager {
            xdp.update_asset_hash_map(hash.0, metadata.shard_count, true)
                .map_err(|e| EbpfError::Xdp(e.to_string()))?;
        }

        Ok(())
    }

    /// Set PoS validation status for a CONTENT hash (userspace cache).
    ///
    /// This is the content-addressed userspace validation cache, keyed by
    /// the 32-byte BLAKE3 content hash. It is distinct from the kernel
    /// `pos_header_map`, which is keyed by the peer's 16-byte IPv6 SOURCE
    /// address — to mirror an authenticated peer into the kernel PoS gate,
    /// use [`Self::set_peer_pos_validated`] (source-keyed), not this method.
    pub fn set_pos_validation(&self, hash: ContentHash, valid: bool) -> Result<(), EbpfError> {
        self.pos_validations.write().insert(hash.0, valid);
        tracing::debug!(
            "PoS content-validation cached for {}: valid={}",
            hex::encode(&hash.0[..8]),
            valid
        );
        Ok(())
    }

    /// Set kernel-side PoS validation configuration.
    ///
    /// Configures the non-cryptographic structural checks that the XDP
    /// program applies at wire speed (algorithm validation, PoW difficulty,
    /// cache TTL).  Full asymmetric crypto verification remains in userspace.
    ///
    /// Delegates to `XdpManager::set_kernel_pos_config()` when the XDP
    /// manager is attached.  Returns Ok if no XDP manager is present.
    pub fn set_kernel_pos_config(&mut self, config: &KernelPosConfig) -> Result<(), EbpfError> {
        if let Some(ref mut xdp) = self.xdp_manager {
            xdp.set_kernel_pos_config(config)
                .map_err(|e| EbpfError::Xdp(e.to_string()))?;
        } else {
            tracing::debug!(
                "No XDP manager attached; kernel PoS config not synced \
                 (difficulty={}, ttl={}ns, enabled={})",
                config.min_difficulty,
                config.validation_ttl_ns,
                config.enabled
            );
        }
        Ok(())
    }

    // -------------------------------------------------------------------
    // Transport (called by STOQ)
    // -------------------------------------------------------------------

    /// Attach XDP program to a network interface
    pub fn attach_xdp(&mut self, interface: &str) -> Result<(), EbpfError> {
        let mut xdp = XdpManager::new(self.policy_manager.clone())
            .map_err(|e| EbpfError::Xdp(e.to_string()))?;
        xdp.attach(interface)
            .map_err(|e| EbpfError::Xdp(e.to_string()))?;
        self.xdp_manager = Some(xdp);
        Ok(())
    }

    /// Detach XDP program
    pub fn detach_xdp(&mut self) -> Result<(), EbpfError> {
        if let Some(mut xdp) = self.xdp_manager.take() {
            xdp.detach_all()
                .map_err(|e| EbpfError::Xdp(e.to_string()))?;
            tracing::info!("HyperMesh XDP detached");
        }
        Ok(())
    }

    /// Create an AF_XDP socket for zero-copy I/O on an interface
    pub fn create_af_xdp_socket(
        &mut self,
        interface: &str,
        queue_id: u32,
    ) -> Result<AfXdpSocket, EbpfError> {
        self.af_xdp_manager
            .create_socket(interface, queue_id)
            .map_err(|e| EbpfError::AfXdp(e.to_string()))
    }

    /// Create a multi-queue AF_XDP setup with load balancing.
    ///
    /// Allocates `queue_count` AF_XDP sockets on `interface` and wraps
    /// them in a [`MultiQueueManager`] that uses `balancer` to steer
    /// packets across queues.
    pub fn create_multi_queue(
        &mut self,
        interface: &str,
        queue_count: u32,
        balancer: Box<dyn QueueBalancer>,
    ) -> Result<MultiQueueManager, EbpfError> {
        MultiQueueManager::new(&mut self.af_xdp_manager, balancer, interface, queue_count)
            .map_err(|e| EbpfError::AfXdp(e.to_string()))
    }

    // -------------------------------------------------------------------
    // Hooks
    // -------------------------------------------------------------------

    /// Register validation hooks (called by STOQ/blockmatrix at startup)
    pub fn set_validation_hooks(&mut self, hooks: ValidationHooks) {
        self.validation_hooks = hooks;
    }

    /// Get reference to validation hooks
    pub fn validation_hooks(&self) -> &ValidationHooks {
        &self.validation_hooks
    }

    // -------------------------------------------------------------------
    // Metrics
    // -------------------------------------------------------------------

    /// Get the metrics collector reference
    pub fn metrics(&self) -> &HyperMeshMetricsCollector {
        &self.metrics_collector
    }

    /// Collect a snapshot of all metrics
    pub fn collect_metrics(&self) -> HyperMeshMetrics {
        self.metrics_collector.collect()
    }

    // -------------------------------------------------------------------
    // Loader
    // -------------------------------------------------------------------

    /// Get mutable reference to the eBPF program loader
    pub fn loader_mut(&mut self) -> &mut EbpfLoader {
        &mut self.loader
    }

    /// Get policy manager reference
    pub fn policy_manager(&self) -> &PolicyManager {
        &self.policy_manager
    }

    /// Get mutable policy manager
    pub fn policy_manager_mut(&mut self) -> &mut PolicyManager {
        &mut self.policy_manager
    }

    // -------------------------------------------------------------------
    // State accessors (routing rules, asset hashes, PoS validations)
    // -------------------------------------------------------------------

    /// Look up the next-hop for a destination matrix position.
    ///
    /// Returns `Some(next_hop)` if a routing rule exists for the
    /// discretized destination coordinates.
    pub fn get_routing_rule(&self, dest: &MatrixPosition) -> Option<MatrixPosition> {
        let key = MatrixPositionKey::from(dest);
        self.routing_rules.read().get(&key).copied()
    }

    /// Get the number of stored routing rules.
    pub fn routing_rule_count(&self) -> usize {
        self.routing_rules.read().len()
    }

    /// Look up shard metadata for a registered asset hash.
    pub fn get_asset_hash(&self, hash: &ContentHash) -> Option<ShardMetadata> {
        self.asset_hashes.read().get(&hash.0).cloned()
    }

    /// Get the number of registered asset hashes.
    pub fn asset_hash_count(&self) -> usize {
        self.asset_hashes.read().len()
    }

    /// Look up PoS validation status for a content hash.
    ///
    /// Returns `Some(true)` if validated, `Some(false)` if invalidated,
    /// `None` if not registered.
    pub fn get_pos_validation(&self, hash: &ContentHash) -> Option<bool> {
        self.pos_validations.read().get(&hash.0).copied()
    }

    /// Get the number of stored PoS validation entries.
    pub fn pos_validation_count(&self) -> usize {
        self.pos_validations.read().len()
    }
}

impl Default for HyperMeshEbpf {
    fn default() -> Self {
        Self::new(EbpfConfig::default()).expect("ebpf: failed to create default HyperMeshEbpf")
    }
}

impl Drop for HyperMeshEbpf {
    fn drop(&mut self) {
        let _ = self.detach_xdp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hypermesh_ebpf_creation() {
        let ebpf = HyperMeshEbpf::new(EbpfConfig::default());
        assert!(ebpf.is_ok());

        let ebpf = ebpf.expect("test: create HyperMeshEbpf");
        let caps = ebpf.capabilities();
        assert!(!caps.kernel_version.is_empty());
    }

    #[test]
    fn test_default_creation() {
        let ebpf = HyperMeshEbpf::default();
        assert!(!ebpf.capabilities().kernel_version.is_empty());
    }

    #[test]
    fn test_metrics_collection() {
        let ebpf = HyperMeshEbpf::default();
        let metrics = ebpf.collect_metrics();
        assert_eq!(metrics.pos_metrics.total_validations, 0);
        assert_eq!(metrics.transport_metrics.total_packets, 0);
    }

    #[test]
    fn test_set_privacy_tier() {
        let ebpf = HyperMeshEbpf::default();
        let network = NetworkId([1u8; 16]);
        let result = ebpf.set_privacy_tier(network, PrivacyMode::PUBLIC);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_routing_rule() {
        let ebpf = HyperMeshEbpf::default();
        let dest = MatrixPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let next_hop = MatrixPosition {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let result = ebpf.set_routing_rule(dest, next_hop);
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_asset_hash() {
        let ebpf = HyperMeshEbpf::default();
        let hash = ContentHash::from_bytes([42u8; 32]);
        let metadata = ShardMetadata {
            shard_index: 0,
            shard_count: 10,
            position: MatrixPosition {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        };
        let result = ebpf.register_asset_hash(hash, metadata);
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------
    // State storage and retrieval tests
    // -------------------------------------------------------------------

    #[test]
    fn test_routing_rule_stores_and_retrieves() {
        let ebpf = HyperMeshEbpf::default();
        let dest = MatrixPosition {
            x: 10.0,
            y: 20.0,
            z: 30.0,
        };
        let next_hop = MatrixPosition {
            x: 11.0,
            y: 21.0,
            z: 31.0,
        };

        assert_eq!(ebpf.routing_rule_count(), 0);

        ebpf.set_routing_rule(dest, next_hop)
            .expect("test: set routing rule");

        assert_eq!(ebpf.routing_rule_count(), 1);

        let retrieved = ebpf.get_routing_rule(&dest);
        assert!(retrieved.is_some(), "routing rule should be retrievable");

        let hop = retrieved.expect("test: unwrap routing rule");
        assert_eq!(hop.x, 11.0);
        assert_eq!(hop.y, 21.0);
        assert_eq!(hop.z, 31.0);

        // Non-existent destination returns None
        let missing = MatrixPosition {
            x: 99.0,
            y: 99.0,
            z: 99.0,
        };
        assert!(ebpf.get_routing_rule(&missing).is_none());
    }

    #[test]
    fn test_routing_rule_overwrites() {
        let ebpf = HyperMeshEbpf::default();
        let dest = MatrixPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let hop_a = MatrixPosition {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let hop_b = MatrixPosition {
            x: 7.0,
            y: 8.0,
            z: 9.0,
        };

        ebpf.set_routing_rule(dest, hop_a)
            .expect("test: set first rule");
        ebpf.set_routing_rule(dest, hop_b)
            .expect("test: set second rule");

        assert_eq!(ebpf.routing_rule_count(), 1);
        let hop = ebpf
            .get_routing_rule(&dest)
            .expect("test: get overwritten rule");
        assert_eq!(hop.x, 7.0);
    }

    #[test]
    fn test_asset_hash_stores_and_retrieves() {
        let ebpf = HyperMeshEbpf::default();
        let hash = ContentHash::from_bytes([0xABu8; 32]);
        let metadata = ShardMetadata {
            shard_index: 3,
            shard_count: 14,
            position: MatrixPosition {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
        };

        assert_eq!(ebpf.asset_hash_count(), 0);

        ebpf.register_asset_hash(hash, metadata)
            .expect("test: register asset hash");

        assert_eq!(ebpf.asset_hash_count(), 1);

        let retrieved = ebpf.get_asset_hash(&hash);
        assert!(retrieved.is_some(), "asset hash should be retrievable");

        let meta = retrieved.expect("test: unwrap asset hash metadata");
        assert_eq!(meta.shard_index, 3);
        assert_eq!(meta.shard_count, 14);
        assert_eq!(meta.position.x, 5.0);

        // Non-existent hash returns None
        let missing = ContentHash::from_bytes([0xFFu8; 32]);
        assert!(ebpf.get_asset_hash(&missing).is_none());
    }

    #[test]
    fn test_pos_validation_stores_and_retrieves() {
        let ebpf = HyperMeshEbpf::default();
        let hash_valid = ContentHash::from_bytes([0x01u8; 32]);
        let hash_invalid = ContentHash::from_bytes([0x02u8; 32]);

        assert_eq!(ebpf.pos_validation_count(), 0);

        ebpf.set_pos_validation(hash_valid, true)
            .expect("test: set valid pos");
        ebpf.set_pos_validation(hash_invalid, false)
            .expect("test: set invalid pos");

        assert_eq!(ebpf.pos_validation_count(), 2);

        assert_eq!(ebpf.get_pos_validation(&hash_valid), Some(true));
        assert_eq!(ebpf.get_pos_validation(&hash_invalid), Some(false));

        // Non-existent returns None
        let missing = ContentHash::from_bytes([0xFFu8; 32]);
        assert_eq!(ebpf.get_pos_validation(&missing), None);
    }

    #[test]
    fn test_pos_validation_overwrite() {
        let ebpf = HyperMeshEbpf::default();
        let hash = ContentHash::from_bytes([0x10u8; 32]);

        ebpf.set_pos_validation(hash, true)
            .expect("test: set valid");
        assert_eq!(ebpf.get_pos_validation(&hash), Some(true));

        // Overwrite with false
        ebpf.set_pos_validation(hash, false)
            .expect("test: set invalid");
        assert_eq!(ebpf.get_pos_validation(&hash), Some(false));
        assert_eq!(ebpf.pos_validation_count(), 1);
    }

    // -------------------------------------------------------------------
    // NIC capabilities and offload policy tests
    // -------------------------------------------------------------------

    #[test]
    fn test_detect_nic_capabilities() {
        let mut ebpf = HyperMeshEbpf::default();
        assert!(ebpf.capabilities().nic_capabilities.is_none());

        let nic = ebpf.detect_nic_capabilities("lo");
        assert_eq!(nic.interface, "lo");
        assert!(!nic.supports_xdp_offload);

        // Should be cached in capabilities now
        assert!(ebpf.capabilities().nic_capabilities.is_some());
    }

    #[test]
    fn test_set_offload_policy_without_xdp() {
        // set_offload_policy is a no-op if no XDP manager is attached
        let mut ebpf = HyperMeshEbpf::default();
        ebpf.set_offload_policy(OffloadPolicy::Required);
        // Should not panic or error
    }

    // -------------------------------------------------------------------
    // KernelPosConfig orchestrator tests
    // -------------------------------------------------------------------

    #[test]
    fn test_set_kernel_pos_config_no_xdp() {
        // Without an XDP manager, set_kernel_pos_config should succeed
        let mut ebpf = HyperMeshEbpf::default();
        let cfg = KernelPosConfig::default();
        let result = ebpf.set_kernel_pos_config(&cfg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_kernel_pos_config_custom_values() {
        let mut ebpf = HyperMeshEbpf::default();
        let cfg = KernelPosConfig {
            min_difficulty: 16,
            max_timestamp_skew_ns: 10 * 60 * 1_000_000_000,
            validation_ttl_ns: 30 * 60 * 1_000_000_000,
            enabled: false,
        };
        let result = ebpf.set_kernel_pos_config(&cfg);
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------
    // P5 map-population wiring tests (userspace-verifiable, no kernel).
    //
    // Without an attached XDP program these are graceful no-ops returning
    // Ok — proving the userspace-only tier is unaffected (graceful
    // degradation). The kernel DROP == userspace-reject behaviour is
    // deferred to P8 (real load on trust.hypermesh.online).
    // -------------------------------------------------------------------

    #[test]
    fn test_set_peer_pos_validated_no_xdp_is_noop() {
        let ebpf = HyperMeshEbpf::default();
        let src_ip = [0x20u8; 16]; // IPv6 global unicast prefix
        // valid=true, algorithm=FALCON — no XDP attached → graceful Ok.
        assert!(ebpf
            .set_peer_pos_validated(src_ip, true, ALG_FALCON_1024)
            .is_ok());
        // Removing a peer (valid=false) is likewise a graceful no-op.
        assert!(ebpf
            .set_peer_pos_validated(src_ip, false, ALG_FALCON_1024)
            .is_ok());
    }

    #[test]
    fn test_set_matrix_route_active_no_xdp_is_noop() {
        let ebpf = HyperMeshEbpf::default();
        let dest = MatrixPosition {
            x: 3.0,
            y: 4.0,
            z: 5.0,
        };
        assert!(ebpf.set_matrix_route_active(dest, 2, true).is_ok());
        assert!(ebpf.set_matrix_route_active(dest, 2, false).is_ok());
    }

    #[test]
    fn test_matrix_position_wire_bytes_layout() {
        // The routing_map key is the raw 12-byte (3x f32 LE) position,
        // exactly as carried in the on-wire MATRIX extension header.
        let pos = MatrixPosition {
            x: 1.5,
            y: -2.0,
            z: 3.25,
        };
        let bytes = matrix_position_to_wire_bytes(&pos);
        assert_eq!(bytes.len(), 12);
        assert_eq!(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]), 1.5);
        assert_eq!(f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), -2.0);
        assert_eq!(f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]), 3.25);
    }

    #[test]
    fn test_register_asset_hash_no_xdp_still_stores_userspace() {
        // register_asset_hash pushes to the kernel asset_hash_map when the
        // XDP program is attached, but MUST still populate the userspace
        // HashMap when it is not (graceful degradation).
        let ebpf = HyperMeshEbpf::default();
        let hash = ContentHash::from_bytes([0x77u8; 32]);
        let metadata = ShardMetadata {
            shard_index: 2,
            shard_count: 14,
            position: MatrixPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        };
        assert!(ebpf.register_asset_hash(hash, metadata).is_ok());
        assert_eq!(ebpf.asset_hash_count(), 1);
        assert!(ebpf.get_asset_hash(&hash).is_some());
    }

    // -------------------------------------------------------------------
    // DEFERRED TO P8 (kernel runtime verification on trust.hypermesh.online)
    // -------------------------------------------------------------------

    /// P8 checklist — verify the kernel DROP mirrors the userspace reject.
    ///
    /// This CANNOT run here: loading/attaching an XDP program needs root +
    /// a writable bpffs, and injecting crafted IPv6/QUIC frames to observe
    /// XDP_DROP needs a real (or veth) NIC. On the remote:
    ///
    ///   1. `mount -t bpf bpf /sys/fs/bpf` (if not already mounted).
    ///   2. Ship `target/bpf/hypermesh_xdp.o` alongside the binary (the
    ///      musl deploy build must include it; `attach()` looks in
    ///      `/sys/fs/bpf/hypermesh_xdp` then `target/bpf/hypermesh_xdp.o`).
    ///   3. Run the node as root (or with CAP_BPF+CAP_NET_ADMIN) so
    ///      `XdpManager::attach()` succeeds instead of falling back.
    ///   4. Authenticate a peer; confirm `pos_header_map[src].validated=1`
    ///      and `policy_map[src].requires_pos=1` (`bpftool map dump`).
    ///   5. Inject a HyperMesh QUIC frame from an UNvalidated source →
    ///      expect `stats_map.packets_dropped` to increment (XDP_DROP),
    ///      matching the userspace `validate_packet` → Drop decision.
    ///   6. Register an asset; confirm `asset_hash_map[hash].registered=1`
    ///      and that a frame referencing an unregistered asset is dropped.
    ///   7. Activate a `routing_map` entry; confirm a matrix-routed frame
    ///      returns XDP_TX (packets_redirected increments).
    #[test]
    #[ignore = "P8: requires root + bpffs + real NIC to load/attach XDP and observe kernel DROP; verified on trust.hypermesh.online, not in this sandbox"]
    fn p8_kernel_drop_equals_userspace_reject() {
        // Intentionally empty — the assertion is the checklist above,
        // executed manually/scripted on the remote in P8.
    }
}
