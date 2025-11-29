//! HyperMesh Blockchain Integration for Asset Management
//!
//! This module implements asset-based blockchain operations following Proof of State patterns.
//! Assets are stored directly in blockchain blocks with ConsensusProof validation.

use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use crate::assets::core::asset_id::{AssetId, AssetType};
use crate::consensus::{ConsensusProof, ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime, Consensus, ConsensusResult, LogIndex};

/// Asset record types for blockchain operations
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AssetRecordType {
    /// Asset creation/registration
    Creation,
    /// Asset ownership transfer
    Transfer,
    /// Asset status update (allocation, usage, etc.)
    StatusUpdate,
    /// Asset association with utility tokens
    TokenAssociation,
    /// Compute execution on asset
    ComputeExecution,
    /// Asset privacy configuration change
    PrivacyUpdate,
    /// Asset adapter configuration
    AdapterConfig,
    /// Custom asset operation
    Custom(String),
}

impl AssetRecordType {
    pub fn to_string(&self) -> String {
        match self {
            AssetRecordType::Custom(ref s) => s.clone(),
            _ => format!("{:?}", self),
        }
    }
}

/// HyperMesh asset record for blockchain storage
#[derive(Clone, Serialize, Deserialize)]
pub struct HyperMeshAssetRecord {
    /// Universal asset identifier
    pub asset_id: AssetId,
    /// Type of asset operation
    pub record_type: AssetRecordType,
    /// Operation timestamp
    pub timestamp: SystemTime,
    /// Authority performing the operation
    pub issuing_authority: String,
    /// Asset-specific data payload
    pub data_payload: Vec<u8>,
    /// Consensus proofs (all 4 required: PoSp+PoSt+PoWk+PoTm)
    pub consensus_proofs: Vec<ConsensusProof>,
    /// Privacy level for this operation
    pub privacy_level: AssetPrivacyLevel,
    /// Asset adapter that handled this operation
    pub adapter_type: AssetType,
}

/// Privacy levels for asset operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AssetPrivacyLevel {
    /// Internal network only
    Private,
    /// Specific networks/groups
    PrivateNetwork,
    /// Trusted peer sharing
    P2P,
    /// Specific public networks
    PublicNetwork,
    /// Maximum rewards, full HyperMesh participation
    FullPublic,
}

impl HyperMeshAssetRecord {
    /// Create new asset record with consensus validation
    pub fn new(
        asset_id: AssetId,
        record_type: AssetRecordType,
        issuing_authority: String,
        data_payload: Vec<u8>,
        consensus_proofs: Vec<ConsensusProof>,
        privacy_level: AssetPrivacyLevel,
    ) -> Self {
        let adapter_type = asset_id.asset_type.clone();
        
        Self {
            asset_id,
            record_type,
            timestamp: SystemTime::now(),
            issuing_authority,
            data_payload,
            consensus_proofs,
            privacy_level,
            adapter_type,
        }
    }

    /// Validate all consensus proofs for this asset operation
    pub async fn validate_consensus(&self) -> Result<bool, String> {
        // All asset operations must have at least one consensus proof
        if self.consensus_proofs.is_empty() {
            return Err("Asset operations require consensus proofs".to_string());
        }

        // Validate each consensus proof (all 4 proofs required)
        for proof in &self.consensus_proofs {
            match proof.validate().await {
                Ok(true) => continue,
                Ok(false) => return Ok(false),
                Err(e) => return Err(format!("Consensus validation error: {:?}", e)),
            }
        }

        Ok(true)
    }

    /// Get asset record hash for blockchain inclusion
    pub fn calculate_hash(&self) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        // Hash asset ID
        hasher.update(self.asset_id.to_hex_string().as_bytes());
        
        // Hash record type
        hasher.update(self.record_type.to_string().as_bytes());
        
        // Hash authority
        hasher.update(self.issuing_authority.as_bytes());
        
        // Hash data payload
        hasher.update(&self.data_payload);
        
        // Hash timestamp
        if let Ok(duration) = self.timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            hasher.update(&duration.as_micros().to_le_bytes());
        }
        
        // Hash privacy level
        hasher.update(&[self.privacy_level.to_u8()]);
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Check if asset record meets privacy requirements
    pub fn validates_privacy(&self, required_level: &AssetPrivacyLevel) -> bool {
        match (required_level, &self.privacy_level) {
            (AssetPrivacyLevel::Private, AssetPrivacyLevel::Private) => true,
            (AssetPrivacyLevel::PrivateNetwork, AssetPrivacyLevel::PrivateNetwork | AssetPrivacyLevel::Private) => true,
            (AssetPrivacyLevel::P2P, AssetPrivacyLevel::P2P | AssetPrivacyLevel::PrivateNetwork | AssetPrivacyLevel::Private) => true,
            (AssetPrivacyLevel::PublicNetwork, _) => true, // Public network accepts all
            (AssetPrivacyLevel::FullPublic, _) => true, // Full public accepts all
            _ => false,
        }
    }
}

