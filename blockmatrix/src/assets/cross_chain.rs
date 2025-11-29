//! Cross-Chain Validation System for HyperMesh Matrix Chains
//!
//! Privacy-preserving cross-chain validation following Proof of State patterns.
//! Enables multi-entity validation workflows without exposing sensitive data.
//! Supports real-world business interactions like car purchasing across multiple
//! blockchain networks (Honda→Dealer→Bank→Insurance→DMV).

use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::assets::core::asset_id::{AssetId, AssetType};
use super::blockchain::{HyperMeshAssetRecord, AssetPrivacyLevel};
use super::matrix_blockchain::{
    MatrixBlockchainManager, EntityBlockchain, EntityType, ValidationRequest,
    PublicValidationResponse, ValidationResult, ZKStatement, ValidationValue,
    EntityIdentifier, ProofRequirement
};
use crate::consensus::ConsensusProof;

/// Cross-network validator for privacy-preserving multi-chain validation
#[derive(Clone, Serialize, Deserialize)]
pub struct CrossNetworkValidator {
    /// Source network performing validation
    pub source_network: String,
    /// Asset being validated across chains
    pub asset_identifier: AssetId,
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
    GreaterThan {
        threshold: f64,
        field: String,
    },
    /// Prove value is less than threshold
    LessThan {
        threshold: f64,
        field: String,
    },
    /// Prove value is within range
    InRange {
        min: f64,
        max: f64,
        field: String,
    },
    /// Prove equality without revealing value
    EqualTo {
        field: String,
        commitment: Vec<u8>,
    },
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
    Invalid {
        failure_reason: String,
    },
    /// Validation pending
    Pending {
        pending_networks: Vec<String>,
    },
    /// Insufficient permissions
    Unauthorized {
        unauthorized_networks: Vec<String>,
    },
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

/// Cross-chain validation manager
pub struct CrossChainValidationManager {
    /// Matrix blockchain manager
    matrix_manager: MatrixBlockchainManager,
    /// Active validation cache
    validation_cache: HashMap<String, CrossChainValidationResult>,
    /// Validation rules registry
    validation_rules: HashMap<String, CrossChainValidationRule>,
    /// ZK proof system configuration
    zk_config: ZKProofConfig,
    /// Network trust relationships
    trust_relationships: HashMap<String, Vec<String>>,
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

impl CrossChainValidationManager {
    /// Create new cross-chain validation manager
    pub fn new(matrix_manager: MatrixBlockchainManager) -> Self {
        Self {
            matrix_manager,
            validation_cache: HashMap::new(),
            validation_rules: HashMap::new(),
            zk_config: ZKProofConfig {
                proof_system: "PLONK".to_string(),
                security_parameters: HashMap::new(),
                verification_keys: HashMap::new(),
                trusted_setup: None,
            },
            trust_relationships: HashMap::new(),
        }
    }

    /// Register validation rule for cross-chain interactions
    pub fn register_validation_rule(&mut self, rule: CrossChainValidationRule) {
        self.validation_rules.insert(rule.rule_id.clone(), rule);
    }

