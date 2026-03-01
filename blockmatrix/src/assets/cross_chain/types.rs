// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-chain validation types - validators, rules, ZK proofs, and results

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::assets::core::asset_id::AssetRegistration;
use crate::assets::matrix_blockchain::{
    EntityType, ProofRequirement, ValidationResult, ZKStatement,
};
use crate::consensus::ConsensusProof;

/// Cross-network validator for privacy-preserving multi-chain validation
#[derive(Clone, Serialize, Deserialize)]
pub struct CrossNetworkValidator {
    /// Source network performing validation
    pub source_network: String,
    /// Asset being validated across chains
    pub asset_identifier: AssetRegistration,
    /// Fields required for validation (without exposing values)
    pub required_fields: Vec<String>,
    /// Privacy-preserving validation rules
    pub validation_rules: Vec<CrossChainValidationRule>,
    /// Zero-knowledge proof statements
    pub zk_statements: Vec<ZKProofStatement>,
    /// Multi-network validation chain
    pub validation_chain: Vec<NetworkValidationStep>,
    /// Caching configuration
    pub cache_config: ValidationCacheConfig,
}

/// Privacy-preserving validation rule
#[derive(Clone, Serialize, Deserialize)]
pub struct CrossChainValidationRule {
    /// Rule identifier
    pub rule_id: String,
    /// Source entity type
    pub source_entity: EntityType,
    /// Target entity type
    pub target_entity: EntityType,
    /// Validation type (existence, property, zk-proof)
    pub validation_type: CrossChainValidationType,
    /// Privacy requirements
    pub privacy_requirements: PrivacyRequirements,
    /// Required consensus proofs
    pub required_proofs: Vec<ProofRequirement>,
}

/// Cross-chain validation types
#[derive(Clone, Serialize, Deserialize)]
pub enum CrossChainValidationType {
    /// Simple asset existence check
    AssetExistence,
    /// Property validation without revealing values
    PropertyValidation {
        field: String,
        validation_statement: ZKStatement,
    },
    /// Multi-field validation
    MultiFieldValidation {
        required_fields: Vec<String>,
        validation_logic: ValidationLogic,
    },
    /// Business workflow validation
    BusinessWorkflow {
        workflow_type: BusinessWorkflowType,
        steps: Vec<WorkflowValidationStep>,
    },
}

/// Business workflow types for real-world interactions
#[derive(Clone, Serialize, Deserialize)]
pub enum BusinessWorkflowType {
    /// Vehicle purchase workflow
    VehiclePurchase,
    /// Insurance claim validation
    InsuranceClaim,
    /// Asset financing workflow
    AssetFinancing,
    /// Supply chain validation
    SupplyChain,
    /// Custom business workflow
    Custom(String),
}

/// Individual step in business workflow validation
#[derive(Clone, Serialize, Deserialize)]
pub struct WorkflowValidationStep {
    /// Step name/identifier
    pub step_id: String,
    /// Entity responsible for this step
    pub responsible_entity: EntityType,
    /// Required validations for this step
    pub required_validations: Vec<String>,
    /// Public confirmations produced by this step
    pub public_confirmations: Vec<String>,
    /// Dependencies on other steps
    pub dependencies: Vec<String>,
}

/// Validation logic for multi-field validation
#[derive(Clone, Serialize, Deserialize)]
pub enum ValidationLogic {
    /// All fields must be valid
    And,
    /// At least one field must be valid
    Or,
    /// Exactly N fields must be valid
    ExactCount(usize),
    /// Custom validation logic
    Custom(String),
}

/// Zero-knowledge proof statement for privacy-preserving validation
#[derive(Clone, Serialize, Deserialize)]
pub struct ZKProofStatement {
    /// Statement identifier
    pub statement_id: String,
    /// Field being proven about
    pub field_name: String,
    /// ZK statement type
    pub statement_type: ZKStatementType,
    /// Public parameters (thresholds, ranges, etc.)
    pub public_parameters: HashMap<String, String>,
    /// Proof verification key
    pub verification_key: Vec<u8>,
}

/// Types of zero-knowledge statements
#[derive(Clone, Serialize, Deserialize)]
pub enum ZKStatementType {
    /// Prove value is greater than threshold without revealing value
    GreaterThan { threshold: f64, field: String },
    /// Prove value is less than threshold
    LessThan { threshold: f64, field: String },
    /// Prove value is within range
    InRange { min: f64, max: f64, field: String },
    /// Prove equality without revealing value
    EqualTo { field: String, commitment: Vec<u8> },
    /// Prove membership in set
    SetMembership {
        field: String,
        set_commitment: Vec<u8>,
    },
    /// Custom ZK statement
    Custom {
        statement_type: String,
        parameters: HashMap<String, String>,
    },
}

/// Privacy requirements for cross-chain validation
#[derive(Clone, Serialize, Deserialize)]
pub struct PrivacyRequirements {
    /// Fields that can be publicly confirmed
    pub public_confirmable_fields: Vec<String>,
    /// Fields requiring zero-knowledge proofs
    pub zk_proof_fields: Vec<String>,
    /// Fields that should never be exposed
    pub private_fields: Vec<String>,
    /// Trusted entities that can access federated data
    pub trusted_entities: Vec<String>,
    /// Maximum data retention period
    pub max_retention_period: Duration,
}

