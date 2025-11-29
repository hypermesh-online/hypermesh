//! Core Asset Package System
//!
//! Provides the foundational asset package format parsing, validation, and management
//! for the Catalog asset library ecosystem.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use base64::Engine;

/// Asset package unique identifier
pub type AssetPackageId = Uuid;

/// Complete asset package containing all metadata and content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPackage {
    /// Package specification following catalog-asset-spec.yaml format
    pub spec: AssetSpec,
    /// Resolved content with all files loaded
    pub content: AssetContentResolved,
    /// Validation status and results
    pub validation: AssetValidationStatus,
    /// Computed package hash for integrity verification
    pub package_hash: String,
    /// Package creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
}

/// Asset specification following YAML schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSpec {
    /// API version for backward compatibility
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    /// Asset kind/type
    pub kind: String,
    /// Asset metadata
    pub metadata: AssetMetadata,
    /// Asset specification details
    pub spec: AssetSpecification,
}

/// Asset metadata section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    /// Asset name (unique within namespace)
    pub name: String,
    /// Asset version (semantic versioning)
    pub version: String,
    /// Descriptive tags for categorization
    pub tags: Vec<String>,
    /// Human-readable description
    pub description: Option<String>,
    /// Asset author information
    pub author: Option<String>,
    /// License information
    pub license: Option<String>,
    /// Homepage URL
    pub homepage: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// Keywords for search
    pub keywords: Vec<String>,
    /// Creation timestamp
    pub created: Option<DateTime<Utc>>,
    /// Last updated timestamp
    pub updated: Option<DateTime<Utc>>,
}

/// Asset specification details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSpecification {
    /// Asset type (julia-program, lua-script, wasm-module, etc.)
    #[serde(rename = "type")]
    pub asset_type: String,
    /// Asset content definitions
    pub content: AssetContent,
    /// Security requirements and configurations
    pub security: AssetSecurity,
    /// Resource constraints and requirements
    pub resources: AssetResources,
    /// Execution policy and configuration
    pub execution: AssetExecution,
    /// Dependencies on other assets
    pub dependencies: Vec<AssetDependency>,
    /// Environment variables required
    pub environment: HashMap<String, String>,
    /// Configuration schema for runtime parameters
    pub config_schema: Option<serde_json::Value>,
}

/// Asset content definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetContent {
    /// Main entry point file
    pub main: String,
    /// Additional files included in the asset
    pub files: Vec<String>,
    /// Inline content for simple assets
    pub inline: Option<String>,
    /// Binary assets (base64 encoded)
    pub binary: Vec<BinaryAsset>,
    /// Template parameters for dynamic content
    pub templates: Vec<TemplateParameter>,
}

/// Binary asset definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryAsset {
    /// File name
    pub name: String,
    /// MIME type
    pub mime_type: String,
    /// Base64 encoded content
    pub content: String,
    /// File size in bytes
    pub size: u64,
    /// SHA-256 hash for integrity
    pub hash: String,
}

/// Template parameter for dynamic content generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type (string, number, boolean, array, object)
    pub param_type: String,
    /// Default value
    pub default: Option<serde_json::Value>,
    /// Human-readable description
    pub description: Option<String>,
    /// Whether parameter is required
    pub required: bool,
    /// Validation constraints
    pub constraints: Option<ParameterConstraints>,
}

/// Parameter validation constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConstraints {
    /// Minimum value (for numbers)
    pub min: Option<f64>,
    /// Maximum value (for numbers)
    pub max: Option<f64>,
    /// Minimum length (for strings/arrays)
    pub min_length: Option<usize>,
    /// Maximum length (for strings/arrays)
    pub max_length: Option<usize>,
    /// Regular expression pattern (for strings)
    pub pattern: Option<String>,
    /// Allowed values (enum)
    pub allowed_values: Option<Vec<serde_json::Value>>,
}

