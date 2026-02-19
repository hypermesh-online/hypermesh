// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Operations and implementation for the Matrix-aware VM system

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;

use crate::assets::matrix_blockchain::{
    MatrixBlockchainManager, EntityType,
    ValidationRequest, PublicValidationResponse,
};
use super::super::{
    ConsensusProofVM, ExecutionResult, ExecutionContext,
    PrivacyMode, AssetAllocation,
};
use super::types::*;
use super::coordinator::EntityAssetCoordinator;

/// Matrix-aware VM that integrates with entity blockchains
pub struct MatrixAwareVM {
    /// Base consensus VM
    base_vm: Arc<ConsensusProofVM>,
    /// Matrix blockchain manager
    matrix_manager: Arc<MatrixBlockchainManager>,
    /// Entity-specific configurations
    entity_configs: HashMap<String, EntityVMConfig>,
    /// Cross-chain validation cache
    validation_cache: Arc<std::sync::Mutex<HashMap<String, CachedValidation>>>,
    /// Asset allocation coordinator
    asset_coordinator: Arc<EntityAssetCoordinator>,
}

impl MatrixAwareVM {
    /// Create new matrix-aware VM instance
    pub async fn new(
        base_vm: Arc<ConsensusProofVM>,
        matrix_manager: Arc<MatrixBlockchainManager>,
    ) -> Result<Self> {
        let asset_coordinator = Arc::new(EntityAssetCoordinator::new());

        Ok(Self {
            base_vm,
            matrix_manager,
            entity_configs: HashMap::new(),
            validation_cache: Arc::new(std::sync::Mutex::new(HashMap::new())),
            asset_coordinator,
        })
    }

    /// Register entity configuration for VM operations
    pub fn register_entity_config(
        &mut self,
        entity_domain: String,
        config: EntityVMConfig,
    ) -> Result<()> {
        self.asset_coordinator.update_entity_pool(&entity_domain, &config)?;
        self.entity_configs.insert(entity_domain, config);
        Ok(())
    }

    /// Execute code with matrix chain awareness
    pub async fn execute_matrix_aware(
        &self,
        code: &str,
        language: &str,
        context: MatrixExecutionContext,
    ) -> Result<MatrixExecutionResult> {
        self.validate_consensus_against_entity(&context).await?;
        let validation_results = self.perform_cross_entity_validations(&context).await?;
        let asset_allocations = self.allocate_entity_assets(&context).await?;

        let execution_result = if let Some(workflow) = &context.workflow_config {
            self.execute_multi_entity_workflow(
                code, language, &context, workflow, &asset_allocations,
            ).await?
        } else {
            self.execute_on_target_entity(
                code, language, &context, &asset_allocations,
            ).await?
        };

        self.cleanup_asset_allocations(&asset_allocations).await?;

        Ok(MatrixExecutionResult {
            base_result: execution_result,
            cross_entity_validations: validation_results,
            asset_allocations: asset_allocations.into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            entity_interactions: self.get_entity_interactions(&context).await?,
        })
    }

    async fn validate_consensus_against_entity(
        &self,
        context: &MatrixExecutionContext,
    ) -> Result<()> {
        if let Some(target_entity) = &context.target_entity {
            if let Some(_entity_config) = self.entity_configs.get(target_entity) {
                let consensus_proof = &context.base_context.consensus_proof;

                if !self.base_vm.consensus_vm()
                    .read()
                    .await
                    .validate_consensus_proof(consensus_proof)
                    .await? {
                    return Err(anyhow::anyhow!(
                        "Consensus proof validation failed for entity: {}",
                        target_entity
                    ));
                }
            }
        }
        Ok(())
    }

