//! HyperMesh Extension Interface Architecture
//!
//! This module defines the comprehensive plugin/extension system for HyperMesh,
//! allowing external components like Catalog to integrate as dynamic extensions
//! that provide specialized functionality while maintaining consensus validation
//! and security requirements.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    HyperMesh Core System                        │
//! │                                                                  │
//! │  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐         │
//! │  │   Asset     │  │   Consensus  │  │   Transport   │         │
//! │  │   Manager   │  │   (NKrypt)   │  │    (STOQ)     │         │
//! │  └──────┬──────┘  └──────┬───────┘  └───────┬───────┘         │
//! │         │                 │                   │                  │
//! │  ┌──────┴─────────────────┴───────────────────┴──────────┐     │
//! │  │              Extension Manager Runtime                  │     │
//! │  │                                                          │     │
//! │  │  • Dynamic Loading    • Dependency Resolution           │     │
//! │  │  • Lifecycle Control  • Security Sandboxing             │     │
//! │  │  • Resource Limits    • Consensus Validation            │     │
//! │  └──────────────────────────────────────────────────────────┘   │
//! └─────────────────────────┬────────────────────────────────────┘
//!                           │ Extension Interface
//!     ┌────────────────────┴────────────────────────┐
//!     │                                              │
//! ┌────┴──────┐  ┌──────────────┐  ┌───────────────┴───────────┐
//! │  Catalog  │  │   Custom     │  │     Future Extensions     │
//! │ Extension │  │  Extensions  │  │                           │
//! │           │  │              │  │  • Analytics Engine       │
//! │ • Assets  │  │ • User Apps  │  │  • Machine Learning       │
//! │ • Library │  │ • Protocols  │  │  • Specialized Compute    │
//! │ • VM/Julia│  │ • Services   │  │  • Domain-Specific Tools  │
//! └───────────┘  └──────────────┘  └──────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! All extensions operate under strict security constraints:
//! - Capability-based security with explicit permission grants
//! - Resource quotas and runtime limits
//! - Consensus validation for critical operations
//! - TrustChain certificate verification for signed extensions
//! - Isolated execution environments with controlled API access

#![warn(missing_docs)]
#![deny(unsafe_code)]

// Submodules for extension system implementation
pub mod loader;
pub mod manager;
pub mod registry;
pub mod security;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use semver::Version;

// Import core HyperMesh types
use crate::assets::core::{
    AssetManager, AssetId, AssetType, ConsensusProof,
    PrivacyLevel, AssetAllocation, ProxyAddress,
};
use crate::consensus::nkrypt_integration::{
    SpaceProof, StakeProof, WorkProof, TimeProof,
};

/// Extension system result type
pub type ExtensionResult<T> = Result<T, ExtensionError>;

/// Extension system errors
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    /// Extension not found
    #[error("Extension not found: {id}")]
    ExtensionNotFound { id: String },

    /// Extension already loaded
    #[error("Extension already loaded: {id}")]
    ExtensionAlreadyLoaded { id: String },

    /// Dependency resolution failed
    #[error("Dependency resolution failed: {extension} requires {dependency}")]
    DependencyResolutionFailed {
        extension: String,
        dependency: String,
    },

    /// Version incompatibility
    #[error("Version incompatibility: {extension} requires {required}, found {found}")]
    VersionIncompatible {
        extension: String,
        required: String,
        found: String,
    },

    /// Capability not granted
    #[error("Capability not granted: {capability}")]
    CapabilityNotGranted { capability: String },

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {resource}")]
    ResourceLimitExceeded { resource: String },

    /// Consensus validation failed
    #[error("Consensus validation failed: {reason}")]
    ConsensusValidationFailed { reason: String },

    /// Certificate validation failed
    #[error("Certificate validation failed: {fingerprint}")]
    CertificateValidationFailed { fingerprint: String },

    /// Extension initialization failed
    #[error("Extension initialization failed: {reason}")]
    InitializationFailed { reason: String },

    /// Extension runtime error
    #[error("Extension runtime error: {message}")]
    RuntimeError { message: String },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Extension metadata describing the plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    /// Unique identifier for the extension
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Extension version (semver)
    pub version: Version,

    /// Description of functionality
    pub description: String,

    /// Author information
    pub author: String,

    /// License identifier (SPDX)
    pub license: String,

    /// Homepage or repository URL
    pub homepage: Option<String>,

    /// Extension category
    pub category: ExtensionCategory,

    /// Required HyperMesh version
    pub hypermesh_version: Version,

    /// Dependencies on other extensions
    pub dependencies: Vec<ExtensionDependency>,

    /// Required capabilities
    pub required_capabilities: HashSet<ExtensionCapability>,

    /// Asset types this extension provides/manages
    pub provided_assets: Vec<AssetType>,

    /// TrustChain certificate fingerprint for verification
    pub certificate_fingerprint: Option<String>,

    /// Extension-specific configuration schema
    pub config_schema: Option<serde_json::Value>,
}

