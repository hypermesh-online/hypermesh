//! Consensus Operations - VM-native proof operations
//!
//! This module defines VM operations that are inherently consensus-aware.
//! Every operation carries its consensus proof and executes through the
//! four-proof validation system as a native language construct.

use std::sync::Arc;
use std::time::SystemTime;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::consensus::proof::ConsensusProof;
use super::{VMConsensusContext, ConsensusExecutionResult, ProofValidationResults, ResourceUsageMetrics};
use crate::catalog::vm::AssetId;

/// A consensus-native VM operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusOperation {
    /// Unique operation identifier
    id: String,
    /// Type of operation (load, store, compute, sync, etc.)
    operation_type: String,
    /// Asset being operated on
    asset_id: AssetId,
    /// Consensus proof for this operation
    consensus_proof: ConsensusProof,
    /// Operation creation timestamp
    created_at: SystemTime,
    /// Operation state
    state: OperationState,
    /// Execution metadata
    metadata: OperationMetadata,
}

/// Operation state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationState {
    /// Operation created but not yet validated
    Created,
    /// Consensus proof validated
    Validated,
    /// Currently executing
    Executing,
    /// Execution completed successfully
    Completed(ConsensusExecutionResult),
    /// Execution failed
    Failed(String),
}

/// Operation metadata for tracking and optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetadata {
    /// Priority level (0-100)
    pub priority: u8,
    /// Expected resource usage
    pub expected_resources: ResourceRequirements,
    /// Execution timeout (microseconds)
    pub timeout_micros: u64,
    /// Operation dependencies
    pub dependencies: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Resource requirements for operation planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cycles: u64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_bytes: u64,
    pub gpu_required: bool,
}

impl ConsensusOperation {
    /// Create new consensus operation
    pub fn new(
        operation_type: String,
        asset_id: AssetId,
        consensus_proof: ConsensusProof,
        _context: Arc<VMConsensusContext>,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        
        Self {
            id,
            operation_type,
            asset_id,
            consensus_proof,
            created_at: SystemTime::now(),
            state: OperationState::Created,
            metadata: OperationMetadata::default(),
        }
    }
    
    /// Execute the operation with consensus validation
    pub async fn execute(&self, execution_data: &[u8]) -> Result<ConsensusExecutionResult> {
        let start_time = SystemTime::now();
        
        // Validate consensus proof
        let proof_valid = self.consensus_proof.validate().await?;
        if !proof_valid {
            return Ok(ConsensusExecutionResult::failure(
                self.id.clone(),
                "Consensus proof validation failed".to_string(),
                self.create_proof_validation_results(false).await,
                ResourceUsageMetrics::minimal(),
            ));
        }
        
        // Execute based on operation type
        let execution_result = match self.operation_type.as_str() {
            "load" => self.execute_load(execution_data).await,
            "store" => self.execute_store(execution_data).await,
            "compute" => self.execute_compute(execution_data).await,
            "sync" => self.execute_sync(execution_data).await,
            _ => self.execute_generic(execution_data).await,
        };
        
        let execution_duration = start_time.elapsed()
            .unwrap_or_default()
            .as_micros() as u64;
        
        match execution_result {
            Ok(output) => {
                let proof_validations = self.create_proof_validation_results(true).await;
                let resource_usage = ResourceUsageMetrics {
                    cpu_cycles: self.estimate_cpu_usage(&self.operation_type),
                    memory_bytes: execution_data.len() as u64,
                    storage_bytes: if self.operation_type == "store" { execution_data.len() as u64 } else { 0 },
                    network_bytes: 0, // TODO: Track actual network usage
                    execution_duration_micros: execution_duration,
                };
                
                Ok(ConsensusExecutionResult::success(
                    self.id.clone(),
                    Some(output),
                    proof_validations,
                    resource_usage,
                ))
            }
            Err(e) => {
                let proof_validations = self.create_proof_validation_results(true).await;
                let resource_usage = ResourceUsageMetrics {
                    cpu_cycles: self.estimate_cpu_usage(&self.operation_type),
                    memory_bytes: execution_data.len() as u64,
                    storage_bytes: 0,
                    network_bytes: 0,
                    execution_duration_micros: execution_duration,
                };
                
                Ok(ConsensusExecutionResult::failure(
                    self.id.clone(),
                    e.to_string(),
                    proof_validations,
                    resource_usage,
                ))
            }
        }
    }
    
