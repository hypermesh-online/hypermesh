// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Container Integration - Bridge between container runtime and asset system
//!
//! This module integrates the container runtime with HyperMesh's asset management
//! system, enabling containers to be treated as first-class assets with consensus
//! proof validation and resource allocation.

pub mod types;

pub use types::*;

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock};

use crate::assets::core::{
    AssetAllocation, AssetAllocationRequest, AssetManager, AssetRegistration, AssetType,
    ConsensusProof, PrivacyMode, ResourceRequirements,
};
use crate::container::runtime::ContainerHandle;
use crate::container::{ContainerId, ContainerRuntime, ContainerSpec, CreateOptions};
/// HyperMesh-integrated container orchestrator
pub struct HyperMeshContainerOrchestrator {
    /// Core container runtime
    container_runtime: Arc<ContainerRuntime>,
    /// Asset management system
    asset_manager: Arc<AssetManager>,
    /// Container-to-asset mapping
    container_assets: Arc<RwLock<HashMap<ContainerId, Vec<AssetRegistration>>>>,
    /// Asset-to-container mapping
    asset_containers: Arc<RwLock<HashMap<AssetRegistration, ContainerId>>>,
    /// Orchestration metrics
    metrics: Arc<Mutex<OrchestrationMetrics>>,
    /// Configuration
    config: HyperMeshIntegrationConfig,
}

