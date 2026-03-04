// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset package types - core data structures for asset specifications

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::registry::AssetValidationStatus;

/// Asset package unique identifier
pub type AssetPackageId = Uuid;

/// Complete asset package containing all metadata and content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPackage {
    pub spec: AssetSpec,
    pub content: AssetContentResolved,
    pub validation: AssetValidationStatus,
    pub package_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<serde_json::Value>,
}

impl AssetPackage {
    /// Get package size in bytes (estimated)
    pub fn size(&self) -> u64 {
        self.content.main_content.len() as u64
            + self
                .content
                .file_contents
                .values()
                .map(|c| c.len())
                .sum::<usize>() as u64
            + self
                .content
                .binary_contents
                .values()
                .map(|c| c.len())
                .sum::<usize>() as u64
    }
}

/// Asset specification following YAML schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSpec {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: PackageSpecMetadata,
    pub spec: AssetSpecification,
}

/// Asset metadata section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpecMetadata {
    pub name: String,
    pub version: String,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    /// Total download/reference count for this type definition
    #[serde(default)]
    pub download_count: u64,
    /// Featured type definition flag (curated or stake-weighted)
    #[serde(default)]
    pub featured: bool,
    pub keywords: Vec<String>,
    pub created: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
}

/// Asset specification details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSpecification {
    #[serde(rename = "type")]
    pub asset_type: String,
    pub content: AssetContent,
    pub security: AssetSecurity,
    pub resources: AssetResources,
    pub execution: AssetExecution,
    pub dependencies: Vec<AssetDependency>,
    pub environment: HashMap<String, String>,
    pub config_schema: Option<serde_json::Value>,
}

impl AssetSpecification {
    /// Get resource requirements (compatibility method for BlockMatrix integration)
    pub fn requirements(&self) -> ResourceRequirements {
        ResourceRequirements {
            cpu_limit: self.resources.cpu_limit.clone(),
            memory_limit: self.resources.memory_limit.clone(),
            storage: self.resources.storage_required.clone(),
            gpu_required: self.resources.gpu_required,
        }
    }
}

/// Resource requirements (compatibility struct for BlockMatrix)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_limit: String,
    pub memory_limit: String,
    pub storage: Option<String>,
    pub gpu_required: bool,
}

/// Asset content definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetContent {
    pub main: String,
    pub files: Vec<String>,
    pub inline: Option<String>,
    pub binary: Vec<BinaryAsset>,
    pub templates: Vec<TemplateParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryAsset {
    pub name: String,
    pub mime_type: String,
    pub content: String,
    pub size: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    pub name: String,
    pub param_type: String,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
    pub required: bool,
    pub constraints: Option<ParameterConstraints>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConstraints {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub allowed_values: Option<Vec<serde_json::Value>>,
}

/// Security requirements and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSecurity {
    pub state_proof_required: bool,
    pub certificate_pinning: bool,
    pub hash_validation: String,
    pub sandbox_level: String,
    pub allowed_syscalls: Vec<String>,
    pub network_access: NetworkAccess,
    pub file_access: FileAccess,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAccess {
    pub enabled: bool,
    pub allowed_domains: Vec<String>,
    pub allowed_ports: Vec<u16>,
    pub require_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccess {
    pub level: String,
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub allow_temp: bool,
}

/// Resource constraints and requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetResources {
    pub cpu_limit: String,
    pub memory_limit: String,
    pub execution_timeout: String,
    pub storage_required: Option<String>,
    pub network_bandwidth: Option<String>,
    pub gpu_required: bool,
    pub hardware_requirements: Vec<HardwareRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareRequirement {
    pub hardware_type: String,
    pub minimum_spec: String,
    pub preferred_spec: Option<String>,
    pub required_features: Vec<String>,
}

/// Execution policy and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetExecution {
    pub delegation_strategy: String,
    pub minimum_state_proof: u32,
    pub retry_policy: String,
    pub max_concurrent: Option<u32>,
    pub priority: String,
    pub timeout_config: TimeoutConfig,
    pub scheduling: SchedulingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub execution: String,
    pub network: String,
    pub io: String,
    pub compilation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingConfig {
    pub timing: String,
    pub allocation_strategy: String,
    pub node_affinity: Vec<AffinityRule>,
    pub anti_affinity: Vec<AffinityRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityRule {
    pub rule_type: String,
    pub key: String,
    pub operator: String,
    pub values: Vec<String>,
    pub weight: Option<u32>,
}

/// Asset dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDependency {
    pub name: String,
    pub version: String,
    pub optional: bool,
    pub source: DependencySource,
    pub features: Vec<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DependencySource {
    Registry {
        registry: String,
        namespace: Option<String>,
    },
    Git {
        url: String,
        reference: String,
        path: Option<String>,
    },
    Local {
        path: String,
    },
    Http {
        url: String,
        blake3_hash: String,
    },
}

/// Resolved asset content with all files loaded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetContentResolved {
    pub main_content: String,
    pub file_contents: HashMap<String, String>,
    pub binary_contents: HashMap<String, Vec<u8>>,
    pub template_content: HashMap<String, String>,
    pub resolved_dependencies: Vec<ResolvedDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: String,
    pub resolved_source: String,
    pub package_hash: String,
    pub resolved_at: DateTime<Utc>,
}
