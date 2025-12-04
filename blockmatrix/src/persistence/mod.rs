//! Matrix Persistence Layer Implementation
//!
//! Provides persistent storage for the Block-MATRIX distributed computing platform,
//! ensuring all state (matrix coordinates, every-node-blockchain data, geospatial
//! mappings, network topology) survives node restarts with zero data loss.

pub mod matrix_state;
pub mod blockchain_storage;
pub mod topology_backup;
pub mod snapshots;
pub mod recovery;
pub mod manager;

// Re-export main types
pub use matrix_state::{MatrixStateSerializer, MatrixState, SerializationFormat};
pub use blockchain_storage::{BlockchainStorage, BlockQuery, ChainMetadata};
pub use topology_backup::{TopologyBackup, BackupMode};
pub use snapshots::{SnapshotManager, SnapshotMetadata, SnapshotSchedule};
pub use recovery::{RecoveryManager, RecoveryReport, RecoveryStatus};
pub use manager::{PersistenceManager, PersistenceConfig, StorageStats};

use thiserror::Error;

/// Errors that can occur during persistence operations
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// I/O error during file operations
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Compression error
    #[error("Compression error: {0}")]
    Compression(String),

    /// Decompression error
    #[error("Decompression error: {0}")]
    Decompression(String),

    /// Checksum validation failed
    #[error("Checksum validation failed: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// Version mismatch
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },

    /// Recovery failed
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    /// Snapshot error
    #[error("Snapshot error: {0}")]
    SnapshotError(String),

    /// Disk space error
    #[error("Insufficient disk space: needed {needed} bytes, available {available} bytes")]
    InsufficientDiskSpace { needed: u64, available: u64 },

    /// Lock error
    #[error("Failed to acquire lock: {0}")]
    LockError(String),

    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

pub type PersistenceResult<T> = Result<T, PersistenceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PersistenceError::ChecksumMismatch {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Checksum validation failed: expected abc123, got def456"
        );
    }
}

#[cfg(test)]
mod test_integration;