    async fn perform_cross_entity_validations(
        &self,
        context: &MatrixExecutionContext,
    ) -> Result<HashMap<String, PublicValidationResponse>> {
        let mut results = HashMap::new();

        for validation in &context.cross_entity_validations {
            let cache_key = format!(
                "{}:{}:{}",
                validation.entity_domain,
                validation.asset_id,
                serde_json::to_string(&validation.validation_fields)?
            );

            if let Some(cached) = self.get_cached_validation(&cache_key) {
                if cached.expires_at > std::time::SystemTime::now() {
                    results.insert(validation.entity_domain.clone(), cached.validation_response);
                    continue;
                }
            }

            let _validation_request = ValidationRequest {
                asset_id: validation.asset_id.clone(),
                requested_fields: validation.validation_fields.clone(),
                validation_type: self.convert_validation_type(&validation.validation_type),
                requester: self.create_entity_identifier(&context.base_context)?,
                proof_requirements: vec![],
            };

            let validation_results = self.matrix_manager
                .multi_entity_validation(validation.asset_id.clone(), vec![validation.entity_domain.clone()])
                .await
                .map_err(|e| anyhow::anyhow!("Matrix validation failed: {}", e))?;

            if let Some(response) = validation_results.get(&validation.entity_domain) {
                results.insert(validation.entity_domain.clone(), response.clone());
                self.cache_validation(cache_key, validation.clone(), response.clone());
            }
        }

        Ok(results)
    }

    async fn allocate_entity_assets(
        &self,
        context: &MatrixExecutionContext,
    ) -> Result<HashMap<String, EntityAssetAllocation>> {
        let mut allocations = HashMap::new();

        for request in &context.entity_asset_requests {
            let allocation = self.asset_coordinator
                .allocate_asset_from_entity(request)
                .await?;

            allocations.insert(
                format!("{}_{}", request.entity_domain, request.asset_type),
                allocation,
            );
        }

        Ok(allocations)
    }

    async fn execute_multi_entity_workflow(
        &self,
        code: &str,
        language: &str,
        context: &MatrixExecutionContext,
        workflow: &MultiEntityWorkflow,
        asset_allocations: &HashMap<String, EntityAssetAllocation>,
    ) -> Result<ExecutionResult> {
        let mut workflow_results = Vec::new();
        let mut intermediate_data: HashMap<String, serde_json::Value> = HashMap::new();

        for (step_index, entity_domain) in workflow.entity_sequence.iter().enumerate() {
            self.check_sync_requirements(workflow, step_index, &intermediate_data).await?;

            let _entity_context = self.create_entity_execution_context(
                context, entity_domain, asset_allocations, &intermediate_data,
            )?;

            let step_result = self.execute_on_target_entity(
                code, language,
                &MatrixExecutionContext {
                    target_entity: Some(entity_domain.clone()),
                    ..context.clone()
                },
                asset_allocations,
            ).await?;

            if workflow.workflow_privacy.intermediate_access.contains(entity_domain) {
                if let Some(output) = &step_result.output {
                    intermediate_data.insert(entity_domain.clone(), output.clone());
                }
            }

            workflow_results.push((entity_domain.clone(), step_result));
        }

        self.aggregate_workflow_results(workflow_results, workflow).await
    }

    async fn execute_on_target_entity(
        &self,
        code: &str,
        language: &str,
        context: &MatrixExecutionContext,
        asset_allocations: &HashMap<String, EntityAssetAllocation>,
    ) -> Result<ExecutionResult> {
        let _enhanced_context = ExecutionContext {
            asset_allocations: self.convert_entity_allocations_to_vm_allocations(asset_allocations)?,
            ..context.base_context.clone()
        };

        let consensus_proof = context.base_context.consensus_proof.clone();
        self.base_vm.execute_with_consensus(code, language, consensus_proof).await
    }

    async fn get_entity_interactions(
        &self,
        context: &MatrixExecutionContext,
    ) -> Result<Vec<EntityInteraction>> {
        let mut interactions = Vec::new();

        for validation in &context.cross_entity_validations {
            interactions.push(EntityInteraction {
                interaction_type: InteractionType::Validation,
                source_entity: context.target_entity.clone().unwrap_or_default(),
                target_entity: validation.entity_domain.clone(),
                asset_id: Some({
                    let mut bytes = [0u8; 16];
                    bytes.copy_from_slice(&validation.asset_id.content_hash[..16]);
                    uuid::Uuid::from_bytes(bytes)
                }),
                timestamp: std::time::SystemTime::now(),
                privacy_level: validation.privacy_level.clone(),
            });
        }

        for request in &context.entity_asset_requests {
            interactions.push(EntityInteraction {
                interaction_type: InteractionType::AssetRequest,
                source_entity: context.target_entity.clone().unwrap_or_default(),
                target_entity: request.entity_domain.clone(),
                asset_id: None,
                timestamp: std::time::SystemTime::now(),
                privacy_level: PrivacyMode::PRIVATE,
            });
        }

        Ok(interactions)
    }

