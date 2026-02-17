// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Type definitions for MFN-Enhanced Container Orchestration

use crate::{ContainerId, ServiceId, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use super::resource_manager::NodeResources;
use super::scheduler::NodeCandidate;

/// Container specification for orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    /// Container identifier
    pub id: ContainerId,
    /// Service this container belongs to
    pub service_id: ServiceId,
    /// Container image
    pub image: String,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Volume mounts
    pub volumes: Vec<VolumeMount>,
    /// Placement constraints
    pub constraints: Vec<PlacementConstraint>,
    /// Scaling policy
    pub scaling_policy: Option<ScalingPolicy>,
    /// Health check configuration
    pub health_check: Option<HealthCheckConfig>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Resource requirements for container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU cores (fractional allowed, e.g., 0.5)
    pub cpu_cores: f64,
    /// Memory in bytes
    pub memory_bytes: u64,
    /// Storage in bytes
    pub storage_bytes: u64,
    /// GPU units (optional)
    pub gpu_units: Option<u32>,
    /// Network bandwidth (bytes/sec)
    pub network_bandwidth: Option<u64>,
    /// Custom resources
    pub custom_resources: HashMap<String, String>,
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port
    pub container_port: u16,
    /// Host port (optional, auto-assigned if None)
    pub host_port: Option<u16>,
    /// Protocol (TCP/UDP)
    pub protocol: NetworkProtocol,
}

/// Network protocol types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkProtocol {
    Tcp,
    Udp,
    Sctp,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Volume name or path
    pub source: String,
    /// Target path in container
    pub target: String,
    /// Mount type
    pub mount_type: MountType,
    /// Read-only flag
    pub read_only: bool,
}

/// Volume mount types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MountType {
    Bind,
    Volume,
    Tmpfs,
}

/// Placement constraints for container scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlacementConstraint {
    /// Require specific node
    RequireNode(NodeId),
    /// Prefer specific node
    PreferNode(NodeId),
    /// Avoid specific node
    AvoidNode(NodeId),
    /// Require node label
    RequireLabel(String, String),
    /// Prefer node label
    PreferLabel(String, String),
    /// Anti-affinity with service
    AntiAffinity(ServiceId),
    /// Affinity with service
    Affinity(ServiceId),
    /// Require zone
    RequireZone(String),
    /// Prefer zone
    PreferZone(String),
}

/// Container scaling policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPolicy {
    /// Minimum replicas
    pub min_replicas: u32,
    /// Maximum replicas
    pub max_replicas: u32,
    /// Target CPU utilization (0.0 - 1.0)
    pub target_cpu_utilization: f64,
    /// Target memory utilization (0.0 - 1.0)
    pub target_memory_utilization: f64,
    /// Scale up cooldown
    pub scale_up_cooldown: Duration,
    /// Scale down cooldown
    pub scale_down_cooldown: Duration,
    /// Predictive scaling enabled
    pub predictive_enabled: bool,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check type
    pub check_type: HealthCheckType,
    /// Check interval
    pub interval: Duration,
    /// Check timeout
    pub timeout: Duration,
    /// Retries before marking unhealthy
    pub retries: u32,
    /// Initial delay before first check
    pub initial_delay: Duration,
}

/// Health check types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckType {
    /// HTTP endpoint check
    Http { path: String, port: u16 },
    /// TCP port check
    Tcp { port: u16 },
    /// Command execution check
    Command { command: Vec<String> },
    /// gRPC health check
    Grpc { service: String, port: u16 },
}

/// Container instance runtime information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInstance {
    /// Container specification
    pub spec: ContainerSpec,
    /// Current node assignment
    pub node_id: NodeId,
    /// Container state
    pub state: ContainerState,
    /// Current resource usage
    pub resource_usage: ResourceUsage,
    /// Health status
    pub health_status: HealthStatus,
    /// Start time
    pub start_time: SystemTime,
    /// Last updated
    pub last_updated: SystemTime,
    /// Restart count
    pub restart_count: u32,
    /// Assigned ports
    pub assigned_ports: HashMap<u16, u16>, // container_port -> host_port
}

/// Container states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerState {
    /// Container is being created
    Pending,
    /// Container is starting
    Starting,
    /// Container is running normally
    Running,
    /// Container is stopping
    Stopping,
    /// Container has stopped
    Stopped,
    /// Container has failed
    Failed,
    /// Container is being migrated
    Migrating,
    /// Container state is unknown
    Unknown,
}

/// Current resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU utilization (0.0 - 1.0)
    pub cpu_utilization: f64,
    /// Memory utilization (0.0 - 1.0)
    pub memory_utilization: f64,
    /// Network I/O bytes per second
    pub network_io_bps: u64,
    /// Disk I/O bytes per second
    pub disk_io_bps: u64,
    /// GPU utilization (0.0 - 1.0)
    pub gpu_utilization: Option<f64>,
    /// Last measurement time
    pub measured_at: SystemTime,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Health check not configured
    Unknown,
    /// Starting up, health check not ready
    Starting,
    /// Container is healthy
    Healthy,
    /// Container is unhealthy but still running
    Unhealthy,
    /// Container has failed health checks
    Failed,
}

