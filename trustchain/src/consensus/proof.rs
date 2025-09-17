//! Individual Proof Implementations
//! 
//! Based on NKrypt reference implementation from /home/persist/repos/personal/NKrypt/src/mods/proof.rs
//! Adapted for TrustChain certificate operations with IPv6-only networking

use serde::{Serialize, Deserialize};
use std::time::{SystemTime, Duration};
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use anyhow::{Result, anyhow};
use rand::Rng;

/// Proof trait for validation
pub trait Proof {
    fn validate(&self) -> bool;
}

/// StakeProof - WHO owns/validates (economic security)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakeProof {
    /// Entity owning the asset (e.g., CA, CT log, DNS server)
    pub stake_holder: String,
    /// ID of the validating node
    pub stake_holder_id: String,
    /// Economic stake amount
    pub stake_amount: u64,
    /// When stake was created
    pub stake_timestamp: SystemTime,
}

impl StakeProof {
    pub fn new(stake_holder: String, stake_holder_id: String, stake_amount: u64) -> Self {
        Self {
            stake_holder,
            stake_holder_id,
            stake_amount,
            stake_timestamp: SystemTime::now(),
        }
    }

    pub fn default() -> Self {
        Self {
            stake_holder: "localhost_test".to_string(),
            stake_holder_id: "test_node_001".to_string(),
            stake_amount: 1000,
            stake_timestamp: SystemTime::now(),
        }
    }

    pub fn verify_signature(&self) -> bool {
        // Simplified signature verification for now
        // In production, this would verify cryptographic signatures
        !self.stake_holder_id.is_empty() && self.stake_amount > 0
    }

    pub fn sign(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}-{}-{}", 
            self.stake_holder_id, 
            self.stake_amount, 
            self.stake_timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
        ));
        format!("{:x}", hasher.finalize())
    }
}

impl Proof for StakeProof {
    fn validate(&self) -> bool {
        // Validate stake amount
        if self.stake_amount == 0 {
            return false;
        }

        // Validate stake age (not too old)
        if let Ok(elapsed) = self.stake_timestamp.elapsed() {
            if elapsed > Duration::from_secs(60 * 60 * 24 * 30) { // 30 days max
                return false;
            }
        }

        // Validate signature
        self.verify_signature()
    }
}

impl PartialEq for StakeProof {
    fn eq(&self, other: &Self) -> bool {
        self.stake_holder == other.stake_holder &&
        self.stake_holder_id == other.stake_holder_id &&
        self.stake_amount == other.stake_amount &&
        self.stake_timestamp == other.stake_timestamp
    }
}

/// TimeProof - WHEN it occurred (temporal ordering)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeProof {
    /// Network time synchronization offset
    pub network_time_offset: Duration,
    /// When proof was created
    pub time_verification_timestamp: SystemTime,
    /// Prevent replay attacks
    pub nonce: u64,
    /// Cryptographic proof hash
    pub proof_hash: Vec<u8>,
}

impl TimeProof {
    pub fn new(network_time_offset: Duration) -> Self {
        let time_verification_timestamp = SystemTime::now();
        let nonce = rand::thread_rng().gen::<u64>();

        // Generate cryptographic proof hash
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
        Self::new(Duration::from_secs(0))
    }

    /// Serialize for network transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Serialize network_time_offset
        bytes.extend_from_slice(&self.network_time_offset.as_micros().to_le_bytes());
        
        // Serialize time_verification_timestamp
        bytes.extend_from_slice(&self.time_verification_timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros().to_le_bytes());
        
        // Serialize nonce
        bytes.extend_from_slice(&self.nonce.to_le_bytes());
        
        // Serialize proof_hash
        bytes.extend_from_slice(&self.proof_hash);
        
        bytes
    }

    /// Deserialize from network transmission
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 40 { // Minimum size check
            return Err(anyhow!("Invalid data length for TimeProof"));
        }

        // Deserialize network_time_offset (bytes 0-15)
        let network_time_offset_bytes: [u8; 16] = data[0..16].try_into()
            .map_err(|_| anyhow!("Invalid network_time_offset slice"))?;
        let network_time_offset = Duration::from_micros(u128::from_le_bytes(network_time_offset_bytes) as u64);

        // Deserialize timestamp (bytes 16-31)
        let timestamp_bytes: [u8; 16] = data[16..32].try_into()
            .map_err(|_| anyhow!("Invalid timestamp slice"))?;
        let timestamp_micros = u128::from_le_bytes(timestamp_bytes) as u64;
        let time_verification_timestamp = SystemTime::UNIX_EPOCH + Duration::from_micros(timestamp_micros);

        // Deserialize nonce (bytes 32-39)
        let nonce_bytes: [u8; 8] = data[32..40].try_into()
            .map_err(|_| anyhow!("Invalid nonce slice"))?;
        let nonce = u64::from_le_bytes(nonce_bytes);

        // Deserialize proof_hash (remaining bytes)
        let proof_hash = data[40..].to_vec();

        Ok(Self {
            network_time_offset,
            time_verification_timestamp,
            nonce,
            proof_hash,
        })
    }
}

impl Proof for TimeProof {
    fn validate(&self) -> bool {
        // Validate proof hash
        let mut hasher = Sha256::new();
        hasher.update(&self.network_time_offset.as_micros().to_le_bytes());
        hasher.update(&self.time_verification_timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros().to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());
        
        let expected_hash = hasher.finalize().to_vec();
        expected_hash == self.proof_hash
    }
}

