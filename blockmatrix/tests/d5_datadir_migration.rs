// Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
//
// D5 Part 1 — the data dir is keyed by the DEVICE IDENTITY, not the matrix
// coordinate string. These proofs pin the migration contract:
//
//   ADOPT   — an existing install whose state lives under the legacy
//             `node_{x}_{y}_{z}` key is adopted onto the identity key WITHOUT
//             losing its chain, certificate, or keypair. The identity moves to
//             the coordinate-independent `data_dir/identity`; everything else
//             moves to `data_dir/{device_id}`.
//   FRESH   — a node with no legacy dir gets the identity-keyed layout directly;
//             the adopt steps fabricate nothing.
//   IDEMPOTENT — running the adopt twice, or against an already-migrated tree,
//             never clobbers live state.

use blockmatrix::bootstrap::{
    adopt_legacy_identity, adopt_legacy_state_dir, identity_dir, node_id, state_dir_key,
};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use std::fs;
use std::path::Path;

/// A fake 64-hex device id (`BLAKE3(falcon_pubkey)` shape). Its exact value is
/// irrelevant — only that it is not a `node_{x}_{y}_{z}` string.
const DEVICE_ID: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("test: has parent"))
        .expect("test: mkdir -p");
    fs::write(path, contents).expect("test: write file");
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("test: read file")
}

/// Seed a pre-migration install: identity + chain + cert under the legacy key.
fn seed_legacy(data_dir: &Path, legacy_key: &str) {
    let legacy = data_dir.join(legacy_key);
    write(&legacy.join("identity").join("falcon_public.der"), "IDENTITY-BYTES");
    write(&legacy.join("blockchain").join("metadata.json"), "{\"height\":7}");
    write(&legacy.join("blockchain").join("blocks.dat"), "GENESIS..HEAD");
    write(&legacy.join("certificate.json"), "CERT-BYTES");
    write(&legacy.join("shards").join("shard0"), "SHARD-BYTES");
}

#[test]
fn d5_boot_with_legacy_datadir_adopts_and_preserves_state() {
    let tmp = tempfile::TempDir::new().expect("test: tempdir");
    let data_dir = tmp.path();
    let legacy_key = node_id(&MatrixCoordinate::new(0, 0, 0).expect("test: coord"));
    assert_eq!(legacy_key, "node_0_0_0");

    seed_legacy(data_dir, &legacy_key);

    // Step 1: adopt the in-tree identity to the coordinate-independent location.
    let id_dir = adopt_legacy_identity(data_dir, &legacy_key).expect("test: adopt identity");
    assert_eq!(id_dir, identity_dir(data_dir));
    assert_eq!(read(&id_dir.join("falcon_public.der")), "IDENTITY-BYTES");
    // The legacy identity copy is gone (moved, not copied).
    assert!(!data_dir.join(&legacy_key).join("identity").exists());

    // Step 2: adopt the coordinate-keyed chain state onto the identity key.
    let nid = state_dir_key(DEVICE_ID);
    adopt_legacy_state_dir(data_dir, &legacy_key, &nid).expect("test: adopt state");

    let new_dir = data_dir.join(&nid);
    // Chain state survives byte-for-byte under the new key.
    assert_eq!(read(&new_dir.join("blockchain").join("blocks.dat")), "GENESIS..HEAD");
    assert_eq!(read(&new_dir.join("blockchain").join("metadata.json")), "{\"height\":7}");
    assert_eq!(read(&new_dir.join("certificate.json")), "CERT-BYTES");
    assert_eq!(read(&new_dir.join("shards").join("shard0")), "SHARD-BYTES");
    // The legacy coordinate dir is fully consumed.
    assert!(!data_dir.join(&legacy_key).exists());

    // The metadata gate main.rs uses to pick resume-vs-fresh now resolves under
    // the identity key — an existing install resumes.
    assert!(new_dir.join("blockchain").join("metadata.json").exists());
}

#[test]
fn d5_fresh_boot_uses_identity_layout_without_fabricating_legacy() {
    let tmp = tempfile::TempDir::new().expect("test: tempdir");
    let data_dir = tmp.path();
    let legacy_key = node_id(&MatrixCoordinate::new(0, 0, 0).expect("test: coord"));

    // No legacy dir exists.
    let id_dir = adopt_legacy_identity(data_dir, &legacy_key).expect("test: adopt identity");
    assert_eq!(id_dir, identity_dir(data_dir));
    // A fresh node has no identity yet — adopt fabricates nothing; the real
    // keypair is created later by `load_or_create` at this exact path.
    assert!(!id_dir.exists());

    let nid = state_dir_key(DEVICE_ID);
    adopt_legacy_state_dir(data_dir, &legacy_key, &nid).expect("test: adopt state");
    assert!(!data_dir.join(&nid).exists());
    assert!(!data_dir.join(&legacy_key).exists());
    // metadata gate is absent => main.rs takes the fresh_boot branch.
    assert!(!data_dir.join(&nid).join("blockchain").join("metadata.json").exists());
}

#[test]
fn d5_adopt_is_idempotent_and_never_clobbers() {
    let tmp = tempfile::TempDir::new().expect("test: tempdir");
    let data_dir = tmp.path();
    let legacy_key = node_id(&MatrixCoordinate::new(0, 0, 0).expect("test: coord"));
    let nid = state_dir_key(DEVICE_ID);

    seed_legacy(data_dir, &legacy_key);
    adopt_legacy_identity(data_dir, &legacy_key).expect("test: adopt identity 1");
    adopt_legacy_state_dir(data_dir, &legacy_key, &nid).expect("test: adopt state 1");

    // Mutate the migrated live state, then run the adopt AGAIN. An already-
    // migrated tree (new dir present) must be left untouched.
    let head = data_dir.join(&nid).join("blockchain").join("blocks.dat");
    write(&head, "ADVANCED-HEAD");

    adopt_legacy_identity(data_dir, &legacy_key).expect("test: adopt identity 2");
    adopt_legacy_state_dir(data_dir, &legacy_key, &nid).expect("test: adopt state 2");

    assert_eq!(read(&head), "ADVANCED-HEAD", "migrated state must not be clobbered");
    assert_eq!(read(&identity_dir(data_dir).join("falcon_public.der")), "IDENTITY-BYTES");
}

#[test]
fn d5_partial_migration_does_not_overwrite_new_identity() {
    // New identity already present (a prior run created it) AND a stale legacy
    // identity lingers. Adopt must keep the NEW one, not the legacy copy.
    let tmp = tempfile::TempDir::new().expect("test: tempdir");
    let data_dir = tmp.path();
    let legacy_key = node_id(&MatrixCoordinate::new(0, 0, 0).expect("test: coord"));

    write(&identity_dir(data_dir).join("falcon_public.der"), "NEW-IDENTITY");
    write(&data_dir.join(&legacy_key).join("identity").join("falcon_public.der"), "STALE");

    let id_dir = adopt_legacy_identity(data_dir, &legacy_key).expect("test: adopt identity");
    assert_eq!(read(&id_dir.join("falcon_public.der")), "NEW-IDENTITY");
}
