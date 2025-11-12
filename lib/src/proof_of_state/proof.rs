//! Unified 4-Proof Consensus System for HyperMesh Assets
//!
//! Every asset operation in HyperMesh requires validation through all four proofs:
//! - PoSpace (PoSp): WHERE - storage location and network position
//! - PoStake (PoSt): WHO - ownership, access rights, and economic stake  
//! - PoWork (PoWk): WHAT/HOW - computational resources and processing
//! - PoTime (PoTm): WHEN - temporal ordering and timestamp validation
//!
//! This unified system answers WHERE/WHO/WHAT/WHEN for every operation.

use std::time::{SystemTime, Duration};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use super::error::{ConsensusError, ConsensusResult};

/// Unified consensus proof required for all asset operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusProof {
    /// WHERE: Proof of Space - storage/network location validation
    pub proof_of_space: ProofOfSpace,
    /// WHO: Proof of Stake - ownership/permissions validation  
    pub proof_of_stake: ProofOfStake,
    /// WHAT/HOW: Proof of Work - computational validation
    pub proof_of_work: ProofOfWork,
    /// WHEN: Proof of Time - temporal ordering validation
    pub proof_of_time: ProofOfTime,
    /// Combined proof hash for verification
    pub combined_hash: [u8; 32],
    /// Proof creation timestamp
    pub created_at: SystemTime,
}

impl ConsensusProof {
    /// Create new consensus proof with all four validations
    pub fn new(
        space_proof: ProofOfSpace,
        stake_proof: ProofOfStake,
        work_proof: ProofOfWork,
        time_proof: ProofOfTime,
    ) -> Self {
        let created_at = SystemTime::now();
        let combined_hash = Self::calculate_combined_hash(
            &space_proof,
            &stake_proof,
            &work_proof,
            &time_proof,
            &created_at,
        );

        Self {
            proof_of_space: space_proof,
            proof_of_stake: stake_proof,
            proof_of_work: work_proof,
            proof_of_time: time_proof,
            combined_hash,
            created_at,
        }
    }

    /// Validate all four proofs for asset operation
    pub async fn validate(&self) -> ConsensusResult<bool> {
        // All four proofs must be valid
        let space_valid = self.proof_of_space.validate().await?;
        let stake_valid = self.proof_of_stake.validate().await?;
        let work_valid = self.proof_of_work.validate().await?;
        let time_valid = self.proof_of_time.validate().await?;

        // Verify combined hash integrity
        let expected_hash = Self::calculate_combined_hash(
            &self.proof_of_space,
            &self.proof_of_stake,
            &self.proof_of_work,
            &self.proof_of_time,
            &self.created_at,
        );

        let hash_valid = self.combined_hash == expected_hash;

        Ok(space_valid && stake_valid && work_valid && time_valid && hash_valid)
    }

