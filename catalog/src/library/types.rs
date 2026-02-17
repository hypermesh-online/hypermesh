// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Core types for the asset library
//!
//! Lightweight types optimized for in-memory operations and HyperMesh integration.
//! These types are designed to be zero-copy where possible and minimize allocations.
//!
//! MIGRATION: This module now wraps the Asset Registry architecture for backward compatibility.

use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::sync::Arc;

// Import Asset Registry types
use crate::registry::AssetTypeDefinition;

// Serde support for Arc<str>
mod arc_str_serde {
    use super::*;
    use serde::de::{Deserializer, Visitor};
    use std::fmt;

    pub fn serialize<S>(value: &Arc<str>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<str>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArcStrVisitor;
        impl<'de> Visitor<'de> for ArcStrVisitor {
            type Value = Arc<str>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Arc::from(v))
            }
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Arc::from(v))
            }
        }
        deserializer.deserialize_string(ArcStrVisitor)
    }
}

// Serde support for Option<Arc<str>>
mod option_arc_str_serde {
    use super::*;
    use serde::de::{Deserializer, Visitor};
    use std::fmt;

    pub fn serialize<S>(value: &Option<Arc<str>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(s) => serializer.serialize_some(s.as_ref()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Arc<str>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OptArcStrVisitor;
        impl<'de> Visitor<'de> for OptArcStrVisitor {
            type Value = Option<Arc<str>>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an optional string")
            }
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(None)
            }
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                super::arc_str_serde::deserialize(deserializer).map(Some)
            }
        }
        deserializer.deserialize_option(OptArcStrVisitor)
    }
}

// Serde support for Arc<[Arc<str>]>
mod arc_slice_arc_str_serde {
    use super::*;
    use serde::de::{Deserializer, SeqAccess, Visitor};
    use std::fmt;

    pub fn serialize<S>(value: &Arc<[Arc<str>]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(value.len()))?;
        for item in value.iter() {
            seq.serialize_element(item.as_ref())?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<[Arc<str>]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArcSliceVisitor;
        impl<'de> Visitor<'de> for ArcSliceVisitor {
            type Value = Arc<[Arc<str>]>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence of strings")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = seq.next_element::<String>()? {
                    vec.push(Arc::from(elem.as_str()));
                }
                Ok(Arc::from(vec.into_boxed_slice()))
            }
        }
        deserializer.deserialize_seq(ArcSliceVisitor)
    }
}

// Generic serde for Arc<[T]> where T: Serialize + Deserialize
macro_rules! arc_slice_serde {
    ($name:ident, $ty:ty) => {
        mod $name {
            use super::*;
            use serde::de::{Deserializer, SeqAccess, Visitor};
            use std::fmt;

            pub fn serialize<S>(value: &Arc<[$ty]>, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                use serde::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(value.len()))?;
                for item in value.iter() {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<[$ty]>, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct ArcSliceVisitor;
                impl<'de> Visitor<'de> for ArcSliceVisitor {
                    type Value = Arc<[$ty]>;
                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a sequence")
                    }
                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: SeqAccess<'de>,
                    {
                        let mut vec = Vec::new();
                        while let Some(elem) = seq.next_element::<$ty>()? {
                            vec.push(elem);
                        }
                        Ok(Arc::from(vec.into_boxed_slice()))
                    }
                }
                deserializer.deserialize_seq(ArcSliceVisitor)
            }
        }
    };
}

arc_slice_serde!(arc_slice_dependency_serde, PackageDependency);
arc_slice_serde!(arc_slice_contentref_serde, ContentRef);
arc_slice_serde!(arc_slice_binaryref_serde, BinaryRef);
arc_slice_serde!(arc_slice_templateparam_serde, TemplateParameter);

// Serde support for Arc<HashMap<Arc<str>, Arc<str>>>
mod arc_hashmap_arc_str_serde {
    use super::*;
    use serde::de::{Deserializer, MapAccess, Visitor};
    use std::fmt;

