// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! `SystemAssetAdapter` -- hoists REAL host hardware into a HyperMesh asset.
//!
//! Generic over a [`HardwareProbe`] so the same adapter logic runs on any OS;
//! the configured [`AssetType`] (one of `Cpu` / `Memory` / `Storage`) selects
//! which probe drives [`AssetAdapter::get_resource_usage`]. Every read is live
//! hardware -- the adapter holds no cached resource state.
//!
//! Trait methods that genuinely require a live distributed node (allocation,
//! deallocation, proxy NAT, resource limits) return an honest typed error
//! rather than a stub: this module's remit is real-metric probing + PoS
//! plausibility, not ownership of the cluster allocation registry.

use async_trait::async_trait;
use std::time::SystemTime;

use crate::assets::core::adapter::{AdapterCapabilities, AdapterHealth};
use crate::assets::core::status::{AssetHealthStatus, HealthTrend};
use crate::assets::core::{
    AssetAdapter, AssetAllocation, AssetAllocationRequest, AssetError, AssetRegistration,
    AssetResult, AssetState, AssetStatus, AssetType, CpuUsage, MemoryUsage, PrivacyMode,
    ProxyAddress, ResourceLimits, ResourceUsage, StateProof, StorageUsage,
};

use super::probe::HardwareProbe;

/// Generic system-asset adapter backed by a live [`HardwareProbe`].
pub struct SystemAssetAdapter<P: HardwareProbe> {
    asset_type: AssetType,
    probe: P,
}

impl<P: HardwareProbe> SystemAssetAdapter<P> {
    /// Create an adapter for `asset_type`, driven by `probe`.
    ///
    /// `asset_type` must be one of [`AssetType::Cpu`], [`AssetType::Memory`],
    /// or [`AssetType::Storage`]; any other variant is rejected, since this
    /// adapter only hoists real base-system hardware.
    pub fn new(asset_type: AssetType, probe: P) -> AssetResult<Self> {
        match asset_type {
            AssetType::Cpu | AssetType::Memory | AssetType::Storage => {
                Ok(Self { asset_type, probe })
            }
            other => Err(AssetError::AdapterError {
                message: format!(
                    "SystemAssetAdapter handles Cpu/Memory/Storage only, got {other:?}"
                ),
            }),
        }
    }

    /// Convenience constructor for a CPU adapter.
    pub fn cpu(probe: P) -> Self {
        Self {
            asset_type: AssetType::Cpu,
            probe,
        }
    }

    /// Convenience constructor for a memory adapter.
    pub fn memory(probe: P) -> Self {
        Self {
            asset_type: AssetType::Memory,
            probe,
        }
    }

    /// Convenience constructor for a storage adapter.
    pub fn storage(probe: P) -> Self {
        Self {
            asset_type: AssetType::Storage,
            probe,
        }
    }

    /// Borrow the underlying probe (used by the proof binding to re-measure).
    pub fn probe(&self) -> &P {
        &self.probe
    }

    /// Build a live [`ResourceUsage`] for this adapter's configured kind.
    ///
    /// Only the relevant metric slot is populated; the rest stay `None`.
    pub async fn live_resource_usage(&self) -> AssetResult<ResourceUsage> {
        let mut usage = ResourceUsage {
            measurement_timestamp: SystemTime::now(),
            ..Default::default()
        };

        match self.asset_type {
            AssetType::Cpu => {
                let cpu = self.probe.probe_cpu().await?;
                usage.cpu_usage = Some(CpuUsage {
                    utilization_percent: cpu.utilization_percent,
                    frequency_mhz: cpu.frequency_mhz,
                    temperature_celsius: None,
                    active_cores: cpu.logical_cores,
                });
            }
            AssetType::Memory => {
                let mem = self.probe.probe_memory().await?;
                usage.memory_usage = Some(MemoryUsage {
                    used_bytes: mem.used_bytes,
                    total_bytes: mem.total_bytes,
                    cached_bytes: 0,
                    swap_used_bytes: mem.used_swap_bytes,
                });
            }
            AssetType::Storage => {
                let storage = self.probe.probe_storage().await?;
                usage.storage_usage = Some(StorageUsage {
                    used_bytes: storage.used_bytes(),
                    total_bytes: storage.total_bytes,
                    read_iops: 0,
                    write_iops: 0,
                    read_mbps: 0.0,
                    write_mbps: 0.0,
                });
            }
            _ => unreachable!("constructor restricts asset_type to Cpu/Memory/Storage"),
        }

        Ok(usage)
    }

    /// Check that a proof's claimed metrics are plausible versus live hardware.
    ///
    /// "Plausible" means non-degenerate and not exceeding what this machine can
    /// actually back: storage claimed must not exceed live total storage, and
    /// the work proof's compute must be consistent with the live core count.
    /// This is the binding's defence against a proof minted on richer hardware.
    pub async fn proof_metrics_plausible(&self, proof: &StateProof) -> AssetResult<bool> {
        // The space proof's committed storage must fit on real disk.
        let storage = self.probe.probe_storage().await?;
        if proof.space_proof.total_storage == 0 {
            return Ok(false);
        }
        if proof.space_proof.total_storage > storage.total_bytes {
            return Ok(false);
        }

        // The work proof's compute must not exceed live capacity (cores * 1000,
        // the same metric TrustChain derives), with no zero/degenerate claim.
        let cpu = self.probe.probe_cpu().await?;
        let live_compute = u64::from(cpu.logical_cores).saturating_mul(1000);
        if proof.work_proof.computational_power == 0 {
            return Ok(false);
        }
        if proof.work_proof.computational_power > live_compute {
            return Ok(false);
        }

        Ok(true)
    }

