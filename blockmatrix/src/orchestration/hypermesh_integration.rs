//! HyperMesh Container Integration - Bridge between container runtime and asset system
//!
//! This module integrates the container runtime with HyperMesh's asset management
//! system, enabling containers to be treated as first-class assets with consensus
//! proof validation and resource allocation.

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, Mutex};
use uuid::Uuid;

use crate::container::{
    ContainerRuntime, ContainerHandle, ContainerSpec, CreateOptions,
    ContainerId, ContainerStatus, ContainerState,
    ResourceRequirements, ResourceLimits, ResourceUsage as ContainerResourceUsage,
};
use crate::assets::core::{
    AssetManager, AssetId, AssetType, AssetAllocationRequest, AssetAllocation,
    ConsensusProof, AssetResult, AssetStatus, AssetState,
    SpaceProof, StakeProof, WorkProof, TimeProof, WorkloadType, WorkState,
};
use crate::catalog::vm::{
    ConsensusProofVM, VMConfig, ExecutionContext, ExecutionResult,
    PrivacyLevel, AssetManagementConfig,
};

/// HyperMesh-integrated container orchestrator
pub struct HyperMeshContainerOrchestrator {
    /// Core container runtime
    container_runtime: Arc<ContainerRuntime>,
    /// Asset management system
    asset_manager: Arc<AssetManager>,
    /// Consensus VM for validation
    consensus_vm: Arc<ConsensusProofVM>,
    /// Container-to-asset mapping
    container_assets: Arc<RwLock<HashMap<ContainerId, Vec<AssetId>>>>,
    /// Asset-to-container mapping
    asset_containers: Arc<RwLock<HashMap<AssetId, ContainerId>>>,
    /// Orchestration metrics
    metrics: Arc<Mutex<OrchestrationMetrics>>,
    /// Configuration
    config: HyperMeshIntegrationConfig,
}

/// Configuration for HyperMesh integration
#[derive(Debug, Clone)]
pub struct HyperMeshIntegrationConfig {
    /// Enable automatic asset allocation for containers
    pub auto_asset_allocation: bool,
    /// Enable consensus validation for container operations
    pub enable_consensus_validation: bool,
    /// Default privacy level for container assets
    pub default_privacy_level: PrivacyLevel,
    /// Resource allocation strategy
    pub resource_allocation_strategy: ResourceAllocationStrategy,
    /// Maximum containers per node
    pub max_containers_per_node: u32,
    /// Asset reallocation threshold
    pub reallocation_threshold: f64,
    /// Enable container migration
    pub enable_container_migration: bool,
}

impl Default for HyperMeshIntegrationConfig {
    fn default() -> Self {
        Self {
            auto_asset_allocation: true,
            enable_consensus_validation: true,
            default_privacy_level: PrivacyLevel::Private,
            resource_allocation_strategy: ResourceAllocationStrategy::Balanced,
            max_containers_per_node: 100,
            reallocation_threshold: 0.8,
            enable_container_migration: true,
        }
    }
}

/// Resource allocation strategies
#[derive(Debug, Clone)]
pub enum ResourceAllocationStrategy {
    /// Balance resource usage across all assets
    Balanced,
    /// Pack containers onto fewer nodes
    Packed,
    /// Spread containers across many nodes
    Spread,
    /// Optimize for specific workload types
    WorkloadOptimized(WorkloadType),
    /// Custom allocation algorithm
    Custom(AllocationAlgorithm),
}

/// Custom allocation algorithm parameters
#[derive(Debug, Clone)]
pub struct AllocationAlgorithm {
    pub cpu_weight: f64,
    pub memory_weight: f64,
    pub network_weight: f64,
    pub storage_weight: f64,
    pub latency_weight: f64,
    pub cost_weight: f64,
}

