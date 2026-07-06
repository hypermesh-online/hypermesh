// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! End-to-end bilateral handshake test with real FALCON-1024 crypto.
//!
//! Proves the full 3-message handshake flow works over a real QUIC
//! connection between two nodes, each with a genuine FALCON-1024
//! identity and a BlockMatrixProofProvider that generates FALCON-signed
//! state proofs.

use std::sync::Arc;

use anyhow::Result;
use stoq::{StoqTransport, TransportConfig};
use trustchain::identity::FalconIdentity;

use blockmatrix::proof_of_state::BlockMatrixProofProvider;
use hypermesh_lib::{NodeSigner, StateProofProvider};

/// Create a StoqTransport bound to localhost on an OS-assigned port.
async fn make_transport() -> Result<StoqTransport> {
    let config = TransportConfig {
        port: 0,
        ..TransportConfig::default()
    };
    StoqTransport::new(config).await
}

/// Full bilateral handshake between two real nodes over QUIC.
///
/// 1. Creates two FalconIdentity instances (real FALCON-1024 keypairs)
/// 2. Creates two BlockMatrixProofProvider instances (real FALCON-signed proofs)
/// 3. Spins up two STOQ transports on localhost
/// 4. Node A connects to Node B
/// 5. Both sides run the bilateral handshake concurrently
/// 6. Asserts identity binding, proof validation, and coordinate exchange
#[tokio::test]
async fn bilateral_handshake_e2e_with_falcon() {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_bilateral_handshake(),
    )
    .await;

    match result {
        Ok(Ok(())) => {} // success
        Ok(Err(e)) => panic!("Handshake failed: {e:#}"),
        Err(_) => panic!("Handshake timed out after 10 seconds"),
    }
}

async fn run_bilateral_handshake() -> Result<()> {
    // --- 1. Generate FALCON-1024 identities for both nodes ---
    let identity_a = FalconIdentity::generate();
    let identity_b = FalconIdentity::generate();

    let node_id_a = identity_a.node_id.clone();
    let node_id_b = identity_b.node_id.clone();
    let pubkey_a = identity_a.public_key.clone();
    let pubkey_b = identity_b.public_key.clone();

    // --- 2. Create BlockMatrixProofProviders with real signers ---
    let signer_a: Arc<dyn NodeSigner + Send + Sync> = Arc::new(identity_a);
    let signer_b: Arc<dyn NodeSigner + Send + Sync> = Arc::new(identity_b);

    let proof_provider_a = BlockMatrixProofProvider::new(
        node_id_a.clone(),
        signer_a.clone(),
    );
    let proof_provider_b = BlockMatrixProofProvider::new(
        node_id_b.clone(),
        signer_b.clone(),
    );

    // Verify proof generation/validation works standalone before network test
    let proof_bytes = proof_provider_a.generate_proof().await
        .expect("test: Node A proof generation");
    assert!(
        proof_provider_b.validate_proof(&proof_bytes).await
            .expect("test: Node B validates A's proof"),
        "Node B should accept Node A's FALCON-signed proof"
    );

    // --- 3. Create STOQ transports ---
    let transport_a = Arc::new(make_transport().await
        .expect("test: transport A"));
    let transport_b = Arc::new(make_transport().await
        .expect("test: transport B"));

    let addr_b = transport_b.local_addr()
        .expect("test: get Node B listen address");

    // Coordinates for the two nodes
    let coord_a: (i64, i64, i64) = (1, 2, 3);
    let coord_b: (i64, i64, i64) = (10, 20, 30);

    // --- 4. Run handshake: B accepts, A connects and initiates ---
    // Clone what we need for the spawned task
    let signer_b_clone = signer_b.clone();

    let acceptor = tokio::spawn(async move {
        // B: accept incoming connection
        let conn_b = transport_b.accept().await
            .expect("test: Node B accept connection");

        // B: accept the stream opened by A, then run accept_handshake
        let mut stream = conn_b.accept_stream().await
            .expect("test: Node B accept stream");

        stoq::protocol::bilateral::accept_handshake(
            &mut stream,
            signer_b_clone.as_ref(),
            &proof_provider_b,
            coord_b,
        )
        .await
    });

    // A: connect to B
    let endpoint_b = stoq::transport::connection::Endpoint::new(
        std::net::Ipv6Addr::LOCALHOST,
        addr_b.port(),
    );
    let conn_a = transport_a.connect(&endpoint_b).await
        .expect("test: Node A connect to B");

    // A: open stream and run initiate_handshake
    let result_a = stoq::protocol::bilateral::initiate_handshake(
        &conn_a,
        signer_a.as_ref(),
        &proof_provider_a,
        coord_a,
    )
    .await
    .expect("test: Node A initiate_handshake");

    let result_b = acceptor.await
        .expect("test: acceptor task join")
        .expect("test: Node B accept_handshake");

    // --- 5. Verify handshake results ---

    // A sees B's identity
    assert_eq!(
        result_a.peer_node_id, node_id_b,
        "Node A should see Node B's node_id"
    );
    assert_eq!(
        result_a.peer_pubkey, pubkey_b,
        "Node A should see Node B's FALCON pubkey"
    );
    assert_eq!(
        result_a.peer_coordinate, coord_b,
        "Node A should see Node B's coordinate"
    );

    // B sees A's identity
    assert_eq!(
        result_b.peer_node_id, node_id_a,
        "Node B should see Node A's node_id"
    );
    assert_eq!(
        result_b.peer_pubkey, pubkey_a,
        "Node B should see Node A's FALCON pubkey"
    );
    assert_eq!(
        result_b.peer_coordinate, coord_a,
        "Node B should see Node A's coordinate"
    );

    // Both received non-empty state proofs
    assert!(
        !result_a.peer_proof.is_empty(),
        "Node A should receive B's state proof"
    );
    assert!(
        !result_b.peer_proof.is_empty(),
        "Node B should receive A's state proof"
    );

    // Verify identity binding: node_id == BLAKE3(pubkey)
    let expected_id_a = blake3::hash(&pubkey_a).to_hex().to_string();
    let expected_id_b = blake3::hash(&pubkey_b).to_hex().to_string();
    assert_eq!(result_b.peer_node_id, expected_id_a, "Identity binding check for A");
    assert_eq!(result_a.peer_node_id, expected_id_b, "Identity binding check for B");

    // Verify the received proofs are valid WireSignedProofs (FALCON-signed)
    // by running them through the provider's validate_proof
    let a_validates_b_proof = proof_provider_a
        .validate_proof(&result_a.peer_proof)
        .await
        .expect("test: A validates B's received proof");
    assert!(a_validates_b_proof, "A should validate B's state proof post-handshake");

    let b_validates_a_proof = BlockMatrixProofProvider::new(
        node_id_b.clone(),
        signer_b.clone(),
    )
    .validate_proof(&result_b.peer_proof)
    .await
    .expect("test: B validates A's received proof");
    assert!(b_validates_a_proof, "B should validate A's state proof post-handshake");

    // Rotation chains should be empty (fresh keys, no rotations)
    assert!(
        result_a.peer_rotation_chain.is_empty(),
        "Fresh identity should have empty rotation chain"
    );
    assert!(
        result_b.peer_rotation_chain.is_empty(),
        "Fresh identity should have empty rotation chain"
    );

    Ok(())
}

