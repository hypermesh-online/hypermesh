//! TrustChain Error Types
//! 
//! Comprehensive error handling for TrustChain services with detailed
//! error context and recovery information, including security error types.

use std::fmt;
use thiserror::Error;
use serde::{Serialize, Deserialize};

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

    /// Security errors (NEW)
    #[error("Security error: {message}")]
    SecurityError { message: String },

    /// Security validation failed (NEW)
    #[error("Security validation failed: {reason}")]
    SecurityValidationFailed { reason: String },

    /// Byzantine fault detected (NEW)
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

/// Certificate Authority specific errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum CAError {
    #[error("Certificate generation failed: {reason}")]
    CertificateGeneration { reason: String },

    #[error("Certificate validation failed: {reason}")]
    CertificateValidation { reason: String },

    #[error("Certificate not found: {identifier}")]
    CertificateNotFound { identifier: String },

    #[error("Certificate revoked: {serial_number} - {reason}")]
    CertificateRevoked {
        serial_number: String,
        reason: String,
    },

    #[error("Certificate expired: {serial_number}")]
    CertificateExpired { serial_number: String },

    #[error("Root CA not available: {ca_id}")]
    RootCANotAvailable { ca_id: String },

    #[error("Policy validation failed: {policy} - {reason}")]
    PolicyValidation { policy: String, reason: String },

    #[error("Insufficient consensus proof for certificate operation")]
    InsufficientConsensusProof,
}

/// Certificate Transparency specific errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum CTError {
    #[error("CT log not found: {log_id}")]
    LogNotFound { log_id: String },

    #[error("Merkle tree error: {operation} - {reason}")]
    MerkleTree { operation: String, reason: String },

    #[error("Certificate fingerprint mismatch: expected {expected}, got {actual}")]
    FingerprintMismatch { expected: String, actual: String },

    #[error("CT log entry not found: {entry_id}")]
    EntryNotFound { entry_id: String },

    #[error("CT log full: {log_id} - {current_entries} entries")]
    LogFull {
        log_id: String,
        current_entries: u64,
    },

    #[error("Merkle proof verification failed: {entry_id}")]
    MerkleProofVerification { entry_id: String },

    #[error("Real-time fingerprinting failed: {certificate_id}")]
    RealtimeFingerprinting { certificate_id: String },

    #[error("SCT generation failed: {reason}")]
    SCTGeneration { reason: String },

    #[error("Log consistency proof failed: {log_id}")]
    LogConsistencyProof { log_id: String },
}

/// DNS resolver specific errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DnsError {
    #[error("DNS query failed: {query} - {reason}")]
    QueryFailed { query: String, reason: String },

    #[error("DNS server binding failed: {address}:{port}")]
    ServerBindFailed { address: String, port: u16 },

    #[error("QUIC connection failed: {reason}")]
    QuicConnectionFailed { reason: String },

    #[error("DNS record not found: {domain}")]
    RecordNotFound { domain: String },

    #[error("DNS cache error: {operation} - {reason}")]
    CacheError { operation: String, reason: String },

    #[error("Certificate DNS validation failed: {domain}")]
    CertificateValidationFailed { domain: String },

    #[error("Upstream resolver error: {resolver} - {reason}")]
    UpstreamResolver { resolver: String, reason: String },

    #[error("IPv6-only networking violated: attempted IPv4 operation")]
    IPv6OnlyViolation,

    #[error("TrustChain domain resolution failed: {domain}")]
    TrustChainDomainResolution { domain: String },

    #[error("Domain not found: {domain}")]
    DomainNotFound { domain: String },
}

/// API server specific errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ApiError {
    #[error("API endpoint not found: {path}")]
    EndpointNotFound { path: String },

    #[error("Authentication failed: {reason}")]
    Authentication { reason: String },

    #[error("Authorization failed: {operation} - {reason}")]
    Authorization { operation: String, reason: String },

    #[error("Rate limit exceeded: {limit} requests per minute")]
    RateLimitExceeded { limit: u32 },

    #[error("Request body too large: {size} bytes (max: {max_size})")]
    RequestBodyTooLarge { size: usize, max_size: usize },

    #[error("Invalid request format: {reason}")]
    InvalidRequestFormat { reason: String },

    #[error("CORS error: {origin} not allowed")]
    CorsError { origin: String },

    #[error("TLS handshake failed: {reason}")]
    TlsHandshake { reason: String },

    #[error("Server startup failed: {reason}")]
    ServerStartup { reason: String },
}

