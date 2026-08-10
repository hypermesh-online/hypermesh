// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! End-to-end network sync test: two nodes exchange blocks over STOQ.
//!
//! Proves that:
//! 1. A block can be serialized into the wire format (TAG_BLOCK_ANNOUNCE)
//! 2. Sent over a real QUIC connection between two STOQ transports
//! 3. Deserialized and BLAKE3-verified on the receiving side
//! 4. Inserted into the receiver's blockchain via `insert_received_block`

use std::sync::Arc;

use anyhow::Result;
use stoq::{StoqTransport, TransportConfig};
use trustchain::identity::FalconIdentity;

use blockmatrix::blockchain::block::{Block, BlockAssetEntry, StoragePointer};
use blockmatrix::blockchain::NodeBlockchain;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::proof_of_state::BlockMatrixProofProvider;
use hypermesh_lib::NodeSigner;
use trustchain::proof_of_state::{StateProof, StateRequirements};

/// Wire-protocol tag byte for block announcements (matches stoq_transport.rs).
const TAG_BLOCK_ANNOUNCE: u8 = 0x03;

/// Connection-type discriminator for peer messages (matches network/mod.rs).
const CONN_TYPE_PEER_MESSAGE: u8 = 0x01;

/// Create a STOQ transport on an OS-assigned port.
async fn make_transport() -> Result<StoqTransport> {
    let config = TransportConfig {
        port: 0,
        ..TransportConfig::default()
    };
    StoqTransport::new(config).await
}

/// Build a test BlockAssetEntry whose proof is bound to its asset_hash AND
/// claims `author` as its state-proof author.
///
/// The proof is bound via `BlockAssetEntry::new_bound` so the entry satisfies
/// the signed-to-content half of the mirror invariant (P1). The `stake_holder_id`
/// is set to `author.node_id()` (`BLAKE3(pubkey)`) so that when the entry is
/// FALCON-signed by that same identity — done inside `add_block` on a chain
/// carrying `author` as its signer — the accept-path author binding
/// (`signer_binds_to_author`) holds and the block passes the H3 gate on the
/// receive path WITHOUT the legacy accept-unsigned flag.
fn test_asset_entry(label: &str, author: &FalconIdentity) -> BlockAssetEntry {
    let data = format!("test-asset-{label}");
    let asset_hash = *blake3::hash(data.as_bytes()).as_bytes();

    let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coord");
    let registration = blockmatrix::assets::core::AssetRegistration::genesis(coord);

    let mut proof = StateProof::new_for_testing();
    proof.stake_proof.stake_holder_id = author.node_id().to_string();

    BlockAssetEntry::new_bound(
        asset_hash,
        &proof,
        StoragePointer::Local {
            path: format!("/test/{label}"),
        },
        registration,
    )
}

/// Build the wire payload for a block announcement.
///
/// Layout (mirrors `StoqBlockTransportAdapter::build_wire_payload`):
/// - [0]      TAG_BLOCK_ANNOUNCE (0x03)
/// - [1..9]   block_json_len: u64 LE
/// - [9..9+N] block JSON bytes
/// - [9+N..17+N]  proof_hash_len: u64 LE
/// - [17+N..17+N+P] proof_hash bytes
fn build_wire_payload(block: &Block) -> Vec<u8> {
    let block_json = serde_json::to_vec(block).expect("test: serialize block");
    let block_json_len = block_json.len() as u64;

    let first_proof_hash: Option<[u8; 32]> =
        block.entries.first().map(|e| e.proof_hash);
    let (proof_hash_len, proof_hash_bytes): (u64, Vec<u8>) = match first_proof_hash {
        Some(hash) => (32u64, hash.to_vec()),
        None => (0u64, Vec::new()),
    };

    let total = 1 + 8 + block_json.len() + 8 + proof_hash_bytes.len();
    let mut buf = Vec::with_capacity(total);

    buf.push(TAG_BLOCK_ANNOUNCE);
    buf.extend_from_slice(&block_json_len.to_le_bytes());
    buf.extend_from_slice(&block_json);
    buf.extend_from_slice(&proof_hash_len.to_le_bytes());
    buf.extend_from_slice(&proof_hash_bytes);

    buf
}

