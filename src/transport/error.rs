//! Error types for HyperMesh Transport Layer

use thiserror::Error;
// NodeId is a HyperMesh concept, not from stoq
use super::types::NodeId;

/// Transport result type alias
pub type Result<T> = std::result::Result<T, TransportError>;

/// Transport layer errors
#[derive(Error, Debug)]
pub enum TransportError {
    /// STOQ transport error
    #[error("STOQ transport error: {0}")]
    StoqError(#[from] anyhow::Error),

    /// Connection not found
    #[error("No connection found for node: {0}")]
    NoConnection(String),

    /// Connection is inactive
    #[error("Connection to node {0} is inactive")]
    ConnectionInactive(String),

    /// Authentication failed with details
    #[error("Authentication failed for node {0}: {1}")]
    AuthenticationFailedWithDetails(String, String),

    /// Not authenticated
    #[error("Connection not authenticated")]
    NotAuthenticated,

    /// Authentication failed (simple)
    #[error("Authentication failed")]
    AuthenticationFailed,

    /// Missing node ID
    #[error("Missing node ID for authentication")]
    MissingNodeId,

    /// Certificate validation error
    #[error("Certificate validation failed: {0}")]
    CertificateValidation(String),

    /// Connection closed error
    #[error("Connection is closed")]
    ConnectionClosed,

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Checksum mismatch error
    #[error("Checksum mismatch - data corruption detected")]
    ChecksumMismatch,

    /// Message ID mismatch error
    #[error("Message ID mismatch - protocol error")]
    MessageIdMismatch,

    /// Missing chunk error
    #[error("Missing chunk {0} in message")]
    MissingChunk(u32),
    
    /// Connection pool full
    #[error("Connection pool is full (max: {max_size})")]
    PoolFull { max_size: usize },
    
    /// Connection timeout
    #[error("Connection timeout after {timeout:?}")]
    ConnectionTimeout { timeout: std::time::Duration },
    
    /// Network error
    #[error("Network error: {0}")]
    Network(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// Monitoring error
    #[error("Monitoring error: {0}")]
    Monitoring(String),
    
    /// Resource exhaustion
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },
    
    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    
    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl TransportError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            // Temporary failures that can be retried
            Self::ConnectionTimeout { .. } |
            Self::Network(_) |
            Self::ResourceExhausted { .. } => true,
            
            // Permanent failures that should not be retried
            Self::AuthenticationFailedWithDetails(_, _) |
            Self::AuthenticationFailed |
            Self::NotAuthenticated |
            Self::MissingNodeId |
            Self::CertificateValidation(_) |
            Self::Configuration(_) |
            Self::InvalidOperation(_) => false,
            
            // Connection issues may be retryable depending on context
            Self::NoConnection(_) |
            Self::ConnectionInactive(_) => true,
            
            // Pool full is retryable after backoff
            Self::PoolFull { .. } => true,
            
            // Check underlying error for STOQ errors
            Self::StoqError(err) => {
                // Heuristic: assume network-related errors are retryable
                let err_str = err.to_string().to_lowercase();
                err_str.contains("timeout") || 
                err_str.contains("network") || 
                err_str.contains("connection")
            }
            
            // Other errors default to not retryable
            _ => false,
        }
    }
    
    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            Self::StoqError(_) => "stoq",
            Self::NoConnection(_) | Self::ConnectionInactive(_) => "connection",
            Self::AuthenticationFailed | Self::CertificateValidation(_) => "auth",
            Self::PoolFull { .. } => "pool",
            Self::ConnectionTimeout { .. } => "timeout",
            Self::Network(_) => "network",
            Self::Serialization(_) => "serialization",
            Self::Configuration(_) => "config",
            Self::Monitoring(_) => "monitoring",
            Self::ResourceExhausted { .. } => "resources",
            Self::InvalidOperation(_) => "operation",
            Self::Protocol(_) => "protocol",
            Self::Io(_) => "io",
            Self::AuthenticationFailedWithDetails(_, _) => "auth",
            Self::NotAuthenticated => "auth",
            Self::MissingNodeId => "auth",
            Self::ConnectionClosed => "connection",
            Self::SerializationError(_) => "serialization",
            Self::ChecksumMismatch => "integrity",
            Self::MessageIdMismatch => "protocol",
            Self::MissingChunk(_) => "protocol",
        }
    }
    
    /// Create a network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network(message.into())
    }
    
    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Configuration(message.into())
    }
    
    /// Create a monitoring error
    pub fn monitoring<S: Into<String>>(message: S) -> Self {
        Self::Monitoring(message.into())
    }
    
    /// Create an invalid operation error
    pub fn invalid_op<S: Into<String>>(message: S) -> Self {
        Self::InvalidOperation(message.into())
    }
    
    /// Create a protocol error
    pub fn protocol<S: Into<String>>(message: S) -> Self {
        Self::Protocol(message.into())
    }
    
    /// Create a resource exhausted error
    pub fn resource_exhausted<S: Into<String>>(resource: S) -> Self {
        Self::ResourceExhausted { resource: resource.into() }
    }
}