/// Extension categories for organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExtensionCategory {
    /// Asset management and libraries (like Catalog)
    AssetLibrary,
    /// Compute and execution engines
    ComputeEngine,
    /// Storage backends and providers
    StorageProvider,
    /// Networking and transport protocols
    NetworkProtocol,
    /// Security and authentication
    Security,
    /// Monitoring and observability
    Monitoring,
    /// Developer tools and utilities
    DeveloperTools,
    /// Machine learning and AI
    MachineLearning,
    /// Blockchain and consensus
    Blockchain,
    /// Custom user-defined category
    Custom(String),
}

/// Extension dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionDependency {
    /// Extension ID
    pub extension_id: String,

    /// Required version range
    pub version_requirement: semver::VersionReq,

    /// Whether this dependency is optional
    pub optional: bool,
}

/// Security capabilities that extensions can request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExtensionCapability {
    /// Access to asset management system
    AssetManagement,
    /// Create and manage containers
    ContainerManagement,
    /// Execute code through VM runtime
    VMExecution,
    /// Network access (inbound/outbound)
    NetworkAccess,
    /// File system access
    FileSystemAccess,
    /// Access to consensus validation
    ConsensusAccess,
    /// Access to TrustChain certificates
    TrustChainAccess,
    /// Access to STOQ transport layer
    TransportAccess,
    /// Access to proxy/NAT system
    ProxyAccess,
    /// Monitoring and metrics collection
    MonitoringAccess,
    /// User data access
    UserDataAccess,
    /// System configuration access
    ConfigurationAccess,
    /// Custom capability
    Custom(String),
}

/// Extension configuration passed during initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfig {
    /// Extension-specific configuration values
    pub settings: serde_json::Value,

    /// Resource limits for this extension
    pub resource_limits: ResourceLimits,

    /// Granted capabilities
    pub granted_capabilities: HashSet<ExtensionCapability>,

    /// Privacy level for extension operations
    pub privacy_level: PrivacyLevel,

    /// Whether to enable debug mode
    pub debug_mode: bool,
}

/// Resource limits for extension execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU usage percentage (0-100)
    pub max_cpu_percent: f32,

    /// Maximum memory in bytes
    pub max_memory_bytes: u64,

    /// Maximum storage in bytes
    pub max_storage_bytes: u64,

    /// Maximum network bandwidth in bytes/sec
    pub max_network_bandwidth: u64,

    /// Maximum concurrent operations
    pub max_concurrent_operations: usize,

    /// Maximum execution time per operation
    pub max_execution_time: std::time::Duration,

    /// Maximum number of assets
    pub max_assets: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_percent: 25.0,
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            max_storage_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            max_network_bandwidth: 100 * 1024 * 1024, // 100MB/s
            max_concurrent_operations: 100,
            max_execution_time: std::time::Duration::from_secs(300),
            max_assets: 10000,
        }
    }
}

/// Core trait that all HyperMesh extensions must implement
#[async_trait]
pub trait HyperMeshExtension: Send + Sync {
    /// Get extension metadata
    fn metadata(&self) -> ExtensionMetadata;

    /// Initialize the extension with configuration
    /// Called once when the extension is loaded
    async fn initialize(&mut self, config: ExtensionConfig) -> ExtensionResult<()>;

    /// Register assets provided by this extension
    /// Returns a map of asset types to their handlers
    async fn register_assets(&self) -> ExtensionResult<HashMap<AssetType, Box<dyn AssetExtensionHandler>>>;

    /// Extend the asset manager with custom functionality
    /// Allows the extension to add new capabilities to the core asset system
    async fn extend_manager(&self, asset_manager: Arc<AssetManager>) -> ExtensionResult<()>;

    /// Handle extension-specific API calls
    /// Provides a way for extensions to expose custom APIs
    async fn handle_request(&self, request: ExtensionRequest) -> ExtensionResult<ExtensionResponse>;

    /// Get current extension status
    async fn status(&self) -> ExtensionStatus;

    /// Validate extension integrity and configuration
    async fn validate(&self) -> ExtensionResult<ValidationReport>;

