//! Common utilities and types

use thiserror::Error;

/// Common error types
#[derive(Debug, Error)]
pub enum HyperMeshError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Consensus validation failed: {0}")]
    ConsensusError(String),

    #[error("Asset error: {0}")]
    AssetError(String),
}

/// Common result type
pub type Result<T> = std::result::Result<T, HyperMeshError>;
