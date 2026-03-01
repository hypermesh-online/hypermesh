// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Library types - metadata, package specs, content references, and templates

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::core::*;

/// Lightweight asset package for library operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryAssetPackage {
    #[serde(with = "arc_str_serde")]
    pub id: Arc<str>,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub asset_type: String,
    pub size: u64,
    pub hash: String,
    pub content: String,
    pub metadata: Option<PackageMetadata>,
    pub spec: Option<PackageSpec>,
    pub content_refs: Option<ContentReferences>,
    pub validation: Option<ValidationStatus>,
}

impl LibraryAssetPackage {
    pub fn version(&self) -> &str {
        &self.version
    }

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

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn dependencies(&self) -> &[PackageDependency] {
        self.spec
            .as_ref()
            .map(|s| s.dependencies.as_ref())
            .unwrap_or(&[])
    }

    pub fn author(&self) -> Option<&str> {
        self.metadata
            .as_ref()
            .and_then(|m| m.author.as_ref().map(|a| a.as_ref()))
    }

    pub fn license(&self) -> Option<&str> {
        self.metadata
            .as_ref()
            .and_then(|m| m.license.as_ref().map(|l| l.as_ref()))
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn tags(&self) -> Vec<String> {
        self.metadata
            .as_ref()
            .map(|m| m.tags.iter().map(|t| t.to_string()).collect())
            .unwrap_or_default()
    }

    pub fn runtime_type(&self) -> Option<&str> {
        self.spec.as_ref().map(|s| s.runtime.runtime_type.as_str())
    }

    pub fn content_ref(&self) -> &str {
        &self.content
    }

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
    #[serde(with = "arc_str_serde")]
    pub name: Arc<str>,
    #[serde(with = "arc_str_serde")]
    pub version: Arc<str>,
    #[serde(with = "option_arc_str_serde")]
    pub description: Option<Arc<str>>,
    #[serde(with = "option_arc_str_serde")]
    pub author: Option<Arc<str>>,
    #[serde(with = "option_arc_str_serde")]
    pub license: Option<Arc<str>>,
    #[serde(with = "arc_slice_arc_str_serde")]
    pub tags: Arc<[Arc<str>]>,
    #[serde(with = "arc_slice_arc_str_serde")]
    pub keywords: Arc<[Arc<str>]>,
    pub created: i64,
    pub modified: i64,
}

/// Package specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpec {
    pub runtime: RuntimeRequirements,
    pub resources: ResourceRequirements,
    pub security: SecurityConfig,
    pub execution: ExecutionConfig,
    #[serde(with = "arc_slice_dependency_serde")]
    pub dependencies: Arc<[PackageDependency]>,
    #[serde(with = "arc_hashmap_arc_str_serde")]
    pub environment: Arc<HashMap<Arc<str>, Arc<str>>>,
}

/// Runtime requirements for asset execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRequirements {
    pub runtime_type: String,
    pub version: String,
    pub dependencies: Vec<String>,
}

/// Resource requirements for asset execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_millicores: u32,
    pub memory_mb: u32,
    pub storage_mb: Option<u32>,
    pub gpu_required: bool,
    pub network_mbps: Option<u32>,
    pub timeout_seconds: u32,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_millicores: 100,
            memory_mb: 128,
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
    pub consensus_required: bool,
    pub sandbox_level: SandboxLevel,
    pub network_access: bool,
    pub filesystem_access: FilesystemAccess,
    #[serde(with = "arc_slice_arc_str_serde")]
    pub permissions: Arc<[Arc<str>]>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxLevel {
    None,
    Standard,
    Strict,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FilesystemAccess {
    None,
    ReadOnly,
    ReadWrite,
}

/// Execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub strategy: ExecutionStrategy,
    pub min_consensus: u32,
    pub max_concurrent: Option<u32>,
    pub priority: ExecutionPriority,
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStrategy {
    NearestNode,
    RandomNode,
    SpecificNode,
    LoadBalanced,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u32,
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
    #[serde(with = "arc_str_serde")]
    pub name: Arc<str>,
    #[serde(with = "arc_str_serde")]
    pub version_constraint: Arc<str>,
    pub optional: bool,
    #[serde(with = "option_arc_str_serde")]
    pub platform: Option<Arc<str>>,
}

/// Content references for lazy loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentReferences {
    pub main_ref: ContentRef,
    #[serde(with = "arc_slice_contentref_serde")]
    pub file_refs: Arc<[ContentRef]>,
    #[serde(with = "arc_slice_binaryref_serde")]
    pub binary_refs: Arc<[BinaryRef]>,
    pub total_size: u64,
}

/// Reference to content that can be loaded on demand
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRef {
    #[serde(with = "arc_str_serde")]
    pub path: Arc<str>,
    #[serde(with = "arc_str_serde")]
    pub hash: Arc<str>,
    pub size: u64,
    pub content_type: ContentType,
}

/// Binary content reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryRef {
    #[serde(with = "arc_str_serde")]
    pub id: Arc<str>,
    #[serde(with = "arc_str_serde")]
    pub mime_type: Arc<str>,
    #[serde(with = "arc_str_serde")]
    pub hash: Arc<str>,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentType {
    Source,
    Config,
    Documentation,
    Binary,
    Template,
}

/// Validation status for cached validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStatus {
    pub valid: bool,
    pub security_score: u32,
    pub validated_at: i64,
    pub expires_at: i64,
}

/// Template for package generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageTemplate {
    #[serde(with = "arc_str_serde")]
    pub id: Arc<str>,
    #[serde(with = "arc_str_serde")]
    pub name: Arc<str>,
    #[serde(with = "option_arc_str_serde")]
    pub description: Option<Arc<str>>,
    pub runtime: RuntimeRequirements,
    #[serde(with = "arc_slice_templateparam_serde")]
    pub parameters: Arc<[TemplateParameter]>,
    #[serde(with = "arc_hashmap_arc_str_serde")]
    pub files: Arc<HashMap<Arc<str>, Arc<str>>>,
}

/// Template parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    #[serde(with = "arc_str_serde")]
    pub name: Arc<str>,
    pub param_type: ParameterType,
    #[serde(with = "option_arc_str_serde")]
    pub default: Option<Arc<str>>,
    #[serde(with = "option_arc_str_serde")]
    pub description: Option<Arc<str>>,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}