    /// Execute load operation with space proof validation
    async fn execute_load(&self, _data: &[u8]) -> Result<serde_json::Value> {
        // Validate Proof of Space is present and valid
        let space_valid = self.consensus_proof.proof_of_space.validate().await?;
        if !space_valid {
            return Err(anyhow::anyhow!("Space proof validation failed for load operation"));
        }
        
        // Simulate loading asset data based on space proof location
        let location = &self.consensus_proof.proof_of_space.storage_location;
        
        Ok(serde_json::json!({
            "operation": "load",
            "asset_id": self.asset_id,
            "location": location,
            "size": self.consensus_proof.proof_of_space.committed_space,
            "loaded_at": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs()
        }))
    }
    
    /// Execute store operation with stake proof validation
    async fn execute_store(&self, data: &[u8]) -> Result<serde_json::Value> {
        // Validate Proof of Stake for write permissions
        let stake_valid = self.consensus_proof.proof_of_stake.validate().await?;
        if !stake_valid {
            return Err(anyhow::anyhow!("Stake proof validation failed for store operation"));
        }
        
        // Check write permissions
        use crate::consensus::proof::AccessLevel;
        let write_level = &self.consensus_proof.proof_of_stake.permissions.write_level;
        if matches!(write_level, AccessLevel::None) {
            return Err(anyhow::anyhow!("Insufficient write permissions for store operation"));
        }
        
        // Simulate storing data
        Ok(serde_json::json!({
            "operation": "store",
            "asset_id": self.asset_id,
            "data_size": data.len(),
            "stake_holder": self.consensus_proof.proof_of_stake.stake_holder,
            "authority_level": self.consensus_proof.proof_of_stake.authority_level,
            "stored_at": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs()
        }))
    }
    
    /// Execute compute operation with work proof validation
    async fn execute_compute(&self, data: &[u8]) -> Result<serde_json::Value> {
        // Validate Proof of Work for computational resources
        let work_valid = self.consensus_proof.proof_of_work.validate().await?;
        if !work_valid {
            return Err(anyhow::anyhow!("Work proof validation failed for compute operation"));
        }
        
        // Validate difficulty meets minimum requirements
        if self.consensus_proof.proof_of_work.difficulty < 16 {
            return Err(anyhow::anyhow!("Insufficient work difficulty for compute operation"));
        }
        
        // Simulate computation based on work proof
        let computation_result = self.simulate_computation(data)?;
        
        Ok(serde_json::json!({
            "operation": "compute",
            "asset_id": self.asset_id,
            "input_size": data.len(),
            "result": computation_result,
            "difficulty": self.consensus_proof.proof_of_work.difficulty,
            "resource_type": self.consensus_proof.proof_of_work.resource_type,
            "computed_at": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs()
        }))
    }
    
    /// Execute sync operation with time proof validation
    async fn execute_sync(&self, _data: &[u8]) -> Result<serde_json::Value> {
        // Validate Proof of Time for temporal consistency
        let time_valid = self.consensus_proof.proof_of_time.validate().await?;
        if !time_valid {
            return Err(anyhow::anyhow!("Time proof validation failed for sync operation"));
        }
        
        // Check time drift is within acceptable bounds
        let current_time = SystemTime::now();
        let proof_time = self.consensus_proof.proof_of_time.wall_clock;
        let time_diff = current_time.duration_since(proof_time)
            .or_else(|_| proof_time.duration_since(current_time))?;
        
        if time_diff.as_micros() > 1_000_000 { // 1 second max drift
            return Err(anyhow::anyhow!("Time drift too large for sync operation"));
        }
        
        Ok(serde_json::json!({
            "operation": "sync",
            "asset_id": self.asset_id,
            "logical_timestamp": self.consensus_proof.proof_of_time.logical_timestamp,
            "sequence_number": self.consensus_proof.proof_of_time.sequence_number,
            "time_drift_micros": time_diff.as_micros(),
            "synced_at": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs()
        }))
    }
    
