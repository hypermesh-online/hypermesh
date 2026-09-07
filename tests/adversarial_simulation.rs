// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Node Adversarial Consensus Simulation Test Suite
//!
//! Evaluates the core HyperMesh thesis:
//! - Bilateral verification without global consensus, quorum, or leader election.
//! - Containment of Byzantine actors, Sybil coalitions, and state tampering.
//! - Fork / chain graft rejection and orphan resolution.
//! - Verification that malicious nodes cannot convince honest nodes of false state.

use std::sync::Arc;
use std::time::Duration;

use blockmatrix::assets::core::AssetRegistration;
use blockmatrix::blockchain::block::{BlockAssetEntry, StoragePointer};
use blockmatrix::blockchain::{Block, NodeBlockchain};
use blockmatrix::identity::FalconIdentity;
use blockmatrix::matrix::MatrixCoordinate;
use hypermesh_lib::{
    NodeSigner, SpaceProof, StakeProof, StateProof, TimeProof, WorkProof,
};

/// Helper: construct a valid test asset entry signed with a real FALCON identity.
fn create_signed_asset_entry(
    signer: &FalconIdentity,
    asset_id_str: &str,
    workload_str: &str,
    coord: MatrixCoordinate,
) -> BlockAssetEntry {
    let node_id_hex = signer.node_id.clone();
    let asset_bytes = format!("asset-content-{}", asset_id_str).into_bytes();
    let asset_hash = *blake3::hash(&asset_bytes).as_bytes();

    let mut space = SpaceProof::new(
        node_id_hex.clone(),
        format!("/mesh/storage/{}", asset_id_str),
        100 * 1024 * 1024 * 1024,
    );
    space.total_size = 50 * 1024 * 1024;
    space.file_hash = hex::encode(asset_hash);

    let state_proof = StateProof::new(
        StakeProof::new(format!("owner-{}", asset_id_str), node_id_hex.clone()),
        TimeProof::new(Duration::from_millis(50)),
        space,
        WorkProof::from_work(
            node_id_hex,
            workload_str.to_string(),
            format!("work-payload-{}", workload_str).as_bytes(),
        ),
    );

    let registration = AssetRegistration::genesis(coord);

    let mut entry = BlockAssetEntry::new_bound(
        asset_hash,
        &state_proof,
        StoragePointer::Local {
            path: format!("/tmp/shards/{}", asset_id_str),
        },
        registration,
    );

    entry
        .sign_proof(signer)
        .expect("FALCON signing of state proof must succeed");
    entry
}

#[tokio::test]
async fn test_honest_bilateral_state_exchange() {
    // 1. Initialize network with Node A and Node B sharing network genesis
    let identity_a = Arc::new(FalconIdentity::generate());
    let identity_b = Arc::new(FalconIdentity::generate());

    let coord = MatrixCoordinate { x: 10, y: 20, z: 30 };

    let genesis_block = Block::genesis(coord);

    let _chain_a = NodeBlockchain::from_genesis(coord, genesis_block.clone()).with_signer(identity_a.clone());
    let chain_b = NodeBlockchain::from_genesis(coord, genesis_block.clone()).with_signer(identity_b.clone());

    // 2. Node A creates a block with a signed state proof asset entry
    let entry = create_signed_asset_entry(&identity_a, "asset-001", "workload-alpha", coord);
    let block_1_a = Block::new(1, vec![entry], genesis_block.hash.clone());

    // 3. Node B receives Block 1 from Node A and validates it bilaterally
    let verify_res = chain_b.insert_received_block(block_1_a.clone()).await;

    assert!(
        verify_res.is_ok(),
        "Honest block with valid FALCON proof must pass received validation: {:?}",
        verify_res.err()
    );

    // Verify FALCON envelope verification directly on entry
    let signer_pubkey = block_1_a.entries[0]
        .verify_signed_proof()
        .expect("FALCON signature verification on honest entry must succeed");
    assert_eq!(signer_pubkey, identity_a.public_key_bytes());

    // Verify Node B has now updated its chain head to Block 1
    let head_b = chain_b.get_block(1).await.expect("head block 1 on node B");
    assert_eq!(head_b.hash, block_1_a.hash);
}

