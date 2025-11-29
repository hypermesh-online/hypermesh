//! Asset package and library types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use semver::Version;

use crate::assets::core::{AssetType, AssetId, ConsensusProof, ProxyAddress};
use super::types::ExtensionResult;

/// Asset package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPackage {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub description: String,
    pub author: String,
    pub license: String,
    pub asset_types: Vec<AssetType>,
    pub size_bytes: u64,
    pub install_count: u64,
    pub rating: f32,
    pub dependencies: Vec<PackageDependency>,
    pub signature: Option<String>,
    pub distribution_hash: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Package filter for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFilter {
    pub asset_type: Option<AssetType>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub min_rating: Option<f32>,
    pub verified_only: bool,
}

/// Package dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDependency {
    pub package_id: String,
    pub version_req: semver::VersionReq,
    pub optional: bool,
}

/// Installation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallOptions {
    pub install_dir: Option<PathBuf>,
    pub include_optional: bool,
    pub verify_signatures: bool,
    pub use_proxy: Option<ProxyAddress>,
    pub consensus_proof: ConsensusProof,
}

/// Installation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub package_id: String,
    pub install_path: PathBuf,
    pub installed_assets: Vec<AssetId>,
    pub install_time: Duration,
}

/// Update result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub package_id: String,
    pub from_version: Version,
    pub to_version: Version,
    pub update_time: Duration,
}

/// Search options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
}

/// Package specification for publishing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPackageSpec {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub contents: Vec<u8>,
    pub assets: Vec<AssetManifest>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Asset manifest in package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    pub asset_type: AssetType,
    pub name: String,
    pub data: Vec<u8>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Publish result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub package_id: String,
    pub version: Version,
    pub distribution_hash: String,
    pub signature: String,
}

/// Package verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub verified: bool,
    pub signature_valid: Option<bool>,
    pub integrity_valid: bool,
    pub license_compliant: bool,
    pub security_issues: Vec<SecurityIssue>,
}

/// Security issue found in package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub severity: String,
    pub issue_type: String,
    pub description: String,
    pub affected_files: Vec<String>,
}
