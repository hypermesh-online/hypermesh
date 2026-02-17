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

use crate::catalog::vm::ConsensusProofVM;
use crate::orchestration::hypermesh_integration::{
    HyperMeshContainerOrchestrator, HyperMeshContainerSpec,
    PrivacyRequirements, PerformanceRequirements, ContainerMetadata,
};
use crate::assets::core::ConsensusProof;
use crate::container::{ContainerSpec, ResourceRequirements};

use super::types::*;

/// Catalog-HyperMesh deployment bridge
#[allow(dead_code)] // Fields used during deployment bridging
pub struct CatalogHyperMeshBridge {
    /// VM runtime for code execution
    vm_runtime: Arc<ConsensusProofVM>,
    /// Container orchestrator
    container_orchestrator: Arc<HyperMeshContainerOrchestrator>,
    /// Active deployments tracking
    active_deployments: Arc<RwLock<HashMap<String, DeploymentInfo>>>,
    /// Bridge metrics
    metrics: Arc<Mutex<BridgeMetrics>>,
    /// Configuration
    config: BridgeConfiguration,
}

impl CatalogHyperMeshBridge {
    /// Create new Catalog-HyperMesh bridge
    pub async fn new(
        vm_runtime: Arc<ConsensusProofVM>,
        container_orchestrator: Arc<HyperMeshContainerOrchestrator>,
        config: BridgeConfiguration,
    ) -> Result<Self> {
        Ok(Self {
            vm_runtime,
            container_orchestrator,
            active_deployments: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(BridgeMetrics::default())),
            config,
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
            DeploymentStrategy::VMExecution { vm_config } => {
                self.deploy_as_vm(&deployment_spec.asset, vm_config, &consensus_proof).await?
            },
            DeploymentStrategy::Container { container_config } => {
                self.deploy_as_container(&deployment_spec.asset, container_config, &consensus_proof).await?
            },
            DeploymentStrategy::Serverless { function_config } => {
                self.deploy_as_function(&deployment_spec.asset, function_config, &consensus_proof).await?
            },
            DeploymentStrategy::Hybrid { vm_config, container_config } => {
                self.deploy_as_hybrid(&deployment_spec.asset, vm_config, container_config, &consensus_proof).await?
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

            match deployment_spec.deployment_strategy {
                DeploymentStrategy::VMExecution { .. } => metrics.vm_deployments += 1,
                DeploymentStrategy::Container { .. } => metrics.container_deployments += 1,
                DeploymentStrategy::Hybrid { .. } => metrics.hybrid_deployments += 1,
                _ => {},
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

    /// Deploy asset as VM execution
    async fn deploy_as_vm(
        &self,
        asset: &CatalogAssetType,
        _vm_config: &VMDeploymentConfig,
        consensus_proof: &ConsensusProof,
    ) -> Result<InternalDeploymentResult> {
        match asset {
            CatalogAssetType::PythonApp { code, .. } => {
                let result = self.vm_runtime.execute_with_consensus(
                    code, "python", consensus_proof.clone(),
                ).await?;

                Ok(InternalDeploymentResult {
                    success: result.success,
                    output: result.output,
                    error_message: result.error_message,
                    resource_allocations: HashMap::new(),
                })
            },
            CatalogAssetType::RustBinary { source_code, .. } => {
                let result = self.vm_runtime.execute_with_consensus(
                    source_code, "rust", consensus_proof.clone(),
                ).await?;

                Ok(InternalDeploymentResult {
                    success: result.success,
                    output: result.output,
                    error_message: result.error_message,
                    resource_allocations: HashMap::new(),
                })
            },
            _ => Err(anyhow!("Asset type not supported for VM deployment")),
        }
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

    /// Deploy asset as serverless function
    async fn deploy_as_function(
        &self,
        asset: &CatalogAssetType,
        function_config: &FunctionDeploymentConfig,
        consensus_proof: &ConsensusProof,
    ) -> Result<InternalDeploymentResult> {
        let vm_config = VMDeploymentConfig {
            language_runtime: function_config.runtime.clone(),
            execution_timeout: function_config.timeout,
            memory_limit: function_config.memory_size,
            cpu_limit: 1,
            enable_gpu: false,
            environment_variables: HashMap::new(),
        };

        self.deploy_as_vm(asset, &vm_config, consensus_proof).await
    }

    /// Deploy asset as hybrid (VM + Container)
    async fn deploy_as_hybrid(
        &self,
        asset: &CatalogAssetType,
        vm_config: &VMDeploymentConfig,
        container_config: &ContainerDeploymentConfig,
        consensus_proof: &ConsensusProof,
    ) -> Result<InternalDeploymentResult> {
        let vm_result = self.deploy_as_vm(asset, vm_config, consensus_proof).await?;
        let container_result = self.deploy_as_container(asset, container_config, consensus_proof).await?;

        let success = vm_result.success && container_result.success;
        let combined_output = serde_json::json!({
            "vm_result": vm_result.output,
            "container_result": container_result.output
        });

        Ok(InternalDeploymentResult {
            success,
            output: Some(combined_output),
            error_message: if success { None } else {
                Some(format!("VM: {:?}, Container: {:?}", vm_result.error_message, container_result.error_message))
            },
            resource_allocations: {
                let mut allocations = vm_result.resource_allocations;
                allocations.extend(container_result.resource_allocations);
                allocations
            },
        })
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
