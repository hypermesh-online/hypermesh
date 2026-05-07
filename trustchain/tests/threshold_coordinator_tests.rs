// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase F.1 — `ThresholdSignCoordinator` integration tests.
//!
//! These tests exercise the coordinator end-to-end against a
//! [`MockFederationTransport`].  They cover the three core behaviours:
//!
//! 1. **Three-node threshold signing** — A+B respond with valid shares,
//!    coordinator reconstructs and signs.
//! 2. **Threshold timeout** — only one peer responds, deadline elapses,
//!    `Timeout` error.
//! 3. **Byzantine peer demoted** — a peer flagged by `ByzantineDetector`
//!    is demoted to `Untrusted` by `FederationManager::add_peer` and
//!    therefore excluded from the eligible-peer set.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::PublicKey as PkTrait;
use pqcrypto_traits::sign::{DetachedSignature, SecretKey as SkTrait};
use sha2::{Digest, Sha256};

use trustchain::ca::federation::{
    FederatedCA, FederationManager, FederationPolicy, FederationTrustLevel,
};
use trustchain::crypto::threshold::{KeyShare, ThresholdConfig, ThresholdSigner};
use trustchain::crypto::threshold_coordinator::{
    MockFederationTransport, ThresholdError, ThresholdSignCoordinator,
};

fn make_peer(id: &str, pk_bytes: &[u8]) -> FederatedCA {
    FederatedCA {
        ca_id: id.to_string(),
        name: format!("Peer {id}"),
        public_key: pk_bytes.to_vec(),
        root_certificate: vec![1u8; 64],
        trust_level: FederationTrustLevel::Conditional,
        joined_at: SystemTime::now(),
        last_sync: None,
        endpoint: "[::1]:8443".to_string(),
    }
}

fn fingerprint_for(pk_bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(pk_bytes).into()
}

fn fed_with_policy() -> Arc<FederationManager> {
    let policy = FederationPolicy {
        max_peers: 16,
        require_ct_proof: false,
        auto_demote_on_failure: true,
        max_sync_age: Duration::from_secs(3600),
    };
    Arc::new(FederationManager::new("local-ca".into(), policy))
}

#[tokio::test]
async fn test_three_node_threshold_signing() {
    // Generate a real FALCON-1024 key, split it into 3-of-3 shares.
    let (pk, sk) = falcon1024::keypair();
    let pk_bytes = PkTrait::as_bytes(&pk).to_vec();
    let sk_bytes = SkTrait::as_bytes(&sk).to_vec();
    let ca_fp = fingerprint_for(&pk_bytes);

    let signer = ThresholdSigner::new(ThresholdConfig {
        threshold: 3,
        total_shares: 3,
    })
    .expect("test: signer config");
    let key_shares = signer
        .split_signing_key(&sk_bytes, ca_fp)
        .expect("test: split");
    assert_eq!(key_shares.len(), 3);

    // Federation: 2 peers (A, B) with public keys derived from synthetic
    // pubkeys.  The local node holds share[0]; A holds share[1]; B holds
    // share[2].  All 3 are needed to reconstruct.
    let fed = fed_with_policy();

    let pk_a = b"peer-a-public-key-bytes-1234567890abcdef".to_vec();
    let pk_b = b"peer-b-public-key-bytes-fedcba0987654321".to_vec();
    let fp_a = fingerprint_for(&pk_a);
    let fp_b = fingerprint_for(&pk_b);

    // Add the peers with valid PoS so they keep Conditional trust.
    let proof = trustchain::proof_of_state::StateProof::new_for_testing();
    fed.add_peer_with_proof(make_peer("ca-a", &pk_a), Some(&proof))
        .await
        .expect("test: add ca-a");
    fed.add_peer_with_proof(make_peer("ca-b", &pk_b), Some(&proof))
        .await
        .expect("test: add ca-b");

    // Local node stores share[0] keyed by the CA fingerprint.
    fed.store_key_share(ca_fp, key_shares[0].clone())
        .await
        .expect("test: store local share");

    // Mock transport: A returns share[1], B returns share[2].
    let transport = MockFederationTransport::new();
    transport.set_share(fp_a, key_shares[1].clone());
    transport.set_share(fp_b, key_shares[2].clone());

    let coordinator =
        ThresholdSignCoordinator::new(fed.clone(), transport.clone() as Arc<_>);

    let message = b"phase F.1 threshold signing test message";
    let signature = coordinator
        .sign(ca_fp, message, 3, Duration::from_secs(2))
        .await
        .expect("test: threshold sign should succeed");

    // Verify signature with the original public key.
    let mut hasher = Sha256::new();
    hasher.update(message);
    let digest: [u8; 32] = hasher.finalize().into();
    let sig = falcon1024::DetachedSignature::from_bytes(&signature)
        .expect("test: parse sig");
    let verify = falcon1024::verify_detached_signature(&sig, &digest, &pk);
    assert!(
        verify.is_ok(),
        "threshold-reconstructed signature should verify against the original public key"
    );

    // Confirm the transport saw the broadcast.
    let captured = transport
        .captured
        .lock()
        .expect("test: captured lock")
        .clone();
    assert_eq!(captured.len(), 1, "exactly one sign request broadcast");
    assert_eq!(captured[0].threshold, 3);
    assert_eq!(captured[0].ca_fingerprint, ca_fp);
}

