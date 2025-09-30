//! NKrypt Consensus Integration for TrustChain
//! 
//! This module implements the four-proof consensus system extracted from NKrypt
//! for use in TrustChain certificate operations and CT log validation.

use serde::{Serialize, Deserialize};
use std::time::{SystemTime, Duration};
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use anyhow::{Result, anyhow};
use rand::Rng;

pub mod proof;
pub mod validator;
pub mod block_matrix;
pub mod hypermesh_client;
pub mod real_validator;

pub use proof::*;
pub use validator::*;
pub use block_matrix::*;
pub use hypermesh_client::*;

/// NKrypt Four-Proof Consensus System
/// Based on the reference implementation from /home/persist/repos/personal/NKrypt/src/mods/proof.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusProof {
    /// WHO owns/validates (economic security)
    pub stake_proof: StakeProof,
    /// WHEN it occurred (temporal ordering)
    pub time_proof: TimeProof,
    /// WHERE it's stored (storage commitment)
    pub space_proof: SpaceProof,
    /// WHAT computational work (resource proof)
    pub work_proof: WorkProof,
}

impl ConsensusProof {
    /// Create a new consensus proof with all four proofs
    pub fn new(
        stake_proof: StakeProof,
        time_proof: TimeProof,
        space_proof: SpaceProof,
        work_proof: WorkProof,
    ) -> Self {
        Self {
            stake_proof,
            time_proof,
            space_proof,
            work_proof,
        }
    }

    /// Generate real consensus proof from network state
    /// This replaces the security bypass default_for_testing() method
    pub async fn generate_from_network(node_id: &str) -> Result<Self> {
        // Generate real stake proof with cryptographic signatures
        let stake_proof = StakeProof::generate_from_network(node_id).await?;

        // Generate time proof with network synchronization
        let time_proof = TimeProof::generate_with_ntp_sync().await?;

        // Generate space proof with actual storage commitment
        let space_proof = SpaceProof::generate_from_system(node_id).await?;

        // Generate work proof with computational challenge
        let work_proof = WorkProof::generate_from_computation(node_id).await?;

        Ok(Self {
            stake_proof,
            time_proof,
            space_proof,
            work_proof,
        })
    }

    /// DEPRECATED - SECURITY BYPASS - DO NOT USE IN PRODUCTION
    /// This method creates invalid proofs and bypasses security validation
    /// TODO: Replace all calls to this method with generate_from_network()
    #[cfg(test)]
    pub fn default_for_testing() -> Self {
        Self {
            stake_proof: StakeProof::default(),
            time_proof: TimeProof::default(),
            space_proof: SpaceProof::default(),
            work_proof: WorkProof::default(),
        }
    }

    /// Validate all four proofs
    pub fn validate(&self) -> bool {
        self.stake_proof.validate() &&
        self.time_proof.validate() &&
        self.space_proof.validate() &&
        self.work_proof.validate()
    }

    /// Validate with specific requirements
    pub fn validate_with_requirements(&self, requirements: &ConsensusRequirements) -> bool {
        // Validate stake requirements
        if self.stake_proof.stake_amount < requirements.minimum_stake {
            return false;
        }

        // Validate time synchronization
        if self.time_proof.network_time_offset > requirements.max_time_offset {
            return false;
        }

        // Validate storage commitment
        if self.space_proof.total_storage < requirements.minimum_storage {
            return false;
        }

        // Validate computational work
        if self.work_proof.computational_power < requirements.minimum_compute {
            return false;
        }

        // Validate all proofs cryptographically
        self.validate()
    }

    /// Serialize for network transmission
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow!("Failed to serialize ConsensusProof: {}", e))
    }

    /// Deserialize from network transmission
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(|e| anyhow!("Failed to deserialize ConsensusProof: {}", e))
    }

    /// Generate cryptographic hash of the consensus proof
    pub fn hash(&self) -> Result<[u8; 32]> {
        let bytes = self.to_bytes()?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        Ok(hasher.finalize().into())
    }
}

