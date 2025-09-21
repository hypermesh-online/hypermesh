//! Consensus Type Definitions
//!
//! Core types for the Four-Proof Consensus system

use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};
use std::collections::HashMap;

/// Combined four-proof consensus validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusProof {
    /// Space proof (WHERE)
    pub space_proof: SpaceProof,

    /// Stake proof (WHO)
    pub stake_proof: StakeProof,

    /// Work proof (WHAT/HOW)
    pub work_proof: WorkProof,

    /// Time proof (WHEN)
    pub time_proof: TimeProof,

    /// Combined proof hash
    pub combined_hash: String,

    /// Proof generation timestamp
    pub timestamp: SystemTime,

    /// Consensus round
    pub round: u64,

    /// Validator signatures
    pub validator_signatures: Vec<ValidatorSignature>,
}

/// Space proof for asset location (WHERE)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceProof {
    /// Physical location identifier
    pub location_id: String,

    /// Storage commitment proof
    pub storage_commitment: String,

    /// Network topology position
    pub network_position: NetworkPosition,

    /// Geographic coordinates (optional)
    pub geo_location: Option<GeoLocation>,

    /// Data availability proof
    pub availability_proof: String,

    /// Replication factor
    pub replication_factor: u32,
}

/// Network position in topology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPosition {
    pub datacenter_id: String,
    pub rack_id: String,
    pub node_id: String,
    pub latency_map: HashMap<String, f64>,
}

/// Geographic location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub country: String,
    pub region: String,
}

/// Stake proof for ownership/authority (WHO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeProof {
    /// Staker identity
    pub staker_id: String,

    /// Stake amount in tokens
    pub stake_amount: u64,

    /// Stake duration
    pub stake_duration: Duration,

    /// Delegation info
    pub delegation: Option<DelegationInfo>,

    /// Reputation score
    pub reputation_score: f64,

    /// Stake lock proof
    pub lock_proof: String,
}

/// Delegation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationInfo {
    pub delegator_id: String,
    pub delegation_amount: u64,
    pub delegation_period: Duration,
}

/// Work proof for computational resources (WHAT/HOW)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkProof {
    /// Workload identifier
    pub workload_id: String,

    /// Workload type
    pub workload_type: WorkloadType,

    /// Computation result hash
    pub result_hash: String,

    /// Resource usage metrics
    pub resource_metrics: ResourceMetrics,

    /// Difficulty level
    pub difficulty: u64,

    /// Nonce for proof-of-work
    pub nonce: u64,

    /// Verification checkpoints
    pub checkpoints: Vec<WorkCheckpoint>,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub cpu_cycles: u64,
    pub memory_usage_mb: u64,
    pub io_operations: u64,
    pub network_bytes: u64,
}

/// Work verification checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCheckpoint {
    pub step: u32,
    pub hash: String,
    pub timestamp: SystemTime,
}

/// Time proof for temporal ordering (WHEN)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeProof {
    /// Timestamp
    pub timestamp: SystemTime,

    /// Time beacon reference
    pub time_beacon: String,

    /// Network time consensus
    pub network_time: NetworkTime,

    /// Block height reference
    pub block_height: u64,

    /// Previous block hash
    pub previous_hash: String,

    /// Time weight (for adjusting time-based rewards)
    pub time_weight: f64,
}

/// Network time consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTime {
    pub median_time: SystemTime,
    pub time_sources: Vec<TimeSource>,
    pub deviation_ms: f64,
}

/// Time source for network time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSource {
    pub source_id: String,
    pub timestamp: SystemTime,
    pub weight: f64,
}

/// Workload types for work proof
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadType {
    /// General computation
    Computation,
    /// Data storage
    Storage,
    /// Network relay
    Network,
    /// GPU processing
    Gpu,
    /// Consensus validation
    Validation,
    /// Smart contract execution
    Contract,
}

/// Work state in consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkState {
    Pending,
    Validating,
    Accepted,
    Rejected,
    Challenged,
}

/// Proof types in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofType {
    Space,
    Stake,
    Work,
    Time,
    Combined,
}

/// Validation results for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResults {
    /// Overall validation status
    pub is_valid: bool,

    /// Individual proof validations
    pub space_valid: bool,
    pub stake_valid: bool,
    pub work_valid: bool,
    pub time_valid: bool,

    /// Validation scores (0.0 to 1.0)
    pub space_score: f64,
    pub stake_score: f64,
    pub work_score: f64,
    pub time_score: f64,

    /// Combined consensus score
    pub consensus_score: f64,

    /// Validation errors
    pub errors: Vec<String>,
}

/// Consensus operation for tracking
#[derive(Debug, Clone)]
pub struct ConsensusOperation {
    pub operation_id: String,
    pub operation_type: String,
    pub proof: ConsensusProof,
    pub status: OperationStatus,
    pub created_at: SystemTime,
    pub completed_at: Option<SystemTime>,
}

/// Operation status
#[derive(Debug, Clone)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Consensus statistics
#[derive(Debug, Clone, Default)]
pub struct ConsensusStats {
    pub total_validations: u64,
    pub successful_validations: u64,
    pub failed_validations: u64,
    pub average_validation_time_ms: f64,
    pub proofs_by_type: HashMap<String, u64>,
}

/// Validator signature for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSignature {
    pub validator_id: String,
    pub signature: String,
    pub timestamp: SystemTime,
}