/// Parse a block from the wire payload and verify its BLAKE3 hash.
///
/// Mirrors `message_handlers::parse_and_verify_block`.
fn parse_block_from_wire(data: &[u8]) -> Result<Block> {
    if data.len() < 9 {
        anyhow::bail!("payload too short: {} bytes", data.len());
    }

    let tag = data[0];
    if tag != TAG_BLOCK_ANNOUNCE {
        anyhow::bail!("unexpected tag 0x{:02x}, expected 0x{:02x}", tag, TAG_BLOCK_ANNOUNCE);
    }

    let block_json_len = u64::from_le_bytes(
        data[1..9].try_into().expect("test: 8 bytes for len"),
    ) as usize;

    if data.len() < 9 + block_json_len {
        anyhow::bail!(
            "truncated: need {} bytes, have {}",
            9 + block_json_len,
            data.len(),
        );
    }

    let block: Block = serde_json::from_slice(&data[9..9 + block_json_len])?;

    if !block.verify_hash() {
        anyhow::bail!(
            "BLAKE3 hash mismatch: computed {}, stored {}",
            block.calculate_hash(),
            block.hash,
        );
    }

    Ok(block)
}

// ─── Test 1: Wire protocol — block travels over STOQ intact ─────────

#[tokio::test]
async fn block_sync_wire_protocol() {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        run_wire_protocol_test(),
    )
    .await;

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("Wire protocol test failed: {e:#}"),
        Err(_) => panic!("Wire protocol test timed out after 15 seconds"),
    }
}

async fn run_wire_protocol_test() -> Result<()> {
    // --- 1. Create two STOQ transports ---
    let transport_a = Arc::new(make_transport().await?);
    let transport_b = Arc::new(make_transport().await?);
    let addr_b = transport_b.local_addr()?;

    // --- 2. Create a block on Node A ---
    // A carries a FALCON signer so `add_block` attaches a signed_proof envelope
    // to every entry (the real produced-block shape a peer receives).
    let coord_a = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
    let identity_a = Arc::new(FalconIdentity::generate());
    let blockchain_a = NodeBlockchain::with_requirements(coord_a, StateRequirements::localhost_testing())
        .with_signer(identity_a.clone() as Arc<dyn NodeSigner + Send + Sync>);
    assert_eq!(blockchain_a.get_height().await, 0, "genesis only");

    let entry = test_asset_entry("wire-test", &identity_a);
    let block_a = blockchain_a
        .add_block(vec![entry])
        .await
        .expect("test: add block to A");
    assert_eq!(block_a.index, 1);
    assert!(block_a.verify_hash());

    // --- 3. Build wire payload ---
    let payload = build_wire_payload(&block_a);

    // --- 4. Node B accepts, Node A connects and sends ---
    let transport_b_clone = transport_b.clone();
    let receiver = tokio::spawn(async move {
        let conn = transport_b_clone.accept().await
            .expect("test: B accept connection");
        let mut stream = conn.accept_stream().await
            .expect("test: B accept stream");

        // Read the full payload (discriminator + block announce)
        let data = stream.receive().await
            .expect("test: B receive data");
        data.to_vec()
    });

    // A connects to B
    let endpoint_b = stoq::transport::connection::Endpoint::new(
        std::net::Ipv6Addr::LOCALHOST,
        addr_b.port(),
    );
    let conn_a = transport_a.connect(&endpoint_b).await?;

    // A opens stream, writes discriminator + payload
    let mut stream_a = conn_a.open_stream().await?;
    let mut full_payload = vec![CONN_TYPE_PEER_MESSAGE];
    full_payload.extend_from_slice(&payload);
    stream_a.send(&full_payload).await?;

    // --- 5. Receive and parse on B ---
    let received = receiver.await.expect("test: receiver join");

    // First byte is CONN_TYPE_PEER_MESSAGE discriminator
    assert_eq!(
        received[0], CONN_TYPE_PEER_MESSAGE,
        "first byte should be peer message discriminator"
    );

    // Parse the block from the remaining bytes
    let received_block = parse_block_from_wire(&received[1..])
        .expect("test: parse block from wire");

    // --- 6. Verify the block arrived intact ---
    assert_eq!(received_block.index, block_a.index);
    assert_eq!(received_block.hash, block_a.hash);
    assert_eq!(received_block.previous_hash, block_a.previous_hash);
    assert_eq!(received_block.entries.len(), block_a.entries.len());
    assert_eq!(
        received_block.entries[0].asset_hash,
        block_a.entries[0].asset_hash,
    );
    assert!(received_block.verify_hash(), "BLAKE3 hash must verify");

    Ok(())
}

