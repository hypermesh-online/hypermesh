// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate request/response types and status metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

use crate::proof_of_state::StateProof;

/// Certificate issuance request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertificateRequest {
    /// Common name for certificate
    pub common_name: String,
    /// Subject alternative names
    pub san_entries: Vec<String>,
    /// Requesting node ID
    pub node_id: String,
    /// IPv6 addresses for certificate
    pub ipv6_addresses: Vec<std::net::Ipv6Addr>,
    /// State proof for validation
    pub state_proof: StateProof,
    /// Request timestamp
    pub timestamp: SystemTime,
    /// Identity scope for scope-aware certificates (Item 2.6/2.7)
    /// When None, defaults to Device scope, untracked (anonymous)
    #[serde(default)]
    pub identity_scope: Option<CertificateIdentityScope>,
    /// Certificate subject type for KeyUsage/EKU selection (Item 2.4)
    /// When None, defaults to Node
    #[serde(default)]
    pub subject_type: Option<CertificateSubjectType>,
}

/// Identity scope embedded into certificates (Items 2.6, 2.7)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertificateIdentityScope {
    /// Blockchain scope: Device (local) or Network (synced)
    pub blockchain_scope: hypermesh_lib::BlockchainScope,
    /// Whether the identity is tracked
    pub tracked: bool,
}

/// Certificate subject type for KeyUsage/EKU decisions (Item 2.4)
#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CertificateSubjectType {
    /// Node identity: gets digitalSignature + keyEncipherment + serverAuth + clientAuth
    Node,
    /// Service identity: gets digitalSignature + serverAuth
    Service,
    /// Agent identity: gets digitalSignature + clientAuth
    Agent,
}

impl From<CertificateSubjectType> for hypermesh_lib::WorkloadType {
    fn from(cst: CertificateSubjectType) -> Self {
        match cst {
            CertificateSubjectType::Node => Self::Node,
            CertificateSubjectType::Service => Self::Service,
            CertificateSubjectType::Agent => Self::Agent,
        }
    }
}

/// Issued certificate information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IssuedCertificate {
    /// Certificate serial number
    pub serial_number: String,
    /// DER-encoded certificate
    pub certificate_der: Vec<u8>,
    /// PEM-encoded certificate (for API compatibility)
    pub certificate_pem: String,
    /// PEM-encoded certificate chain (for API compatibility)
    pub chain_pem: String,
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: [u8; 32],
    /// Common name
    pub common_name: String,
    /// Issue timestamp
    pub issued_at: SystemTime,
    /// Expiration timestamp
    pub expires_at: SystemTime,
    /// Issuing CA ID
    pub issuer_ca_id: String,
    /// Associated state proof
    pub state_proof: StateProof,
    /// Certificate status
    pub status: CertificateStatus,
    /// Additional metadata
    pub metadata: CertificateMetadata,
}

/// Additional certificate metadata
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CertificateMetadata {
    /// Key algorithm used
    pub key_algorithm: Option<String>,
    /// Signature algorithm used
    pub signature_algorithm: Option<String>,
    /// Extensions included
    pub extensions: Vec<String>,
    /// Additional tags
    pub tags: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CertificateStatus {
    Valid,
    Revoked {
        reason: String,
        revoked_at: SystemTime,
    },
    Expired,
}
