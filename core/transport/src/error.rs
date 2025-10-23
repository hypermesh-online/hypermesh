//! Transport layer error types

use nexus_shared::NexusError;

/// Result type alias for transport operations
pub type Result<T> = std::result::Result<T, TransportError>;

/// Transport-specific error types
#[derive(thiserror::Error, Debug)]
pub enum TransportError {
    #[error("QUIC connection error: {message}")]
    Connection { message: String },

    #[error("QUIC endpoint error: {message}")]
    Endpoint { message: String },

    #[error("Certificate error: {message}")]
    Certificate { message: String },

    #[error("TLS error: {message}")]
    Tls { message: String },

    #[error("Stream error: {message}")]
    Stream { message: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Authentication failed: {reason}")]
    Authentication { reason: String },

    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolVersion { expected: u32, actual: u32 },

    #[error("Timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("QUIC error: {0}")]
    Quinn(#[from] quinn::ConnectionError),

    #[error("Rustls error: {0}")]
    Rustls(#[from] rustls::Error),
}

impl TransportError {
    /// Check if the error is recoverable/retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            TransportError::Network(_) => true,
            TransportError::Timeout { .. } => true,
            TransportError::Connection { .. } => true,
            TransportError::Quinn(e) => match e {
                quinn::ConnectionError::TimedOut => true,
                quinn::ConnectionError::TransportError(_) => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            TransportError::Connection { .. } => "connection",
            TransportError::Endpoint { .. } => "endpoint",
            TransportError::Certificate { .. } => "certificate",
            TransportError::Tls { .. } => "tls",
            TransportError::Stream { .. } => "stream",
            TransportError::Serialization { .. } => "serialization",
            TransportError::Configuration { .. } => "configuration",
            TransportError::Authentication { .. } => "authentication",
            TransportError::ProtocolVersion { .. } => "protocol",
            TransportError::Timeout { .. } => "timeout",
            TransportError::Network(_) => "network",
            TransportError::Quinn(_) => "quinn",
            TransportError::Rustls(_) => "rustls",
        }
    }
}

impl From<TransportError> for NexusError {
    fn from(err: TransportError) -> Self {
        match err {
            TransportError::Network(io_err) => NexusError::Network(io_err),
            TransportError::Authentication { reason } => NexusError::Authentication { reason },
            TransportError::Timeout { duration_ms } => NexusError::Timeout { duration_ms },
            other => NexusError::Transport {
                message: other.to_string(),
            },
        }
    }
}

/// Convenience macros for creating transport errors
#[macro_export]
macro_rules! connection_error {
    ($msg:expr) => {
        TransportError::Connection {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! stream_error {
    ($msg:expr) => {
        TransportError::Stream {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! cert_error {
    ($msg:expr) => {
        TransportError::Certificate {
            message: $msg.to_string(),
        }
    };
}