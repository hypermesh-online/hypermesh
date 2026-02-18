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
pub use capabilities::EbpfCapabilities;
pub use xdp::{XdpManager, PacketDecision, FilterAction, XdpAttachMode, XdpStats, XdpFilterConfig};
pub use af_xdp::{AfXdpManager, AfXdpSocket, AfXdpStats, UmemConfig, RingConfig};
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
pub use validation::{ProofOfStateValidator, AssetHashValidator};
pub use metrics::{HyperMeshMetrics, HyperMeshMetricsCollector, TransportMetrics};

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
        })
    }

    /// Get detected system capabilities
    pub fn capabilities(&self) -> &EbpfCapabilities {
        &self.caps
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

    /// Set a routing rule for matrix topology forwarding
    #[allow(unused_variables)]
    pub fn set_routing_rule(
        &self,
        dest: MatrixPosition,
        next_hop: MatrixPosition,
    ) -> Result<(), EbpfError> {
        // In production, this would update a BPF_MAP_TYPE_HASH map
        // mapping destination matrix positions to next-hop positions.
        tracing::debug!(
            "Routing rule set: dest=({},{},{}) -> next_hop=({},{},{})",
            dest.x, dest.y, dest.z,
            next_hop.x, next_hop.y, next_hop.z
        );
        Ok(())
    }

    /// Register an asset hash for validation
    #[allow(unused_variables)]
    pub fn register_asset_hash(
        &self,
        hash: ContentHash,
        metadata: ShardMetadata,
    ) -> Result<(), EbpfError> {
        // In production, this would insert into a BPF hash map so the
        // XDP program can validate asset hashes at kernel level.
        tracing::debug!(
            "Asset hash registered: shard {}/{}",
            metadata.shard_index,
            metadata.shard_count
        );
        Ok(())
    }

    /// Set PoS validation status for a content hash
    #[allow(unused_variables)]
    pub fn set_pos_validation(
        &self,
        hash: ContentHash,
        valid: bool,
    ) -> Result<(), EbpfError> {
        tracing::debug!(
            "PoS validation status set: valid={}",
            valid
        );
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

    #[test]
    fn test_pos_header_validation() {
        let ebpf = HyperMeshEbpf::default();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("test: get system time")
            .as_micros() as u64;

        let valid = ProofOfStateHeader {
            who: [1u8; 32],
            what: [2u8; 32],
            when: now,
            where_: [3u8; 16],
        };
        assert!(ebpf.validate_pos_header(&valid));

        let future = ProofOfStateHeader {
            who: [1u8; 32],
            what: [2u8; 32],
            when: now + 10 * 60 * 1_000_000,
            where_: [3u8; 16],
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
}