// ─── Test 2: Full path — block inserted into receiver's blockchain ───

#[tokio::test]
async fn block_sync_full_insertion() {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        run_full_insertion_test(),
    )
    .await;

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("Full insertion test failed: {e:#}"),
        Err(_) => panic!("Full insertion test timed out after 15 seconds"),
    }
}

async fn run_full_insertion_test() -> Result<()> {
    // --- 1. Two transports, two blockchains (different genesis) ---
    let transport_a = Arc::new(make_transport().await?);
    let transport_b = Arc::new(make_transport().await?);
    let addr_b = transport_b.local_addr()?;

    let coord_a = MatrixCoordinate::new(5, 5, 5).expect("test: valid coord");
    let coord_b = MatrixCoordinate::new(10, 10, 10).expect("test: valid coord");

    // A carries a FALCON signer so its produced block-1 arrives at B with a
    // valid, author-bound signed_proof envelope (H3 gate on the receive path).
    let identity_a = Arc::new(FalconIdentity::generate());
    let blockchain_a = Arc::new(
        NodeBlockchain::with_requirements(coord_a, StateRequirements::localhost_testing())
            .with_signer(identity_a.clone() as Arc<dyn NodeSigner + Send + Sync>),
    );
    let blockchain_b = Arc::new(NodeBlockchain::with_requirements(coord_b, StateRequirements::localhost_testing()));

    // Both start at height 0 (genesis only)
    assert_eq!(blockchain_a.get_height().await, 0);
    assert_eq!(blockchain_b.get_height().await, 0);

    // Different genesis hashes (different coordinates)
    let genesis_a = blockchain_a.get_block(0).await.expect("test: A genesis");
    let genesis_b = blockchain_b.get_block(0).await.expect("test: B genesis");
    assert_ne!(
        genesis_a.hash, genesis_b.hash,
        "different coordinates should produce different genesis blocks"
    );

    // Zero-trust (P1/F7): a block only links into a chain that shares its
    // predecessor. To join A's network, B ADOPTS A's genesis first — a foreign
    // block-1 referencing a genesis B does not have would be HARD-REJECTED
    // (no cross-genesis graft). This mirrors the real join-a-network flow.
    // TODO(cluster-C): adopt_genesis / cross-genesis convergence is being retired for the per-asset-genesis model
    blockchain_b
        .adopt_genesis(genesis_a.clone())
        .await
        .expect("test: B adopts A's genesis to join A's chain");

    // --- 2. Node A creates a block with a test asset ---
    let entry = test_asset_entry("sync-test", &identity_a);
    let block_from_a = blockchain_a
        .add_block(vec![entry])
        .await
        .expect("test: add block to A");
    assert_eq!(blockchain_a.get_height().await, 1);

    // --- 3. Send block over STOQ ---
    let payload = build_wire_payload(&block_from_a);

    let transport_b_clone = transport_b.clone();
    let receiver = tokio::spawn(async move {
        let conn = transport_b_clone.accept().await
            .expect("test: B accept");
        let mut stream = conn.accept_stream().await
            .expect("test: B accept stream");
        stream.receive().await
            .expect("test: B receive")
            .to_vec()
    });

    let endpoint_b = stoq::transport::connection::Endpoint::new(
        std::net::Ipv6Addr::LOCALHOST,
        addr_b.port(),
    );
    let conn_a = transport_a.connect(&endpoint_b).await?;
    let mut stream_a = conn_a.open_stream().await?;

    let mut full_payload = vec![CONN_TYPE_PEER_MESSAGE];
    full_payload.extend_from_slice(&payload);
    stream_a.send(&full_payload).await?;

    let received = receiver.await.expect("test: receiver join");

    // --- 4. Parse and insert into B's blockchain ---
    let received_block = parse_block_from_wire(&received[1..])
        .expect("test: parse block");

    // B adopted A's genesis (step 1), so A's block-1 links legitimately —
    // verified predecessor, zero-trust insertion succeeds.
    blockchain_b
        .insert_received_block(received_block.clone())
        .await
        .expect("test: insert block into B");

    // --- 5. Verify B's chain state ---
    assert_eq!(
        blockchain_b.get_height().await,
        1,
        "B's chain should be at height 1 after receiving A's block"
    );

    assert!(
        blockchain_b.has_block(&block_from_a.hash).await,
        "B should have A's block by hash"
    );

    let stored = blockchain_b.get_block(1).await
        .expect("test: B should have block at index 1");
    assert_eq!(stored.hash, block_from_a.hash);
    assert_eq!(stored.entries.len(), 1);
    assert_eq!(stored.entries[0].asset_hash, block_from_a.entries[0].asset_hash);

    // A's chain should be unaffected
    assert_eq!(blockchain_a.get_height().await, 1);

    Ok(())
}

