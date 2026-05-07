// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase F.2 integration tests: OCSP federation fallback, CRL auto-
//! propagation, cross-CA validation against trusted federated peers.
//!
//! All tests use the in-process `RecordingTransport` mock so they exercise
//! the real `OcspResponder`/`CrlDistributor`/`CertificateStore` wiring
//! without needing a live STOQ peer.  The transport records every call
//! and lets each test plant per-peer responses.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey};
use sha2::{Digest, Sha256};

use trustchain::ca::certificate_store::CertificateStore;
use trustchain::ca::crl::CrlDistributor;
use trustchain::ca::federation::{
    FederatedCA, FederationManager, FederationPolicy, FederationTrustLevel,
};
use trustchain::ca::ocsp::{
    FederationOcspTransport, OcspCertStatus, OcspResponder, OcspRequest,
};
use trustchain::ca::stoq_ca_client::RevocationReason;
use trustchain::ca::types::{CertificateMetadata, CertificateStatus, IssuedCertificate};
use trustchain::crypto::falcon::FalconCrypto;
use trustchain::crypto::KeyUsage;
use trustchain::proof_of_state::StateProof;

// ---------------------------------------------------------------------------
// Test harness — recording mock transport
// ---------------------------------------------------------------------------

/// Per-peer answers a test wants the mock transport to return for each
/// `query_peer_revocation` call.
#[derive(Clone)]
struct PeerAnswer {
    revocation: Option<OcspCertStatus>,
    accept_push: bool,
}

impl PeerAnswer {
    fn revoked(reason: RevocationReason) -> Self {
        Self {
            revocation: Some(OcspCertStatus::Revoked {
                revocation_time: SystemTime::now(),
                reason,
            }),
            accept_push: true,
        }
    }

    fn unknown() -> Self {
        Self {
            revocation: Some(OcspCertStatus::Unknown),
            accept_push: true,
        }
    }
}

#[derive(Clone, Debug)]
struct PushRecord {
    peer_ca_id: String,
    serial: String,
    reason: String,
}

struct RecordingTransport {
    answers: Mutex<HashMap<String, PeerAnswer>>,
    pushed: Mutex<Vec<PushRecord>>,
    queried: Mutex<Vec<(String, String)>>,
}

impl RecordingTransport {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            answers: Mutex::new(HashMap::new()),
            pushed: Mutex::new(Vec::new()),
            queried: Mutex::new(Vec::new()),
        })
    }

    fn set_answer(&self, peer_ca_id: &str, answer: PeerAnswer) {
        self.answers
            .lock()
            .expect("test: answers lock")
            .insert(peer_ca_id.to_string(), answer);
    }

    fn pushes(&self) -> Vec<PushRecord> {
        self.pushed.lock().expect("test: pushed lock").clone()
    }
}

#[async_trait]
impl FederationOcspTransport for RecordingTransport {
    async fn query_peer_revocation(
        &self,
        peer_ca_id: &str,
        serial_number: &str,
    ) -> Option<OcspCertStatus> {
        self.queried
            .lock()
            .expect("test: queried lock")
            .push((peer_ca_id.to_string(), serial_number.to_string()));
        self.answers
            .lock()
            .expect("test: answers lock")
            .get(peer_ca_id)
            .and_then(|a| a.revocation.clone())
    }

