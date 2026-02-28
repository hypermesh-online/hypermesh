// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! 10-Node IPv6 Asset Transfer Simulation
//!
//! Proves that Node 0 can transfer an asset to Node 9 with full PoS
//! authentication, accurate receipts, and verifiable blockchain records
//! on both source and target chains.

use std::collections::HashMap;
use std::f64::consts::TAU;
use std::sync::Arc;

use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::consensus::validation::DefaultStateAuthenticator;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::transfer::{
    create_transfer_intent, compute_receipt_hash, proof_to_bytes,
    StateProofBytes, TransferEngine, TransferError, TransferValidation,
};
use hypermesh_lib::{AssetAddress, ContentHash};
use trustchain::consensus::ConsensusProof;

/// A simulated node with its own blockchain and asset registry.
struct SimNode {
    #[allow(dead_code)]
    id: usize,
    coord: MatrixCoordinate,
    chain: NodeBlockchain,
    assets: HashMap<AssetAddress, String>, // address -> description
}

impl SimNode {
    fn new(id: usize, coord: MatrixCoordinate) -> Self {
        Self {
            id,
            coord,
            chain: NodeBlockchain::new(coord),
            assets: HashMap::new(),
        }
    }
}

/// Place 10 nodes on a 3D helix.
fn create_nodes() -> Vec<SimNode> {
    (0..10)
        .map(|i| {
            let angle = (i as f64) * TAU / 10.0;
            let x = (angle.cos() * 100.0) as i64;
            let y = (angle.sin() * 100.0) as i64;
            let z = i as i64 * 10;
            let coord = MatrixCoordinate::new(x, y, z).expect("test: valid helix coord");
            SimNode::new(i, coord)
        })
        .collect()
}

/// Helper: get valid test proof bytes from trustchain.
fn valid_proof_bytes() -> StateProofBytes {
    let proof = ConsensusProof::new_for_testing();
    proof_to_bytes(&proof).expect("test: proof serialization")
}

// ---------------------------------------------------------------------------
// Test 1: AssetAddress roundtrip
// ---------------------------------------------------------------------------
#[test]
fn test_asset_address_roundtrip() {
    let coord = MatrixCoordinate::new(42, -17, 99).expect("test: valid coord");
    let hash = ContentHash::from_bytes([0xDE; 32]);

    let addr = AssetAddress::new(coord.x, coord.y, coord.z, &hash)
        .expect("test: valid address");

    // to_ipv6 -> from_ipv6 roundtrip
    let ipv6 = addr.to_ipv6();
    let recovered = AssetAddress::from_ipv6(ipv6).expect("test: valid roundtrip");
    assert_eq!(addr, recovered);

    // matrix_coords extraction matches original
    let (x, y, z) = recovered.matrix_coords();
    assert_eq!((x, y, z), (42, -17, 99));

    // shard_index is 0 for whole-asset address
    assert_eq!(recovered.shard_index(), 0);

    // is_hypermesh
    assert!(recovered.is_hypermesh());

    // Display produces valid IPv6 string
    let display = format!("{}", addr);
    assert!(display.contains(':'), "Should be colon-hex: {}", display);
}

// ---------------------------------------------------------------------------
// Test 2: Shard sub-addressing
// ---------------------------------------------------------------------------
#[test]
fn test_shard_sub_addressing() {
    let hash = ContentHash::from_bytes([0xCA; 32]);
    let parent = AssetAddress::new(10, 20, 30, &hash).expect("test: valid address");
    assert_eq!(parent.shard_index(), 0);

    // Derive 14 shard sub-addresses (1-14, matching Reed-Solomon 10+4)
    let mut shard_addrs = Vec::new();
    for i in 1..=14u8 {
        let shard = parent.shard(i).expect("test: valid shard index");
        assert_eq!(shard.shard_index(), i);
        // parent() of shard should equal the original parent
        assert_eq!(shard.parent(), parent);
        shard_addrs.push(shard);
    }

    // All shard addresses are unique
    let unique_count = shard_addrs.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(unique_count, 14, "All 14 shard addresses should be unique");

    // Shard 15 is valid (max nibble)
    let shard15 = parent.shard(15).expect("test: shard 15 valid");
    assert_eq!(shard15.shard_index(), 15);

    // Shard 16 should fail
    assert!(parent.shard(16).is_err());
}