/// Network validation step in multi-chain workflow
#[derive(Clone, Serialize, Deserialize)]
pub struct NetworkValidationStep {
    /// Network domain (e.g., "honda.hypermesh.online")
    pub network_domain: String,
    /// Entity type in this network
    pub entity_type: EntityType,
    /// Validations to perform at this step
    pub validations: Vec<String>,
    /// Expected public confirmations
    pub expected_confirmations: Vec<String>,
    /// Order in validation chain
    pub step_order: usize,
    /// Dependencies on previous steps
    pub dependencies: Vec<usize>,
}

/// Caching configuration for validation performance
#[derive(Clone, Serialize, Deserialize)]
pub struct ValidationCacheConfig {
    /// Enable validation result caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Maximum cache entries
    pub max_cache_entries: usize,
    /// Cache invalidation triggers
    pub invalidation_triggers: Vec<CacheInvalidationTrigger>,
}

/// Triggers for cache invalidation
#[derive(Clone, Serialize, Deserialize)]
pub enum CacheInvalidationTrigger {
    /// Asset status change
    AssetStatusChange,
    /// Privacy policy update
    PrivacyPolicyUpdate,
    /// Entity trust relationship change
    TrustRelationshipChange,
    /// Time-based expiration
    TimeExpired,
    /// Manual invalidation
    Manual,
}

/// Cross-chain validation result with privacy preservation
#[derive(Clone, Serialize, Deserialize)]
pub struct CrossChainValidationResult {
    /// Validation request identifier
    pub validation_id: String,
    /// Overall validation status
    pub validation_status: CrossChainValidationStatus,
    /// Results from each network in validation chain
    pub network_results: HashMap<String, NetworkValidationResult>,
    /// Public confirmations that can be shared
    pub public_confirmations: HashMap<String, String>,
    /// ZK proof validations
    pub zk_proof_results: Vec<ZKProofResult>,
    /// Validation timestamp
    pub validated_at: SystemTime,
    /// Validation expiry
    pub expires_at: SystemTime,
    /// Consensus proofs for validation
    pub consensus_proofs: Vec<ConsensusProof>,
}

/// Cross-chain validation status
#[derive(Clone, Serialize, Deserialize)]
pub enum CrossChainValidationStatus {
    /// All validations successful
    Valid,
    /// Some validations failed
    PartiallyValid {
        valid_networks: Vec<String>,
        failed_networks: Vec<String>,
    },
    /// All validations failed
    Invalid { failure_reason: String },
    /// Validation pending
    Pending { pending_networks: Vec<String> },
    /// Insufficient permissions
    Unauthorized { unauthorized_networks: Vec<String> },
}

/// Result from individual network validation
#[derive(Clone, Serialize, Deserialize)]
pub struct NetworkValidationResult {
    /// Network domain
    pub network_domain: String,
    /// Entity type that performed validation
    pub validating_entity: EntityType,
    /// Validation status
    pub status: ValidationResult,
    /// Public confirmations from this network
    pub public_confirmations: HashMap<String, String>,
    /// Validation proof
    pub validation_proof: Vec<u8>,
    /// Timestamp of validation
    pub validated_at: SystemTime,
}

/// Zero-knowledge proof validation result
#[derive(Clone, Serialize, Deserialize)]
pub struct ZKProofResult {
    /// Statement that was proven
    pub statement_id: String,
    /// Proof verification result
    pub verification_result: bool,
    /// Public parameters used
    pub public_parameters: HashMap<String, String>,
    /// Proof data (without private information)
    pub proof_data: Vec<u8>,
}

/// Zero-knowledge proof system configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct ZKProofConfig {
    /// Proof system type (e.g., "PLONK", "STARK")
    pub proof_system: String,
    /// Security parameters
    pub security_parameters: HashMap<String, String>,
    /// Verification key storage
    pub verification_keys: HashMap<String, Vec<u8>>,
    /// Trusted setup parameters
    pub trusted_setup: Option<Vec<u8>>,
}

/// Cross-chain validation errors
#[derive(Debug, thiserror::Error)]
pub enum CrossChainValidationError {
    /// Network validation failed
    #[error("Network validation failed for {network_usage}: {error}")]
    NetworkValidationFailed {
        network_usage: String,
        error: String,
    },

    /// Zero-knowledge proof validation failed
    #[error("ZK proof validation failed for statement {statement_id}: {error}")]
    ZKProofValidationFailed { statement_id: String, error: String },

    /// Insufficient permissions
    #[error("Insufficient permissions for cross-chain validation")]
    InsufficientPermissions,

    /// Invalid validation rule
    #[error("Invalid validation rule: {rule_id}")]
    InvalidValidationRule { rule_id: String },

    /// Network not found
    #[error("Network not found: {network_usage}")]
    NetworkNotFound { network_usage: String },

    /// Asset not found
    #[error("Asset not found: {asset_id}")]
    AssetNotFound { asset_id: String },

    /// Validation timeout
    #[error("Validation timeout")]
    ValidationTimeout,

    /// Consensus proof validation failed
    #[error("Consensus proof validation failed: {error}")]
    ConsensusValidationFailed { error: String },
}
