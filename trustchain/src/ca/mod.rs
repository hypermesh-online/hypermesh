// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Authority Implementation
//!
//! TrustChain Certificate Authority with Proof of State validation and mandatory security integration.
//! Supports both localhost testing and production deployment with IPv6-only networking.
//!
//! Split into submodules:
//! - `config`: `CAConfig`, `CAMode`, `CAMetrics`
//! - `types`: certificate request / issued / status / metadata types
//! - `trustchain_ca`: legacy `TrustChainCA` struct + `impl`

pub mod certificate_authority;
pub mod certificate_manager;
pub mod certificate_store;
pub mod config;
pub mod crl;
pub mod federation;
pub mod field_bootstrap;
pub mod grace_period;
pub mod ocsp;
pub mod policy;
pub mod security_integration;
pub mod stoq_ca_client;
pub mod trust_provider;
pub mod trustchain_ca;
pub mod types;

pub use certificate_manager::*;
pub use certificate_store::{CertificateStore as CertStore, CertificateStoreMetrics};
pub use config::{CAConfig, CAMetrics, CAMode};
pub use field_bootstrap::{BootstrapState, FieldBootstrap, FieldBootstrapConfig};
pub use grace_period::{GracePeriodConfig, GracePeriodManager, GraceScope, RenewalToken};
pub use policy::*;
// AWS CloudHSM dependencies REMOVED - software-only operation
pub use stoq_ca_client::*;
// Re-export from certificate_authority with qualified imports
pub use certificate_authority::{TrustChainCA as TrustChainCAImpl, *};
// Re-export security integration
pub use security_integration::*;
// Re-export legacy TrustChainCA struct
pub use trustchain_ca::TrustChainCA;
pub use types::{
    CertificateIdentityScope, CertificateMetadata, CertificateRequest, CertificateStatus,
    CertificateSubjectType, IssuedCertificate,
};
// Re-export threshold cryptography types for distributed CA signing
pub use crate::crypto::threshold::{KeyShare, ThresholdConfig, ThresholdSigner};
// Re-export federation types
pub use federation::{
    FederatedCA, FederatedValidationResult, FederationManager, FederationPolicy, FederationStatus,
    FederationTrustLevel,
};
// Re-export the engauge-facing trust provider abstraction (Phase F.1).
pub use trust_provider::{PeerCertFingerprint, PeerTrustBand, TrustSignalProvider};
