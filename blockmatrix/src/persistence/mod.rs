// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Matrix Persistence Layer Implementation
//!
//! Provides persistent storage for the Block-MATRIX distributed computing platform,
//! ensuring all state (matrix coordinates, every-node-blockchain data, geospatial
//! mappings, network topology) survives node restarts with zero data loss.

pub mod blockchain_storage;
pub mod manager;
pub mod matrix_state;
pub mod recovery;
pub mod snapshots;
pub mod topology_backup;

// Re-export main types
pub use blockchain_storage::{BlockQuery, BlockchainStorage, ChainMetadata};
pub use manager::{PersistenceConfig, PersistenceManager, StorageStats};
pub use matrix_state::{MatrixState, MatrixStateSerializer, SerializationFormat};
pub use recovery::{RecoveryManager, RecoveryReport, RecoveryStatus};
pub use snapshots::{SnapshotManager, SnapshotMetadata, SnapshotSchedule};
pub use topology_backup::{BackupMode, TopologyBackup};

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

    /// Block integrity violation detected — possible tampering
    ///
    /// SECURITY REVIEW REQUIRED: This error indicates the persisted block's
    /// canonical hash does not match the stored/expected hash. This could mean:
    /// 1. Data corruption (disk error)
    /// 2. Format incompatibility (software bug)
    /// 3. Intentional tampering (security breach)
    ///
    /// The node MUST NOT accept this block. Manual investigation required.
    /// See papers/HYPERMESH.md §6.2 and §7.2 for the security model.
    #[error("Block integrity violation at index {index}: stored hash {stored_hash}, computed hash {computed_hash} — possible tampering, SECURITY REVIEW REQUIRED")]
    IntegrityViolation {
        index: u64,
        stored_hash: String,
        computed_hash: String,
    },
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