    /// Perform privacy-preserving cross-chain validation
    pub async fn validate_cross_chain(
        &mut self,
        validator: CrossNetworkValidator,
    ) -> Result<CrossChainValidationResult, CrossChainValidationError> {
        let validation_id = self.generate_validation_id(&validator);

        // Check cache first if enabled
        if validator.cache_config.enable_caching {
            if let Some(cached_result) = self.validation_cache.get(&validation_id) {
                if cached_result.expires_at > SystemTime::now() {
                    return Ok(cached_result.clone());
                }
            }
        }

        // Validate each network in the validation chain
        let mut network_results = HashMap::new();
        let mut public_confirmations = HashMap::new();
        let mut zk_proof_results = Vec::new();

        for step in &validator.validation_chain {
            match self.validate_network_step(&validator, step).await {
                Ok(result) => {
                    // Add public confirmations
                    for (key, value) in &result.public_confirmations {
                        public_confirmations.insert(
                            format!("{}:{}", step.network_domain, key),
                            value.clone()
                        );
                    }
                    network_results.insert(step.network_domain.clone(), result);
                }
                Err(e) => {
                    return Err(CrossChainValidationError::NetworkValidationFailed {
                        network_usage: step.network_domain.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }

        // Perform zero-knowledge proof validations
        for zk_statement in &validator.zk_statements {
            match self.validate_zk_proof(zk_statement).await {
                Ok(proof_result) => {
                    zk_proof_results.push(proof_result);
                }
                Err(e) => {
                    return Err(CrossChainValidationError::ZKProofValidationFailed {
                        statement_id: zk_statement.statement_id.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }

        // Determine overall validation status
        let validation_status = self.determine_validation_status(&network_results, &zk_proof_results);

        let result = CrossChainValidationResult {
            validation_id: validation_id.clone(),
            validation_status,
            network_results,
            public_confirmations,
            zk_proof_results,
            validated_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(validator.cache_config.cache_ttl_seconds),
            consensus_proofs: Vec::new(), // Would be populated with actual proofs
        };

        // Cache result if enabled
        if validator.cache_config.enable_caching {
            self.validation_cache.insert(validation_id, result.clone());
        }

        Ok(result)
    }

    /// Validate business workflow across multiple entities
    pub async fn validate_business_workflow(
        &mut self,
        workflow_type: BusinessWorkflowType,
        asset_id: AssetId,
        participating_entities: Vec<String>,
    ) -> Result<CrossChainValidationResult, CrossChainValidationError> {
        match workflow_type {
            BusinessWorkflowType::VehiclePurchase => {
                self.validate_vehicle_purchase_workflow(asset_id, participating_entities).await
            }
            BusinessWorkflowType::AssetFinancing => {
                self.validate_asset_financing_workflow(asset_id, participating_entities).await
            }
            BusinessWorkflowType::InsuranceClaim => {
                self.validate_insurance_claim_workflow(asset_id, participating_entities).await
            }
            BusinessWorkflowType::SupplyChain => {
                self.validate_supply_chain_workflow(asset_id, participating_entities).await
            }
            BusinessWorkflowType::Custom(workflow_name) => {
                self.validate_custom_workflow(workflow_name, asset_id, participating_entities).await
            }
        }
    }

    /// Vehicle purchase workflow validation (Honda→Dealer→Bank→Insurance→DMV)
    async fn validate_vehicle_purchase_workflow(
        &mut self,
        vehicle_asset_id: AssetId,
        entities: Vec<String>,
    ) -> Result<CrossChainValidationResult, CrossChainValidationError> {
        // Create validation chain for vehicle purchase
        let validation_chain = vec![
            NetworkValidationStep {
                network_domain: "honda.hypermesh.online".to_string(),
                entity_type: EntityType::Manufacturer,
                validations: vec!["vehicle_exists".to_string(), "manufacturing_complete".to_string()],
                expected_confirmations: vec!["vehicle_manufactured".to_string()],
                step_order: 0,
                dependencies: vec![],
            },
            NetworkValidationStep {
                network_domain: "dealer.hypermesh.online".to_string(),
                entity_type: EntityType::Dealer,
                validations: vec!["vehicle_in_inventory".to_string(), "price_set".to_string()],
                expected_confirmations: vec!["available_for_sale".to_string()],
                step_order: 1,
                dependencies: vec![0],
            },
            NetworkValidationStep {
                network_domain: "bank.hypermesh.online".to_string(),
                entity_type: EntityType::Bank,
                validations: vec!["financing_approved".to_string()],
                expected_confirmations: vec!["financing_approved".to_string()],
                step_order: 2,
                dependencies: vec![1],
            },
            NetworkValidationStep {
                network_domain: "insurance.hypermesh.online".to_string(),
                entity_type: EntityType::Insurance,
                validations: vec!["policy_issued".to_string()],
                expected_confirmations: vec!["insurance_active".to_string()],
                step_order: 3,
                dependencies: vec![2],
            },
            NetworkValidationStep {
                network_domain: "dmv.hypermesh.online".to_string(),
                entity_type: EntityType::DMV,
                validations: vec!["registration_complete".to_string()],
                expected_confirmations: vec!["vehicle_registered".to_string()],
                step_order: 4,
                dependencies: vec![3],
            },
        ];

        // ZK statements for sensitive data
        let zk_statements = vec![
            ZKProofStatement {
                statement_id: "financing_amount_sufficient".to_string(),
                field_name: "loan_amount".to_string(),
                statement_type: ZKStatementType::GreaterThan {
                    threshold: 0.0, // Vehicle price (would be dynamically set)
                    field: "loan_amount".to_string(),
                },
                public_parameters: HashMap::new(),
                verification_key: vec![], // Would contain actual verification key
            },
        ];

        let validator = CrossNetworkValidator {
            source_network: "buyer.hypermesh.online".to_string(),
            asset_identifier: vehicle_asset_id,
            required_fields: vec![
                "vin".to_string(),
                "manufacturing_status".to_string(),
                "price".to_string(),
                "financing_status".to_string(),
                "insurance_status".to_string(),
                "registration_status".to_string(),
            ],
            validation_rules: vec![],
            zk_statements,
            validation_chain,
            cache_config: ValidationCacheConfig {
                enable_caching: true,
                cache_ttl_seconds: 3600, // 1 hour
                max_cache_entries: 1000,
                invalidation_triggers: vec![
                    CacheInvalidationTrigger::AssetStatusChange,
                    CacheInvalidationTrigger::TimeExpired,
                ],
            },
        };

        self.validate_cross_chain(validator).await
    }

    /// Asset financing workflow validation
    async fn validate_asset_financing_workflow(
        &mut self,
        asset_id: AssetId,
        entities: Vec<String>,
    ) -> Result<CrossChainValidationResult, CrossChainValidationError> {
        // Implementation for asset financing workflow
        // This would include credit checks, asset valuation, etc.
        todo!("Implement asset financing workflow validation")
    }

    /// Insurance claim workflow validation
    async fn validate_insurance_claim_workflow(
        &mut self,
        asset_id: AssetId,
        entities: Vec<String>,
    ) -> Result<CrossChainValidationResult, CrossChainValidationError> {
        // Implementation for insurance claim workflow
        todo!("Implement insurance claim workflow validation")
    }

    /// Supply chain workflow validation
    async fn validate_supply_chain_workflow(
        &mut self,
        asset_id: AssetId,
        entities: Vec<String>,
    ) -> Result<CrossChainValidationResult, CrossChainValidationError> {
        // Implementation for supply chain workflow
        todo!("Implement supply chain workflow validation")
    }

    /// Custom workflow validation
    async fn validate_custom_workflow(
        &mut self,
        workflow_name: String,
        asset_id: AssetId,
        entities: Vec<String>,
    ) -> Result<CrossChainValidationResult, CrossChainValidationError> {
        // Implementation for custom workflow validation
        todo!("Implement custom workflow validation")
    }

    /// Validate individual network step
    async fn validate_network_step(
        &self,
        validator: &CrossNetworkValidator,
        step: &NetworkValidationStep,
    ) -> Result<NetworkValidationResult, CrossChainValidationError> {
        // This would interface with the actual entity blockchain
        // For now, return a mock result
        Ok(NetworkValidationResult {
            network_domain: step.network_domain.clone(),
            validating_entity: step.entity_type.clone(),
            status: ValidationResult::Valid,
            public_confirmations: HashMap::new(),
            validation_proof: vec![],
            validated_at: SystemTime::now(),
        })
    }

    /// Validate zero-knowledge proof
    async fn validate_zk_proof(
        &self,
        statement: &ZKProofStatement,
    ) -> Result<ZKProofResult, CrossChainValidationError> {
        // This would interface with the actual ZK proof system
        // For now, return a mock result
        Ok(ZKProofResult {
            statement_id: statement.statement_id.clone(),
            verification_result: true,
            public_parameters: statement.public_parameters.clone(),
            proof_data: vec![],
        })
    }

    /// Determine overall validation status
    fn determine_validation_status(
        &self,
        network_results: &HashMap<String, NetworkValidationResult>,
        zk_results: &[ZKProofResult],
    ) -> CrossChainValidationStatus {
        let valid_networks: Vec<String> = network_results
            .iter()
            .filter_map(|(domain, result)| {
                match result.status {
                    ValidationResult::Valid => Some(domain.clone()),
                    _ => None,
                }
            })
            .collect();

        let failed_networks: Vec<String> = network_results
            .iter()
            .filter_map(|(domain, result)| {
                match result.status {
                    ValidationResult::Invalid { .. } => Some(domain.clone()),
                    _ => None,
                }
            })
            .collect();

        let zk_proofs_valid = zk_results.iter().all(|result| result.verification_result);

        if valid_networks.len() == network_results.len() && zk_proofs_valid {
            CrossChainValidationStatus::Valid
        } else if !valid_networks.is_empty() {
            CrossChainValidationStatus::PartiallyValid {
                valid_networks,
                failed_networks,
            }
        } else {
            CrossChainValidationStatus::Invalid {
                failure_reason: "All network validations failed".to_string(),
            }
        }
    }

    /// Generate unique validation ID
    fn generate_validation_id(&self, validator: &CrossNetworkValidator) -> String {
        let mut hasher = Sha256::new();
        hasher.update(validator.source_network.as_bytes());
        hasher.update(validator.asset_identifier.to_hex_string().as_bytes());
        hasher.update(&SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos().to_le_bytes());
        
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Clear expired cache entries
    pub fn cleanup_cache(&mut self) {
        let now = SystemTime::now();
        self.validation_cache.retain(|_, result| result.expires_at > now);
    }

    /// Add trust relationship between entities
    pub fn add_trust_relationship(&mut self, entity1: String, entity2: String) {
        self.trust_relationships.entry(entity1).or_insert_with(Vec::new).push(entity2);
    }

    /// Check if entities have trust relationship
    pub fn has_trust_relationship(&self, entity1: &str, entity2: &str) -> bool {
        self.trust_relationships
            .get(entity1)
            .map(|trusted| trusted.contains(&entity2.to_string()))
            .unwrap_or(false)
    }
}

/// Cross-chain validation errors
#[derive(Debug, thiserror::Error)]
pub enum CrossChainValidationError {
    /// Network validation failed
    #[error("Network validation failed for {network}: {error}")]
    NetworkValidationFailed { network_usage: String, error: String },
    
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
    #[error("Network not found: {network}")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::asset_id::AssetId;

    #[test]
    fn test_cross_network_validator_creation() {
        let asset_id = AssetId::new(AssetType::Container);
        
        let validator = CrossNetworkValidator {
            source_network: "buyer.hypermesh.online".to_string(),
            asset_identifier: asset_id,
            required_fields: vec!["status".to_string()],
            validation_rules: vec![],
            zk_statements: vec![],
            validation_chain: vec![],
            cache_config: ValidationCacheConfig {
                enable_caching: true,
                cache_ttl_seconds: 3600,
                max_cache_entries: 1000,
                invalidation_triggers: vec![],
            },
        };

        assert_eq!(validator.source_network, "buyer.hypermesh.online");
        assert!(validator.cache_config.enable_caching);
    }

    #[test]
    fn test_validation_rule_creation() {
        let rule = CrossChainValidationRule {
            rule_id: "vehicle_purchase_rule".to_string(),
            source_entity: EntityType::User,
            target_entity: EntityType::Dealer,
            validation_type: CrossChainValidationType::AssetExistence,
            privacy_requirements: PrivacyRequirements {
                public_confirmable_fields: vec!["vehicle_available".to_string()],
                zk_proof_fields: vec!["price".to_string()],
                private_fields: vec!["cost_basis".to_string()],
                trusted_entities: vec!["bank.hypermesh.online".to_string()],
                max_retention_period: Duration::from_secs(86400),
            },
            required_proofs: vec![ProofRequirement::ConsensusProof],
        };

        assert_eq!(rule.rule_id, "vehicle_purchase_rule");
        assert!(matches!(rule.source_entity, EntityType::User));
        assert!(matches!(rule.target_entity, EntityType::Dealer));
    }

    #[test]
    fn test_zk_proof_statement() {
        let statement = ZKProofStatement {
            statement_id: "price_greater_than_threshold".to_string(),
            field_name: "vehicle_price".to_string(),
            statement_type: ZKStatementType::GreaterThan {
                threshold: 20000.0,
                field: "vehicle_price".to_string(),
            },
            public_parameters: HashMap::new(),
            verification_key: vec![1, 2, 3, 4],
        };

        assert_eq!(statement.statement_id, "price_greater_than_threshold");
        assert_eq!(statement.field_name, "vehicle_price");
        if let ZKStatementType::GreaterThan { threshold, .. } = statement.statement_type {
            assert_eq!(threshold, 20000.0);
        } else {
            panic!("Expected GreaterThan statement type");
        }
    }

    #[test]
    fn test_business_workflow_type() {
        let workflow = BusinessWorkflowType::VehiclePurchase;
        assert!(matches!(workflow, BusinessWorkflowType::VehiclePurchase));

        let custom_workflow = BusinessWorkflowType::Custom("real_estate_transaction".to_string());
        if let BusinessWorkflowType::Custom(name) = custom_workflow {
            assert_eq!(name, "real_estate_transaction");
        } else {
            panic!("Expected Custom workflow type");
        }
    }

    #[tokio::test]
    async fn test_cross_chain_validation_manager() {
        let matrix_manager = MatrixBlockchainManager::new();
        let mut validator_manager = CrossChainValidationManager::new(matrix_manager);

        // Add trust relationship
        validator_manager.add_trust_relationship(
            "dealer.hypermesh.online".to_string(),
            "bank.hypermesh.online".to_string()
        );

        assert!(validator_manager.has_trust_relationship(
            "dealer.hypermesh.online",
            "bank.hypermesh.online"
        ));
        assert!(!validator_manager.has_trust_relationship(
            "bank.hypermesh.online",
            "dealer.hypermesh.online"
        ));
    }
}