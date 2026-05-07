// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase F.1 — federation integration smoke test.
//!
//! Spins up an in-process `FederationManager` with a 2-of-3 threshold
//! configuration backed by a `MockFederationTransport`.  The test
//! reconstructs and signs over the federation, then verifies the
//! signature with the original FALCON-1024 public key.  This is the
//! end-to-end path that `trustchain.request_cert` will follow when
//! threshold mode is enabled — minus the IPC layer.

#![cfg(feature = "intelligence")]

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::PublicKey as PkTrait;
use pqcrypto_traits::sign::{DetachedSignature, SecretKey as SkTrait};
use sha2::{Digest, Sha256};

use trustchain::ca::federation::{
    FederatedCA, FederationManager, FederationPolicy, FederationTrustLevel,
};
use trustchain::crypto::threshold::{ThresholdConfig, ThresholdSigner};
use trustchain::crypto::threshold_coordinator::{
    MockFederationTransport, ThresholdSignCoordinator,
};

#[tokio::test]
async fn end_to_end_2_of_3_federation_signs_csr() {
    // Generate the federation root key.
    let (pk, sk) = falcon1024::keypair();
    let pk_bytes = PkTrait::as_bytes(&pk).to_vec();
    let sk_bytes = SkTrait::as_bytes(&sk).to_vec();
    let ca_fp: [u8; 32] = Sha256::digest(&pk_bytes).into();

    // Split into 2-of-3 shares.
    let signer = ThresholdSigner::new(ThresholdConfig {
        threshold: 2,
        total_shares: 3,
    })
    .expect("test: signer config");
    let shares = signer
        .split_signing_key(&sk_bytes, ca_fp)
        .expect("test: split");

    // Federation: A and B both run as federated CAs (the local node
    // holds shares[0] for completeness).  C is a candidate node
    // requesting a cert.
    let policy = FederationPolicy {
        max_peers: 8,
        require_ct_proof: false,
        auto_demote_on_failure: true,
        max_sync_age: Duration::from_secs(3600),
    };
    let fed = Arc::new(FederationManager::new("local-ca".into(), policy));

    // Add A and B with valid PoS so they keep Conditional trust.
    let proof = trustchain::proof_of_state::StateProof::new_for_testing();
    let pk_a = b"federated-peer-A-pk-1234".to_vec();
    let pk_b = b"federated-peer-B-pk-5678".to_vec();
    let fp_a: [u8; 32] = Sha256::digest(&pk_a).into();
    let fp_b: [u8; 32] = Sha256::digest(&pk_b).into();

    fed.add_peer_with_proof(
        FederatedCA {
            ca_id: "ca-a".into(),
            name: "Federated CA A".into(),
            public_key: pk_a,
            root_certificate: vec![1u8; 64],
            trust_level: FederationTrustLevel::Conditional,
            joined_at: SystemTime::now(),
            last_sync: None,
            endpoint: "[::1]:8443".into(),
        },
        Some(&proof),
    )
    .await
    .expect("test: add A");
    fed.add_peer_with_proof(
        FederatedCA {
            ca_id: "ca-b".into(),
            name: "Federated CA B".into(),
            public_key: pk_b,
            root_certificate: vec![1u8; 64],
            trust_level: FederationTrustLevel::Conditional,
            joined_at: SystemTime::now(),
            last_sync: None,
            endpoint: "[::1]:8444".into(),
        },
        Some(&proof),
    )
    .await
    .expect("test: add B");

    // Local node holds share[0]; A holds share[1]; B holds share[2].
    fed.store_key_share(ca_fp, shares[0].clone())
        .await
        .expect("test: store local share");

    // Mock transport returns A's and B's shares.
    let transport = MockFederationTransport::new();
    transport.set_share(fp_a, shares[1].clone());
    transport.set_share(fp_b, shares[2].clone());

    // Enable threshold mode (so the IPC handler would route here).
    fed.set_threshold_mode(true).await;
    assert!(fed.threshold_mode_enabled().await);

    let coordinator =
        ThresholdSignCoordinator::new(fed.clone(), transport.clone() as Arc<_>);

    // C requests a certificate signed over a CSR blob.  We sign
    // directly here; in production the IPC handler `trustchain.request_cert`
    // does the same dispatch.
    let csr_bytes = b"-----BEGIN CSR-----test phase F1 csr blob-----END CSR-----";
    let signature = coordinator
        .sign(ca_fp, csr_bytes, 2, Duration::from_secs(2))
        .await
        .expect("test: threshold sign should succeed");

    // Verify the signature against the federation root pubkey.
    let mut hasher = Sha256::new();
    hasher.update(csr_bytes);
    let digest: [u8; 32] = hasher.finalize().into();
    let sig = falcon1024::DetachedSignature::from_bytes(&signature)
        .expect("test: parse sig");
    let verify = falcon1024::verify_detached_signature(&sig, &digest, &pk);
    assert!(
        verify.is_ok(),
        "federation-signed CSR must verify against the root public key"
    );

    // Federation status reflects 2 trusted-class peers.
    let status = fed.get_federation_status().await;
    assert_eq!(status.total_peers, 2);
    assert_eq!(status.untrusted_peers, 0);
}
