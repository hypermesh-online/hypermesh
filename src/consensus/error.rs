//! Error types for the consensus system

use thiserror::Error;
use super::transport::NodeId;

/// Result type alias for consensus operations
pub type Result<T> = std::result::Result<T, ConsensusError>;

/// Comprehensive error types for the consensus system
#[derive(Error, Debug)]
pub enum ConsensusError {
    /// Error in the Raft consensus algorithm
    #[error("Raft consensus error: {0}")]
    RaftError(String),
    
    /// Log-related errors
    #[error("Log error: {0}")]
    LogError(String),
    
    /// Storage backend errors
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// Byzantine fault tolerance errors
    #[error("Byzantine error: node {node_id} - {message}")]
    ByzantineError { node_id: String, message: String },
    
    /// Transaction processing errors
    #[error("Transaction error: {0}")]
    TransactionError(String),
    
    /// Network/transport errors
    #[error("Network error: {0}")]
    NetworkError(String),
    
    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    /// Leader election errors
    #[error("Election error: {0}")]
    ElectionError(String),
    
    /// Sharding-related errors
    #[error("Shard error: shard {shard_id} - {message}")]
    ShardError { shard_id: String, message: String },
    
    /// Timeout errors
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    
    /// Invalid state errors
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    /// Security-related errors
    #[error("Security error: {0}")]
    SecurityError(String),
    
    /// Node isolation errors
    #[error("Isolation error: {0}")]
    IsolationError(String),
    
    /// Recovery operation errors
    #[error("Recovery error: {0}")]
    RecoveryError(String),
    
    /// Attack prevention errors
    #[error("Attack prevention error: {0}")]
    AttackPreventionError(String),
    
    /// Quantum security errors
    #[error("Quantum security error: {0}")]
    QuantumSecurityError(String),
    
    /// Service not initialized errors
    #[error("Service not initialized: {0}")]
    ServiceNotInitialized(String),
    
    /// General service errors
    #[error("Service error: {0}")]
    ServiceError(String),
    
    /// Reputation management errors
    #[error("Reputation error: {0}")]
    ReputationError(String),
    
    /// Real-time detection errors
    #[error("Detection error: {0}")]
    DetectionError(String),
    
    /// Proof of Work validation failed
    #[error("Proof of Work validation failed")]
    ProofOfWorkFailed,
    
    /// Internal system error
    #[error("Internal error: {0}")]
    Internal(String),
    
    /// System not ready for operation
    #[error("System not ready: {message}")]
    NotReady { message: String },
    
    /// Invalid storage commitment in Proof of Space
    #[error("Invalid storage commitment")]
    InvalidStorageCommitment,
    
    /// Invalid network position in Proof of Space
    #[error("Invalid network position")]
    InvalidNetworkPosition,
    
    /// Network position too distant for reliable operation
    #[error("Network position too distant")]
    NetworkPositionTooDistant,
    
    /// Insufficient authority level for asset operation
    #[error("Insufficient authority level")]
    InsufficientAuthority,
    
    /// Invalid stake holder identity
    #[error("Invalid stake holder")]
    InvalidStakeHolder,
    
    /// Invalid allowance format
    #[error("Invalid allowance")]
    InvalidAllowance,
    
    /// Invalid work proof computation
    #[error("Invalid work proof")]
    InvalidWorkProof,
    
    /// Insufficient difficulty for Proof of Work
    #[error("Insufficient difficulty")]
    InsufficientDifficulty,
    
    /// Invalid timestamp in Proof of Time
    #[error("Invalid timestamp")]
    InvalidTimestamp,
    
    /// Timestamp drift exceeded acceptable bounds
    #[error("Timestamp drift exceeded")]
    TimestampDriftExceeded,
}

/// Result type alias using our custom error type
pub type ConsensusResult<T> = std::result::Result<T, ConsensusError>;

impl From<std::io::Error> for ConsensusError {
    fn from(err: std::io::Error) -> Self {
        ConsensusError::StorageError(err.to_string())
    }
}

impl From<serde_json::Error> for ConsensusError {
    fn from(err: serde_json::Error) -> Self {
        ConsensusError::SerializationError(err.to_string())
    }
}

impl From<bincode::Error> for ConsensusError {
    fn from(err: bincode::Error) -> Self {
        ConsensusError::SerializationError(err.to_string())
    }
}

#[cfg(feature = "rocksdb-storage")]
impl From<rocksdb::Error> for ConsensusError {
    fn from(err: rocksdb::Error) -> Self {
        ConsensusError::StorageError(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for ConsensusError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        ConsensusError::TimeoutError("Operation timed out".to_string())
    }
}

impl From<String> for ConsensusError {
    fn from(err: String) -> Self {
        ConsensusError::QuantumSecurityError(err)
    }
}