/// Security requirements and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSecurity {
    /// Whether consensus proof is required for execution
    pub consensus_required: bool,
    /// Certificate pinning requirements
    pub certificate_pinning: bool,
    /// Hash validation algorithm (sha256, sha512)
    pub hash_validation: String,
    /// Sandbox security level
    pub sandbox_level: String,
    /// Allowed system calls (for strict sandboxing)
    pub allowed_syscalls: Vec<String>,
    /// Network access permissions
    pub network_access: NetworkAccess,
    /// File access permissions
    pub file_access: FileAccess,
    /// Required permissions for execution
    pub permissions: Vec<String>,
}

/// Network access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAccess {
    /// Whether network access is allowed
    pub enabled: bool,
    /// Allowed domains (if restricted)
    pub allowed_domains: Vec<String>,
    /// Allowed ports (if restricted)
    pub allowed_ports: Vec<u16>,
    /// Whether HTTPS is required
    pub require_tls: bool,
}

/// File access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccess {
    /// Access level (none, read_only, read_write)
    pub level: String,
    /// Allowed paths (if restricted)
    pub allowed_paths: Vec<String>,
    /// Denied paths (blacklist)
    pub denied_paths: Vec<String>,
    /// Whether temporary files are allowed
    pub allow_temp: bool,
}

/// Resource constraints and requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetResources {
    /// CPU limit (millicores)
    pub cpu_limit: String,
    /// Memory limit (bytes with units like "1Gi")
    pub memory_limit: String,
    /// Execution timeout
    pub execution_timeout: String,
    /// Storage space required
    pub storage_required: Option<String>,
    /// Network bandwidth required
    pub network_bandwidth: Option<String>,
    /// Whether GPU is required
    pub gpu_required: bool,
    /// Specific hardware requirements
    pub hardware_requirements: Vec<HardwareRequirement>,
}

/// Hardware requirement specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareRequirement {
    /// Hardware type (cpu, gpu, memory, storage, network)
    pub hardware_type: String,
    /// Minimum specification required
    pub minimum_spec: String,
    /// Preferred specification
    pub preferred_spec: Option<String>,
    /// Required features or capabilities
    pub required_features: Vec<String>,
}

/// Execution policy and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetExecution {
    /// Delegation strategy for remote execution
    pub delegation_strategy: String,
    /// Minimum consensus nodes required
    pub minimum_consensus: u32,
    /// Retry policy configuration
    pub retry_policy: String,
    /// Maximum concurrent executions
    pub max_concurrent: Option<u32>,
    /// Execution priority (low, normal, high, critical)
    pub priority: String,
    /// Timeout configuration
    pub timeout_config: TimeoutConfig,
    /// Scheduling preferences
    pub scheduling: SchedulingConfig,
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Overall execution timeout
    pub execution: String,
    /// Network operation timeout
    pub network: String,
    /// I/O operation timeout
    pub io: String,
    /// Compilation timeout (for compiled assets)
    pub compilation: Option<String>,
}

/// Scheduling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingConfig {
    /// Preferred execution time (immediate, scheduled, background)
    pub timing: String,
    /// Resource allocation strategy (best_fit, first_fit, balanced)
    pub allocation_strategy: String,
    /// Affinity rules for node selection
    pub node_affinity: Vec<AffinityRule>,
    /// Anti-affinity rules to avoid certain nodes
    pub anti_affinity: Vec<AffinityRule>,
}

/// Node affinity/anti-affinity rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityRule {
    /// Rule type (required, preferred)
    pub rule_type: String,
    /// Node selector key
    pub key: String,
    /// Node selector operator (in, not_in, exists, does_not_exist)
    pub operator: String,
    /// Values to match against
    pub values: Vec<String>,
    /// Rule weight (for preferred rules)
    pub weight: Option<u32>,
}