    /// Build a live [`AssetStatus`] for `asset_id` with a real health score.
    async fn live_status(&self, asset_id: &AssetRegistration) -> AssetResult<AssetStatus> {
        let usage = self.live_resource_usage().await?;
        let health = self.live_health().await?;

        let now = SystemTime::now();
        let mut status = AssetStatus::new(
            asset_id.clone(),
            String::new(),
            PrivacyMode::PRIVATE,
        );
        status.state = AssetState::Available;
        status.allocated_at = now;
        status.last_accessed = now;
        status.resource_usage = usage;
        status.health_status = health;
        Ok(status)
    }

    /// Compute a real health status from a live probe of this adapter's kind.
    async fn live_health(&self) -> AssetResult<AssetHealthStatus> {
        // A successful probe is the health signal: if the resource reads back
        // with a non-degenerate capacity, the asset is healthy.
        let usage = self.live_resource_usage().await?;
        let healthy = match self.asset_type {
            AssetType::Cpu => usage
                .cpu_usage
                .as_ref()
                .is_some_and(|c| c.active_cores > 0),
            AssetType::Memory => usage
                .memory_usage
                .as_ref()
                .is_some_and(|m| m.total_bytes > 0),
            AssetType::Storage => usage
                .storage_usage
                .as_ref()
                .is_some_and(|s| s.total_bytes > 0),
            _ => false,
        };

        Ok(AssetHealthStatus {
            health_score: if healthy { 1.0 } else { 0.0 },
            last_health_check: SystemTime::now(),
            health_metrics: Default::default(),
            alerts: Vec::new(),
            health_trend: HealthTrend::Stable,
        })
    }

    /// Typed error for operations that require a live distributed node.
    fn requires_live_node(op: &str) -> AssetError {
        AssetError::ResourceUnavailable(format!(
            "{op} requires a live HyperMesh node allocation registry; \
             SystemAssetAdapter only probes local hardware + binds proofs"
        ))
    }
}

#[async_trait]
impl<P: HardwareProbe> AssetAdapter for SystemAssetAdapter<P> {
    fn asset_type(&self) -> AssetType {
        self.asset_type.clone()
    }

    async fn validate_state_proof(&self, proof: &StateProof) -> AssetResult<bool> {
        // Real check: structural validity AND metrics plausible vs THIS host.
        if !proof.validate() {
            return Ok(false);
        }
        self.proof_metrics_plausible(proof).await
    }

    async fn allocate_asset(
        &self,
        _request: &AssetAllocationRequest,
    ) -> AssetResult<AssetAllocation> {
        Err(Self::requires_live_node("allocate_asset"))
    }

    async fn deallocate_asset(&self, _asset_id: &AssetRegistration) -> AssetResult<()> {
        Err(Self::requires_live_node("deallocate_asset"))
    }

    async fn get_asset_status(&self, asset_id: &AssetRegistration) -> AssetResult<AssetStatus> {
        self.live_status(asset_id).await
    }

    async fn configure_privacy_level(
        &self,
        _asset_id: &AssetRegistration,
        _privacy: PrivacyMode,
    ) -> AssetResult<()> {
        Err(Self::requires_live_node("configure_privacy_level"))
    }

    async fn assign_proxy_address(
        &self,
        _asset_id: &AssetRegistration,
    ) -> AssetResult<ProxyAddress> {
        Err(Self::requires_live_node("assign_proxy_address"))
    }

    async fn resolve_proxy_address(
        &self,
        _proxy_addr: &ProxyAddress,
    ) -> AssetResult<AssetRegistration> {
        Err(Self::requires_live_node("resolve_proxy_address"))
    }

    async fn get_resource_usage(
        &self,
        _asset_id: &AssetRegistration,
    ) -> AssetResult<ResourceUsage> {
        self.live_resource_usage().await
    }

    async fn set_resource_limits(
        &self,
        _asset_id: &AssetRegistration,
        _limits: ResourceLimits,
    ) -> AssetResult<()> {
        Err(Self::requires_live_node("set_resource_limits"))
    }

    async fn health_check(&self) -> AssetResult<AdapterHealth> {
        let health = self.live_health().await?;
        let healthy = health.health_score > 0.0;
        Ok(AdapterHealth {
            healthy,
            message: format!(
                "{} probe {}",
                self.asset_type.type_name(),
                if healthy { "healthy" } else { "degraded" }
            ),
            last_check: SystemTime::now(),
            performance_metrics: Default::default(),
        })
    }

    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            asset_type: self.asset_type.clone(),
            supported_privacy_levels: vec![PrivacyMode::PRIVATE, PrivacyMode::PUBLIC],
            supports_proxy_addressing: false,
            supports_resource_monitoring: true,
            supports_dynamic_limits: false,
            max_concurrent_allocations: None,
            features: vec![
                "real-hardware-probe".to_string(),
                "proof-of-state-binding".to_string(),
                "cross-platform".to_string(),
            ],
        }
    }
}
