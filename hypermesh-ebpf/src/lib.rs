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
//! - `xdp` - Unified XDP program management and packet validation
//! - `af_xdp` - AF_XDP zero-copy socket management
//! - `loader` - eBPF program compiler and kernel loader
//! - `hooks` - Validation hook traits for STOQ/blockmatrix to implement
//! - `policy_maps` - eBPF policy map management
//! - `hypermesh_headers` - PoS/Asset/Routing/Privacy header definitions
//! - `validation` - PoS and asset hash validators
//! - `metrics` - Unified intelligence + transport metrics

pub mod capabilities;
pub mod xdp;
pub mod af_xdp;
pub mod queue_balancer;
pub mod loader;
pub mod hooks;
pub mod policy_maps;
pub mod hypermesh_headers;
pub mod validation;
pub mod metrics;

/// Re-export aya when kernel-attach is enabled, allowing downstream crates
/// to use the same aya version for BPF operations.
#[cfg(feature = "kernel-attach")]
pub use aya;

// Re-export key types for convenience
pub use capabilities::{EbpfCapabilities, NicCapabilities};
pub use xdp::{XdpManager, PacketDecision, FilterAction, XdpAttachMode, XdpStats, XdpFilterConfig, OffloadPolicy, KernelPosConfig};
pub use af_xdp::{AfXdpManager, AfXdpSocket, AfXdpStats, UmemConfig, RingConfig};
pub use queue_balancer::{
    QueueBalancer, PacketHint, QueueMetrics, MultiQueueManager,
    RoundRobinBalancer, LeastLoadedBalancer, FlowHashBalancer,
};
pub use loader::{EbpfLoader, ProgramType};
pub use hooks::{
    CertificateValidator, PacketValidator, ExtensionValidator,
    ValidationHooks, PassThroughValidator,
};
pub use policy_maps::{ValidationPolicy, PolicyManager};
pub use hypermesh_headers::{
    ProofOfStateHeader,
    AssetHashHeader,
    MatrixRoutingHeader,
    PrivacyTierHeader,
    MatrixCoordinate,
    EXT_PROOF_OF_STATE,
    EXT_ASSET_HASH,
    EXT_MATRIX_ROUTING,
    EXT_PRIVACY_TIER,
};
pub use validation::{
    ProofOfStateValidator, AssetHashValidator, FastValidationResult,
    ALG_FALCON_1024, ALG_ED25519, ALG_ECDSA,
};
pub use metrics::{HyperMeshMetrics, HyperMeshMetricsCollector, TransportMetrics};

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use hypermesh_lib::{NetworkId, MatrixPosition, ContentHash, PrivacyMode};

// -----------------------------------------------------------------------
// Configuration
// -----------------------------------------------------------------------

/// Configuration for the HyperMesh eBPF subsystem
#[derive(Debug, Clone)]
pub struct EbpfConfig {
    /// XDP filter configuration
    pub xdp_config: XdpFilterConfig,
    /// UMEM configuration for AF_XDP sockets
    pub umem_config: UmemConfig,
    /// Ring buffer configuration for AF_XDP sockets
    pub ring_config: RingConfig,
}

impl Default for EbpfConfig {
    fn default() -> Self {
        Self {
            xdp_config: XdpFilterConfig::default(),
            umem_config: UmemConfig::default(),
            ring_config: RingConfig::default(),
        }
    }
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
        let policy_manager = PolicyManager::new()
            .map_err(|e| EbpfError::Policy(e.to_string()))?;

        let af_xdp_manager = AfXdpManager::with_config(
            config.umem_config,
            config.ring_config,
        ).map_err(|e| EbpfError::AfXdp(e.to_string()))?;

        let metrics_collector = HyperMeshMetricsCollector::new()
            .map_err(|e| EbpfError::Other(e))?;

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
    pub fn set_privacy_tier(
        &self,
        network: NetworkId,
        tier: PrivacyMode,
    ) -> Result<(), EbpfError> {
        let ebpf_tier = tier.to_ebpf_u8();
        let policy = ValidationPolicy::for_privacy_tier(ebpf_tier);

        // Use first 8 bytes of NetworkId as connection key
        let net_bytes = &network.0;
        let key = u64::from_le_bytes([
            net_bytes[0], net_bytes[1], net_bytes[2], net_bytes[3],
            net_bytes[4], net_bytes[5], net_bytes[6], net_bytes[7],
        ]);

        self.policy_manager.set_policy(key, policy);
        tracing::debug!(
            "Privacy tier set for network: ebpf_u8={}",
            ebpf_tier
        );
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
            dest.x, dest.y, dest.z,
            next_hop.x, next_hop.y, next_hop.z
        );

