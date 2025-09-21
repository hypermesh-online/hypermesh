//! Certificate Authority Types
//!
//! Core type definitions for the embedded certificate authority.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Certificate request for issuance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateRequest {
    /// Subject distinguished name
    pub subject_dn: String,

    /// Common name (CN)
    pub common_name: String,

    /// Organization (O)
    pub organization: Option<String>,

    /// Organizational unit (OU)
    pub organizational_unit: Option<String>,

    /// Country (C)
    pub country: Option<String>,

    /// State or province (ST)
    pub state: Option<String>,

    /// Locality (L)
    pub locality: Option<String>,

    /// Email address
    pub email: Option<String>,

    /// DNS names (Subject Alternative Names)
    pub dns_names: Vec<String>,

    /// IP addresses (Subject Alternative Names)
    pub ip_addresses: Vec<String>,

    /// Requested validity period (days)
    pub validity_days: u32,

    /// Key usage flags
    pub key_usage: KeyUsage,

    /// Extended key usage
    pub extended_key_usage: ExtendedKeyUsage,

    /// Post-quantum algorithm preference
    pub pq_algorithm: Option<String>,
}

/// Key usage flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyUsage {
    pub digital_signature: bool,
    pub content_commitment: bool,
    pub key_encipherment: bool,
    pub data_encipherment: bool,
    pub key_agreement: bool,
    pub key_cert_sign: bool,
    pub crl_sign: bool,
    pub encipher_only: bool,
    pub decipher_only: bool,
}

/// Extended key usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedKeyUsage {
    pub server_auth: bool,
    pub client_auth: bool,
    pub code_signing: bool,
    pub email_protection: bool,
    pub time_stamping: bool,
    pub ocsp_signing: bool,
}

/// CA certificate structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaCertificate {
    /// Certificate serial number
    pub serial_number: String,

    /// Subject distinguished name
    pub subject_dn: String,

    /// Issuer distinguished name
    pub issuer_dn: String,

    /// Not valid before
    pub not_before: SystemTime,

    /// Not valid after
    pub not_after: SystemTime,

    /// Public key (DER encoded)
    pub public_key: Vec<u8>,

    /// Certificate signature
    pub signature: Vec<u8>,

    /// Post-quantum signature (optional)
    pub pq_signature: Option<Vec<u8>>,

    /// Certificate extensions
    pub extensions: HashMap<String, Vec<u8>>,

    /// Full certificate (DER encoded)
    pub certificate_der: Vec<u8>,
}

/// Issued certificate record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuedCertificate {
    /// Certificate
    pub certificate: CaCertificate,

    /// Issue timestamp
    pub issued_at: SystemTime,

    /// Requestor identity
    pub requestor: String,

    /// Certificate status
    pub status: CertificateStatus,

    /// Associated domain names
    pub domains: Vec<String>,

    /// Associated IP addresses
    pub ip_addresses: Vec<String>,

    /// Certificate fingerprint (SHA256)
    pub fingerprint: String,

    /// Post-quantum algorithm used (if any)
    pub pq_algorithm: Option<String>,

    /// Renewal count
    pub renewal_count: u32,

    /// Last validation timestamp
    pub last_validated: Option<SystemTime>,
}

/// Certificate status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CertificateStatus {
    Active,
    Revoked,
    Expired,
    Suspended,
}

/// Revocation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationEntry {
    /// Certificate serial number
    pub serial_number: String,

    /// Revocation timestamp
    pub revoked_at: SystemTime,

    /// Revocation reason
    pub reason: RevocationReason,
}

/// Revocation reason codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevocationReason {
    Unspecified,
    KeyCompromise,
    CaCompromise,
    AffiliationChanged,
    Superseded,
    CessationOfOperation,
    CertificateHold,
    RemoveFromCrl,
    PrivilegeWithdrawn,
    AaCompromise,
}

/// Certificate validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidationResult {
    /// Is certificate valid
    pub is_valid: bool,

    /// Certificate chain validated
    pub chain_valid: bool,

    /// Signature verified
    pub signature_valid: bool,

    /// Not expired
    pub not_expired: bool,

    /// Not revoked
    pub not_revoked: bool,

    /// Domain match (if applicable)
    pub domain_match: Option<bool>,

    /// Validation errors
    pub errors: Vec<String>,

    /// Certificate details
    pub certificate_info: Option<CertificateInfo>,
}

/// Certificate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    pub serial_number: String,
    pub subject_dn: String,
    pub issuer_dn: String,
    pub not_before: SystemTime,
    pub not_after: SystemTime,
    pub fingerprint: String,
}

/// CA statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CaStats {
    pub certificates_issued: u64,
    pub certificates_revoked: u64,
    pub certificates_renewed: u64,
    pub validation_requests: u64,
    pub validation_failures: u64,
}

impl Default for CertificateRequest {
    fn default() -> Self {
        Self {
            subject_dn: String::new(),
            common_name: String::new(),
            organization: None,
            organizational_unit: None,
            country: None,
            state: None,
            locality: None,
            email: None,
            dns_names: Vec::new(),
            ip_addresses: Vec::new(),
            validity_days: 365,
            key_usage: KeyUsage::default(),
            extended_key_usage: ExtendedKeyUsage::default(),
            pq_algorithm: None,
        }
    }
}

impl Default for KeyUsage {
    fn default() -> Self {
        Self {
            digital_signature: true,
            content_commitment: false,
            key_encipherment: true,
            data_encipherment: false,
            key_agreement: false,
            key_cert_sign: false,
            crl_sign: false,
            encipher_only: false,
            decipher_only: false,
        }
    }
}

impl Default for ExtendedKeyUsage {
    fn default() -> Self {
        Self {
            server_auth: true,
            client_auth: false,
            code_signing: false,
            email_protection: false,
            time_stamping: false,
            ocsp_signing: false,
        }
    }
}

impl RevocationReason {
    /// Get human-readable description
    pub fn description(&self) -> &str {
        match self {
            Self::Unspecified => "Unspecified reason",
            Self::KeyCompromise => "Private key compromised",
            Self::CaCompromise => "CA key compromised",
            Self::AffiliationChanged => "Subject affiliation changed",
            Self::Superseded => "Certificate superseded",
            Self::CessationOfOperation => "Cessation of operation",
            Self::CertificateHold => "Certificate on hold",
            Self::RemoveFromCrl => "Remove from CRL",
            Self::PrivilegeWithdrawn => "Privilege withdrawn",
            Self::AaCompromise => "AA compromise",
        }
    }

    /// Convert to RFC 5280 reason code
    pub fn to_code(&self) -> u8 {
        match self {
            Self::Unspecified => 0,
            Self::KeyCompromise => 1,
            Self::CaCompromise => 2,
            Self::AffiliationChanged => 3,
            Self::Superseded => 4,
            Self::CessationOfOperation => 5,
            Self::CertificateHold => 6,
            Self::RemoveFromCrl => 8,
            Self::PrivilegeWithdrawn => 9,
            Self::AaCompromise => 10,
        }
    }
}