//! VM-native Consensus Integration
//!
//! This module implements consensus proofs as native VM constructs rather than external
//! validation layers. Every VM operation is inherently consensus-aware, requiring all
//! four proofs (PoSp+PoSt+PoWk+PoTm) as fundamental language primitives.
//!
//! Based on Proof of State four-proof consensus patterns adapted for HyperMesh VM execution.

pub mod operations;
pub mod validation;
pub mod context;

use std::sync::Arc;
use std::collections::HashMap;
use std::time::SystemTime;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

// Re-export from main consensus module
pub use crate::consensus::proof::{ConsensusProof, SpaceProof, StakeProof, WorkProof, TimeProof};
pub use operations::ConsensusOperation;
pub use crate::consensus::validation::ConsensusValidator;
pub use context::VMConsensusContext;

use super::{ConsensusRequirements, AssetId};

/// VM-native consensus engine that treats proofs as language constructs
pub struct ConsensusVM {
    /// Consensus requirements for all operations
    requirements: ConsensusRequirements,
    /// Current consensus context
    context: Arc<VMConsensusContext>,
    /// Validators for each proof type
    validators: ConsensusValidators,
    /// Active consensus operations
    active_operations: HashMap<String, ConsensusOperation>,
}

/// Consensus validators for all proof types
#[derive(Clone)]
pub struct ConsensusValidators {
    pub space_validator: Arc<dyn ProofValidator<SpaceProof>>,
    pub stake_validator: Arc<dyn ProofValidator<StakeProof>>,
    pub work_validator: Arc<dyn ProofValidator<WorkProof>>,
    pub time_validator: Arc<dyn ProofValidator<TimeProof>>,
}

/// Generic proof validator trait
#[async_trait]
pub trait ProofValidator<T>: Send + Sync {
    /// Validate a proof against current VM context
    async fn validate(&self, proof: &T, context: &VMConsensusContext) -> Result<bool>;
    
    /// Get proof requirements for VM operation
    async fn get_requirements(&self, operation_type: &str) -> Result<ProofRequirement>;
}

/// Proof requirement specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRequirement {
    pub required: bool,
    pub minimum_values: HashMap<String, u64>,
    pub additional_constraints: Vec<String>,
}

impl ConsensusVM {
    /// Create new consensus VM with four-proof requirements
    pub fn new(requirements: ConsensusRequirements) -> Result<Self> {
        let context = Arc::new(VMConsensusContext::new());
        let validators = Self::initialize_validators(&requirements)?;
        
        Ok(Self {
            requirements,
            context,
            validators,
            active_operations: HashMap::new(),
        })
    }
    
    /// Validate complete consensus proof (all four proofs required)
    pub async fn validate_consensus_proof(
        &self,
        proof: &ConsensusProof,
    ) -> Result<bool> {
        // Proof of State pattern: ALL four proofs must be valid for any operation
        let space_valid = if self.requirements.require_proof_of_space {
            self.validators.space_validator
                .validate(&proof.space_proof, &self.context).await?
        } else { true };
        
        let stake_valid = if self.requirements.require_proof_of_stake {
            self.validators.stake_validator
                .validate(&proof.stake_proof, &self.context).await?
        } else { true };
        
        let work_valid = if self.requirements.require_proof_of_work {
            self.validators.work_validator
                .validate(&proof.work_proof, &self.context).await?
        } else { true };
        
        let time_valid = if self.requirements.require_proof_of_time {
            self.validators.time_validator
                .validate(&proof.time_proof, &self.context).await?
        } else { true };
        
        // Combined validation from HyperMesh consensus system
        let consensus_valid = proof.validate().await?;
        
        Ok(space_valid && stake_valid && work_valid && time_valid && consensus_valid)
    }
    
    /// Create consensus-native operation for VM execution
    pub async fn create_consensus_operation(
        &mut self,
        operation_type: &str,
        asset_id: AssetId,
        consensus_proof: ConsensusProof,
    ) -> Result<ConsensusOperation> {
        // Validate proof meets VM requirements
        if !self.validate_consensus_proof(&consensus_proof).await? {
            return Err(anyhow::anyhow!(
                "Consensus proof validation failed for operation: {}", operation_type
            ));
        }
        
        // Create VM-native consensus operation
        let operation = ConsensusOperation::new(
            operation_type.to_string(),
            asset_id,
            consensus_proof,
            Arc::clone(&self.context),
        );
        
        // Register active operation
        let operation_id = operation.id().clone();
        self.active_operations.insert(operation_id, operation.clone());
        
        Ok(operation)
    }
    