/// Convert from common error types
impl From<quinn::ConnectionError> for TransportError {
    fn from(err: quinn::ConnectionError) -> Self {
        Self::Network(err.to_string())
    }
}

impl From<quinn::ConnectError> for TransportError {
    fn from(err: quinn::ConnectError) -> Self {
        Self::Network(err.to_string())
    }
}

impl From<rustls::Error> for TransportError {
    fn from(err: rustls::Error) -> Self {
        Self::CertificateValidation(err.to_string())
    }
}

impl From<config::ConfigError> for TransportError {
    fn from(err: config::ConfigError) -> Self {
        Self::Configuration(err.to_string())
    }
}

/// Error context extension trait
pub trait TransportErrorContext<T> {
    /// Add transport-specific context to errors
    fn transport_context(self, context: &str) -> Result<T>;
    
    /// Add node context to errors
    fn node_context(self, node_id: &NodeId) -> Result<T>;
}

impl<T, E> TransportErrorContext<T> for std::result::Result<T, E>
where
    E: Into<TransportError>,
{
    fn transport_context(self, context: &str) -> Result<T> {
        self.map_err(|e| {
            let transport_err = e.into();
            match transport_err {
                TransportError::StoqError(err) => {
                    TransportError::StoqError(err.context(context.to_string()))
                }
                other => other,
            }
        })
    }
    
    fn node_context(self, node_id: &NodeId) -> Result<T> {
        self.map_err(|e| {
            let transport_err = e.into();
            let context = format!("Node: {:?}", node_id);
            match transport_err {
                TransportError::StoqError(err) => {
                    TransportError::StoqError(err.context(context))
                }
                TransportError::Network(msg) => {
                    TransportError::Network(format!("{} ({})", msg, context))
                }
                other => other,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_categories() {
        let errors = vec![
            (TransportError::network("test"), "network"),
            (TransportError::config("test"), "config"),
            (TransportError::NoConnection(NodeId::new("test".to_string())), "connection"),
            (TransportError::PoolFull { max_size: 100 }, "pool"),
        ];
        
        for (error, expected_category) in errors {
            assert_eq!(error.category(), expected_category);
        }
    }
    
    #[test]
    fn test_retryable_errors() {
        let retryable = vec![
            TransportError::ConnectionTimeout { timeout: std::time::Duration::from_secs(1) },
            TransportError::Network("test".to_string()),
            TransportError::PoolFull { max_size: 100 },
        ];
        
        let not_retryable = vec![
            TransportError::AuthenticationFailed(NodeId::new("test".to_string()), "test".to_string()),
            TransportError::Configuration("test".to_string()),
            TransportError::InvalidOperation("test".to_string()),
        ];
        
        for error in retryable {
            assert!(error.is_retryable(), "Expected {} to be retryable", error);
        }
        
        for error in not_retryable {
            assert!(!error.is_retryable(), "Expected {} to not be retryable", error);
        }
    }
}