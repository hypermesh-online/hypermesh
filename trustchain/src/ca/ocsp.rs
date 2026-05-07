// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! OCSP Responder for TrustChain Certificate Authority
//!
//! Provides Online Certificate Status Protocol responses for real-time
//! certificate revocation checking. Responses are signed with FALCON-1024
//! for quantum-resistant authenticity.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::certificate_store::CertificateStore;
use super::federation::{FederationManager, FederationTrustLevel};
use super::stoq_ca_client::RevocationReason;
use super::CertificateStatus;
use crate::crypto::falcon::FalconCrypto;
use crate::crypto::{FalconPrivateKey, FalconSignature};
use crate::errors::Result as TrustChainResult;
use crate::errors::TrustChainError;

/// OCSP certificate status as returned in a response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OcspCertStatus {
    /// Certificate is valid and not revoked.
    Good,
    /// Certificate has been revoked.
    Revoked {
        revocation_time: SystemTime,
        reason: RevocationReason,
    },
    /// Certificate status is unknown to this responder.
    Unknown,
}

/// Simplified OCSP request (not full ASN.1, suitable for STOQ transport).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OcspRequest {
    /// Serial number of the certificate to check.
    pub serial_number: String,
    /// SHA-256 hash of the issuer's distinguished name.
    pub issuer_name_hash: [u8; 32],
    /// SHA-256 hash of the issuer's public key.
    pub issuer_key_hash: [u8; 32],
}

/// Signed OCSP response containing certificate status.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OcspResponse {
    /// Revocation status of the queried certificate.
    pub status: OcspCertStatus,
    /// Time at which this status was determined.
    pub this_update: SystemTime,
    /// Time at which the next update will be available.
    pub next_update: SystemTime,
    /// Serial number of the queried certificate.
    pub serial_number: String,
    /// Identifier of the OCSP responder that produced this response.
    pub responder_id: String,
    /// FALCON-1024 signature over the response payload.
    pub signature: FalconSignature,
    /// Time at which this response was produced.
    pub produced_at: SystemTime,
}

/// Pluggable transport used by [`OcspResponder::federated_check`] and
/// [`CrlDistributor::propagate_to_federation`] to talk to federated peers
/// over wire (Phase F.2).
///
/// Production wraps STOQ; tests use the in-memory mock below.  Returning
/// `None` from `query_peer_revocation` means the peer answered "Unknown"
/// (or did not respond before deadline).
#[async_trait]
pub trait FederationOcspTransport: Send + Sync {
    /// Ask `peer_ca_id` whether `serial_number` is revoked.  Returns
    /// `Some(status)` when the peer answers, `None` on timeout / no
    /// answer / transport error.
    async fn query_peer_revocation(
        &self,
        peer_ca_id: &str,
        serial_number: &str,
    ) -> Option<OcspCertStatus>;

    /// Push a CRL revocation entry to `peer_ca_id`.  Returns `true` on
    /// successful delivery, `false` otherwise.  In alpha this is best-
    /// effort and failures are non-fatal.
    async fn push_revocation(
        &self,
        peer_ca_id: &str,
        serial_number: &str,
        reason: &str,
    ) -> bool;
}

/// OCSP Responder backed by the TrustChain certificate store.
///
/// Checks certificate status against the in-memory store and returns
/// FALCON-1024 signed responses for quantum-resistant verification.
///
/// Transport-agnostic: requests and responses are serde-serializable
/// structs suitable for STOQ stream delivery. No HTTP-specific
/// transport assumptions exist in this layer.
pub struct OcspResponder {
    /// Certificate store to query status from.
    store: Arc<CertificateStore>,
    /// FALCON-1024 signing key for response authentication.
    signing_key: FalconPrivateKey,
    /// FALCON crypto engine.
    falcon: FalconCrypto,
    /// Responder identifier (typically the CA name or URI).
    responder_id: String,
    /// How long a response is considered valid.
    validity_period: Duration,
    /// Optional federation transport (Phase F.2). When set,
    /// [`Self::federated_check`] consults peers after the local store
    /// returns Unknown.  When `None`, federated check stays local-only
    /// (alpha default — preserves the existing single-node behaviour).
    federation_transport: RwLock<Option<Arc<dyn FederationOcspTransport>>>,
}