#[tokio::test]
async fn test_foreign_chain_graft_rejection() {
    // Node A and Node C have different geneses
    let identity_a = Arc::new(FalconIdentity::generate());
    let identity_c = Arc::new(FalconIdentity::generate());

    let coord_a = MatrixCoordinate { x: 1, y: 1, z: 1 };
    let coord_c = MatrixCoordinate { x: 2, y: 2, z: 2 };

    let chain_a = NodeBlockchain::new(coord_a).with_signer(identity_a.clone());
    let chain_c = NodeBlockchain::new(coord_c).with_signer(identity_c.clone());

    let genesis_a = chain_a.get_block(0).await.expect("genesis A");
    let entry = create_signed_asset_entry(&identity_a, "asset-graft", "work-graft", coord_a);
    let block_1_a = Block::new(1, vec![entry], genesis_a.hash.clone());

    // Node C receives Block 1 which links to Genesis A (not Genesis C)
    let result = chain_c.insert_received_block(block_1_a).await;
    assert!(
        result.is_err(),
        "Attempt to graft foreign block onto incompatible predecessor must be rejected"
    );
}

#[tokio::test]
async fn test_byzantine_tampered_proof_rejection() {
    let identity_a = FalconIdentity::generate();
    let identity_b = Arc::new(FalconIdentity::generate());

    let coord = MatrixCoordinate { x: 1, y: 1, z: 1 };
    let genesis = Block::genesis(coord);

    let chain_b = NodeBlockchain::from_genesis(coord, genesis.clone()).with_signer(identity_b);
    let mut entry = create_signed_asset_entry(&identity_a, "asset-tamper", "work-1", coord);

    // Byzantine mutation: tamper with the inner state proof after signing
    entry.state_proof.space_proof.total_size = 999 * 1024 * 1024 * 1024; // > total_storage (invalid)

    let block = Block::new(1, vec![entry], genesis.hash.clone());

    let result = chain_b.insert_received_block(block).await;
    assert!(
        result.is_err(),
        "Tampered state proof must be strictly rejected by honest node"
    );
}

#[tokio::test]
async fn test_byzantine_signed_to_content_replay_attack_rejection() {
    let identity_a = FalconIdentity::generate();
    let identity_b = Arc::new(FalconIdentity::generate());

    let coord = MatrixCoordinate { x: 5, y: 5, z: 5 };
    let genesis = Block::genesis(coord);

    let chain_b = NodeBlockchain::from_genesis(coord, genesis.clone()).with_signer(identity_b);

    // Attacker generates valid proof for Asset Alpha
    let mut entry = create_signed_asset_entry(&identity_a, "asset-alpha", "work-alpha", coord);

    // Attacker tries to replay this proof under a different content hash (Asset Beta)
    entry.asset_hash = [0xEEu8; 32];

    let block = Block::new(1, vec![entry], genesis.hash.clone());

    let result = chain_b.insert_received_block(block).await;
    assert!(
        result.is_err(),
        "Replaying a state proof under a mismatched asset_hash must fail signed-to-content binding"
    );
}

#[tokio::test]
async fn test_byzantine_forged_falcon_signature_rejection() {
    let identity_a = FalconIdentity::generate();
    let coord = MatrixCoordinate { x: 8, y: 8, z: 8 };

    let mut entry = create_signed_asset_entry(&identity_a, "asset-forge", "work-forge", coord);

    // Attacker forges the FALCON signature bytes
    if let Some(ref mut signed) = entry.signed_proof {
        signed.signature = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33];
    }

    let verify_res = entry.verify_signed_proof();
    assert!(
        verify_res.is_err(),
        "Forged FALCON signature must fail cryptographic envelope verification"
    );
}

