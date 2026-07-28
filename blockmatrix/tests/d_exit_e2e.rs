// Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
//
// Track D EXIT — the capstone E2E proving `node ≡ asset ≡ index` end to end.
//
// One flow drives the REAL production surfaces of D1–D5 (no mocks):
//
//   D1  — A authors an asset chain: two `register_asset_record` calls through
//         the real `add_block` chokepoint, so `stamp_asset_lineage` chains them
//         (AssetLineage length 2, asset_seq 0→1) and H3 FALCON-signs each entry.
//   D2/ — B receives it as an ASSET, not a spine graft: A's lineage is presented
//   D3    as a `PresentedAssetChain`, framed by the D3 wire codec
//         (`encode_presented_asset_chain` → `decode_presented_asset_chain`) and
//         fed straight into `accept_asset_chain` — byte-identical to the network
//         handler body (`handle_asset_chain` = decode-then-accept). B adopts it,
//         its `received_asset_lineage` matches A's history, and B's SPINE does
//         not move (height/head/blocks/index unchanged — the type invariant).
//   D4  — B validates under the per-network resolver: every authored entry passes
//         `ValidationService::validate` through the `NetworkRuleSet` resolver for
//         its `network_scope`, a structurally-broken proof is rejected by the
//         SAME service, and a signature-tampered chain is rejected through the
//         SAME decode→accept wire path.
//   D5  — A's identity + state survive the datadir re-key: the chain is persisted
//         under the legacy `node_{x}_{y}_{z}` key, the real `adopt_legacy_identity`
//         + `adopt_legacy_state_dir` migration re-keys it to the identity layout,
//         and A's identity AND the asset chain it authored in D1 survive
//         byte-for-byte under the new key.
//
// Wire path note: this drives the in-process `encode → decode → accept` sequence
// rather than a live two-node STOQ handshake. That sequence IS the network
// handler body (`asset_chain_handlers::handle_asset_chain` decodes the framed
// bytes then calls `accept_asset_chain` with no second check), so the D3 wire
// codec is IN the loop, not bypassed — only the QUIC stream carriage is elided,
// which the FALCON-envelope friction in the two-node harnesses makes flaky.

use std::sync::Arc;

use blockmatrix::assets::core::{AssetRegistration, AssetType, NetworkScope, NodeFingerprint};
use blockmatrix::blockchain::lineage::AssetLineage;
use blockmatrix::blockchain::{AcceptReject, NodeBlockchain, PresentedAssetChain};
use blockmatrix::bootstrap::{
    adopt_legacy_identity, adopt_legacy_state_dir, identity_dir, node_id, state_dir_key,
};
use blockmatrix::identity::FalconIdentity;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::asset_chain_wire::{
    decode_presented_asset_chain, encode_presented_asset_chain,
};
use blockmatrix::persistence::{BlockQuery, PersistenceConfig, PersistenceManager};
use blockmatrix::proof_of_state::{StateProofValidationService, ValidationService};
use hypermesh_lib::NodeSigner;
use trustchain::proof_of_state::StateProof;

fn coord(x: i64, y: i64, z: i64) -> MatrixCoordinate {
    MatrixCoordinate::new(x, y, z).expect("test: valid coordinate")
}

/// Everything about a node spine that receiving an asset must leave untouched.
#[derive(Debug, PartialEq)]
struct Spine {
    height: u64,
    head_hash: String,
    total_blocks: u64,
    indexed_assets: usize,
}

async fn spine(chain: &NodeBlockchain) -> Spine {
    Spine {
        height: chain.get_height().await,
        head_hash: chain.get_head().await.expect("test: head").hash,
        total_blocks: chain.get_stats().await.total_blocks,
        indexed_assets: chain.indexed_asset_count().await,
    }
}

/// A real persistence manager rooted at `data_dir`, keyed by `key`, with the
/// background saver off so the test's writes are deterministic.
async fn persistence(data_dir: &std::path::Path, key: &str) -> Arc<PersistenceManager> {
    let config = PersistenceConfig {
        storage_dir: data_dir.to_path_buf(),
        enable_background: false,
        ..PersistenceConfig::default()
    };
    Arc::new(
        PersistenceManager::new(config, key.to_string())
            .await
            .expect("test: persistence manager"),
    )
}

