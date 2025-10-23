//! Container runtime error types

use thiserror::Error;
use std::io;

/// Container runtime result type
pub type Result<T> = std::result::Result<T, ContainerError>;

/// Container runtime errors
#[derive(Debug, Error)]
pub enum ContainerError {
    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    /// Container not found
    #[error("Container not found: {id}")]
    NotFound { id: String },
    
    /// Container already exists
    #[error("Container already exists: {id}")]
    AlreadyExists { id: String },
    
    /// Invalid container state
    #[error("Invalid container state: expected {expected}, found {actual}")]
    InvalidState { expected: String, actual: String },
    
    /// Image operation failed
    #[error("Image error: {message}")]
    Image { message: String },
    
    /// Network operation failed
    #[error("Network error: {message}")]
    Network { message: String },
    
    /// Filesystem operation failed
    #[error("Filesystem error: {message}")]
    Filesystem { message: String },
    
    /// Resource management failed
    #[error("Resource error: {message}")]
    Resource { message: String },
    
    /// Migration operation failed
    #[error("Migration error: {message}")]
    Migration { message: String },
    
    /// Security validation failed
    #[error("Security error: {message}")]
    Security { message: String },
    
    /// Configuration error
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    /// Runtime error
    #[error("Runtime error: {message}")]
    Runtime { message: String },
    
    /// Timeout error
    #[error("Operation timed out after {duration:?}")]
    Timeout { duration: std::time::Duration },
    
    /// Permission denied
    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },
    
    /// Insufficient resources
    #[error("Insufficient resources: {resource}")]
    InsufficientResources { resource: String },
}

impl ContainerError {
    /// Create a new image error
    pub fn image(message: impl Into<String>) -> Self {
        Self::Image { message: message.into() }
    }
    
    /// Create a new network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network { message: message.into() }
    }
    
    /// Create a new filesystem error
    pub fn filesystem(message: impl Into<String>) -> Self {
        Self::Filesystem { message: message.into() }
    }
    
    /// Create a new resource error
    pub fn resource(message: impl Into<String>) -> Self {
        Self::Resource { message: message.into() }
    }
    
    /// Create a new migration error
    pub fn migration(message: impl Into<String>) -> Self {
        Self::Migration { message: message.into() }
    }
    
    /// Create a new security error
    pub fn security(message: impl Into<String>) -> Self {
        Self::Security { message: message.into() }
    }
    
    /// Create a new config error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config { message: message.into() }
    }
    
    /// Create a new runtime error
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime { message: message.into() }
    }
}