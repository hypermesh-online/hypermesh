//! Catalog VM Matrix Chain Integration
//!
//! Integration layer for the Catalog VM system with the new matrix chain architecture.
//! Each entity has its own blockchain, and VM operations can:
//! 1. Execute on specific entity chains
//! 2. Validate compute assets across entity chains
//! 3. Respect entity privacy policies during execution
//! 4. Support multi-entity workflows spanning multiple chains
//! 5. Request asset allocation from specific entity blockchains
//!
//! Based on Proof of State patterns adapted for the HyperMesh matrix architecture.

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use async_trait::async_trait;

use crate::assets::matrix_blockchain::{
    MatrixBlockchainManager, EntityBlockchain, EntityConfig, EntityType,
    ValidationRequest, PublicValidationResponse, ValidationResult,
    MatrixCoordinate, PrivacyPolicyConfig, AssetPrivacyLevel
};
use crate::assets::blockchain::HyperMeshAssetRecord;
use crate::consensus::ConsensusProof;
use super::{
    ConsensusProofVM, VMConfig, ExecutionResult, ExecutionContext,
    PrivacyLevel, AssetAllocation, AssetId
};
use super::consensus::ConsensusVM;
use super::execution::VMExecutor;
use super::languages::MultiLanguageSupport;

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
    pub asset_id: AssetId,
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
struct CachedValidation {
    validation_request: CrossEntityValidation,
    validation_response: PublicValidationResponse,
    cached_at: std::time::SystemTime,
    expires_at: std::time::SystemTime,
}

/// Entity asset allocation coordinator
pub struct EntityAssetCoordinator {
    /// Available assets per entity
    entity_assets: Arc<std::sync::Mutex<HashMap<String, EntityAssetPool>>>,
    /// Active asset allocations
    active_allocations: Arc<std::sync::Mutex<HashMap<String, Vec<ActiveAllocation>>>>,
    /// Asset request queue
    request_queue: Arc<std::sync::Mutex<Vec<EntityAssetRequest>>>,
}

/// Available assets for an entity
#[derive(Debug, Clone)]
struct EntityAssetPool {
    cpu_available: u64,
    gpu_available: u64,
    memory_available: u64,
    storage_available: u64,
    privacy_constraints: EntityPrivacyConstraints,
}

/// Active asset allocation tracking
#[derive(Debug, Clone)]
struct ActiveAllocation {
    allocation_id: Uuid,
    entity_domain: String,
    asset_type: String,
    allocated_amount: u64,
    start_time: std::time::SystemTime,
    expires_at: std::time::SystemTime,
    executing_workflow: Option<String>,
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
        // Update asset coordinator with entity's available resources
        self.asset_coordinator.update_entity_pool(
            &entity_domain,
            &config,
        )?;
        
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
        // Validate consensus proof against target entity chain
        self.validate_consensus_against_entity(&context).await?;
        
        // Perform cross-entity validations
        let validation_results = self.perform_cross_entity_validations(&context).await?;
        
        // Allocate assets from specified entities
        let asset_allocations = self.allocate_entity_assets(&context).await?;
        
        // Execute multi-entity workflow if specified
        let execution_result = if let Some(workflow) = &context.workflow_config {
            self.execute_multi_entity_workflow(
                code,
                language,
                &context,
                workflow,
                &asset_allocations,
            ).await?
        } else {
            // Single entity execution
            self.execute_on_target_entity(
                code,
                language,
                &context,
                &asset_allocations,
            ).await?
        };
        
        // Clean up asset allocations
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
    
    /// Validate consensus proof against target entity chain
    async fn validate_consensus_against_entity(
        &self,
        context: &MatrixExecutionContext,
    ) -> Result<()> {
        if let Some(target_entity) = &context.target_entity {
            // Get entity blockchain for validation
            if let Some(entity_config) = self.entity_configs.get(target_entity) {
                // Validate that consensus proof meets entity-specific requirements
                let consensus_proof = &context.base_context.consensus_proof;
                
                if !self.base_vm.consensus_vm()
                    .validate_consensus_proof(consensus_proof)
                    .await? {
                    return Err(anyhow::anyhow!(
                        "Consensus proof validation failed for entity: {}", 
                        target_entity
                    ));
                }
                
                // Entity-specific validation logic would go here
                // e.g., checking entity's minimum proof requirements
            }
        }
        Ok(())
    }
    