    async fn push_revocation(
        &self,
        peer_ca_id: &str,
        serial_number: &str,
        reason: &str,
    ) -> bool {
        self.pushed.lock().expect("test: pushed lock").push(PushRecord {
            peer_ca_id: peer_ca_id.to_string(),
            serial: serial_number.to_string(),
            reason: reason.to_string(),
        });
        self.answers
            .lock()
            .expect("test: answers lock")
            .get(peer_ca_id)
            .map(|a| a.accept_push)
            .unwrap_or(true)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_revoked_cert(serial: &str, reason: &str) -> IssuedCertificate {
    IssuedCertificate {
        serial_number: serial.to_string(),
        certificate_der: vec![0u8; 32],
        certificate_pem: String::new(),
        chain_pem: String::new(),
        fingerprint: [0u8; 32],
        common_name: format!("test-{serial}"),
        issued_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(86400),
        issuer_ca_id: "local-ca".to_string(),
        state_proof: StateProof::new_for_testing(),
        status: CertificateStatus::Revoked {
            reason: reason.to_string(),
            revoked_at: SystemTime::now(),
        },
        metadata: CertificateMetadata::default(),
    }
}

async fn make_responder() -> (Arc<CertificateStore>, OcspResponder) {
    let store = Arc::new(
        CertificateStore::new()
            .await
            .expect("test: create cert store"),
    );
    let falcon = FalconCrypto::new().expect("test: init FALCON");
    let kp = falcon
        .generate_keypair(KeyUsage::CertificateAuthority)
        .await
        .expect("test: keypair");
    let responder = OcspResponder::new(
        Arc::clone(&store),
        kp.private_key,
        "test-responder".to_string(),
        Some(Duration::from_secs(60)),
    )
    .expect("test: ocsp responder");
    (store, responder)
}

fn make_federated_peer(ca_id: &str, public_key: Vec<u8>, level: FederationTrustLevel) -> FederatedCA {
    FederatedCA {
        ca_id: ca_id.to_string(),
        name: format!("peer-{ca_id}"),
        public_key,
        root_certificate: vec![1u8; 64],
        trust_level: level,
        joined_at: SystemTime::now(),
        last_sync: None,
        endpoint: "[::1]:8443".to_string(),
    }
}

async fn add_trusted_peer(
    fm: &FederationManager,
    ca_id: &str,
    public_key: Vec<u8>,
    level: FederationTrustLevel,
) {
    let peer = make_federated_peer(ca_id, public_key, level);
    let proof = StateProof::new_for_testing();
    fm.add_peer_with_proof(peer, Some(&proof))
        .await
        .expect("test: add peer with proof");
}

/// Build a FALCON-signed cert blob in the wire format expected by
/// `verify_falcon_signature`: `[4B sig_len LE][signature][cert_body]`.
fn build_signed_cert_blob(body: &[u8], sk: &falcon1024::SecretKey) -> Vec<u8> {
    let hash: [u8; 32] = Sha256::digest(body).into();
    let sig = falcon1024::detached_sign(&hash, sk);
    let sig_bytes = sig.as_bytes();
    let mut blob = (sig_bytes.len() as u32).to_le_bytes().to_vec();
    blob.extend_from_slice(sig_bytes);
    blob.extend_from_slice(body);
    blob
}

// ---------------------------------------------------------------------------
// 1. OCSP local revoked → returns Revoked
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ocsp_local_revoked_returns_revoked() {
    let (store, responder) = make_responder().await;
    store
        .store_certificate(&make_revoked_cert("serial-100", "KeyCompromise"))
        .await
        .expect("test: store revoked cert");

    let req = OcspRequest {
        serial_number: "serial-100".to_string(),
        issuer_name_hash: [0u8; 32],
        issuer_key_hash: [0u8; 32],
    };
    let resp = responder
        .check_status(&req)
        .await
        .expect("test: ocsp local check");
    assert!(
        matches!(resp.status, OcspCertStatus::Revoked { .. }),
        "expected Revoked, got {:?}",
        resp.status
    );
}

// ---------------------------------------------------------------------------
// 2. OCSP unknown locally → falls back to federation peer that says revoked
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ocsp_unknown_falls_back_to_federation() {
    let (_store, responder) = make_responder().await;
    let fm = FederationManager::new("local-ca".into(), FederationPolicy::default());
    let transport = RecordingTransport::new();

    // Wire opt-in transport (Phase F.2).
    responder
        .set_federation_transport(transport.clone())
        .await;

    // Add a Full-trust peer; mock transport reports the cert as revoked.
    add_trusted_peer(
        &fm,
        "peer-knows",
        vec![0xAA; 32],
        FederationTrustLevel::Full,
    )
    .await;
    transport.set_answer(
        "peer-knows",
        PeerAnswer::revoked(RevocationReason::CaCompromise),
    );

    let status = responder
        .federated_check("serial-not-local", &fm)
        .await;
    assert!(
        matches!(status, OcspCertStatus::Revoked { .. }),
        "expected federated Revoked, got {:?}",
        status
    );
}

// ---------------------------------------------------------------------------
// 3. OCSP federation unanimous Unknown → returns Unknown
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ocsp_federation_unanimous_unknown() {
    let (_store, responder) = make_responder().await;
    let fm = FederationManager::new("local-ca".into(), FederationPolicy::default());
    let transport = RecordingTransport::new();
    responder
        .set_federation_transport(transport.clone())
        .await;

    for ca_id in &["peer-a", "peer-b", "peer-c"] {
        add_trusted_peer(
            &fm,
            ca_id,
            vec![0xBB; 32],
            FederationTrustLevel::Conditional,
        )
        .await;
        transport.set_answer(ca_id, PeerAnswer::unknown());
    }

    let status = responder
        .federated_check("serial-nobody-knows", &fm)
        .await;
    assert!(
        matches!(status, OcspCertStatus::Unknown),
        "expected Unknown when all peers Unknown, got {:?}",
        status
    );
}

// ---------------------------------------------------------------------------
// 4. Revocation propagates to federation when context attached
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_revocation_propagates_to_federation() {
    let store = Arc::new(
        CertificateStore::new()
            .await
            .expect("test: create cert store"),
    );

    // Pre-populate a cert so revoke_certificate finds it.
    let mut valid_cert = make_revoked_cert("serial-prop", "Unspecified");
    valid_cert.status = CertificateStatus::Valid;
    store
        .store_certificate(&valid_cert)
        .await
        .expect("test: store valid");

    let fm = Arc::new(FederationManager::new(
        "local-ca".into(),
        FederationPolicy::default(),
    ));
    add_trusted_peer(
        &fm,
        "peer-receiver",
        vec![0xCC; 32],
        FederationTrustLevel::Full,
    )
    .await;

    let crl = Arc::new(CrlDistributor::new());
    let transport = RecordingTransport::new();
    transport.set_answer("peer-receiver", PeerAnswer::unknown());
    crl.set_federation_transport(transport.clone()).await;

    // Phase F.2 wiring: store has both federation + crl, so revoke
    // auto-pushes.
    store
        .set_federation(Arc::clone(&fm), Arc::clone(&crl))
        .await;

    store
        .revoke_certificate("serial-prop", "KeyCompromise".to_string())
        .await
        .expect("test: revoke");

    let pushed = transport.pushes();
    assert_eq!(
        pushed.len(),
        1,
        "expected exactly one push, got {pushed:?}"
    );
    assert_eq!(pushed[0].peer_ca_id, "peer-receiver");
    assert_eq!(pushed[0].serial, "serial-prop");
    assert_eq!(pushed[0].reason, "KeyCompromise");
}

// ---------------------------------------------------------------------------
// 5. Cross-CA validation accepts a federated-peer-signed cert
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cross_ca_validation_accepts_federated() {
    let fm = FederationManager::new("local-ca".into(), FederationPolicy::default());

    // Generate a real FALCON keypair and admit it as a Full peer.
    let (pk, sk) = falcon1024::keypair();
    add_trusted_peer(
        &fm,
        "peer-signer",
        pk.as_bytes().to_vec(),
        FederationTrustLevel::Full,
    )
    .await;

    let blob = build_signed_cert_blob(b"federation-issued-cert-body", &sk);
    assert!(
        fm.is_federation_signed(&blob).await,
        "Full-trust peer's signature should be accepted"
    );
}

// ---------------------------------------------------------------------------
// 6. Cross-CA validation rejects a cert signed by an unknown key
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cross_ca_validation_rejects_unknown_signer() {
    let fm = FederationManager::new("local-ca".into(), FederationPolicy::default());

    // Federation has one trusted peer.
    let (pk_known, _sk_known) = falcon1024::keypair();
    add_trusted_peer(
        &fm,
        "peer-known",
        pk_known.as_bytes().to_vec(),
        FederationTrustLevel::Full,
    )
    .await;

    // But the cert is signed by a *different* random keypair.
    let (_pk_evil, sk_evil) = falcon1024::keypair();
    let blob = build_signed_cert_blob(b"unauthorized-cert", &sk_evil);

    assert!(
        !fm.is_federation_signed(&blob).await,
        "Unknown signer must be rejected"
    );
}
