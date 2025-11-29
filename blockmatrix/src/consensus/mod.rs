//! HyperMesh Consensus System
//!
//! This module re-exports the Proof of State consensus system from TrustChain.
//! TrustChain implements the four-proof consensus (WHO, WHEN, WHERE, WHAT).
//!
//! For the full consensus implementation, see trustchain::consensus module.

// Submodule for nested import compatibility
pub mod proof;
pub mod validation;

// Re-export all consensus types from TrustChain
pub use trustchain::consensus::*;

// Backward compatibility type aliases for legacy naming convention
// These map the old ProofOf* names to the new *Proof names
pub type ProofOfStake = StakeProof;
pub type ProofOfTime = TimeProof;
pub type ProofOfSpace = SpaceProof;
pub type ProofOfWork = WorkProof;

// BlockMatrix-specific consensus types that extend TrustChain
use serde::{Serialize, Deserialize};

/// Access level for resources in the HyperMesh network
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccessLevel {
    /// Public data accessible to all networks
    Public,
    /// Private data within organization only
    Private,
    /// Federated sharing with trusted partners
    Federated,
    /// Restricted access requiring special permissions
    Restricted,
}

/// Network position information for node topology
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkPosition {
    /// Geographic region or data center
    pub region: String,
    /// Network zone within region
    pub zone: String,
    /// Rack or cluster identifier
    pub cluster_id: String,
    /// Node identifier within cluster
    pub node_id: String,
}

/// Access permissions configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessPermissions {
    /// Access level for this resource
    pub level: AccessLevel,
    /// Required roles for access
    pub required_roles: Vec<String>,
    /// Allowed IP ranges (IPv6)
    pub allowed_networks: Vec<String>,
    /// Whether to require consensus validation
    pub require_consensus: bool,
}

/// Consensus error types for BlockMatrix
#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Insufficient proofs: {0}")]
    InsufficientProofs(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Consensus trait for validation
pub trait Consensus {
    fn validate(&self) -> Result<bool, ConsensusError>;
    fn generate_proof(&self) -> Result<ConsensusProof, ConsensusError>;
}

/// Log index for blockchain operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogIndex {
    pub block_height: u64,
    pub transaction_index: u32,
    pub log_index: u32,
}

/// Consensus configuration for BlockMatrix
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Minimum stake required for validation
    pub min_stake: u64,
    /// Maximum time offset allowed
    pub max_time_offset: std::time::Duration,
    /// Minimum storage capacity
    pub min_storage: u64,
    /// Minimum computational power
    pub min_compute_power: u64,
    /// Byzantine fault tolerance threshold
    pub byzantine_threshold: f64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            min_stake: 1000,
            max_time_offset: std::time::Duration::from_secs(300),
            min_storage: 1024 * 1024 * 1024, // 1GB
            min_compute_power: 100,
            byzantine_threshold: 0.33,
        }
    }
}

// Placeholder modules for missing imports (to be implemented)
pub mod validation_service {
    use super::*;

    pub struct ValidationService;

    impl ValidationService {
        pub fn new() -> Self {
            Self
        }
    }

    // Trait for consensus validation service
    pub trait ConsensusValidationService {
        fn validate(&self, proof: &ConsensusProof) -> Result<bool, ConsensusError>;
    }

    impl ConsensusValidationService for ValidationService {
        fn validate(&self, _proof: &ConsensusProof) -> Result<bool, ConsensusError> {
            // Placeholder implementation
            Ok(true)
        }
    }
}

pub mod stoq_handlers {
    use super::*;

    pub struct StoqHandler;

    impl StoqHandler {
        pub fn new() -> Self {
            Self
        }
    }

    // Handler types for API
    pub struct ValidateCertificateHandler;
    pub struct ValidateProofsHandler;
    pub struct ValidationStatusHandler;
    pub struct ConsensusHealthHandler;

    impl ValidateCertificateHandler {
        pub fn new() -> Self {
            Self
        }
    }

    impl ValidateProofsHandler {
        pub fn new() -> Self {
            Self
        }
    }

    impl ValidationStatusHandler {
        pub fn new() -> Self {
            Self
        }
    }

    impl ConsensusHealthHandler {
        pub fn new() -> Self {
            Self
        }
    }
}

pub mod proof_of_state_integration {
    use super::*;

    // Re-export all consensus types for compatibility
    pub use super::{
        ConsensusProof,
        SpaceProof,
        StakeProof,
        WorkProof,
        TimeProof,
        WorkloadType,
        WorkState,
        Proof,
    };

    pub struct ProofOfStateIntegration;

    impl ProofOfStateIntegration {
        pub fn new() -> Self {
            Self
        }
    }

    // Additional types that may be required
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ClientCredentials {
        pub client_id: String,
        pub client_secret: String,
    }
}

// Additional blockmatrix-specific consensus types can be added here