/// Verify that a tampered identity (wrong node_id) is rejected during handshake.
///
/// Creates a rogue signer whose node_id does not match BLAKE3(pubkey),
/// which should cause the peer to reject the handshake at the identity
/// binding step.
#[tokio::test]
async fn bilateral_handshake_rejects_forged_identity() {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_forged_identity_test(),
    )
    .await;

    match result {
        Ok(Ok(())) => {} // success
        Ok(Err(e)) => panic!("Test failed: {e:#}"),
        Err(_) => panic!("Test timed out after 10 seconds"),
    }
}

async fn run_forged_identity_test() -> Result<()> {
    // Legitimate identity for Node B
    let identity_b = FalconIdentity::generate();
    let signer_b: Arc<dyn NodeSigner + Send + Sync> = Arc::new(identity_b);

    // Rogue identity for Node A: real keys but forged node_id
    let rogue_identity = FalconIdentity::generate();
    let rogue_signer: Arc<dyn NodeSigner + Send + Sync> = Arc::new(RogueNodeSigner {
        real_identity: rogue_identity,
        fake_node_id: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
    });

    let proof_provider_a = BlockMatrixProofProvider::new(
        "rogue".to_string(),
        rogue_signer.clone(),
    );
    let proof_provider_b = BlockMatrixProofProvider::new(
        signer_b.node_id().to_string(),
        signer_b.clone(),
    );

    let transport_a = Arc::new(make_transport().await?);
    let transport_b = Arc::new(make_transport().await?);
    let addr_b = transport_b.local_addr()?;

    let signer_b_clone = signer_b.clone();
    let acceptor = tokio::spawn(async move {
        let conn = transport_b.accept().await
            .expect("test: accept");
        let mut stream = conn.accept_stream().await
            .expect("test: accept stream");
        stoq::protocol::bilateral::accept_handshake(
            &mut stream,
            signer_b_clone.as_ref(),
            &proof_provider_b,
            (0, 0, 0),
        )
        .await
    });

    let endpoint_b = stoq::transport::connection::Endpoint::new(
        std::net::Ipv6Addr::LOCALHOST,
        addr_b.port(),
    );
    let conn_a = transport_a.connect(&endpoint_b).await?;

    let initiate_result = stoq::protocol::bilateral::initiate_handshake(
        &conn_a,
        rogue_signer.as_ref(),
        &proof_provider_a,
        (0, 0, 0),
    )
    .await;

    let accept_result = acceptor.await.expect("test: join acceptor");

    // At least one side must fail: either the acceptor rejects A's forged
    // identity, or the initiator fails because B rejects it and never
    // sends Msg2 (causing a read error on A's side).
    let both_ok = initiate_result.is_ok() && accept_result.is_ok();
    assert!(
        !both_ok,
        "Handshake should fail when node_id does not match BLAKE3(pubkey)"
    );

    Ok(())
}