/// Asset dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDependency {
    /// Dependency name
    pub name: String,
    /// Version constraint (^1.0.0, ~1.2.0, >=1.0.0, etc.)
    pub version: String,
    /// Whether dependency is optional
    pub optional: bool,
    /// Dependency source (registry, git, local)
    pub source: DependencySource,
    /// Features to enable (if applicable)
    pub features: Vec<String>,
    /// Platform-specific dependency
    pub platform: Option<String>,
}

/// Dependency source specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DependencySource {
    /// From asset registry
    Registry {
        /// Registry URL
        registry: String,
        /// Optional namespace
        namespace: Option<String>,
    },
    /// From Git repository
    Git {
        /// Git repository URL
        url: String,
        /// Branch, tag, or commit
        reference: String,
        /// Subdirectory within repository
        path: Option<String>,
    },
    /// Local file system path
    Local {
        /// Local path to asset
        path: String,
    },
    /// HTTP/HTTPS URL
    Http {
        /// Download URL
        url: String,
        /// Expected SHA-256 hash
        sha256: String,
    },
}

/// Resolved asset content with all files loaded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetContentResolved {
    /// Main entry point content
    pub main_content: String,
    /// All file contents mapped by file path
    pub file_contents: HashMap<String, String>,
    /// Binary content mapped by file name
    pub binary_contents: HashMap<String, Vec<u8>>,
    /// Template content after parameter resolution
    pub template_content: HashMap<String, String>,
    /// Resolved dependencies
    pub resolved_dependencies: Vec<ResolvedDependency>,
}

/// Resolved dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDependency {
    /// Dependency name
    pub name: String,
    /// Resolved version
    pub version: String,
    /// Resolved source location
    pub resolved_source: String,
    /// Dependency package hash
    pub package_hash: String,
    /// Resolution timestamp
    pub resolved_at: DateTime<Utc>,
}

/// Asset validation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetValidationStatus {
    /// Whether asset passed validation
    pub is_valid: bool,
    /// Validation timestamp
    pub validated_at: DateTime<Utc>,
    /// Validation errors
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
    /// Security scan results
    pub security_results: SecurityScanResults,
    /// Dependency validation results
    pub dependency_results: DependencyValidationResults,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// File path where error occurred
    pub file: Option<String>,
    /// Line number where error occurred
    pub line: Option<u32>,
    /// Column number where error occurred
    pub column: Option<u32>,
    /// Severity level
    pub severity: ErrorSeverity,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,
    /// Warning message
    pub message: String,
    /// File path where warning occurred
    pub file: Option<String>,
    /// Line number where warning occurred
    pub line: Option<u32>,
    /// Suggestion for resolution
    pub suggestion: Option<String>,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Critical error - asset cannot be used
    Critical,
    /// Error - asset may not work correctly
    Error,
    /// Warning - asset should work but has issues
    Warning,
    /// Info - informational message
    Info,
}

/// Security scan results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResults {
    /// Overall security score (0-100)
    pub security_score: u32,
    /// Vulnerabilities found
    pub vulnerabilities: Vec<SecurityVulnerability>,
    /// Security recommendations
    pub recommendations: Vec<String>,
    /// Scan timestamp
    pub scanned_at: DateTime<Utc>,
}

/// Security vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityVulnerability {
    /// Vulnerability ID
    pub id: String,
    /// Vulnerability description
    pub description: String,
    /// Severity level
    pub severity: VulnerabilitySeverity,
    /// Affected file/component
    pub affected_component: String,
    /// Remediation suggestion
    pub remediation: Option<String>,
    /// CVE identifier if applicable
    pub cve: Option<String>,
}

/// Vulnerability severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilitySeverity {
    /// Critical vulnerability requiring immediate attention
    Critical,
    /// High severity vulnerability
    High,
    /// Medium severity vulnerability
    Medium,
    /// Low severity vulnerability
    Low,
    /// Informational finding
    Info,
}

