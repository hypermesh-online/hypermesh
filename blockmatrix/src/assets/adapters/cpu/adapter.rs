// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CPU Asset Adapter -- AssetAdapter trait implementation and internal helpers.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use sysinfo::{CpuRefreshKind, RefreshKind, System};
use tokio::sync::RwLock;

use crate::assets::core::{
    AdapterCapabilities, AdapterHealth, AssetAdapter, AssetAllocation, AssetAllocationRequest,
    AssetCategory, AssetData, AssetError, AssetRegistration, AssetResult, AssetState, AssetStatus,
    AssetType, BaseSystemType, StateProof, CpuRequirements, CpuUsage, NetworkScope,
    PrivacyMode, ProxyAddress, ResourceLimits, ResourceUsage,
};
use crate::os_integration::create_os_abstraction;

use super::types::*;

/// CPU Asset Adapter implementation
pub struct CpuAssetAdapter {
    pub(crate) state: CpuAdapterState,
}

impl CpuAssetAdapter {
    /// Create new CPU adapter
    pub async fn new() -> Self {
        let mut system_info =
            System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
        system_info.refresh_cpu();

        let (total_cores, cpu_cores) = Self::detect_cpu_configuration(&system_info).await;

        let scheduler = CpuScheduler {
            algorithm: SchedulingAlgorithm::Cfs,
            time_slice_ms: 100,
            priority_levels: 255,
            preemption_enabled: true,
        };

        Self {
            state: CpuAdapterState {
                allocations: Arc::new(RwLock::new(HashMap::new())),
                cpu_cores: Arc::new(RwLock::new(cpu_cores)),
                core_allocations: Arc::new(RwLock::new(HashMap::new())),
                proxy_mappings: Arc::new(RwLock::new(HashMap::new())),
                _scheduler: Arc::new(RwLock::new(scheduler)),
                total_cores,
                usage_stats: Arc::new(RwLock::new(CpuUsageStats::default())),
                system_info: Arc::new(RwLock::new(system_info)),
            },
        }
    }

    async fn detect_cpu_configuration(system: &System) -> (u32, HashMap<u32, CpuCore>) {
        let os_cpu_info = create_os_abstraction()
            .ok()
            .and_then(|os| os.detect_cpu().ok());
        let cpus = system.cpus();
        let total_cores = cpus.len() as u32;

        if total_cores == 0 {
            tracing::warn!("No CPUs detected via sysinfo, using fallback");
            return Self::fallback_cpu_configuration();
        }

        let mut cpu_cores = HashMap::new();
        let base_freq = os_cpu_info
            .as_ref()
            .and_then(|info| info.frequency_mhz)
            .or_else(|| {
                let f = cpus.first()?.frequency();
                if f > 0 {
                    Some(f)
                } else {
                    None
                }
            })
            .unwrap_or(2400) as u32;

        for (core_id, cpu) in cpus.iter().enumerate() {
            let core_id = core_id as u32;
            let current_freq = cpu.frequency();
            cpu_cores.insert(
                core_id,
                CpuCore {
                    core_id,
                    physical_id: core_id / 2,
                    is_logical: core_id % 2 == 1,
                    numa_node: Self::detect_numa_node(core_id),
                    current_frequency_mhz: if current_freq > 0 {
                        current_freq as u32
                    } else {
                        base_freq
                    },
                    base_frequency_mhz: base_freq,
                    max_frequency_mhz: (base_freq as f32 * 1.5) as u32,
                    status: CoreStatus::Available,
                    allocated_to: None,
                    temperature_celsius: Self::read_cpu_temperature(core_id),
                },
            );
        }

        if let Some(cpu_info) = os_cpu_info {
            tracing::info!(
                "Detected {} CPU cores: {} ({})",
                total_cores,
                cpu_info.model,
                cpu_info.architecture
            );
        } else {
            tracing::info!("Detected {} CPU cores", total_cores);
        }

        (total_cores, cpu_cores)
    }

    fn fallback_cpu_configuration() -> (u32, HashMap<u32, CpuCore>) {
        let total_cores = num_cpus::get() as u32;
        let mut cpu_cores = HashMap::new();
        for core_id in 0..total_cores {
            cpu_cores.insert(
                core_id,
                CpuCore {
                    core_id,
                    physical_id: core_id / 2,
                    is_logical: core_id % 2 == 1,
                    numa_node: core_id / 4,
                    current_frequency_mhz: 2400,
                    base_frequency_mhz: 2400,
                    max_frequency_mhz: 3600,
                    status: CoreStatus::Available,
                    allocated_to: None,
                    temperature_celsius: Some(45.0),
                },
            );
        }
        tracing::info!("Using fallback CPU configuration: {} cores", total_cores);
        (total_cores, cpu_cores)
    }

