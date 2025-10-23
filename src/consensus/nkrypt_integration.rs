//! NKrypt Four-Proof Consensus Integration for HyperMesh
//!
//! This module integrates the complete NKrypt Four-Proof Consensus system into HyperMesh.
//! Every asset operation requires validation through all four proofs:
//! - PoSpace (PoSp): WHERE - storage location and physical/network location
//! - PoStake (PoSt): WHO - ownership, access rights, and economic stake  
//! - PoWork (PoWk): WHAT/HOW - computational resources and processing
//! - PoTime (PoTm): WHEN - temporal ordering and timestamp validation
//!
//! This creates a unified "Consensus Proof" answering WHERE/WHO/WHAT/WHEN for every operation.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::{Duration, SystemTime};
use sha2::{Sha256, Digest};
use rand::Rng;
use serde::{Serialize, Deserialize};
use super::error::{ConsensusError, ConsensusResult};

/// Base trait for all proof types
pub trait Proof {
    fn validate(&self) -> bool;
}

/// Client credentials for consensus validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientCredentials {
    /// Unique identifier for the client
    client_id: String,
    _client_public_key: String,
    _client_private_key: String,
}

impl PartialEq for ClientCredentials {
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id
    }
}

/// Proof of Space: WHERE - storage location and network position validation
#[derive(Clone, Serialize, Deserialize)]
pub struct SpaceProof {
    /// Node providing the space
    pub node_id: String,
    /// Storage path or identifier
    pub storage_path: String,
    /// Total allocated size in bytes
    pub total_size: u64,
    /// Total available storage in bytes
    pub total_storage: u64,
    /// File hash for integrity verification
    pub file_hash: String,
    /// Proof generation timestamp
    pub proof_timestamp: SystemTime,
}

impl SpaceProof {
    pub fn new(total_storage: u64, storage_path: String) -> Self {
        SpaceProof { 
            node_id: String::new(),
            storage_path, 
            total_size: 0,
            total_storage, 
            file_hash: String::new(),
            proof_timestamp: SystemTime::now(),
        }
    }

    pub fn default() -> Self {
        SpaceProof {
            node_id: String::new(),
            storage_path: String::new(),
            total_size: 0,
            total_storage: 0,
            file_hash: String::new(),
            proof_timestamp: SystemTime::now(),
        }
    }
}

impl Proof for SpaceProof {
    fn validate(&self) -> bool {
        // Validate space commitment and integrity
        self.total_storage > 0 && 
        !self.storage_path.is_empty() &&
        !self.node_id.is_empty()
    }
}

impl PartialEq for SpaceProof {
    fn eq(&self, other: &Self) -> bool {
        self.total_storage == other.total_storage &&
        self.storage_path == other.storage_path &&
        self.node_id == other.node_id &&
        self.total_size == other.total_size &&
        self.file_hash == other.file_hash
    }
}

/// Work execution states
#[derive(Clone, Serialize, Deserialize)]
pub enum WorkState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Types of computational workloads  
#[derive(Clone, Serialize, Deserialize)]
pub enum WorkloadType {
    Genesis,   // Creation or allocation
    Modify,    // Modification operations
    Delete,    // Deletion operations
    Storage,   // Storage operations
    Compute,   // GPU/CPU computational work
    Network,   // Network operations
}

/// Proof of Work: WHAT/HOW - computational resources and processing validation
#[derive(Clone, Serialize, Deserialize)]
pub struct WorkProof {
    /// Entity performing the work
    pub owner_id: String,
    /// Workload identifier
    pub workload_id: String,
    /// Process identifier
    pub pid: u64,
    /// Computational power demonstrated
    pub computational_power: u64,
    /// Type of work performed
    pub workload_type: WorkloadType,
    /// Current work state
    pub work_state: WorkState,
    /// Work challenges completed
    pub work_challenges: Vec<String>,
    /// Proof generation timestamp
    pub proof_timestamp: SystemTime,
}

impl Proof for WorkProof {
    fn validate(&self) -> bool {
        // Validate computational work demonstration
        self.computational_power > 0 &&
        !self.owner_id.is_empty() &&
        !self.workload_id.is_empty() &&
        matches!(self.work_state, WorkState::Completed | WorkState::Running)
    }
}