impl OcspResponder {
    /// Create a new OCSP responder.
    ///
    /// * `store` - Certificate store to query.
    /// * `signing_key` - FALCON-1024 private key for signing responses.
    /// * `responder_id` - Identifier for this responder.
    /// * `validity_period` - Duration a response remains valid (default: 1 hour).
    pub fn new(
        store: Arc<CertificateStore>,
        signing_key: FalconPrivateKey,
        responder_id: String,
        validity_period: Option<Duration>,
    ) -> TrustChainResult<Self> {
        let falcon = FalconCrypto::new()
            .map_err(|e| TrustChainError::CryptoError {
                reason: format!("Failed to initialize FALCON-1024 for OCSP: {}", e),
            })?;

        info!(
            "OCSP responder initialized: id={}, validity={}s",
            responder_id,
            validity_period
                .unwrap_or_else(|| Duration::from_secs(3600))
                .as_secs()
        );

        Ok(Self {
            store,
            signing_key,
            falcon,
            responder_id,
            validity_period: validity_period.unwrap_or_else(|| Duration::from_secs(3600)),
            federation_transport: RwLock::new(None),
        })
    }

    /// Attach (or replace) the federation transport used by
    /// [`Self::federated_check`] to query peers when the local store
    /// returns Unknown (Phase F.2).
    ///
    /// Default behaviour without a transport set: local-only check (alpha
    /// behaviour, mirrors the F.1 pattern of opt-in federation).
    pub async fn set_federation_transport(
        &self,
        transport: Arc<dyn FederationOcspTransport>,
    ) {
        *self.federation_transport.write().await = Some(transport);
    }

    /// Check the revocation status of a certificate and return a signed response.
    pub async fn check_status(
        &self,
        request: &OcspRequest,
    ) -> TrustChainResult<OcspResponse> {
        debug!("OCSP status check for serial: {}", request.serial_number);

        let now = SystemTime::now();
        let next_update = now + self.validity_period;

        // Look up certificate in store by serial number
        let cert_status = match self.store.get_certificate_by_serial(&request.serial_number).await? {
            Some(cert) => self.map_certificate_status(&cert.status),
            None => {
                warn!("OCSP: certificate not found: {}", request.serial_number);
                OcspCertStatus::Unknown
            }
        };

        // Build the response payload for signing
        let payload = self.build_signing_payload(
            &cert_status,
            &request.serial_number,
            now,
            next_update,
        );

        // Sign with FALCON-1024
        let signature = self
            .falcon
            .sign(&payload, &self.signing_key)
            .await
            .map_err(|e| TrustChainError::CryptoError {
                reason: format!("OCSP response signing failed: {}", e),
            })?;

        debug!(
            "OCSP response signed for serial={}, status={:?}",
            request.serial_number, cert_status
        );

        Ok(OcspResponse {
            status: cert_status,
            this_update: now,
            next_update,
            serial_number: request.serial_number.clone(),
            responder_id: self.responder_id.clone(),
            signature,
            produced_at: now,
        })
    }

