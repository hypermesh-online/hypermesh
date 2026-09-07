// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Tests for the Catalog VCS write side ([`super`]). Split out of
//! `version_write.rs` (via `#[path]`) so the implementation file stays under the
//! 500-line bar; `use super::*` resolves against the `version_write` module.

use super::*;
use crate::registry::catalog_registry::{RegistryConfig, TrustPolicy};
use crate::registry::AssetTypeDefinition;
use blockmatrix::blockchain::NodeBlockchain;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::proof_of_state::proof_of_state_integration::{
    SpaceProof, StakeProof, TimeProof, WorkProof,
};
use hypermesh_lib::{NodeSigner, PrivacyMode};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// A real (structurally valid) author proof: PoStake carries a bound
/// authorization identity (WHO), never a magnitude — the canonical model.
fn author_proof(who: &str) -> StateProof {
    let stake = StakeProof::new(who.to_string(), format!("{who}-id"));
    let space = SpaceProof::new(who.to_string(), format!("/{who}"), 1024);
    let work = WorkProof::from_work(who.to_string(), "register".to_string(), b"work");
    let time = TimeProof::new(Duration::from_secs(5));
    StateProof::new(stake, time, space, work)
}

/// A registry with a fresh in-mem `NodeBlockchain` attached and a relaxed
/// trust policy (proofs are constructed explicitly per version).
fn registry_with_chain() -> (CatalogRegistry, Arc<NodeBlockchain>) {
    let coord = MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate");
    let chain = Arc::new(NodeBlockchain::new(coord));
    let registry = CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    )
    .with_chain(chain.clone());
    (registry, chain)
}

/// Recover a branch entry's parent pointer from its `StoragePointer::Local`
/// payload (the recoverable, C1-hash-committed place the pointer rides).
fn branch_pointer(entry: &BlockAssetEntry) -> serde_json::Value {
    match &entry.storage_pointer {
        StoragePointer::Local { path } => {
            serde_json::from_str(path).expect("test: branch pointer JSON")
        }
        other => unreachable!("test: branch genesis expected Local pointer, got {other:?}"),
    }
}

fn type_def(name: &str, schema_id: &str, version: &str) -> AssetTypeDefinition {
    let schema = json!({ "type": "object", "id": schema_id });
    let mut td = AssetTypeDefinition::new(name.to_string(), schema, author_proof(name));
    td.metadata.version = version.to_string();
    td
}

/// v1 (genesis) → v2 (progression): the same name written twice lands two
/// entries on ONE asset-chain, in seq order, and the lineage verifies as an
/// unbroken chain — exactly what the Phase-2 read view reports.
#[tokio::test]
async fn test_register_genesis_then_progression() {
    let (registry, chain) = registry_with_chain();

    // v1 = genesis.
    registry
        .register_type(type_def("foo", "v1", "1.0.0"))
        .await
        .expect("test: register foo v1 (genesis)");

    let asset_hash = registry
        .asset_hash_for_name("foo")
        .await
        .expect("test: foo resolves to a genesis asset_hash");

    let after_v1 = registry.list_versions(&asset_hash).await;
    assert_eq!(after_v1.len(), 1, "genesis is the only version");
    assert_eq!(after_v1[0].seq, 0, "genesis is seq 0");

    // v2 = progression (single-slot relaxed: same name, new version).
    registry
        .register_type(type_def("foo", "v2", "2.0.0"))
        .await
        .expect("test: register foo v2 (progression)");

    let versions = registry.list_versions(&asset_hash).await;
    assert_eq!(versions.len(), 2, "genesis + one successor");
    assert_eq!(
        versions.iter().map(|v| v.seq).collect::<Vec<_>>(),
        vec![0, 1],
        "versions returned in seq order (v1 then v2)"
    );
    assert_eq!(
        versions[0].asset_hash, asset_hash,
        "both versions share the genesis asset_hash identity"
    );
    assert_eq!(versions[1].asset_hash, asset_hash);
    assert_ne!(
        versions[0].lineage_id, versions[1].lineage_id,
        "each version has a distinct per-version content id"
    );

    // The lineage the read view reports is the one that verifies unbroken.
    let lineage = chain.asset_lineage(&asset_hash).await;
    lineage
        .verify()
        .expect("test: linear asset chain verifies green");
    assert_eq!(lineage.sequence(), vec![0, 1]);
}