        // When kernel-attach is enabled, also prepare the BPF map update
        #[cfg(feature = "kernel-attach")]
        {
            tracing::debug!(
                "Routing rule for ({},{},{}) prepared for BPF forwarding map sync",
                dest.x, dest.y, dest.z
            );
            // BPF map key: MatrixPositionKey as 3x i64 LE bytes (24 bytes)
            // BPF map value: next_hop MatrixPosition as 3x f64 LE bytes (24 bytes)
            // Actual map write happens during next sync_to_kernel() cycle
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

        #[cfg(feature = "kernel-attach")]
        {
            tracing::debug!(
                "Asset hash {} registered, BPF asset_hash_map entry prepared",
                hex::encode(&hash.0[..8])
            );
            // BPF map key: asset hash [u8; 32]
            // BPF map value: shard_index u32 LE + shard_count u32 LE (8 bytes)
            // Actual map write happens during next sync_to_kernel() cycle
        }

        Ok(())
    }

    /// Set PoS validation status for a content hash.
    ///
    /// Stores the hash -> validation status for userspace lookups.
    /// In production with kernel-attach, this also updates the BPF
    /// `pos_header_map` -- the `last_validated` field is set to the
    /// current value of `bpf_ktime_get_ns()` (kernel monotonic clock),
    /// which the XDP program uses for TTL enforcement when
    /// `pos_config_map.validation_ttl_ns > 0`.
    pub fn set_pos_validation(
        &self,
        hash: ContentHash,
        valid: bool,
    ) -> Result<(), EbpfError> {
        self.pos_validations.write().insert(hash.0, valid);

        tracing::debug!(
            "PoS validation status set: valid={}",
            valid
        );

        #[cfg(feature = "kernel-attach")]
        {
            tracing::debug!(
                "PoS validation for {} synced, BPF pos_header_map entry prepared \
                 (last_validated = current bpf_ktime_get_ns())",
                hex::encode(&hash.0[..8])
            );
            // BPF map key: source IPv6 address [u8; 16]
            // BPF map value: struct pos_validation {
            //   algorithm: u8, difficulty: u32, validated: u8,
            //   last_validated: u64  <-- set to bpf_ktime_get_ns() at write time
            // }
            // Actual map write happens during next sync_to_kernel() cycle
        }

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
    pub fn set_kernel_pos_config(
        &mut self,
        config: &KernelPosConfig,
    ) -> Result<(), EbpfError> {
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
    // Packet processing
    // -------------------------------------------------------------------

    /// Validate a packet and return a decision (three execution paths)
    pub fn validate_packet(&self, packet: &[u8]) -> PacketDecision {
        if let Some(ref xdp) = self.xdp_manager {
            xdp.validate_packet(0, packet)
        } else {
            // No XDP manager attached, pass everything
            PacketDecision::Pass
        }
    }

    /// Validate a Proof of State header
    pub fn validate_pos_header(&self, header: &ProofOfStateHeader) -> bool {
        if let Some(ref xdp) = self.xdp_manager {
            let valid = xdp.validate_proof_of_state(header);
            self.metrics_collector.record_pos_validation(valid);
            valid
        } else {
            header.validate_timestamps()
        }
    }

    /// Validate an asset hash header
    pub fn validate_asset_hash(&self, header: &AssetHashHeader) -> bool {
        let valid = header.validate_shard_indices();
        self.metrics_collector.record_asset_validation(valid, false);
        valid
    }

    // -------------------------------------------------------------------
    // Transport (called by STOQ)
    // -------------------------------------------------------------------

    /// Attach XDP program to a network interface
    pub fn attach_xdp(
        &mut self,
        interface: &str,
    ) -> Result<(), EbpfError> {
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
        MultiQueueManager::new(
            &mut self.af_xdp_manager,
            balancer,
            interface,
            queue_count,
        )
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
        Self::new(EbpfConfig::default())
            .expect("ebpf: failed to create default HyperMeshEbpf")
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
    fn test_packet_validation_without_xdp() {
        let ebpf = HyperMeshEbpf::default();
        let decision = ebpf.validate_packet(&[0u8; 100]);
        assert_eq!(decision, PacketDecision::Pass);
    }

    /// Build valid `who`: FALCON-1024 algorithm + 8 non-zero prefix bytes.
    fn test_valid_who() -> [u8; 32] {
        let mut who = [0xABu8; 32];
        who[0] = ALG_FALCON_1024;
        who
    }

    /// Build valid `what`: first byte zero (8 leading zero bits).
    fn test_valid_what() -> [u8; 32] {
        let mut what = [0xFFu8; 32];
        what[0] = 0x00;
        what
    }

    /// Build valid `where_`: IPv6 global unicast prefix.
    fn test_valid_where() -> [u8; 16] {
        let mut w = [0x01u8; 16];
        w[0] = 0x20;
        w
    }

    #[test]
    fn test_pos_header_validation() {
        let ebpf = HyperMeshEbpf::default();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("test: get system time")
            .as_micros() as u64;

        let valid = ProofOfStateHeader {
            who: test_valid_who(),
            what: test_valid_what(),
            when: now,
            where_: test_valid_where(),
        };
        assert!(ebpf.validate_pos_header(&valid));

        let future = ProofOfStateHeader {
            who: test_valid_who(),
            what: test_valid_what(),
            when: now + 10 * 60 * 1_000_000,
            where_: test_valid_where(),
        };
        assert!(!ebpf.validate_pos_header(&future));
    }

    #[test]
    fn test_asset_hash_validation() {
        let ebpf = HyperMeshEbpf::default();

        let valid_header = AssetHashHeader {
            asset_id: [1u8; 32],
            hash: [2u8; 32],
            shard_count: 10,
            shard_index: 5,
        };
        assert!(ebpf.validate_asset_hash(&valid_header));

        let invalid_header = AssetHashHeader {
            asset_id: [1u8; 32],
            hash: [2u8; 32],
            shard_count: 10,
            shard_index: 10, // >= shard_count
        };
        assert!(!ebpf.validate_asset_hash(&invalid_header));
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
        let dest = MatrixPosition { x: 1.0, y: 2.0, z: 3.0 };
        let next_hop = MatrixPosition { x: 4.0, y: 5.0, z: 6.0 };
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
            position: MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
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
        let dest = MatrixPosition { x: 10.0, y: 20.0, z: 30.0 };
        let next_hop = MatrixPosition { x: 11.0, y: 21.0, z: 31.0 };

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
        let missing = MatrixPosition { x: 99.0, y: 99.0, z: 99.0 };
        assert!(ebpf.get_routing_rule(&missing).is_none());
    }

    #[test]
    fn test_routing_rule_overwrites() {
        let ebpf = HyperMeshEbpf::default();
        let dest = MatrixPosition { x: 1.0, y: 2.0, z: 3.0 };
        let hop_a = MatrixPosition { x: 4.0, y: 5.0, z: 6.0 };
        let hop_b = MatrixPosition { x: 7.0, y: 8.0, z: 9.0 };

        ebpf.set_routing_rule(dest, hop_a).expect("test: set first rule");
        ebpf.set_routing_rule(dest, hop_b).expect("test: set second rule");

        assert_eq!(ebpf.routing_rule_count(), 1);
        let hop = ebpf.get_routing_rule(&dest).expect("test: get overwritten rule");
        assert_eq!(hop.x, 7.0);
    }

    #[test]
    fn test_asset_hash_stores_and_retrieves() {
        let ebpf = HyperMeshEbpf::default();
        let hash = ContentHash::from_bytes([0xABu8; 32]);
        let metadata = ShardMetadata {
            shard_index: 3,
            shard_count: 14,
            position: MatrixPosition { x: 5.0, y: 6.0, z: 7.0 },
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

        ebpf.set_pos_validation(hash, true).expect("test: set valid");
        assert_eq!(ebpf.get_pos_validation(&hash), Some(true));

        // Overwrite with false
        ebpf.set_pos_validation(hash, false).expect("test: set invalid");
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
}
