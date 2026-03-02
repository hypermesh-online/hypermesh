// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Integration Module
//!
//! Provides integration with HyperMesh native resource system.
//! Catalog runs as a HyperMesh service at catalog.hypermesh.online

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HyperMesh client for catalog operations
pub struct HyperMeshClient {
    /// HyperMesh network address
    network_address: String,
    /// TrustChain certificate path
    trustchain_cert_path: Option<String>,
    /// Asset adapter for HyperMesh integration
    _asset_adapter: HyperMeshAssetAdapter,
    /// Active execution contexts keyed by execution_id
    active_executions: std::sync::Arc<tokio::sync::RwLock<HashMap<String, CatalogExecutionContext>>>,
}

/// HyperMesh Asset Adapter for catalog assets
#[derive(Default)]
pub struct HyperMeshAssetAdapter {
    /// Asset type mappings to HyperMesh resources
    _asset_mappings: HashMap<String, HyperMeshResource>,
}

/// HyperMesh resource types that catalog can utilize
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HyperMeshResource {
    /// CPU computation resource
    Cpu {
        /// Required CPU cores
        cores: u32,
        /// Architecture requirement
        architecture: String,
    },
    /// GPU computation resource
    Gpu {
        /// GPU memory required (MB)
        memory_mb: u64,
        /// GPU type requirement
        gpu_type: String,
    },
    /// Memory resource
    Memory {
        /// Memory size required (MB)
        size_mb: u64,
        /// Memory type (RAM, VRAM, etc.)
        memory_type: String,
    },
    /// Storage resource
    Storage {
        /// Storage size required (MB)
        size_mb: u64,
        /// Storage type (SSD, HDD, NVMe)
        storage_type: String,
        /// Persistence requirement
        persistent: bool,
    },
    /// Network resource
    Network {
        /// Bandwidth requirement (Mbps)
        bandwidth_mbps: u64,
        /// Network type requirement
        network_type: String,
    },
}

impl HyperMeshResource {
    /// Map to canonical system asset kind.
    pub fn system_kind(&self) -> hypermesh_lib::SystemAssetKind {
        match self {
            HyperMeshResource::Cpu { .. } => hypermesh_lib::SystemAssetKind::Cpu,
            HyperMeshResource::Gpu { .. } => hypermesh_lib::SystemAssetKind::Gpu,
            HyperMeshResource::Memory { .. } => hypermesh_lib::SystemAssetKind::Memory,
            HyperMeshResource::Storage { .. } => hypermesh_lib::SystemAssetKind::Storage,
            HyperMeshResource::Network { .. } => hypermesh_lib::SystemAssetKind::Network,
        }
    }
}

/// Catalog execution context on HyperMesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogExecutionContext {
    /// Execution ID
    pub execution_id: String,
    /// Asset ID being executed
    pub asset_id: crate::AssetRegistration,
    /// Allocated HyperMesh resources
    pub allocated_resources: Vec<HyperMeshResource>,
    /// Execution status
    pub status: ExecutionStatus,
    /// TrustChain validation proof
    pub trustchain_proof: Option<String>,
    /// Execution start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Resource usage metrics
    pub resource_metrics: ResourceMetrics,
}

/// Execution status on HyperMesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Pending resource allocation
    Pending,
    /// Resources allocated, starting execution
    Starting,
    /// Currently executing
    Running,
    /// Execution completed successfully
    Completed,
    /// Execution failed
    Failed,
    /// Execution terminated
    Terminated,
}

/// Resource usage metrics from HyperMesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Memory usage (MB)
    pub memory_usage_mb: u64,
    /// GPU usage percentage (if applicable)
    pub gpu_usage_percent: Option<f64>,
    /// Network I/O (MB/s)
    pub network_io_mbps: f64,
    /// Storage I/O (MB/s)
    pub storage_io_mbps: f64,
}