/// Container deployment specification with HyperMesh integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperMeshContainerSpec {
    /// Base container specification
    pub container_spec: ContainerSpec,
    /// Required asset allocations
    pub required_assets: HashMap<AssetType, AssetRequirements>,
    /// Consensus proof for deployment
    pub consensus_proof: ConsensusProof,
    /// Privacy requirements
    pub privacy_requirements: PrivacyRequirements,
    /// Performance requirements
    pub performance_requirements: PerformanceRequirements,
    /// Integration metadata
    pub metadata: ContainerMetadata,
}

/// Asset requirements for container deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRequirements {
    /// Minimum required capacity
    pub min_capacity: u64,
    /// Preferred capacity
    pub preferred_capacity: u64,
    /// Maximum capacity
    pub max_capacity: u64,
    /// Resource priority
    pub priority: AssetPriority,
    /// Duration requirements
    pub duration: Duration,
    /// Quality of Service requirements
    pub qos_requirements: QoSRequirements,
}

/// Asset priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
    Emergency = 5,
}

/// Quality of Service requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QoSRequirements {
    /// Maximum latency allowed
    pub max_latency: Option<Duration>,
    /// Minimum throughput required
    pub min_throughput: Option<u64>,
    /// Reliability requirements (0.0-1.0)
    pub reliability: f64,
    /// Availability requirements (0.0-1.0)
    pub availability: f64,
}

/// Privacy requirements for container deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyRequirements {
    /// Privacy level for container execution
    pub execution_privacy: PrivacyLevel,
    /// Privacy level for data storage
    pub storage_privacy: PrivacyLevel,
    /// Privacy level for network communication
    pub network_privacy: PrivacyLevel,
    /// Data encryption requirements
    pub encryption_requirements: EncryptionRequirements,
    /// Access control requirements
    pub access_control: AccessControlRequirements,
}

/// Encryption requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionRequirements {
    /// Require data-at-rest encryption
    pub data_at_rest: bool,
    /// Require data-in-transit encryption
    pub data_in_transit: bool,
    /// Require data-in-memory encryption
    pub data_in_memory: bool,
    /// Encryption algorithm preferences
    pub preferred_algorithms: Vec<String>,
}

/// Access control requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlRequirements {
    /// Required authentication methods
    pub authentication_methods: Vec<String>,
    /// Authorization policies
    pub authorization_policies: Vec<String>,
    /// Audit requirements
    pub audit_level: AuditLevel,
}

/// Audit levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditLevel {
    None,
    Basic,
    Detailed,
    Complete,
}

/// Performance requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirements {
    /// Startup time requirements
    pub max_startup_time: Option<Duration>,
    /// Memory overhead limits
    pub max_memory_overhead: Option<f64>,
    /// CPU overhead limits
    pub max_cpu_overhead: Option<f64>,
    /// Network overhead limits
    pub max_network_overhead: Option<f64>,
    /// Target resource utilization
    pub target_utilization: f64,
}

/// Container metadata for HyperMesh integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMetadata {
    /// Deployment identifier
    pub deployment_id: String,
    /// Application name
    pub application_name: String,
    /// Version information
    pub version: String,
    /// Owner information
    pub owner: String,
    /// Tags for categorization
    pub tags: HashMap<String, String>,
    /// Deployment timestamp
    pub deployed_at: SystemTime,
}

/// Container deployment result
#[derive(Debug, Clone)]
pub struct ContainerDeploymentResult {
    /// Container handle
    pub container_handle: ContainerHandle,
    /// Allocated assets
    pub allocated_assets: HashMap<AssetType, AssetAllocation>,
    /// Deployment metrics
    pub deployment_metrics: DeploymentMetrics,
    /// Status information
    pub status: DeploymentStatus,
}

/// Deployment metrics
#[derive(Debug, Clone)]
pub struct DeploymentMetrics {
    /// Total deployment time
    pub deployment_time: Duration,
    /// Asset allocation time
    pub allocation_time: Duration,
    /// Container creation time
    pub creation_time: Duration,
    /// Startup time
    pub startup_time: Duration,
    /// Resource efficiency
    pub resource_efficiency: f64,
}