/// F2 regression: reject a proof whose FALCON signer key does NOT match the
/// peer's authenticated handshake identity.
///
/// The attack: Node A presents an HONEST identity (node_id == BLAKE3(pubkey),
/// challenge signed with its real key — so it passes `verify_identity_binding`
/// AND `verify_challenge_response`), but its STATE PROOF is a `WireSignedProof`
/// signed by a DIFFERENT (throwaway) key, carrying that key as `signer_pubkey`.
/// Before F2, `validate_proof` verified the proof signature against the key
/// carried inside the proof and passed — letting any peer join as trusted at
/// zero cost (unlimited Sybils). After F2, the handshake decodes the proof's
/// `signer_pubkey` and rejects it when it differs from the authenticated
/// handshake key.
///
/// We construct this by giving A's `BlockMatrixProofProvider` a signer that is
/// a DIFFERENT identity than the one A uses for the handshake itself.
#[tokio::test]
async fn bilateral_handshake_rejects_mismatched_proof_signer() {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_mismatched_signer_test(),
    )
    .await;

    match result {
        Ok(Ok(())) => {} // success
        Ok(Err(e)) => panic!("Test failed: {e:#}"),
        Err(_) => panic!("Test timed out after 10 seconds"),
    }
}

async fn run_mismatched_signer_test() -> Result<()> {
    // Node A's HANDSHAKE identity (honest: node_id == BLAKE3(pubkey))
    let identity_a = FalconIdentity::generate();
    let node_id_a = identity_a.node_id.clone();
    let signer_a: Arc<dyn NodeSigner + Send + Sync> = Arc::new(identity_a);

    // A DIFFERENT identity used ONLY to sign A's state proof — the mismatch.
    let proof_key_x = FalconIdentity::generate();
    let signer_x: Arc<dyn NodeSigner + Send + Sync> = Arc::new(proof_key_x);

    // A's proof provider signs with signer_x, so the WireSignedProof carries
    // signer_x's pubkey — which does NOT match A's handshake key (signer_a).
    let proof_provider_a = BlockMatrixProofProvider::new(
        node_id_a.clone(),
        signer_x.clone(),
    );

    // Node B is fully honest.
    let identity_b = FalconIdentity::generate();
    let signer_b: Arc<dyn NodeSigner + Send + Sync> = Arc::new(identity_b);
    let proof_provider_b = BlockMatrixProofProvider::new(
        signer_b.node_id().to_string(),
        signer_b.clone(),
    );

    let transport_a = Arc::new(make_transport().await?);
    let transport_b = Arc::new(make_transport().await?);
    let addr_b = transport_b.local_addr()?;

    let coord: (i64, i64, i64) = (0, 0, 0);

    let signer_b_clone = signer_b.clone();
    let acceptor = tokio::spawn(async move {
        let conn = transport_b.accept().await
            .expect("test: accept");
        let mut stream = conn.accept_stream().await
            .expect("test: accept stream");
        stoq::protocol::bilateral::accept_handshake(
            &mut stream,
            signer_b_clone.as_ref(),
            &proof_provider_b,
            coord,
        )
        .await
    });

    let endpoint_b = stoq::transport::connection::Endpoint::new(
        std::net::Ipv6Addr::LOCALHOST,
        addr_b.port(),
    );
    let conn_a = transport_a.connect(&endpoint_b).await?;

    // A initiates with its honest handshake signer but a mismatched proof.
    let initiate_result = stoq::protocol::bilateral::initiate_handshake(
        &conn_a,
        signer_a.as_ref(),
        &proof_provider_a,
        coord,
    )
    .await;

    let accept_result = acceptor.await.expect("test: join acceptor");

    // B (the responder) MUST reject: A's Msg3 proof is signed by signer_x, but
    // A's authenticated identity is signer_a → signer mismatch → rejected.
    assert!(
        accept_result.is_err(),
        "Responder must reject a proof whose signer != authenticated peer identity"
    );

    // The whole handshake must not succeed on both sides.
    let both_ok = initiate_result.is_ok() && accept_result.is_ok();
    assert!(
        !both_ok,
        "Handshake must fail when the proof signer does not match peer identity"
    );

    Ok(())
}

/// A NodeSigner that lies about its node_id (BLAKE3(pubkey) != declared id).
struct RogueNodeSigner {
    real_identity: FalconIdentity,
    fake_node_id: String,
}

impl NodeSigner for RogueNodeSigner {
    fn node_id(&self) -> &str {
        &self.fake_node_id
    }

    fn public_key_bytes(&self) -> &[u8] {
        &self.real_identity.public_key
    }

    fn sign(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.real_identity.sign(data)
    }

    fn verify_signature(pubkey: &[u8], data: &[u8], signature: &[u8]) -> anyhow::Result<bool>
    where
        Self: Sized,
    {
        FalconIdentity::verify_signature(pubkey, data, signature)
    }
}