// ---------------------------------------------------------------------------
// Test 3: Transfer Node 0 -> Node 9 (main test)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_transfer_node0_to_node9() {
    let mut nodes = create_nodes();
    let validator = Arc::new(DefaultStateAuthenticator::for_testing());
    let engine = TransferEngine::new(validator);

    // Node 0 creates an asset
    let asset_data = b"HyperMesh simulation asset v1";
    let content_hash = ContentHash::from_bytes(*blake3::hash(asset_data).as_bytes());

    let source = &nodes[0];
    let asset_addr = AssetAddress::new(
        source.coord.x,
        source.coord.y,
        source.coord.z,
        &content_hash,
    )
    .expect("test: valid asset address");

    // Register asset on node 0
    nodes[0].assets.insert(asset_addr, "simulation-asset".to_string());

    let source_coord = nodes[0].coord;
    let target_coord = nodes[9].coord;

    // Create transfer intent with valid PoS proof bytes
    let intent = create_transfer_intent(
        asset_addr,
        source_coord,
        target_coord,
        valid_proof_bytes(),
        vec![],
    );

    // Execute transfer
    let receipt = engine
        .execute_transfer(
            &intent,
            &valid_proof_bytes(),
            &nodes[0].chain,
            &nodes[9].chain,
        )
        .await
        .expect("test: transfer should succeed");

    // Verify: source chain has transfer-out block
    assert_eq!(receipt.source_block_index, 1, "First block after genesis on source");
    let source_block = nodes[0].chain.get_block(1).await;
    assert!(source_block.is_some(), "Source chain should have block 1");

    // Verify: target chain has transfer-in block
    assert_eq!(receipt.target_block_index, 1, "First block after genesis on target");
    let target_block = nodes[9].chain.get_block(1).await;
    assert!(target_block.is_some(), "Target chain should have block 1");

    // Verify: new address has Node 9's matrix coordinates
    let (nx, ny, nz) = receipt.new_address.matrix_coords();
    assert_eq!(
        (nx, ny, nz),
        (target_coord.x, target_coord.y, target_coord.z),
        "New address should encode Node 9's coordinates"
    );

    // Verify: old address has Node 0's matrix coordinates
    let (ox, oy, oz) = receipt.old_address.matrix_coords();
    assert_eq!(
        (ox, oy, oz),
        (source_coord.x, source_coord.y, source_coord.z),
    );

    // Verify: receipt_hash is correct BLAKE3
    let expected_hash = compute_receipt_hash(
        &intent.transfer_id,
        &receipt.old_address,
        &receipt.new_address,
        receipt.source_block_index,
        receipt.target_block_index,
    );
    assert_eq!(receipt.receipt_hash, expected_hash);

    // Move asset in registry
    nodes[0].assets.remove(&asset_addr);
    nodes[9]
        .assets
        .insert(receipt.new_address, "simulation-asset".to_string());

    // Final state check
    assert!(nodes[0].assets.is_empty(), "Node 0 should have no assets");
    assert_eq!(nodes[9].assets.len(), 1, "Node 9 should have the asset");
}

// ---------------------------------------------------------------------------
// Test 4: Chain transfer through all 10 nodes sequentially
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_chain_transfer_0_through_9() {
    let nodes = create_nodes();
    let validator = Arc::new(DefaultStateAuthenticator::for_testing());
    let engine = TransferEngine::new(validator);

    let asset_data = b"chain-transfer-asset";
    let content_hash = ContentHash::from_bytes(*blake3::hash(asset_data).as_bytes());

    // Start at node 0
    let mut current_addr = AssetAddress::new(
        nodes[0].coord.x,
        nodes[0].coord.y,
        nodes[0].coord.z,
        &content_hash,
    )
    .expect("test: valid initial address");

    // Transfer through nodes 0->1->2->...->9
    for i in 0..9 {
        let source_coord = nodes[i].coord;
        let target_coord = nodes[i + 1].coord;

        let intent = create_transfer_intent(
            current_addr,
            source_coord,
            target_coord,
            valid_proof_bytes(),
            vec![],
        );

        let receipt = engine
            .execute_transfer(
                &intent,
                &valid_proof_bytes(),
                &nodes[i].chain,
                &nodes[i + 1].chain,
            )
            .await
            .unwrap_or_else(|e| panic!("test: transfer {}->{} failed: {}", i, i + 1, e));

        // Update current address to the new one
        current_addr = receipt.new_address;
    }

    // Final address should have Node 9's coordinates
    let (fx, fy, fz) = current_addr.matrix_coords();
    assert_eq!(
        (fx, fy, fz),
        (nodes[9].coord.x, nodes[9].coord.y, nodes[9].coord.z),
        "After 9 hops, asset should be at Node 9's position"
    );

    // Each node's chain should have grown (genesis + transfer blocks)
    // Node 0: genesis + 1 transfer-out
    assert_eq!(nodes[0].chain.get_height().await, 1);
    // Node 9: genesis + 1 transfer-in + 0 transfer-out (it's the final destination)
    // Actually node 9 receives from node 8, so 1 transfer-in block
    assert_eq!(nodes[9].chain.get_height().await, 1);
    // Middle nodes (e.g. node 5): genesis + 1 transfer-in + 1 transfer-out = height 2
    assert_eq!(nodes[5].chain.get_height().await, 2);
}