/// A structurally-valid four-proof set authored by `node_id` (WHO = the signer).
fn author_proof(node_id: &str) -> StateProof {
    let mut proof = StateProof::new_for_testing();
    proof.stake_proof.stake_holder_id = node_id.to_string();
    proof
}

/// D1 — A authors a real, persisted, FALCON-signed asset chain of two entries.
///
/// Returns the on-disk legacy key it was persisted under, A's device id, the
/// asset hash, and A's lineage as read back off the spine.
async fn d1_author(
    data_dir: &std::path::Path,
    coord_a: MatrixCoordinate,
) -> (String, String, [u8; 32], AssetLineage) {
    let legacy_key = node_id(&coord_a);

    // A's identity lives in-tree under the legacy key (the pre-migration layout).
    let legacy_identity = data_dir.join(&legacy_key).join("identity");
    let identity = FalconIdentity::load_or_create(&legacy_identity).expect("test: identity");
    let a_node_id = identity.node_id().to_string();

    // Real persistence + signer wired into the chain, exactly as fresh_boot does.
    let store = persistence(data_dir, &legacy_key).await;
    let signer: Arc<dyn NodeSigner + Send + Sync> = Arc::new(identity);
    let chain = NodeBlockchain::new(coord_a)
        .with_signer(signer)
        .with_persistence(store.clone());
    // Genesis was minted before the sink was attached — persist it through.
    let genesis = chain.get_block(0).await.expect("test: genesis");
    store.save_block(&genesis).await.expect("test: persist genesis");

    // A distinct, Private-scoped asset (not the blockchain-genesis asset), so D4
    // exercises the resolver's non-Global inherit branch. One registration reused
    // so both entries share the asset hash and genuinely extend one lineage.
    let mut registration = AssetRegistration::new(AssetType::Storage);
    registration.network_scope = NetworkScope::Private(NodeFingerprint([0x0D; 32]));
    let asset = registration.content_hash;
    let proof = author_proof(&a_node_id);

    chain
        .register_asset_record(registration.clone(), &proof)
        .await
        .expect("test: author asset-genesis (seq 0)");
    chain
        .register_asset_record(registration, &proof)
        .await
        .expect("test: extend asset lineage (seq 1)");

    store.flush().await.expect("test: flush to disk");
    let lineage = chain.asset_lineage(&asset).await;
    (legacy_key, a_node_id, asset, lineage)
}

/// D5 — reload A's chain from the migrated, identity-keyed layout.
async fn reload_chain(
    data_dir: &std::path::Path,
    nid: &str,
    coord_a: MatrixCoordinate,
) -> NodeBlockchain {
    let store = persistence(data_dir, nid).await;
    let mut blocks = Vec::new();
    let mut idx = 0u64;
    while let Some(block) = store
        .load_block(BlockQuery::ByIndex(idx))
        .await
        .expect("test: load block")
    {
        blocks.push(block);
        idx += 1;
    }
    assert!(blocks.len() >= 2, "test: migrated chain must carry genesis + authored blocks");
    NodeBlockchain::from_blocks(coord_a, blocks).expect("test: reconstruct from migrated blocks")
}