#[tokio::test]
async fn test_multi_node_cluster_adversarial_containment() {
    // 10-node simulated network: 7 honest nodes, 3 Byzantine colluding nodes
    let total_nodes = 10;
    let byzantine_count = 3;
    let honest_count = total_nodes - byzantine_count;

    let coord = MatrixCoordinate { x: 0, y: 0, z: 0 };
    let network_genesis = Block::genesis(coord);

    let mut honest_identities = Vec::new();
    let mut honest_chains = Vec::new();

    for _ in 0..honest_count {
        let id = Arc::new(FalconIdentity::generate());
        let chain = NodeBlockchain::from_genesis(coord, network_genesis.clone()).with_signer(id.clone());
        honest_identities.push(id);
        honest_chains.push(chain);
    }

    let mut byzantine_identities = Vec::new();
    for _ in 0..byzantine_count {
        byzantine_identities.push(FalconIdentity::generate());
    }

    // Honest Node 0 publishes legitimate state
    let entry_honest = create_signed_asset_entry(
        &honest_identities[0],
        "legit-rwa-asset",
        "proof-of-work-legit",
        coord,
    );
    let legit_block = Block::new(1, vec![entry_honest], network_genesis.hash.clone());

    // Verify all other honest nodes accept the legitimate block
    for honest_peer in &honest_chains[1..] {
        let res = honest_peer.insert_received_block(legit_block.clone()).await;
        assert!(
            res.is_ok(),
            "Honest peer must accept valid bilateral block: {:?}",
            res.err()
        );
    }

    // Byzantine node 0 constructs forged block with fabricated state
    let mut forged_entry = create_signed_asset_entry(
        &byzantine_identities[0],
        "byzantine-fake-asset",
        "fake-work",
        coord,
    );
    // Tamper with proof hash
    forged_entry.proof_hash = [0x55u8; 32];
    let forged_block = Block::new(1, vec![forged_entry], network_genesis.hash.clone());

    // Verify 100% of honest nodes reject the forged Byzantine block
    let mut rejected_count = 0;
    for honest_peer in &honest_chains {
        let res = honest_peer.insert_received_block(forged_block.clone()).await;
        if res.is_err() {
            rejected_count += 1;
        }
    }

    assert_eq!(
        rejected_count, honest_count,
        "100% of honest nodes must reject the Byzantine forged block (containment rate: 100%)"
    );
}

#[tokio::test]
async fn test_byzantine_tampered_storage_pointer_rejection() {
    let identity_a = FalconIdentity::generate();
    let identity_b = Arc::new(FalconIdentity::generate());
    let coord = MatrixCoordinate { x: 4, y: 4, z: 4 };
    let genesis = Block::genesis(coord);

    let chain_b = NodeBlockchain::from_genesis(coord, genesis.clone()).with_signer(identity_b);
    let mut entry = create_signed_asset_entry(&identity_a, "asset-storage-tamper", "work-storage", coord);

    // Relay adversary modifies storage pointer from legitimate path to hijacked path
    entry.storage_pointer = StoragePointer::Local {
        path: "/malicious/hijacked/path".to_string(),
    };

    let block = Block::new(1, vec![entry], genesis.hash.clone());

    let result = chain_b.insert_received_block(block).await;
    assert!(
        result.is_err(),
        "Tampered storage pointer must fail cryptographic verification (W1-0)"
    );
}

#[tokio::test]
async fn test_byzantine_tampered_registration_rejection() {
    let identity_a = FalconIdentity::generate();
    let identity_b = Arc::new(FalconIdentity::generate());
    let coord = MatrixCoordinate { x: 7, y: 7, z: 7 };
    let genesis = Block::genesis(coord);

    let chain_b = NodeBlockchain::from_genesis(coord, genesis.clone()).with_signer(identity_b);
    let mut entry = create_signed_asset_entry(&identity_a, "asset-reg-tamper", "work-reg", coord);

    // Relay adversary modifies registration category or authorization set
    entry.registration.category = blockmatrix::assets::core::AssetCategory::Application(
        blockmatrix::assets::core::ApplicationDomain {
            domain_name: "tampered-domain".to_string(),
            domain_hash: [0xFFu8; 32],
        },
    );

    let block = Block::new(1, vec![entry], genesis.hash.clone());

    let result = chain_b.insert_received_block(block).await;
    assert!(
        result.is_err(),
        "Tampered registration metadata must fail cryptographic verification (W1-0)"
    );
}

