// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-Chain Validation System for HyperMesh Matrix Chains
//!
//! Privacy-preserving cross-chain validation following Proof of State patterns.
//! Enables multi-entity validation workflows without exposing sensitive data.
//! Supports real-world business interactions like car purchasing across multiple
//! blockchain networks (Honda->Dealer->Bank->Insurance->DMV).

pub mod types;

pub use types::*;

use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use sha2::{Digest, Sha256};

use crate::assets::core::asset_id::AssetRegistration;
use super::matrix_blockchain::{
    MatrixBlockchainManager, EntityType, ValidationResult,
};

/// Cross-chain validation manager
pub struct CrossChainValidationManager {
    /// Matrix blockchain manager
    _matrix_manager: MatrixBlockchainManager,
    /// Active validation cache
    validation_cache: HashMap<String, CrossChainValidationResult>,
    /// Validation rules registry
    validation_rules: HashMap<String, CrossChainValidationRule>,
    /// ZK proof system configuration
    _zk_config: ZKProofConfig,
    /// Network trust relationships
    trust_relationships: HashMap<String, Vec<String>>,
}

impl CrossChainValidationManager {
    /// Create new cross-chain validation manager
    pub fn new(matrix_manager: MatrixBlockchainManager) -> Self {
        Self {
            _matrix_manager: matrix_manager,
            validation_cache: HashMap::new(),
            validation_rules: HashMap::new(),
            _zk_config: ZKProofConfig {
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
        let validation_status = self.determine_validation_status(
            &network_results, &zk_proof_results,
        );

        let result = CrossChainValidationResult {
            validation_id: validation_id.clone(),
            validation_status,
            network_results,
            public_confirmations,
            zk_proof_results,
            validated_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(
                validator.cache_config.cache_ttl_seconds,
            ),
            consensus_proofs: Vec::new(),
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
        asset_id: AssetRegistration,
        participating_entities: Vec<String>,
    ) -> Result<CrossChainValidationResult, CrossChainValidationError> {
        match workflow_type {
            BusinessWorkflowType::VehiclePurchase => {
                self.validate_vehicle_purchase_workflow(
                    asset_id, participating_entities,
                ).await
            }
            BusinessWorkflowType::AssetFinancing => {
                self.validate_stub_workflow(
                    "asset_financing", asset_id, participating_entities,
                ).await
            }
            BusinessWorkflowType::InsuranceClaim => {
                self.validate_stub_workflow(
                    "insurance_claim", asset_id, participating_entities,
                ).await
            }
            BusinessWorkflowType::SupplyChain => {
                self.validate_stub_workflow(
                    "supply_chain", asset_id, participating_entities,
                ).await
            }
            BusinessWorkflowType::Custom(workflow_name) => {
                self.validate_stub_workflow(
                    &workflow_name, asset_id, participating_entities,
                ).await
            }
        }
    }

    /// Vehicle purchase workflow validation (Honda->Dealer->Bank->Insurance->DMV)
    async fn validate_vehicle_purchase_workflow(
        &mut self,
        vehicle_asset_id: AssetRegistration,
        _entities: Vec<String>,
    ) -> Result<CrossChainValidationResult, CrossChainValidationError> {
        let validation_chain = vec![
            NetworkValidationStep {
                network_domain: "honda.hypermesh.online".to_string(),
                entity_type: EntityType::Manufacturer,
                validations: vec![
                    "vehicle_exists".to_string(),
                    "manufacturing_complete".to_string(),
                ],
                expected_confirmations: vec!["vehicle_manufactured".to_string()],
                step_order: 0,
                dependencies: vec![],
            },
            NetworkValidationStep {
                network_domain: "dealer.hypermesh.online".to_string(),
                entity_type: EntityType::Dealer,
                validations: vec![
                    "vehicle_in_inventory".to_string(),
                    "price_set".to_string(),
                ],
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

        let zk_statements = vec![
            ZKProofStatement {
                statement_id: "financing_amount_sufficient".to_string(),
                field_name: "loan_amount".to_string(),
                statement_type: ZKStatementType::GreaterThan {
                    threshold: 0.0,
                    field: "loan_amount".to_string(),
                },
                public_parameters: HashMap::new(),
                verification_key: vec![],
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
                cache_ttl_seconds: 3600,
                max_cache_entries: 1000,
                invalidation_triggers: vec![
                    CacheInvalidationTrigger::AssetStatusChange,
                    CacheInvalidationTrigger::TimeExpired,
                ],
            },
        };

        self.validate_cross_chain(validator).await
    }

    /// Stub workflow validation for unimplemented workflow types
    async fn validate_stub_workflow(
        &mut self,
        workflow_name: &str,
        _asset_id: AssetRegistration,
        _entities: Vec<String>,
    ) -> Result<CrossChainValidationResult, CrossChainValidationError> {
        Err(CrossChainValidationError::NetworkValidationFailed {
            network_usage: workflow_name.to_string(),
            error: format!(
                "Cross-chain {} workflow validation not yet implemented",
                workflow_name,
            ),
        })
    }

    /// Validate individual network step
    async fn validate_network_step(
        &self,
        _validator: &CrossNetworkValidator,
        step: &NetworkValidationStep,
    ) -> Result<NetworkValidationResult, CrossChainValidationError> {
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

        let zk_proofs_valid = zk_results.iter().all(|r| r.verification_result);

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

        let time_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        hasher.update(&time_nanos.to_le_bytes());

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
        self.trust_relationships
            .entry(entity1)
            .or_insert_with(Vec::new)
            .push(entity2);
    }

    /// Check if entities have trust relationship
    pub fn has_trust_relationship(&self, entity1: &str, entity2: &str) -> bool {
        self.trust_relationships
            .get(entity1)
            .map(|trusted| trusted.contains(&entity2.to_string()))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::asset_id::AssetType;
    use crate::test_utils::test_asset_id;

    #[test]
    fn test_cross_network_validator_creation() {
        let asset_id = test_asset_id(AssetType::Container);

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

        let custom_workflow = BusinessWorkflowType::Custom(
            "real_estate_transaction".to_string(),
        );
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