// ---------------------------------------------------------------------------
// Test 5: Invalid proof rejected
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_invalid_proof_rejected() {
    let nodes = create_nodes();
    let validator = Arc::new(DefaultStateAuthenticator::for_testing());
    let engine = TransferEngine::new(validator);

    let hash = ContentHash::from_bytes([0xFF; 32]);
    let addr = AssetAddress::new(
        nodes[0].coord.x,
        nodes[0].coord.y,
        nodes[0].coord.z,
        &hash,
    )
    .expect("test: valid address");

    // Empty proof bytes = always fails PoS authentication
    let empty_proof = StateProofBytes::new(vec![]);

    let intent = create_transfer_intent(
        addr,
        nodes[0].coord,
        nodes[1].coord,
        empty_proof,
        vec![],
    );

    let result = engine
        .execute_transfer(
            &intent,
            &valid_proof_bytes(),
            &nodes[0].chain,
            &nodes[1].chain,
        )
        .await;

    assert!(result.is_err(), "Empty proof should be rejected");
    match result {
        Err(TransferError::ValidationFailed(TransferValidation::InvalidSourceProof(_))) => {
            // Expected: source proof fails authentication
        }
        other => panic!(
            "Expected InvalidSourceProof, got: {:?}",
            other,
        ),
    }
}

// ---------------------------------------------------------------------------
// Test 6: Address preserves fingerprint across transfers
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_address_preserves_fingerprint() {
    let nodes = create_nodes();
    let validator = Arc::new(DefaultStateAuthenticator::for_testing());
    let engine = TransferEngine::new(validator);

    let content_hash = ContentHash::from_bytes([0x42; 32]);
    let original_addr = AssetAddress::new(
        nodes[0].coord.x,
        nodes[0].coord.y,
        nodes[0].coord.z,
        &content_hash,
    )
    .expect("test: valid address");

    let original_fingerprint = original_addr.asset_fingerprint();

    // Transfer node 0 -> node 5
    let intent = create_transfer_intent(
        original_addr,
        nodes[0].coord,
        nodes[5].coord,
        valid_proof_bytes(),
        vec![],
    );

    let receipt = engine
        .execute_transfer(
            &intent,
            &valid_proof_bytes(),
            &nodes[0].chain,
            &nodes[5].chain,
        )
        .await
        .expect("test: transfer should succeed");

    // The content fingerprint (bytes 10-15 with shard nibble masked) should match
    let new_fingerprint = receipt.new_address.asset_fingerprint();

    // Mask out the shard nibble (low 4 bits of byte 5) for comparison
    let mut orig_masked = original_fingerprint;
    let mut new_masked = new_fingerprint;
    orig_masked[5] &= 0xF0;
    new_masked[5] &= 0xF0;

    assert_eq!(
        orig_masked, new_masked,
        "Asset fingerprint (44 bits) should be preserved across transfers"
    );

    // But the coordinates should differ
    let (ox, oy, oz) = original_addr.matrix_coords();
    let (nx, ny, nz) = receipt.new_address.matrix_coords();
    assert_ne!(
        (ox, oy, oz),
        (nx, ny, nz),
        "Coordinates should change after transfer"
    );
}
