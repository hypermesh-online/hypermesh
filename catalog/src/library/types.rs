//! Core types for the asset library
//!
//! Lightweight types optimized for in-memory operations and HyperMesh integration.
//! These types are designed to be zero-copy where possible and minimize allocations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Lightweight asset package for library operations
///
/// This is a streamlined version of AssetPackage optimized for:
/// - Zero-copy operations where possible
/// - Minimal memory footprint
/// - Fast serialization/deserialization
/// - HyperMesh native type compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryAssetPackage {
    /// Unique package identifier
    pub id: Arc<str>,
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description
    pub description: Option<String>,
    /// Asset type
    pub asset_type: String,
    /// Package size in bytes
    pub size: u64,
    /// Package hash for integrity
    pub hash: String,
    /// Package content (for simple compatibility)
    pub content: String,
    /// Package metadata (optional, full structure)
    pub metadata: Option<PackageMetadata>,
    /// Package specification (optional, full structure)
    pub spec: Option<PackageSpec>,
    /// Content references (optional, not loaded by default)
    pub content_refs: Option<ContentReferences>,
    /// Validation status
    pub validation: Option<ValidationStatus>,
}

/// Package metadata optimized for fast access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package name
    pub name: Arc<str>,
    /// Semantic version
    pub version: Arc<str>,
    /// Package description
    pub description: Option<Arc<str>>,
    /// Author information
    pub author: Option<Arc<str>>,
    /// License identifier
    pub license: Option<Arc<str>>,
    /// Tags for categorization
    pub tags: Arc<[Arc<str>]>,
    /// Keywords for search
    pub keywords: Arc<[Arc<str>]>,
    /// Creation timestamp
    pub created: i64,
    /// Last modified timestamp
    pub modified: i64,
}

/// Package specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpec {
    /// Asset type (julia, lua, wasm, etc.)
    pub asset_type: AssetType,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Security configuration
    pub security: SecurityConfig,
    /// Execution configuration
    pub execution: ExecutionConfig,
    /// Dependencies
    pub dependencies: Arc<[PackageDependency]>,
    /// Environment variables
    pub environment: Arc<HashMap<Arc<str>, Arc<str>>>,
}

/// Asset types supported by the library
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetType {
    /// Julia program
    JuliaProgram,
    /// Lua script
    LuaScript,
    /// WebAssembly module
    WasmModule,
    /// Container application
    Container,
    /// Machine learning model
    MLModel,
    /// Data processing pipeline
    DataPipeline,
    /// Generic binary
    Binary,
    /// Custom asset type
    Custom,
}

impl AssetType {
    /// Get string representation
    pub fn as_str(&self) -> &str {
        match self {
            AssetType::JuliaProgram => "julia",
            AssetType::LuaScript => "lua",
            AssetType::WasmModule => "wasm",
            AssetType::Container => "container",
            AssetType::MLModel => "ml_model",
            AssetType::DataPipeline => "data_pipeline",
            AssetType::Binary => "binary",
            AssetType::Custom => "custom",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "julia" | "julia-program" => Some(AssetType::JuliaProgram),
            "lua" | "lua-script" => Some(AssetType::LuaScript),
            "wasm" | "wasm-module" => Some(AssetType::WasmModule),
            "container" => Some(AssetType::Container),
            "ml_model" | "ml-model" => Some(AssetType::MLModel),
            "data_pipeline" | "data-pipeline" => Some(AssetType::DataPipeline),
            "binary" => Some(AssetType::Binary),
            "custom" => Some(AssetType::Custom),
            _ => None,
        }
    }
}

/// Resource requirements for asset execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU requirement in millicores
    pub cpu_millicores: u32,
    /// Memory requirement in MB
    pub memory_mb: u32,
    /// Storage requirement in MB
    pub storage_mb: Option<u32>,
    /// GPU requirement
    pub gpu_required: bool,
    /// Network bandwidth in Mbps
    pub network_mbps: Option<u32>,
    /// Execution timeout in seconds
    pub timeout_seconds: u32,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_millicores: 100,     // 0.1 CPU
            memory_mb: 128,          // 128 MB
            storage_mb: None,
            gpu_required: false,
            network_mbps: None,
            timeout_seconds: 30,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Consensus validation required
    pub consensus_required: bool,
    /// Sandbox level (none, standard, strict)
    pub sandbox_level: SandboxLevel,
    /// Network access allowed
    pub network_access: bool,
    /// File system access level
    pub filesystem_access: FilesystemAccess,
    /// Required permissions
    pub permissions: Arc<[Arc<str>]>,
}

