//! VM Type Definitions and Data Structures
//!
//! Contains all type definitions for VM execution including specifications,
//! resource requirements, and configuration structures.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::Duration;

/// VM execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmExecutionRequest {
    /// Operation to perform
    pub operation: String,

    /// VM specification
    pub vm_spec: VmSpecification,

    /// Resource requirements
    pub resource_requirements: VmResourceRequirements,

    /// Execution parameters
    pub execution_params: ExecutionParameters,

    /// Security configuration
    pub security_config: super::security::VmSecurityConfig,
}

/// VM specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSpecification {
    /// VM image information
    pub image_id: String,
    pub image_source: VmImageSource,

    /// VM configuration
    pub vm_type: VmType,
    pub vm_size: VmSize,

    /// Runtime configuration
    pub runtime_config: RuntimeConfig,

    /// Network configuration
    pub network_config: NetworkConfig,

    /// Storage configuration
    pub storage_config: StorageConfig,
}

/// VM image sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmImageSource {
    /// From Catalog system
    Catalog,
    /// Pre-built system image
    System,
    /// Container image
    Container,
    /// Custom image
    Custom,
}

/// VM types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmType {
    /// General purpose VM
    General,
    /// Compute optimized
    Compute,
    /// Memory optimized
    Memory,
    /// Storage optimized
    Storage,
    /// GPU accelerated
    Gpu,
    /// Network optimized
    Network,
    /// Custom configuration
    Custom,
}

/// VM sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmSize {
    /// Nano: 0.5 CPU, 512MB RAM
    Nano,
    /// Micro: 1 CPU, 1GB RAM
    Micro,
    /// Small: 2 CPU, 2GB RAM
    Small,
    /// Medium: 4 CPU, 8GB RAM
    Medium,
    /// Large: 8 CPU, 16GB RAM
    Large,
    /// XLarge: 16 CPU, 32GB RAM
    XLarge,
    /// Custom size
    Custom(VmCustomSize),
}

/// Custom VM size specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmCustomSize {
    pub cpu_cores: u32,
    pub memory_gb: f32,
    pub storage_gb: Option<u32>,
    pub gpu_count: Option<u32>,
}

/// VM resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmResourceRequirements {
    /// CPU requirements
    pub cpu: CpuRequirements,

    /// Memory requirements
    pub memory: MemoryRequirements,

    /// Storage requirements
    pub storage: Option<StorageRequirements>,

    /// Network requirements
    pub network: Option<NetworkRequirements>,

    /// GPU requirements
    pub gpu: Option<GpuRequirements>,
}

/// CPU requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirements {
    pub cores: u32,
    pub min_frequency: Option<f32>,
    pub architecture: Option<String>,
}

/// Memory requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    pub size_gb: f32,
    pub memory_type: MemoryType,
}

/// Memory types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Standard,
    HighSpeed,
    Persistent,
}

/// Storage requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    pub size_gb: u32,
    pub storage_type: StorageType,
    pub iops: Option<u32>,
}

/// Storage types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageType {
    Standard,
    Ssd,
    NvmeSsd,
}

/// Network requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    pub bandwidth_gbps: f32,
    pub latency_ms: Option<f32>,
}

/// GPU requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    pub count: u32,
    pub memory_gb: f32,
    pub compute_capability: Option<String>,
}

/// Execution parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionParameters {
    /// Execution timeout
    pub timeout: Option<Duration>,

    /// Execution priority
    pub priority: ExecutionPriority,

    /// Environment variables
    pub environment: HashMap<String, String>,

    /// Command arguments
    pub args: Vec<String>,

    /// Working directory
    pub working_directory: Option<String>,

    /// Resource limits
    pub resource_limits: Option<ResourceLimits>,
}

/// Execution priorities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Resource limits for VM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_percent: Option<f32>,
    pub max_memory_gb: Option<f32>,
    pub max_storage_gb: Option<u32>,
    pub max_network_mbps: Option<f32>,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub runtime_type: RuntimeType,
    pub version: Option<String>,
    pub extensions: Vec<String>,
    pub health_check: Option<HealthCheckConfig>,
}

/// Runtime types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeType {
    /// Docker container runtime
    Docker,
    /// Podman container runtime
    Podman,
    /// KVM virtual machine
    Kvm,
    /// QEMU virtual machine
    Qemu,
    /// WebAssembly runtime
    Wasm,
    /// Native process
    Native,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub check_type: HealthCheckType,
    pub interval: Duration,
    pub timeout: Duration,
    pub retries: u32,
}

/// Health check types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthCheckType {
    Http,
    Tcp,
    Command,
    Script,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub network_mode: NetworkMode,
    pub ports: Vec<PortMapping>,
    pub dns_config: Option<DnsConfig>,
}

/// Network modes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    Bridge,
    Host,
    Isolated,
    Custom,
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub nameservers: Vec<String>,
    pub search_domains: Vec<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub volumes: Vec<VolumeMount>,
    pub ephemeral_storage_gb: Option<u32>,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

/// VM execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmExecution {
    pub execution_id: String,
    pub vm_id: String,
    pub status: VmStatus,
    pub started_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub resource_usage: ResourceUsage,
    pub output: Option<ExecutionOutput>,
    pub error: Option<String>,
}

/// VM status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmStatus {
    Pending,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Completed,
}

/// Resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_seconds: f64,
    pub memory_gb_hours: f64,
    pub storage_gb_hours: f64,
    pub network_gb: f64,
    pub gpu_hours: f64,
}

/// Execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub artifacts: Vec<String>,
}

/// VM registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmRegistryEntry {
    pub image_id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub size_gb: f32,
    pub created_at: SystemTime,
    pub tags: Vec<String>,
}

/// VM statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VmStats {
    pub total_executions: u64,
    pub active_executions: u32,
    pub completed_executions: u64,
    pub failed_executions: u64,
    pub total_cpu_hours: f64,
    pub total_memory_gb_hours: f64,
    pub total_storage_gb_hours: f64,
    pub total_network_gb: f64,
    pub total_gpu_hours: f64,
}

/// VM registry
#[derive(Debug, Clone, Default)]
pub struct VmRegistry {
    pub entries: HashMap<String, VmRegistryEntry>,
}

use std::time::SystemTime;

/// VM Asset representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmAsset {
    pub execution_id: String,
    pub vm_id: String,
    pub status: VmStatus,
    pub resource_allocation: ResourceAllocation,
}

/// Resource allocation for VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub cpu_cores: u32,
    pub memory_gb: f32,
    pub storage_gb: u64,
    pub network_gbps: f32,
}