impl HyperMeshContainerOrchestrator {
    /// Create new HyperMesh container orchestrator
    pub async fn new(
        container_runtime: Arc<ContainerRuntime>,
        asset_manager: Arc<AssetManager>,
        config: HyperMeshIntegrationConfig,
    ) -> Result<Self> {
        Ok(Self {
            container_runtime,
            asset_manager,
            container_assets: Arc::new(RwLock::new(HashMap::new())),
            asset_containers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(OrchestrationMetrics::default())),
            config,
        })
    }

    /// Deploy container with HyperMesh asset integration
    pub async fn deploy_container(
        &self,
        spec: HyperMeshContainerSpec,
    ) -> Result<ContainerDeploymentResult> {
        let deployment_start = SystemTime::now();

        if self.config.enable_consensus_validation {
            self.validate_deployment_consensus(&spec.consensus_proof)
                .await?;
        }

        let allocation_start = SystemTime::now();
        let allocated_assets = if self.config.auto_asset_allocation {
            self.allocate_container_assets(&spec.required_assets, &spec.consensus_proof)
                .await?
        } else {
            HashMap::new()
        };
        let allocation_time = allocation_start.elapsed().unwrap_or_default();

        let creation_start = SystemTime::now();
        let container_spec = self
            .adapt_container_spec_for_assets(&spec.container_spec, &allocated_assets)
            .await?;

        let create_options = CreateOptions {
            name: container_spec.name.clone(),
            image: container_spec.image.clone(),
            env: container_spec.env.clone(),
            resources: container_spec.resources.clone(),
        };

        let container_handle = self
            .container_runtime
            .create(container_spec, create_options)
            .await?;
        let creation_time = creation_start.elapsed().unwrap_or_default();

        self.bind_assets_to_container(container_handle.id, &allocated_assets)
            .await?;

        let startup_start = SystemTime::now();
        container_handle.start().await?;
        let startup_time = startup_start.elapsed().unwrap_or_default();

        let total_deployment_time = deployment_start.elapsed().unwrap_or_default();

        let resource_efficiency = self
            .calculate_resource_efficiency(&allocated_assets, &container_handle)
            .await?;

        {
            let mut metrics = self.metrics.lock().await;
            metrics.total_deployments += 1;
            metrics.successful_deployments += 1;
            let total_time = metrics.average_deployment_time.as_micros() as u64
                * (metrics.total_deployments - 1)
                + total_deployment_time.as_micros() as u64;
            metrics.average_deployment_time =
                Duration::from_micros(total_time / metrics.total_deployments);
            metrics.active_containers += 1;
            metrics.resource_efficiency =
                (metrics.resource_efficiency * 0.9) + (resource_efficiency * 0.1);
        }

        Ok(ContainerDeploymentResult {
            container_handle,
            allocated_assets,
            deployment_metrics: DeploymentMetrics {
                deployment_time: total_deployment_time,
                allocation_time,
                creation_time,
                startup_time,
                resource_efficiency,
            },
            status: DeploymentStatus::Running,
        })
    }

    /// Validate deployment consensus proof
    async fn validate_deployment_consensus(&self, consensus_proof: &ConsensusProof) -> Result<()> {
        if !consensus_proof.validate() {
            return Err(anyhow!("Invalid consensus proof for container deployment"));
        }

        if consensus_proof.space_proof.total_size == 0 {
            return Err(anyhow!("Space proof required but not provided"));
        }

        if consensus_proof.stake_proof.stake_amount == 0 {
            return Err(anyhow!("Stake proof required but not provided"));
        }

        Ok(())
    }

    /// Allocate assets for container deployment
    async fn allocate_container_assets(
        &self,
        required_assets: &HashMap<AssetType, AssetRequirements>,
        consensus_proof: &ConsensusProof,
    ) -> Result<HashMap<AssetType, AssetAllocation>> {
        let mut allocated_assets = HashMap::new();

        for (asset_type, requirements) in required_assets {
            let allocation_request = AssetAllocationRequest {
                asset_type: asset_type.clone(),
                requested_resources: ResourceRequirements {
                    cpu: None,
                    gpu_usage: None,
                    memory_usage: None,
                    storage_usage: None,
                    network_usage: None,
                    container: None,
                    economic: None,
                },
                privacy_level: PrivacyMode::PRIVATE,
                consensus_proof: consensus_proof.clone(),
                certificate_fingerprint: String::new(),
                duration_limit: Some(requirements.duration),
                tags: HashMap::new(),
            };

            let allocation = self
                .asset_manager
                .allocate_asset(allocation_request)
                .await?;
            allocated_assets.insert(asset_type.clone(), allocation);
        }

        Ok(allocated_assets)
    }

    /// Map asset priority to internal priority system
    fn _map_asset_priority(&self, priority: &AssetPriority) -> crate::assets::core::AssetPriority {
        match priority {
            AssetPriority::Low => crate::assets::core::AssetPriority::Low,
            AssetPriority::Normal => crate::assets::core::AssetPriority::Normal,
            AssetPriority::High => crate::assets::core::AssetPriority::High,
            AssetPriority::Critical => crate::assets::core::AssetPriority::Critical,
            AssetPriority::Emergency => crate::assets::core::AssetPriority::Emergency,
        }
    }

    /// Adapt container specification based on allocated assets
    async fn adapt_container_spec_for_assets(
        &self,
        container_spec: &ContainerSpec,
        allocated_assets: &HashMap<AssetType, AssetAllocation>,
    ) -> Result<ContainerSpec> {
        let mut adapted_spec = container_spec.clone();

        for (asset_type, allocation) in allocated_assets {
            let alloc_config = &allocation.allocation_config.resource_allocation;

            match asset_type {
                AssetType::Cpu => {
                    let cpu_factor = alloc_config.cpu_allocation;
                    adapted_spec.resources.cpu_millicores =
                        (adapted_spec.resources.cpu_millicores as f32 * cpu_factor.max(0.1)) as u64;
                }
                AssetType::Memory => {
                    let memory_factor = alloc_config.memory_allocation;
                    adapted_spec.resources.memory_bytes =
                        (adapted_spec.resources.memory_bytes as f32 * memory_factor.max(0.1))
                            as u64;
                }
                AssetType::Storage => {
                    let storage_factor = alloc_config.storage_allocation;
                    adapted_spec.resources.storage_bytes =
                        (adapted_spec.resources.storage_bytes as f32 * storage_factor.max(0.1))
                            as u64;
                }
                AssetType::Network => {
                    tracing::debug!(
                        "Network allocation: {}%",
                        alloc_config.network_allocation * 100.0,
                    );
                }
                _ => {}
            }
        }

        Ok(adapted_spec)
    }

    /// Bind allocated assets to container
    async fn bind_assets_to_container(
        &self,
        container_id: ContainerId,
        allocated_assets: &HashMap<AssetType, AssetAllocation>,
    ) -> Result<()> {
        let mut container_assets = self.container_assets.write().await;
        let mut asset_containers = self.asset_containers.write().await;

        let asset_ids: Vec<AssetRegistration> = allocated_assets
            .values()
            .map(|allocation| allocation.asset_id.clone())
            .collect();

        container_assets.insert(container_id, asset_ids.clone());

        for asset_id in asset_ids {
            asset_containers.insert(asset_id, container_id);
        }

        Ok(())
    }

    /// Calculate resource efficiency for deployment
    async fn calculate_resource_efficiency(
        &self,
        allocated_assets: &HashMap<AssetType, AssetAllocation>,
        container_handle: &ContainerHandle,
    ) -> Result<f64> {
        let usage = container_handle.usage().await?;
        let mut efficiency_scores = Vec::new();

        for (asset_type, allocation) in allocated_assets {
            let actual_usage = match asset_type {
                AssetType::Cpu => usage.cpu_usage_percent as u64,
                AssetType::Memory => usage.memory_usage,
                AssetType::Storage => usage.io_bytes_written + usage.io_bytes_read,
                AssetType::Network => usage.network_bytes_rx + usage.network_bytes_tx,
                _ => 0,
            };

            let alloc_config = &allocation.allocation_config.resource_allocation;
            let allocation_percentage = match asset_type {
                AssetType::Cpu => alloc_config.cpu_allocation,
                AssetType::Memory => alloc_config.memory_allocation,
                AssetType::Storage => alloc_config.storage_allocation,
                AssetType::Network => alloc_config.network_allocation,
                _ => 0.0,
            };

            if allocation_percentage > 0.0 && actual_usage > 0 {
                let efficiency = (actual_usage as f64 / 100.0) / allocation_percentage as f64;
                efficiency_scores.push(efficiency.min(1.0));
            }
        }

        if efficiency_scores.is_empty() {
            Ok(0.0)
        } else {
            Ok(efficiency_scores.iter().sum::<f64>() / efficiency_scores.len() as f64)
        }
    }

    /// Stop and deallocate container
    pub async fn stop_container(&self, container_id: &ContainerId) -> Result<()> {
        let container_handle = self.container_runtime.get_handle(container_id).await?;
        container_handle.stop(Some(Duration::from_secs(30))).await?;

        let asset_ids = {
            let container_assets = self.container_assets.read().await;
            container_assets
                .get(container_id)
                .cloned()
                .unwrap_or_default()
        };

        for asset_id in &asset_ids {
            self.asset_manager.deallocate_asset(asset_id).await?;
        }

        {
            let mut container_assets = self.container_assets.write().await;
            let mut asset_containers = self.asset_containers.write().await;

            container_assets.remove(container_id);
            for asset_id in &asset_ids {
                asset_containers.remove(asset_id);
            }
        }

        {
            let mut metrics = self.metrics.lock().await;
            metrics.active_containers = metrics.active_containers.saturating_sub(1);
        }

        Ok(())
    }

    /// Scale container resources
    pub async fn scale_container(
        &self,
        container_id: ContainerId,
        new_requirements: HashMap<AssetType, AssetRequirements>,
        consensus_proof: ConsensusProof,
    ) -> Result<()> {
        self.validate_deployment_consensus(&consensus_proof).await?;

        let current_asset_ids = {
            let container_assets = self.container_assets.read().await;
            container_assets
                .get(&container_id)
                .cloned()
                .unwrap_or_default()
        };

        for asset_id in &current_asset_ids {
            self.asset_manager.deallocate_asset(asset_id).await?;
        }

        let new_allocated_assets = self
            .allocate_container_assets(&new_requirements, &consensus_proof)
            .await?;

        let container_handle = self.container_runtime.get_handle(&container_id).await?;
        let _updated_spec = self
            .adapt_container_spec_for_assets(&container_handle.spec, &new_allocated_assets)
            .await?;

        self.bind_assets_to_container(container_id, &new_allocated_assets)
            .await?;

        Ok(())
    }

    /// Get orchestration metrics
    pub async fn get_metrics(&self) -> OrchestrationMetrics {
        let metrics = self.metrics.lock().await;
        metrics.clone()
    }

    /// List all managed containers
    pub async fn list_containers(&self) -> Result<Vec<ManagedContainer>> {
        let container_assets = self.container_assets.read().await;
        let mut managed_containers = Vec::new();

        for (container_id, asset_ids) in container_assets.iter() {
            let container_handle = self.container_runtime.get_handle(container_id).await?;
            let status = container_handle.status().await?;

            let mut asset_info = Vec::new();
            for asset_id in asset_ids {
                let asset_status = self.asset_manager.get_asset_status(asset_id).await?;
                asset_info.push(asset_status);
            }

            managed_containers.push(ManagedContainer {
                container_id: *container_id,
                container_status: status,
                allocated_assets: asset_info,
                deployment_time: container_handle.created_at,
            });
        }

        Ok(managed_containers)
    }

    /// Shutdown orchestrator gracefully
    pub async fn shutdown(&self) -> Result<()> {
        let container_ids: Vec<ContainerId> = {
            let container_assets = self.container_assets.read().await;
            container_assets.keys().cloned().collect()
        };

        for container_id in container_ids {
            if let Err(e) = self.stop_container(&container_id).await {
                tracing::warn!(
                    "Failed to stop container {} during shutdown: {}",
                    container_id,
                    e,
                );
            }
        }

        self.container_runtime.shutdown().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::ContainerConfig;

    fn test_container_config() -> ContainerConfig {
        let tmp = std::env::temp_dir().join("hypermesh_test_orchestrator");
        let mut config = ContainerConfig::default();
        config.runtime.data_dir = tmp.join("data");
        config.storage_usage.root = tmp.join("storage");
        config.storage_usage.images = tmp.join("images");
        config.storage_usage.containers = tmp.join("containers");
        config.storage_usage.tmp_dir = tmp.join("tmp");
        config
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let container_config = test_container_config();
        let container_runtime = Arc::new(
            ContainerRuntime::new(container_config)
                .await
                .expect("test: runtime"),
        );
        let asset_manager = Arc::new(AssetManager::new());
        let integration_config = HyperMeshIntegrationConfig::default();

        let orchestrator = HyperMeshContainerOrchestrator::new(
            container_runtime,
            asset_manager,
            integration_config,
        )
        .await;

        assert!(orchestrator.is_ok());
    }

    #[tokio::test]
    async fn test_asset_priority_mapping() {
        let orchestrator = create_test_orchestrator()
            .await
            .expect("test: orchestrator");

        assert!(matches!(
            orchestrator._map_asset_priority(&AssetPriority::High),
            crate::assets::core::AssetPriority::High
        ));

        assert!(matches!(
            orchestrator._map_asset_priority(&AssetPriority::Low),
            crate::assets::core::AssetPriority::Low
        ));
    }

    async fn create_test_orchestrator() -> Result<HyperMeshContainerOrchestrator> {
        let container_config = test_container_config();
        let container_runtime = Arc::new(ContainerRuntime::new(container_config).await?);
        let asset_manager = Arc::new(AssetManager::new());
        let integration_config = HyperMeshIntegrationConfig::default();

        HyperMeshContainerOrchestrator::new(container_runtime, asset_manager, integration_config)
            .await
    }
}
