// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Type definitions for the Catalog-HyperMesh Integration Bridge

use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use serde::{Serialize, Deserialize};

use crate::assets::core::AssetType;

/// Catalog asset types that can be deployed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CatalogAssetType {
    /// Python application
    PythonApp {
        code: String,
        requirements_txt: String,
        entry_point: String,
    },
    /// Rust binary
    RustBinary {
        source_code: String,
        cargo_toml: String,
        target: String,
    },
    /// Container image
    ContainerImage {
        image_name: String,
        image_tag: String,
        registry_url: String,
        dockerfile: Option<String>,
    },
    /// WebAssembly module
    WasmModule {
        wasm_bytes: Vec<u8>,
        metadata: WasmMetadata,
    },
    /// Data processing pipeline
    DataPipeline {
        stages: Vec<PipelineStage>,
        data_sources: Vec<DataSource>,
        outputs: Vec<DataOutput>,
    },
}

/// WebAssembly module metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmMetadata {
    pub module_name: String,
    pub version: String,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
    pub memory_requirements: u64,
}

/// Data pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    pub stage_id: String,
    pub stage_type: StageType,
    pub configuration: serde_json::Value,
    pub dependencies: Vec<String>,
}

/// Pipeline stage types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageType {
    DataIngestion,
    DataTransformation,
    DataValidation,
    DataAggregation,
    DataOutput,
    MachineLearning,
    CustomProcessing(String),
}

/// Data source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub source_id: String,
    pub source_type: DataSourceType,
    pub connection_config: serde_json::Value,
    pub schema: Option<DataSchema>,
}

/// Data source types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSourceType {
    Database,
    FileSystem,
    S3,
    Stream,
    API,
    Blockchain,
    Custom(String),
}

/// Data schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSchema {
    pub fields: Vec<SchemaField>,
    pub constraints: Vec<DataConstraint>,
}

/// Schema field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    pub field_type: FieldType,
    pub nullable: bool,
    pub description: Option<String>,
}

/// Field types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    Json,
    Binary,
    Custom(String),
}

/// Data constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataConstraint {
    Required(String),
    Unique(String),
    Range(String, f64, f64),
    Pattern(String, String),
    Custom(String, serde_json::Value),
}

/// Data output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataOutput {
    pub output_id: String,
    pub output_type: DataOutputType,
    pub destination_config: serde_json::Value,
    pub format: DataFormat,
}

/// Data output types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataOutputType {
    Database,
    FileSystem,
    S3,
    Stream,
    API,
    Blockchain,
    Custom(String),
}

/// Data formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataFormat {
    Json,
    Csv,
    Parquet,
    Avro,
    Binary,
    Custom(String),
}

/// Catalog asset deployment specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogDeploymentSpec {
    /// Asset to deploy
    pub asset: CatalogAssetType,
    /// Deployment strategy
    pub deployment_strategy: DeploymentStrategy,
    /// Resource requirements
    pub resource_requirements: CatalogResourceRequirements,
    /// Privacy and security settings
    pub privacy_settings: CatalogPrivacySettings,
    /// Execution configuration
    pub execution_config: ExecutionConfiguration,
    /// Monitoring and observability
    pub monitoring_config: MonitoringConfiguration,
}

/// Deployment strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    /// Deploy as VM execution
    VMExecution {
        vm_config: VMDeploymentConfig,
    },
    /// Deploy as container
    Container {
        container_config: ContainerDeploymentConfig,
    },
    /// Deploy as serverless function
    Serverless {
        function_config: FunctionDeploymentConfig,
    },
    /// Hybrid deployment (VM + Container)
    Hybrid {
        vm_config: VMDeploymentConfig,
        container_config: ContainerDeploymentConfig,
    },
}

/// VM deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMDeploymentConfig {
    pub language_runtime: String,
    pub execution_timeout: Duration,
    pub memory_limit: u64,
    pub cpu_limit: u32,
    pub enable_gpu: bool,
    pub environment_variables: HashMap<String, String>,
}

/// Container deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerDeploymentConfig {
    pub base_image: String,
    pub ports: Vec<PortMapping>,
    pub volumes: Vec<VolumeMount>,
    pub environment_variables: HashMap<String, String>,
    pub command: Vec<String>,
    pub args: Vec<String>,
}

/// Port mapping for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String,
}

/// Volume mount for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub source: String,
    pub destination: String,
    pub read_only: bool,
}

/// Function deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeploymentConfig {
    pub runtime: String,
    pub handler: String,
    pub timeout: Duration,
    pub memory_size: u64,
    pub triggers: Vec<FunctionTrigger>,
}

/// Function triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionTrigger {
    Http { path: String, methods: Vec<String> },
    Timer { schedule: String },
    Queue { queue_name: String },
    Event { event_type: String },
}