    /// Export extension state for migration or backup
    async fn export_state(&self) -> ExtensionResult<ExtensionStateData>;

    /// Import previously exported state
    async fn import_state(&mut self, state: ExtensionStateData) -> ExtensionResult<()>;

    /// Shutdown the extension gracefully
    /// Called when the extension is being unloaded
    async fn shutdown(&mut self) -> ExtensionResult<()>;
}

/// Handler for extension-provided assets
#[async_trait]
pub trait AssetExtensionHandler: Send + Sync {
    /// Get asset type this handler manages
    fn asset_type(&self) -> AssetType;

    /// Create a new asset instance
    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetId>;

    /// Update an existing asset
    async fn update_asset(&self, id: &AssetId, update: AssetUpdate) -> ExtensionResult<()>;

    /// Delete an asset
    async fn delete_asset(&self, id: &AssetId) -> ExtensionResult<()>;

    /// Query assets based on criteria
    async fn query_assets(&self, query: AssetQuery) -> ExtensionResult<Vec<AssetId>>;

    /// Get asset metadata
    async fn get_metadata(&self, id: &AssetId) -> ExtensionResult<AssetMetadata>;

    /// Validate asset with consensus proofs
    async fn validate_asset(&self, id: &AssetId, proof: ConsensusProof) -> ExtensionResult<bool>;

    /// Handle asset-specific operations
    async fn handle_operation(&self, id: &AssetId, operation: AssetOperation) -> ExtensionResult<OperationResult>;
}

/// Asset creation specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetCreationSpec {
    /// Asset name
    pub name: String,

    /// Asset description
    pub description: Option<String>,

    /// Asset metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Privacy level for the asset
    pub privacy_level: PrivacyLevel,

    /// Initial allocation settings
    pub allocation: Option<AssetAllocation>,

    /// Consensus requirements
    pub consensus_requirements: ConsensusRequirements,

    /// Parent asset ID (for hierarchical assets)
    pub parent_id: Option<AssetId>,

    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Asset update specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetUpdate {
    /// Updated name
    pub name: Option<String>,

    /// Updated description
    pub description: Option<String>,

    /// Updated metadata (merged with existing)
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// Updated privacy level
    pub privacy_level: Option<PrivacyLevel>,

    /// Updated allocation
    pub allocation: Option<AssetAllocation>,

    /// Updated tags
    pub tags: Option<Vec<String>>,
}

/// Asset query specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetQuery {
    /// Filter by asset type
    pub asset_type: Option<AssetType>,

    /// Filter by name pattern
    pub name_pattern: Option<String>,

    /// Filter by tags
    pub tags: Option<Vec<String>>,

    /// Filter by privacy level
    pub privacy_level: Option<PrivacyLevel>,

    /// Filter by parent ID
    pub parent_id: Option<AssetId>,

    /// Maximum results to return
    pub limit: Option<usize>,

    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Asset metadata returned by extensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    /// Asset ID
    pub id: AssetId,

    /// Asset type
    pub asset_type: AssetType,

    /// Asset name
    pub name: String,

    /// Asset description
    pub description: Option<String>,

    /// Creation timestamp
    pub created_at: std::time::SystemTime,

    /// Last modification timestamp
    pub updated_at: std::time::SystemTime,

    /// Asset size in bytes
    pub size_bytes: u64,

    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Current privacy level
    pub privacy_level: PrivacyLevel,

    /// Current allocation
    pub allocation: Option<AssetAllocation>,

    /// Consensus validation status
    pub consensus_status: ConsensusStatus,

    /// Associated tags
    pub tags: Vec<String>,
}

/// Asset-specific operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetOperation {
    /// Deploy asset (for deployable assets like VMs, containers)
    Deploy(DeploymentSpec),

    /// Execute code (for VM assets)
    Execute(ExecutionSpec),

    /// Transfer ownership
    Transfer(TransferSpec),

    /// Share with other users
    Share(SharingSpec),

    /// Validate with consensus
    Validate(ConsensusProof),

    /// Custom operation
    Custom(serde_json::Value),
}

/// Result of asset operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationResult {
    /// Deployment result
    Deployed(DeploymentResult),

    /// Execution result
    Executed(ExecutionResult),

    /// Transfer result
    Transferred(TransferResult),

    /// Sharing result
    Shared(SharingResult),

    /// Validation result
    Validated(bool),

    /// Custom result
    Custom(serde_json::Value),
}

