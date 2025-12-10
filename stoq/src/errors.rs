//! STOQ Error Types
//!
//! Comprehensive error handling for STOQ transport layer with proper error propagation.

use std::fmt;
use std::io;

/// Main STOQ error type with 5 categories
#[derive(Debug)]
pub enum StoqError {
    /// Transport-level errors (QUIC, connection, streams)
    Transport(TransportError),

    /// Protocol-level errors (PoS validation, API, service discovery)
    Protocol(ProtocolError),

    /// Network-level errors (isolation, tunnels, packet violations)
    Network(NetworkError),

    /// Security-level errors (crypto, certificates, validation)
    Security(SecurityError),

    /// API-level errors (handlers, serialization, requests)
    Api(ApiError),
}

/// Transport layer errors
#[derive(Debug)]
pub enum TransportError {
    /// Connection failed
    ConnectionFailed(String),

    /// Connection closed
    ConnectionClosed(String),

    /// Stream error
    StreamError(String),

    /// Endpoint binding failed
    BindFailed(String),

    /// Configuration error
    ConfigError(String),

    /// I/O error
    Io(io::Error),

    /// QUIC protocol error
    QuicError(String),
}

/// Protocol layer errors
#[derive(Debug)]
pub enum ProtocolError {
    /// PoS validation failed
    ValidationFailed(String),

    /// Token expired
    TokenExpired,

    /// Invalid proof
    InvalidProof(String),

    /// Service not found
    ServiceNotFound(String),

    /// Service discovery failed
    DiscoveryFailed(String),

    /// Cache error
    CacheError(String),
}

/// Network layer errors
#[derive(Debug)]
pub enum NetworkError {
    /// Network stack not found
    NetworkNotFound(String),

    /// Isolation violation
    IsolationViolation(String),

    /// Tunnel error
    TunnelError(String),

    /// Network limit reached
    NetworkLimitReached,

    /// Privacy tier mismatch
    PrivacyTierMismatch(String),
}

/// Security layer errors
#[derive(Debug)]
pub enum SecurityError {
    /// Certificate error
    CertificateError(String),

    /// Signature verification failed
    SignatureVerificationFailed,

    /// Crypto provider not initialized
    CryptoProviderNotInitialized,

    /// TLS error
    TlsError(String),

    /// Key generation failed
    KeyGenerationFailed(String),
}

/// API layer errors
#[derive(Debug, Clone)]
pub enum ApiError {
    /// Handler not found
    NotFound(String),

    /// Invalid request format
    InvalidRequest(String),

    /// Handler execution failed
    HandlerError(String),

    /// Serialization/deserialization failed
    SerializationError(String),

    /// Transport error
    TransportError(String),
}

// Display implementations
impl fmt::Display for StoqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoqError::Transport(e) => write!(f, "Transport error: {}", e),
            StoqError::Protocol(e) => write!(f, "Protocol error: {}", e),
            StoqError::Network(e) => write!(f, "Network error: {}", e),
            StoqError::Security(e) => write!(f, "Security error: {}", e),
            StoqError::Api(e) => write!(f, "API error: {}", e),
        }
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::ConnectionFailed(s) => write!(f, "Connection failed: {}", s),
            TransportError::ConnectionClosed(s) => write!(f, "Connection closed: {}", s),
            TransportError::StreamError(s) => write!(f, "Stream error: {}", s),
            TransportError::BindFailed(s) => write!(f, "Bind failed: {}", s),
            TransportError::ConfigError(s) => write!(f, "Config error: {}", s),
            TransportError::Io(e) => write!(f, "I/O error: {}", e),
            TransportError::QuicError(s) => write!(f, "QUIC error: {}", s),
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::ValidationFailed(s) => write!(f, "Validation failed: {}", s),
            ProtocolError::TokenExpired => write!(f, "Token expired"),
            ProtocolError::InvalidProof(s) => write!(f, "Invalid proof: {}", s),
            ProtocolError::ServiceNotFound(s) => write!(f, "Service not found: {}", s),
            ProtocolError::DiscoveryFailed(s) => write!(f, "Discovery failed: {}", s),
            ProtocolError::CacheError(s) => write!(f, "Cache error: {}", s),
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::NetworkNotFound(s) => write!(f, "Network not found: {}", s),
            NetworkError::IsolationViolation(s) => write!(f, "Isolation violation: {}", s),
            NetworkError::TunnelError(s) => write!(f, "Tunnel error: {}", s),
            NetworkError::NetworkLimitReached => write!(f, "Network limit reached"),
            NetworkError::PrivacyTierMismatch(s) => write!(f, "Privacy tier mismatch: {}", s),
        }
    }
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::CertificateError(s) => write!(f, "Certificate error: {}", s),
            SecurityError::SignatureVerificationFailed => write!(f, "Signature verification failed"),
            SecurityError::CryptoProviderNotInitialized => write!(f, "Crypto provider not initialized"),
            SecurityError::TlsError(s) => write!(f, "TLS error: {}", s),
            SecurityError::KeyGenerationFailed(s) => write!(f, "Key generation failed: {}", s),
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::NotFound(s) => write!(f, "Not found: {}", s),
            ApiError::InvalidRequest(s) => write!(f, "Invalid request: {}", s),
            ApiError::HandlerError(s) => write!(f, "Handler error: {}", s),
            ApiError::SerializationError(s) => write!(f, "Serialization error: {}", s),
            ApiError::TransportError(s) => write!(f, "Transport error: {}", s),
        }
    }
}