    pub fn serialize<S>(value: &Arc<HashMap<Arc<str>, Arc<str>>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (k, v) in value.iter() {
            map.serialize_entry(k.as_ref(), v.as_ref())?;
        }
        map.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<HashMap<Arc<str>, Arc<str>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArcHashMapVisitor;
        impl<'de> Visitor<'de> for ArcHashMapVisitor {
            type Value = Arc<HashMap<Arc<str>, Arc<str>>>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of strings to strings")
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut hashmap = HashMap::new();
                while let Some((key, value)) = map.next_entry::<String, String>()? {
                    hashmap.insert(Arc::from(key.as_str()), Arc::from(value.as_str()));
                }
                Ok(Arc::new(hashmap))
            }
        }
        deserializer.deserialize_map(ArcHashMapVisitor)
    }
}

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
    #[serde(with = "arc_str_serde")]
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

impl LibraryAssetPackage {
    /// Get package version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get package metadata (returns own metadata fields)
    pub fn metadata(&self) -> PackageMetadataView {
        PackageMetadataView {
            name: &self.name,
            version: &self.version,
            description: self.description.as_deref(),
            asset_type: &self.asset_type,
            size: self.size,
            hash: &self.hash,
        }
    }

    /// Get package ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get package dependencies
    pub fn dependencies(&self) -> &[PackageDependency] {
        self.spec
            .as_ref()
            .map(|s| s.dependencies.as_ref())
            .unwrap_or(&[])
    }

    /// Get package author
    pub fn author(&self) -> Option<&str> {
        self.metadata.as_ref()
            .and_then(|m| m.author.as_ref().map(|a| a.as_ref()))
    }

    /// Get package license
    pub fn license(&self) -> Option<&str> {
        self.metadata.as_ref()
            .and_then(|m| m.license.as_ref().map(|l| l.as_ref()))
    }

    /// Get package size (compatibility accessor)
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get package tags
    pub fn tags(&self) -> Vec<String> {
        self.metadata
            .as_ref()
            .map(|m| m.tags.iter().map(|t| t.to_string()).collect())
            .unwrap_or_default()
    }

    /// Get runtime type
    pub fn runtime_type(&self) -> Option<&str> {
        self.spec.as_ref().map(|s| s.runtime.runtime_type.as_str())
    }

    /// Get content reference
    pub fn content_ref(&self) -> &str {
        &self.content
    }

    /// Check if package has metadata
    pub fn has_metadata(&self) -> bool {
        self.metadata.is_some()
    }
}

/// View into package metadata for borrowing
#[derive(Debug, Clone, Copy)]
pub struct PackageMetadataView<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub description: Option<&'a str>,
    pub asset_type: &'a str,
    pub size: u64,
    pub hash: &'a str,
}

/// Package metadata optimized for fast access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package name
    #[serde(with = "arc_str_serde")]
    pub name: Arc<str>,
    /// Semantic version
    #[serde(with = "arc_str_serde")]
    pub version: Arc<str>,
    /// Package description
    #[serde(with = "option_arc_str_serde")]
    pub description: Option<Arc<str>>,
    /// Author information
    #[serde(with = "option_arc_str_serde")]
    pub author: Option<Arc<str>>,
    /// License identifier
    #[serde(with = "option_arc_str_serde")]
    pub license: Option<Arc<str>>,
    /// Tags for categorization
    #[serde(with = "arc_slice_arc_str_serde")]
    pub tags: Arc<[Arc<str>]>,
    /// Keywords for search
    #[serde(with = "arc_slice_arc_str_serde")]
    pub keywords: Arc<[Arc<str>]>,
    /// Creation timestamp
    pub created: i64,
    /// Last modified timestamp
    pub modified: i64,
}

/// Package specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpec {
    /// Runtime requirements (language, version, dependencies)
    pub runtime: RuntimeRequirements,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Security configuration
    pub security: SecurityConfig,
    /// Execution configuration
    pub execution: ExecutionConfig,
    /// Dependencies
    #[serde(with = "arc_slice_dependency_serde")]
    pub dependencies: Arc<[PackageDependency]>,
    /// Environment variables
    #[serde(with = "arc_hashmap_arc_str_serde")]
    pub environment: Arc<HashMap<Arc<str>, Arc<str>>>,
}

