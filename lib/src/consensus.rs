//! Shared consensus proof types (Four-Proof System)

use serde::{Deserialize, Serialize};

/// Proof of Space - WHERE (storage location and physical/network location)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfSpace {
    pub location: String,
    pub storage_commitment: Vec<u8>,
    pub verification_data: Vec<u8>,
}

/// Proof of Stake - WHO (ownership, access rights, and economic stake)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfStake {
    pub stake_amount: u64,
    pub owner_id: String,
    pub verification_signature: Vec<u8>,
}

/// Proof of Work - WHAT/HOW (computational resources and processing)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfWork {
    pub computation_proof: Vec<u8>,
    pub difficulty: u32,
    pub nonce: u64,
}

/// Proof of Time - WHEN (temporal ordering and timestamp validation)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfTime {
    pub timestamp: u64,
    pub sequence_number: u64,
    pub time_verification: Vec<u8>,
}

/// Combined Consensus Proof (all four proofs required)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusProof {
    pub space: ProofOfSpace,
    pub stake: ProofOfStake,
    pub work: ProofOfWork,
    pub time: ProofOfTime,
}
