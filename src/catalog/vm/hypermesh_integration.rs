//! HyperMesh Blockchain Integration for Catalog VM
//!
//! This module provides the connection between the Catalog VM execution layer
//! and the HyperMesh blockchain asset system. The VM requests assets from
//! HyperMesh and stores execution results back to the blockchain.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use hypermesh_assets::blockchain::{
    HyperMeshAssetRecord, AssetRecordType, ComputeExecutionRecord,
    ComputeResourceRequirements, ComputeExecutionResult, ActualResourceUsage,
    AssetPrivacyLevel, AssetBlockchainManager,
};
use hypermesh_assets::core::asset_id::{AssetId, AssetType};
use crate::consensus::ConsensusProof;

/// Catalog VM integration with HyperMesh blockchain
pub struct HyperMeshBlockchainClient {
    blockchain_manager: AssetBlockchainManager,
    node_identity: String,
}

impl HyperMeshBlockchainClient {
    /// Create new blockchain client for VM integration
    pub fn new(node_identity: String) -> Self {
        Self {
            blockchain_manager: AssetBlockchainManager::new(),
            node_identity,
        }
    }

    /// Request compute asset allocation from HyperMesh blockchain
    pub async fn request_compute_asset(
        &self,
        resource_requirements: ComputeResourceRequirements,
        privacy_level: AssetPrivacyLevel,
        consensus_proof: ConsensusProof,
    ) -> Result<AssetId, String> {
        // Create asset ID for compute resource
        let asset_type = if resource_requirements.gpu_required {
            AssetType::Gpu
        } else {
            AssetType::Cpu
        };
        
        let asset_id = AssetId::new(asset_type);

        // Create asset record for compute allocation request
        let allocation_data = serde_json::to_vec(&resource_requirements)
            .map_err(|e| format!("Failed to serialize requirements: {}", e))?;

        let asset_record = HyperMeshAssetRecord::new(
            asset_id.clone(),
            AssetRecordType::ComputeExecution,
            self.node_identity.clone(),
            allocation_data,
            vec![consensus_proof],
            privacy_level,
        );

        // Store allocation request in blockchain
        self.blockchain_manager
            .add_asset_record(asset_record)
            .await?;

        Ok(asset_id)
    }

    /// Store execution results back to HyperMesh blockchain
    pub async fn store_execution_result(
        &self,
        asset_id: AssetId,
        execution_result: ComputeExecutionResult,
        consensus_proof: ConsensusProof,
    ) -> Result<(), String> {
        // Serialize execution result
        let result_data = serde_json::to_vec(&execution_result)
            .map_err(|e| format!("Failed to serialize result: {}", e))?;

        // Create asset record for execution completion
        let completion_record = HyperMeshAssetRecord::new(
            asset_id,
            AssetRecordType::StatusUpdate,
            self.node_identity.clone(),
            result_data,
            vec![consensus_proof],
            AssetPrivacyLevel::FullPublic, // Results are typically public
        );

        // Store completion in blockchain
        self.blockchain_manager
            .add_asset_record(completion_record)
            .await?;

        Ok(())
    }

    /// Query asset execution history from blockchain
    pub async fn get_asset_execution_history(
        &self,
        asset_id: &AssetId,
    ) -> Result<Vec<ComputeExecutionRecord>, String> {
        let records = self.blockchain_manager
            .get_asset_records(asset_id)
            .await?;

        let mut execution_records = Vec::new();

        for record in records {
            if record.record_type == AssetRecordType::ComputeExecution {
                match serde_json::from_slice::<ComputeExecutionRecord>(&record.data_payload) {
                    Ok(exec_record) => execution_records.push(exec_record),
                    Err(e) => {
                        tracing::warn!("Failed to deserialize execution record: {}", e);
                    }
                }
            }
        }

        Ok(execution_records)
    }

    /// Get current asset status from blockchain
    pub async fn get_asset_status(
        &self,
        asset_id: &AssetId,
    ) -> Result<Option<HyperMeshAssetRecord>, String> {
        self.blockchain_manager.get_asset_status(asset_id).await
    }

    /// Validate asset ownership and permissions
    pub async fn validate_asset_access(
        &self,
        asset_id: &AssetId,
        required_privacy_level: AssetPrivacyLevel,
        consensus_proof: &ConsensusProof,
    ) -> Result<bool, String> {
        // Get current asset status
        let asset_record = match self.get_asset_status(asset_id).await? {
            Some(record) => record,
            None => return Ok(false), // Asset doesn't exist
        };

        // Validate privacy level
        if !asset_record.validates_privacy(&required_privacy_level) {
            return Ok(false);
        }

        // Validate consensus proof
        consensus_proof.validate().await.map_err(|e| format!("Consensus validation failed: {:?}", e))
    }
}

