//! State management error types

use nexus_shared::NexusError;

/// Result type alias for state operations
pub type Result<T> = std::result::Result<T, StateError>;

/// State management specific error types
#[derive(thiserror::Error, Debug)]
pub enum StateError {
    #[error("Consensus error: {message}")]
    Consensus { message: String },

    #[error("Storage error: {message}")]
    Storage { message: String },

    #[error("Replication error: {message}")]
    Replication { message: String },

    #[error("Transaction error: {message}")]
    Transaction { message: String },

    #[error("Encryption error: {message}")]
    Encryption { message: String },

    #[error("Sharding error: {message}")]
    Sharding { message: String },

    #[error("Leadership election failed: {message}")]
    Leadership { message: String },

    #[error("Cluster membership error: {message}")]
    Membership { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Key not found: {key}")]
    KeyNotFound { key: String },

    #[error("Key already exists: {key}")]
    KeyExists { key: String },

    #[error("Invalid key format: {key}")]
    InvalidKey { key: String },

    #[error("Transaction conflict on key: {key}")]
    TransactionConflict { key: String },

    #[error("Transaction timeout after {duration_ms}ms")]
    TransactionTimeout { duration_ms: u64 },

    #[error("Quorum not available: need {required}, have {available}")]
    QuorumNotAvailable { required: usize, available: usize },

    #[error("Node not in cluster: {node_id}")]
    NodeNotInCluster { node_id: String },

    #[error("Split brain detected: multiple leaders")]
    SplitBrain,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Time error: {0}")]
    Time(#[from] std::time::SystemTimeError),

    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    // #[error("RocksDB error: {0}")]
    // RocksDb(#[from] rocksdb::Error),  // Temporarily disabled for emergency stabilization
}

impl StateError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            StateError::QuorumNotAvailable { .. } => true,
            StateError::TransactionConflict { .. } => true,
            StateError::TransactionTimeout { .. } => true,
            StateError::Leadership { .. } => true,
            StateError::Io(_) => true,
            StateError::Join(_) => true,
            _ => false,
        }
    }

    /// Check if the error is related to leadership
    pub fn is_leadership_error(&self) -> bool {
        matches!(
            self,
            StateError::Leadership { .. } | StateError::SplitBrain
        )
    }

    /// Check if the error is related to consensus
    pub fn is_consensus_error(&self) -> bool {
        matches!(
            self,
            StateError::Consensus { .. } | StateError::QuorumNotAvailable { .. }
        )
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            StateError::Consensus { .. } => "consensus",
            StateError::Storage { .. } => "storage",
            StateError::Replication { .. } => "replication",
            StateError::Transaction { .. } => "transaction",
            StateError::Encryption { .. } => "encryption",
            StateError::Sharding { .. } => "sharding",
            StateError::Leadership { .. } => "leadership",
            StateError::Membership { .. } => "membership",
            StateError::Configuration { .. } => "configuration",
            StateError::KeyNotFound { .. } => "key_not_found",
            StateError::KeyExists { .. } => "key_exists",
            StateError::InvalidKey { .. } => "invalid_key",
            StateError::TransactionConflict { .. } => "transaction_conflict",
            StateError::TransactionTimeout { .. } => "transaction_timeout",
            StateError::QuorumNotAvailable { .. } => "quorum",
            StateError::NodeNotInCluster { .. } => "node_not_in_cluster",
            StateError::SplitBrain => "split_brain",
            StateError::Serialization(_) => "serialization",
            StateError::Io(_) => "io",
            StateError::Time(_) => "time",
            StateError::Join(_) => "join",
            // StateError::RocksDb(_) => "rocksdb",  // Temporarily disabled
        }
    }
}

impl From<StateError> for NexusError {
    fn from(err: StateError) -> Self {
        match err {
            StateError::Io(io_err) => NexusError::Network(io_err),
            StateError::Configuration { message } => NexusError::Config(message),
            StateError::Serialization(serde_err) => {
                NexusError::Internal {
                    message: format!("Serialization error: {}", serde_err),
                }
            }
            other => NexusError::Internal {
                message: other.to_string(),
            },
        }
    }
}

/// Convenience macros for creating state errors
#[macro_export]
macro_rules! consensus_error {
    ($msg:expr) => {
        StateError::Consensus {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! storage_error {
    ($msg:expr) => {
        StateError::Storage {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! transaction_error {
    ($msg:expr) => {
        StateError::Transaction {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! leadership_error {
    ($msg:expr) => {
        StateError::Leadership {
            message: $msg.to_string(),
        }
    };
}