/// Sandbox security levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxLevel {
    /// No sandboxing
    None,
    /// Standard sandboxing
    Standard,
    /// Strict sandboxing with minimal permissions
    Strict,
}

/// Filesystem access levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FilesystemAccess {
    /// No filesystem access
    None,
    /// Read-only access
    ReadOnly,
    /// Full read-write access
    ReadWrite,
}

/// Execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Execution strategy
    pub strategy: ExecutionStrategy,
    /// Minimum consensus nodes
    pub min_consensus: u32,
    /// Maximum concurrent executions
    pub max_concurrent: Option<u32>,
    /// Execution priority
    pub priority: ExecutionPriority,
    /// Retry policy
    pub retry_policy: RetryPolicy,
}

/// Execution strategies
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStrategy {
    /// Execute on nearest available node
    NearestNode,
    /// Execute on random node
    RandomNode,
    /// Execute on specific node type
    SpecificNode,
    /// Load-balanced execution
    LoadBalanced,
}

/// Execution priority levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Retry policies for failed executions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Base delay between retries (milliseconds)
    pub base_delay_ms: u32,
    /// Use exponential backoff
    pub exponential_backoff: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1000,
            exponential_backoff: true,
        }
    }
}

/// Package dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDependency {
    /// Dependency name
    pub name: Arc<str>,
    /// Version constraint
    pub version_constraint: Arc<str>,
    /// Optional dependency
    pub optional: bool,
    /// Platform-specific
    pub platform: Option<Arc<str>>,
}

/// Content references for lazy loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentReferences {
    /// Main entry point reference
    pub main_ref: ContentRef,
    /// Additional file references
    pub file_refs: Arc<[ContentRef]>,
    /// Binary content references
    pub binary_refs: Arc<[BinaryRef]>,
    /// Total content size in bytes
    pub total_size: u64,
}

/// Reference to content that can be loaded on demand
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRef {
    /// File path or identifier
    pub path: Arc<str>,
    /// Content hash for verification
    pub hash: Arc<str>,
    /// Content size in bytes
    pub size: u64,
    /// Content type
    pub content_type: ContentType,
}

/// Binary content reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryRef {
    /// Binary identifier
    pub id: Arc<str>,
    /// MIME type
    pub mime_type: Arc<str>,
    /// Content hash
    pub hash: Arc<str>,
    /// Size in bytes
    pub size: u64,
}

/// Content types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentType {
    /// Source code
    Source,
    /// Configuration
    Config,
    /// Documentation
    Documentation,
    /// Binary data
    Binary,
    /// Template
    Template,
}

/// Validation status for cached validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStatus {
    /// Validation passed
    pub valid: bool,
    /// Security score (0-100)
    pub security_score: u32,
    /// Validation timestamp
    pub validated_at: i64,
    /// Validation expiry timestamp
    pub expires_at: i64,
}

/// Template for package generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageTemplate {
    /// Template identifier
    pub id: Arc<str>,
    /// Template name
    pub name: Arc<str>,
    /// Template description
    pub description: Option<Arc<str>>,
    /// Template type
    pub template_type: AssetType,
    /// Template parameters
    pub parameters: Arc<[TemplateParameter]>,
    /// Template files
    pub files: Arc<HashMap<Arc<str>, Arc<str>>>,
}

/// Template parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name
    pub name: Arc<str>,
    /// Parameter type
    pub param_type: ParameterType,
    /// Default value
    pub default: Option<Arc<str>>,
    /// Parameter description
    pub description: Option<Arc<str>>,
    /// Required parameter
    pub required: bool,
}

/// Parameter types for templates
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

// Performance optimization: Pre-allocated common strings
lazy_static::lazy_static! {
    pub static ref EMPTY_STR: Arc<str> = Arc::from("");
    pub static ref DEFAULT_VERSION: Arc<str> = Arc::from("1.0.0");
    pub static ref DEFAULT_LICENSE: Arc<str> = Arc::from("MIT");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_type_conversion() {
        assert_eq!(AssetType::JuliaProgram.as_str(), "julia");
        assert_eq!(AssetType::from_str("julia"), Some(AssetType::JuliaProgram));
        assert_eq!(AssetType::from_str("julia-program"), Some(AssetType::JuliaProgram));
        assert_eq!(AssetType::from_str("unknown"), None);
    }

    #[test]
    fn test_resource_requirements_default() {
        let resources = ResourceRequirements::default();
        assert_eq!(resources.cpu_millicores, 100);
        assert_eq!(resources.memory_mb, 128);
        assert!(!resources.gpu_required);
        assert_eq!(resources.timeout_seconds, 30);
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay_ms, 1000);
        assert!(policy.exponential_backoff);
    }
}