/// Node state in the cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeState {
    /// Node identifier
    pub node_id: NodeId,
    /// Node availability
    pub available: bool,
    /// Total resources
    pub total_resources: NodeResources,
    /// Available resources
    pub available_resources: NodeResources,
    /// Allocated resources
    pub allocated_resources: NodeResources,
    /// Node labels
    pub labels: HashMap<String, String>,
    /// Node zone/region
    pub zone: Option<String>,
    /// Last heartbeat
    pub last_heartbeat: SystemTime,
    /// Node health
    pub health: NodeHealth,
    /// Performance metrics
    pub performance: NodePerformance,
}

/// Node health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeHealth {
    /// Node is healthy
    Healthy,
    /// Node has warnings
    Warning,
    /// Node is unhealthy
    Unhealthy,
    /// Node is unreachable
    Unreachable,
    /// Node is draining (no new containers)
    Draining,
}

/// Node performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePerformance {
    /// CPU load average (1 minute)
    pub load_average: f64,
    /// Memory pressure (0.0 - 1.0)
    pub memory_pressure: f64,
    /// Disk pressure (0.0 - 1.0)
    pub disk_pressure: f64,
    /// Network latency to other nodes (ms)
    pub network_latency_ms: f64,
    /// Container density (containers per core)
    pub container_density: f64,
}

/// Scheduling decision information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingDecision {
    /// Decision ID
    pub id: Uuid,
    /// Container being scheduled
    pub container_id: ContainerId,
    /// Selected node
    pub selected_node: NodeId,
    /// Node candidates evaluated
    pub node_candidates: Vec<NodeCandidate>,
    /// Decision latency (ms)
    pub decision_latency_ms: u64,
    /// Confidence in decision
    pub confidence: f64,
    /// DSR pattern matching used
    pub dsr_enhanced: bool,
    /// CPE prediction used
    pub cpe_enhanced: bool,
    /// IFR resource lookup used
    pub ifr_enhanced: bool,
    /// Expected performance improvement
    pub improvement_factor: f64,
    /// Decision timestamp
    pub timestamp: SystemTime,
}

/// Node selector for filtering candidates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSelector {
    /// Required labels
    pub match_labels: HashMap<String, String>,
    /// Label expressions
    pub match_expressions: Vec<LabelExpression>,
    /// Node names to include
    pub node_names: Option<Vec<NodeId>>,
    /// Zones to include
    pub zones: Option<Vec<String>>,
}

/// Label expression for advanced node selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelExpression {
    /// Label key
    pub key: String,
    /// Operator
    pub operator: LabelOperator,
    /// Values
    pub values: Vec<String>,
}

/// Label operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LabelOperator {
    In,
    NotIn,
    Exists,
    DoesNotExist,
    Gt,
    Lt,
}

/// Container orchestration performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMetrics {
    /// Total containers managed
    pub total_containers: u64,
    /// Running containers
    pub running_containers: u64,
    /// Failed containers
    pub failed_containers: u64,
    /// Scheduling decisions made
    pub scheduling_decisions: u64,
    /// DSR-enhanced scheduling percentage
    pub dsr_scheduling_percentage: f64,
    /// Average scheduling latency (ms)
    pub avg_scheduling_latency_ms: f64,
    /// Peak scheduling latency (ms)
    pub peak_scheduling_latency_ms: u64,
    /// Scheduling accuracy (successful placements)
    pub scheduling_accuracy: f64,
    /// IFR resource lookup percentage
    pub ifr_lookup_percentage: f64,
    /// CPE predictive placement percentage
    pub cpe_placement_percentage: f64,
    /// Container migration count
    pub migrations_performed: u64,
    /// Resource efficiency (utilization vs allocation)
    pub resource_efficiency: f64,
    /// Performance vs traditional improvement factor
    pub traditional_vs_mfn_factor: f64,
}

/// Container orchestration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    /// Total nodes in cluster
    pub total_nodes: usize,
    /// Available nodes
    pub available_nodes: usize,
    /// Total containers
    pub total_containers: usize,
    /// Running containers
    pub running_containers: usize,
    /// Pending containers
    pub pending_containers: usize,
    /// Failed containers
    pub failed_containers: usize,
    /// Average scheduling latency (ms)
    pub avg_scheduling_latency_ms: f64,
    /// MFN enhancement utilization
    pub mfn_utilization_percentage: f64,
    /// Cluster resource utilization
    pub cluster_cpu_utilization: f64,
    /// Cluster memory utilization
    pub cluster_memory_utilization: f64,
    /// Container density per node
    pub avg_container_density: f64,
}

/// Scaling actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingAction {
    /// Scale up by specified count
    ScaleUp(u32),
    /// Scale down by removing specified containers
    ScaleDown(Vec<ContainerId>),
    /// No scaling action needed
    NoAction,
}