/// A branch mints a NEW independent asset-chain (its own genesis) whose
/// metadata names the parent version + parent asset. The branch verifies as
/// its own valid genesis and does NOT extend the parent chain.
#[tokio::test]
async fn test_branch_mints_independent_genesis_naming_parent() {
    let (registry, chain) = registry_with_chain();

    registry
        .register_type(type_def("foo", "v1", "1.0.0"))
        .await
        .expect("test: register foo v1");
    registry
        .register_type(type_def("foo", "v2", "2.0.0"))
        .await
        .expect("test: register foo v2");

    let parent_hash = registry
        .asset_hash_for_name("foo")
        .await
        .expect("test: foo resolves");
    let parent_head = registry
        .head_version(&parent_hash)
        .await
        .expect("test: parent head present");

    // Branch from the head (v2).
    let branch = registry
        .branch_version("foo", None, &author_proof("brancher"))
        .await
        .expect("test: branch foo from head");

    // The branch is a fresh, independent identity.
    assert_ne!(
        branch.asset_hash, parent_hash,
        "branch is a NEW asset-chain, not the parent"
    );
    assert_eq!(branch.seq, 0, "branch is its own genesis (seq 0)");

    // The branch genesis is a valid independent genesis.
    let branch_lineage = chain.asset_lineage(&branch.asset_hash).await;
    assert_eq!(branch_lineage.len(), 1, "branch chain has just its genesis");
    branch_lineage
        .verify()
        .expect("test: branch genesis verifies as a valid independent genesis");
    assert!(
        branch_lineage
            .root()
            .expect("test: branch root")
            .is_asset_genesis(),
        "branch root is a true asset genesis (prev=None, seq=0)"
    );

    // The branch genesis pointer NAMES the parent (recoverable, C1-committed).
    let root = branch_lineage.root().expect("test: branch root");
    let meta = branch_pointer(root);
    assert_eq!(
        meta["branch_parent"], parent_head.lineage_id,
        "branch_parent names the parent version's lineage_id"
    );
    assert_eq!(
        meta["branch_of_asset"],
        hex::encode(parent_hash),
        "branch_of_asset names the parent genesis asset_hash"
    );

    // The parent chain is untouched by the branch.
    assert_eq!(
        registry.list_versions(&parent_hash).await.len(),
        2,
        "branching did not append to the parent chain"
    );
}

/// Branch from a SPECIFIC earlier version (seq 0) — the pointer names that
/// version's lineage_id, not the head's.
#[tokio::test]
async fn test_branch_from_specific_version() {
    let (registry, chain) = registry_with_chain();
    registry
        .register_type(type_def("bar", "v1", "1.0.0"))
        .await
        .expect("test: bar v1");
    registry
        .register_type(type_def("bar", "v2", "2.0.0"))
        .await
        .expect("test: bar v2");

    let parent_hash = registry.asset_hash_for_name("bar").await.expect("test: bar");
    let versions = registry.list_versions(&parent_hash).await;
    let v1 = &versions[0];

    let branch = registry
        .branch_version("bar", Some(0), &author_proof("brancher"))
        .await
        .expect("test: branch bar from seq 0");

    let root = chain
        .asset_lineage(&branch.asset_hash)
        .await
        .root()
        .cloned()
        .expect("test: branch root");
    let meta = branch_pointer(&root);
    assert_eq!(
        meta["branch_parent"], v1.lineage_id,
        "branch_parent names v1's lineage_id (not the head)"
    );
}

/// Without a chain handle Catalog is unchanged: a repeat name still rejects,
/// and branching is refused.
#[tokio::test]
async fn test_standalone_unchanged_without_chain() {
    let registry = CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    );
    registry
        .register_type(type_def("solo", "v1", "1.0.0"))
        .await
        .expect("test: first registration succeeds");
    let repeat = registry.register_type(type_def("solo", "v2", "2.0.0")).await;
    assert!(repeat.is_err(), "standalone: repeat name still rejects");
    assert!(repeat
        .unwrap_err()
        .to_string()
        .contains("already registered"));

    let branch = registry
        .branch_version("solo", None, &author_proof("x"))
        .await;
    assert!(branch.is_err(), "branch requires a chain handle");
}

/// Branching an unknown name errors rather than minting an orphan chain.
#[tokio::test]
async fn test_branch_unknown_name_errors() {
    let (registry, _chain) = registry_with_chain();
    let branch = registry
        .branch_version("ghost", None, &author_proof("x"))
        .await;
    assert!(branch.is_err(), "cannot branch an unregistered name");
}

/// Phase-3 coverage close-out: a version written to a chain that carries a node
/// signer must carry a valid FALCON `WireSignedProof`. `add_block` is the single
/// local-write signing chokepoint (H3), so a catalog-written genesis and a
/// catalog-written successor BOTH pick up an envelope that `verify_signed_proof`
/// accepts and that binds to the signing author.
#[tokio::test]
async fn test_with_signer_version_carries_valid_falcon_proof() {
    use trustchain::identity::FalconIdentity;

    let coord = MatrixCoordinate::new(3, 3, 3).expect("test: valid coordinate");
    let signer = Arc::new(FalconIdentity::generate());
    let chain = Arc::new(NodeBlockchain::new(coord).with_signer(signer.clone()));
    let registry = CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    )
    .with_chain(chain.clone());

    // Genesis + a successor, both written through the signing chain.
    registry
        .register_type(type_def("signed", "v1", "1.0.0"))
        .await
        .expect("test: register signed v1 (genesis)");
    registry
        .register_type(type_def("signed", "v2", "2.0.0"))
        .await
        .expect("test: register signed v2 (progression)");

    let asset_hash = registry
        .asset_hash_for_name("signed")
        .await
        .expect("test: signed resolves");

    // EVERY entry on the asset chain carries a verifiable FALCON envelope.
    let lineage = chain.asset_lineage(&asset_hash).await;
    let expected_signer = signer.public_key_bytes().to_vec();
    assert_eq!(lineage.sequence(), vec![0, 1], "genesis + successor present");
    for entry in &lineage.entries {
        let pubkey = entry
            .verify_signed_proof()
            .expect("test: catalog-written version carries a valid FALCON WireSignedProof");
        assert_eq!(
            pubkey, expected_signer,
            "the envelope binds to the chain's signing author"
        );
    }
}