/// Deployment specification for deployable assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentSpec {
    /// Target environment
    pub environment: String,

    /// Resource requirements
    pub resources: ResourceRequirements,

    /// Environment variables
    pub env_vars: HashMap<String, String>,

    /// Network configuration
    pub network_config: Option<NetworkConfig>,

    /// Volume mounts
    pub volumes: Vec<VolumeMount>,

    /// Consensus proof for deployment
    pub consensus_proof: ConsensusProof,
}

/// Resource requirements for deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU requirements
    pub cpu: CpuRequirement,

    /// Memory requirements
    pub memory: MemoryRequirement,

    /// Storage requirements
    pub storage: Option<StorageRequirement>,

    /// GPU requirements
    pub gpu: Option<GpuRequirement>,
}

/// CPU requirement specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirement {
    /// Minimum CPU cores
    pub min_cores: f32,

    /// Maximum CPU cores
    pub max_cores: Option<f32>,

    /// CPU architecture (x86_64, arm64, etc)
    pub architecture: Option<String>,
}

/// Memory requirement specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirement {
    /// Minimum memory in bytes
    pub min_bytes: u64,

    /// Maximum memory in bytes
    pub max_bytes: Option<u64>,
}

/// Storage requirement specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirement {
    /// Minimum storage in bytes
    pub min_bytes: u64,

    /// Storage type (ssd, hdd, nvme)
    pub storage_type: Option<String>,
}

/// GPU requirement specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirement {
    /// Minimum GPU count
    pub min_count: u32,

    /// GPU model requirements
    pub models: Option<Vec<String>>,

    /// Minimum VRAM in bytes
    pub min_vram: Option<u64>,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Port mappings
    pub ports: Vec<PortMapping>,

    /// DNS settings
    pub dns: Option<Vec<String>>,

    /// Network mode (bridge, host, none)
    pub mode: String,
}

/// Port mapping specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Host port
    pub host_port: u16,

    /// Container port
    pub container_port: u16,

    /// Protocol (tcp, udp)
    pub protocol: String,
}

/// Volume mount specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Source path
    pub source: String,

    /// Target path
    pub target: String,

    /// Read-only flag
    pub read_only: bool,
}

/// Execution specification for code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSpec {
    /// Code to execute
    pub code: String,

    /// Programming language
    pub language: String,

    /// Input parameters
    pub inputs: HashMap<String, serde_json::Value>,

    /// Execution timeout
    pub timeout: Option<std::time::Duration>,

    /// Consensus proof
    pub consensus_proof: ConsensusProof,
}

/// Transfer specification for ownership transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSpec {
    /// New owner ID
    pub new_owner: String,

    /// Transfer reason
    pub reason: Option<String>,

    /// Consensus proof
    pub consensus_proof: ConsensusProof,
}

/// Sharing specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingSpec {
    /// Users to share with
    pub users: Vec<String>,

    /// Access level (read, write, admin)
    pub access_level: String,

    /// Expiration time
    pub expires_at: Option<std::time::SystemTime>,

    /// Consensus proof
    pub consensus_proof: ConsensusProof,
}

/// Deployment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentResult {
    /// Deployment ID
    pub deployment_id: String,

    /// Deployment status
    pub status: String,

    /// Access endpoints
    pub endpoints: Vec<String>,

    /// Deployment metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Execution ID
    pub execution_id: String,

    /// Output data
    pub output: serde_json::Value,

    /// Execution time
    pub execution_time: std::time::Duration,

    /// Resource usage
    pub resource_usage: ResourceUsageReport,
}

/// Transfer result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    /// Transfer ID
    pub transfer_id: String,

    /// Transfer timestamp
    pub transferred_at: std::time::SystemTime,

    /// New owner
    pub new_owner: String,
}

/// Sharing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingResult {
    /// Share ID
    pub share_id: String,

    /// Shared with users
    pub shared_with: Vec<String>,

    /// Share timestamp
    pub shared_at: std::time::SystemTime,
}

/// Resource usage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageReport {
    /// CPU usage in core-seconds
    pub cpu_usage: f64,

    /// Memory usage in byte-seconds
    pub memory_usage: u64,

    /// Network bytes transferred
    pub network_bytes: u64,

    /// Storage bytes used
    pub storage_bytes: u64,
}