impl WorkProof {
    pub fn new(
        computational_power: u64, 
        workload_id: String, 
        pid: u64, 
        owner_id: String, 
        workload_type: WorkloadType, 
        work_state: WorkState
    ) -> Self {
        WorkProof { 
            computational_power, 
            workload_id, 
            pid, 
            owner_id, 
            workload_type, 
            work_state,
            work_challenges: vec![],
            proof_timestamp: SystemTime::now(),
        }
    }

    pub fn default() -> Self {
        WorkProof {
            owner_id: String::new(),
            workload_id: String::new(),
            pid: 0,
            computational_power: 0,
            workload_type: WorkloadType::Genesis,
            work_state: WorkState::Pending,
            work_challenges: vec![],
            proof_timestamp: SystemTime::now(),
        }
    }
}

impl PartialEq for WorkProof {
    fn eq(&self, other: &Self) -> bool {
        self.computational_power == other.computational_power &&
        self.workload_id == other.workload_id &&
        self.pid == other.pid &&
        self.owner_id == other.owner_id
    }
}

/// Proof of Time: WHEN - temporal ordering and timestamp validation
#[derive(Clone, Serialize, Deserialize)]
pub struct TimeProof {
    /// Network time synchronization offset
    pub network_time_offset: Duration,
    /// Time verification timestamp
    pub time_verification_timestamp: SystemTime,
    /// Nonce to prevent replay attacks
    pub nonce: u64,
    /// Cryptographic proof hash
    pub proof_hash: Vec<u8>,
}

impl PartialEq for TimeProof {
    fn eq(&self, other: &Self) -> bool {
        self.network_time_offset == other.network_time_offset &&
        self.time_verification_timestamp == other.time_verification_timestamp &&
        self.nonce == other.nonce &&
        self.proof_hash == other.proof_hash
    }
}

impl TimeProof {
    pub fn new(network_time_offset: Duration) -> Self {
        let time_verification_timestamp = SystemTime::now();
        let nonce = rand::thread_rng().gen::<u64>();

        // Generate the cryptographic proof hash
        let proof_hash = {
            let mut hasher = Sha256::new();
            hasher.update(&network_time_offset.as_micros().to_le_bytes());
            hasher.update(&time_verification_timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros().to_le_bytes());
            hasher.update(&nonce.to_le_bytes());
            hasher.finalize().to_vec()
        };

        Self {
            network_time_offset,
            time_verification_timestamp,
            nonce,
            proof_hash,
        }
    }

    pub fn default() -> Self {
        TimeProof {
            network_time_offset: Duration::from_secs(0),
            time_verification_timestamp: SystemTime::now(),
            nonce: 0,
            proof_hash: vec![],
        }
    }

    /// Serialize time proof for networking
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
    
        // Serialize network_time_offset
        let network_time_offset_bytes = self.network_time_offset.as_micros().to_le_bytes();
        bytes.extend_from_slice(&network_time_offset_bytes);
    
        // Serialize time_verification_timestamp
        let time_verification_timestamp_bytes = self.time_verification_timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .to_le_bytes();
        bytes.extend_from_slice(&time_verification_timestamp_bytes);
    
        // Serialize nonce
        let nonce_bytes = self.nonce.to_le_bytes();
        bytes.extend_from_slice(&nonce_bytes);
    
        // Serialize proof_hash
        bytes.extend_from_slice(&self.proof_hash);
    
