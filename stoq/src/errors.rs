// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

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
    /// Connection failed with remote endpoint
    ConnectionFailed {
        remote: String,
        reason: String,
    },

    /// Connection closed unexpectedly
    ConnectionClosed {
        remote: String,
        reason: String,
    },

    /// Stream operation failed
    StreamError {
        stream_id: Option<u64>,
        operation: String,
        reason: String,
    },

    /// Endpoint binding failed
    BindFailed {
        address: String,
        port: u16,
        reason: String,
    },

    /// Configuration error
    ConfigError {
        parameter: String,
        reason: String,
    },

    /// I/O error
    Io(io::Error),

    /// QUIC protocol error
    QuicError {
        error_code: Option<u64>,
        reason: String,
    },

    /// Connection pool exhausted
    PoolExhausted {
        max_connections: usize,
    },

    /// Endpoint not reachable
    EndpointUnreachable {
        remote: String,
    },
}

/// Protocol layer errors
#[derive(Debug)]
pub enum ProtocolError {
    /// PoS validation failed with detailed proof errors
    ValidationFailed {
        token_id: Vec<u8>,
        errors: Vec<String>,
    },

    /// Token expired
    TokenExpired {
        token_id: Vec<u8>,
        expired_at: u64,
        current_time: u64,
    },

    /// Invalid proof component
    InvalidProof {
        proof_type: ProofType,
        reason: String,
    },

    /// Service not found
    ServiceNotFound {
        service_name: String,
    },

    /// Service discovery failed
    DiscoveryFailed {
        service_name: String,
        reason: String,
    },

    /// Cache error
    CacheError {
        operation: String,
        reason: String,
    },

    /// Frame decoding failed
    FrameDecodeFailed {
        frame_type: Option<u64>,
        reason: String,
    },

    /// Frame encoding failed
    FrameEncodeFailed {
        frame_type: String,
        reason: String,
    },

    /// Shard reassembly failed
    ShardReassemblyFailed {
        shard_id: u32,
        reason: String,
    },

    /// Token replay attack detected
    TokenReplayDetected {
        token_hash: [u8; 32],
    },
}

// Re-export ProofType from canonical shared lib (single source of truth)
pub use hypermesh_lib::ProofType;

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
            TransportError::ConnectionFailed { remote, reason } =>
                write!(f, "Connection to {} failed: {}", remote, reason),
            TransportError::ConnectionClosed { remote, reason } =>
                write!(f, "Connection to {} closed: {}", remote, reason),
            TransportError::StreamError { stream_id, operation, reason } => {
                if let Some(id) = stream_id {
                    write!(f, "Stream {} {} failed: {}", id, operation, reason)
                } else {
                    write!(f, "Stream {} failed: {}", operation, reason)
                }
            }
            TransportError::BindFailed { address, port, reason } =>
                write!(f, "Failed to bind to [{}]:{}: {}", address, port, reason),
            TransportError::ConfigError { parameter, reason } =>
                write!(f, "Config error for '{}': {}", parameter, reason),
            TransportError::Io(e) =>
                write!(f, "I/O error: {}", e),
            TransportError::QuicError { error_code, reason } => {
                if let Some(code) = error_code {
                    write!(f, "QUIC error {}: {}", code, reason)
                } else {
                    write!(f, "QUIC error: {}", reason)
                }
            }
            TransportError::PoolExhausted { max_connections } =>
                write!(f, "Connection pool exhausted (max: {})", max_connections),
            TransportError::EndpointUnreachable { remote } =>
                write!(f, "Endpoint {} is unreachable", remote),
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::ValidationFailed { token_id, errors } =>
                write!(f, "Token {:?} validation failed: {}", token_id, errors.join(", ")),
            ProtocolError::TokenExpired { token_id, expired_at, current_time } =>
                write!(f, "Token {:?} expired at {} (current: {})", token_id, expired_at, current_time),
            ProtocolError::InvalidProof { proof_type, reason } =>
                write!(f, "Invalid {} proof: {}", proof_type, reason),
            ProtocolError::ServiceNotFound { service_name } =>
                write!(f, "Service '{}' not found", service_name),
            ProtocolError::DiscoveryFailed { service_name, reason } =>
                write!(f, "Discovery of '{}' failed: {}", service_name, reason),
            ProtocolError::CacheError { operation, reason } =>
                write!(f, "Cache {} error: {}", operation, reason),
            ProtocolError::FrameDecodeFailed { frame_type, reason } => {
                if let Some(ft) = frame_type {
                    write!(f, "Frame type 0x{:x} decode failed: {}", ft, reason)
                } else {
                    write!(f, "Frame decode failed: {}", reason)
                }
            }
            ProtocolError::FrameEncodeFailed { frame_type, reason } =>
                write!(f, "Frame {} encode failed: {}", frame_type, reason),
            ProtocolError::ShardReassemblyFailed { shard_id, reason } =>
                write!(f, "Shard {} reassembly failed: {}", shard_id, reason),
            ProtocolError::TokenReplayDetected { token_hash } =>
                write!(f, "Token replay attack detected: {:?}", token_hash),
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
        let error_code = match &err {
            quinn::ConnectionError::VersionMismatch => Some(0x01),
            quinn::ConnectionError::TransportError(_) => Some(0x02),
            quinn::ConnectionError::ConnectionClosed(_) => Some(0x03),
            quinn::ConnectionError::ApplicationClosed(_) => Some(0x04),
            quinn::ConnectionError::Reset => Some(0x05),
            quinn::ConnectionError::TimedOut => Some(0x06),
            quinn::ConnectionError::LocallyClosed => Some(0x07),
            _ => None,
        };

        StoqError::Transport(TransportError::QuicError {
            error_code,
            reason: err.to_string(),
        })
    }
}