/// Runtime requirements for asset execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRequirements {
    /// Runtime type (python, lua, wasm, native, etc.)
    pub runtime_type: String,
    /// Runtime version (e.g. "3.11", "5.4", "1.0")
    pub version: String,
    /// Runtime-specific dependencies
    pub dependencies: Vec<String>,
}

// AssetType removed - use blockmatrix::assets::core::AssetCategory instead
// For applications: AssetCategory::Application(ApplicationDomain)
// Runtime requirements go in RuntimeRequirements metadata

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
    #[serde(with = "arc_slice_arc_str_serde")]
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
    #[serde(with = "arc_str_serde")]
    pub name: Arc<str>,
    /// Version constraint
    #[serde(with = "arc_str_serde")]
    pub version_constraint: Arc<str>,
    /// Optional dependency
    pub optional: bool,
    /// Platform-specific
    #[serde(with = "option_arc_str_serde")]
    pub platform: Option<Arc<str>>,
}

/// Content references for lazy loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentReferences {
    /// Main entry point reference
    pub main_ref: ContentRef,
    /// Additional file references
    #[serde(with = "arc_slice_contentref_serde")]
    pub file_refs: Arc<[ContentRef]>,
    /// Binary content references
    #[serde(with = "arc_slice_binaryref_serde")]
    pub binary_refs: Arc<[BinaryRef]>,
    /// Total content size in bytes
    pub total_size: u64,
}

/// Reference to content that can be loaded on demand
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRef {
    /// File path or identifier
    #[serde(with = "arc_str_serde")]
    pub path: Arc<str>,
    /// Content hash for verification
    #[serde(with = "arc_str_serde")]
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
    #[serde(with = "arc_str_serde")]
    pub id: Arc<str>,
    /// MIME type
    #[serde(with = "arc_str_serde")]
    pub mime_type: Arc<str>,
    /// Content hash
    #[serde(with = "arc_str_serde")]
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
    #[serde(with = "arc_str_serde")]
    pub id: Arc<str>,
    /// Template name
    #[serde(with = "arc_str_serde")]
    pub name: Arc<str>,
    /// Template description
    #[serde(with = "option_arc_str_serde")]
    pub description: Option<Arc<str>>,
    /// Template runtime requirements
    pub runtime: RuntimeRequirements,
    /// Template parameters
    #[serde(with = "arc_slice_templateparam_serde")]
    pub parameters: Arc<[TemplateParameter]>,
    /// Template files
    #[serde(with = "arc_hashmap_arc_str_serde")]
    pub files: Arc<HashMap<Arc<str>, Arc<str>>>,
}

/// Template parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name
    #[serde(with = "arc_str_serde")]
    pub name: Arc<str>,
    /// Parameter type
    pub param_type: ParameterType,
    /// Default value
    #[serde(with = "option_arc_str_serde")]
    pub default: Option<Arc<str>>,
    /// Parameter description
    #[serde(with = "option_arc_str_serde")]
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