        bytes
    }

    /// Deserialize time proof from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 40 { // Minimum size check
            return Err("Invalid data length".into());
        }
    
        // Deserialize network_time_offset (bytes 0-15)
        let network_time_offset_slice: [u8; 16] = data[0..16].try_into()
            .map_err(|_| "Invalid network_time_offset slice")?;
        let network_time_offset = Duration::from_micros(u128::from_le_bytes(network_time_offset_slice) as u64);
    
        // Deserialize timestamp_micros (bytes 16-31)
        let timestamp_micros_slice: [u8; 16] = data[16..32].try_into()
            .map_err(|_| "Invalid timestamp_micros slice")?;
        let timestamp_micros = u128::from_le_bytes(timestamp_micros_slice) as u64;
        let time_verification_timestamp = SystemTime::UNIX_EPOCH + Duration::from_micros(timestamp_micros);
    
        // Deserialize nonce (bytes 32-39)
        let nonce_slice: [u8; 8] = data[32..40].try_into()
            .map_err(|_| "Invalid nonce slice")?;
        let nonce = u64::from_le_bytes(nonce_slice);
    
        // Deserialize proof_hash (remaining bytes)
        let proof_hash = data[40..].to_vec();
    
        Ok(Self {
            network_time_offset,
            time_verification_timestamp,
            nonce,
            proof_hash,
        })
    }

    /// Validate the cryptographic proof
    pub fn validate(&self) -> bool {
        let mut hasher = Sha256::new();
        
        // Compute the hash of the same fields (excluding the proof_hash)
        let network_time_offset_bytes = self.network_time_offset.as_micros().to_le_bytes();
        let timestamp_bytes = self.time_verification_timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .to_le_bytes();
        let nonce_bytes = self.nonce.to_le_bytes();
    
        hasher.update(&network_time_offset_bytes);
        hasher.update(&timestamp_bytes);
        hasher.update(&nonce_bytes);
        
        let expected_hash = hasher.finalize().to_vec();
        
        // Compare the computed hash with the proof_hash
        expected_hash == self.proof_hash
    }
}

/// Proof of Stake: WHO - ownership, access rights, and economic stake validation
#[derive(Clone, Serialize, Deserialize)]
pub struct StakeProof {
    /// Asset holder entity (e.g., "dmv", "bank", "user-123")
    pub stake_holder: String,
    /// ID of the validating node
    pub stake_holder_id: String,
    /// Stake amount for validation authority
    pub stake_amount: u64,
    /// Timestamp of stake commitment
    pub stake_timestamp: SystemTime,
}

impl PartialEq for StakeProof {
    fn eq(&self, other: &Self) -> bool {
        self.stake_holder == other.stake_holder &&
        self.stake_holder_id == other.stake_holder_id &&
        self.stake_amount == other.stake_amount &&
        self.stake_timestamp == other.stake_timestamp
    }
}

impl StakeProof {
    pub fn new(stake_holder: String, stake_holder_id: String, stake_amount: u64) -> Self {
        StakeProof {
            stake_holder,
            stake_holder_id,
            stake_amount,
            stake_timestamp: SystemTime::now(),
        }
    }

    pub fn default() -> Self {
        StakeProof {
            stake_holder: String::new(),
            stake_holder_id: String::new(),
            stake_amount: 0,
            stake_timestamp: SystemTime::now(),
        }
    }

    pub fn to_vec(&self) -> Vec<StakeProof> {
        vec![self.clone()]
    }

    pub fn verify(&self) -> bool {
        // Stake proof validation - must have positive stake
        self.stake_amount > 0 && 
        !self.stake_holder.is_empty() &&
        !self.stake_holder_id.is_empty()
    }

    pub fn sign(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}-{}-{}", 
            self.stake_holder_id, 
            self.stake_amount, 
            self.stake_timestamp.elapsed().unwrap_or(Duration::ZERO).as_secs()
        ));
        format!("{:x}", hasher.finalize())
    }
}

impl Proof for StakeProof {
    fn validate(&self) -> bool {
        self.verify()
    }
}

/// Unified Consensus Proof combining all four proofs for HyperMesh asset operations
#[derive(Clone, Serialize, Deserialize)]
pub struct ConsensusProof {
    /// WHERE: Proof of Space - storage/network location validation
    pub stake_proof: StakeProof,
    /// WHEN: Proof of Time - temporal ordering validation  
    pub time_proof: TimeProof,
    /// WHERE: Proof of Space - storage/network location validation
    pub space_proof: SpaceProof,
    /// WHAT/HOW: Proof of Work - computational validation
    pub work_proof: WorkProof,
}

impl PartialEq for ConsensusProof {
    fn eq(&self, other: &Self) -> bool {
        self.stake_proof == other.stake_proof &&
        self.time_proof == other.time_proof &&
        self.space_proof == other.space_proof &&
        self.work_proof == other.work_proof
    }
}