    /// Check certificate revocation status across the federation.
    ///
    /// 1. Local store: if a matching cert is found, return its status.
    /// 2. Federation: if a transport is attached (via
    ///    [`Self::set_federation_transport`]) and the local store says
    ///    Unknown, query each Full/Conditional peer.  Any peer reporting
    ///    `Revoked` short-circuits the lookup.  If all peers report
    ///    Unknown (or none answer), the result is Unknown.
    /// 3. With no transport attached, behaviour is the alpha default —
    ///    local-only — so single-node CAs preserve their existing flow.
    pub async fn federated_check(
        &self,
        serial_number: &str,
        federation: &FederationManager,
    ) -> OcspCertStatus {
        // 1. Check local store first
        match self.store.get_certificate_by_serial(serial_number).await {
            Ok(Some(cert)) => {
                let status = self.map_certificate_status(&cert.status);
                debug!(
                    "OCSP federated check for serial={}: local status={:?}",
                    serial_number, status
                );
                return status;
            }
            Ok(None) => {
                debug!(
                    "OCSP federated check: serial={} not found locally, querying federation",
                    serial_number
                );
            }
            Err(e) => {
                warn!(
                    "OCSP federated check: local store error for serial={}: {}",
                    serial_number, e
                );
            }
        }

        // 2. Federation fallback (Phase F.2).  Only when a transport is
        //    attached AND the federation has at least one trusted peer.
        let transport = match self.federation_transport.read().await.as_ref().cloned() {
            Some(t) => t,
            None => {
                debug!(
                    "OCSP federated check: no transport attached, returning Unknown for {}",
                    serial_number
                );
                return OcspCertStatus::Unknown;
            }
        };

        let mut had_revoked = false;
        let mut revoked_status = None;
        for peer in federation.list_peers().await {
            if matches!(peer.trust_level, FederationTrustLevel::Untrusted) {
                continue;
            }
            match transport
                .query_peer_revocation(&peer.ca_id, serial_number)
                .await
            {
                Some(OcspCertStatus::Revoked { revocation_time, reason }) => {
                    info!(
                        "OCSP federated check: peer '{}' reports serial={} REVOKED",
                        peer.ca_id, serial_number
                    );
                    had_revoked = true;
                    revoked_status = Some(OcspCertStatus::Revoked {
                        revocation_time,
                        reason,
                    });
                    break;
                }
                Some(OcspCertStatus::Good) => {
                    debug!(
                        "OCSP federated check: peer '{}' reports serial={} GOOD",
                        peer.ca_id, serial_number
                    );
                    // Don't return early — another peer may know it's revoked.
                }
                Some(OcspCertStatus::Unknown) | None => {
                    // Peer didn't answer or doesn't know; keep polling others.
                }
            }
        }

        if had_revoked {
            revoked_status.unwrap_or(OcspCertStatus::Unknown)
        } else {
            OcspCertStatus::Unknown
        }
    }

    /// Map internal `CertificateStatus` to `OcspCertStatus`.
    fn map_certificate_status(&self, status: &CertificateStatus) -> OcspCertStatus {
        match status {
            CertificateStatus::Valid => OcspCertStatus::Good,
            CertificateStatus::Revoked { reason, revoked_at } => {
                OcspCertStatus::Revoked {
                    revocation_time: *revoked_at,
                    reason: Self::parse_revocation_reason(reason),
                }
            }
            CertificateStatus::Expired => OcspCertStatus::Good,
            // Expired certificates are not revoked; the client should
            // check validity dates separately per RFC 6960.
        }
    }

    /// Parse a reason string into a `RevocationReason` enum value.
    fn parse_revocation_reason(reason: &str) -> RevocationReason {
        match reason.to_lowercase().as_str() {
            "keycompromise" | "key_compromise" | "key compromise" => {
                RevocationReason::KeyCompromise
            }
            "cacompromise" | "ca_compromise" | "ca compromise" => {
                RevocationReason::CaCompromise
            }
            "affiliationchanged" | "affiliation_changed" | "affiliation changed" => {
                RevocationReason::AffiliationChanged
            }
            "superseded" => RevocationReason::Superseded,
            "cessationofoperation" | "cessation_of_operation" | "cessation of operation" => {
                RevocationReason::CessationOfOperation
            }
            "privilegewithdrawn" | "privilege_withdrawn" | "privilege withdrawn" => {
                RevocationReason::PrivilegeWithdrawn
            }
            _ => RevocationReason::Unspecified,
        }
    }