impl AssetPrivacyLevel {
    pub fn to_u8(&self) -> u8 {
        match self {
            AssetPrivacyLevel::Private => 0,
            AssetPrivacyLevel::PrivateNetwork => 1,
            AssetPrivacyLevel::P2P => 2,
            AssetPrivacyLevel::PublicNetwork => 3,
            AssetPrivacyLevel::FullPublic => 4,
        }
    }
}

/// HyperMesh blockchain data following Proof of State patterns
#[derive(Clone, Serialize, Deserialize)]
pub enum HyperMeshBlockData {
    /// Genesis block
    Genesis,
    /// Asset management operation (following Proof of State AssetRecord pattern)
    AssetRecord(HyperMeshAssetRecord),
    /// Raw data block
    Raw(Vec<u8>),
}

impl HyperMeshBlockData {
    /// Get block data as bytes for hashing
    pub fn data(&self) -> Vec<u8> {
        match self {
            HyperMeshBlockData::Genesis => b"GENESIS".to_vec(),
            HyperMeshBlockData::AssetRecord(record) => {
                // Serialize asset record for block inclusion
                serde_json::to_vec(record).unwrap_or_default()
            },
            HyperMeshBlockData::Raw(data) => data.clone(),
        }
    }

    /// Check if this block data requires consensus validation
    pub fn requires_consensus(&self) -> bool {
        match self {
            HyperMeshBlockData::Genesis => false,
            HyperMeshBlockData::AssetRecord(_) => true, // All asset operations require consensus
            HyperMeshBlockData::Raw(_) => false,
        }
    }
}

/// Asset blockchain manager for HyperMesh
pub struct AssetBlockchainManager {
    /// Consensus system for blockchain operations
    consensus: Arc<Consensus>,
    /// Current node authority for asset operations
    node_authority: String,
}

use std::sync::Arc;

impl AssetBlockchainManager {
    /// Create new asset blockchain manager
    pub fn new(consensus: Arc<Consensus>, node_authority: String) -> Self {
        Self {
            consensus,
            node_authority,
        }
    }

    /// Add asset record to blockchain
    pub async fn add_asset_record(
        &self,
        mut record: HyperMeshAssetRecord,
    ) -> Result<[u8; 32], String> {
        // Validate consensus proofs
        if !record.validate_consensus().await? {
            return Err("Consensus validation failed for asset record".to_string());
        }

        // Ensure record has proper consensus proofs if missing
        if record.consensus_proofs.is_empty() {
            let consensus_proof = self.create_asset_consensus_proof(&record).await
                .map_err(|e| format!("Failed to create consensus proof: {:?}", e))?;
            record.consensus_proofs.push(consensus_proof);
        }

        // Create block data
        let block_data = HyperMeshBlockData::AssetRecord(record.clone());
        let block_data_bytes = block_data.data();
        
        // Add to blockchain through consensus system
        let log_index = self.consensus.engine.replicate_entry(block_data_bytes).await
            .map_err(|e| format!("Failed to replicate to blockchain: {:?}", e))?;
        
        // Calculate final block hash
        let block_hash = record.calculate_hash();
        
        Ok(block_hash)
    }

    /// Query asset records by asset ID
    pub async fn get_asset_records(
        &self,
        asset_id: &AssetId,
    ) -> Result<Vec<HyperMeshAssetRecord>, String> {
        // TODO: Query blockchain for asset records
        // This would search the replicated log for all records matching the asset ID
        // For now, return empty vector as the log querying API needs to be extended
        Ok(vec![])
    }

    /// Get current asset status from blockchain
    pub async fn get_asset_status(
        &self,
        asset_id: &AssetId,
    ) -> Result<Option<HyperMeshAssetRecord>, String> {
        // TODO: Get latest asset record from blockchain
        // This would query the latest committed state for the asset
        Ok(None)
    }
    
    /// Create consensus proof for asset operation
    async fn create_asset_consensus_proof(
        &self,
        record: &HyperMeshAssetRecord,
    ) -> ConsensusResult<ConsensusProof> {
        let node_id = hypermesh_transport::NodeId::new(self.node_authority.clone());
        let operation_type = record.record_type.to_string();
        
        self.consensus.create_consensus_proof(
            &record.asset_id.to_hex_string(),
            &node_id,
            &operation_type,
        ).await
    }
    
    /// Validate asset operation with consensus
    pub async fn validate_asset_operation(
        &self,
        record: &HyperMeshAssetRecord,
    ) -> Result<bool, String> {
        // Check if we're in a valid consensus state
        if !self.consensus.is_leader().await {
            return Err("Asset operations require leader consensus".to_string());
        }
        
        // Validate all consensus proofs
        for proof in &record.consensus_proofs {
            let is_valid = self.consensus.validate_consensus_proof(proof).await
                .map_err(|e| format!("Consensus proof validation failed: {:?}", e))?;
            if !is_valid {
                return Ok(false);
            }
        }
        
        // Check asset privacy requirements
        let required_privacy = &record.privacy_level;
        if !record.validates_privacy(required_privacy) {
            return Ok(false);
        }
        
        Ok(true)
    }
}