impl PartialEq for TimeProof {
    fn eq(&self, other: &Self) -> bool {
        self.network_time_offset == other.network_time_offset &&
        self.time_verification_timestamp == other.time_verification_timestamp &&
        self.nonce == other.nonce &&
        self.proof_hash == other.proof_hash
    }
}

/// SpaceProof - WHERE it's stored (storage commitment)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceProof {
    /// Node providing storage
    pub node_id: String,
    /// Storage location path (IPv6 network path)
    pub storage_path: String,
    /// Bytes actually stored
    pub total_size: u64,
    /// Total storage capacity
    pub total_storage: u64,
    /// Content integrity hash
    pub file_hash: String,
    /// When proof was created
    pub proof_timestamp: SystemTime,
}

impl SpaceProof {
    pub fn new(node_id: String, storage_path: String, total_storage: u64) -> Self {
        Self {
            node_id,
            storage_path,
            total_size: 0,
            total_storage,
            file_hash: String::new(),
            proof_timestamp: SystemTime::now(),
        }
    }

    pub fn default() -> Self {
        Self {
            node_id: "localhost_node".to_string(),
            storage_path: "/tmp/trustchain_test".to_string(),
            total_size: 1024,
            total_storage: 1024 * 1024,
            file_hash: "test_hash".to_string(),
            proof_timestamp: SystemTime::now(),
        }
    }
}

impl Proof for SpaceProof {
    fn validate(&self) -> bool {
        // Validate storage capacity
        if self.total_storage == 0 {
            return false;
        }

        // Validate size doesn't exceed capacity
        if self.total_size > self.total_storage {
            return false;
        }

        // Validate node ID is not empty
        !self.node_id.is_empty()
    }
}

impl PartialEq for SpaceProof {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id &&
        self.storage_path == other.storage_path &&
        self.total_size == other.total_size &&
        self.total_storage == other.total_storage &&
        self.file_hash == other.file_hash
    }
}

/// WorkProof - WHAT computational work (resource proof)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkProof {
    /// Entity requesting work
    pub owner_id: String,
    /// Unique work identifier
    pub workload_id: String,
    /// Process ID for work
    pub pid: u64,
    /// CPU/GPU resources used
    pub computational_power: u64,
    /// Type of computation
    pub workload_type: WorkloadType,
    /// Current work status
    pub work_state: WorkState,
    /// Work challenges for validation
    pub work_challenges: Vec<String>,
    /// When proof was created
    pub proof_timestamp: SystemTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkloadType {
    /// Certificate generation/validation
    Certificate,
    /// CT log operations
    CertificateTransparency,
    /// DNS resolution
    DnsResolution,
    /// General computation
    Compute,
    /// Network operations
    Network,
    /// Storage operations
    Storage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkState {
    Pending,
    Running,
    Completed,
    Failed,
}

impl WorkProof {
    pub fn new(
        owner_id: String,
        workload_id: String,
        pid: u64,
        computational_power: u64,
        workload_type: WorkloadType,
        work_state: WorkState,
    ) -> Self {
        Self {
            owner_id,
            workload_id,
            pid,
            computational_power,
            workload_type,
            work_state,
            work_challenges: Vec::new(),
            proof_timestamp: SystemTime::now(),
        }
    }

    pub fn default() -> Self {
        Self {
            owner_id: "localhost_test".to_string(),
            workload_id: "test_work_001".to_string(),
            pid: 1000,
            computational_power: 100,
            workload_type: WorkloadType::Certificate,
            work_state: WorkState::Completed,
            work_challenges: vec!["test_challenge".to_string()],
            proof_timestamp: SystemTime::now(),
        }
    }
}

impl Proof for WorkProof {
    fn validate(&self) -> bool {
        // Validate computational power
        if self.computational_power == 0 {
            return false;
        }

        // Validate work is not pending indefinitely
        if matches!(self.work_state, WorkState::Pending) {
            if let Ok(elapsed) = self.proof_timestamp.elapsed() {
                if elapsed > Duration::from_secs(60 * 10) { // 10 minutes max pending
                    return false;
                }
            }
        }

        // Validate owner ID is not empty
        !self.owner_id.is_empty()
    }
}

impl PartialEq for WorkProof {
    fn eq(&self, other: &Self) -> bool {
        self.owner_id == other.owner_id &&
        self.workload_id == other.workload_id &&
        self.pid == other.pid &&
        self.computational_power == other.computational_power
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stake_proof_validation() {
        let stake_proof = StakeProof::default();
        assert!(stake_proof.validate());
    }

    #[test]
    fn test_time_proof_validation() {
        let time_proof = TimeProof::default();
        assert!(time_proof.validate());
    }

    #[test]
    fn test_time_proof_serialization() {
        let time_proof = TimeProof::default();
        let bytes = time_proof.to_bytes();
        let deserialized = TimeProof::from_bytes(&bytes).unwrap();
        
        assert_eq!(time_proof, deserialized);
    }

    #[test]
    fn test_space_proof_validation() {
        let space_proof = SpaceProof::default();
        assert!(space_proof.validate());
    }

    #[test]
    fn test_work_proof_validation() {
        let work_proof = WorkProof::default();
        assert!(work_proof.validate());
    }

    #[test]
    fn test_stake_proof_signature() {
        let stake_proof = StakeProof::default();
        let signature = stake_proof.sign();
        assert!(!signature.is_empty());
    }
}