/// Consensus requirements for assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusRequirements {
    /// Require Proof of Space
    pub require_proof_of_space: bool,

    /// Require Proof of Stake
    pub require_proof_of_stake: bool,

    /// Require Proof of Work
    pub require_proof_of_work: bool,

    /// Require Proof of Time
    pub require_proof_of_time: bool,

    /// Minimum space commitment
    pub min_space_commitment: Option<u64>,

    /// Minimum stake amount
    pub min_stake_amount: Option<u64>,

    /// Minimum work difficulty
    pub min_work_difficulty: Option<u32>,

    /// Time window for validation
    pub time_window: Option<std::time::Duration>,
}

impl Default for ConsensusRequirements {
    fn default() -> Self {
        Self {
            require_proof_of_space: true,
            require_proof_of_stake: true,
            require_proof_of_work: true,
            require_proof_of_time: true,
            min_space_commitment: Some(1024 * 1024), // 1MB
            min_stake_amount: Some(100),
            min_work_difficulty: Some(4),
            time_window: Some(std::time::Duration::from_secs(300)),
        }
    }
}

/// Consensus validation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStatus {
    /// Whether consensus is validated
    pub validated: bool,

    /// Last validation timestamp
    pub last_validated: Option<std::time::SystemTime>,

    /// Validation proofs
    pub proofs: Option<ConsensusProof>,

    /// Validation errors
    pub errors: Vec<String>,
}

/// Extension request for custom API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRequest {
    /// Request ID for tracking
    pub id: String,

    /// Request method/operation
    pub method: String,

    /// Request parameters
    pub params: serde_json::Value,

    /// Optional consensus proof
    pub consensus_proof: Option<ConsensusProof>,
}

/// Extension response for custom API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionResponse {
    /// Request ID this responds to
    pub request_id: String,

    /// Success flag
    pub success: bool,

    /// Response data
    pub data: Option<serde_json::Value>,

    /// Error message if failed
    pub error: Option<String>,
}

/// Extension status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStatus {
    /// Extension state
    pub state: ExtensionState,

    /// Health status
    pub health: ExtensionHealth,

    /// Resource usage
    pub resource_usage: ResourceUsageReport,

    /// Active operations count
    pub active_operations: usize,

    /// Total processed requests
    pub total_requests: u64,

    /// Error count
    pub error_count: u64,

    /// Uptime duration
    pub uptime: std::time::Duration,
}

/// Extension state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionState {
    /// Extension is initializing
    Initializing,

    /// Extension is running normally
    Running,

    /// Extension is paused
    Paused,

    /// Extension is shutting down
    ShuttingDown,

    /// Extension has stopped
    Stopped,

    /// Extension has errored
    Error(String),
}

/// Extension health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionHealth {
    /// Extension is healthy
    Healthy,

    /// Extension is degraded but functional
    Degraded(String),

    /// Extension is unhealthy
    Unhealthy(String),
}

/// Validation report for extension integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Overall validation status
    pub valid: bool,

    /// Certificate validation status
    pub certificate_valid: Option<bool>,

    /// Dependency validation
    pub dependencies_satisfied: bool,

    /// Resource limits compliance
    pub resource_compliance: bool,

    /// Security policy compliance
    pub security_compliance: bool,

    /// Validation errors
    pub errors: Vec<ValidationError>,

    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error context
    pub context: Option<serde_json::Value>,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,

    /// Warning message
    pub message: String,

    /// Warning context
    pub context: Option<serde_json::Value>,
}

/// Extension state for export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStateData {
    /// State version for compatibility
    pub version: u32,

    /// Extension metadata
    pub metadata: ExtensionMetadata,

    /// Serialized state data
    pub state_data: Vec<u8>,

    /// State checksum for integrity
    pub checksum: String,

    /// Export timestamp
    pub exported_at: std::time::SystemTime,
}

/// Asset library extension trait for Catalog-like functionality
#[async_trait]
pub trait AssetLibraryExtension: HyperMeshExtension {
    /// List available asset packages
    async fn list_packages(&self, filter: PackageFilter) -> ExtensionResult<Vec<AssetPackage>>;

    /// Get package details
    async fn get_package(&self, package_id: &str) -> ExtensionResult<AssetPackage>;

    /// Install an asset package
    async fn install_package(&self, package_id: &str, options: InstallOptions) -> ExtensionResult<InstallResult>;

    /// Uninstall an asset package
    async fn uninstall_package(&self, package_id: &str) -> ExtensionResult<()>;

    /// Update an installed package
    async fn update_package(&self, package_id: &str, version: Option<Version>) -> ExtensionResult<UpdateResult>;

    /// Search for packages
    async fn search_packages(&self, query: &str, options: SearchOptions) -> ExtensionResult<Vec<AssetPackage>>;