/// Compute execution request stored in blockchain
#[derive(Clone, Serialize, Deserialize)]
pub struct ComputeExecutionRecord {
    /// Asset ID of the compute resource
    pub compute_asset_id: AssetId,
    /// Code to execute
    pub code: String,
    /// Language for execution
    pub language: String,
    /// Required resources
    pub resource_requirements: ComputeResourceRequirements,
    /// Privacy level for execution
    pub privacy_level: AssetPrivacyLevel,
    /// Execution results (if completed)
    pub execution_result: Option<ComputeExecutionResult>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ComputeResourceRequirements {
    /// CPU cores needed
    pub cpu_cores: u32,
    /// Memory in MB
    pub memory_mb: u64,
    /// GPU requirements
    pub gpu_required: bool,
    /// Storage in MB
    pub storage_mb: u64,
    /// Execution timeout in seconds
    pub timeout_seconds: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ComputeExecutionResult {
    /// Execution output
    pub output: String,
    /// Exit code
    pub exit_code: i32,
    /// Resource usage
    pub resource_usage: ActualResourceUsage,
    /// Execution time
    pub execution_time_ms: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ActualResourceUsage {
    pub cpu_time_ms: u64,
    pub memory_peak_mb: u64,
    pub storage_used_mb: u64,
    pub network_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::asset_id::AssetType;

    #[tokio::test]
    async fn test_asset_record_creation() {
        let asset_id = AssetId::new(AssetType::Cpu);
        
        // Create mock consensus proof (would be real in production)
        let consensus_proofs = vec![]; // TODO: Add real consensus proofs
        
        let record = HyperMeshAssetRecord::new(
            asset_id,
            AssetRecordType::Creation,
            "test-authority".to_string(),
            b"test-data".to_vec(),
            consensus_proofs,
            AssetPrivacyLevel::FullPublic,
        );

        assert_eq!(record.record_type, AssetRecordType::Creation);
        assert_eq!(record.issuing_authority, "test-authority");
        assert_eq!(record.privacy_level.to_u8(), 4); // FullPublic = 4
    }

    #[tokio::test]
    async fn test_asset_record_with_consensus_proof() {
        let asset_id = AssetId::new(AssetType::Cpu);
        
        // Create real consensus proof for testing
        let space_proof = ProofOfSpace::new(
            format!("/hypermesh/assets/{}", asset_id.to_hex_string()),
            crate::consensus::proof::NetworkPosition {
                address: "hypermesh://test-node".to_string(),
                zone: "test-zone".to_string(),
                distance_metric: 1,
            },
            1024, // 1KB allocation
        );

        let stake_proof = ProofOfStake::new(
            "test-authority".to_string(),
            "test-node-id".to_string(),
            1000,
            crate::consensus::proof::AccessPermissions {
                read_level: crate::consensus::proof::AccessLevel::Public,
                write_level: crate::consensus::proof::AccessLevel::Network,
                admin_level: crate::consensus::proof::AccessLevel::None,
                allocation_rights: vec!["Creation".to_string()],
            },
            vec!["delegate:cpu".to_string()],
        );

        let work_proof = ProofOfWork::new(
            b"test-challenge",
            8, // Low difficulty for testing
            "Creation".to_string(),
        ).unwrap();

        let time_proof = ProofOfTime::new(1000, None, 1);

        let consensus_proof = ConsensusProof::new(
            space_proof,
            stake_proof,
            work_proof,
            time_proof,
        );

        let record = HyperMeshAssetRecord::new(
            asset_id,
            AssetRecordType::Creation,
            "test-authority".to_string(),
            b"test-data".to_vec(),
            vec![consensus_proof],
            AssetPrivacyLevel::FullPublic,
        );

        // Validate consensus proofs
        let is_valid = record.validate_consensus().await.unwrap();
        assert!(is_valid, "Asset record with consensus proof should be valid");

        assert_eq!(record.record_type, AssetRecordType::Creation);
        assert_eq!(record.issuing_authority, "test-authority");
        assert_eq!(record.consensus_proofs.len(), 1);
    }

    #[test]
    fn test_privacy_validation() {
        let asset_id = AssetId::new(AssetType::Memory);
        let record = HyperMeshAssetRecord::new(
            asset_id,
            AssetRecordType::StatusUpdate,
            "test".to_string(),
            vec![],
            vec![],
            AssetPrivacyLevel::P2P,
        );

        assert!(record.validates_privacy(&AssetPrivacyLevel::FullPublic));
        assert!(record.validates_privacy(&AssetPrivacyLevel::P2P));
        assert!(!record.validates_privacy(&AssetPrivacyLevel::Private));
    }

    #[test]
    fn test_block_data_serialization() {
        let asset_id = AssetId::new(AssetType::Storage);
        let record = HyperMeshAssetRecord::new(
            asset_id,
            AssetRecordType::Transfer,
            "test".to_string(),
            vec![1, 2, 3],
            vec![],
            AssetPrivacyLevel::Private,
        );

        let block_data = HyperMeshBlockData::AssetRecord(record);
        assert!(block_data.requires_consensus());
        assert!(!block_data.data().is_empty());
    }
}