/// VM execution context with HyperMesh integration
#[derive(Clone, Serialize, Deserialize)]
pub struct VMExecutionContext {
    /// Allocated compute asset ID
    pub asset_id: AssetId,
    /// Resource allocation details
    pub resource_allocation: ComputeResourceRequirements,
    /// Privacy level for this execution
    pub privacy_level: AssetPrivacyLevel,
    /// Execution metadata
    pub execution_metadata: ExecutionMetadata,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Execution request ID
    pub request_id: String,
    /// Language being executed
    pub language: String,
    /// Requesting node identity
    pub requester: String,
    /// Execution priority
    pub priority: u8,
    /// Maximum execution time
    pub timeout_seconds: u64,
}

impl VMExecutionContext {
    pub fn new(
        asset_id: AssetId,
        resource_allocation: ComputeResourceRequirements,
        privacy_level: AssetPrivacyLevel,
        execution_metadata: ExecutionMetadata,
    ) -> Self {
        Self {
            asset_id,
            resource_allocation,
            privacy_level,
            execution_metadata,
        }
    }

    /// Check if this context meets resource requirements
    pub fn meets_requirements(&self, required: &ComputeResourceRequirements) -> bool {
        self.resource_allocation.cpu_cores >= required.cpu_cores
            && self.resource_allocation.memory_mb >= required.memory_mb
            && self.resource_allocation.storage_mb >= required.storage_mb
            && (!required.gpu_required || self.resource_allocation.gpu_required)
    }