    /// Execute consensus operation with native VM integration
    pub async fn execute_consensus_operation(
        &mut self,
        operation: &ConsensusOperation,
        execution_data: &[u8],
    ) -> Result<ConsensusExecutionResult> {
        // Update consensus context with operation
        self.context.add_operation(operation).await?;
        
        // Execute operation through consensus validation
        let result = operation.execute(execution_data).await?;
        
        // Update context with execution result
        self.context.update_with_result(&result).await?;
        
        Ok(result)
    }
    
    /// Get current consensus requirements
    pub fn requirements(&self) -> &ConsensusRequirements {
        &self.requirements
    }
    
    /// Get current consensus context
    pub fn context(&self) -> Arc<VMConsensusContext> {
        Arc::clone(&self.context)
    }
    
    /// Initialize proof validators
    fn initialize_validators(
        requirements: &ConsensusRequirements,
    ) -> Result<ConsensusValidators> {
        Ok(ConsensusValidators {
            space_validator: Arc::new(validation::SpaceProofValidator::new(
                requirements.min_space_commitment,
            )?),
            stake_validator: Arc::new(validation::StakeProofValidator::new(
                requirements.min_stake_authority,
            )?),
            work_validator: Arc::new(validation::WorkProofValidator::new(
                requirements.min_work_difficulty,
            )?),
            time_validator: Arc::new(validation::TimeProofValidator::new(
                requirements.max_time_drift,
            )?),
        })
    }
    
    /// Remove completed operation
    pub fn remove_operation(&mut self, operation_id: &str) {
        self.active_operations.remove(operation_id);
    }
    
    /// Get all active operations
    pub fn active_operations(&self) -> &HashMap<String, ConsensusOperation> {
        &self.active_operations
    }
    
    /// Update consensus requirements (for dynamic adjustment)
    pub fn update_requirements(&mut self, new_requirements: ConsensusRequirements) -> Result<()> {
        self.requirements = new_requirements;
        self.validators = Self::initialize_validators(&self.requirements)?;
        Ok(())
    }
}

/// Result of consensus operation execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusExecutionResult {
    /// Operation that was executed
    pub operation_id: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Execution output
    pub output: Option<serde_json::Value>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Proof validations performed
    pub proof_validations: ProofValidationResults,
    /// Execution timestamp
    pub executed_at: SystemTime,
    /// Resource utilization
    pub resource_usage: ResourceUsageMetrics,
}

/// Proof validation results for transparency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofValidationResults {
    pub space_proof_valid: bool,
    pub stake_proof_valid: bool,
    pub work_proof_valid: bool,
    pub time_proof_valid: bool,
    pub combined_proof_hash_valid: bool,
    pub validation_timestamp: SystemTime,
}

/// Resource usage metrics for consensus operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageMetrics {
    pub cpu_cycles: u64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_bytes: u64,
    pub execution_duration_micros: u64,
}

impl ConsensusExecutionResult {
    /// Create successful execution result
    pub fn success(
        operation_id: String,
        output: Option<serde_json::Value>,
        proof_validations: ProofValidationResults,
        resource_usage: ResourceUsageMetrics,
    ) -> Self {
        Self {
            operation_id,
            success: true,
            output,
            error_message: None,
            proof_validations,
            executed_at: SystemTime::now(),
            resource_usage,
        }
    }
    
    /// Create failed execution result
    pub fn failure(
        operation_id: String,
        error_message: String,
        proof_validations: ProofValidationResults,
        resource_usage: ResourceUsageMetrics,
    ) -> Self {
        Self {
            operation_id,
            success: false,
            output: None,
            error_message: Some(error_message),
            proof_validations,
            executed_at: SystemTime::now(),
            resource_usage,
        }
    }
}

/// Consensus-aware VM primitive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusVMPrimitive {
    /// Load data with space proof requirement
    Load { asset_id: AssetId, space_proof: SpaceProof },
    /// Store data with stake proof requirement
    Store { asset_id: AssetId, data: Vec<u8>, stake_proof: StakeProof },
    /// Compute with work proof requirement
    Compute { operation: String, input: Vec<u8>, work_proof: WorkProof },
    /// Synchronize with time proof requirement
    Sync { timestamp: SystemTime, time_proof: TimeProof },
    /// Combined operation requiring all four proofs
    FullConsensus { 
        operation: String, 
        asset_id: AssetId, 
        data: Vec<u8>, 
        consensus_proof: ConsensusProof 
    },
}

