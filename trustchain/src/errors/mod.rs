// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain Error Types
//!
//! Comprehensive error handling for TrustChain services with detailed
//! error context and recovery information, including security error types.

pub mod domain;
pub mod response;

// Re-export all domain errors for backward compatibility
pub use domain::*;
pub use response::*;

use thiserror::Error;

/// Main TrustChain error type
#[derive(Debug, Error)]
pub enum TrustChainError {
    /// Certificate Authority errors
    #[error("Certificate Authority error: {0}")]
    CertificateAuthority(#[from] CAError),

    /// Certificate Transparency errors
    #[error("Certificate Transparency error: {0}")]
    CertificateTransparency(#[from] CTError),

    /// DNS resolver errors
    #[error("DNS resolver error: {0}")]
    DnsResolver(#[from] DnsError),

    /// API server errors
    #[error("API server error: {0}")]
    ApiServer(#[from] ApiError),

    /// Consensus validation errors
    #[error("Consensus validation error: {0}")]
    ConsensusValidation(#[from] ConsensusError),

    /// Security errors
    #[error("Security error: {message}")]
    SecurityError { message: String },

    /// Security validation failed
    #[error("Security validation failed: {reason}")]
    SecurityValidationFailed { reason: String },

    /// Byzantine fault detected
    #[error("Byzantine fault detected: {node_id} - {reason}")]
    ByzantineFaultDetected { node_id: String, reason: String },

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigError),

    /// Network errors
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// Storage errors
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Cryptographic errors
    #[error("Cryptographic error: {0}")]
    Cryptographic(#[from] CryptoError),

    /// General I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Timeout errors
    #[error("Operation timed out: {operation} after {duration:?}")]
    Timeout {
        operation: String,
        duration: std::time::Duration,
    },

    /// Merkle tree initialization failed
    #[error("Merkle tree initialization failed: {reason}")]
    MerkleTreeInitFailed { reason: String },

    /// Merkle tree insert failed
    #[error("Merkle tree insert failed for entry {entry_id}: {reason}")]
    MerkleTreeInsertFailed { entry_id: String, reason: String },

    /// Internal errors
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Consensus validation failed
    #[error("Consensus proof validation failed: {reason}")]
    ConsensusValidationFailed { reason: String },

    /// Certificate transparency disabled
    #[error("Certificate transparency is disabled")]
    CertificateTransparencyDisabled,

    /// Certificate not found in CT logs
    #[error("Certificate not found in transparency logs: {fingerprint}")]
    CertificateNotFoundInCT { fingerprint: String },

    /// Certificate parsing failed
    #[error("Certificate parsing failed: {reason}")]
    CertificateParsingFailed { reason: String },

    /// Merkle tree update failed
    #[error("Merkle tree update failed: {reason}")]
    MerkleTreeUpdateFailed { reason: String },

    /// Merkle proof generation failed
    #[error("Merkle proof generation failed: {reason}")]
    MerkleProofGenerationFailed { reason: String },

    /// QUIC connection failed
    #[error("QUIC connection failed: {reason}")]
    QuicConnectionFailed { reason: String },

    /// DNS serialization failed
    #[error("DNS serialization failed: {reason}")]
    DnsSerializationFailed { reason: String },

    /// No upstream DNS servers configured
    #[error("No upstream DNS servers configured")]
    NoUpstreamServers,

    /// Domain validation failed
    #[error("Domain validation failed for {domain}: {reason}")]
    DomainValidationFailed { domain: String, reason: String },

    /// Service discovery error
    #[error("Service discovery failed for {service}: {reason}")]
    ServiceDiscoveryError { service: String, reason: String },

    /// Certificate validation failed
    #[error("Certificate validation failed: {reason}")]
    CertificateValidationFailed { reason: String },

    /// Network error with operation context
    #[error("Network operation failed: {operation} - {reason}")]
    NetworkError { operation: String, reason: String },

    /// Serialization error with operation context
    #[error("Serialization failed for {operation}: {reason}")]
    SerializationError { operation: String, reason: String },

    /// DNS error with operation context
    #[error("DNS operation failed: {operation} - {reason}")]
    DNSError { operation: String, reason: String },

    /// Certificate generation failed
    #[error("Certificate generation failed: {reason}")]
    CertificateGenerationFailed { reason: String },

    /// Invalid fingerprint
    #[error("Invalid certificate fingerprint")]
    InvalidFingerprint,

    /// Resource not found
    #[error("Resource not found")]
    NotFound,

    /// Invalid request
    #[error("Invalid request: {reason}")]
    InvalidRequest { reason: String },

    /// Key not found (software-only key management)
    #[error("Key not found: {key_id}")]
    KeyNotFound { key_id: String },

    /// Key operation error
    #[error("Key operation error: {reason}")]
    KeyOperationError { reason: String },

    /// Duplicate certificate
    #[error("Duplicate certificate: {fingerprint}")]
    DuplicateCertificate { fingerprint: String },

    /// Timestamp error
    #[error("Timestamp error: {reason}")]
    TimestampError { reason: String },

    /// Merkle tree error
    #[error("Merkle tree error: {reason}")]
    MerkleTreeError { reason: String },

    /// Serialization failed
    #[error("Serialization failed: {reason}")]
    SerializationFailed { reason: String },

    /// Key configuration error
    #[error("Key configuration error: {reason}")]
    KeyConfigError { reason: String },

    /// Crypto error
    #[error("Cryptographic error: {reason}")]
    CryptoError { reason: String },

    /// Key operation failed
    #[error("Key operation failed: {operation} - {reason}")]
    KeyOperationFailed { operation: String, reason: String },

    /// Software key security violation
    #[error("Software key security violation: {reason}")]
    SoftwareKeySecurityViolation { reason: String },

    /// Security policy violation
    #[error("Security policy violation: {reason}")]
    SecurityPolicyViolation { reason: String },

    /// Security violation
    #[error("Security violation: {reason}")]
    SecurityViolation { reason: String },

    /// Storage configuration error
    #[error("Storage configuration error: {reason}")]
    StorageConfigError { reason: String },

    /// Storage connection error
    #[error("Storage connection error: {reason}")]
    StorageConnectionError { reason: String },

    /// Storage operation failed
    #[error("Storage operation failed: {operation} - {reason}")]
    StorageOperationFailed { operation: String, reason: String },
}

/// Result type for TrustChain operations
pub type Result<T> = std::result::Result<T, TrustChainError>;

/// Convert anyhow::Error to TrustChainError
impl From<anyhow::Error> for TrustChainError {
    fn from(error: anyhow::Error) -> Self {
        TrustChainError::Internal {
            message: error.to_string(),
        }
    }
}

/// Convert serde_json::Error to TrustChainError
impl From<serde_json::Error> for TrustChainError {
    fn from(error: serde_json::Error) -> Self {
        TrustChainError::Serialization(error.to_string())
    }
}

/// Convert bincode::Error to TrustChainError
impl From<bincode::Error> for TrustChainError {
    fn from(error: bincode::Error) -> Self {
        TrustChainError::Serialization(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_creation() {
        let error = TrustChainError::Configuration(ConfigError::FileNotFound {
            path: "/etc/trustchain.toml".to_string(),
        });

        let response = ErrorResponse::new(&error);
        assert_eq!(response.code, "CONFIG_ERROR");
        assert!(response.error.contains("Configuration file not found"));
    }

    #[test]
    fn test_security_error_codes() {
        let security_error = TrustChainError::SecurityValidationFailed {
            reason: "Consensus validation failed".to_string(),
        };

        let response = ErrorResponse::new(&security_error);
        assert_eq!(response.code, "SECURITY_VALIDATION_FAILED");

        let byzantine_error = TrustChainError::ByzantineFaultDetected {
            node_id: "malicious_node_001".to_string(),
            reason: "Invalid consensus proof".to_string(),
        };

        let response = ErrorResponse::new(&byzantine_error);
        assert_eq!(response.code, "BYZANTINE_FAULT_DETECTED");
    }

    #[test]
    fn test_error_code_mapping() {
        let ca_error = TrustChainError::CertificateAuthority(CAError::CertificateNotFound {
            identifier: "test-cert".to_string(),
        });

        let response = ErrorResponse::new(&ca_error);
        assert_eq!(response.code, "CA_CERT_NOT_FOUND");
    }

    #[test]
    fn test_error_details_extraction() {
        let timeout_error = TrustChainError::Timeout {
            operation: "certificate_validation".to_string(),
            duration: std::time::Duration::from_secs(30),
        };

        let response = ErrorResponse::new(&timeout_error);
        assert!(response.details.is_some());

        let details = response.details.expect("test");
        assert_eq!(details["operation"], "certificate_validation");
        assert_eq!(details["timeout_duration_secs"], 30);
    }

    #[test]
    fn test_security_error_details() {
        let security_error = TrustChainError::SecurityValidationFailed {
            reason: "Four-proof consensus validation failed".to_string(),
        };

        let response = ErrorResponse::new(&security_error);
        assert!(response.details.is_some());

        let details = response.details.expect("test");
        assert_eq!(
            details["security_failure_reason"],
            "Four-proof consensus validation failed"
        );
    }

    #[test]
    fn test_serialization() {
        let error = CAError::CertificateRevoked {
            serial_number: "123456".to_string(),
            reason: "Private key compromised".to_string(),
        };

        let json = serde_json::to_string(&error).expect("test");
        let deserialized: CAError = serde_json::from_str(&json).expect("test");

        match deserialized {
            CAError::CertificateRevoked {
                serial_number,
                reason,
            } => {
                assert_eq!(serial_number, "123456");
                assert_eq!(reason, "Private key compromised");
            }
            _ => unreachable!("Unexpected error variant"),
        }
    }
}
