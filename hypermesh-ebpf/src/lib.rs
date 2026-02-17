// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh eBPF Intelligence Layer
//!
//! Provides kernel-level enforcement of HyperMesh intelligence policies including:
//! - Proof of State (PoS) validation at kernel level
//! - Asset Hash verification for data integrity
//! - Matrix Routing validation for topology compliance
//! - Privacy Tier enforcement for access control
//!
//! This crate implements HyperMesh-specific intelligence that runs in eBPF,
//! while STOQ remains a generic transport protocol.

pub mod policy_maps;
pub mod hypermesh_headers;
pub mod packet_filter;
pub mod validation;
pub mod metrics;

/// Re-export aya when kernel-attach is enabled, allowing downstream crates
/// to use the same aya version for BPF operations.
#[cfg(feature = "kernel-attach")]
pub use aya;

pub use policy_maps::{ValidationPolicy, PolicyManager};
pub use hypermesh_headers::{
    ProofOfStateHeader,
    AssetHashHeader,
    MatrixRoutingHeader,
    PrivacyTierHeader,
    EXT_PROOF_OF_STATE,
    EXT_ASSET_HASH,
    EXT_MATRIX_ROUTING,
    EXT_PRIVACY_TIER,
};
pub use packet_filter::{HyperMeshPacketFilter, FilterAction};
pub use validation::{ProofOfStateValidator, AssetHashValidator};
pub use metrics::{HyperMeshMetrics, HyperMeshMetricsCollector};

/// HyperMesh eBPF manager - orchestrates intelligence enforcement
pub struct HyperMeshEbpf {
    policy_manager: PolicyManager,
    packet_filter: Option<HyperMeshPacketFilter>,
    metrics_collector: HyperMeshMetricsCollector,
}

impl HyperMeshEbpf {
    /// Create new HyperMesh eBPF manager
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            policy_manager: PolicyManager::new()?,
            packet_filter: None,
            metrics_collector: HyperMeshMetricsCollector::new()?,
        })
    }

    /// Attach HyperMesh intelligence to network interface
    #[cfg(feature = "kernel-attach")]
    pub fn attach(&mut self, interface: &str) -> anyhow::Result<()> {
        let mut filter = HyperMeshPacketFilter::new(
            interface,
            self.policy_manager.clone(),
        )?;

        filter.attach()?;
        self.packet_filter = Some(filter);

        tracing::info!("HyperMesh eBPF intelligence attached to {}", interface);
        Ok(())
    }

    /// Detach HyperMesh intelligence
    pub fn detach(&mut self) -> anyhow::Result<()> {
        if let Some(filter) = self.packet_filter.take() {
            filter.detach()?;
            tracing::info!("HyperMesh eBPF intelligence detached");
        }
        Ok(())
    }

    /// Get policy manager for configuration
    pub fn policy_manager(&self) -> &PolicyManager {
        &self.policy_manager
    }

    /// Get mutable policy manager
    pub fn policy_manager_mut(&mut self) -> &mut PolicyManager {
        &mut self.policy_manager
    }

    /// Get current HyperMesh metrics
    pub fn get_metrics(&self) -> HyperMeshMetrics {
        self.metrics_collector.collect()
    }
}

impl Default for HyperMeshEbpf {
    fn default() -> Self {
        Self::new().expect("Failed to create HyperMeshEbpf")
    }
}

impl Drop for HyperMeshEbpf {
    fn drop(&mut self) {
        let _ = self.detach();
    }
}
