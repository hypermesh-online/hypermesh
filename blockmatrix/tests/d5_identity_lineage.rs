// Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
//
// D5 Part 2 — a node's identity is a first-class asset chain. The genesis
// identity asset and every subsequent key rotation share ONE `asset_hash` and
// therefore walk as a single, unbroken `AssetLineage`: the rotation is a
// SUCCESSOR of the identity (prev = identity's `lineage_id`, `asset_seq` +1),
// not a re-rooted fresh asset.

use blockmatrix::assets::core::{
    AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope, NodeFingerprint,
};
use blockmatrix::blockchain::NodeBlockchain;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use trustchain::identity::{FalconIdentity, KeyRotationReason};
use trustchain::proof_of_state::StateProof;

fn coord() -> MatrixCoordinate {
    MatrixCoordinate::new(2, 4, 6).expect("test: valid coordinate")
}

#[tokio::test]
async fn d5_identity_and_key_rotation_walk_as_one_lineage() {
    let chain = NodeBlockchain::new(coord());
    let node = FalconIdentity::generate();

    // The node's identity, scoped to the node itself.
    let fingerprint =
        NetworkScope::Private(NodeFingerprint::from(hypermesh_lib::NodeId::from_public_key(
            &node.public_key,
        )));
    let identity_registration = AssetRegistration::from_asset_data(
        &AssetData {
            config: Vec::new(),
            definition: node.public_key.clone(),
            metadata: node.kyber_public_key.clone(),
        },
        fingerprint.clone(),
        AssetCategory::BaseSystem(BaseSystemType::Identity),
    );
    let identity_asset_hash = identity_registration.content_hash;

    chain
        .register_asset_record(identity_registration, &StateProof::new_for_testing())
        .await
        .expect("test: register identity asset");

    // A key rotation extends the SAME asset.
    let (rotation, _new_identity) = node
        .rotate_keys(1, KeyRotationReason::Scheduled)
        .expect("test: rotate keys");
    chain
        .add_key_rotation_block(
            &rotation,
            &StateProof::new_for_testing(),
            identity_asset_hash,
            fingerprint.clone(),
        )
        .await
        .expect("test: add key rotation block");

    // ── The identity chain walks as ONE unbroken lineage. ──
    let lineage = chain.asset_lineage(&identity_asset_hash).await;
    assert_eq!(lineage.len(), 2, "identity + one rotation = two entries");
    lineage
        .verify()
        .expect("identity + rotation must form one unbroken lineage");
    assert_eq!(lineage.sequence(), vec![0, 1], "rotation succeeds identity");

    let root = lineage.root().expect("test: root");
    let head = lineage.head().expect("test: head");
    assert!(root.is_asset_genesis(), "identity is the asset genesis");
    assert_eq!(
        head.prev_asset_entry(),
        Some(root.lineage_id().as_str()),
        "the rotation names the identity as its predecessor"
    );

    // The rotation entry is a Private, KeyRotation-categorised successor.
    assert_eq!(
        head.registration.category,
        AssetCategory::BaseSystem(BaseSystemType::KeyRotation)
    );
    assert_eq!(head.registration.network_scope, fingerprint);
    // And it still addresses the identity asset (same lineage), not a new one.
    assert_eq!(head.asset_hash, identity_asset_hash);
}