#[tokio::test]
async fn d_exit_capstone_node_is_asset_is_index_end_to_end() {
    let tmp = tempfile::TempDir::new().expect("test: tempdir");
    let data_dir = tmp.path();
    let coord_a = coord(1, 1, 1);

    // ── D1: an asset is its own chain ──────────────────────────────────────
    let (legacy_key, a_node_id, asset, lineage_a) = d1_author(data_dir, coord_a).await;

    assert_eq!(lineage_a.entries.len(), 2, "D1: lineage must have been extended");
    assert_eq!(lineage_a.sequence(), vec![0, 1], "D1: asset_seq must progress 0→1");
    assert_eq!(lineage_a.verify(), Ok(()), "D1: the authored lineage must verify");
    assert!(
        lineage_a.entries.iter().all(|e| e.signed_proof.is_some()),
        "D1: every authored entry must carry a FALCON envelope"
    );

    // ── D2/D3: a foreign chain is 'received an asset', not a spine graft ────
    let chain_b = NodeBlockchain::new(coord(2, 2, 2));
    let before_b = spine(&chain_b).await;

    let presented = PresentedAssetChain::new(asset, lineage_a.entries.clone());
    let framed = encode_presented_asset_chain(&presented).expect("test: D3 encode");
    let decoded = decode_presented_asset_chain(&framed).expect("test: D3 decode");
    let receipt = chain_b
        .accept_asset_chain(decoded)
        .await
        .expect("test: D2 accept a valid presented chain");

    assert_eq!(receipt.entries, 2);
    assert_eq!(receipt.added, 2);
    let received_b = chain_b
        .received_asset_lineage(&asset)
        .await
        .expect("test: B adopted the asset off-spine");
    assert_eq!(
        received_b.entries, lineage_a.entries,
        "D2: B's received lineage must match A's history exactly"
    );
    assert_eq!(
        spine(&chain_b).await,
        before_b,
        "D2 INVARIANT: receiving an asset chain must not move B's spine"
    );
    assert!(!chain_b.has_ever_seen_asset(&asset).await, "D2: the asset is off-spine on B");

    // ── D4: validated per-network through the resolver ─────────────────────
    let service = ValidationService::new();
    for entry in &lineage_a.entries {
        assert!(
            service
                .validate(&entry.state_proof, &entry.registration.network_scope)
                .is_ok(),
            "D4: an authored entry must validate against its network's rules"
        );
    }
    // The resolver IS the selection seam: a Private scope with no published
    // ruleset inherits the anchor (byte-identical to the pre-D4 single bar).
    let scope = &lineage_a.entries[0].registration.network_scope;
    assert!(matches!(scope, NetworkScope::Private(_)));
    assert_eq!(
        service.rules().resolve(scope),
        service.rules().default_requirements(),
        "D4: an unpublished network inherits the anchor through the resolver"
    );
    // A structurally-broken proof is rejected by the SAME resolver-backed service.
    let mut broken = lineage_a.entries[0].state_proof.clone();
    broken.stake_proof.stake_holder_id.clear();
    assert!(
        service.validate(&broken, scope).is_err(),
        "D4: a proof with no WHO must be rejected through the resolver path"
    );
    // A signature-tampered chain is rejected through the SAME decode→accept wire
    // path — never by a check restated at the wire.
    let chain_forge = NodeBlockchain::new(coord(3, 3, 3));
    let mut forged = presented.clone();
    if let Some(wire) = forged.entries[1].signed_proof.as_mut() {
        wire.nonce[0] ^= 0xFF;
    }
    let framed_forge = encode_presented_asset_chain(&forged).expect("test: encode forged");
    let decoded_forge = decode_presented_asset_chain(&framed_forge).expect("test: decode forged");
    assert!(
        matches!(
            chain_forge.accept_asset_chain(decoded_forge).await,
            Err(AcceptReject::BadSignature { position: 1, .. })
        ),
        "D4: a tampered chain must be refused by the one accept gate"
    );
    assert!(!chain_forge.has_received_asset_chain(&asset).await);

    // ── D5: A's identity + state survive the datadir re-key ────────────────
    let id_dir = adopt_legacy_identity(data_dir, &legacy_key).expect("test: D5 adopt identity");
    assert_eq!(id_dir, identity_dir(data_dir));
    let migrated_id = FalconIdentity::load_or_create(&id_dir).expect("test: reload identity");
    assert_eq!(
        migrated_id.node_id, a_node_id,
        "D5: A's identity must survive the migration unchanged"
    );

    let nid = state_dir_key(&a_node_id);
    adopt_legacy_state_dir(data_dir, &legacy_key, &nid).expect("test: D5 adopt state dir");
    assert!(
        !data_dir.join(&legacy_key).exists(),
        "D5: the legacy coordinate-keyed dir is fully consumed"
    );

    let reloaded = reload_chain(data_dir, &nid, coord_a).await;
    let lineage_after = reloaded.asset_lineage(&asset).await;
    assert_eq!(
        lineage_after.entries, lineage_a.entries,
        "D5: the asset chain A authored in D1 must survive the re-key byte-for-byte"
    );
    assert_eq!(lineage_after.verify(), Ok(()), "D5: the survived lineage still verifies");
}
