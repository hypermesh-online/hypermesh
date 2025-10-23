//! Security framework error types

use thiserror::Error;
use std::io;

/// Security framework result type
pub type Result<T> = std::result::Result<T, SecurityError>;

/// Security framework errors
#[derive(Debug, Error)]
pub enum SecurityError {
    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    /// eBPF operation failed
    #[error("eBPF error: {message}")]
    EBPFError { message: String },
    
    /// Certificate operation failed
    #[error("Certificate error: {message}")]
    CertificateError { message: String },
    
    /// Capability operation failed
    #[error("Capability error: {message}")]
    CapabilityError { message: String },
    
    /// Policy violation
    #[error("Policy violation: {message}")]
    PolicyViolation { message: String },
    
    /// Authentication failed
    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },
    
    /// Authorization failed
    #[error("Authorization failed: {message}")]
    AuthorizationFailed { message: String },
    
    /// Threat detection
    #[error("Threat detected: {message}")]
    ThreatDetected { message: String },
    
    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },
    
    /// HSM error
    #[error("HSM error: {message}")]
    HSMError { message: String },
    
    /// Cryptographic operation failed
    #[error("Crypto error: {message}")]
    CryptoError { message: String },
    
    /// Network security error
    #[error("Network security error: {message}")]
    NetworkSecurityError { message: String },
    
    /// System integration error
    #[error("System integration error: {message}")]
    SystemIntegrationError { message: String },
    
    /// Monitoring error
    #[error("Monitoring error: {message}")]
    MonitoringError { message: String },
    
    /// Resource not found
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },
    
    /// Operation timeout
    #[error("Operation timed out after {duration:?}")]
    Timeout { duration: std::time::Duration },
    
    /// Invalid input
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },
    
    /// Internal error
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl SecurityError {
    /// Create a new eBPF error
    pub fn ebpf(message: impl Into<String>) -> Self {
        Self::EBPFError { message: message.into() }
    }
    
    /// Create a new certificate error
    pub fn certificate(message: impl Into<String>) -> Self {
        Self::CertificateError { message: message.into() }
    }
    
    /// Create a new capability error
    pub fn capability(message: impl Into<String>) -> Self {
        Self::CapabilityError { message: message.into() }
    }
    
    /// Create a new policy violation
    pub fn policy_violation(message: impl Into<String>) -> Self {
        Self::PolicyViolation { message: message.into() }
    }
    
    /// Create a new authentication failure
    pub fn authentication_failed(message: impl Into<String>) -> Self {
        Self::AuthenticationFailed { message: message.into() }
    }
    
    /// Create a new authorization failure
    pub fn authorization_failed(message: impl Into<String>) -> Self {
        Self::AuthorizationFailed { message: message.into() }
    }
    
    /// Create a new threat detection
    pub fn threat_detected(message: impl Into<String>) -> Self {
        Self::ThreatDetected { message: message.into() }
    }
    
    /// Create a new configuration error
    pub fn configuration_error(message: impl Into<String>) -> Self {
        Self::ConfigurationError { message: message.into() }
    }
    
    /// Create a new HSM error
    pub fn hsm_error(message: impl Into<String>) -> Self {
        Self::HSMError { message: message.into() }
    }
    
    /// Create a new crypto error
    pub fn crypto_error(message: impl Into<String>) -> Self {
        Self::CryptoError { message: message.into() }
    }
    
    /// Create a new network security error
    pub fn network_security_error(message: impl Into<String>) -> Self {
        Self::NetworkSecurityError { message: message.into() }
    }
    
    /// Create a new system integration error
    pub fn system_integration_error(message: impl Into<String>) -> Self {
        Self::SystemIntegrationError { message: message.into() }
    }
    
    /// Create a new monitoring error
    pub fn monitoring_error(message: impl Into<String>) -> Self {
        Self::MonitoringError { message: message.into() }
    }
    
    /// Create a new invalid input error
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput { message: message.into() }
    }
    
    /// Create a new internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal { message: message.into() }
    }
}