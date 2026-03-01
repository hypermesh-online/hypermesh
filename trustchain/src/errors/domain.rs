// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Domain-specific error types for TrustChain subsystems

use serde::{Deserialize, Serialize};
use thiserror::Error;

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

    #[error("Invalid fingerprint: {reason}")]
    InvalidFingerprint { reason: String },

    #[error("Invalid request: {reason}")]
    InvalidRequest { reason: String },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Timestamp error: {reason}")]
    TimestampError { reason: String },
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