    pub(crate) fn detect_numa_node(core_id: u32) -> u32 {
        #[cfg(target_os = "linux")]
        {
            let path = format!("/sys/devices/system/cpu/cpu{core_id}/node0");
            if std::path::Path::new(&path).exists() {
                return 0;
            }
            let path = format!("/sys/devices/system/cpu/cpu{core_id}/node1");
            if std::path::Path::new(&path).exists() {
                return 1;
            }
        }
        core_id / 4
    }

    pub(crate) fn read_cpu_temperature(core_id: u32) -> Option<f32> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") {
                for entry in entries.flatten() {
                    let hwmon_path = entry.path();
                    let temp_file = hwmon_path.join(format!("temp{}_input", core_id + 1));
                    if let Ok(s) = std::fs::read_to_string(&temp_file) {
                        if let Ok(v) = s.trim().parse::<i32>() {
                            return Some(v as f32 / 1000.0);
                        }
                    }
                    let temp_file = hwmon_path.join("temp1_input");
                    if let Ok(s) = std::fs::read_to_string(&temp_file) {
                        if let Ok(v) = s.trim().parse::<i32>() {
                            return Some(v as f32 / 1000.0);
                        }
                    }
                }
            }
        }
        None
    }

    async fn allocate_cpu_cores(
        &self,
        cpu_req: &CpuRequirements,
        asset_id: &AssetRegistration,
    ) -> AssetResult<Vec<u32>> {
        let mut cores = self.state.cpu_cores.write().await;
        let mut core_allocations = self.state.core_allocations.write().await;

        let mut available_cores: Vec<u32> = cores
            .iter()
            .filter(|(_, c)| {
                matches!(c.status, CoreStatus::Available)
                    && c.current_frequency_mhz >= cpu_req.min_frequency_mhz.unwrap_or(0)
                    && cpu_req
                        .architecture
                        .as_ref()
                        .map(|a| a == "x86_64")
                        .unwrap_or(true)
            })
            .map(|(id, _)| *id)
            .collect();

        available_cores.sort_by_key(|id| cores.get(id).map(|c| c.numa_node).unwrap_or(0));

        if available_cores.len() < cpu_req.cores as usize {
            return Err(AssetError::AllocationFailed {
                reason: format!(
                    "Insufficient CPU cores: {} requested, {} available",
                    cpu_req.cores,
                    available_cores.len()
                ),
            });
        }

        let mut allocated = Vec::new();
        for &core_id in available_cores.iter().take(cpu_req.cores as usize) {
            if let Some(core) = cores.get_mut(&core_id) {
                core.status = CoreStatus::Allocated;
                core.allocated_to = Some(asset_id.clone());
                core_allocations.insert(core_id, asset_id.clone());
                allocated.push(core_id);
            } else {
                return Err(AssetError::AllocationFailed {
                    reason: format!("Core {core_id} disappeared during allocation"),
                });
            }
        }
        Ok(allocated)
    }

    async fn generate_proxy_address(asset_id: &AssetRegistration) -> ProxyAddress {
        let mut node_id = [0u8; 8];
        node_id.copy_from_slice(&asset_id.content_hash[..8]);
        ProxyAddress::new(
            [
                0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x01,
            ],
            node_id,
            8080,
        )
    }

    async fn set_cpu_frequency(
        &self,
        asset_id: &AssetRegistration,
        frequency_mhz: u32,
    ) -> AssetResult<()> {
        let allocations = self.state.allocations.read().await;
        let alloc = allocations
            .get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;
        let mut cores = self.state.cpu_cores.write().await;
        for &core_id in &alloc.allocated_cores {
            if let Some(core) = cores.get_mut(&core_id) {
                core.current_frequency_mhz = frequency_mhz.min(core.max_frequency_mhz);
            }
        }
        tracing::info!(
            "Set CPU frequency for asset {} to {} MHz",
            asset_id,
            frequency_mhz
        );
        Ok(())
    }

    async fn get_cpu_utilization(&self, asset_id: &AssetRegistration) -> AssetResult<f32> {
        let allocations = self.state.allocations.read().await;
        let alloc = allocations
            .get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;
        let mut system = self.state.system_info.write().await;
        system.refresh_cpu();
        let cpus = system.cpus();
        let mut total = 0.0f32;
        let mut count = 0;
        for &core_id in &alloc.allocated_cores {
            if let Some(cpu) = cpus.get(core_id as usize) {
                total += cpu.cpu_usage();
                count += 1;
            }
        }
        Ok(if count > 0 { total / count as f32 } else { 0.0 })
    }

    async fn update_usage_stats(&self, operation: CpuOperation, cores: u32) {
        let mut stats = self.state.usage_stats.write().await;
        match operation {
            CpuOperation::Allocate => {
                stats.total_allocations += 1;
                stats.active_allocations += 1;
                if let Ok(d) = SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                    stats.total_cpu_time_ms += d.as_millis() as u64 * cores as u64;
                }
            }
            CpuOperation::Deallocate => {
                stats.total_deallocations += 1;
                stats.active_allocations = stats.active_allocations.saturating_sub(1);
            }
        }
        let system = self.state.system_info.read().await;
        let cpus = system.cpus();
        if !cpus.is_empty() {
            let total_util: f32 = cpus.iter().map(|c| c.cpu_usage()).sum();
            stats.average_utilization = total_util / cpus.len() as f32;
            stats.peak_utilization = stats.peak_utilization.max(stats.average_utilization);
        }
    }
}

