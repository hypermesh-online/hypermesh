//! Error types and handling for Nexus components


/// Result type alias for Nexus operations
pub type Result<T> = std::result::Result<T, NexusError>;

/// Primary error type for all Nexus operations
#[derive(thiserror::Error, Debug)]
pub enum NexusError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Transport error: {message}")]
    Transport { message: String },

    #[error("Authentication failed: {reason}")]
    Authentication { reason: String },

    #[error("Authorization denied: {resource}")]
    Authorization { resource: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Resource not found: {resource_type} {id}")]
    ResourceNotFound {
        resource_type: String,
        id: String,
    },

    #[error("Resource conflict: {message}")]
    ResourceConflict { message: String },

    #[error("Timeout occurred after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    #[error("Consensus error: {message}")]
    Consensus { message: String },

    #[error("Storage error: {message}")]
    Storage { message: String },

    #[error("Invalid state: {message}")]
    InvalidState { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },

    #[error("System error: {message}")]
    System { message: String },
}

impl NexusError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            NexusError::Network(_) => true,
            NexusError::Timeout { .. } => true,
            NexusError::Storage { .. } => true,
            NexusError::Consensus { .. } => true,
            _ => false,
        }
    }

    /// Get error category for metrics/logging
    pub fn category(&self) -> &'static str {
        match self {
            NexusError::Network(_) => "network",
            NexusError::Serialization(_) => "serialization", 
            NexusError::Transport { .. } => "transport",
            NexusError::Authentication { .. } => "auth",
            NexusError::Authorization { .. } => "authz",
            NexusError::Config(_) => "config",
            NexusError::ResourceNotFound { .. } => "not_found",
            NexusError::ResourceConflict { .. } => "conflict",
            NexusError::Timeout { .. } => "timeout",
            NexusError::Consensus { .. } => "consensus",
            NexusError::Storage { .. } => "storage",
            NexusError::InvalidState { .. } => "invalid_state",
            NexusError::Internal { .. } => "internal",
            NexusError::System { .. } => "system",
        }
    }
}

/// Convenience macros for creating specific error types
#[macro_export]
macro_rules! transport_error {
    ($msg:expr) => {
        NexusError::Transport {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! auth_error {
    ($reason:expr) => {
        NexusError::Authentication {
            reason: $reason.to_string(),
        }
    };
}

#[macro_export]
macro_rules! internal_error {
    ($msg:expr) => {
        NexusError::Internal {
            message: $msg.to_string(),
        }
    };
}