    /// Publish a new package to the library
    async fn publish_package(&self, package: AssetPackageSpec, proof: ConsensusProof) -> ExtensionResult<PublishResult>;

    /// Verify package integrity
    async fn verify_package(&self, package_id: &str) -> ExtensionResult<VerificationResult>;
}

/// Asset package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPackage {
    /// Package ID
    pub id: String,

    /// Package name
    pub name: String,

    /// Package version
    pub version: Version,

    /// Package description
    pub description: String,

    /// Package author
    pub author: String,

    /// Package license
    pub license: String,

    /// Asset types provided
    pub asset_types: Vec<AssetType>,

    /// Package size
    pub size_bytes: u64,

    /// Installation count
    pub install_count: u64,

    /// Package rating (0-5)
    pub rating: f32,

    /// Dependencies
    pub dependencies: Vec<PackageDependency>,

    /// TrustChain signature
    pub signature: Option<String>,

    /// IPFS/STOQ distribution hash
    pub distribution_hash: String,

    /// Package metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Package filter for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFilter {
    /// Filter by asset type
    pub asset_type: Option<AssetType>,

    /// Filter by author
    pub author: Option<String>,

    /// Filter by license
    pub license: Option<String>,

    /// Minimum rating
    pub min_rating: Option<f32>,

    /// Only verified packages
    pub verified_only: bool,
}

/// Package dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDependency {
    /// Package ID
    pub package_id: String,

    /// Version requirement
    pub version_req: semver::VersionReq,

    /// Optional dependency
    pub optional: bool,
}

/// Installation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallOptions {
    /// Installation directory
    pub install_dir: Option<PathBuf>,

    /// Include optional dependencies
    pub include_optional: bool,

    /// Verify signatures
    pub verify_signatures: bool,

    /// Use proxy for download
    pub use_proxy: Option<ProxyAddress>,

    /// Consensus proof for installation
    pub consensus_proof: ConsensusProof,
}

/// Installation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    /// Installed package ID
    pub package_id: String,

    /// Installation path
    pub install_path: PathBuf,

    /// Installed assets
    pub installed_assets: Vec<AssetId>,

    /// Installation time
    pub install_time: std::time::Duration,
}

/// Update result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    /// Updated package ID
    pub package_id: String,

    /// Previous version
    pub from_version: Version,

    /// New version
    pub to_version: Version,

    /// Update time
    pub update_time: std::time::Duration,
}

/// Search options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum results
    pub limit: Option<usize>,

    /// Result offset
    pub offset: Option<usize>,

    /// Sort by (relevance, downloads, rating, date)
    pub sort_by: Option<String>,

    /// Sort order (asc, desc)
    pub order: Option<String>,
}

/// Package specification for publishing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPackageSpec {
    /// Package name
    pub name: String,

    /// Package version
    pub version: Version,

    /// Package description
    pub description: String,

    /// Package contents
    pub contents: Vec<u8>,

    /// Asset manifests
    pub assets: Vec<AssetManifest>,

    /// Package metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Asset manifest in package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    /// Asset type
    pub asset_type: AssetType,

    /// Asset name
    pub name: String,

    /// Asset data
    pub data: Vec<u8>,

    /// Asset metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Publish result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    /// Published package ID
    pub package_id: String,

    /// Package version
    pub version: Version,

    /// Distribution hash
    pub distribution_hash: String,

    /// TrustChain signature
    pub signature: String,
}

/// Package verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Overall verification status
    pub verified: bool,

    /// Signature verification
    pub signature_valid: Option<bool>,

    /// Integrity check
    pub integrity_valid: bool,

    /// License compliance
    pub license_compliant: bool,

    /// Security scan results
    pub security_issues: Vec<SecurityIssue>,
}

/// Security issue found in package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    /// Severity level (critical, high, medium, low)
    pub severity: String,

    /// Issue type
    pub issue_type: String,

    /// Issue description
    pub description: String,

    /// Affected files
    pub affected_files: Vec<String>,
}

/// Extension manager for loading and managing extensions
pub struct ExtensionManager {
    /// Loaded extensions
    extensions: Arc<RwLock<HashMap<String, Box<dyn HyperMeshExtension>>>>,

    /// Extension metadata registry
    registry: Arc<RwLock<HashMap<String, ExtensionMetadata>>>,

    /// Asset handlers from extensions
    asset_handlers: Arc<RwLock<HashMap<AssetType, Box<dyn AssetExtensionHandler>>>>,

    /// Extension dependencies graph
    dependencies: Arc<RwLock<HashMap<String, HashSet<String>>>>,