    /// Execute generic operation requiring all four proofs
    async fn execute_generic(&self, data: &[u8]) -> Result<serde_json::Value> {
        // Validate all four proofs for generic operations
        let all_valid = self.consensus_proof.validate().await?;
        if !all_valid {
            return Err(anyhow::anyhow!("Full consensus validation failed for generic operation"));
        }
        
        Ok(serde_json::json!({
            "operation": self.operation_type,
            "asset_id": self.asset_id,
            "data_size": data.len(),
            "full_consensus_validated": true,
            "executed_at": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs()
        }))
    }
    
    /// Simulate computation for work proof validation
    fn simulate_computation(&self, data: &[u8]) -> Result<String> {
        use sha2::{Sha256, Digest};
        
        // Simple computation simulation - hash the data multiple times
        // based on work proof difficulty
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(&self.consensus_proof.proof_of_work.nonce.to_le_bytes());
        
        let mut result = hasher.finalize().to_vec();
        
        // Additional computational work based on difficulty
        for _ in 0..self.consensus_proof.proof_of_work.difficulty {
            hasher = Sha256::new();
            hasher.update(&result);
            result = hasher.finalize().to_vec();
        }
        
        Ok(format!("{:x}", result.iter().take(8).fold(0u64, |acc, &b| acc * 256 + b as u64)))
    }
    
    /// Create proof validation results
    async fn create_proof_validation_results(&self, overall_valid: bool) -> ProofValidationResults {
        // In a real implementation, these would be actual validation results
        ProofValidationResults {
            space_proof_valid: overall_valid && self.consensus_proof.proof_of_space.validate().await.unwrap_or(false),
            stake_proof_valid: overall_valid && self.consensus_proof.proof_of_stake.validate().await.unwrap_or(false),
            work_proof_valid: overall_valid && self.consensus_proof.proof_of_work.validate().await.unwrap_or(false),
            time_proof_valid: overall_valid && self.consensus_proof.proof_of_time.validate().await.unwrap_or(false),
            combined_proof_hash_valid: overall_valid,
            validation_timestamp: SystemTime::now(),
        }
    }
    
    /// Estimate CPU usage for operation type
    fn estimate_cpu_usage(&self, operation_type: &str) -> u64 {
        match operation_type {
            "load" => 1000,
            "store" => 2000,
            "compute" => (self.consensus_proof.proof_of_work.difficulty as u64) * 1000,
            "sync" => 500,
            _ => 1500,
        }
    }
    
    /// Get operation ID
    pub fn id(&self) -> &str {
        &self.id
    }
    
    /// Get operation type
    pub fn operation_type(&self) -> &str {
        &self.operation_type
    }
    
    /// Get asset ID
    pub fn asset_id(&self) -> AssetId {
        self.asset_id
    }
    
    /// Get consensus proof
    pub fn consensus_proof(&self) -> &ConsensusProof {
        &self.consensus_proof
    }
    
    /// Get current state
    pub fn state(&self) -> &OperationState {
        &self.state
    }
    
    /// Update operation state
    pub fn set_state(&mut self, state: OperationState) {
        self.state = state;
    }
    
    /// Get operation metadata
    pub fn metadata(&self) -> &OperationMetadata {
        &self.metadata
    }
    
    /// Set operation metadata
    pub fn set_metadata(&mut self, metadata: OperationMetadata) {
        self.metadata = metadata;
    }
}