// Error trait implementations
impl std::error::Error for StoqError {}
impl std::error::Error for TransportError {}
impl std::error::Error for ProtocolError {}
impl std::error::Error for NetworkError {}
impl std::error::Error for SecurityError {}
impl std::error::Error for ApiError {}

// From<T> conversions for common error types
impl From<io::Error> for StoqError {
    fn from(err: io::Error) -> Self {
        StoqError::Transport(TransportError::Io(err))
    }
}

impl From<quinn::ConnectionError> for StoqError {
    fn from(err: quinn::ConnectionError) -> Self {
        StoqError::Transport(TransportError::QuicError(err.to_string()))
    }
}

impl From<quinn::ConnectError> for StoqError {
    fn from(err: quinn::ConnectError) -> Self {
        StoqError::Transport(TransportError::ConnectionFailed(err.to_string()))
    }
}

impl From<quinn::ReadError> for StoqError {
    fn from(err: quinn::ReadError) -> Self {
        StoqError::Transport(TransportError::StreamError(err.to_string()))
    }
}

impl From<quinn::WriteError> for StoqError {
    fn from(err: quinn::WriteError) -> Self {
        StoqError::Transport(TransportError::StreamError(err.to_string()))
    }
}

impl From<bincode::Error> for StoqError {
    fn from(err: bincode::Error) -> Self {
        StoqError::Api(ApiError::SerializationError(err.to_string()))
    }
}

impl From<serde_json::Error> for StoqError {
    fn from(err: serde_json::Error) -> Self {
        StoqError::Api(ApiError::SerializationError(err.to_string()))
    }
}

impl From<anyhow::Error> for StoqError {
    fn from(err: anyhow::Error) -> Self {
        // Try to downcast to specific error types
        if let Some(io_err) = err.downcast_ref::<io::Error>() {
            return StoqError::Transport(TransportError::Io(
                io::Error::new(io_err.kind(), err.to_string())
            ));
        }

        // Default to transport error for unknown anyhow errors
        StoqError::Transport(TransportError::ConfigError(err.to_string()))
    }
}

impl From<TransportError> for StoqError {
    fn from(err: TransportError) -> Self {
        StoqError::Transport(err)
    }
}

impl From<ProtocolError> for StoqError {
    fn from(err: ProtocolError) -> Self {
        StoqError::Protocol(err)
    }
}

impl From<NetworkError> for StoqError {
    fn from(err: NetworkError) -> Self {
        StoqError::Network(err)
    }
}

impl From<SecurityError> for StoqError {
    fn from(err: SecurityError) -> Self {
        StoqError::Security(err)
    }
}

impl From<ApiError> for StoqError {
    fn from(err: ApiError) -> Self {
        StoqError::Api(err)
    }
}

// Convenience type alias
pub type Result<T> = std::result::Result<T, StoqError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = StoqError::Transport(TransportError::ConnectionFailed("test".to_string()));
        assert!(err.to_string().contains("Connection failed"));

        let err = StoqError::Protocol(ProtocolError::TokenExpired);
        assert!(err.to_string().contains("Token expired"));

        let err = StoqError::Network(NetworkError::NetworkLimitReached);
        assert!(err.to_string().contains("Network limit reached"));
    }

    #[test]
    fn test_error_conversions() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test");
        let stoq_err: StoqError = io_err.into();
        assert!(matches!(stoq_err, StoqError::Transport(TransportError::Io(_))));
    }
}