// Conversion implementations for Asset Registry integration
impl LibraryAssetPackage {
    /// Convert to AssetTypeDefinition (new registry architecture)
    pub fn to_asset_type_definition(&self) -> Result<AssetTypeDefinition, anyhow::Error> {
        use blockmatrix::assets::ConsensusProof;
        use blockmatrix::consensus::proof_of_state_integration::{
            SpaceProof, StakeProof, WorkProof, TimeProof,
            WorkloadType, WorkState,
        };
        use std::time::Duration;
        use serde_json::json;

        // Create consensus proof from package validation data
        let stake_proof = StakeProof::new(
            self.author().unwrap_or("unknown").to_string(),
            self.id.to_string(),
            1000, // Default stake amount
        );

        let space_proof = SpaceProof::new(
            "catalog-node".to_string(),
            format!("/catalog/{}", self.id),
            self.size,
        );

        let work_proof = WorkProof::new(
            self.author().unwrap_or("unknown").to_string(),
            format!("package-{}", self.id),
            chrono::Utc::now().timestamp() as u64,
            100, // Default resource units
            WorkloadType::Compute,
            WorkState::Completed,
        );

        let time_proof = TimeProof::new(Duration::from_secs(
            self.metadata.as_ref()
                .map(|m| (m.modified - m.created) as u64)
                .unwrap_or(0)
        ));

        let consensus_proof = ConsensusProof::new(
            stake_proof,
            time_proof,
            space_proof,
            work_proof,
        );

        // Build JSON schema from package specification
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "version": { "type": "string" },
                "asset_type": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["name", "version", "asset_type"]
        });

        let mut type_def = AssetTypeDefinition::new(
            self.name.clone(),
            schema,
            consensus_proof,
        );

        // Populate metadata
        type_def.metadata.version = self.version.clone();
        type_def.metadata.author = self.author().map(|s| s.to_string());
        type_def.metadata.description = self.description.clone();
        type_def.metadata.tags = self.tags();
        type_def.metadata.license = self.license().map(|s| s.to_string());

        if let Some(metadata) = &self.metadata {
            type_def.metadata.created_at = chrono::DateTime::from_timestamp(metadata.created, 0)
                .unwrap_or_else(chrono::Utc::now);
            type_def.metadata.updated_at = chrono::DateTime::from_timestamp(metadata.modified, 0)
                .unwrap_or_else(chrono::Utc::now);
        }

        // Add dependencies
        for dep in self.dependencies() {
            type_def.add_dependency(dep.name.to_string());
        }

        Ok(type_def)
    }

    /// Create LibraryAssetPackage from AssetTypeDefinition (new registry architecture)
    pub fn from_asset_type_definition(type_def: &AssetTypeDefinition) -> Self {
        Self {
            id: Arc::from(type_def.asset_id.to_string().as_str()),
            name: type_def.type_name.clone(),
            version: type_def.metadata.version.clone(),
            description: type_def.metadata.description.clone(),
            asset_type: "library".to_string(),
            size: 0, // Will be calculated from content
            hash: type_def.asset_id.to_string(),
            content: type_def.schema.to_string(),
            metadata: Some(PackageMetadata {
                name: Arc::from(type_def.type_name.as_str()),
                version: Arc::from(type_def.metadata.version.as_str()),
                description: type_def.metadata.description.as_ref().map(|d| Arc::from(d.as_str())),
                author: type_def.metadata.author.as_ref().map(|a| Arc::from(a.as_str())),
                license: type_def.metadata.license.as_ref().map(|l| Arc::from(l.as_str())),
                tags: type_def.metadata.tags.iter().map(|t| Arc::from(t.as_str())).collect(),
                keywords: Arc::new([]), // Not available in AssetTypeDefinition
                created: type_def.metadata.created_at.timestamp(),
                modified: type_def.metadata.updated_at.timestamp(),
            }),
            spec: None, // Could be populated from validation rules if needed
            content_refs: None,
            validation: None, // Validation will be performed on-demand
        }
    }

    /// Convert to Catalog AssetPackage for operations requiring full package structure
    pub fn to_asset_package(&self) -> crate::assets::AssetPackage {
        use crate::assets::*;
        use chrono::Utc;

        AssetPackage {
            spec: AssetSpec {
                api_version: "v1".to_string(),
                kind: self.asset_type.clone(),
                metadata: AssetMetadata {
                    name: self.name.clone(),
                    version: self.version.clone(),
                    tags: self.tags(),
                    description: self.description.clone(),
                    author: self.author().map(|s| s.to_string()),
                    license: self.license().map(|s| s.to_string()),
                    homepage: None,
                    repository: None,
                    keywords: vec![],
                    created: None,
                    updated: None,
                },
                spec: AssetSpecification {
                    asset_type: self.asset_type.clone(),
                    content: AssetContent {
                        main: self.content.clone(),
                        files: vec![],
                        inline: None,
                        binary: vec![],
                        templates: vec![],
                    },
                    security: AssetSecurity {
                        consensus_required: self.spec.as_ref().map(|s| s.security.consensus_required).unwrap_or(false),
                        certificate_pinning: false,
                        hash_validation: "sha256".to_string(),
                        sandbox_level: self.spec.as_ref().map(|s| match s.security.sandbox_level {
                            SandboxLevel::None => "none".to_string(),
                            SandboxLevel::Standard => "standard".to_string(),
                            SandboxLevel::Strict => "strict".to_string(),
                        }).unwrap_or_else(|| "standard".to_string()),
                        allowed_syscalls: vec![],
                        network_access: NetworkAccess {
                            enabled: self.spec.as_ref().map(|s| s.security.network_access).unwrap_or(false),
                            allowed_domains: vec![],
                            allowed_ports: vec![],
                            require_tls: true,
                        },
                        file_access: FileAccess {
                            level: self.spec.as_ref().map(|s| match s.security.filesystem_access {
                                FilesystemAccess::None => "none".to_string(),
                                FilesystemAccess::ReadOnly => "read_only".to_string(),
                                FilesystemAccess::ReadWrite => "read_write".to_string(),
                            }).unwrap_or_else(|| "none".to_string()),
                            allowed_paths: vec![],
                            denied_paths: vec![],
                            allow_temp: false,
                        },
                        permissions: vec![],
                    },
                    resources: AssetResources {
                        cpu_limit: self.spec.as_ref()
                            .map(|s| format!("{}m", s.resources.cpu_millicores))
                            .unwrap_or_else(|| "100m".to_string()),
                        memory_limit: self.spec.as_ref()
                            .map(|s| format!("{}Mi", s.resources.memory_mb))
                            .unwrap_or_else(|| "128Mi".to_string()),
                        execution_timeout: self.spec.as_ref()
                            .map(|s| format!("{}s", s.resources.timeout_seconds))
                            .unwrap_or_else(|| "30s".to_string()),
                        storage_required: self.spec.as_ref()
                            .and_then(|s| s.resources.storage_mb.map(|mb| format!("{}Mi", mb))),
                        network_bandwidth: self.spec.as_ref()
                            .and_then(|s| s.resources.network_mbps.map(|mbps| format!("{}Mbps", mbps))),
                        gpu_required: self.spec.as_ref().map(|s| s.resources.gpu_required).unwrap_or(false),
                        hardware_requirements: vec![],
                    },
                    execution: AssetExecution {
                        delegation_strategy: self.spec.as_ref().map(|s| match s.execution.strategy {
                            ExecutionStrategy::NearestNode => "nearest".to_string(),
                            ExecutionStrategy::RandomNode => "random".to_string(),
                            ExecutionStrategy::SpecificNode => "specific".to_string(),
                            ExecutionStrategy::LoadBalanced => "loadbalanced".to_string(),
                        }).unwrap_or_else(|| "loadbalanced".to_string()),
                        minimum_consensus: self.spec.as_ref().map(|s| s.execution.min_consensus).unwrap_or(1),
                        retry_policy: self.spec.as_ref().map(|s| {
                            if s.execution.retry_policy.exponential_backoff {
                                format!("exponential:{}:{}", s.execution.retry_policy.max_attempts, s.execution.retry_policy.base_delay_ms)
                            } else {
                                format!("fixed:{}:{}", s.execution.retry_policy.max_attempts, s.execution.retry_policy.base_delay_ms)
                            }
                        }).unwrap_or_else(|| "exponential:3:1000".to_string()),
                        max_concurrent: self.spec.as_ref().and_then(|s| s.execution.max_concurrent),
                        priority: self.spec.as_ref().map(|s| match s.execution.priority {
                            ExecutionPriority::Low => "low".to_string(),
                            ExecutionPriority::Normal => "normal".to_string(),
                            ExecutionPriority::High => "high".to_string(),
                            ExecutionPriority::Critical => "critical".to_string(),
                        }).unwrap_or_else(|| "normal".to_string()),
                        timeout_config: TimeoutConfig {
                            execution: self.spec.as_ref().map(|s| format!("{}s", s.resources.timeout_seconds)).unwrap_or_else(|| "30s".to_string()),
                            network: "10s".to_string(),
                            io: "5s".to_string(),
                            compilation: None,
                        },
                        scheduling: SchedulingConfig {
                            timing: "immediate".to_string(),
                            allocation_strategy: "balanced".to_string(),
                            node_affinity: vec![],
                            anti_affinity: vec![],
                        },
                    },
                    dependencies: self.dependencies().iter().map(|d| AssetDependency {
                        name: d.name.to_string(),
                        version: d.version_constraint.to_string(),
                        optional: d.optional,
                        source: DependencySource::Registry {
                            registry: "default".to_string(),
                            namespace: None,
                        },
                        features: vec![],
                        platform: d.platform.as_ref().map(|p| p.to_string()),
                    }).collect(),
                    environment: self.spec.as_ref()
                        .map(|s| s.environment.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
                        .unwrap_or_default(),
                    config_schema: None,
                },
            },
            content: AssetContentResolved {
                main_content: self.content.clone(),
                file_contents: std::collections::HashMap::new(),
                binary_contents: std::collections::HashMap::new(),
                template_content: std::collections::HashMap::new(),
                resolved_dependencies: vec![],
            },
            validation: AssetValidationStatus {
                is_valid: self.validation.as_ref().map(|v| v.valid).unwrap_or(false),
                validated_at: Utc::now(),
                errors: vec![],
                warnings: vec![],
                security_results: SecurityScanResults {
                    security_score: self.validation.as_ref().map(|v| v.security_score).unwrap_or(0),
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
            package_hash: self.hash.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            signature: None,
        }
    }

    /// Create LibraryAssetPackage from Catalog AssetPackage
    pub fn from_asset_package(package: &crate::assets::AssetPackage) -> Self {
        Self {
            id: Arc::from(package.package_hash.as_str()),
            name: package.spec.metadata.name.clone(),
            version: package.spec.metadata.version.clone(),
            description: package.spec.metadata.description.clone(),
            asset_type: package.spec.spec.asset_type.clone(),
            size: package.content.main_content.len() as u64,
            hash: package.package_hash.clone(),
            content: package.content.main_content.clone(),
            metadata: Some(PackageMetadata {
                name: Arc::from(package.spec.metadata.name.as_str()),
                version: Arc::from(package.spec.metadata.version.as_str()),
                description: package.spec.metadata.description.as_ref().map(|d| Arc::from(d.as_str())),
                author: package.spec.metadata.author.as_ref().map(|a| Arc::from(a.as_str())),
                license: package.spec.metadata.license.as_ref().map(|l| Arc::from(l.as_str())),
                tags: package.spec.metadata.tags.iter().map(|t| Arc::from(t.as_str())).collect(),
                keywords: package.spec.metadata.keywords.iter().map(|k| Arc::from(k.as_str())).collect(),
                created: package.spec.metadata.created.map(|t| t.timestamp()).unwrap_or(0),
                modified: package.spec.metadata.updated.map(|t| t.timestamp()).unwrap_or(0),
            }),
            spec: None, // Can be populated if needed
            content_refs: None,
            validation: if package.validation.is_valid {
                Some(ValidationStatus {
                    valid: package.validation.is_valid,
                    security_score: package.validation.security_results.security_score,
                    validated_at: package.validation.validated_at.timestamp(),
                    expires_at: package.validation.validated_at.timestamp() + 86400,
                })
            } else {
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_requirements() {
        let runtime = RuntimeRequirements {
            runtime_type: "lua".to_string(),
            version: "5.4".to_string(),
            dependencies: vec!["luasocket".to_string()],
        };
        assert_eq!(runtime.runtime_type, "lua");
        assert_eq!(runtime.version, "5.4");
        assert_eq!(runtime.dependencies.len(), 1);
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