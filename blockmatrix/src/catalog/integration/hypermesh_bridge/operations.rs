// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Operations and implementation for the Catalog-HyperMesh Integration Bridge

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use anyhow::{Result, anyhow};
use tokio::sync::{RwLock, Mutex};
use uuid::Uuid;

use crate::orchestration::hypermesh_integration::{
    HyperMeshContainerOrchestrator, HyperMeshContainerSpec,
    PrivacyRequirements, PerformanceRequirements, ContainerMetadata,
};
use crate::assets::core::ConsensusProof;
use crate::container::{ContainerSpec, ResourceRequirements};

use super::types::*;

/// Catalog-HyperMesh deployment bridge
pub struct CatalogHyperMeshBridge {
    /// Container orchestrator
    container_orchestrator: Arc<HyperMeshContainerOrchestrator>,
    /// Active deployments tracking
    active_deployments: Arc<RwLock<HashMap<String, DeploymentInfo>>>,
    /// Bridge metrics
    metrics: Arc<Mutex<BridgeMetrics>>,
    /// Configuration
    _config: BridgeConfiguration,
}

impl CatalogHyperMeshBridge {
    /// Create new Catalog-HyperMesh bridge
    pub async fn new(
        container_orchestrator: Arc<HyperMeshContainerOrchestrator>,
        config: BridgeConfiguration,
    ) -> Result<Self> {
        Ok(Self {
            container_orchestrator,
            active_deployments: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(BridgeMetrics::default())),
            _config: config,
        })
    }

    /// Deploy catalog asset using specified strategy
    pub async fn deploy_catalog_asset(
        &self,
        deployment_spec: CatalogDeploymentSpec,
        consensus_proof: ConsensusProof,
    ) -> Result<CatalogDeploymentResult> {
        let deployment_id = Uuid::new_v4().to_string();
        let start_time = SystemTime::now();

        let deployment_info = DeploymentInfo {
            deployment_id: deployment_id.clone(),
            asset_type: self.get_asset_type_name(&deployment_spec.asset),
            deployment_strategy: self.get_strategy_name(&deployment_spec.deployment_strategy),
            deployed_at: start_time,
            status: DeploymentStatus::Deploying,
            resource_allocations: HashMap::new(),
            performance_metrics: PerformanceMetrics::default(),
        };

        {
            let mut deployments = self.active_deployments.write().await;
            deployments.insert(deployment_id.clone(), deployment_info);
        }

        let deployment_result = match &deployment_spec.deployment_strategy {
            DeploymentStrategy::VMExecution { .. } => {
                return Err(anyhow!(
                    "VM execution not supported; use remote HyperMesh execution via STOQ"
                ));
            },
            DeploymentStrategy::Container { container_config } => {
                self.deploy_as_container(&deployment_spec.asset, container_config, &consensus_proof).await?
            },
            DeploymentStrategy::Serverless { .. } => {
                return Err(anyhow!(
                    "Serverless execution not supported; use remote HyperMesh execution via STOQ"
                ));
            },
            DeploymentStrategy::Hybrid { .. } => {
                return Err(anyhow!(
                    "Hybrid VM execution not supported; use container deployment or remote HyperMesh execution via STOQ"
                ));
            },
        };

        {
            let mut deployments = self.active_deployments.write().await;
            if let Some(deployment) = deployments.get_mut(&deployment_id) {
                deployment.status = if deployment_result.success {
                    DeploymentStatus::Running
                } else {
                    DeploymentStatus::Failed(deployment_result.error_message.clone().unwrap_or_default())
                };
            }
        }

        {
            let mut metrics = self.metrics.lock().await;
            metrics.total_deployments += 1;

            if let DeploymentStrategy::Container { .. } = deployment_spec.deployment_strategy {
                metrics.container_deployments += 1;
            }

            if deployment_result.success {
                metrics.successful_deployments += 1;
            } else {
                metrics.failed_deployments += 1;
            }

            let deployment_time = start_time.elapsed().unwrap_or_default();
            let total_time = metrics.average_deployment_time.as_micros() as u64 * (metrics.total_deployments - 1)
                + deployment_time.as_micros() as u64;
            metrics.average_deployment_time = Duration::from_micros(total_time / metrics.total_deployments);
        }

        Ok(CatalogDeploymentResult {
            deployment_id,
            success: deployment_result.success,
            output: deployment_result.output,
            error_message: deployment_result.error_message,
            deployment_time: start_time.elapsed().unwrap_or_default(),
            resource_allocations: deployment_result.resource_allocations,
            performance_metrics: PerformanceMetrics::default(),
        })
    }

    /// Deploy asset as container
    async fn deploy_as_container(
        &self,
        asset: &CatalogAssetType,
        container_config: &ContainerDeploymentConfig,
        consensus_proof: &ConsensusProof,
    ) -> Result<InternalDeploymentResult> {
        match asset {
            CatalogAssetType::ContainerImage { image_name, image_tag, .. } => {
                let container_spec = ContainerSpec {
                    name: format!("{}-{}", image_name, image_tag),
                    image: format!("{}:{}", image_name, image_tag),
                    command: Some(container_config.command.clone()),
                    args: Some(container_config.args.clone()),
                    env: container_config.environment_variables.clone(),
                    resources: ResourceRequirements::default(),
                    limits: None,
                    labels: HashMap::new(),
                };

                let hypermesh_spec = HyperMeshContainerSpec {
                    container_spec,
                    required_assets: HashMap::new(),
                    consensus_proof: consensus_proof.clone(),
                    privacy_requirements: PrivacyRequirements::default(),
                    performance_requirements: PerformanceRequirements::default(),
                    metadata: ContainerMetadata {
                        deployment_id: Uuid::new_v4().to_string(),
                        application_name: image_name.clone(),
                        version: image_tag.clone(),
                        owner: "catalog-bridge".to_string(),
                        tags: HashMap::new(),
                        deployed_at: SystemTime::now(),
                    },
                };

                let deployment_result = self.container_orchestrator.deploy_container(hypermesh_spec).await?;

                Ok(InternalDeploymentResult {
                    success: true,
                    output: Some(serde_json::json!({
                        "container_id": deployment_result.container_handle.id.to_string(),
                        "status": "running"
                    })),
                    error_message: None,
                    resource_allocations: deployment_result.allocated_assets.keys()
                        .map(|asset_type| (asset_type.clone(), 1))
                        .collect(),
                })
            },
            _ => Err(anyhow!("Asset type not supported for container deployment")),
        }
    }

    /// Get asset type name for tracking
    fn get_asset_type_name(&self, asset: &CatalogAssetType) -> String {
        match asset {
            CatalogAssetType::PythonApp { .. } => "python_app".to_string(),
            CatalogAssetType::RustBinary { .. } => "rust_binary".to_string(),
            CatalogAssetType::ContainerImage { .. } => "container_image".to_string(),
            CatalogAssetType::WasmModule { .. } => "wasm_module".to_string(),
            CatalogAssetType::DataPipeline { .. } => "data_pipeline".to_string(),
        }
    }

    /// Get deployment strategy name for tracking
    fn get_strategy_name(&self, strategy: &DeploymentStrategy) -> String {
        match strategy {
            DeploymentStrategy::VMExecution { .. } => "vm_execution".to_string(),
            DeploymentStrategy::Container { .. } => "container".to_string(),
            DeploymentStrategy::Serverless { .. } => "serverless".to_string(),
            DeploymentStrategy::Hybrid { .. } => "hybrid".to_string(),
        }
    }

    /// Get bridge metrics
    pub async fn get_metrics(&self) -> BridgeMetrics {
        let metrics = self.metrics.lock().await;
        metrics.clone()
    }

    /// List active deployments
    pub async fn list_deployments(&self) -> Vec<DeploymentInfo> {
        let deployments = self.active_deployments.read().await;
        deployments.values().cloned().collect()
    }

    /// Stop deployment
    pub async fn stop_deployment(&self, deployment_id: &str) -> Result<()> {
        let mut deployments = self.active_deployments.write().await;
        if let Some(deployment) = deployments.get_mut(deployment_id) {
            deployment.status = DeploymentStatus::Stopped;
        }
        Ok(())
    }
}