impl Default for OperationMetadata {
    fn default() -> Self {
        Self {
            priority: 50,
            expected_resources: ResourceRequirements::default(),
            timeout_micros: 30_000_000, // 30 seconds
            dependencies: Vec::new(),
            tags: Vec::new(),
        }
    }
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_cycles: 10_000,
            memory_bytes: 1024 * 1024, // 1MB
            storage_bytes: 0,
            network_bytes: 1024, // 1KB
            gpu_required: false,
        }
    }
}

impl ResourceUsageMetrics {
    /// Create minimal resource usage for failed operations
    pub fn minimal() -> Self {
        Self {
            cpu_cycles: 100,
            memory_bytes: 1024,
            storage_bytes: 0,
            network_bytes: 0,
            execution_duration_micros: 0,
        }
    }
}

/// Operation builder for creating consensus operations with specific requirements
pub struct ConsensusOperationBuilder {
    operation_type: String,
    asset_id: Option<AssetId>,
    consensus_proof: Option<ConsensusProof>,
    metadata: OperationMetadata,
}

impl ConsensusOperationBuilder {
    /// Create new operation builder
    pub fn new(operation_type: String) -> Self {
        Self {
            operation_type,
            asset_id: None,
            consensus_proof: None,
            metadata: OperationMetadata::default(),
        }
    }
    
    /// Set asset ID
    pub fn with_asset_id(mut self, asset_id: AssetId) -> Self {
        self.asset_id = Some(asset_id);
        self
    }
    
    /// Set consensus proof
    pub fn with_consensus_proof(mut self, proof: ConsensusProof) -> Self {
        self.consensus_proof = Some(proof);
        self
    }
    
    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.metadata.priority = priority;
        self
    }
    
    /// Set timeout
    pub fn with_timeout_micros(mut self, timeout: u64) -> Self {
        self.metadata.timeout_micros = timeout;
        self
    }
    
    /// Add dependency
    pub fn with_dependency(mut self, dep: String) -> Self {
        self.metadata.dependencies.push(dep);
        self
    }
    
    /// Add tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.metadata.tags.push(tag);
        self
    }
    
    /// Build the consensus operation
    pub fn build(self, context: Arc<VMConsensusContext>) -> Result<ConsensusOperation> {
        let asset_id = self.asset_id.unwrap_or_else(|| Uuid::new_v4());
        let consensus_proof = self.consensus_proof
            .ok_or_else(|| anyhow::anyhow!("Consensus proof is required"))?;
        
        let mut operation = ConsensusOperation::new(
            self.operation_type,
            asset_id,
            consensus_proof,
            context,
        );
        
        operation.set_metadata(self.metadata);
        
        Ok(operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::proof::{ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime};
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_consensus_operation_creation() {
        let context = Arc::new(VMConsensusContext::new());
        let consensus_proof = ConsensusProof::new(
            ProofOfSpace::default(),
            ProofOfStake::default(),
            ProofOfWork::default(),
            ProofOfTime::default(),
        );
        
        let operation = ConsensusOperation::new(
            "test".to_string(),
            Uuid::new_v4(),
            consensus_proof,
            context,
        );
        
        assert_eq!(operation.operation_type(), "test");
        assert!(matches!(operation.state(), OperationState::Created));
    }
    
    #[tokio::test]
    async fn test_operation_builder() {
        let context = Arc::new(VMConsensusContext::new());
        let consensus_proof = ConsensusProof::new(
            ProofOfSpace::default(),
            ProofOfStake::default(),
            ProofOfWork::default(),
            ProofOfTime::default(),
        );
        
        let operation = ConsensusOperationBuilder::new("test".to_string())
            .with_consensus_proof(consensus_proof)
            .with_priority(75)
            .with_tag("test-tag".to_string())
            .build(context);
        
        assert!(operation.is_ok());
        let op = operation.unwrap();
        assert_eq!(op.metadata().priority, 75);
        assert!(op.metadata().tags.contains(&"test-tag".to_string()));
    }
}