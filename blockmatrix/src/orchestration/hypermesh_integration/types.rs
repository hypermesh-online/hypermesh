// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh container integration types - config, specs, metrics, and results

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::assets::core::{
    AssetAllocation, AssetStatus, AssetType, StateProof, PrivacyMode, WorkloadType,
};
use crate::container::runtime::ContainerHandle;
use crate::container::{ContainerId, ContainerStatus};

/// Configuration for HyperMesh integration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HyperMeshIntegrationConfig {
    /// Enable automatic asset allocation for containers
    pub auto_asset_allocation: bool,
    /// Enable state proof validation for container operations
    pub enable_state_validation: bool,
    /// Default privacy level for container assets
    pub default_privacy_level: PrivacyMode,
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
            enable_state_validation: true,
            default_privacy_level: PrivacyMode::PRIVATE,
            resource_allocation_strategy: ResourceAllocationStrategy::Balanced,
            max_containers_per_node: 100,
            reallocation_threshold: 0.8,
            enable_container_migration: true,
        }
    }
}

/// Resource allocation strategies
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub container_spec: crate::container::ContainerSpec,
    /// Required asset allocations
    pub required_assets: HashMap<AssetType, AssetRequirements>,
    /// State proof for deployment
    pub state_proof: StateProof,
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
    pub execution_privacy: PrivacyMode,
    /// Privacy level for data storage
    pub storage_privacy: PrivacyMode,
    /// Privacy level for network communication
    pub network_privacy: PrivacyMode,
    /// Data encryption requirements
    pub encryption_requirements: EncryptionRequirements,
    /// Access control requirements
    pub access_control: AccessControlRequirements,
}

impl Default for PrivacyRequirements {
    fn default() -> Self {
        Self {
            execution_privacy: PrivacyMode::PRIVATE,
            storage_privacy: PrivacyMode::PRIVATE,
            network_privacy: PrivacyMode::PRIVATE,
            encryption_requirements: EncryptionRequirements::default(),
            access_control: AccessControlRequirements::default(),
        }
    }
}

/// Encryption requirements
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccessControlRequirements {
    /// Required authentication methods
    pub authentication_methods: Vec<String>,
    /// Authorization policies
    pub authorization_policies: Vec<String>,
    /// Audit requirements
    pub audit_level: AuditLevel,
}

/// Audit levels
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum AuditLevel {
    #[default]
    None,
    Basic,
    Detailed,
    Complete,
}

/// Performance requirements
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

/// Managed container information
#[derive(Debug, Clone)]
pub struct ManagedContainer {
    pub container_id: ContainerId,
    pub container_status: ContainerStatus,
    pub allocated_assets: Vec<AssetStatus>,
    pub deployment_time: std::time::Instant,
}