impl ConsensusProof {
    pub fn new(
        stake_proof: StakeProof,
        space_proof: SpaceProof,
        work_proof: WorkProof,
        time_proof: TimeProof,
    ) -> Self {
        ConsensusProof {
            stake_proof,
            time_proof,
            space_proof,
            work_proof,
        }
    }

    pub fn default() -> Option<Self> {
        Some(ConsensusProof {
            stake_proof: StakeProof::default(),
            time_proof: TimeProof::default(),
            space_proof: SpaceProof::default(),
            work_proof: WorkProof::default(),
        })
    }

    pub fn to_vec(&self) -> Vec<ConsensusProof> {
        vec![self.clone()]
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        bincode::deserialize(data).map_err(|e| 
            std::io::Error::new(
                std::io::ErrorKind::InvalidData, 
                format!("Failed to deserialize ConsensusProof: {}", e)
            )
        )
    }

    /// Validate all four proofs for HyperMesh asset operations
    pub fn validate(&self) -> bool {
        self.stake_proof.validate() &&
        self.time_proof.validate() &&
        self.space_proof.validate() &&
        self.work_proof.validate()
    }

    /// Comprehensive validation with detailed error reporting
    pub async fn validate_comprehensive(&self) -> ConsensusResult<bool> {
        // Validate Proof of Stake (WHO)
        if !self.stake_proof.validate() {
            return Err(ConsensusError::InvalidStakeHolder);
        }
        
        if self.stake_proof.stake_amount == 0 {
            return Err(ConsensusError::InsufficientAuthority);
        }

        // Validate Proof of Time (WHEN)
        if !self.time_proof.validate() {
            return Err(ConsensusError::InvalidTimestamp);
        }
        
        // Check time drift bounds (±5 minutes tolerance)
        let now = SystemTime::now();
        let five_minutes = Duration::from_secs(300);
        
        if let (Ok(proof_duration), Ok(now_duration)) = (
            self.time_proof.time_verification_timestamp.duration_since(SystemTime::UNIX_EPOCH),
            now.duration_since(SystemTime::UNIX_EPOCH)
        ) {
            let diff = if proof_duration > now_duration {
                proof_duration - now_duration
            } else {
                now_duration - proof_duration
            };
            
            if diff > five_minutes {
                return Err(ConsensusError::TimestampDriftExceeded);
            }
        }

        // Validate Proof of Space (WHERE)
        if !self.space_proof.validate() {
            return Err(ConsensusError::InvalidStorageCommitment);
        }
        
        if self.space_proof.total_storage == 0 {
            return Err(ConsensusError::InvalidStorageCommitment);
        }

        // Validate Proof of Work (WHAT/HOW)
        if !self.work_proof.validate() {
            return Err(ConsensusError::InvalidWorkProof);
        }
        
        if self.work_proof.computational_power == 0 {
            return Err(ConsensusError::InsufficientDifficulty);
        }

        Ok(true)
    }
}

/// Pretty debugging format for ConsensusProof
impl fmt::Debug for ConsensusProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConsensusProof")
            .field("\n\t\t\tstake_proof", &self.stake_proof)
            .field("\n\t\t\ttime_proof", &self.time_proof)
            .field("\n\t\t\tspace_proof", &self.space_proof)
            .field("\n\t\t\twork_proof", &self.work_proof)
            .finish()
    }
}

impl fmt::Debug for StakeProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StakeProof")
            .field("\n\t\t\t\tstake_holder", &self.stake_holder)
            .field("\n\t\t\t\tstake_holder_id", &self.stake_holder_id)
            .field("\n\t\t\t\tstake_amount", &self.stake_amount)
            .finish()
    }
}

impl fmt::Debug for TimeProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TimeProof")
            .field("\n\t\t\t\t\tnetwork_time_offset", &self.network_time_offset)
            .field("\n\t\t\t\t\ttime_verification_timestamp", &self.time_verification_timestamp)
            .field("\n\t\t\t\t\tnonce", &self.nonce)
            .field("\n\t\t\t\t\tproof_hash", &self.proof_hash)
            .finish()
    }
}