impl From<quinn::ConnectError> for StoqError {
    fn from(err: quinn::ConnectError) -> Self {
        let remote = match &err {
            quinn::ConnectError::EndpointStopping => "endpoint-stopping".to_string(),
            quinn::ConnectError::InvalidRemoteAddress(_) => "invalid-address".to_string(),
            quinn::ConnectError::InvalidServerName(_) => "invalid-server-name".to_string(),
            quinn::ConnectError::NoDefaultClientConfig => "no-default-config".to_string(),
            quinn::ConnectError::UnsupportedVersion => "unsupported-version".to_string(),
            _ => "unknown".to_string(),
        };

        StoqError::Transport(TransportError::ConnectionFailed {
            remote,
            reason: err.to_string(),
        })
    }
}

impl From<quinn::ReadError> for StoqError {
    fn from(err: quinn::ReadError) -> Self {
        StoqError::Transport(TransportError::StreamError {
            stream_id: None,
            operation: "read".to_string(),
            reason: err.to_string(),
        })
    }
}

impl From<quinn::WriteError> for StoqError {
    fn from(err: quinn::WriteError) -> Self {
        StoqError::Transport(TransportError::StreamError {
            stream_id: None,
            operation: "write".to_string(),
            reason: err.to_string(),
        })
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
        StoqError::Transport(TransportError::ConfigError {
            parameter: "unknown".to_string(),
            reason: err.to_string(),
        })
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
        let err = StoqError::Transport(TransportError::ConnectionFailed {
            remote: "[::1]:9292".to_string(),
            reason: "timeout".to_string(),
        });
        assert!(err.to_string().contains("Connection to [::1]:9292 failed"));

        let err = StoqError::Protocol(ProtocolError::TokenExpired {
            token_id: vec![1, 2, 3],
            expired_at: 100,
            current_time: 200,
        });
        assert!(err.to_string().contains("expired"));

        let err = StoqError::Network(NetworkError::NetworkLimitReached);
        assert!(err.to_string().contains("Network limit reached"));
    }

    #[test]
    fn test_error_conversions() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test");
        let stoq_err: StoqError = io_err.into();
        assert!(matches!(stoq_err, StoqError::Transport(TransportError::Io(_))));
    }

    #[test]
    fn test_transport_error_context() {
        let err = TransportError::StreamError {
            stream_id: Some(42),
            operation: "read".to_string(),
            reason: "connection reset".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Stream 42"));
        assert!(msg.contains("read"));
        assert!(msg.contains("connection reset"));
    }

    #[test]
    fn test_protocol_error_proof_type() {
        let err = ProtocolError::InvalidProof {
            proof_type: ProofType::Stake,
            reason: "insufficient stake".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("ProofOfStake"));
        assert!(msg.contains("insufficient stake"));
    }
}