    /// Extension load order
    load_order: Arc<RwLock<Vec<String>>>,

    /// Asset manager reference
    asset_manager: Arc<AssetManager>,

    /// Extension configuration
    config: ExtensionManagerConfig,
}

/// Extension manager configuration
#[derive(Debug, Clone)]
pub struct ExtensionManagerConfig {
    /// Extension directory paths
    pub extension_dirs: Vec<PathBuf>,

    /// Auto-load extensions on startup
    pub auto_load: bool,

    /// Verify extension signatures
    pub verify_signatures: bool,

    /// Maximum extensions to load
    pub max_extensions: usize,

    /// Global resource limits
    pub global_limits: ResourceLimits,

    /// Allowed capabilities for extensions
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

impl ExtensionManager {
    /// Create new extension manager
    pub fn new(asset_manager: Arc<AssetManager>, config: ExtensionManagerConfig) -> Self {
        Self {
            extensions: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(RwLock::new(HashMap::new())),
            asset_handlers: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            load_order: Arc::new(RwLock::new(Vec::new())),
            asset_manager,
            config,
        }
    }

    /// Load an extension
    pub async fn load_extension(&self, mut extension: Box<dyn HyperMeshExtension>) -> ExtensionResult<()> {
        let metadata = extension.metadata();
        let extension_id = metadata.id.clone();

        // Check if already loaded
        {
            let extensions = self.extensions.read().await;
            if extensions.contains_key(&extension_id) {
                return Err(ExtensionError::ExtensionAlreadyLoaded { id: extension_id });
            }
        }

        // Verify dependencies
        self.verify_dependencies(&metadata).await?;

        // Verify signature if required
        if self.config.verify_signatures {
            if let Some(fingerprint) = &metadata.certificate_fingerprint {
                self.verify_certificate(fingerprint).await?;
            }
        }

        // Create extension configuration
        let config = ExtensionConfig {
            settings: serde_json::Value::Null,
            resource_limits: self.config.global_limits.clone(),
            granted_capabilities: metadata.required_capabilities.intersection(&self.config.allowed_capabilities).cloned().collect(),
            privacy_level: PrivacyLevel::Private,
            debug_mode: false,
        };

        // Initialize extension
        extension.initialize(config).await?;

        // Register asset handlers
        let handlers = extension.register_assets().await?;
        {
            let mut asset_handlers = self.asset_handlers.write().await;
            for (asset_type, handler) in handlers {
                asset_handlers.insert(asset_type, handler);
            }
        }

        // Extend asset manager
        extension.extend_manager(self.asset_manager.clone()).await?;

        // Store extension
        {
            let mut extensions = self.extensions.write().await;
            extensions.insert(extension_id.clone(), extension);
        }

        // Update registry
        {
            let mut registry = self.registry.write().await;
            registry.insert(extension_id.clone(), metadata);
        }

        // Update load order
        {
            let mut load_order = self.load_order.write().await;
            load_order.push(extension_id);
        }

        Ok(())
    }

    /// Unload an extension
    pub async fn unload_extension(&self, extension_id: &str) -> ExtensionResult<()> {
        // Get and remove extension
        let mut extension = {
            let mut extensions = self.extensions.write().await;
            extensions.remove(extension_id)
                .ok_or_else(|| ExtensionError::ExtensionNotFound { id: extension_id.to_string() })?
        };

        // Shutdown extension
        extension.shutdown().await?;

        // Remove from registry
        {
            let mut registry = self.registry.write().await;
            registry.remove(extension_id);
        }

        // Remove from load order
        {
            let mut load_order = self.load_order.write().await;
            load_order.retain(|id| id != extension_id);
        }

        // Remove asset handlers
        // TODO: Track which handlers belong to which extension

        Ok(())
    }

    /// Get loaded extension
    pub async fn get_extension(&self, extension_id: &str) -> Option<Arc<dyn HyperMeshExtension>> {
        let extensions = self.extensions.read().await;
        extensions.get(extension_id).map(|e| Arc::from(e.as_ref()))
    }

    /// List loaded extensions
    pub async fn list_extensions(&self) -> Vec<ExtensionMetadata> {
        let registry = self.registry.read().await;
        registry.values().cloned().collect()
    }

    /// Get asset handler for type
    pub async fn get_asset_handler(&self, asset_type: &AssetType) -> Option<Arc<dyn AssetExtensionHandler>> {
        let handlers = self.asset_handlers.read().await;
        handlers.get(asset_type).map(|h| Arc::from(h.as_ref()))
    }

