// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Proof of State Consensus Integration for TrustChain
//!
//! This module implements the four-proof consensus system extracted from Proof of State
//! for use in TrustChain certificate operations and CT log validation.

use serde::{Serialize, Deserialize};
use std::time::{SystemTime, Duration};
use sha2::{Sha256, Digest};
use anyhow::{Result, anyhow};

pub mod proof;
pub mod validator;
pub mod validation;
pub mod asset_integration;
pub mod block_matrix;
pub mod hypermesh_client;
pub mod real_validator;

pub use proof::*;
pub use validator::*;
pub use validation::*;
pub use asset_integration::*;
pub use block_matrix::*;
pub use hypermesh_client::*;

/// Proof of State Four-Proof Consensus System
/// Based on the reference implementation from /home/persist/repos/personal/Proof of State/src/mods/proof.rs
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

    /// TEST-ONLY: Create a valid test proof
    /// This is ONLY available in test builds and should NEVER be used in production
    #[cfg(test)]
    pub fn default_for_testing() -> Self {
        Self::new_for_testing()
    }

    /// Create a testing proof - only available in test builds or with localhost-testing feature
    #[cfg(any(test, feature = "localhost-testing"))]
    pub fn new_for_testing() -> Self {
        use std::time::Duration;

        // Create space proof with proper total_size
        let mut space_proof = SpaceProof::new(
            "test_node_001".to_string(),  // node_id
            "test_storage_path".to_string(),  // storage_path
            100 * 1024 * 1024 * 1024  // 100GB total_storage
        );
        // Set total_size to a non-zero value (50GB used)
        space_proof.total_size = 50 * 1024 * 1024 * 1024;
        space_proof.file_hash = "test_hash_1234567890".to_string();

        Self {
            stake_proof: StakeProof::new(
                "test_stake_holder".to_string(),
                "test_node_001".to_string(),
                10000  // Sufficient stake amount for validation
            ),
            time_proof: TimeProof::new(Duration::from_secs(1)),  // Valid time offset
            space_proof,
            work_proof: WorkProof::new(
                "test_owner".to_string(),
                "test_workload_001".to_string(),
                1234,  // PID
                1000,  // Valid computational power (>16 for CPU validation)
                WorkloadType::Compute,  // General computation
                WorkState::Running
            ),
        }
    }

    /// Validate all four proofs
    pub fn validate(&self) -> bool {
        self.stake_proof.validate() &&
        self.time_proof.validate() &&
        self.space_proof.validate() &&
        self.work_proof.validate()
    }

    /// Comprehensive validation of all four proofs with detailed error reporting
    /// This is the async version required by BlockMatrix for detailed validation
    pub async fn validate_comprehensive(&self) -> Result<bool> {
        // Use the new validation module for detailed checking
        let validation = self.verify_all()?;

        if !validation.all_valid {
            return Err(anyhow!(
                "Consensus proof validation failed: {}",
                validation.error_summary()
            ));
        }

        // All proofs passed comprehensive validation
        Ok(true)
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

// STUB: Phase 3 - Default implementation for ConsensusProof
impl Default for ConsensusProof {
    fn default() -> Self {
        Self {
            stake_proof: StakeProof::default(),
            time_proof: TimeProof::default(),
            space_proof: SpaceProof::default(),
            work_proof: WorkProof::default(),
        }
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

    #[tokio::test]
    async fn test_consensus_proof_creation() -> anyhow::Result<()> {
        let node_id = "test-node-01";
        let proof = ConsensusProof::generate_from_network(node_id).await?;
        assert!(proof.validate());
        Ok(())
    }

    #[tokio::test]
    async fn test_consensus_proof_serialization() -> anyhow::Result<()> {
        let node_id = "test-node-01";
        let proof = ConsensusProof::generate_from_network(node_id).await?;
        let bytes = proof.to_bytes()?;
        let deserialized = ConsensusProof::from_bytes(&bytes)?;

        assert_eq!(proof.stake_proof.stake_amount, deserialized.stake_proof.stake_amount);
        Ok(())
    }

    #[tokio::test]
    async fn test_consensus_requirements_validation() -> anyhow::Result<()> {
        let node_id = "test-node-01";
        let proof = ConsensusProof::generate_from_network(node_id).await?;
        let requirements = ConsensusRequirements::localhost_testing();

        assert!(proof.validate_with_requirements(&requirements));
        Ok(())
    }

    #[test]
    fn test_new_for_testing_creates_valid_proof() {
        // Test that new_for_testing creates a valid proof that passes validation
        let proof = ConsensusProof::new_for_testing();

        // Check that all components have valid values
        assert!(proof.space_proof.total_size > 0, "Space proof should have non-zero total_size");
        assert!(proof.stake_proof.stake_amount >= 50, "Stake proof should have sufficient amount for CPU validation");
        assert!(proof.work_proof.computational_power >= 16, "Work proof should have sufficient computational power for CPU");
        assert!(proof.time_proof.nonce > 0, "Time proof should have non-zero nonce");

        // Validate the overall proof
        assert!(proof.validate(), "Test proof should pass validation");
    }

    #[tokio::test]
    async fn test_consensus_proof_hash() -> anyhow::Result<()> {
        let node_id = "test-node-01";
        let proof = ConsensusProof::generate_from_network(node_id).await?;
        let hash1 = proof.hash()?;
        let hash2 = proof.hash()?;

        assert_eq!(hash1, hash2); // Same proof should have same hash
        Ok(())
    }
}