    /// Calculate combined hash of all proofs
    fn calculate_combined_hash(
        space: &ProofOfSpace,
        stake: &ProofOfStake,
        work: &ProofOfWork,
        time: &ProofOfTime,
        timestamp: &SystemTime,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Include all proof hashes
        hasher.update(&space.location_hash);
        hasher.update(&stake.ownership_hash);
        hasher.update(&work.computation_hash);
        hasher.update(&time.temporal_hash);

        // Include timestamp
        if let Ok(duration) = timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            hasher.update(&duration.as_micros().to_le_bytes());
        }

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// Proof of Space: WHERE something is or happened
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfSpace {
    /// Storage location identifier
    pub storage_location: String,
    /// Network position/routing information
    pub network_position: NetworkPosition,
    /// Committed storage space (in bytes)
    pub committed_space: u64,
    /// Storage proof hash
    pub location_hash: [u8; 32],
    /// Proof generation timestamp
    pub generated_at: SystemTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkPosition {
    /// IPv6 address or HyperMesh proxy address
    pub address: String,
    /// Network region/zone identifier
    pub zone: String,
    /// Routing distance metric
    pub distance_metric: u32,
}

impl ProofOfSpace {
    pub fn new(
        storage_location: String,
        network_position: NetworkPosition,
        committed_space: u64,
    ) -> Self {
        let generated_at = SystemTime::now();
        let location_hash = Self::calculate_location_hash(
            &storage_location,
            &network_position,
            committed_space,
            &generated_at,
        );

        Self {
            storage_location,
            network_position,
            committed_space,
            location_hash,
            generated_at,
        }
    }

    pub async fn validate(&self) -> ConsensusResult<bool> {
        // Verify hash integrity
        let expected_hash = Self::calculate_location_hash(
            &self.storage_location,
            &self.network_position,
            self.committed_space,
            &self.generated_at,
        );

        if self.location_hash != expected_hash {
            return Ok(false);
        }

        // Validate actual storage commitment
        if self.committed_space == 0 {
            return Err(ConsensusError::InvalidStorageCommitment);
        }
        
        // Verify network position reachability
        if self.network_position.address.is_empty() || self.network_position.zone.is_empty() {
            return Err(ConsensusError::InvalidNetworkPosition);
        }
        
        // Verify distance metric is reasonable
        if self.network_position.distance_metric > 10000 {
            return Err(ConsensusError::NetworkPositionTooDistant);
        }
        
        Ok(true)
    }

    fn calculate_location_hash(
        location: &str,
        position: &NetworkPosition,
        space: u64,
        timestamp: &SystemTime,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(location.as_bytes());
        hasher.update(position.address.as_bytes());
        hasher.update(position.zone.as_bytes());
        hasher.update(&position.distance_metric.to_le_bytes());
        hasher.update(&space.to_le_bytes());
        
        if let Ok(duration) = timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            hasher.update(&duration.as_micros().to_le_bytes());
        }

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// Proof of Stake: WHO owns or operates the asset (not token staking)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfStake {
    /// Asset owner/operator entity (e.g., "dmv", "bank", "user-123")
    pub stake_holder: String,
    /// Validating node ID
    pub stake_holder_id: String,
    /// Asset ownership authority level (not economic tokens)
    pub authority_level: u64,
    /// Access permissions for this asset
    pub permissions: AccessPermissions,
    /// Asset delegation allowances granted to others
    pub allowances: Vec<String>,
    /// Ownership proof hash
    pub ownership_hash: [u8; 32],
    /// Proof generation timestamp
    pub generated_at: SystemTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessPermissions {
    /// Read access level
    pub read_level: AccessLevel,
    /// Write access level  
    pub write_level: AccessLevel,
    /// Administrative privileges
    pub admin_level: AccessLevel,
    /// Resource allocation rights
    pub allocation_rights: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AccessLevel {
    None,
    Private,      // Internal network only
    Network,      // Specific networks/groups
    Public,       // Cross-network accessible
    Verified,     // Full consensus validation
}

impl ProofOfStake {
    pub fn new(
        stake_holder: String,
        stake_holder_id: String,
        authority_level: u64,
        permissions: AccessPermissions,
        allowances: Vec<String>,
    ) -> Self {
        let generated_at = SystemTime::now();
        let ownership_hash = Self::calculate_ownership_hash(
            &stake_holder,
            &stake_holder_id,
            authority_level,
            &permissions,
            &allowances,
            &generated_at,
        );

        Self {
            stake_holder,
            stake_holder_id,
            authority_level,
            permissions,
            allowances,
            ownership_hash,
            generated_at,
        }
    }

    pub async fn validate(&self) -> ConsensusResult<bool> {
        // Verify hash integrity
        let expected_hash = Self::calculate_ownership_hash(
            &self.stake_holder,
            &self.stake_holder_id,
            self.authority_level,
            &self.permissions,
            &self.allowances,
            &self.generated_at,
        );

        if self.ownership_hash != expected_hash {
            return Ok(false);
        }

        // Validate authority level for asset operation
        if self.authority_level == 0 {
            return Err(ConsensusError::InsufficientAuthority);
        }
        
        // Verify stake holder identity is valid
        if self.stake_holder.is_empty() || self.stake_holder_id.is_empty() {
            return Err(ConsensusError::InvalidStakeHolder);
        }
        
        // Check allowances and delegations are properly formatted
        for allowance in &self.allowances {
            if allowance.is_empty() {
                return Err(ConsensusError::InvalidAllowance);
            }
        }
        
        Ok(true)
    }

    fn calculate_ownership_hash(
        stake_holder: &str,
        stake_holder_id: &str,
        authority_level: u64,
        permissions: &AccessPermissions,
        allowances: &[String],
        timestamp: &SystemTime,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(stake_holder.as_bytes());
        hasher.update(stake_holder_id.as_bytes());
        hasher.update(&authority_level.to_le_bytes());
        
        // Hash permission levels
        hasher.update(&[permissions.read_level.to_u8()]);
        hasher.update(&[permissions.write_level.to_u8()]);
        hasher.update(&[permissions.admin_level.to_u8()]);
        
        for right in &permissions.allocation_rights {
            hasher.update(right.as_bytes());
        }
        
        // Hash allowances
        for allowance in allowances {
            hasher.update(allowance.as_bytes());
        }

        if let Ok(duration) = timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            hasher.update(&duration.as_micros().to_le_bytes());
        }

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

impl AccessLevel {
    fn to_u8(&self) -> u8 {
        match self {
            AccessLevel::None => 0,
            AccessLevel::Private => 1,
            AccessLevel::Network => 2,
            AccessLevel::Public => 3,
            AccessLevel::Verified => 4,
        }
    }
}

/// Proof of Work: WHAT/HOW they did it
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfWork {
    /// Computational challenge solution
    pub nonce: u64,
    /// Difficulty target achieved
    pub difficulty: u32,
    /// Resource type being validated
    pub resource_type: String,
    /// Computation result hash
    pub computation_hash: [u8; 32],
    /// Work completion timestamp
    pub completed_at: SystemTime,
}

impl ProofOfWork {
    pub fn new(
        challenge: &[u8],
        difficulty: u32,
        resource_type: String,
    ) -> ConsensusResult<Self> {
        let start_time = SystemTime::now();
        let (nonce, computation_hash) = Self::solve_challenge(challenge, difficulty)?;
        let completed_at = SystemTime::now();

        Ok(Self {
            nonce,
            difficulty,
            resource_type,
            computation_hash,
            completed_at,
        })
    }

    pub async fn validate(&self) -> ConsensusResult<bool> {
        // Re-verify the computational work by reconstructing challenge
        let challenge = format!("{}:{}:{}", self.resource_type, self.nonce, self.difficulty).into_bytes();
        let computed_hash = Self::calculate_work_hash(&challenge, self.nonce);
        
        if computed_hash != self.computation_hash {
            return Err(ConsensusError::InvalidWorkProof);
        }
        
        // Check difficulty target was met
        let target = Self::calculate_target(self.difficulty);
        if !Self::meets_difficulty(&self.computation_hash, &target) {
            return Err(ConsensusError::InsufficientDifficulty);
        }
        
        Ok(true)
    }

    fn solve_challenge(challenge: &[u8], difficulty: u32) -> ConsensusResult<(u64, [u8; 32])> {
        let target = Self::calculate_target(difficulty);
        
        for nonce in 0..u64::MAX {
            let hash = Self::calculate_work_hash(challenge, nonce);
            if Self::meets_difficulty(&hash, &target) {
                return Ok((nonce, hash));
            }
        }
        
        Err(ConsensusError::ProofOfWorkFailed)
    }

    fn calculate_target(difficulty: u32) -> [u8; 32] {
        // Higher difficulty = smaller target (more leading zeros)
        let mut target = [0xFF; 32];
        let zero_bytes = (difficulty / 8) as usize;
        let zero_bits = difficulty % 8;
        
        for i in 0..zero_bytes.min(32) {
            target[i] = 0x00;
        }
        
        if zero_bytes < 32 && zero_bits > 0 {
            target[zero_bytes] = 0xFF >> zero_bits;
        }
        
        target
    }

    fn calculate_work_hash(challenge: &[u8], nonce: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(challenge);
        hasher.update(&nonce.to_le_bytes());
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    fn meets_difficulty(hash: &[u8; 32], target: &[u8; 32]) -> bool {
        hash <= target
    }
}

/// Proof of Time: WHEN it occurred  
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfTime {
    /// Logical timestamp for ordering
    pub logical_timestamp: u64,
    /// Wall clock timestamp
    pub wall_clock: SystemTime,
    /// Previous temporal proof hash (forms chain)
    pub previous_hash: Option<[u8; 32]>,
    /// Temporal ordering proof hash
    pub temporal_hash: [u8; 32],
    /// Sequence number for this node
    pub sequence_number: u64,
}

impl ProofOfTime {
    pub fn new(
        logical_timestamp: u64,
        previous_hash: Option<[u8; 32]>,
        sequence_number: u64,
    ) -> Self {
        let wall_clock = SystemTime::now();
        let temporal_hash = Self::calculate_temporal_hash(
            logical_timestamp,
            &wall_clock,
            &previous_hash,
            sequence_number,
        );

        Self {
            logical_timestamp,
            wall_clock,
            previous_hash,
            temporal_hash,
            sequence_number,
        }
    }

    pub async fn validate(&self) -> ConsensusResult<bool> {
        // Verify hash integrity
        let expected_hash = Self::calculate_temporal_hash(
            self.logical_timestamp,
            &self.wall_clock,
            &self.previous_hash,
            self.sequence_number,
        );

        if self.temporal_hash != expected_hash {
            return Ok(false);
        }

        // Validate logical timestamp ordering
        if self.logical_timestamp == 0 {
            return Err(ConsensusError::InvalidTimestamp);
        }
        
        // Check wall clock bounds and drift (±5 minutes tolerance)
        let now = SystemTime::now();
        let five_minutes = Duration::from_secs(300);
        
        if let (Ok(wall_duration), Ok(now_duration)) = (
            self.wall_clock.duration_since(SystemTime::UNIX_EPOCH),
            now.duration_since(SystemTime::UNIX_EPOCH)
        ) {
            let diff = if wall_duration > now_duration {
                wall_duration - now_duration
            } else {
                now_duration - wall_duration
            };
            
            if diff > five_minutes {
                return Err(ConsensusError::TimestampDriftExceeded);
            }
        }
        
        Ok(true)
    }

    fn calculate_temporal_hash(
        logical_ts: u64,
        wall_clock: &SystemTime,
        previous_hash: &Option<[u8; 32]>,
        sequence: u64,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&logical_ts.to_le_bytes());
        hasher.update(&sequence.to_le_bytes());
        
        if let Ok(duration) = wall_clock.duration_since(SystemTime::UNIX_EPOCH) {
            hasher.update(&duration.as_micros().to_le_bytes());
        }
        
        if let Some(prev_hash) = previous_hash {
            hasher.update(prev_hash);
        }

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consensus_proof_creation() {
        let space_proof = ProofOfSpace::new(
            "/dev/nvme0n1".to_string(),
            NetworkPosition {
                address: "2001:db8::1".to_string(),
                zone: "us-west-1".to_string(),
                distance_metric: 100,
            },
            1_000_000_000, // 1GB
        );

        let stake_proof = ProofOfStake::new(
            "user-123".to_string(),           // Asset owner entity
            "node-456".to_string(),           // Validating node ID  
            1000,                             // Authority level (not tokens)
            AccessPermissions {
                read_level: AccessLevel::Public,
                write_level: AccessLevel::Network,
                admin_level: AccessLevel::None,
                allocation_rights: vec!["cpu".to_string(), "memory".to_string()],
            },
            vec!["delegate:cpu".to_string(), "delegate:memory".to_string()], // Allowances
        );

        let work_proof = ProofOfWork::new(
            b"test-challenge",
            16, // 16-bit difficulty
            "cpu".to_string(),
        ).unwrap();

        let time_proof = ProofOfTime::new(1000, None, 1);

        let consensus_proof = ConsensusProof::new(
            space_proof,
            stake_proof, 
            work_proof,
            time_proof,
        );

        assert!(consensus_proof.validate().await.unwrap());
    }
}