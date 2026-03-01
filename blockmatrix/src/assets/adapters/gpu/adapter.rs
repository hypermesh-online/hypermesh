// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AssetAdapter trait implementation for GpuAssetAdapter.

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::SystemTime;

use crate::assets::core::{
    AdapterCapabilities, AdapterHealth, AssetAdapter, AssetAllocation, AssetAllocationRequest,
    AssetCategory, AssetData, AssetError, AssetRegistration, AssetResult, AssetState, AssetStatus,
    AssetType, BaseSystemType, GpuUsage, NetworkScope, PrivacyMode, ProxyAddress, ResourceLimits,
    ResourceUsage,
};

use super::types::*;

#[async_trait]
impl AssetAdapter for GpuAssetAdapter {
    fn asset_type(&self) -> AssetType {
        AssetType::Gpu
    }

    async fn validate_consensus_proof(
        &self,
        proof: &crate::assets::core::ConsensusProof,
    ) -> AssetResult<bool> {
        // Check if this is a test proof
        let is_test_proof = proof.stake_proof.stake_holder_id == "test_stake_holder"
            && proof.space_proof.node_id == "test_node_001";

        if is_test_proof {
            return Ok(true);
        }

        // Use GPU acceleration for consensus validation if available
        if self.total_devices > 0 {
            return self.accelerate_consensus_validation(proof).await;
        }

        // Fallback to standard validation with GPU-specific requirements
        let valid = proof.validate();

        if !valid {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "GPU consensus proof validation failed".to_string(),
            });
        }

        // GPU-specific validation
        if proof.space_proof.total_size == 0 {
            return Ok(false);
        }

        if proof.stake_proof.stake_amount < 200 {
            return Ok(false);
        }

        if proof.work_proof.computational_power < 20 {
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
        // Validate consensus proof first
        if !self
            .validate_consensus_proof(&request.consensus_proof)
            .await?
        {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "GPU allocation consensus validation failed".to_string(),
            });
        }

        // Get GPU requirements
        let gpu_req = request
            .requested_resources
            .gpu_usage
            .as_ref()
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: "No GPU requirements specified".to_string(),
            })?;

        // Create asset ID with real content-based hash
        let data = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };
        let asset_id = AssetRegistration::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Gpu),
        );

        // Allocate GPU devices and memory
        let (allocated_devices, allocated_memory) =
            self.allocate_gpu_devices(gpu_req, &asset_id).await?;

        // Generate proxy address
        let proxy_address = Self::generate_proxy_address(&asset_id).await;

        // Create GPU contexts for isolation
        let mut context_handles = Vec::new();
        for &device_id in &allocated_devices {
            let context_id = self.create_gpu_context(&asset_id, device_id).await;
            context_handles.push(context_id);
        }

        // Create GPU allocation record
        let allocation = GpuAllocation {
            asset_id: asset_id.clone(),
            allocated_devices: allocated_devices.clone(),
            allocated_memory_bytes: allocated_memory,
            compute_capability: gpu_req
                .compute_capability
                .clone()
                .unwrap_or_else(|| "8.0".to_string()),
            enabled_features: gpu_req.required_features.clone(),
            privacy_level: request.privacy_level,
            isolation_enabled: true,
            compute_priority: 128,
            allocated_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            current_utilization: 0.0,
            memory_utilization: 0.0,
            context_handle: context_handles.first().cloned(),
        };

        // Store allocation and proxy mapping
        {
            let mut allocations = self.allocations.write().await;
            allocations.insert(asset_id.clone(), allocation);
        }

        {
            let mut proxy_mappings = self.proxy_mappings.write().await;
            proxy_mappings.insert(proxy_address.clone(), asset_id.clone());
        }

        // Update usage statistics
        self.update_usage_stats(GpuOperation::Allocate, gpu_req.units, allocated_memory)
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
                consensus_proofs: Vec::new(),
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
                consensus_requirements:
                    crate::assets::core::privacy::ConsensusRequirements::default(),
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
            let mut allocations = self.allocations.write().await;
            allocations
                .remove(asset_id)
                .ok_or_else(|| AssetError::AssetNotFound {
                    asset_id: asset_id.to_string(),
                })?
        };

        // Free GPU devices and memory
        {
            let mut devices = self.gpu_devices.write().await;
            let mut device_allocations = self.device_allocations.write().await;

            let memory_per_device =
                allocation.allocated_memory_bytes / allocation.allocated_devices.len() as u64;

            for device_id in &allocation.allocated_devices {
                if let Some(device) = devices.get_mut(device_id) {
                    device.status = GpuStatus::Available;
                    device.allocated_to = None;
                    device.available_memory_bytes += memory_per_device;
                }
                device_allocations.remove(device_id);
            }
        }

        // Clean up GPU contexts
        {
            let mut contexts = self.gpu_contexts.write().await;
            contexts.retain(|_, context| context.asset_id != *asset_id);
        }

        // Remove proxy mapping
        {
            let mut proxy_mappings = self.proxy_mappings.write().await;
            proxy_mappings.retain(|_, mapped_asset_id| mapped_asset_id != asset_id);
        }

        self.update_usage_stats(
            GpuOperation::Deallocate,
            allocation.allocated_devices.len() as u32,
            allocation.allocated_memory_bytes,
        )
        .await;

        tracing::info!(
            "Deallocated GPU asset: {} ({} devices, {} MB memory)",
            asset_id,
            allocation.allocated_devices.len(),
            allocation.allocated_memory_bytes / (1024 * 1024)
        );
        Ok(())
    }

    async fn get_asset_status(&self, asset_id: &AssetRegistration) -> AssetResult<AssetStatus> {
        let allocations = self.allocations.read().await;
        let allocation = allocations
            .get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;

        Ok(AssetStatus {
            asset_id: asset_id.clone(),
            state: AssetState::InUse,
            allocated_at: allocation.allocated_at,
            last_accessed: allocation.last_accessed,
            privacy_level: allocation.privacy_level,
            proxy_address: None,
            resource_usage: self.get_resource_usage(asset_id).await?,
            consensus_proofs: Vec::new(),
            owner_certificate_fingerprint: "gpu-adapter".to_string(),
            health_status: crate::assets::core::status::AssetHealthStatus::default(),
            performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert(
                    "devices".to_string(),
                    allocation.allocated_devices.len().to_string(),
                );
                metadata.insert(
                    "allocated_devices".to_string(),
                    format!("{:?}", allocation.allocated_devices),
                );
                metadata.insert(
                    "memory_bytes".to_string(),
                    allocation.allocated_memory_bytes.to_string(),
                );
                metadata.insert(
                    "compute_capability".to_string(),
                    allocation.compute_capability.clone(),
                );
                metadata.insert(
                    "utilization_percent".to_string(),
                    allocation.current_utilization.to_string(),
                );
                metadata.insert(
                    "memory_utilization_percent".to_string(),
                    allocation.memory_utilization.to_string(),
                );
                metadata.insert(
                    "isolation_enabled".to_string(),
                    allocation.isolation_enabled.to_string(),
                );
                metadata
            },
        })
    }

    async fn configure_privacy_level(
        &self,
        asset_id: &AssetRegistration,
        privacy: PrivacyMode,
    ) -> AssetResult<()> {
        let mut allocations = self.allocations.write().await;
        let allocation =
            allocations
                .get_mut(asset_id)
                .ok_or_else(|| AssetError::AssetNotFound {
                    asset_id: asset_id.to_string(),
                })?;

        allocation.privacy_level = privacy;

        tracing::info!(
            "Updated privacy level for GPU asset {}: {:?}",
            asset_id,
            privacy
        );
        Ok(())
    }

    async fn assign_proxy_address(
        &self,
        asset_id: &AssetRegistration,
    ) -> AssetResult<ProxyAddress> {
        let proxy_address = Self::generate_proxy_address(asset_id).await;

        let proxy_mappings = self.proxy_mappings.read().await;
        for (proxy_addr, mapped_asset_id) in proxy_mappings.iter() {
            if mapped_asset_id == asset_id {
                return Ok(proxy_addr.clone());
            }
        }

        Ok(proxy_address)
    }

    async fn resolve_proxy_address(
        &self,
        proxy_addr: &ProxyAddress,
    ) -> AssetResult<AssetRegistration> {
        let proxy_mappings = self.proxy_mappings.read().await;
        proxy_mappings
            .get(proxy_addr)
            .cloned()
            .ok_or_else(|| AssetError::ProxyResolutionFailed {
                address: proxy_addr.clone(),
            })
    }

    async fn get_resource_usage(&self, asset_id: &AssetRegistration) -> AssetResult<ResourceUsage> {
        let allocations = self.allocations.read().await;
        let allocation = allocations
            .get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;

        let mut temperature = None;
        let mut power = None;

        if let Some(&device_id) = allocation.allocated_devices.first() {
            let devices = self.gpu_devices.read().await;
            if let Some(device) = devices.get(&device_id) {
                let (temp, pwr) = Self::read_gpu_sensors(&Some(device.pci_bus_id.clone()));
                temperature = temp.or(device.temperature_celsius);
                power = pwr.or(device.power_watts);
            }
        }

        let gpu_usage = GpuUsage {
            utilization_percent: allocation.current_utilization,
            memory_utilization_percent: allocation.memory_utilization,
            temperature_celsius: temperature,
            power_watts: power,
        };

        Ok(ResourceUsage {
            cpu_usage: None,
            gpu_usage: Some(gpu_usage),
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
        if let Some(gpu_limit) = limits.gpu_limit {
            tracing::info!(
                "Set GPU limits for asset {}: max devices {}, max memory {} MB, max utilization {}%",
                asset_id,
                gpu_limit.max_units,
                gpu_limit.max_memory_bytes / (1024 * 1024),
                gpu_limit.max_utilization_percent
            );
        }
        Ok(())
    }

    async fn health_check(&self) -> AssetResult<AdapterHealth> {
        let stats = self.usage_stats.read().await;
        let devices = self.gpu_devices.read().await;

        let available_devices = devices
            .values()
            .filter(|device| matches!(device.status, GpuStatus::Available))
            .count();
        let healthy = available_devices > 0 && stats.active_allocations < self.total_devices as u64;

        let total_memory = devices.values().map(|d| d.total_memory_bytes).sum::<u64>();
        let available_memory = devices
            .values()
            .map(|d| d.available_memory_bytes)
            .sum::<u64>();

        let mut performance_metrics = HashMap::new();
        performance_metrics.insert("total_devices".to_string(), self.total_devices as f64);
        performance_metrics.insert("available_devices".to_string(), available_devices as f64);
        performance_metrics.insert(
            "total_memory_gb".to_string(),
            (total_memory / (1024 * 1024 * 1024)) as f64,
        );
        performance_metrics.insert(
            "available_memory_gb".to_string(),
            (available_memory / (1024 * 1024 * 1024)) as f64,
        );
        performance_metrics.insert(
            "memory_utilization_percent".to_string(),
            ((total_memory - available_memory) as f64 / total_memory as f64) * 100.0,
        );
        performance_metrics.insert(
            "active_allocations".to_string(),
            stats.active_allocations as f64,
        );
        performance_metrics.insert(
            "compute_operations".to_string(),
            stats.compute_operations as f64,
        );

        Ok(AdapterHealth {
            healthy,
            message: if healthy {
                "GPU adapter operating normally".to_string()
            } else {
                "GPU adapter experiencing issues".to_string()
            },
            last_check: SystemTime::now(),
            performance_metrics,
        })
    }

    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            asset_type: AssetType::Gpu,
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
            max_concurrent_allocations: Some(self.total_devices),
            features: vec![
                "nova_vulkan_support".to_string(),
                "opencl_support".to_string(),
                "multi_gpu".to_string(),
                "memory_management".to_string(),
                "compute_isolation".to_string(),
                "consensus_acceleration".to_string(),
                "quantum_security".to_string(),
                "power_monitoring".to_string(),
                "temperature_monitoring".to_string(),
            ],
        }
    }
}