/// Dependency validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyValidationResults {
    /// Whether all dependencies are valid
    pub dependencies_valid: bool,
    /// Total dependencies checked
    pub total_dependencies: usize,
    /// Valid dependencies count
    pub valid_dependencies: usize,
    /// Invalid dependencies
    pub invalid_dependencies: Vec<InvalidDependency>,
    /// Dependency conflicts
    pub conflicts: Vec<DependencyConflict>,
    /// Validation timestamp
    pub validated_at: DateTime<Utc>,
}

/// Invalid dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidDependency {
    /// Dependency name
    pub name: String,
    /// Requested version
    pub requested_version: String,
    /// Reason for invalidity
    pub reason: String,
    /// Suggested resolution
    pub suggestion: Option<String>,
}

/// Dependency conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflict {
    /// Primary dependency
    pub dependency_a: String,
    /// Conflicting dependency
    pub dependency_b: String,
    /// Conflict description
    pub conflict_reason: String,
    /// Possible resolution strategies
    pub resolution_strategies: Vec<String>,
}

impl AssetPackage {
    /// Create a new asset package from YAML specification
    pub async fn from_yaml<P: AsRef<Path>>(yaml_path: P) -> Result<Self> {
        let yaml_content = tokio::fs::read_to_string(yaml_path).await?;
        let spec: AssetSpec = serde_yaml::from_str(&yaml_content)?;
        
        let mut package = Self {
            spec: spec.clone(),
            content: AssetContentResolved {
                main_content: String::new(),
                file_contents: HashMap::new(),
                binary_contents: HashMap::new(),
                template_content: HashMap::new(),
                resolved_dependencies: Vec::new(),
            },
            validation: AssetValidationStatus {
                is_valid: false,
                validated_at: Utc::now(),
                errors: Vec::new(),
                warnings: Vec::new(),
                security_results: SecurityScanResults {
                    security_score: 0,
                    vulnerabilities: Vec::new(),
                    recommendations: Vec::new(),
                    scanned_at: Utc::now(),
                },
                dependency_results: DependencyValidationResults {
                    dependencies_valid: false,
                    total_dependencies: spec.spec.dependencies.len(),
                    valid_dependencies: 0,
                    invalid_dependencies: Vec::new(),
                    conflicts: Vec::new(),
                    validated_at: Utc::now(),
                },
            },
            package_hash: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Load content files
        package.load_content().await?;
        
        // Compute package hash
        package.compute_hash()?;
        
        Ok(package)
    }
    
    /// Load all content files referenced in the asset specification
    async fn load_content(&mut self) -> Result<()> {
        // Load main content
        if !self.spec.spec.content.main.is_empty() {
            match tokio::fs::read_to_string(&self.spec.spec.content.main).await {
                Ok(content) => {
                    self.content.main_content = content;
                }
                Err(e) => {
                    self.validation.errors.push(ValidationError {
                        code: "MAIN_FILE_NOT_FOUND".to_string(),
                        message: format!("Main file '{}' not found: {}", self.spec.spec.content.main, e),
                        file: Some(self.spec.spec.content.main.clone()),
                        line: None,
                        column: None,
                        severity: ErrorSeverity::Critical,
                    });
                }
            }
        }
        
        // Load additional files
        for file_path in &self.spec.spec.content.files {
            match tokio::fs::read_to_string(file_path).await {
                Ok(content) => {
                    self.content.file_contents.insert(file_path.clone(), content);
                }
                Err(e) => {
                    self.validation.errors.push(ValidationError {
                        code: "FILE_NOT_FOUND".to_string(),
                        message: format!("File '{}' not found: {}", file_path, e),
                        file: Some(file_path.clone()),
                        line: None,
                        column: None,
                        severity: ErrorSeverity::Error,
                    });
                }
            }
        }
        
        // Handle inline content
        if let Some(inline_content) = &self.spec.spec.content.inline {
            self.content.main_content = inline_content.clone();
        }
        
        // Process binary assets
        for binary_asset in &self.spec.spec.content.binary {
            match base64::engine::general_purpose::STANDARD.decode(&binary_asset.content) {
                Ok(decoded) => {
                    self.content.binary_contents.insert(binary_asset.name.clone(), decoded);
                }
                Err(e) => {
                    self.validation.errors.push(ValidationError {
                        code: "BINARY_DECODE_ERROR".to_string(),
                        message: format!("Failed to decode binary asset '{}': {}", binary_asset.name, e),
                        file: Some(binary_asset.name.clone()),
                        line: None,
                        column: None,
                        severity: ErrorSeverity::Error,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Compute package hash for integrity verification
    fn compute_hash(&mut self) -> Result<()> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        // Hash the specification
        let spec_json = serde_json::to_string(&self.spec)?;
        hasher.update(spec_json.as_bytes());
        
        // Hash all content
        hasher.update(self.content.main_content.as_bytes());
        
        for (path, content) in &self.content.file_contents {
            hasher.update(path.as_bytes());
            hasher.update(content.as_bytes());
        }
        
        for (name, content) in &self.content.binary_contents {
            hasher.update(name.as_bytes());
            hasher.update(content);
        }
        
        let result = hasher.finalize();
        self.package_hash = hex::encode(result);
        
        Ok(())
    }
    
    /// Verify package integrity against stored hash
    pub fn verify_integrity(&self) -> Result<bool> {
        let mut temp_package = self.clone();
        temp_package.compute_hash()?;
        Ok(temp_package.package_hash == self.package_hash)
    }
    
    /// Get asset package unique identifier
    pub fn get_package_id(&self) -> AssetPackageId {
        // Generate deterministic UUID from package hash
        Uuid::new_v5(&Uuid::NAMESPACE_OID, self.package_hash.as_bytes())
    }
    
    /// Check if asset package is valid for execution
    pub fn is_execution_ready(&self) -> bool {
        self.validation.is_valid && 
        self.validation.errors.iter().all(|e| !matches!(e.severity, ErrorSeverity::Critical | ErrorSeverity::Error)) &&
        self.validation.dependency_results.dependencies_valid &&
        self.validation.security_results.security_score >= 70 // Minimum security threshold
    }
    
    /// Get human-readable summary of the asset package
    pub fn get_summary(&self) -> String {
        format!(
            "{} v{}\n  Type: {}\n  Files: {} main + {} additional + {} binary\n  Dependencies: {}\n  Valid: {}\n  Security Score: {}",
            self.spec.metadata.name,
            self.spec.metadata.version,
            self.spec.spec.asset_type,
            if self.content.main_content.is_empty() { "none" } else { "present" },
            self.content.file_contents.len(),
            self.content.binary_contents.len(),
            self.spec.spec.dependencies.len(),
            if self.validation.is_valid { "yes" } else { "no" },
            self.validation.security_results.security_score
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[tokio::test]
    async fn test_asset_package_creation() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path = temp_dir.path().join("test_asset.yaml");
        
        let yaml_content = r#"
apiVersion: "catalog.v1"
kind: "Asset"
metadata:
  name: "test-asset"
  version: "1.0.0"
  tags: ["test", "example"]
  description: "Test asset for validation"
spec:
  type: "julia-program"
  content:
    main: ""
    files: []
    binary: []
    templates: []
  security:
    consensus_required: false
    certificate_pinning: false
    hash_validation: "sha256"
    sandbox_level: "standard"
    allowed_syscalls: []
    network_access:
      enabled: false
      allowed_domains: []
      allowed_ports: []
      require_tls: true
    file_access:
      level: "read_only"
      allowed_paths: []
      denied_paths: []
      allow_temp: false
    permissions: []
  resources:
    cpu_limit: "1000m"
    memory_limit: "1Gi"
    execution_timeout: "30s"
    gpu_required: false
    hardware_requirements: []
  execution:
    delegation_strategy: "nearest_node"
    minimum_consensus: 1
    retry_policy: "none"
    priority: "normal"
    timeout_config:
      execution: "30s"
      network: "10s"
      io: "5s"
    scheduling:
      timing: "immediate"
      allocation_strategy: "best_fit"
      node_affinity: []
      anti_affinity: []
  dependencies: []
  environment: {}
"#;
        
        fs::write(&yaml_path, yaml_content).unwrap();
        
        let package = AssetPackage::from_yaml(&yaml_path).await.unwrap();
        
        assert_eq!(package.spec.metadata.name, "test-asset");
        assert_eq!(package.spec.metadata.version, "1.0.0");
        assert_eq!(package.spec.spec.asset_type, "julia-program");
        assert!(!package.package_hash.is_empty());
    }
    
    #[test]
    fn test_package_hash_computation() {
        let mut package = AssetPackage {
            spec: AssetSpec {
                api_version: "catalog.v1".to_string(),
                kind: "Asset".to_string(),
                metadata: AssetMetadata {
                    name: "test".to_string(),
                    version: "1.0.0".to_string(),
                    tags: vec!["test".to_string()],
                    description: None,
                    author: None,
                    license: None,
                    homepage: None,
                    repository: None,
                    keywords: vec![],
                    created: None,
                    updated: None,
                },
                spec: AssetSpecification {
                    asset_type: "test".to_string(),
                    content: AssetContent {
                        main: "".to_string(),
                        files: vec![],
                        inline: None,
                        binary: vec![],
                        templates: vec![],
                    },
                    security: AssetSecurity {
                        consensus_required: false,
                        certificate_pinning: false,
                        hash_validation: "sha256".to_string(),
                        sandbox_level: "standard".to_string(),
                        allowed_syscalls: vec![],
                        network_access: NetworkAccess {
                            enabled: false,
                            allowed_domains: vec![],
                            allowed_ports: vec![],
                            require_tls: true,
                        },
                        file_access: FileAccess {
                            level: "read_only".to_string(),
                            allowed_paths: vec![],
                            denied_paths: vec![],
                            allow_temp: false,
                        },
                        permissions: vec![],
                    },
                    resources: AssetResources {
                        cpu_limit: "1000m".to_string(),
                        memory_limit: "1Gi".to_string(),
                        execution_timeout: "30s".to_string(),
                        storage_required: None,
                        network_bandwidth: None,
                        gpu_required: false,
                        hardware_requirements: vec![],
                    },
                    execution: AssetExecution {
                        delegation_strategy: "nearest_node".to_string(),
                        minimum_consensus: 1,
                        retry_policy: "none".to_string(),
                        max_concurrent: None,
                        priority: "normal".to_string(),
                        timeout_config: TimeoutConfig {
                            execution: "30s".to_string(),
                            network: "10s".to_string(),
                            io: "5s".to_string(),
                            compilation: None,
                        },
                        scheduling: SchedulingConfig {
                            timing: "immediate".to_string(),
                            allocation_strategy: "best_fit".to_string(),
                            node_affinity: vec![],
                            anti_affinity: vec![],
                        },
                    },
                    dependencies: vec![],
                    environment: HashMap::new(),
                    config_schema: None,
                },
            },
            content: AssetContentResolved {
                main_content: "test content".to_string(),
                file_contents: HashMap::new(),
                binary_contents: HashMap::new(),
                template_content: HashMap::new(),
                resolved_dependencies: vec![],
            },
            validation: AssetValidationStatus {
                is_valid: true,
                validated_at: Utc::now(),
                errors: vec![],
                warnings: vec![],
                security_results: SecurityScanResults {
                    security_score: 85,
                    vulnerabilities: vec![],
                    recommendations: vec![],
                    scanned_at: Utc::now(),
                },
                dependency_results: DependencyValidationResults {
                    dependencies_valid: true,
                    total_dependencies: 0,
                    valid_dependencies: 0,
                    invalid_dependencies: vec![],
                    conflicts: vec![],
                    validated_at: Utc::now(),
                },
            },
            package_hash: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        package.compute_hash().unwrap();
        assert!(!package.package_hash.is_empty());
        assert!(package.verify_integrity().unwrap());
    }
}