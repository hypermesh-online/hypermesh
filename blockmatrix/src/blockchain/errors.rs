// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Error types for blockchain operations
//!
//! Provides structured error types for better error handling and debugging.

use thiserror::Error;
use std::io;

/// Error type for blockchain operations
#[derive(Error, Debug)]
pub enum BlockchainError {
    /// Block validation failed
    #[error("Block validation failed: {reason}")]
    ValidationError { reason: String },

    /// Block not found by index
    #[error("Block {index} not found")]
    BlockNotFound { index: u64 },

    /// Block not found by hash
    #[error("Block with hash '{hash}' not found")]
    BlockNotFoundByHash { hash: String },

    /// Duplicate block detected
    #[error("Block {index} already exists")]
    DuplicateBlock { index: u64 },

    /// Chain head not available
    #[error("Chain head not found")]
    NoHeadBlock,

    /// Invalid block index sequence
    #[error("Invalid block index: expected {expected}, got {actual}")]
    InvalidIndex { expected: u64, actual: u64 },

    /// Block size exceeds maximum
    #[error("Block size {size} exceeds maximum {max}")]
    BlockTooLarge { size: usize, max: usize },

    /// Time validation failed
    #[error("Time validation failed: {reason}")]
    TimeValidation { reason: String },

    /// Node ownership mismatch
    #[error("Block does not belong to node at ({x},{y},{z})")]
    OwnershipMismatch { x: i64, y: i64, z: i64 },

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    /// Storage error
    #[error("Storage error: {operation} failed: {reason}")]
    StorageError { operation: String, reason: String },

    /// Empty chain error
    #[error("Chain is empty")]
    EmptyChain,

    /// Invalid hash
    #[error("Invalid hash: {reason}")]
    InvalidHash { reason: String },

    /// Invalid signature
    #[error("Invalid signature: {reason}")]
    InvalidSignature { reason: String },

    /// Generic error with context
    #[error("{0}")]
    Other(String),
}

/// Result type alias for blockchain operations
pub type Result<T> = std::result::Result<T, BlockchainError>;

/// Error type for chain state management
#[derive(Error, Debug)]
pub enum StateError {
    /// Failed to initialize storage
    #[error("Failed to initialize storage at {path:?}: {reason}")]
    InitializationError { path: String, reason: String },

    /// Failed to create directory
    #[error("Failed to create directory {path:?}: {reason}")]
    DirectoryCreationError { path: String, reason: String },

    /// Failed to store block
    #[error("Failed to store block {index}: {reason}")]
    BlockStoreError { index: u64, reason: String },

    /// Failed to load block
    #[error("Failed to load block {index}: {reason}")]
    BlockLoadError { index: u64, reason: String },

    /// Failed to create snapshot
    #[error("Failed to create snapshot: {reason}")]
    SnapshotError { reason: String },

    /// Query error
    #[error("Query failed: {reason}")]
    QueryError { reason: String },

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

/// Result type alias for state operations
pub type StateResult<T> = std::result::Result<T, StateError>;

/// Error type for block propagation
#[derive(Error, Debug)]
pub enum PropagationError {
    /// No targets available
    #[error("No propagation targets available")]
    NoTargets,

    /// Propagation failed to node
    #[error("Failed to propagate to node ({x},{y},{z}): {reason}")]
    SendFailure {
        x: i64,
        y: i64,
        z: i64,
        reason: String,
    },

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Timeout error
    #[error("Propagation timeout after {elapsed_ms}ms")]
    Timeout { elapsed_ms: u64 },

    /// Block already propagated
    #[error("Block {hash} already propagated to node ({x},{y},{z})")]
    AlreadyPropagated {
        hash: String,
        x: i64,
        y: i64,
        z: i64,
    },
}

/// Result type alias for propagation operations
pub type PropagationResult<T> = std::result::Result<T, PropagationError>;

impl From<String> for BlockchainError {
    fn from(s: String) -> Self {
        BlockchainError::Other(s)
    }
}

impl From<&str> for BlockchainError {
    fn from(s: &str) -> Self {
        BlockchainError::Other(s.to_string())
    }
}

impl From<StateError> for BlockchainError {
    fn from(e: StateError) -> Self {
        BlockchainError::Other(e.to_string())
    }
}

impl From<PropagationError> for BlockchainError {
    fn from(e: PropagationError) -> Self {
        BlockchainError::Other(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BlockchainError::BlockNotFound { index: 42 };
        assert_eq!(err.to_string(), "Block 42 not found");

        let err = BlockchainError::InvalidIndex {
            expected: 5,
            actual: 7,
        };
        assert_eq!(err.to_string(), "Invalid block index: expected 5, got 7");

        let err = StateError::BlockStoreError {
            index: 10,
            reason: "disk full".to_string(),
        };
        assert_eq!(err.to_string(), "Failed to store block 10: disk full");
    }

    #[test]
    fn test_error_conversion() {
        let string_err = "test error".to_string();
        let blockchain_err: BlockchainError = string_err.into();
        assert_eq!(blockchain_err.to_string(), "test error");

        let str_err = "another error";
        let blockchain_err: BlockchainError = str_err.into();
        assert_eq!(blockchain_err.to_string(), "another error");
    }

    #[test]
    fn test_error_hierarchy() {
        let state_err = StateError::QueryError {
            reason: "invalid query".to_string(),
        };
        let blockchain_err: BlockchainError = state_err.into();
        assert!(blockchain_err.to_string().contains("Query failed"));
    }

    #[test]
    fn test_propagation_error() {
        let err = PropagationError::SendFailure {
            x: 1,
            y: 2,
            z: 3,
            reason: "connection refused".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Failed to propagate to node (1,2,3): connection refused"
        );
    }
}
