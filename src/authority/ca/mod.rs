//! Embedded Certificate Authority for TrustChain
//!
//! Provides complete certificate authority functionality embedded directly
//! in the Internet 2.0 protocol stack, eliminating external CA dependencies.
//!
//! # Module Organization
//!
//! - `types` - Core type definitions (certificates, requests, results)
//! - `operations` - Certificate operations (issuance, validation, revocation)
//! - `authority` - Main CA implementation with post-quantum support

pub mod authority;
pub mod operations;
pub mod types;

// Re-export main types
pub use authority::EmbeddedCertificateAuthority;
pub use operations::{CertificateOperations, DefaultCertificateOperations};
pub use types::{
    CaCertificate, CaStats, CertificateInfo, CertificateRequest,
    CertificateStatus, CertificateValidationResult, ExtendedKeyUsage,
    IssuedCertificate, KeyUsage, RevocationEntry, RevocationReason,
};