/// Consensus validation specific errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ConsensusError {
    #[error("Proof of Stake validation failed: stake {stake} < minimum {minimum}")]
    ProofOfStakeFailed { stake: u64, minimum: u64 },

    #[error("Proof of Time validation failed: offset {offset:?} > maximum {maximum:?}")]
    ProofOfTimeFailed {
        offset: std::time::Duration,
        maximum: std::time::Duration,
    },

    #[error("Proof of Space validation failed: space {space} < minimum {minimum}")]
    ProofOfSpaceFailed { space: u64, minimum: u64 },

    #[error("Proof of Work validation failed: compute {compute} < minimum {minimum}")]
    ProofOfWorkFailed { compute: u64, minimum: u64 },

    #[error("Byzantine fault detected: {validator_id} - {evidence}")]
    ByzantineFault {
        validator_id: String,
        evidence: String,
    },

    #[error("Consensus proof malformed: {reason}")]
    MalformedProof { reason: String },

    #[error("Consensus timeout: operation {operation} timed out")]
    ConsensusTimeout { operation: String },

    #[error("Insufficient validators: {current} < minimum {minimum}")]
    InsufficientValidators { current: u32, minimum: u32 },
}

/// Configuration specific errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Configuration parse error: {format} - {reason}")]
    ParseError { format: String, reason: String },

    #[error("Configuration validation failed: {field} - {reason}")]
    ValidationFailed { field: String, reason: String },

    #[error("Port conflict detected: {port}")]
    PortConflict { port: u16 },

    #[error("Invalid IPv6 address: {address}")]
    InvalidIPv6Address { address: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid value for {field}: {value} - {reason}")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },
}

/// Network specific errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum NetworkError {
    #[error("Connection failed: {address}:{port} - {reason}")]
    ConnectionFailed {
        address: String,
        port: u16,
        reason: String,
    },

    #[error("Connection timeout: {address}:{port}")]
    ConnectionTimeout { address: String, port: u16 },

    #[error("TLS error: {reason}")]
    TLS { reason: String },

    #[error("QUIC error: {reason}")]
    QUIC { reason: String },

    #[error("IPv6-only constraint violated")]
    IPv6OnlyConstraintViolated,

    #[error("Protocol error: {protocol} - {reason}")]
    Protocol { protocol: String, reason: String },

    #[error("Network interface error: {interface} - {reason}")]
    Interface { interface: String, reason: String },
}

/// Storage specific errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum StorageError {
    #[error("Database error: {operation} - {reason}")]
    Database { operation: String, reason: String },

    #[error("File system error: {path} - {reason}")]
    FileSystem { path: String, reason: String },

    #[error("Data corruption detected: {location}")]
    DataCorruption { location: String },

    #[error("Storage quota exceeded: {used} / {limit} bytes")]
    QuotaExceeded { used: u64, limit: u64 },

    #[error("Backup operation failed: {reason}")]
    BackupFailed { reason: String },

    #[error("Recovery operation failed: {reason}")]
    RecoveryFailed { reason: String },

    #[error("Migration failed: {from_version} -> {to_version} - {reason}")]
    MigrationFailed {
        from_version: String,
        to_version: String,
        reason: String,
    },
}

/// Cryptographic specific errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum CryptoError {
    #[error("Key generation failed: {algorithm} - {reason}")]
    KeyGeneration { algorithm: String, reason: String },

    #[error("Signature verification failed: {reason}")]
    SignatureVerification { reason: String },

    #[error("Encryption failed: {algorithm} - {reason}")]
    Encryption { algorithm: String, reason: String },

    #[error("Decryption failed: {algorithm} - {reason}")]
    Decryption { algorithm: String, reason: String },

    #[error("Hash calculation failed: {algorithm} - {reason}")]
    HashCalculation { algorithm: String, reason: String },

    #[error("Certificate parsing failed: {reason}")]
    CertificateParsing { reason: String },

    #[error("Invalid key format: {format} - {reason}")]
    InvalidKeyFormat { format: String, reason: String },

    #[error("Cryptographic random generation failed")]
    RandomGenerationFailed,
}

/// Result type for TrustChain operations
pub type Result<T> = std::result::Result<T, TrustChainError>;

/// Error response for API endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub request_id: Option<String>,
}

