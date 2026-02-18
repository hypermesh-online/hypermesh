// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Type definitions for Catalog VM Matrix Chain Integration

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::assets::matrix_blockchain::{
    EntityType, PublicValidationResponse, PrivacyPolicyConfig,
};
use crate::assets::core::AssetRegistration as CoreAssetId;
use super::super::{
    ExecutionContext, ExecutionResult, PrivacyLevel, ExecutionId,
    VMConfig,
};

/// Matrix-aware VM execution context
#[derive(Debug, Clone)]
pub struct MatrixExecutionContext {
    /// Base execution context
    pub base_context: ExecutionContext,
    /// Target entity for execution (e.g., "honda.hypermesh.online")
    pub target_entity: Option<String>,
    /// Cross-entity validation requirements
    pub cross_entity_validations: Vec<CrossEntityValidation>,
    /// Entity-specific privacy constraints
    pub entity_privacy_policies: HashMap<String, EntityPrivacyConstraints>,
    /// Multi-entity workflow configuration
    pub workflow_config: Option<MultiEntityWorkflow>,
    /// Asset allocation requests from specific entities
    pub entity_asset_requests: Vec<EntityAssetRequest>,
}

/// Cross-entity validation requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossEntityValidation {
    /// Entity domain to validate against
    pub entity_domain: String,
    /// Asset ID to validate
    pub asset_id: CoreAssetId,
    /// Fields to validate
    pub validation_fields: Vec<String>,
    /// Required validation type
    pub validation_type: ValidationRequirementType,
    /// Privacy level for validation
    pub privacy_level: PrivacyLevel,
}

/// Types of validation requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRequirementType {
    /// Simple existence check
    AssetExists,
    /// Property validation with specific constraints
    PropertyValidation {
        field: String,
        constraint: ValidationConstraint,
    },
    /// Zero-knowledge proof validation
    ZKProofValidation {
        statement: String,
        proof_type: String,
    },
    /// Consensus proof validation
    ConsensusValidation {
        required_proofs: Vec<String>,
    },
}

/// Validation constraints for property validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationConstraint {
    Equal(String),
    GreaterThan(f64),
    LessThan(f64),
    InRange { min: f64, max: f64 },
    OneOf(Vec<String>),
}

/// Entity-specific privacy constraints for VM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityPrivacyConstraints {
    /// Entity domain
    pub entity_domain: String,
    /// Maximum compute resources available from this entity
    pub max_compute_allocation: HashMap<String, u64>,
    /// Allowed operations on this entity's resources
    pub allowed_operations: Vec<String>,
    /// Privacy level for resource sharing
    pub resource_privacy_level: PrivacyLevel,
    /// Duration limits for resource usage
    pub max_duration_seconds: u64,
}

/// Multi-entity workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiEntityWorkflow {
    /// Ordered list of entities in the workflow
    pub entity_sequence: Vec<String>,
    /// Data flow between entities
    pub data_flow: HashMap<String, Vec<String>>,
    /// Synchronization requirements
    pub sync_requirements: Vec<EntitySyncRequirement>,
    /// Workflow privacy policy
    pub workflow_privacy: WorkflowPrivacyPolicy,
}

/// Synchronization requirement between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySyncRequirement {
    /// Source entity
    pub source_entity: String,
    /// Target entity
    pub target_entity: String,
    /// Synchronization type
    pub sync_type: SyncType,
    /// Maximum allowed delay (microseconds)
    pub max_delay_micros: u64,
}

/// Types of entity synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncType {
    /// Sequential execution (wait for completion)
    Sequential,
    /// Parallel execution with sync point
    ParallelSync,
    /// Asynchronous with eventual consistency
    EventuallyConsistent,
    /// Real-time synchronization
    RealTime,
}

/// Workflow privacy policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPrivacyPolicy {
    /// Privacy level for intermediate results
    pub intermediate_privacy: PrivacyLevel,
    /// Privacy level for final results
    pub final_privacy: PrivacyLevel,
    /// Entities that can see intermediate results
    pub intermediate_access: Vec<String>,
    /// Cross-entity data sharing rules
    pub data_sharing_rules: HashMap<String, Vec<String>>,
}