/// Resource requirements for catalog assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogResourceRequirements {
    pub cpu_cores: Option<u32>,
    pub memory_mb: Option<u64>,
    pub storage_gb: Option<u64>,
    pub gpu_count: Option<u32>,
    pub network_bandwidth_mbps: Option<u64>,
    pub custom_resources: HashMap<String, u64>,
}

/// Privacy settings for catalog deployments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogPrivacySettings {
    pub data_privacy_level: DataPrivacyLevel,
    pub execution_privacy_level: ExecutionPrivacyLevel,
    pub network_isolation: bool,
    pub encrypt_at_rest: bool,
    pub encrypt_in_transit: bool,
    pub access_control_policies: Vec<AccessPolicy>,
}

/// Data privacy levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataPrivacyLevel {
    Public,
    Internal,
    Confidential,
    Restricted,
    TopSecret,
}

/// Execution privacy levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionPrivacyLevel {
    Open,
    Isolated,
    Sandboxed,
    Encrypted,
    SecureEnclave,
}

/// Access control policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub policy_id: String,
    pub subjects: Vec<String>,
    pub permissions: Vec<Permission>,
    pub conditions: Vec<AccessCondition>,
}

/// Permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Delete,
    Admin,
    Custom(String),
}

/// Access conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessCondition {
    TimeRange { start: SystemTime, end: SystemTime },
    Location { allowed_locations: Vec<String> },
    Network { allowed_networks: Vec<String> },
    Custom(String, serde_json::Value),
}

// Execution config types are in the config submodule
pub use super::config::*;

// Monitoring types are in the monitoring submodule
pub use super::monitoring::*;

/// Deployment information tracking
#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    pub deployment_id: String,
    pub asset_type: String,
    pub deployment_strategy: String,
    pub deployed_at: SystemTime,
    pub status: DeploymentStatus,
    pub resource_allocations: HashMap<AssetType, u64>,
    pub performance_metrics: PerformanceMetrics,
}

/// Deployment status
#[derive(Debug, Clone)]
pub enum DeploymentStatus {
    Pending,
    Deploying,
    Running,
    Scaling,
    Stopping,
    Stopped,
    Failed(String),
}

/// Performance metrics for deployments
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub execution_count: u64,
    pub average_execution_time: Duration,
    pub success_rate: f64,
    pub error_rate: f64,
    pub throughput: f64,
    pub latency_p95: Duration,
    pub resource_efficiency: f64,
}

/// Bridge metrics
#[derive(Debug, Default)]
pub struct BridgeMetrics {
    pub total_deployments: u64,
    pub vm_deployments: u64,
    pub container_deployments: u64,
    pub hybrid_deployments: u64,
    pub successful_deployments: u64,
    pub failed_deployments: u64,
    pub average_deployment_time: Duration,
    pub resource_utilization: f64,
}

impl Clone for BridgeMetrics {
    fn clone(&self) -> Self {
        Self {
            total_deployments: self.total_deployments,
            vm_deployments: self.vm_deployments,
            container_deployments: self.container_deployments,
            hybrid_deployments: self.hybrid_deployments,
            successful_deployments: self.successful_deployments,
            failed_deployments: self.failed_deployments,
            average_deployment_time: self.average_deployment_time,
            resource_utilization: self.resource_utilization,
        }
    }
}

/// Bridge configuration
#[derive(Debug, Clone)]
pub struct BridgeConfiguration {
    pub enable_vm_deployments: bool,
    pub enable_container_deployments: bool,
    pub enable_hybrid_deployments: bool,
    pub default_vm_config: VMDeploymentConfig,
    pub default_container_config: ContainerDeploymentConfig,
    pub max_concurrent_deployments: u32,
    pub deployment_timeout: Duration,
}

impl Default for BridgeConfiguration {
    fn default() -> Self {
        Self {
            enable_vm_deployments: true,
            enable_container_deployments: true,
            enable_hybrid_deployments: true,
            default_vm_config: VMDeploymentConfig {
                language_runtime: "julia".to_string(),
                execution_timeout: Duration::from_secs(300),
                memory_limit: 1024 * 1024 * 1024, // 1GB
                cpu_limit: 2,
                enable_gpu: false,
                environment_variables: HashMap::new(),
            },
            default_container_config: ContainerDeploymentConfig {
                base_image: "ubuntu:20.04".to_string(),
                ports: vec![],
                volumes: vec![],
                environment_variables: HashMap::new(),
                command: vec![],
                args: vec![],
            },
            max_concurrent_deployments: 50,
            deployment_timeout: Duration::from_secs(600),
        }
    }
}

/// Catalog deployment result
#[derive(Debug, Clone)]
pub struct CatalogDeploymentResult {
    pub deployment_id: String,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub deployment_time: Duration,
    pub resource_allocations: HashMap<AssetType, u64>,
    pub performance_metrics: PerformanceMetrics,
}

/// Internal deployment result
#[derive(Debug)]
pub(crate) struct InternalDeploymentResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub resource_allocations: HashMap<AssetType, u64>,
}