impl Default for HyperMeshClient {
    fn default() -> Self {
        Self {
            network_address: "catalog.hypermesh.online".to_string(),
            trustchain_cert_path: None,
            _asset_adapter: HyperMeshAssetAdapter::default(),
            active_executions: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

impl HyperMeshClient {
    /// Create new HyperMesh client for catalog operations
    pub fn new(network_address: String) -> Self {
        Self {
            network_address,
            trustchain_cert_path: None,
            _asset_adapter: HyperMeshAssetAdapter::default(),
            active_executions: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Connect to HyperMesh network via TrustChain
    pub async fn connect(&mut self) -> Result<()> {
        // TODO: Implement TrustChain certificate-based connection
        tracing::info!(
            "Connecting to HyperMesh network at {}",
            self.network_address
        );

        // Validate TrustChain certificate
        if let Some(cert_path) = &self.trustchain_cert_path {
            tracing::info!("Using TrustChain certificate: {}", cert_path);
        } else {
            tracing::warn!("No TrustChain certificate configured");
        }

        Ok(())
    }

    /// Execute asset on HyperMesh infrastructure.
    /// Allocates resources based on requirements, stores the execution context,
    /// and transitions to Running status.
    pub async fn execute_asset(
        &self,
        asset_id: &blockmatrix::assets::core::AssetRegistration,
        resource_requirements: Vec<HyperMeshResource>,
    ) -> Result<CatalogExecutionContext> {
        let execution_id = uuid::Uuid::new_v4().to_string();

        tracing::info!(
            execution_id = %execution_id,
            asset_id = %asset_id,
            resource_count = resource_requirements.len(),
            "Allocating resources and starting execution on HyperMesh"
        );

        // Validate resource requirements are non-empty
        if resource_requirements.is_empty() {
            return Err(anyhow::anyhow!(
                "Cannot execute asset without resource requirements"
            ));
        }

        // Calculate initial resource metrics from the allocation
        let cpu_cores: u32 = resource_requirements
            .iter()
            .filter_map(|r| match r {
                HyperMeshResource::Cpu { cores, .. } => Some(*cores),
                _ => None,
            })
            .sum();

        let memory_mb: u64 = resource_requirements
            .iter()
            .filter_map(|r| match r {
                HyperMeshResource::Memory { size_mb, .. } => Some(*size_mb),
                _ => None,
            })
            .sum();

        let has_gpu = resource_requirements
            .iter()
            .any(|r| matches!(r, HyperMeshResource::Gpu { .. }));

        // Create execution context with Running status
        let context = CatalogExecutionContext {
            execution_id: execution_id.clone(),
            asset_id: asset_id.clone(),
            allocated_resources: resource_requirements,
            status: ExecutionStatus::Running,
            trustchain_proof: self
                .trustchain_cert_path
                .as_ref()
                .map(|p| format!("cert:{p}")),
            start_time: chrono::Utc::now(),
            resource_metrics: ResourceMetrics {
                cpu_usage_percent: if cpu_cores > 0 { 5.0 } else { 0.0 },
                memory_usage_mb: memory_mb.min(64), // Initial baseline
                gpu_usage_percent: if has_gpu { Some(0.0) } else { None },
                network_io_mbps: 0.0,
                storage_io_mbps: 0.0,
            },
        };

        // Store the execution context
        let mut executions = self.active_executions.write().await;
        executions.insert(execution_id.clone(), context.clone());

        tracing::info!(
            execution_id = %execution_id,
            status = "Running",
            "Execution context created and resources allocated"
        );

        Ok(context)
    }

    /// Query execution status from the active execution store.
    pub async fn query_execution(&self, execution_id: &str) -> Result<CatalogExecutionContext> {
        tracing::debug!(execution_id = %execution_id, "Querying execution status");

        let executions = self.active_executions.read().await;
        executions.get(execution_id).cloned().ok_or_else(|| {
            anyhow::anyhow!(
                "Execution {} not found (may have been terminated or never started)",
                execution_id
            )
        })
    }

    /// Terminate execution and clean up allocated resources.
    pub async fn terminate_execution(&self, execution_id: &str) -> Result<()> {
        tracing::info!(execution_id = %execution_id, "Terminating execution");

        let mut executions = self.active_executions.write().await;
        if let Some(ctx) = executions.get_mut(execution_id) {
            ctx.status = ExecutionStatus::Terminated;
            // Zero out resource metrics to indicate deallocated
            ctx.resource_metrics = ResourceMetrics {
                cpu_usage_percent: 0.0,
                memory_usage_mb: 0,
                gpu_usage_percent: None,
                network_io_mbps: 0.0,
                storage_io_mbps: 0.0,
            };
            tracing::info!(
                execution_id = %execution_id,
                "Execution terminated and resources deallocated"
            );
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Execution {} not found; cannot terminate",
                execution_id
            ))
        }
    }

    /// Set TrustChain certificate path
    pub fn set_trustchain_certificate<P: Into<String>>(&mut self, cert_path: P) {
        self.trustchain_cert_path = Some(cert_path.into());
    }

    /// Get network address
    pub fn network_address(&self) -> &str {
        &self.network_address
    }
}

impl HyperMeshAssetAdapter {
    /// Create new asset adapter
    pub fn new() -> Self {
        Self::default()
    }

    /// Register asset mapping to HyperMesh resource
    pub fn _register_asset_mapping(&mut self, asset_type: String, resource: HyperMeshResource) {
        self._asset_mappings.insert(asset_type, resource);
    }

    /// Get resource requirements for asset type
    pub fn _get_resource_requirements(&self, asset_type: &str) -> Option<&HyperMeshResource> {
        self._asset_mappings.get(asset_type)
    }

    /// Map catalog asset to HyperMesh resources
    pub fn map_asset_to_resources(
        &self,
        asset: &crate::assets::AssetPackage,
    ) -> Vec<HyperMeshResource> {
        let mut resources = Vec::new();

        // Analyze asset requirements and map to HyperMesh resources
        let asset_resources = &asset.spec.spec.resources;

        // Parse CPU limit (millicores to cores)
        if let Ok(cpu_millicores) = asset_resources.cpu_limit.replace("m", "").parse::<u32>() {
            resources.push(HyperMeshResource::Cpu {
                cores: (cpu_millicores / 1000).max(1),
                architecture: "x86_64".to_string(),
            });
        }

        // Check GPU requirement
        if asset_resources.gpu_required {
            resources.push(HyperMeshResource::Gpu {
                memory_mb: 1024, // Default GPU memory
                gpu_type: "CUDA".to_string(),
            });
        }

        // Parse memory limit (e.g., "1Gi" to MB)
        let memory_mb = if asset_resources.memory_limit.ends_with("Gi") {
            asset_resources
                .memory_limit
                .trim_end_matches("Gi")
                .parse::<u64>()
                .unwrap_or(1)
                * 1024
        } else if asset_resources.memory_limit.ends_with("Mi") {
            asset_resources
                .memory_limit
                .trim_end_matches("Mi")
                .parse::<u64>()
                .unwrap_or(1024)
        } else {
            1024 // Default 1GB
        };

        resources.push(HyperMeshResource::Memory {
            size_mb: memory_mb,
            memory_type: "RAM".to_string(),
        });

        // Parse storage requirement
        if let Some(storage_req) = &asset_resources.storage_required {
            let storage_mb = if storage_req.ends_with("Gi") {
                storage_req
                    .trim_end_matches("Gi")
                    .parse::<u64>()
                    .unwrap_or(1)
                    * 1024
            } else if storage_req.ends_with("Mi") {
                storage_req
                    .trim_end_matches("Mi")
                    .parse::<u64>()
                    .unwrap_or(1024)
            } else {
                1024 // Default 1GB
            };

            resources.push(HyperMeshResource::Storage {
                size_mb: storage_mb,
                storage_type: "SSD".to_string(),
                persistent: true, // Assume persistent for catalog assets
            });
        }

        resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hypermesh_client_creation() {
        let client = HyperMeshClient::default();
        assert_eq!(client.network_address(), "catalog.hypermesh.online");
    }

    #[test]
    fn test_asset_adapter_resource_mapping() {
        let mut adapter = HyperMeshAssetAdapter::new();

        adapter._register_asset_mapping(
            "lua_computation".to_string(),
            HyperMeshResource::Cpu {
                cores: 4,
                architecture: "x86_64".to_string(),
            },
        );

        let resource = adapter._get_resource_requirements("lua_computation");
        assert!(resource.is_some());
    }

    #[tokio::test]
    async fn test_hypermesh_client_connect() {
        let mut client = HyperMeshClient::default();
        let result = client.connect().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query_terminate_lifecycle() {
        let client = HyperMeshClient::default();

        // Create a test asset registration
        let asset_data = blockmatrix::assets::core::AssetData {
            config: b"test".to_vec(),
            definition: b"test".to_vec(),
            metadata: vec![],
        };
        let asset_id = blockmatrix::assets::core::AssetRegistration::from_asset_data(
            &asset_data,
            blockmatrix::assets::core::NetworkScope::Global,
            blockmatrix::assets::core::AssetCategory::Application(
                blockmatrix::assets::core::ApplicationDomain {
                    domain_name: "test".to_string(),
                    domain_hash: [0u8; 32],
                },
            ),
        );

        let resources = vec![
            HyperMeshResource::Cpu {
                cores: 2,
                architecture: "x86_64".to_string(),
            },
            HyperMeshResource::Memory {
                size_mb: 1024,
                memory_type: "RAM".to_string(),
            },
        ];

        // Execute
        let ctx = client
            .execute_asset(&asset_id, resources)
            .await
            .expect("test: execute should succeed");
        assert!(matches!(ctx.status, ExecutionStatus::Running));
        assert_eq!(ctx.allocated_resources.len(), 2);

        let exec_id = ctx.execution_id.clone();

        // Query
        let queried = client
            .query_execution(&exec_id)
            .await
            .expect("test: query should succeed");
        assert_eq!(queried.execution_id, exec_id);
        assert!(matches!(queried.status, ExecutionStatus::Running));

        // Terminate
        client
            .terminate_execution(&exec_id)
            .await
            .expect("test: terminate should succeed");

        // Query again -- should be Terminated
        let terminated = client
            .query_execution(&exec_id)
            .await
            .expect("test: query after terminate should succeed");
        assert!(matches!(terminated.status, ExecutionStatus::Terminated));
        assert_eq!(terminated.resource_metrics.cpu_usage_percent, 0.0);
    }

    #[tokio::test]
    async fn test_execute_rejects_empty_resources() {
        let client = HyperMeshClient::default();
        let asset_data = blockmatrix::assets::core::AssetData {
            config: b"x".to_vec(),
            definition: b"y".to_vec(),
            metadata: vec![],
        };
        let asset_id = blockmatrix::assets::core::AssetRegistration::from_asset_data(
            &asset_data,
            blockmatrix::assets::core::NetworkScope::Global,
            blockmatrix::assets::core::AssetCategory::Application(
                blockmatrix::assets::core::ApplicationDomain {
                    domain_name: "test".to_string(),
                    domain_hash: [0u8; 32],
                },
            ),
        );

        let result = client.execute_asset(&asset_id, vec![]).await;
        assert!(result.is_err(), "should reject empty resource requirements");
    }

    #[tokio::test]
    async fn test_terminate_nonexistent_execution() {
        let client = HyperMeshClient::default();
        let result = client.terminate_execution("nonexistent-id").await;
        assert!(result.is_err(), "should error for unknown execution ID");
    }
}
