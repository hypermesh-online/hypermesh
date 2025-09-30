//! Phoenix SDK Error Types
//!
//! Provides comprehensive error handling for Phoenix SDK operations.

use std::fmt;
use thiserror::Error;

/// Phoenix SDK Result type
pub type Result<T> = std::result::Result<T, PhoenixError>;

/// Phoenix SDK Error types
#[derive(Error, Debug)]
pub enum PhoenixError {
    /// Transport layer error
    #[error("Transport error: {0}")]
    TransportError(String),

    /// Security error
    #[error("Security error: {0}")]
    SecurityError(String),

    /// Connection closed
    #[error("Connection closed")]
    ConnectionClosed,

    /// Connection limit exceeded
    #[error("Connection limit exceeded")]
    ConnectionLimitExceeded,

    /// Invalid target address
    #[error("Invalid target '{target}': {reason}")]
    InvalidTarget { target: String, reason: String },

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Compression error
    #[error("Compression error: {0}")]
    CompressionError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Resource exhausted
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Not implemented
    #[error("Feature not implemented: {0}")]
    NotImplemented(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for PhoenixError {
    fn from(err: std::io::Error) -> Self {
        PhoenixError::TransportError(err.to_string())
    }
}

impl From<anyhow::Error> for PhoenixError {
    fn from(err: anyhow::Error) -> Self {
        PhoenixError::Other(err.to_string())
    }
}

impl From<trustchain::TrustChainError> for PhoenixError {
    fn from(err: trustchain::TrustChainError) -> Self {
        PhoenixError::SecurityError(err.to_string())
    }
}