    /// Verify extension dependencies
    async fn verify_dependencies(&self, metadata: &ExtensionMetadata) -> ExtensionResult<()> {
        let registry = self.registry.read().await;

        for dep in &metadata.dependencies {
            if !dep.optional {
                let installed = registry.get(&dep.extension_id)
                    .ok_or_else(|| ExtensionError::DependencyResolutionFailed {
                        extension: metadata.id.clone(),
                        dependency: dep.extension_id.clone(),
                    })?;

                if !dep.version_requirement.matches(&installed.version) {
                    return Err(ExtensionError::VersionIncompatible {
                        extension: metadata.id.clone(),
                        required: dep.version_requirement.to_string(),
                        found: installed.version.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Verify extension certificate
    async fn verify_certificate(&self, fingerprint: &str) -> ExtensionResult<()> {
        // TODO: Implement TrustChain certificate verification
        // This would integrate with the TrustChain module to verify
        // that the extension is signed by a trusted authority

        Ok(())
    }

    /// Auto-discover and load extensions
    pub async fn auto_load_extensions(&self) -> ExtensionResult<Vec<String>> {
        if !self.config.auto_load {
            return Ok(Vec::new());
        }

        let mut loaded = Vec::new();

        for dir in &self.config.extension_dirs {
            if !dir.exists() {
                continue;
            }

            // TODO: Implement extension discovery from filesystem
            // This would scan directories for extension manifests and binaries
        }

        Ok(loaded)
    }
}

/// Integration flow for Catalog as a HyperMesh extension
///
/// This demonstrates how Catalog would integrate as an extension:
///
/// 1. **Discovery Phase**:
///    - HyperMesh scans extension directories
///    - Finds Catalog extension manifest
///    - Validates signature with TrustChain
///
/// 2. **Loading Phase**:
///    - ExtensionManager loads Catalog extension
///    - Verifies dependencies (STOQ, TrustChain)
///    - Grants required capabilities
///
/// 3. **Initialization Phase**:
///    - Catalog initializes with configuration
///    - Registers asset types (VM, Container, Library)
///    - Extends AssetManager with catalog-specific operations
///
/// 4. **Registration Phase**:
///    - Catalog registers asset handlers for each type
///    - Sets up P2P distribution through STOQ
///    - Configures consensus validation requirements
///
/// 5. **Operation Phase**:
///    - Catalog handles asset library requests
///    - Manages package installation/updates
///    - Validates operations with consensus proofs
///    - Distributes assets through P2P network
///
/// 6. **Integration Points**:
///    - **Consensus**: All operations require NKrypt four-proof validation
///    - **TrustChain**: Package signatures and certificate validation
///    - **STOQ**: P2P distribution of asset packages
///    - **Proxy/NAT**: Remote asset access through NAT-like addressing
pub struct CatalogExtensionIntegration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_metadata() {
        let metadata = ExtensionMetadata {
            id: "catalog".to_string(),
            name: "HyperMesh Catalog".to_string(),
            version: Version::parse("1.0.0").unwrap(),
            description: "Decentralized asset library for HyperMesh".to_string(),
            author: "HyperMesh Team".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://hypermesh.online/catalog".to_string()),
            category: ExtensionCategory::AssetLibrary,
            hypermesh_version: Version::parse("1.0.0").unwrap(),
            dependencies: vec![],
            required_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::NetworkAccess,
                ExtensionCapability::ConsensusAccess,
                ExtensionCapability::TransportAccess,
            ]),
            provided_assets: vec![
                AssetType::VirtualMachine,
                AssetType::Container,
                AssetType::Library,
            ],
            certificate_fingerprint: Some("SHA256:1234567890abcdef".to_string()),
            config_schema: None,
        };

        assert_eq!(metadata.id, "catalog");
        assert_eq!(metadata.category, ExtensionCategory::AssetLibrary);
        assert!(metadata.required_capabilities.contains(&ExtensionCapability::AssetManagement));
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_cpu_percent, 25.0);
        assert_eq!(limits.max_memory_bytes, 1024 * 1024 * 1024);
        assert_eq!(limits.max_concurrent_operations, 100);
    }

    #[test]
    fn test_consensus_requirements_default() {
        let reqs = ConsensusRequirements::default();
        assert!(reqs.require_proof_of_space);
        assert!(reqs.require_proof_of_stake);
        assert!(reqs.require_proof_of_work);
        assert!(reqs.require_proof_of_time);
        assert_eq!(reqs.min_space_commitment, Some(1024 * 1024));
    }
}