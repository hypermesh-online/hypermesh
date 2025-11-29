//! Extension system type definitions

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;
use semver::Version;

use crate::assets::core::{AssetType, AssetId, AssetAllocation, PrivacyLevel, ConsensusProof};

/// Extension system errors
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("Extension not found: {id}")]
    ExtensionNotFound { id: String },

    #[error("Extension already loaded: {id}")]
    ExtensionAlreadyLoaded { id: String },

    #[error("Dependency resolution failed: {extension} requires {dependency}")]
    DependencyResolutionFailed {
        extension: String,
        dependency: String,
    },

    #[error("Version incompatibility: {extension} requires {required}, found {found}")]
    VersionIncompatible {
        extension: String,
        required: String,
        found: String,
    },

    #[error("Capability not granted: {capability}")]
    CapabilityNotGranted { capability: String },

    #[error("Resource limit exceeded: {resource}")]
    ResourceLimitExceeded { resource: String },

    #[error("Consensus validation failed: {reason}")]
    ConsensusValidationFailed { reason: String },

    #[error("Certificate validation failed: {fingerprint}")]
    CertificateValidationFailed { fingerprint: String },

    #[error("Extension initialization failed: {reason}")]
    InitializationFailed { reason: String },

    #[error("Extension runtime error: {message}")]
    RuntimeError { message: String },

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Extension system result type
pub type ExtensionResult<T> = Result<T, ExtensionError>;

/// Extension metadata describing the plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub description: String,
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub category: ExtensionCategory,
    pub hypermesh_version: Version,
    pub dependencies: Vec<ExtensionDependency>,
    pub required_capabilities: HashSet<ExtensionCapability>,
    pub provided_assets: Vec<AssetType>,
    pub certificate_fingerprint: Option<String>,
    pub config_schema: Option<serde_json::Value>,
}

/// Extension categories for organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExtensionCategory {
    AssetLibrary,
    ComputeEngine,
    StorageProvider,
    NetworkProtocol,
    Security,
    Monitoring,
    DeveloperTools,
    MachineLearning,
    Blockchain,
    Custom(String),
}

/// Extension dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionDependency {
    pub extension_id: String,
    pub version_requirement: semver::VersionReq,
    pub optional: bool,
}

/// Security capabilities that extensions can request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExtensionCapability {
    AssetManagement,
    ContainerManagement,
    VMExecution,
    NetworkAccess,
    FileSystemAccess,
    ConsensusAccess,
    TrustChainAccess,
    TransportAccess,
    ProxyAccess,
    MonitoringAccess,
    UserDataAccess,
    ConfigurationAccess,
    Custom(String),
}

/// Extension configuration passed during initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfig {
    pub settings: serde_json::Value,
    pub resource_limits: ResourceLimits,
    pub granted_capabilities: HashSet<ExtensionCapability>,
    pub privacy_level: PrivacyLevel,
    pub debug_mode: bool,
}

/// Resource limits for extension execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_percent: f32,
    pub max_memory_bytes: u64,
    pub max_storage_bytes: u64,
    pub max_network_bandwidth: u64,
    pub max_concurrent_operations: usize,
    pub max_execution_time: Duration,
    pub max_assets: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_percent: 25.0,
            max_memory_bytes: 1024 * 1024 * 1024,
            max_storage_bytes: 10 * 1024 * 1024 * 1024,
            max_network_bandwidth: 100 * 1024 * 1024,
            max_concurrent_operations: 100,
            max_execution_time: Duration::from_secs(300),
            max_assets: 10000,
        }
    }
}

/// Extension status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStatus {
    pub state: ExtensionState,
    pub health: ExtensionHealth,
    pub resource_usage: ResourceUsageReport,
    pub active_operations: usize,
    pub total_requests: u64,
    pub error_count: u64,
    pub uptime: Duration,
}

/// Extension state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionState {
    Initializing,
    Running,
    Paused,
    ShuttingDown,
    Stopped,
    Error(String),
}

/// Extension health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionHealth {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

/// Resource usage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageReport {
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub network_bytes: u64,
    pub storage_bytes: u64,
}

/// Validation report for extension integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub certificate_valid: Option<bool>,
    pub dependencies_satisfied: bool,
    pub resource_compliance: bool,
    pub security_compliance: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub context: Option<serde_json::Value>,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
    pub context: Option<serde_json::Value>,
}

/// Extension state for export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStateData {
    pub version: u32,
    pub metadata: ExtensionMetadata,
    pub state_data: Vec<u8>,
    pub checksum: String,
    pub exported_at: std::time::SystemTime,
}

/// Extension request for custom API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRequest {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
    pub consensus_proof: Option<ConsensusProof>,
}

/// Extension response for custom API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionResponse {
    pub request_id: String,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Extension manager configuration
#[derive(Debug, Clone)]
pub struct ExtensionManagerConfig {
    pub extension_dirs: Vec<PathBuf>,
    pub auto_load: bool,
    pub verify_signatures: bool,
    pub max_extensions: usize,
    pub global_limits: ResourceLimits,
    pub allowed_capabilities: HashSet<ExtensionCapability>,
}

impl Default for ExtensionManagerConfig {
    fn default() -> Self {
        Self {
            extension_dirs: vec![PathBuf::from("./extensions")],
            auto_load: true,
            verify_signatures: true,
            max_extensions: 100,
            global_limits: ResourceLimits::default(),
            allowed_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::VMExecution,
                ExtensionCapability::NetworkAccess,
                ExtensionCapability::ConsensusAccess,
            ]),
        }
    }
}