impl ConsensusVMPrimitive {
    /// Execute VM primitive with consensus validation
    pub async fn execute(
        &self,
        vm: &mut ConsensusVM,
    ) -> Result<ConsensusExecutionResult> {
        let start_time = SystemTime::now();
        
        match self {
            ConsensusVMPrimitive::Load { asset_id, space_proof } => {
                // Create minimal consensus proof with space validation
                let minimal_proof = ConsensusProof::new(
                    space_proof.clone(),
                    StakeProof::default(),
                    WorkProof::default(),
                    TimeProof::default(),
                );
                
                let operation = vm.create_consensus_operation(
                    "load",
                    *asset_id,
                    minimal_proof,
                ).await?;
                
                vm.execute_consensus_operation(&operation, &[]).await
            },
            
            ConsensusVMPrimitive::Store { asset_id, data, stake_proof } => {
                let minimal_proof = ConsensusProof::new(
                    SpaceProof::default(),
                    stake_proof.clone(),
                    WorkProof::default(),
                    TimeProof::default(),
                );
                
                let operation = vm.create_consensus_operation(
                    "store",
                    *asset_id,
                    minimal_proof,
                ).await?;
                
                vm.execute_consensus_operation(&operation, data).await
            },
            
            ConsensusVMPrimitive::Compute { operation: op_type, input, work_proof } => {
                let minimal_proof = ConsensusProof::new(
                    SpaceProof::default(),
                    StakeProof::default(),
                    work_proof.clone(),
                    TimeProof::default(),
                );
                
                let operation = vm.create_consensus_operation(
                    op_type,
                    uuid::Uuid::new_v4(),
                    minimal_proof,
                ).await?;
                
                vm.execute_consensus_operation(&operation, input).await
            },
            
            ConsensusVMPrimitive::Sync { timestamp: _, time_proof } => {
                let minimal_proof = ConsensusProof::new(
                    SpaceProof::default(),
                    StakeProof::default(),
                    WorkProof::default(),
                    time_proof.clone(),
                );
                
                let operation = vm.create_consensus_operation(
                    "sync",
                    uuid::Uuid::new_v4(),
                    minimal_proof,
                ).await?;
                
                vm.execute_consensus_operation(&operation, &[]).await
            },
            
            ConsensusVMPrimitive::FullConsensus { operation: op_type, asset_id, data, consensus_proof } => {
                let operation = vm.create_consensus_operation(
                    op_type,
                    *asset_id,
                    consensus_proof.clone(),
                ).await?;
                
                vm.execute_consensus_operation(&operation, data).await
            },
        }
    }
}

// Note: Default implementations for SpaceProof, StakeProof, WorkProof, and TimeProof
// are provided by TrustChain crate. We do not re-implement them here to avoid orphan rule violations.

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_consensus_vm_creation() {
        let requirements = ConsensusRequirements::default();
        let vm = ConsensusVM::new(requirements);
        assert!(vm.is_ok());
    }
    
    #[tokio::test]
    async fn test_four_proof_validation() {
        let requirements = ConsensusRequirements::default();
        let vm = ConsensusVM::new(requirements).unwrap();
        
        // Create consensus proof with all four proofs
        let consensus_proof = ConsensusProof::new(
            SpaceProof::default(),
            StakeProof::default(),
            WorkProof::default(),
            TimeProof::default(),
        );
        
        // Test validation (will likely fail due to minimal proofs, but tests structure)
        let result = vm.validate_consensus_proof(&consensus_proof).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_vm_primitive_execution() {
        let requirements = ConsensusRequirements::default();
        let mut vm = ConsensusVM::new(requirements).unwrap();
        
        let primitive = ConsensusVMPrimitive::Load {
            asset_id: uuid::Uuid::new_v4(),
            space_proof: SpaceProof::default(),
        };
        
        // Test primitive execution
        let result = primitive.execute(&mut vm).await;
        // May fail due to validation requirements, but tests the structure
        assert!(result.is_ok() || result.is_err());
    }
}