/// Requirements for consensus validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusRequirements {
    /// Minimum stake amount for validation
    pub minimum_stake: u64,
    /// Maximum time offset for synchronization
    pub max_time_offset: Duration,
    /// Minimum storage commitment
    pub minimum_storage: u64,
    /// Minimum computational power
    pub minimum_compute: u64,
    /// Byzantine fault tolerance (fraction of malicious nodes)
    pub byzantine_tolerance: f64,
}

impl Default for ConsensusRequirements {
    fn default() -> Self {
        Self {
            minimum_stake: 5000,                          // 5K tokens minimum
            max_time_offset: Duration::from_secs(60),     // 60 second max offset
            minimum_storage: 1024 * 1024 * 1024,         // 1GB minimum
            minimum_compute: 1000,                        // 1000 compute units
            byzantine_tolerance: 0.33,                    // 33% Byzantine tolerance
        }
    }
}

/// Production requirements for high-security operations
impl ConsensusRequirements {
    pub fn production() -> Self {
        Self {
            minimum_stake: 50000,                         // 50K tokens for production
            max_time_offset: Duration::from_secs(30),     // 30 second max offset
            minimum_storage: 10 * 1024 * 1024 * 1024,    // 10GB minimum
            minimum_compute: 10000,                       // 10K compute units
            byzantine_tolerance: 0.33,                    // 33% Byzantine tolerance
        }
    }

    pub fn localhost_testing() -> Self {
        Self {
            minimum_stake: 100,                           // 100 tokens for testing
            max_time_offset: Duration::from_secs(300),    // 5 minute max offset
            minimum_storage: 1024 * 1024,                // 1MB minimum
            minimum_compute: 10,                          // 10 compute units
            byzantine_tolerance: 0.0,                     // No Byzantine tolerance for testing
        }
    }
}

/// Consensus validation result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsensusResult {
    Valid {
        confidence_score: f64,
        validation_timestamp: SystemTime,
        validation_duration: Duration,
    },
    Invalid {
        reason: String,
        failed_proofs: Vec<String>,
        validation_timestamp: SystemTime,
    },
    Pending {
        validation_id: String,
        estimated_completion: SystemTime,
    },
}

impl ConsensusResult {
    /// Check if the consensus result is valid
    pub fn is_valid(&self) -> bool {
        matches!(self, ConsensusResult::Valid { .. })
    }
}

/// Consensus validation context
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusContext {
    pub validator_id: String,
    pub network_id: String,
    pub requirements: ConsensusRequirements,
    pub byzantine_detectors: Vec<String>,
}

impl ConsensusContext {
    pub fn new(validator_id: String, network_id: String) -> Self {
        Self {
            validator_id,
            network_id,
            requirements: ConsensusRequirements::default(),
            byzantine_detectors: Vec::new(),
        }
    }

    pub fn localhost_testing(validator_id: String) -> Self {
        Self {
            validator_id,
            network_id: "localhost".to_string(),
            requirements: ConsensusRequirements::localhost_testing(),
            byzantine_detectors: Vec::new(),
        }
    }

    pub fn production(validator_id: String, network_id: String) -> Self {
        Self {
            validator_id,
            network_id,
            requirements: ConsensusRequirements::production(),
            byzantine_detectors: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_proof_creation() {
        let proof = ConsensusProof::generate_from_network(&node_id).await?;
        assert!(proof.validate());
    }

    #[test]
    fn test_consensus_proof_serialization() {
        let proof = ConsensusProof::generate_from_network(&node_id).await?;
        let bytes = proof.to_bytes()?;
        let deserialized = ConsensusProof::from_bytes(&bytes)?;
        
        assert_eq!(proof.stake_proof.stake_amount, deserialized.stake_proof.stake_amount);
    }

    #[test]
    fn test_consensus_requirements_validation() {
        let proof = ConsensusProof::generate_from_network(&node_id).await?;
        let requirements = ConsensusRequirements::localhost_testing();
        
        assert!(proof.validate_with_requirements(&requirements));
    }

    #[test]
    fn test_consensus_proof_hash() {
        let proof = ConsensusProof::generate_from_network(&node_id).await?;
        let hash1 = proof.hash()?;
        let hash2 = proof.hash()?;
        
        assert_eq!(hash1, hash2); // Same proof should have same hash
    }
}