// ─── Test 3: Bilateral handshake + block sync (full E2E) ─────────────

#[tokio::test]
async fn handshake_then_block_sync() {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        run_handshake_then_sync(),
    )
    .await;

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("Handshake+sync test failed: {e:#}"),
        Err(_) => panic!("Handshake+sync test timed out after 20 seconds"),
    }
}

async fn run_handshake_then_sync() -> Result<()> {
    // --- 1. Create FALCON identities and proof providers ---
    // Keep A's identity as an Arc<FalconIdentity> so it serves BOTH as the
    // handshake signer AND as A's blockchain signer — the block A produces is
    // then FALCON-signed and author-bound for the H3 receive gate.
    let identity_a = Arc::new(FalconIdentity::generate());
    let identity_b = FalconIdentity::generate();

    let node_id_a = identity_a.node_id.clone();
    let node_id_b = identity_b.node_id.clone();

    let signer_a: Arc<dyn NodeSigner + Send + Sync> = identity_a.clone();
    let signer_b: Arc<dyn NodeSigner + Send + Sync> = Arc::new(identity_b);

    let proof_provider_a = BlockMatrixProofProvider::new(
        node_id_a.clone(),
        signer_a.clone(),
    );
    let proof_provider_b = BlockMatrixProofProvider::new(
        node_id_b.clone(),
        signer_b.clone(),
    );

    // --- 2. Transports and blockchains ---
    let transport_a = Arc::new(make_transport().await?);
    let transport_b = Arc::new(make_transport().await?);
    let addr_b = transport_b.local_addr()?;

    let coord_a = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
    let coord_b = MatrixCoordinate::new(4, 5, 6).expect("test: valid coord");

    let blockchain_a = Arc::new(
        NodeBlockchain::with_requirements(coord_a, StateRequirements::localhost_testing())
            .with_signer(signer_a.clone()),
    );
    let blockchain_b = Arc::new(NodeBlockchain::with_requirements(coord_b, StateRequirements::localhost_testing()));

    // Zero-trust (P1/F7): B joins A's chain by adopting A's genesis, so A's
    // block-1 links to a verified predecessor. A foreign block-1 would be
    // hard-rejected (no cross-genesis graft).
    // TODO(cluster-C): adopt_genesis / cross-genesis convergence is being retired for the per-asset-genesis model
    let genesis_a = blockchain_a.get_block(0).await.expect("test: A genesis");
    blockchain_b
        .adopt_genesis(genesis_a)
        .await
        .expect("test: B adopts A's genesis");

    // --- 3. Create a block on A ---
    let entry = test_asset_entry("handshake-sync", &identity_a);
    let block_from_a = blockchain_a
        .add_block(vec![entry])
        .await
        .expect("test: add block to A");

    // --- 4. Bilateral handshake: B accepts, A initiates ---
    let transport_b_hs = transport_b.clone();
    let signer_b_clone = signer_b.clone();
    let coord_b_tuple = (coord_b.x, coord_b.y, coord_b.z);

    let acceptor = tokio::spawn(async move {
        let conn = transport_b_hs.accept().await
            .expect("test: B accept for handshake");
        let mut stream = conn.accept_stream().await
            .expect("test: B accept stream");
        let result = stoq::protocol::bilateral::accept_handshake(
            &mut stream,
            signer_b_clone.as_ref(),
            &proof_provider_b,
            coord_b_tuple,
        )
        .await
        .expect("test: B accept handshake");
        (conn, result)
    });

    let endpoint_b = stoq::transport::connection::Endpoint::new(
        std::net::Ipv6Addr::LOCALHOST,
        addr_b.port(),
    );
    let conn_a = transport_a.connect(&endpoint_b).await?;
    let coord_a_tuple = (coord_a.x, coord_a.y, coord_a.z);

    let hs_result_a = stoq::protocol::bilateral::initiate_handshake(
        &conn_a,
        signer_a.as_ref(),
        &proof_provider_a,
        coord_a_tuple,
    )
    .await?;

    let (_conn_b, hs_result_b) = acceptor.await.expect("test: acceptor join");

    // Verify handshake succeeded
    assert_eq!(hs_result_a.peer_node_id, node_id_b);
    assert_eq!(hs_result_b.peer_node_id, node_id_a);

    // --- 5. Now send block over a NEW connection (post-handshake) ---
    // In production, blocks go over the same connection. Here we open
    // a fresh one to keep the test simple (the handshake consumed the
    // first connection's streams).

    let transport_b_block = transport_b.clone();
    let blockchain_b_clone = blockchain_b.clone();
    let block_hash_expected = block_from_a.hash.clone();

    let block_receiver = tokio::spawn(async move {
        let conn = transport_b_block.accept().await
            .expect("test: B accept block connection");
        let mut stream = conn.accept_stream().await
            .expect("test: B accept block stream");
        let data = stream.receive().await
            .expect("test: B receive block");
        let data = data.to_vec();

        // Parse (skip discriminator byte)
        let block = parse_block_from_wire(&data[1..])
            .expect("test: parse received block");

        // Insert into B's chain
        blockchain_b_clone
            .insert_received_block(block)
            .await
            .expect("test: insert into B");

        block_hash_expected
    });

    // A sends the block
    let conn_a2 = transport_a.connect(&endpoint_b).await?;
    let mut stream = conn_a2.open_stream().await?;
    let payload = build_wire_payload(&block_from_a);
    let mut full = vec![CONN_TYPE_PEER_MESSAGE];
    full.extend_from_slice(&payload);
    stream.send(&full).await?;

    let expected_hash = block_receiver.await.expect("test: block receiver join");

    // --- 6. Verify sync ---
    assert_eq!(blockchain_b.get_height().await, 1);
    assert!(blockchain_b.has_block(&expected_hash).await);

    let synced_block = blockchain_b.get_block(1).await
        .expect("test: B has block 1");
    assert_eq!(synced_block.hash, block_from_a.hash);
    assert_eq!(synced_block.entries[0].asset_hash, block_from_a.entries[0].asset_hash);

    Ok(())
}
