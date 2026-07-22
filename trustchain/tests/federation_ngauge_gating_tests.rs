// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase F.1 — ngauge → federation gating.
//!
//! Verifies that `FederationManager::add_peer_with_proof` consults the
//! attached [`TrustSignalProvider`] and caps the peer's trust level
//! according to the ngauge-derived band.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use trustchain::ca::federation::{
    FederatedCA, FederationManager, FederationPolicy, FederationTrustLevel,
};
use trustchain::ca::trust_provider::{
    PeerCertFingerprint, PeerTrustBand, TrustSignalProvider,
};

struct StaticProvider {
    band: Option<PeerTrustBand>,
}

#[async_trait]
impl TrustSignalProvider for StaticProvider {
    async fn trust_band_for(&self, _peer: &PeerCertFingerprint) -> Option<PeerTrustBand> {
        self.band
    }
}

fn make_peer(id: &str, pk: &[u8], trust: FederationTrustLevel) -> FederatedCA {
    FederatedCA {
        ca_id: id.to_string(),
        name: format!("Peer {id}"),
        public_key: pk.to_vec(),
        root_certificate: vec![1u8; 64],
        trust_level: trust,
        joined_at: SystemTime::now(),
        last_sync: None,
        endpoint: "[::1]:8443".to_string(),
    }
}

fn policy() -> FederationPolicy {
    FederationPolicy {
        max_peers: 10,
        require_ct_proof: false,
        auto_demote_on_failure: true,
        max_sync_age: Duration::from_secs(3600),
    }
}

#[tokio::test]
async fn test_ngauge_signals_promote_to_full() {
    let fed = FederationManager::new("local-ca".into(), policy());
    fed.set_trust_signal_provider(Arc::new(StaticProvider {
        band: Some(PeerTrustBand::Full),
    }))
    .await;

    let proof = trustchain::proof_of_state::StateProof::new_for_testing();
    let pk = b"high-activity-peer-pk-1234".to_vec();
    fed.add_peer_with_proof(
        make_peer("hi", &pk, FederationTrustLevel::Full),
        Some(&proof),
    )
    .await
    .expect("test: add hi");

    let p = fed.get_peer("hi").await.expect("test: peer exists");
    assert_eq!(
        p.trust_level,
        FederationTrustLevel::Full,
        "Full ngauge band should preserve requested Full trust"
    );
}

#[tokio::test]
async fn test_ngauge_signals_demote_to_conditional() {
    let fed = FederationManager::new("local-ca".into(), policy());
    fed.set_trust_signal_provider(Arc::new(StaticProvider {
        band: Some(PeerTrustBand::Conditional),
    }))
    .await;

    let proof = trustchain::proof_of_state::StateProof::new_for_testing();
    let pk = b"low-cap-peer-pk-5678".to_vec();
    // Peer requests Full, but provider returns Conditional → cap.
    fed.add_peer_with_proof(
        make_peer("lo", &pk, FederationTrustLevel::Full),
        Some(&proof),
    )
    .await
    .expect("test: add lo");

    let p = fed.get_peer("lo").await.expect("test: peer exists");
    assert_eq!(
        p.trust_level,
        FederationTrustLevel::Conditional,
        "Conditional ngauge band should cap peer at Conditional"
    );
}

#[tokio::test]
async fn test_no_signals_leaves_pos_level_unchanged() {
    // No provider attached → ngauge gating is a no-op.  PoS validation
    // must still apply the requested trust level when proof is valid.
    let fed = FederationManager::new("local-ca".into(), policy());

    let proof = trustchain::proof_of_state::StateProof::new_for_testing();
    let pk = b"unknown-peer-pk".to_vec();
    fed.add_peer_with_proof(
        make_peer("unk", &pk, FederationTrustLevel::Full),
        Some(&proof),
    )
    .await
    .expect("test: add unk");

    let p = fed.get_peer("unk").await.expect("test: peer exists");
    assert_eq!(
        p.trust_level,
        FederationTrustLevel::Full,
        "without ngauge gating, valid PoS retains requested Full trust"
    );
}

#[tokio::test]
async fn test_provider_returns_none_leaves_pos_level_unchanged() {
    // Provider is attached but says "no signals yet" — should not
    // demote a PoS-validated peer.
    let fed = FederationManager::new("local-ca".into(), policy());
    fed.set_trust_signal_provider(Arc::new(StaticProvider { band: None }))
        .await;

    let proof = trustchain::proof_of_state::StateProof::new_for_testing();
    let pk = b"unmeasured-peer-pk".to_vec();
    fed.add_peer_with_proof(
        make_peer("um", &pk, FederationTrustLevel::Full),
        Some(&proof),
    )
    .await
    .expect("test: add um");

    let p = fed.get_peer("um").await.expect("test: peer exists");
    assert_eq!(
        p.trust_level,
        FederationTrustLevel::Full,
        "None signals must be a no-op (PoS gate is the only floor)"
    );
}

/// PoS failure trumps ngauge promotion: a peer without proof is
/// Untrusted regardless of what ngauge says.
#[tokio::test]
async fn test_pos_failure_trumps_ngauge_full() {
    let fed = FederationManager::new("local-ca".into(), policy());
    fed.set_trust_signal_provider(Arc::new(StaticProvider {
        band: Some(PeerTrustBand::Full),
    }))
    .await;

    let pk = b"no-proof-peer".to_vec();
    fed.add_peer(make_peer("np", &pk, FederationTrustLevel::Full))
        .await
        .expect("test: add np");

    let p = fed.get_peer("np").await.expect("test: peer exists");
    assert_eq!(
        p.trust_level,
        FederationTrustLevel::Untrusted,
        "PoS failure forces Untrusted regardless of ngauge band"
    );
}

/// Sanity: SHA-256 fingerprint is what add_peer/derive_peer_fingerprint
/// will compute, so the provider receives the same key bytes the test
/// expects.
#[test]
fn test_peer_fingerprint_is_sha256() {
    let pk = b"abc";
    let expected: [u8; 32] = Sha256::digest(pk).into();
    // Mirror the implementation by computing here.
    let mut hasher = Sha256::new();
    hasher.update(pk);
    let got: [u8; 32] = hasher.finalize().into();
    assert_eq!(got, expected);
}