    /// Get execution limits based on allocation
    pub fn get_execution_limits(&self) -> ExecutionLimits {
        ExecutionLimits {
            max_cpu_cores: self.resource_allocation.cpu_cores,
            max_memory_mb: self.resource_allocation.memory_mb,
            max_storage_mb: self.resource_allocation.storage_mb,
            max_execution_time_seconds: self.resource_allocation.timeout_seconds,
            gpu_access: self.resource_allocation.gpu_required,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExecutionLimits {
    pub max_cpu_cores: u32,
    pub max_memory_mb: u64,
    pub max_storage_mb: u64,
    pub max_execution_time_seconds: u64,
    pub gpu_access: bool,
}

/// Asset-based execution request for Catalog VM
#[derive(Clone, Serialize, Deserialize)]
pub struct AssetBasedExecutionRequest {
    /// Code to execute
    pub code: String,
    /// Language for execution
    pub language: String,
    /// Required resources
    pub resource_requirements: ComputeResourceRequirements,
    /// Privacy level for execution
    pub privacy_level: AssetPrivacyLevel,
    /// Consensus proof for resource allocation
    pub consensus_proof: ConsensusProof,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
}

impl AssetBasedExecutionRequest {
    /// Create new asset-based execution request
    pub fn new(
        code: String,
        language: String,
        resource_requirements: ComputeResourceRequirements,
        privacy_level: AssetPrivacyLevel,
        consensus_proof: ConsensusProof,
        metadata: ExecutionMetadata,
    ) -> Self {
        Self {
            code,
            language,
            resource_requirements,
            privacy_level,
            consensus_proof,
            metadata,
        }
    }

    /// Validate the execution request
    pub async fn validate(&self) -> Result<(), String> {
        // Validate consensus proof
        if !self.consensus_proof.validate().await.map_err(|e| format!("Consensus validation failed: {:?}", e))? {
            return Err("Invalid consensus proof".to_string());
        }

        // Validate resource requirements
        if self.resource_requirements.cpu_cores == 0 {
            return Err("CPU cores must be greater than 0".to_string());
        }

        if self.resource_requirements.memory_mb == 0 {
            return Err("Memory must be greater than 0".to_string());
        }

        // Validate code
        if self.code.trim().is_empty() {
            return Err("Code cannot be empty".to_string());
        }

        // Validate timeout
        if self.resource_requirements.timeout_seconds == 0 {
            return Err("Timeout must be greater than 0".to_string());
        }

        Ok(())
    }
}

/// Result of asset-based execution
#[derive(Clone, Serialize, Deserialize)]
pub struct AssetBasedExecutionResult {
    /// Execution context that was used
    pub context: VMExecutionContext,
    /// Execution result
    pub result: ComputeExecutionResult,
    /// Asset utilization statistics
    pub asset_utilization: AssetUtilizationStats,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AssetUtilizationStats {
    /// CPU utilization percentage (0.0 - 100.0)
    pub cpu_utilization_percent: f64,
    /// Memory utilization percentage
    pub memory_utilization_percent: f64,
    /// Storage utilization percentage
    pub storage_utilization_percent: f64,
    /// GPU utilization percentage (if applicable)
    pub gpu_utilization_percent: Option<f64>,
    /// Network bytes transferred
    pub network_bytes_transferred: u64,
    /// Total execution time
    pub total_execution_time_ms: u64,
}

impl AssetUtilizationStats {
    /// Calculate efficiency score (0.0 - 1.0)
    pub fn efficiency_score(&self) -> f64 {
        let scores = vec![
            self.cpu_utilization_percent / 100.0,
            self.memory_utilization_percent / 100.0,
            self.storage_utilization_percent / 100.0,
        ];

        // Add GPU score if applicable
        let scores = if let Some(gpu_util) = self.gpu_utilization_percent {
            let mut scores = scores;
            scores.push(gpu_util / 100.0);
            scores
        } else {
            scores
        };

        // Average utilization as efficiency metric
        scores.iter().sum::<f64>() / scores.len() as f64
    }

    /// Check if utilization is within acceptable bounds
    pub fn is_efficient(&self) -> bool {
        let score = self.efficiency_score();
        score >= 0.3 && score <= 0.9 // 30% minimum, 90% maximum for efficiency
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::{ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime, NetworkPosition, AccessPermissions, AccessLevel};
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_blockchain_client_creation() {
        let client = HyperMeshBlockchainClient::new("test-node".to_string());
        assert_eq!(client.node_identity, "test-node");
    }

    #[test]
    fn test_execution_context() {
        let asset_id = AssetId::new(AssetType::Cpu);
        let requirements = ComputeResourceRequirements {
            cpu_cores: 4,
            memory_mb: 8192,
            gpu_required: false,
            storage_mb: 1024,
            timeout_seconds: 300,
        };
        
        let metadata = ExecutionMetadata {
            request_id: "test-request".to_string(),
            language: "julia".to_string(),
            requester: "test-user".to_string(),
            priority: 5,
            timeout_seconds: 300,
        };

        let context = VMExecutionContext::new(
            asset_id,
            requirements.clone(),
            AssetPrivacyLevel::P2P,
            metadata,
        );

        assert!(context.meets_requirements(&requirements));
        
        let limits = context.get_execution_limits();
        assert_eq!(limits.max_cpu_cores, 4);
        assert_eq!(limits.max_memory_mb, 8192);
        assert!(!limits.gpu_access);
    }

    #[test]
    fn test_utilization_stats() {
        let stats = AssetUtilizationStats {
            cpu_utilization_percent: 75.0,
            memory_utilization_percent: 60.0,
            storage_utilization_percent: 45.0,
            gpu_utilization_percent: Some(80.0),
            network_bytes_transferred: 1024 * 1024,
            total_execution_time_ms: 5000,
        };

        let efficiency = stats.efficiency_score();
        assert!(efficiency > 0.6); // Should be reasonable efficiency
        assert!(stats.is_efficient()); // Should be within acceptable bounds
    }

    #[tokio::test]
    async fn test_execution_request_validation() {
        // Create mock consensus proof components
        let space_proof = ProofOfSpace::new(
            "/test/storage".to_string(),
            NetworkPosition {
                address: "test-address".to_string(),
                zone: "test-zone".to_string(),
                distance_metric: 100,
            },
            1024 * 1024 * 1024,
        );

        let stake_proof = ProofOfStake::new(
            "test-holder".to_string(),
            "test-node".to_string(),
            1000,
            AccessPermissions {
                read_level: AccessLevel::Public,
                write_level: AccessLevel::Public,
                admin_level: AccessLevel::None,
                allocation_rights: vec!["cpu".to_string()],
            },
            vec!["test-allowance".to_string()],
        );

        let work_proof = ProofOfWork::new(
            b"test-challenge",
            16,
            "cpu".to_string(),
        ).unwrap();

        let time_proof = ProofOfTime::new(1000, None, 1);

        let consensus_proof = ConsensusProof::new(
            space_proof,
            stake_proof,
            work_proof,
            time_proof,
        );

        let requirements = ComputeResourceRequirements {
            cpu_cores: 2,
            memory_mb: 4096,
            gpu_required: false,
            storage_mb: 512,
            timeout_seconds: 120,
        };

        let metadata = ExecutionMetadata {
            request_id: "test".to_string(),
            language: "julia".to_string(),
            requester: "test-user".to_string(),
            priority: 3,
            timeout_seconds: 120,
        };

        let request = AssetBasedExecutionRequest::new(
            "println(\"Hello, HyperMesh!\")".to_string(),
            "julia".to_string(),
            requirements,
            AssetPrivacyLevel::P2P,
            consensus_proof,
            metadata,
        );

        // Validation should pass for well-formed request
        assert!(request.validate().await.is_ok());
    }
}