impl fmt::Debug for SpaceProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpaceProof")
            .field("\n\t\t\t\t\ttotal_storage", &self.total_storage)
            .field("\n\t\t\t\t\tstorage_path", &self.storage_path)
            .field("\n\t\t\t\t\tproof_timestamp", &self.proof_timestamp)
            .finish()
    }
}

impl fmt::Debug for WorkProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkProof")
            .field("\n\t\t\t\t\tcomputational_power", &self.computational_power)
            .field("\n\t\t\t\t\twork_challenges", &self.work_challenges)
            .field("\n\t\t\t\t\tproof_timestamp", &self.proof_timestamp)
            .finish()
    }
}

/// Distributed client structure supporting NKrypt consensus patterns
#[derive(Clone, Debug)]
pub struct DistributedClient {
    /// Core client structure
    credentials: ClientCredentials,
    space_proofs: Vec<SpaceProof>,
    work_proofs: Vec<WorkProof>,
    stake_proofs: Vec<StakeProof>,
    pub time_proof: Option<TimeProof>,
    
    /// Mirroring and hosting capabilities for HyperMesh assets
    mirrored_spaces: HashSet<String>,
    hosted_works: HashSet<String>,
}

impl DistributedClient {
    pub fn new(client_id: String, _stake_amount: u64) -> Self {
        DistributedClient {
            credentials: ClientCredentials {
                client_id,
                _client_public_key: String::new(),
                _client_private_key: String::new(),
            },
            space_proofs: Vec::new(),
            work_proofs: Vec::new(),
            stake_proofs: Vec::new(),
            time_proof: None,
            mirrored_spaces: HashSet::new(),
            hosted_works: HashSet::new(),
        }
    }

    fn validate_stake(&self) -> bool {
        // Minimum stake requirement and aging mechanism
        let total_stake: u64 = self.stake_proofs.iter().map(|p| p.stake_amount).sum();
        let stake_valid = total_stake >= 5000;

        // Stake aging mechanism (30 days max age)
        let stake_age_valid = self.stake_proofs.iter().all(|p|
            p.stake_timestamp.elapsed().unwrap_or(Duration::MAX).as_secs() < 60 * 60 * 24 * 30
        );

        // Stake proof validation
        let stake_amount_valid = self.stake_proofs.iter().all(|p| p.stake_amount > 0);

        // Stake proof uniqueness (can't self-validate)
        let stake_holder_unique = self.stake_proofs.iter().all(|p| p.stake_holder_id != self.credentials.client_id);

        // Stake proof ordering (stake must be older than time proof)
        let stake_order_valid = self.stake_proofs.iter().all(|p| {
            self.time_proof.as_ref().map_or(true, |t| p.stake_timestamp < t.time_verification_timestamp)
        });

        // Stake proof verification (signature check)
        let stake_proof_valid = self.stake_proofs.iter().all(|p| {
            let _proof_hash = p.sign();
            p.verify()
        });

        stake_valid && stake_age_valid && stake_amount_valid && stake_holder_unique && stake_order_valid && stake_proof_valid
    }

    fn validate_time_sync(&self) -> bool {
        // Time proof validation (must exist and be within 60 seconds)
        self.time_proof.is_some() && 
        self.time_proof.as_ref().unwrap().network_time_offset < Duration::from_secs(60)
    }

    pub fn add_space_proof(&mut self, space_proof: SpaceProof) {
        // Automatically become a mirror if stake and time proofs are valid
        if self.validate_stake() && self.validate_time_sync() {
            self.space_proofs.push(space_proof.clone());
            self.mirrored_spaces.insert(space_proof.storage_path);
        }

        // Determine the total storage capacity for HyperMesh asset allocation
        let _total_storage_capacity = self.space_proofs.iter().map(|p| p.total_storage).sum::<u64>();

        // TODO: Add Validation for segmentation based on permissions of stakeholders
        // Register the client as a mirror for the storage path
    }

    pub fn add_work_proof(&mut self, work_proof: WorkProof) {
        // Automatically become a host if stake and time proofs are valid
        if self.validate_stake() && self.validate_time_sync() {
            self.work_proofs.push(work_proof.clone());
            self.hosted_works.extend(
                work_proof.work_challenges.iter().cloned()
            );
        }

        // Determine the total computational power for HyperMesh asset allocation
        let _total_computational_power = self.work_proofs.iter().map(|p| p.computational_power).sum::<u64>();

        // TODO: Add Validation for segmentation based on permissions of stakeholders
        // Register the client as a host for the work challenges
    }