/// Deployment status
#[derive(Debug, Clone)]
pub enum DeploymentStatus {
    Pending,
    Allocating,
    Creating,
    Starting,
    Running,
    Scaling,
    Migrating,
    Stopping,
    Stopped,
    Failed(String),
}

/// Orchestration metrics
#[derive(Debug, Default)]
pub struct OrchestrationMetrics {
    /// Total deployments
    pub total_deployments: u64,
    /// Successful deployments
    pub successful_deployments: u64,
    /// Failed deployments
    pub failed_deployments: u64,
    /// Average deployment time
    pub average_deployment_time: Duration,
    /// Resource utilization efficiency
    pub resource_efficiency: f64,
    /// Asset allocation success rate
    pub allocation_success_rate: f64,
    /// Container migration count
    pub migrations_performed: u64,
    /// Current active containers
    pub active_containers: u32,
}

impl HyperMeshContainerOrchestrator {
    /// Create new HyperMesh container orchestrator
    pub async fn new(
        container_runtime: Arc<ContainerRuntime>,
        asset_manager: Arc<AssetManager>,
        consensus_vm: Arc<ConsensusProofVM>,
        config: HyperMeshIntegrationConfig,
    ) -> Result<Self> {
        Ok(Self {
            container_runtime,
            asset_manager,
            consensus_vm,
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
        
        // Validate consensus proof if enabled
        if self.config.enable_consensus_validation {
            self.validate_deployment_consensus(&spec.consensus_proof).await?;
        }
        
        // Allocate required assets
        let allocation_start = SystemTime::now();
        let allocated_assets = if self.config.auto_asset_allocation {
            self.allocate_container_assets(&spec.required_assets, &spec.consensus_proof).await?
        } else {
            HashMap::new()
        };
        let allocation_time = allocation_start.elapsed().unwrap_or_default();
        
        // Create container with allocated resources
        let creation_start = SystemTime::now();
        let container_spec = self.adapt_container_spec_for_assets(
            &spec.container_spec,
            &allocated_assets,
        ).await?;
        
        let create_options = CreateOptions {
            auto_start: false, // We'll start manually after asset binding
            ..Default::default()
        };
        
        let container_handle = self.container_runtime.create(container_spec, create_options).await?;
        let creation_time = creation_start.elapsed().unwrap_or_default();
        
        // Bind assets to container
        self.bind_assets_to_container(
            container_handle.id,
            &allocated_assets,
        ).await?;
        
        // Start container
        let startup_start = SystemTime::now();
        container_handle.start().await?;
        let startup_time = startup_start.elapsed().unwrap_or_default();
        
        let total_deployment_time = deployment_start.elapsed().unwrap_or_default();
        
        // Calculate resource efficiency
        let resource_efficiency = self.calculate_resource_efficiency(
            &allocated_assets,
            &container_handle,
        ).await?;
        
        // Update metrics
        {
            let mut metrics = self.metrics.lock().await;
            metrics.total_deployments += 1;
            metrics.successful_deployments += 1;
            
            // Update average deployment time
            let total_time = metrics.average_deployment_time.as_micros() as u64 * (metrics.total_deployments - 1)
                + total_deployment_time.as_micros() as u64;
            metrics.average_deployment_time = Duration::from_micros(total_time / metrics.total_deployments);
            
            metrics.active_containers += 1;
            
            // Update resource efficiency (moving average)
            metrics.resource_efficiency = (metrics.resource_efficiency * 0.9) + (resource_efficiency * 0.1);
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
    async fn validate_deployment_consensus(
        &self,
        consensus_proof: &ConsensusProof,
    ) -> Result<()> {
        if !consensus_proof.validate() {
            return Err(anyhow!("Invalid consensus proof for container deployment"));
        }
        
        // Additional validation through consensus VM
        let vm_config = self.consensus_vm.config();
        if vm_config.consensus_requirements.require_proof_of_space {
            if consensus_proof.space_proof.total_size == 0 {
                return Err(anyhow!("Space proof required but not provided"));
            }
        }
        
        if vm_config.consensus_requirements.require_proof_of_stake {
            if consensus_proof.stake_proof.stake_amount == 0 {
                return Err(anyhow!("Stake proof required but not provided"));
            }
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
                required_capacity: requirements.preferred_capacity,
                priority: self.map_asset_priority(&requirements.priority),
                duration: requirements.duration,
                consensus_proof: consensus_proof.clone(),
            };
            
            let allocation = self.asset_manager.allocate_asset(allocation_request).await?;
            allocated_assets.insert(asset_type.clone(), allocation);
        }
        
        Ok(allocated_assets)
    }
    
    /// Map asset priority to internal priority system
    fn map_asset_priority(&self, priority: &AssetPriority) -> crate::assets::core::AssetPriority {
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
        
        // Update resource limits based on allocated assets
        for (asset_type, allocation) in allocated_assets {
            match asset_type {
                AssetType::Cpu => {
                    adapted_spec.resources.cpu_quota = Some(allocation.allocated_capacity);
                },
                AssetType::Memory => {
                    adapted_spec.resources.memory_limit = Some(allocation.allocated_capacity);
                },
                AssetType::Storage => {
                    // Configure storage mounts based on allocated storage assets
                    // This would be more complex in production
                },
                AssetType::Network => {
                    // Configure network bandwidth limits
                    adapted_spec.resources.network_bandwidth = Some(allocation.allocated_capacity);
                },
                _ => {
                    // Handle other asset types as needed
                }
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
        
        let asset_ids: Vec<AssetId> = allocated_assets.values()
            .map(|allocation| allocation.asset_id.clone())
            .collect();
        
        // Update mappings
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
        // Get actual resource usage
        let usage = container_handle.usage().await?;
        
        let mut efficiency_scores = Vec::new();
        
        for (asset_type, allocation) in allocated_assets {
            let actual_usage = match asset_type {
                AssetType::Cpu => usage.cpu_usage_percentage as u64,
                AssetType::Memory => usage.memory_usage_bytes,
                AssetType::Storage => usage.storage_usage_bytes,
                AssetType::Network => usage.network_usage_bytes,
                _ => 0,
            };
            
            if allocation.allocated_capacity > 0 {
                let efficiency = actual_usage as f64 / allocation.allocated_capacity as f64;
                efficiency_scores.push(efficiency.min(1.0)); // Cap at 100%
            }
        }
        
        // Calculate average efficiency
        if efficiency_scores.is_empty() {
            Ok(0.0)
        } else {
            Ok(efficiency_scores.iter().sum::<f64>() / efficiency_scores.len() as f64)
        }
    }
    
    /// Stop and deallocate container
    pub async fn stop_container(&self, container_id: ContainerId) -> Result<()> {
        // Stop the container
        let container_handle = self.container_runtime.get_handle(container_id).await?;
        container_handle.stop(Some(Duration::from_secs(30))).await?;
        
        // Deallocate associated assets
        let asset_ids = {
            let container_assets = self.container_assets.read().await;
            container_assets.get(&container_id).cloned().unwrap_or_default()
        };
        
        for asset_id in &asset_ids {
            self.asset_manager.deallocate_asset(asset_id).await?;
        }
        
        // Clean up mappings
        {
            let mut container_assets = self.container_assets.write().await;
            let mut asset_containers = self.asset_containers.write().await;
            
            container_assets.remove(&container_id);
            for asset_id in &asset_ids {
                asset_containers.remove(asset_id);
            }
        }
        
        // Update metrics
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
        // Validate scaling consensus
        self.validate_deployment_consensus(&consensus_proof).await?;
        
        // Get current asset allocations
        let current_asset_ids = {
            let container_assets = self.container_assets.read().await;
            container_assets.get(&container_id).cloned().unwrap_or_default()
        };
        
        // Deallocate current assets
        for asset_id in &current_asset_ids {
            self.asset_manager.deallocate_asset(asset_id).await?;
        }
        
        // Allocate new assets with updated requirements
        let new_allocated_assets = self.allocate_container_assets(&new_requirements, &consensus_proof).await?;
        
        // Update container specification
        let container_handle = self.container_runtime.get_handle(container_id).await?;
        let updated_spec = self.adapt_container_spec_for_assets(
            &container_handle.spec,
            &new_allocated_assets,
        ).await?;
        
        // Apply new resource limits to container
        // This would require container runtime support for dynamic resource updates
        // For now, we'll just update our tracking
        
        // Update asset bindings
        self.bind_assets_to_container(container_id, &new_allocated_assets).await?;
        
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
            let container_handle = self.container_runtime.get_handle(*container_id).await?;
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
        // Stop all managed containers
        let container_ids: Vec<ContainerId> = {
            let container_assets = self.container_assets.read().await;
            container_assets.keys().cloned().collect()
        };
        
        for container_id in container_ids {
            if let Err(e) = self.stop_container(container_id).await {
                tracing::warn!("Failed to stop container {} during shutdown: {}", container_id, e);
            }
        }
        
        // Shutdown container runtime
        self.container_runtime.shutdown().await?;
        
        Ok(())
    }
}

/// Managed container information
#[derive(Debug, Clone)]
pub struct ManagedContainer {
    pub container_id: ContainerId,
    pub container_status: ContainerStatus,
    pub allocated_assets: Vec<AssetStatus>,
    pub deployment_time: std::time::Instant,
}

impl Clone for OrchestrationMetrics {
    fn clone(&self) -> Self {
        Self {
            total_deployments: self.total_deployments,
            successful_deployments: self.successful_deployments,
            failed_deployments: self.failed_deployments,
            average_deployment_time: self.average_deployment_time,
            resource_efficiency: self.resource_efficiency,
            allocation_success_rate: self.allocation_success_rate,
            migrations_performed: self.migrations_performed,
            active_containers: self.active_containers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::ContainerConfig;
    
    #[tokio::test]
    async fn test_orchestrator_creation() {
        let container_config = ContainerConfig::default();
        let container_runtime = Arc::new(ContainerRuntime::new(container_config).await.unwrap());
        let asset_manager = Arc::new(AssetManager::new());
        let vm_config = VMConfig::default();
        let consensus_vm = Arc::new(ConsensusProofVM::new(vm_config).await.unwrap());
        let integration_config = HyperMeshIntegrationConfig::default();
        
        let orchestrator = HyperMeshContainerOrchestrator::new(
            container_runtime,
            asset_manager,
            consensus_vm,
            integration_config,
        ).await;
        
        assert!(orchestrator.is_ok());
    }
    
    #[test]
    fn test_asset_priority_mapping() {
        let orchestrator = create_test_orchestrator().await.unwrap();
        
        assert!(matches!(
            orchestrator.map_asset_priority(&AssetPriority::High),
            crate::assets::core::AssetPriority::High
        ));
        
        assert!(matches!(
            orchestrator.map_asset_priority(&AssetPriority::Low),
            crate::assets::core::AssetPriority::Low
        ));
    }
    
    async fn create_test_orchestrator() -> Result<HyperMeshContainerOrchestrator> {
        let container_config = ContainerConfig::default();
        let container_runtime = Arc::new(ContainerRuntime::new(container_config).await?);
        let asset_manager = Arc::new(AssetManager::new());
        let vm_config = VMConfig::default();
        let consensus_vm = Arc::new(ConsensusProofVM::new(vm_config).await?);
        let integration_config = HyperMeshIntegrationConfig::default();
        
        HyperMeshContainerOrchestrator::new(
            container_runtime,
            asset_manager,
            consensus_vm,
            integration_config,
        ).await
    }
}