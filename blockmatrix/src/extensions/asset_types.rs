// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset-related extension types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use super::types::ResourceUsageReport;
use crate::assets::core::{
    AssetAllocation, AssetRegistration, AssetType, StateProof, PrivacyMode,
};

/// Asset creation specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetCreationSpec {
    pub name: String,
    pub description: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub privacy_level: PrivacyMode,
    pub allocation: Option<AssetAllocation>,
    pub state_requirements: StateRequirements,
    pub parent_id: Option<AssetRegistration>,
    pub tags: Vec<String>,
}

/// Asset update specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub privacy_level: Option<PrivacyMode>,
    pub allocation: Option<AssetAllocation>,
    pub tags: Option<Vec<String>>,
}

/// Asset query specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetQuery {
    pub asset_type: Option<AssetType>,
    pub name_pattern: Option<String>,
    pub tags: Option<Vec<String>>,
    pub privacy_level: Option<PrivacyMode>,
    pub parent_id: Option<AssetRegistration>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Rich asset record returned by extension queries.
///
/// This is distinct from `hypermesh_lib::AssetMetadata` (the canonical
/// cross-crate metadata). `ExtensionAssetRecord` is a blockmatrix-specific
/// view containing runtime state like privacy level, allocation, and
/// state proof status that only exist within the extension system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionAssetRecord {
    pub id: AssetRegistration,
    pub asset_type: AssetType,
    pub name: String,
    pub description: Option<String>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub size_bytes: u64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub privacy_level: PrivacyMode,
    pub allocation: Option<AssetAllocation>,
    pub state_proof_status: StateProofStatus,
    pub tags: Vec<String>,
}

/// Asset-specific operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetOperation {
    Deploy(DeploymentSpec),
    Execute(ExecutionSpec),
    Transfer(TransferSpec),
    Share(SharingSpec),
    Validate(StateProof),
    Custom(serde_json::Value),
}

/// Result of asset operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationResult {
    Deployed(DeploymentResult),
    Executed(ExecutionResult),
    Transferred(TransferResult),
    Shared(SharingResult),
    Validated(bool),
    Custom(serde_json::Value),
}

/// Deployment specification for deployable assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentSpec {
    pub environment: String,
    pub resources: ResourceRequirements,
    pub env_vars: HashMap<String, String>,
    pub network_config: Option<NetworkConfig>,
    pub volumes: Vec<VolumeMount>,
    pub state_proof: StateProof,
}

/// Resource requirements for deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu: CpuRequirement,
    pub memory_usage: MemoryRequirement,
    pub storage_usage: Option<StorageRequirement>,
    pub gpu_usage: Option<GpuRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirement {
    pub min_cores: f32,
    pub max_cores: Option<f32>,
    pub architecture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirement {
    pub min_bytes: u64,
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirement {
    pub min_bytes: u64,
    pub storage_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirement {
    pub min_count: u32,
    pub models: Option<Vec<String>>,
    pub min_vram: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub ports: Vec<PortMapping>,
    pub dns: Option<Vec<String>>,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSpec {
    pub code: String,
    pub language: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub timeout: Option<Duration>,
    pub state_proof: StateProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSpec {
    pub new_owner: String,
    pub reason: Option<String>,
    pub state_proof: StateProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingSpec {
    pub users: Vec<String>,
    pub access_level: String,
    pub expires_at: Option<SystemTime>,
    pub state_proof: StateProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentResult {
    pub deployment_id: String,
    pub status: String,
    pub endpoints: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: String,
    pub output: serde_json::Value,
    pub execution_time: Duration,
    pub resource_usage: ResourceUsageReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    pub transfer_id: String,
    pub transferred_at: SystemTime,
    pub new_owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingResult {
    pub share_id: String,
    pub shared_with: Vec<String>,
    pub shared_at: SystemTime,
}

/// State proof requirements for assets (bilateral Proof of State)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateRequirements {
    pub require_proof_of_space: bool,
    pub require_proof_of_stake: bool,
    pub require_proof_of_work: bool,
    pub require_proof_of_time: bool,
    pub min_space_commitment: Option<u64>,
    pub min_stake_amount: Option<u64>,
    pub min_work_difficulty: Option<u32>,
    pub time_window: Option<Duration>,
}

impl Default for StateRequirements {
    fn default() -> Self {
        Self {
            require_proof_of_space: true,
            require_proof_of_stake: true,
            require_proof_of_work: true,
            require_proof_of_time: true,
            min_space_commitment: Some(1024 * 1024),
            min_stake_amount: Some(100),
            min_work_difficulty: Some(4),
            time_window: Some(Duration::from_secs(300)),
        }
    }
}

/// State proof validation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProofStatus {
    pub validated: bool,
    pub last_validated: Option<SystemTime>,
    pub proofs: Option<StateProof>,
    pub errors: Vec<String>,
}