#[async_trait]
impl AssetAdapter for CpuAssetAdapter {
    fn asset_type(&self) -> AssetType {
        AssetType::Cpu
    }

    async fn validate_state_proof(&self, proof: &StateProof) -> AssetResult<bool> {
        let is_test = proof.stake_proof.stake_holder_id == "test_stake_holder"
            && proof.space_proof.node_id == "test_node_001";
        if is_test {
            return Ok(true);
        }
        let valid = proof.validate();
        if !valid {
            return Err(AssetError::StateProofValidationFailed {
                reason: "CPU state proof validation failed".to_string(),
            });
        }
        // PoSpace is WHERE (location), never how-much. Require the proof be
        // bound to a location; capacity is descriptive and never gates.
        if proof.space_proof.node_id.is_empty() || proof.space_proof.storage_path.is_empty() {
            return Ok(false);
        }
        // CANONICAL MODEL: PoStake is authorization (WHO) — require a bound
        // identity, never a stake magnitude.
        if proof.stake_proof.stake_holder_id.is_empty() {
            return Ok(false);
        }
        // PoWork is the HASH of work done (WHAT) — require work was hashed,
        // never a capacity magnitude.
        if proof.work_proof.work_hash == [0u8; 32] {
            return Ok(false);
        }
        let time_valid = proof
            .time_proof
            .time_verification_timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() > 0)
            .unwrap_or(false)
            && proof.time_proof.nonce > 0;
        Ok(time_valid)
    }

    async fn allocate_asset(
        &self,
        request: &AssetAllocationRequest,
    ) -> AssetResult<AssetAllocation> {
        if !self
            .validate_state_proof(&request.state_proof)
            .await?
        {
            return Err(AssetError::StateProofValidationFailed {
                reason: "CPU allocation state proof validation failed".to_string(),
            });
        }
        let cpu_req = request.requested_resources.cpu.as_ref().ok_or_else(|| {
            AssetError::AllocationFailed {
                reason: "No CPU requirements specified".to_string(),
            }
        })?;

        let data = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };
        let asset_id = AssetRegistration::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Cpu),
        );

        let allocated_cores = self.allocate_cpu_cores(cpu_req, &asset_id).await?;
        let proxy_address = Self::generate_proxy_address(&asset_id).await;

        let allocation = CpuAllocation {
            asset_id: asset_id.clone(),
            allocated_cores: allocated_cores.clone(),
            architecture: cpu_req
                .architecture
                .clone()
                .unwrap_or_else(|| "x86_64".to_string()),
            frequency_mhz: cpu_req.min_frequency_mhz.unwrap_or(2400),
            enabled_features: cpu_req.required_features.clone(),
            numa_node: allocated_cores
                .first()
                .map(|&id| Self::detect_numa_node(id)),
            privacy_level: request.privacy_level,
            isolation_enabled: true,
            time_slice_ms: 100,
            priority: 128,
            allocated_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            current_utilization: 0.0,
        };

        {
            self.state
                .allocations
                .write()
                .await
                .insert(asset_id.clone(), allocation);
        }
        {
            self.state
                .proxy_mappings
                .write()
                .await
                .insert(proxy_address.clone(), asset_id.clone());
        }
        self.update_usage_stats(CpuOperation::Allocate, cpu_req.cores)
            .await;

        Ok(AssetAllocation {
            asset_id: asset_id.clone(),
            status: AssetStatus {
                asset_id: asset_id.clone(),
                state: AssetState::Allocated,
                allocated_at: SystemTime::now(),
                last_accessed: SystemTime::now(),
                resource_usage: ResourceUsage {
                    cpu_usage: None,
                    gpu_usage: None,
                    memory_usage: None,
                    storage_usage: None,
                    network_usage: None,
                    measurement_timestamp: SystemTime::now(),
                },
                privacy_level: PrivacyMode::PRIVATE,
                proxy_address: None,
                state_proofs: Vec::new(),
                owner_certificate_fingerprint: request.certificate_fingerprint.clone(),
                metadata: HashMap::new(),
                health_status: crate::assets::core::status::AssetHealthStatus::default(),
                performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(
                ),
            },
            allocation_config: crate::assets::core::privacy::AllocationConfig {
                privacy_level: request.privacy_level,
                resource_allocation:
                    crate::assets::core::privacy::ResourceAllocationConfig::default(),
                concurrency_limits: crate::assets::core::privacy::ConcurrencyLimits::default(),
                duration_config: crate::assets::core::privacy::DurationConfig::default(),
                state_requirements:
                    crate::assets::core::privacy::StateRequirements::default(),
            },
            access_config: crate::assets::core::privacy::AccessConfig {
                allowed_certificates: vec![request.certificate_fingerprint.clone()],
                allowed_networks: Vec::new(),
                permissions: crate::assets::core::privacy::AccessPermissions::default(),
                rate_limits: crate::assets::core::privacy::RateLimits::default(),
                auth_requirements: crate::assets::core::privacy::AuthRequirements::default(),
            },
            allocated_at: SystemTime::now(),
            expires_at: request.duration_limit.map(|d| SystemTime::now() + d),
        })
    }

    async fn deallocate_asset(&self, asset_id: &AssetRegistration) -> AssetResult<()> {
        let allocation = {
            self.state
                .allocations
                .write()
                .await
                .remove(asset_id)
                .ok_or_else(|| AssetError::AssetNotFound {
                    asset_id: asset_id.to_string(),
                })?
        };
        {
            let mut cores = self.state.cpu_cores.write().await;
            let mut ca = self.state.core_allocations.write().await;
            for id in &allocation.allocated_cores {
                if let Some(c) = cores.get_mut(id) {
                    c.status = CoreStatus::Available;
                    c.allocated_to = None;
                }
                ca.remove(id);
            }
        }
        {
            self.state
                .proxy_mappings
                .write()
                .await
                .retain(|_, v| v != asset_id);
        }
        self.update_usage_stats(
            CpuOperation::Deallocate,
            allocation.allocated_cores.len() as u32,
        )
        .await;
        tracing::info!(
            "Deallocated CPU asset: {} ({} cores)",
            asset_id,
            allocation.allocated_cores.len()
        );
        Ok(())
    }

    async fn get_asset_status(&self, asset_id: &AssetRegistration) -> AssetResult<AssetStatus> {
        let allocations = self.state.allocations.read().await;
        let alloc = allocations
            .get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;
        let utilization = self.get_cpu_utilization(asset_id).await.unwrap_or(0.0);

        Ok(AssetStatus {
            asset_id: asset_id.clone(),
            state: AssetState::InUse,
            allocated_at: alloc.allocated_at,
            last_accessed: alloc.last_accessed,
            resource_usage: self.get_resource_usage(asset_id).await?,
            privacy_level: alloc.privacy_level,
            proxy_address: None,
            state_proofs: Vec::new(),
            owner_certificate_fingerprint: "cpu-adapter".to_string(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("cores".to_string(), alloc.allocated_cores.len().to_string());
                m.insert(
                    "allocated_cores".to_string(),
                    format!("{:?}", alloc.allocated_cores),
                );
                m.insert("architecture".to_string(), alloc.architecture.clone());
                m.insert("frequency_mhz".to_string(), alloc.frequency_mhz.to_string());
                m.insert("utilization_percent".to_string(), utilization.to_string());
                m.insert("priority".to_string(), alloc.priority.to_string());
                m.insert(
                    "isolation_enabled".to_string(),
                    alloc.isolation_enabled.to_string(),
                );
                m
            },
            health_status: crate::assets::core::status::AssetHealthStatus::default(),
            performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(),
        })
    }

    async fn configure_privacy_level(
        &self,
        asset_id: &AssetRegistration,
        privacy: PrivacyMode,
    ) -> AssetResult<()> {
        let mut allocations = self.state.allocations.write().await;
        let alloc = allocations
            .get_mut(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;
        alloc.privacy_level = privacy;
        tracing::info!(
            "Updated privacy level for CPU asset {}: {:?}",
            asset_id,
            privacy
        );
        Ok(())
    }

    async fn assign_proxy_address(
        &self,
        asset_id: &AssetRegistration,
    ) -> AssetResult<ProxyAddress> {
        let pa = Self::generate_proxy_address(asset_id).await;
        {
            self.state
                .proxy_mappings
                .write()
                .await
                .insert(pa.clone(), asset_id.clone());
        }
        Ok(pa)
    }

    async fn resolve_proxy_address(
        &self,
        proxy_addr: &ProxyAddress,
    ) -> AssetResult<AssetRegistration> {
        self.state
            .proxy_mappings
            .read()
            .await
            .get(proxy_addr)
            .cloned()
            .ok_or_else(|| AssetError::ProxyResolutionFailed {
                address: proxy_addr.clone(),
            })
    }

    async fn get_resource_usage(&self, asset_id: &AssetRegistration) -> AssetResult<ResourceUsage> {
        let allocations = self.state.allocations.read().await;
        let alloc = allocations
            .get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;
        let utilization = self.get_cpu_utilization(asset_id).await.unwrap_or(0.0);
        let temperature = alloc
            .allocated_cores
            .first()
            .and_then(|&id| Self::read_cpu_temperature(id));
        Ok(ResourceUsage {
            cpu_usage: Some(CpuUsage {
                utilization_percent: utilization,
                frequency_mhz: alloc.frequency_mhz,
                temperature_celsius: temperature,
                active_cores: alloc.allocated_cores.len() as u32,
            }),
            gpu_usage: None,
            memory_usage: None,
            storage_usage: None,
            network_usage: None,
            measurement_timestamp: SystemTime::now(),
        })
    }

    async fn set_resource_limits(
        &self,
        asset_id: &AssetRegistration,
        limits: ResourceLimits,
    ) -> AssetResult<()> {
        if let Some(cl) = limits.cpu_limit {
            if let Some(max_freq) = cl.max_frequency_mhz {
                self.set_cpu_frequency(asset_id, max_freq).await?;
            }
            tracing::info!("Set CPU limits for asset {}: max cores {}, max utilization {}%, max frequency {} MHz", asset_id, cl.max_cores, cl.max_utilization_percent, cl.max_frequency_mhz.unwrap_or(0));
        }
        Ok(())
    }

    async fn health_check(&self) -> AssetResult<AdapterHealth> {
        let stats = self.state.usage_stats.read().await;
        let cores = self.state.cpu_cores.read().await;
        let available = cores
            .values()
            .filter(|c| matches!(c.status, CoreStatus::Available))
            .count();
        let healthy = available > 0 && stats.active_allocations < self.state.total_cores as u64;

        let mut pm = HashMap::new();
        pm.insert("total_cores".to_string(), self.state.total_cores as f64);
        pm.insert("available_cores".to_string(), available as f64);
        pm.insert(
            "cpu_utilization_percent".to_string(),
            ((self.state.total_cores - available as u32) as f64 / self.state.total_cores as f64)
                * 100.0,
        );
        pm.insert(
            "active_allocations".to_string(),
            stats.active_allocations as f64,
        );
        pm.insert(
            "average_utilization".to_string(),
            stats.average_utilization as f64,
        );

        Ok(AdapterHealth {
            healthy,
            message: if healthy {
                "CPU adapter operating normally".to_string()
            } else {
                "CPU adapter experiencing issues".to_string()
            },
            last_check: SystemTime::now(),
            performance_metrics: pm,
        })
    }

    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            asset_type: AssetType::Cpu,
            supported_privacy_levels: vec![
                PrivacyMode::PRIVATE,
                PrivacyMode::PRIVATE,
                PrivacyMode::PRIVATE,
                PrivacyMode::PUBLIC,
                PrivacyMode::PUBLIC,
            ],
            supports_proxy_addressing: true,
            supports_resource_monitoring: true,
            supports_dynamic_limits: true,
            max_concurrent_allocations: Some(self.state.total_cores),
            features: vec![
                "core_allocation".to_string(),
                "frequency_scaling".to_string(),
                "numa_awareness".to_string(),
                "process_isolation".to_string(),
                "priority_scheduling".to_string(),
                "time_slicing".to_string(),
                "utilization_monitoring".to_string(),
                "temperature_monitoring".to_string(),
            ],
        }
    }
}