    /// Build a deterministic byte payload for signing.
    fn build_signing_payload(
        &self,
        status: &OcspCertStatus,
        serial: &str,
        this_update: SystemTime,
        next_update: SystemTime,
    ) -> Vec<u8> {
        // Serialize the critical response fields deterministically
        let status_tag = match status {
            OcspCertStatus::Good => b"good".to_vec(),
            OcspCertStatus::Revoked { .. } => b"revoked".to_vec(),
            OcspCertStatus::Unknown => b"unknown".to_vec(),
        };

        let this_secs = this_update
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let next_secs = next_update
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut payload = Vec::with_capacity(128);
        payload.extend_from_slice(b"OCSP-RESPONSE:");
        payload.extend_from_slice(&status_tag);
        payload.extend_from_slice(b":");
        payload.extend_from_slice(serial.as_bytes());
        payload.extend_from_slice(b":");
        payload.extend_from_slice(self.responder_id.as_bytes());
        payload.extend_from_slice(b":");
        payload.extend_from_slice(&this_secs.to_le_bytes());
        payload.extend_from_slice(b":");
        payload.extend_from_slice(&next_secs.to_le_bytes());
        payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyUsage;

    /// Helper: create a test certificate store and OCSP responder.
    async fn setup_responder() -> (Arc<CertificateStore>, OcspResponder) {
        let store = Arc::new(
            CertificateStore::new()
                .await
                .expect("test: failed to create store"),
        );

        let falcon = FalconCrypto::new().expect("test: failed to init FALCON");
        let keypair = falcon
            .generate_keypair(KeyUsage::CertificateAuthority)
            .await
            .expect("test: failed to generate keypair");

        let responder = OcspResponder::new(
            Arc::clone(&store),
            keypair.private_key,
            "test-ocsp-responder".to_string(),
            Some(Duration::from_secs(300)),
        )
        .expect("test: failed to create OCSP responder");

        (store, responder)
    }

    fn make_request(serial: &str) -> OcspRequest {
        OcspRequest {
            serial_number: serial.to_string(),
            issuer_name_hash: [0u8; 32],
            issuer_key_hash: [0u8; 32],
        }
    }

    #[tokio::test]
    async fn test_ocsp_unknown_certificate() {
        let (_store, responder) = setup_responder().await;

        let request = make_request("nonexistent-serial");
        let response = responder
            .check_status(&request)
            .await
            .expect("test: check_status failed");

        assert!(matches!(response.status, OcspCertStatus::Unknown));
        assert_eq!(response.serial_number, "nonexistent-serial");
        assert_eq!(response.responder_id, "test-ocsp-responder");
        assert!(!response.signature.signature_bytes.is_empty());
    }

    #[tokio::test]
    async fn test_ocsp_good_certificate() {
        let (store, responder) = setup_responder().await;

        // Store a valid certificate
        let cert = create_test_certificate("serial-001", CertificateStatus::Valid);
        store
            .store_certificate(&cert)
            .await
            .expect("test: store_certificate failed");

        let request = make_request("serial-001");
        let response = responder
            .check_status(&request)
            .await
            .expect("test: check_status failed");

        assert!(matches!(response.status, OcspCertStatus::Good));
        assert_eq!(response.serial_number, "serial-001");
        assert!(!response.signature.signature_bytes.is_empty());
        assert!(response.next_update > response.this_update);
    }

    #[tokio::test]
    async fn test_ocsp_revoked_certificate() {
        let (store, responder) = setup_responder().await;

        let revoked_at = SystemTime::now();
        let cert = create_test_certificate(
            "serial-revoked",
            CertificateStatus::Revoked {
                reason: "KeyCompromise".to_string(),
                revoked_at,
            },
        );
        store
            .store_certificate(&cert)
            .await
            .expect("test: store_certificate failed");

        let request = make_request("serial-revoked");
        let response = responder
            .check_status(&request)
            .await
            .expect("test: check_status failed");

        match &response.status {
            OcspCertStatus::Revoked {
                reason,
                revocation_time,
            } => {
                assert!(matches!(reason, RevocationReason::KeyCompromise));
                assert_eq!(*revocation_time, revoked_at);
            }
            other => unreachable!("Expected Revoked status, got {:?}", other),
        }
        assert!(!response.signature.signature_bytes.is_empty());
    }

    #[tokio::test]
    async fn test_ocsp_signature_present_on_all_responses() {
        let (store, responder) = setup_responder().await;

        // Test with a stored valid cert
        let cert = create_test_certificate("serial-sig", CertificateStatus::Valid);
        store
            .store_certificate(&cert)
            .await
            .expect("test: store_certificate failed");

        for serial in &["serial-sig", "nonexistent"] {
            let request = make_request(serial);
            let response = responder
                .check_status(&request)
                .await
                .expect("test: check_status failed");

            assert!(
                !response.signature.signature_bytes.is_empty(),
                "Signature must be present for serial={}",
                serial
            );
            assert_eq!(response.signature.algorithm, "FALCON-1024");
        }
    }

    /// Helper to create a minimal test certificate for store insertion.
    fn create_test_certificate(
        serial: &str,
        status: CertificateStatus,
    ) -> super::super::IssuedCertificate {
        use crate::proof_of_state::StateProof;

        super::super::IssuedCertificate {
            serial_number: serial.to_string(),
            certificate_der: vec![0u8; 32],
            certificate_pem: String::new(),
            chain_pem: String::new(),
            fingerprint: [0u8; 32],
            common_name: format!("test-{}", serial),
            issued_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(86400),
            issuer_ca_id: "test-ca".to_string(),
            state_proof: StateProof::default_for_testing(),
            status,
            metadata: super::super::CertificateMetadata::default(),
        }
    }

    #[tokio::test]
    async fn test_ocsp_federated_check_revoked_locally() {
        use super::super::federation::{FederationManager, FederationPolicy};

        let (store, responder) = setup_responder().await;
        let fm = FederationManager::new(
            "local-ca".into(),
            FederationPolicy::default(),
        );

        // Store a revoked certificate locally
        let revoked_at = SystemTime::now();
        let cert = create_test_certificate(
            "serial-fed-rev",
            CertificateStatus::Revoked {
                reason: "KeyCompromise".to_string(),
                revoked_at,
            },
        );
        store
            .store_certificate(&cert)
            .await
            .expect("test: store_certificate failed");

        let status = responder.federated_check("serial-fed-rev", &fm).await;
        assert!(
            matches!(status, OcspCertStatus::Revoked { .. }),
            "locally revoked cert should return Revoked from federated check"
        );
    }

    #[tokio::test]
    async fn test_ocsp_federated_check_good_locally() {
        use super::super::federation::{FederationManager, FederationPolicy};

        let (store, responder) = setup_responder().await;
        let fm = FederationManager::new(
            "local-ca".into(),
            FederationPolicy::default(),
        );

        let cert = create_test_certificate("serial-fed-good", CertificateStatus::Valid);
        store
            .store_certificate(&cert)
            .await
            .expect("test: store_certificate failed");

        let status = responder.federated_check("serial-fed-good", &fm).await;
        assert!(
            matches!(status, OcspCertStatus::Good),
            "locally valid cert should return Good from federated check"
        );
    }

    #[tokio::test]
    async fn test_ocsp_federated_check_unknown_locally() {
        use super::super::federation::{FederationManager, FederationPolicy};

        let (_store, responder) = setup_responder().await;
        let fm = FederationManager::new(
            "local-ca".into(),
            FederationPolicy::default(),
        );

        let status = responder
            .federated_check("serial-nonexistent", &fm)
            .await;
        assert!(
            matches!(status, OcspCertStatus::Unknown),
            "unknown cert should return Unknown in alpha (no remote query)"
        );
    }
}