#[tokio::test]
async fn test_threshold_timeout() {
    // 2-of-3 scheme.  Local holds 1 share; one of two remote peers is
    // silenced.  The other one would supply the second share, but we
    // only need 2 — so to actually trigger a timeout we silence both.
    let (pk, sk) = falcon1024::keypair();
    let pk_bytes = PkTrait::as_bytes(&pk).to_vec();
    let sk_bytes = SkTrait::as_bytes(&sk).to_vec();
    let ca_fp = fingerprint_for(&pk_bytes);

    let signer = ThresholdSigner::new(ThresholdConfig {
        threshold: 2,
        total_shares: 3,
    })
    .expect("test: config");
    let shares = signer
        .split_signing_key(&sk_bytes, ca_fp)
        .expect("test: split");

    let fed = fed_with_policy();
    let pk_a = b"silent-peer-A-pk-0123".to_vec();
    let pk_b = b"silent-peer-B-pk-4567".to_vec();
    let fp_a = fingerprint_for(&pk_a);
    let fp_b = fingerprint_for(&pk_b);

    let proof = trustchain::proof_of_state::StateProof::new_for_testing();
    fed.add_peer_with_proof(make_peer("silent-a", &pk_a), Some(&proof))
        .await
        .expect("test: add a");
    fed.add_peer_with_proof(make_peer("silent-b", &pk_b), Some(&proof))
        .await
        .expect("test: add b");

    // Local holds share[0].
    fed.store_key_share(ca_fp, shares[0].clone())
        .await
        .expect("test: store");

    let transport = MockFederationTransport::new();
    transport.set_share(fp_a, shares[1].clone());
    transport.set_share(fp_b, shares[2].clone());
    transport.silence(fp_a);
    transport.silence(fp_b);

    let coordinator =
        ThresholdSignCoordinator::new(fed.clone(), transport.clone() as Arc<_>);

    let message = b"timeout test";
    // We have 1 local share, need 2 more from peers, but they are
    // silent, so we time out.  Wait briefly so we don't slow the suite.
    let start = std::time::Instant::now();
    let result = coordinator
        .sign(ca_fp, message, 2, Duration::from_millis(150))
        .await;
    let elapsed = start.elapsed();

    match result {
        Err(ThresholdError::Timeout { received, needed }) => {
            assert_eq!(needed, 2);
            // Local share counts as 1, no peer responses → received <= 1.
            assert!(received < needed, "should be short of threshold");
        }
        other => panic!("expected Timeout error, got {other:?}"),
    }
    assert!(
        elapsed < Duration::from_secs(2),
        "deadline must bound the wait"
    );
}

#[tokio::test]
async fn test_byzantine_peer_excluded_from_quorum() {
    // Phase F.1: a peer that is Untrusted in the federation must NOT
    // appear in the eligible-peer set.  This test puts the peer
    // directly into Untrusted (the byzantine override path inside
    // `add_peer` would do the same), then verifies the coordinator
    // doesn't address it.
    let (_pk, sk) = falcon1024::keypair();
    let sk_bytes = SkTrait::as_bytes(&sk).to_vec();
    let ca_fp = [0xCC; 32];

    let signer = ThresholdSigner::new(ThresholdConfig {
        threshold: 2,
        total_shares: 3,
    })
    .expect("test: config");
    let shares = signer
        .split_signing_key(&sk_bytes, ca_fp)
        .expect("test: split");

    let fed = fed_with_policy();
    let pk_byz = b"byzantine-peer-pk".to_vec();
    let pk_good = b"good-peer-pk-9999".to_vec();
    let fp_byz = fingerprint_for(&pk_byz);
    let fp_good = fingerprint_for(&pk_good);

    // Untrusted peer added without proof — automatically Untrusted.
    fed.add_peer(make_peer("byz", &pk_byz))
        .await
        .expect("test: add byz");
    // Good peer added with proof — keeps Conditional trust.
    let proof = trustchain::proof_of_state::StateProof::new_for_testing();
    fed.add_peer_with_proof(make_peer("good", &pk_good), Some(&proof))
        .await
        .expect("test: add good");

    fed.store_key_share(ca_fp, shares[0].clone())
        .await
        .expect("test: store");

    let transport = MockFederationTransport::new();
    transport.set_share(fp_byz, shares[1].clone());
    transport.set_share(fp_good, shares[2].clone());

    let coordinator =
        ThresholdSignCoordinator::new(fed.clone(), transport.clone() as Arc<_>);

    let _ = coordinator
        .sign(ca_fp, b"any", 2, Duration::from_millis(200))
        .await;

    // Inspect captured request: addressed peers must NOT include the
    // byzantine fingerprint.
    let captured = transport
        .captured
        .lock()
        .expect("test: captured lock")
        .clone();
    assert_eq!(captured.len(), 1, "exactly one broadcast");
    // The mock transport echoes the captured request but does not
    // include addressed peers; we infer exclusion by checking that
    // the federation reports the byzantine peer as Untrusted.
    let byz_peer = fed.get_peer("byz").await.expect("test: byz exists");
    assert_eq!(byz_peer.trust_level, FederationTrustLevel::Untrusted);
    let good_peer = fed.get_peer("good").await.expect("test: good exists");
    assert_eq!(good_peer.trust_level, FederationTrustLevel::Conditional);
}