    fn get_cached_validation(&self, cache_key: &str) -> Option<CachedValidation> {
        self.validation_cache.lock().expect("mutex poisoned").get(cache_key).cloned()
    }

    fn cache_validation(
        &self,
        cache_key: String,
        validation: CrossEntityValidation,
        response: PublicValidationResponse,
    ) {
        let cached = CachedValidation {
            validation_request: validation,
            validation_response: response,
            cached_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(300),
        };
        self.validation_cache.lock().expect("mutex poisoned").insert(cache_key, cached);
    }

    fn convert_validation_type(
        &self,
        validation_type: &ValidationRequirementType,
    ) -> crate::assets::matrix_blockchain::ValidationType {
        match validation_type {
            ValidationRequirementType::AssetExists => {
                crate::assets::matrix_blockchain::ValidationType::Existence
            },
            ValidationRequirementType::PropertyValidation { field, constraint } => {
                let expected_value = match constraint {
                    ValidationConstraint::Equal(s) => {
                        crate::assets::matrix_blockchain::ValidationValue::String(s.clone())
                    },
                    ValidationConstraint::GreaterThan(n) => {
                        crate::assets::matrix_blockchain::ValidationValue::GreaterThan(*n)
                    },
                    ValidationConstraint::LessThan(n) => {
                        crate::assets::matrix_blockchain::ValidationValue::LessThan(*n)
                    },
                    ValidationConstraint::InRange { min, max } => {
                        crate::assets::matrix_blockchain::ValidationValue::Range { min: *min, max: *max }
                    },
                    ValidationConstraint::OneOf(options) => {
                        crate::assets::matrix_blockchain::ValidationValue::String(options[0].clone())
                    },
                };

                crate::assets::matrix_blockchain::ValidationType::PropertyValidation {
                    field: field.clone(),
                    expected_value,
                }
            },
            _ => crate::assets::matrix_blockchain::ValidationType::Existence,
        }
    }

    fn create_entity_identifier(
        &self,
        _context: &ExecutionContext,
    ) -> Result<crate::assets::matrix_blockchain::EntityIdentifier> {
        Ok(crate::assets::matrix_blockchain::EntityIdentifier {
            network_domain: "vm.hypermesh.online".to_string(),
            entity_type: EntityType::Organization("VM".to_string()),
            certificate_fingerprint: "vm-cert-placeholder".to_string(),
        })
    }

    async fn check_sync_requirements(
        &self,
        _workflow: &MultiEntityWorkflow,
        _step_index: usize,
        _intermediate_data: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        Ok(())
    }

    fn create_entity_execution_context(
        &self,
        base_context: &MatrixExecutionContext,
        _entity_domain: &str,
        _asset_allocations: &HashMap<String, EntityAssetAllocation>,
        _intermediate_data: &HashMap<String, serde_json::Value>,
    ) -> Result<ExecutionContext> {
        Ok(base_context.base_context.clone())
    }

    async fn aggregate_workflow_results(
        &self,
        workflow_results: Vec<(String, ExecutionResult)>,
        _workflow: &MultiEntityWorkflow,
    ) -> Result<ExecutionResult> {
        if let Some((_, first_result)) = workflow_results.first() {
            Ok(first_result.clone())
        } else {
            Err(anyhow::anyhow!("No workflow results to aggregate"))
        }
    }

    fn convert_entity_allocations_to_vm_allocations(
        &self,
        entity_allocations: &HashMap<String, EntityAssetAllocation>,
    ) -> Result<HashMap<String, AssetAllocation>> {
        let mut vm_allocations = HashMap::new();

        for (key, entity_allocation) in entity_allocations {
            let asset_allocation = AssetAllocation {
                total_capacity: entity_allocation.total_capacity,
                available_capacity: entity_allocation.allocated_capacity,
                shared_capacity: entity_allocation.allocated_capacity,
                privacy_level: entity_allocation.privacy_level.clone(),
                max_concurrent_usage: 1,
            };
            vm_allocations.insert(key.clone(), asset_allocation);
        }

        Ok(vm_allocations)
    }

    async fn cleanup_asset_allocations(
        &self,
        allocations: &HashMap<String, EntityAssetAllocation>,
    ) -> Result<()> {
        for allocation in allocations.values() {
            self.asset_coordinator
                .release_allocation(&allocation.allocation_id)
                .await?;
        }
        Ok(())
    }
}