    /// Perform cross-entity validations
    async fn perform_cross_entity_validations(
        &self,
        context: &MatrixExecutionContext,
    ) -> Result<HashMap<String, PublicValidationResponse>> {
        let mut results = HashMap::new();
        
        for validation in &context.cross_entity_validations {
            // Check cache first
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
            
            // Perform validation through matrix manager
            let validation_request = ValidationRequest {
                asset_id: validation.asset_id,
                requested_fields: validation.validation_fields.clone(),
                validation_type: self.convert_validation_type(&validation.validation_type),
                requester: self.create_entity_identifier(&context.base_context)?,
                proof_requirements: vec![], // Would be populated based on requirements
            };
            
            let validation_results = self.matrix_manager
                .multi_entity_validation(validation.asset_id, vec![validation.entity_domain.clone()])
                .await?;
            
            if let Some(response) = validation_results.get(&validation.entity_domain) {
                results.insert(validation.entity_domain.clone(), response.clone());
                
                // Cache the result
                self.cache_validation(cache_key, validation.clone(), response.clone());
            }
        }
        
        Ok(results)
    }
    
    /// Allocate assets from specified entities
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
    
    /// Execute multi-entity workflow
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
        
        // Execute workflow steps according to entity sequence
        for (step_index, entity_domain) in workflow.entity_sequence.iter().enumerate() {
            // Check synchronization requirements
            self.check_sync_requirements(workflow, step_index, &intermediate_data).await?;
            
            // Create entity-specific execution context
            let entity_context = self.create_entity_execution_context(
                context,
                entity_domain,
                asset_allocations,
                &intermediate_data,
            )?;
            
            // Execute on this entity
            let step_result = self.execute_on_target_entity(
                code,
                language,
                &MatrixExecutionContext {
                    target_entity: Some(entity_domain.clone()),
                    ..context.clone()
                },
                asset_allocations,
            ).await?;
            
            // Store intermediate results based on workflow privacy policy
            if workflow.workflow_privacy.intermediate_access.contains(entity_domain) {
                if let Some(output) = &step_result.output {
                    intermediate_data.insert(entity_domain.clone(), output.clone());
                }
            }
            
            workflow_results.push((entity_domain.clone(), step_result));
        }
        
