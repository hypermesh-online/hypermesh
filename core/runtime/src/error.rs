//! Runtime error types

use nexus_shared::{NexusError, ResourceId};

/// Result type alias for runtime operations
pub type Result<T> = std::result::Result<T, RuntimeError>;

/// Runtime-specific error types
#[derive(thiserror::Error, Debug)]
pub enum RuntimeError {
    #[error("Container not found: {id}")]
    ContainerNotFound { id: ResourceId },

    #[error("Container is already running: {id}")]
    ContainerRunning { id: ResourceId },

    #[error("Container is not running: {id}")]
    ContainerNotRunning { id: ResourceId },

    #[error("Image not found: {name}:{tag}")]
    ImageNotFound { name: String, tag: String },

    #[error("Image pull failed: {message}")]
    ImagePullFailed { message: String },

    #[error("Isolation error: {message}")]
    Isolation { message: String },

    #[error("Resource allocation failed: {message}")]
    ResourceAllocation { message: String },

    #[error("Network configuration failed: {message}")]
    Network { message: String },

    #[error("Storage configuration failed: {message}")]
    Storage { message: String },

    #[error("Security policy violation: {message}")]
    Security { message: String },

    #[error("Process execution failed: {command}, exit_code: {exit_code}")]
    ProcessExecution { command: String, exit_code: i32 },

    #[error("Namespace operation failed: {operation}, error: {error}")]
    Namespace { operation: String, error: String },

    #[error("Cgroup operation failed: {message}")]
    Cgroup { message: String },

    #[error("Mount operation failed: {message}")]
    Mount { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("System call failed: {syscall}, errno: {errno}")]
    System { syscall: String, errno: i32 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Time error: {0}")]
    Time(#[from] std::time::SystemTimeError),

    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    // Consensus orchestration errors
    #[error("Consensus error: {message}")]
    ConsensusError { message: String },

    #[error("Consensus timeout for operation {operation_id}, timeout: {timeout:?}")]
    ConsensusTimeout { 
        operation_id: u64, 
        timeout: std::time::Duration 
    },

    #[error("Invalid operation: {message}")]
    InvalidOperation { message: String },

    #[error("State synchronization error: {message}")]
    StateError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },
    
    #[error("Network error: {message}")]
    NetworkError { message: String },
    
    #[error("Transport error: {message}")]
    Transport { message: String },
    
    #[error("Byzantine error: {message}")]
    ByzantineError { message: String },

    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl RuntimeError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            RuntimeError::ImagePullFailed { .. } => true,
            RuntimeError::ResourceAllocation { .. } => true,
            RuntimeError::Network { .. } => true,
            RuntimeError::Storage { .. } => true,
            RuntimeError::System { .. } => true,
            RuntimeError::Io(_) => true,
            RuntimeError::ConsensusTimeout { .. } => true,
            RuntimeError::StateError { .. } => true,
            _ => false,
        }
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            RuntimeError::ContainerNotFound { .. } => "container_not_found",
            RuntimeError::ContainerRunning { .. } => "container_running",
            RuntimeError::ContainerNotRunning { .. } => "container_not_running",
            RuntimeError::ImageNotFound { .. } => "image_not_found",
            RuntimeError::ImagePullFailed { .. } => "image_pull",
            RuntimeError::Isolation { .. } => "isolation",
            RuntimeError::ResourceAllocation { .. } => "resources",
            RuntimeError::Network { .. } => "network",
            RuntimeError::Storage { .. } => "storage",
            RuntimeError::Security { .. } => "security",
            RuntimeError::ProcessExecution { .. } => "process",
            RuntimeError::Namespace { .. } => "namespace",
            RuntimeError::Cgroup { .. } => "cgroup",
            RuntimeError::Mount { .. } => "mount",
            RuntimeError::Configuration { .. } => "configuration",
            RuntimeError::System { .. } => "system",
            RuntimeError::Io(_) => "io",
            RuntimeError::Json(_) => "json",
            RuntimeError::Time(_) => "time",
            RuntimeError::Join(_) => "join",
            RuntimeError::ConsensusError { .. } => "consensus",
            RuntimeError::ConsensusTimeout { .. } => "consensus_timeout",
            RuntimeError::InvalidOperation { .. } => "invalid_operation",
            RuntimeError::StateError { .. } => "state_error",
            RuntimeError::SerializationError { .. } => "serialization",
            RuntimeError::NetworkError { .. } => "network",
            RuntimeError::Transport { .. } => "transport",
            RuntimeError::ByzantineError { .. } => "byzantine",
            RuntimeError::LockPoisoned(_) => "lock_poisoned",
            RuntimeError::Internal(_) => "internal",
        }
    }
}

impl From<RuntimeError> for NexusError {
    fn from(err: RuntimeError) -> Self {
        match err {
            RuntimeError::Io(io_err) => NexusError::Network(io_err),
            RuntimeError::Configuration { message } => NexusError::Config(message),
            RuntimeError::Security { message } => NexusError::Authorization { resource: message },
            other => NexusError::Internal {
                message: other.to_string(),
            },
        }
    }
}

/// Convenience macros for creating runtime errors
#[macro_export]
macro_rules! isolation_error {
    ($msg:expr) => {
        RuntimeError::Isolation {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! resource_error {
    ($msg:expr) => {
        RuntimeError::ResourceAllocation {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! security_error {
    ($msg:expr) => {
        RuntimeError::Security {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! system_error {
    ($syscall:expr, $errno:expr) => {
        RuntimeError::System {
            syscall: $syscall.to_string(),
            errno: $errno,
        }
    };
}