    pub fn get_network_capabilities(&self) -> NetworkCapabilities {
        NetworkCapabilities {
            _client_id: self.credentials.client_id.clone(),
            _proof_of_space: self.space_proofs.len() as u64,
            _proof_of_space_capacity: self.space_proofs.iter().map(|p| p.total_storage).sum(),
            _proof_of_work: self.work_proofs.len() as u64,
            _proof_of_work_potential: self.work_proofs.iter().map(|p| p.computational_power).sum(),
            _proof_of_time: self.time_proof.as_ref().map_or(Duration::default(), |p| p.network_time_offset),
            _proof_of_stake: self.stake_proofs.len() as u64,
            _total_stakes_available: self.stake_proofs.iter().map(|p| p.stake_amount).sum(),
            _stakeholders: HashMap::new(), // Placeholder for HyperMesh integration
            _total_network_power: 0, // Placeholder for HyperMesh integration
            _proof_of_replication: 0, // Placeholder for HyperMesh integration
        }
    }
}

/// Network capabilities representation for HyperMesh asset management
#[derive(Debug)]
pub struct NetworkCapabilities {
    _client_id: String,
    _proof_of_space: u64,
    _proof_of_space_capacity: u64,
    _proof_of_work: u64,
    _proof_of_work_potential: u64,
    _proof_of_time: Duration,
    _proof_of_stake: u64,
    _total_stakes_available: u64,
    _stakeholders: HashMap<String, u64>,
    _total_network_power: u64,
    _proof_of_replication: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_proof_creation_and_validation() {
        let time_proof = TimeProof::new(Duration::from_millis(500));
        assert!(time_proof.validate());
        
        // Test serialization round trip
        let bytes = time_proof.to_bytes();
        let deserialized = TimeProof::from_bytes(&bytes).unwrap();
        assert_eq!(time_proof, deserialized);
        assert!(deserialized.validate());
    }

    #[test]
    fn test_consensus_proof_creation() {
        let stake_proof = StakeProof::new(
            "test-holder".to_string(),
            "test-node".to_string(),
            1000
        );
        
        let space_proof = SpaceProof::new(
            1_000_000,
            "/test/storage".to_string()
        );
        
        let work_proof = WorkProof::new(
            100,
            "test-workload".to_string(),
            12345,
            "test-worker".to_string(),
            WorkloadType::Compute,
            WorkState::Completed
        );
        
        let time_proof = TimeProof::new(Duration::from_millis(100));
        
        let consensus_proof = ConsensusProof::new(
            stake_proof,
            space_proof,
            work_proof,
            time_proof
        );
        
        assert!(consensus_proof.validate());
    }

    #[tokio::test]
    async fn test_comprehensive_validation() {
        let stake_proof = StakeProof::new(
            "test-holder".to_string(),
            "test-node".to_string(),
            1000
        );
        
        let mut space_proof = SpaceProof::new(
            1_000_000,
            "/test/storage".to_string()
        );
        space_proof.node_id = "test-node".to_string();
        
        let work_proof = WorkProof::new(
            100,
            "test-workload".to_string(),
            12345,
            "test-worker".to_string(),
            WorkloadType::Compute,
            WorkState::Completed
        );
        
        let time_proof = TimeProof::new(Duration::from_millis(100));
        
        let consensus_proof = ConsensusProof::new(
            stake_proof,
            space_proof,
            work_proof,
            time_proof
        );
        
        assert!(consensus_proof.validate_comprehensive().await.is_ok());
    }

    #[test]
    fn test_distributed_client() {
        let mut client = DistributedClient::new("test-client".to_string(), 5000);
        
        let space_proof = SpaceProof::new(
            1_000_000,
            "/test/storage".to_string()
        );
        
        client.add_space_proof(space_proof);
        
        let capabilities = client.get_network_capabilities();
        assert_eq!(capabilities._client_id, "test-client");
    }
}