/// Asset allocation request from specific entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAssetRequest {
    /// Entity domain to request from
    pub entity_domain: String,
    /// Asset type (cpu, gpu, memory, storage)
    pub asset_type: String,
    /// Requested amount
    pub requested_amount: u64,
    /// Duration of usage
    pub duration_seconds: u64,
    /// Compensation offered (CAESAR tokens)
    pub compensation_tokens: u64,
    /// Priority level
    pub priority: AssetRequestPriority,
}

/// Priority levels for asset requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetRequestPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// VM configuration specific to an entity
#[derive(Debug, Clone)]
pub struct EntityVMConfig {
    /// Entity domain
    pub entity_domain: String,
    /// Entity type
    pub entity_type: EntityType,
    /// VM configuration for this entity
    pub vm_config: VMConfig,
    /// Privacy policies
    pub privacy_policies: PrivacyPolicyConfig,
    /// Trusted partners for cross-entity operations
    pub trusted_partners: Vec<String>,
    /// Maximum resource allocation for external requests
    pub max_external_allocation: HashMap<String, u64>,
}

/// Cached cross-chain validation result
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields populated during validation caching
pub(crate) struct CachedValidation {
    pub validation_request: CrossEntityValidation,
    pub validation_response: PublicValidationResponse,
    pub cached_at: std::time::SystemTime,
    pub expires_at: std::time::SystemTime,
}

/// Available assets for an entity
#[derive(Debug, Clone)]
pub(crate) struct EntityAssetPool {
    pub cpu_available: u64,
    pub gpu_available: u64,
    pub memory_available: u64,
    pub storage_available: u64,
    pub privacy_constraints: EntityPrivacyConstraints,
}

/// Active asset allocation tracking
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields populated during allocation tracking
pub(crate) struct ActiveAllocation {
    pub allocation_id: Uuid,
    pub entity_domain: String,
    pub asset_type: String,
    pub allocated_amount: u64,
    pub start_time: std::time::SystemTime,
    pub expires_at: std::time::SystemTime,
    pub executing_workflow: Option<String>,
}

/// Matrix execution result with entity-aware information
#[derive(Debug, Clone)]
pub struct MatrixExecutionResult {
    /// Base VM execution result
    pub base_result: ExecutionResult,
    /// Cross-entity validation results
    pub cross_entity_validations: HashMap<String, PublicValidationResponse>,
    /// Asset allocations from entities
    pub asset_allocations: HashMap<String, AssetAllocationSummary>,
    /// Entity interactions during execution
    pub entity_interactions: Vec<EntityInteraction>,
}

/// Asset allocation from specific entity
#[derive(Debug, Clone)]
pub struct EntityAssetAllocation {
    pub allocation_id: Uuid,
    pub entity_domain: String,
    pub asset_type: String,
    pub allocated_capacity: u64,
    pub total_capacity: u64,
    pub privacy_level: PrivacyLevel,
    pub expires_at: std::time::SystemTime,
}

/// Asset allocation summary for results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAllocationSummary {
    pub entity_domain: String,
    pub asset_type: String,
    pub allocated_capacity: u64,
    pub privacy_level: PrivacyLevel,
}

/// Entity interaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInteraction {
    pub interaction_type: InteractionType,
    pub source_entity: String,
    pub target_entity: String,
    pub asset_id: Option<ExecutionId>,
    pub timestamp: std::time::SystemTime,
    pub privacy_level: PrivacyLevel,
}

/// Types of entity interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    Validation,
    AssetRequest,
    DataExchange,
    ConsensusValidation,
}

impl From<EntityAssetAllocation> for AssetAllocationSummary {
    fn from(allocation: EntityAssetAllocation) -> Self {
        Self {
            entity_domain: allocation.entity_domain,
            asset_type: allocation.asset_type,
            allocated_capacity: allocation.allocated_capacity,
            privacy_level: allocation.privacy_level,
        }
    }
}