        // Aggregate results according to workflow configuration
        self.aggregate_workflow_results(workflow_results, workflow).await
    }
    
    /// Execute on target entity
    async fn execute_on_target_entity(
        &self,
        code: &str,
        language: &str,
        context: &MatrixExecutionContext,
        asset_allocations: &HashMap<String, EntityAssetAllocation>,
    ) -> Result<ExecutionResult> {
        // Create enhanced execution context with entity assets
        let enhanced_context = ExecutionContext {
            asset_allocations: self.convert_entity_allocations_to_vm_allocations(asset_allocations)?,
            ..context.base_context.clone()
        };
        
        // Get consensus proof from context
        let consensus_proof = context.base_context.consensus_proof.clone();
        
        // Execute through base VM with enhanced context
        self.base_vm.execute_with_consensus(code, language, consensus_proof).await
    }
    
    /// Get entity interactions summary
    async fn get_entity_interactions(
        &self,
        context: &MatrixExecutionContext,
    ) -> Result<Vec<EntityInteraction>> {
        let mut interactions = Vec::new();
        
        // Record cross-entity validations as interactions
        for validation in &context.cross_entity_validations {
            interactions.push(EntityInteraction {
                interaction_type: InteractionType::Validation,
                source_entity: context.target_entity.clone().unwrap_or_default(),
                target_entity: validation.entity_domain.clone(),
                asset_id: Some(validation.asset_id),
                timestamp: std::time::SystemTime::now(),
                privacy_level: validation.privacy_level.clone(),
            });
        }
        
        // Record asset requests as interactions
        for request in &context.entity_asset_requests {
            interactions.push(EntityInteraction {
                interaction_type: InteractionType::AssetRequest,
                source_entity: context.target_entity.clone().unwrap_or_default(),
                target_entity: request.entity_domain.clone(),
                asset_id: None,
                timestamp: std::time::SystemTime::now(),
                privacy_level: PrivacyLevel::P2P, // Default for asset requests
            });
        }
        
        Ok(interactions)
    }
    
    // Helper methods
    
    fn get_cached_validation(&self, cache_key: &str) -> Option<CachedValidation> {
        self.validation_cache.lock().unwrap().get(cache_key).cloned()
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
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(300), // 5 min cache
        };
        
        self.validation_cache.lock().unwrap().insert(cache_key, cached);
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
        context: &ExecutionContext,
    ) -> Result<crate::assets::matrix_blockchain::EntityIdentifier> {
        Ok(crate::assets::matrix_blockchain::EntityIdentifier {
            network_domain: "vm.hypermesh.online".to_string(),
            entity_type: EntityType::Organization("VM".to_string()),
            certificate_fingerprint: "vm-cert-placeholder".to_string(),
        })
    }
    
    async fn check_sync_requirements(
        &self,
        workflow: &MultiEntityWorkflow,
        step_index: usize,
        intermediate_data: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // Implementation would check synchronization requirements
        // between workflow steps
        Ok(())
    }
    
    fn create_entity_execution_context(
        &self,
        base_context: &MatrixExecutionContext,
        entity_domain: &str,
        asset_allocations: &HashMap<String, EntityAssetAllocation>,
        intermediate_data: &HashMap<String, serde_json::Value>,
    ) -> Result<ExecutionContext> {
        // Create entity-specific execution context
        Ok(base_context.base_context.clone())
    }
    
    async fn aggregate_workflow_results(
        &self,
        workflow_results: Vec<(String, ExecutionResult)>,
        workflow: &MultiEntityWorkflow,
    ) -> Result<ExecutionResult> {
        // Aggregate results from multiple entities according to workflow policy
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
    pub asset_id: Option<AssetId>,
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

impl EntityAssetCoordinator {
    pub fn new() -> Self {
        Self {
            entity_assets: Arc::new(std::sync::Mutex::new(HashMap::new())),
            active_allocations: Arc::new(std::sync::Mutex::new(HashMap::new())),
            request_queue: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
    
    pub fn update_entity_pool(
        &self,
        entity_domain: &str,
        config: &EntityVMConfig,
    ) -> Result<()> {
        let pool = EntityAssetPool {
            cpu_available: config.max_external_allocation.get("cpu").copied().unwrap_or(0),
            gpu_available: config.max_external_allocation.get("gpu").copied().unwrap_or(0),
            memory_available: config.max_external_allocation.get("memory").copied().unwrap_or(0),
            storage_available: config.max_external_allocation.get("storage").copied().unwrap_or(0),
            privacy_constraints: EntityPrivacyConstraints {
                entity_domain: entity_domain.to_string(),
                max_compute_allocation: config.max_external_allocation.clone(),
                allowed_operations: vec![], // Would be populated from config
                resource_privacy_level: PrivacyLevel::P2P,
                max_duration_seconds: 3600,
            },
        };
        
        self.entity_assets.lock().unwrap()
            .insert(entity_domain.to_string(), pool);
        
        Ok(())
    }
    
    pub async fn allocate_asset_from_entity(
        &self,
        request: &EntityAssetRequest,
    ) -> Result<EntityAssetAllocation> {
        // Check if entity has available assets
        let mut entity_assets = self.entity_assets.lock().unwrap();
        let pool = entity_assets.get_mut(&request.entity_domain)
            .ok_or_else(|| anyhow::anyhow!("Entity not found: {}", request.entity_domain))?;
        
        // Check availability based on asset type
        let available = match request.asset_type.as_str() {
            "cpu" => pool.cpu_available,
            "gpu" => pool.gpu_available,
            "memory" => pool.memory_available,
            "storage" => pool.storage_available,
            _ => return Err(anyhow::anyhow!("Unknown asset type: {}", request.asset_type)),
        };
        
        if available < request.requested_amount {
            return Err(anyhow::anyhow!(
                "Insufficient {} available from {}: requested {}, available {}",
                request.asset_type, request.entity_domain, request.requested_amount, available
            ));
        }
        
        // Allocate asset
        let allocation_id = Uuid::new_v4();
        let allocation = EntityAssetAllocation {
            allocation_id,
            entity_domain: request.entity_domain.clone(),
            asset_type: request.asset_type.clone(),
            allocated_capacity: request.requested_amount,
            total_capacity: available,
            privacy_level: pool.privacy_constraints.resource_privacy_level.clone(),
            expires_at: std::time::SystemTime::now() + 
                std::time::Duration::from_secs(request.duration_seconds),
        };
        
        // Update available capacity
        match request.asset_type.as_str() {
            "cpu" => pool.cpu_available -= request.requested_amount,
            "gpu" => pool.gpu_available -= request.requested_amount,
            "memory" => pool.memory_available -= request.requested_amount,
            "storage" => pool.storage_available -= request.requested_amount,
            _ => {},
        }
        
        // Track active allocation
        let active_allocation = ActiveAllocation {
            allocation_id,
            entity_domain: request.entity_domain.clone(),
            asset_type: request.asset_type.clone(),
            allocated_amount: request.requested_amount,
            start_time: std::time::SystemTime::now(),
            expires_at: allocation.expires_at,
            executing_workflow: None,
        };
        
        self.active_allocations.lock().unwrap()
            .entry(request.entity_domain.clone())
            .or_insert_with(Vec::new)
            .push(active_allocation);
        
        Ok(allocation)
    }
    
    pub async fn release_allocation(&self, allocation_id: &Uuid) -> Result<()> {
        let mut active_allocations = self.active_allocations.lock().unwrap();
        
        // Find and remove the allocation
        for (entity_domain, allocations) in active_allocations.iter_mut() {
            if let Some(pos) = allocations.iter().position(|a| &a.allocation_id == allocation_id) {
                let allocation = allocations.remove(pos);
                
                // Return capacity to entity pool
                let mut entity_assets = self.entity_assets.lock().unwrap();
                if let Some(pool) = entity_assets.get_mut(entity_domain) {
                    match allocation.asset_type.as_str() {
                        "cpu" => pool.cpu_available += allocation.allocated_amount,
                        "gpu" => pool.gpu_available += allocation.allocated_amount,
                        "memory" => pool.memory_available += allocation.allocated_amount,
                        "storage" => pool.storage_available += allocation.allocated_amount,
                        _ => {},
                    }
                }
                
                return Ok(());
            }
        }
        
        Err(anyhow::anyhow!("Allocation not found: {}", allocation_id))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::vm::{ConsensusRequirements, VMConfig};
    use crate::catalog::vm::consensus::ConsensusVM;

    #[tokio::test]
    async fn test_matrix_aware_vm_creation() {
        let vm_config = VMConfig::default();
        let base_vm = Arc::new(ConsensusProofVM::new(vm_config).await.unwrap());
        let matrix_manager = Arc::new(MatrixBlockchainManager::new());
        
        let matrix_vm = MatrixAwareVM::new(base_vm, matrix_manager).await;
        assert!(matrix_vm.is_ok());
    }
    
    #[test]
    fn test_cross_entity_validation_creation() {
        let validation = CrossEntityValidation {
            entity_domain: "honda.hypermesh.online".to_string(),
            asset_id: Uuid::new_v4(),
            validation_fields: vec!["vin".to_string(), "model".to_string()],
            validation_type: ValidationRequirementType::AssetExists,
            privacy_level: PrivacyLevel::P2P,
        };
        
        assert_eq!(validation.entity_domain, "honda.hypermesh.online");
        assert_eq!(validation.validation_fields.len(), 2);
    }
    
    #[test]
    fn test_entity_asset_request_creation() {
        let request = EntityAssetRequest {
            entity_domain: "dealer.hypermesh.online".to_string(),
            asset_type: "cpu".to_string(),
            requested_amount: 2,
            duration_seconds: 3600,
            compensation_tokens: 100,
            priority: AssetRequestPriority::Normal,
        };
        
        assert_eq!(request.entity_domain, "dealer.hypermesh.online");
        assert_eq!(request.requested_amount, 2);
    }
    
    #[test]
    fn test_multi_entity_workflow_creation() {
        let workflow = MultiEntityWorkflow {
            entity_sequence: vec![
                "honda.hypermesh.online".to_string(),
                "dealer.hypermesh.online".to_string(),
                "bank.hypermesh.online".to_string(),
            ],
            data_flow: HashMap::new(),
            sync_requirements: vec![
                EntitySyncRequirement {
                    source_entity: "honda.hypermesh.online".to_string(),
                    target_entity: "dealer.hypermesh.online".to_string(),
                    sync_type: SyncType::Sequential,
                    max_delay_micros: 1000000,
                }
            ],
            workflow_privacy: WorkflowPrivacyPolicy {
                intermediate_privacy: PrivacyLevel::P2P,
                final_privacy: PrivacyLevel::PublicNetwork,
                intermediate_access: vec!["dealer.hypermesh.online".to_string()],
                data_sharing_rules: HashMap::new(),
            },
        };
        
        assert_eq!(workflow.entity_sequence.len(), 3);
        assert_eq!(workflow.sync_requirements.len(), 1);
    }
    
    #[tokio::test]
    async fn test_entity_asset_coordinator() {
        let coordinator = EntityAssetCoordinator::new();
        
        let config = EntityVMConfig {
            entity_domain: "test.hypermesh.online".to_string(),
            entity_type: EntityType::Organization("Test".to_string()),
            vm_config: VMConfig::default(),
            privacy_policies: PrivacyPolicyConfig {
                public_fields: vec![],
                federated_fields: HashMap::new(),
                zk_proof_fields: vec![],
                default_privacy_level: AssetPrivacyLevel::Private,
            },
            trusted_partners: vec![],
            max_external_allocation: {
                let mut alloc = HashMap::new();
                alloc.insert("cpu".to_string(), 4);
                alloc.insert("memory".to_string(), 8192);
                alloc
            },
        };
        
        assert!(coordinator.update_entity_pool("test.hypermesh.online", &config).is_ok());
        
        let request = EntityAssetRequest {
            entity_domain: "test.hypermesh.online".to_string(),
            asset_type: "cpu".to_string(),
            requested_amount: 2,
            duration_seconds: 3600,
            compensation_tokens: 100,
            priority: AssetRequestPriority::Normal,
        };
        
        let allocation = coordinator.allocate_asset_from_entity(&request).await;
        assert!(allocation.is_ok());
        
        if let Ok(allocation) = allocation {
            assert_eq!(allocation.allocated_capacity, 2);
            assert!(coordinator.release_allocation(&allocation.allocation_id).await.is_ok());
        }
    }
}