impl ErrorResponse {
    pub fn new(error: &TrustChainError) -> Self {
        Self {
            error: error.to_string(),
            code: Self::error_code(error),
            details: Self::error_details(error),
            timestamp: chrono::Utc::now(),
            request_id: None,
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    fn error_code(error: &TrustChainError) -> String {
        match error {
            TrustChainError::CertificateAuthority(e) => match e {
                CAError::CertificateNotFound { .. } => "CA_CERT_NOT_FOUND".to_string(),
                CAError::CertificateRevoked { .. } => "CA_CERT_REVOKED".to_string(),
                CAError::CertificateExpired { .. } => "CA_CERT_EXPIRED".to_string(),
                _ => "CA_ERROR".to_string(),
            },
            TrustChainError::CertificateTransparency(e) => match e {
                CTError::LogNotFound { .. } => "CT_LOG_NOT_FOUND".to_string(),
                CTError::EntryNotFound { .. } => "CT_ENTRY_NOT_FOUND".to_string(),
                _ => "CT_ERROR".to_string(),
            },
            TrustChainError::DnsResolver(e) => match e {
                DnsError::RecordNotFound { .. } => "DNS_RECORD_NOT_FOUND".to_string(),
                DnsError::IPv6OnlyViolation => "DNS_IPV6_ONLY_VIOLATION".to_string(),
                _ => "DNS_ERROR".to_string(),
            },
            TrustChainError::ApiServer(e) => match e {
                ApiError::Authentication { .. } => "API_AUTH_FAILED".to_string(),
                ApiError::Authorization { .. } => "API_AUTHZ_FAILED".to_string(),
                ApiError::RateLimitExceeded { .. } => "API_RATE_LIMIT".to_string(),
                _ => "API_ERROR".to_string(),
            },
            TrustChainError::ConsensusValidation(e) => match e {
                ConsensusError::ByzantineFault { .. } => "CONSENSUS_BYZANTINE_FAULT".to_string(),
                _ => "CONSENSUS_ERROR".to_string(),
            },
            // NEW: Security error codes
            TrustChainError::SecurityError { .. } => "SECURITY_ERROR".to_string(),
            TrustChainError::SecurityValidationFailed { .. } => "SECURITY_VALIDATION_FAILED".to_string(),
            TrustChainError::ByzantineFaultDetected { .. } => "BYZANTINE_FAULT_DETECTED".to_string(),
            TrustChainError::Configuration(_) => "CONFIG_ERROR".to_string(),
            TrustChainError::Network(_) => "NETWORK_ERROR".to_string(),
            TrustChainError::Storage(_) => "STORAGE_ERROR".to_string(),
            TrustChainError::Cryptographic(_) => "CRYPTO_ERROR".to_string(),
            TrustChainError::Timeout { .. } => "TIMEOUT_ERROR".to_string(),
            TrustChainError::Internal { .. } => "INTERNAL_ERROR".to_string(),
            _ => "UNKNOWN_ERROR".to_string(),
        }
    }

    fn error_details(error: &TrustChainError) -> Option<serde_json::Value> {
        match error {
            TrustChainError::Timeout { operation, duration } => {
                Some(serde_json::json!({
                    "operation": operation,
                    "timeout_duration_secs": duration.as_secs()
                }))
            }
            TrustChainError::ConsensusValidation(ConsensusError::ProofOfStakeFailed {
                stake,
                minimum,
            }) => Some(serde_json::json!({
                "current_stake": stake,
                "minimum_required": minimum
            })),
            TrustChainError::ApiServer(ApiError::RateLimitExceeded { limit }) => {
                Some(serde_json::json!({
                    "rate_limit": limit,
                    "unit": "requests_per_minute"
                }))
            }
            TrustChainError::SecurityValidationFailed { reason } => {
                Some(serde_json::json!({
                    "security_failure_reason": reason
                }))
            }
            TrustChainError::ByzantineFaultDetected { node_id, reason } => {
                Some(serde_json::json!({
                    "byzantine_node_id": node_id,
                    "byzantine_reason": reason
                }))
            }
            _ => None,
        }
    }
}

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

// Note: Display implementations are handled by thiserror derive macro

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
        
        let details = response.details.unwrap();
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
        
        let details = response.details.unwrap();
        assert_eq!(details["security_failure_reason"], "Four-proof consensus validation failed");
    }

    #[test]
    fn test_serialization() {
        let error = CAError::CertificateRevoked {
            serial_number: "123456".to_string(),
            reason: "Private key compromised".to_string(),
        };
        
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: CAError = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            CAError::CertificateRevoked { serial_number, reason } => {
                assert_eq!(serial_number, "123456");
                assert_eq!(reason, "Private key compromised");
            }
            _ => panic!("Unexpected error variant"),
        }
    }
}