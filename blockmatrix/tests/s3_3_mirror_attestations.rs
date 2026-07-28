// Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
//
// S3.3 proofs for MIRROR ATTESTATIONS — matrix-indexed, sealed by the owner.
//
//   THIRD PARTY  — an attestation signed by a mirror that is NOT the asset's
//                  author/owner is ACCEPTED. This is the exact thing H3's
//                  `signer_binds_to_author` forbids on a block entry, which is
//                  why `signed_proof` cannot carry mirror attestations.
//   FORGERY      — wrong key, tampered matrix index, tampered asset, tampered
//                  spine point, and identity-binding mismatch are all REJECTED.
//   ORDERING     — the seal root is a function of the SET, ordered by MATRIX
//                  INDEX: shuffling insertion order cannot change the root, and
//                  no clock/counter participates.
//   TAMPER       — stripping or adding an attestation after sealing changes the
//                  recomputed root and is detectable.
//   COMMITMENT   — the seal rides inside the `StateProof` body, so it reaches
//                  `proof_hash` and therefore the BLOCK HASH; altering it
//                  changes the block hash.
//   SPINE        — accumulating attestations changes NOTHING on the spine; the
//                  owner's seal appends exactly one checkpoint entry and the
//                  asset's lineage still verifies unbroken.
//   OWNER GATE   — an ownerless asset (the S3.5 gap) fails CLOSED with a
//                  labelled error; a non-owner is refused; only the owner seals,
//                  and only as itself.

use blockmatrix::blockchain::block::{BlockAssetEntry, StoragePointer};
use blockmatrix::blockchain::{verify_attestation, NodeBlockchain};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use hypermesh_lib::attestation::{
    build_seal, seal_root, sealed_set_contains, verify_membership, verify_sealed_set, MatrixIndex,
    MirrorAttestation, SealBreak,
};
use hypermesh_lib::{NodeSigner, WireSignedProof};
use std::sync::Arc;
use trustchain::identity::FalconIdentity;
use trustchain::proof_of_state::StateProof;

fn coord() -> MatrixCoordinate {
    MatrixCoordinate::new(3, 5, 7).expect("test: valid coordinate")
}

/// A locally-appendable entry for `asset_hash`, owned by `owner` when given.
fn local_entry(asset_hash: [u8; 32], owner: Option<&str>) -> BlockAssetEntry {
    let mut registration = blockmatrix::assets::core::AssetRegistration::genesis(coord());
    if let Some(owner) = owner {
        registration = registration.with_owner(owner);
    }
    BlockAssetEntry::new_bound(
        asset_hash,
        &StateProof::new_for_testing(),
        StoragePointer::Genesis,
        registration,
    )
}

/// Mint a REAL FALCON-signed attestation from `mirror`.
fn attest(
    mirror: &FalconIdentity,
    asset_hash: [u8; 32],
    cell: MatrixIndex,
    spine_point: &str,
    spine_seq: u64,
) -> MirrorAttestation {
    let identity = mirror.node_id().to_string();
    let proof_bytes =
        MirrorAttestation::canonical_bytes(&asset_hash, cell, &identity, spine_point, spine_seq);
    let mut nonce = [0u8; 32];
    nonce[0] = (cell.x & 0xFF) as u8;
    nonce[1] = (spine_seq & 0xFF) as u8;
    let digest = MirrorAttestation::signing_digest(&proof_bytes, &nonce);
    let signature = mirror.sign(&digest).expect("test: FALCON sign");
    MirrorAttestation {
        asset_hash,
        matrix_index: cell,
        mirror: identity,
        spine_point: spine_point.to_string(),
        spine_seq,
        signature: WireSignedProof {
            proof_bytes,
            signature,
            signer_pubkey: mirror.public_key_bytes().to_vec(),
            nonce,
        },
    }
}

/// A chain holding one entry for `asset`, owned by a freshly generated owner.
/// Returns (chain, owner identity string, asset head lineage_id, head seq).
async fn owned_asset(asset: [u8; 32]) -> (NodeBlockchain, String, String, u64) {
    let owner = FalconIdentity::generate().node_id().to_string();
    let chain = NodeBlockchain::new(coord());
    chain
        .add_block(vec![local_entry(asset, Some(&owner))])
        .await
        .expect("test: add_block");
    let head = chain
        .asset_lineage(&asset)
        .await
        .head()
        .cloned()
        .expect("test: asset head");
    (chain, owner, head.lineage_id(), head.asset_seq())
}

// ── THIRD PARTY ────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_3_third_party_attestation_is_accepted() {
    let asset = [0x31u8; 32];
    let (chain, owner, spine, seq) = owned_asset(asset).await;

    // The mirror is a DIFFERENT identity from the asset's owner and from the
    // entry's author (the local chain wrote the head).
    let mirror = FalconIdentity::generate();
    let attestation = attest(&mirror, asset, MatrixIndex::new(1, 2, 3), &spine, seq);
    assert_ne!(
        attestation.mirror, owner,
        "the attestor must be a third party, not the owner"
    );

    // The H3 gate would REJECT this signer on a block entry: the entry's
    // claimed author is the head's stake_holder_id, not the mirror.
    let head_author = chain
        .asset_lineage(&asset)
        .await
        .head()
        .expect("test: head")
        .state_proof
        .stake_proof
        .stake_holder_id
        .clone();
    assert_ne!(
        head_author, attestation.mirror,
        "signer != author — `signer_binds_to_author` would reject this on a block entry"
    );

    // The attestation verifier accepts it: binding is to the NAMED THIRD PARTY.
    assert!(verify_attestation(&attestation), "third-party signature must verify");
    chain
        .record_mirror_attestation(attestation.clone())
        .await
        .expect("test: third-party attestation must be accepted");
    assert_eq!(chain.mirror_attestation_count(&asset).await, 1);
    assert_eq!(chain.mirror_attestations(&asset).await[0], attestation);
}

#[tokio::test]
async fn s3_3_many_mirrors_accumulate_and_dedupe_by_cell_and_identity() {
    let asset = [0x32u8; 32];
    let (chain, _owner, spine, seq) = owned_asset(asset).await;

    let mirrors: Vec<FalconIdentity> = (0..5).map(|_| FalconIdentity::generate()).collect();
    for (i, mirror) in mirrors.iter().enumerate() {
        let cell = MatrixIndex::new(i as i64, 0, 0);
        chain
            .record_mirror_attestation(attest(mirror, asset, cell, &spine, seq))
            .await
            .expect("test: attestation accepted");
    }
    assert_eq!(chain.mirror_attestation_count(&asset).await, 5);

    // Same mirror, same cell, NEWER spine point: replaces, never duplicates.
    chain
        .record_mirror_attestation(attest(
            &mirrors[0],
            asset,
            MatrixIndex::new(0, 0, 0),
            "a-newer-spine-point",
            seq + 1,
        ))
        .await
        .expect("test: re-attestation accepted");
    assert_eq!(
        chain.mirror_attestation_count(&asset).await,
        5,
        "re-attesting must replace, not inflate the mirror set"
    );

    // Same mirror at a DIFFERENT cell is a distinct mirror instance.
    chain
        .record_mirror_attestation(attest(
            &mirrors[0],
            asset,
            MatrixIndex::new(0, 0, 9),
            &spine,
            seq,
        ))
        .await
        .expect("test: attestation accepted");
    assert_eq!(chain.mirror_attestation_count(&asset).await, 6);
}

// ── FORGERY ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_3_forged_attestations_are_rejected() {
    let asset = [0x33u8; 32];
    let (chain, _owner, spine, seq) = owned_asset(asset).await;
    let mirror = FalconIdentity::generate();
    let honest = attest(&mirror, asset, MatrixIndex::new(4, 4, 4), &spine, seq);
    assert!(verify_attestation(&honest));

    // (a) WRONG KEY — a real signature from someone else's key, presented under
    //     a different pubkey.
    let stranger = FalconIdentity::generate();
    let mut wrong_key = honest.clone();
    wrong_key.signature.signer_pubkey = stranger.public_key_bytes().to_vec();
    assert!(!verify_attestation(&wrong_key), "wrong key must be rejected");
    chain
        .record_mirror_attestation(wrong_key)
        .await
        .expect_err("test: wrong-key attestation must be refused");

    // (b) TAMPERED MATRIX INDEX — the ordering key itself.
    let mut moved = honest.clone();
    moved.matrix_index = MatrixIndex::new(4, 4, 5);
    assert!(!verify_attestation(&moved), "tampered matrix cell must be rejected");
    chain
        .record_mirror_attestation(moved)
        .await
        .expect_err("test: relocated attestation must be refused");

    // (c) TAMPERED ASSET.
    let mut other_asset = honest.clone();
    other_asset.asset_hash = [0xFFu8; 32];
    assert!(!verify_attestation(&other_asset), "tampered asset must be rejected");

    // (d) TAMPERED SPINE POINT — attesting to a point the mirror never signed.
    let mut moved_spine = honest.clone();
    moved_spine.spine_point = "0".repeat(64);
    assert!(!verify_attestation(&moved_spine), "tampered spine point must be rejected");
    let mut moved_seq = honest.clone();
    moved_seq.spine_seq = seq + 99;
    assert!(!verify_attestation(&moved_seq), "tampered spine seq must be rejected");

    // (e) IDENTITY-BINDING MISMATCH — a fully valid FALCON signature over
    //     canonical bytes that claim SOMEONE ELSE is the mirror.
    let impersonator = FalconIdentity::generate();
    let victim = FalconIdentity::generate().node_id().to_string();
    let cell = MatrixIndex::new(7, 7, 7);
    let proof_bytes =
        MirrorAttestation::canonical_bytes(&asset, cell, &victim, &spine, seq);
    let nonce = [5u8; 32];
    let digest = MirrorAttestation::signing_digest(&proof_bytes, &nonce);
    let impersonation = MirrorAttestation {
        asset_hash: asset,
        matrix_index: cell,
        mirror: victim,
        spine_point: spine.clone(),
        spine_seq: seq,
        signature: WireSignedProof {
            proof_bytes,
            signature: impersonator.sign(&digest).expect("test: sign"),
            signer_pubkey: impersonator.public_key_bytes().to_vec(),
            nonce,
        },
    };
    assert!(
        impersonation.proof_bytes_match(),
        "structural binding intentionally holds — only identity binding fails"
    );
    assert!(
        !verify_attestation(&impersonation),
        "an attestation claiming another identity must be rejected"
    );
    chain
        .record_mirror_attestation(impersonation)
        .await
        .expect_err("test: impersonation must be refused");

    // Nothing forged entered the pool.
    assert_eq!(chain.mirror_attestation_count(&asset).await, 0);
}

// ── ORDERING ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_3_seal_root_is_matrix_ordered_not_arrival_ordered() {
    let asset = [0x34u8; 32];
    let mirrors: Vec<FalconIdentity> = (0..7).map(|_| FalconIdentity::generate()).collect();
    // Deliberately non-monotonic cells so insertion order != matrix order.
    let cells = [
        MatrixIndex::new(5, -2, 0),
        MatrixIndex::new(-9, 3, 3),
        MatrixIndex::new(0, 0, 0),
        MatrixIndex::new(5, -2, -1),
        MatrixIndex::new(2, 100, 7),
        MatrixIndex::new(-9, 3, 2),
        MatrixIndex::new(1, 1, 1),
    ];
    let set: Vec<MirrorAttestation> = mirrors
        .iter()
        .zip(cells)
        .map(|(m, cell)| attest(m, asset, cell, "spine-abc", 3))
        .collect();

    // Two chains fed the SAME attestations in opposite orders.
    let forward = NodeBlockchain::new(coord());
    let backward = NodeBlockchain::new(coord());
    for attestation in set.iter() {
        forward
            .record_mirror_attestation(attestation.clone())
            .await
            .expect("test: accepted");
    }
    for attestation in set.iter().rev() {
        backward
            .record_mirror_attestation(attestation.clone())
            .await
            .expect("test: accepted");
    }

    let a = forward.mirror_attestations(&asset).await;
    let b = backward.mirror_attestations(&asset).await;
    assert_eq!(a, b, "the pool must present the set in MATRIX order, not arrival order");

    // The presented order IS matrix-lexicographic.
    let presented: Vec<MatrixIndex> = a.iter().map(|x| x.matrix_index).collect();
    let mut expected = cells.to_vec();
    expected.sort();
    assert_eq!(presented, expected);

    // And the root is identical under any shuffle of the same set.
    let root = seal_root(&set);
    assert_eq!(root, seal_root(&a));
    assert_eq!(root, seal_root(&b));
    let mut shuffled = set.clone();
    shuffled.swap(0, 6);
    shuffled.swap(2, 4);
    shuffled.reverse();
    assert_eq!(root, seal_root(&shuffled), "the root is a function of the SET only");
}

// ── SPINE + SEAL ───────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_3_accumulation_leaves_the_spine_untouched_and_seal_extends_it_by_one() {
    let asset = [0x35u8; 32];
    let (chain, owner, spine, seq) = owned_asset(asset).await;

    let before = chain
        .verify_asset_lineage(&asset)
        .await
        .expect("test: lineage verifies before");
    let height_before = chain.get_height().await;

    // Accumulate ten mirrors OFF-SPINE.
    for i in 0..10 {
        let mirror = FalconIdentity::generate();
        chain
            .record_mirror_attestation(attest(
                &mirror,
                asset,
                MatrixIndex::new(i, i * 2, -i),
                &spine,
                seq,
            ))
            .await
            .expect("test: accepted");
    }

    let after = chain
        .verify_asset_lineage(&asset)
        .await
        .expect("test: lineage still verifies");
    assert_eq!(before, after, "accumulation must not touch the asset spine");
    assert_eq!(chain.get_height().await, height_before, "no block was written");

    // The OWNER seals: exactly one checkpoint entry is appended.
    let receipt = chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: owner may seal");
    assert_eq!(receipt.seal.count, 10);
    assert_eq!(receipt.attestations.len(), 10);

    let sealed = chain
        .verify_asset_lineage(&asset)
        .await
        .expect("test: lineage verifies after the seal");
    assert_eq!(
        sealed.sequence(),
        vec![0, 1],
        "the seal is one ordinary spine entry — the title stays gap-free"
    );
    let head = sealed.head().expect("test: head");
    assert_eq!(
        head.prev_asset_entry(),
        Some(after.head().expect("test: prior head").lineage_id().as_str()),
        "the checkpoint names the prior head — single-parent continuity preserved"
    );
    assert!(head.succeeds(after.head().expect("test: prior head")));

    // The seal is readable back off the chain.
    let (entry, on_chain) = chain
        .latest_mirror_seal(&asset)
        .await
        .expect("test: seal recorded on-chain");
    assert_eq!(on_chain, receipt.seal);
    assert_eq!(entry.proof_hash, head.proof_hash);
    verify_sealed_set(&asset, &receipt.attestations, &on_chain)
        .expect("test: the sealed set verifies against its root");
}

#[tokio::test]
async fn s3_3_seal_is_hash_committed_through_proof_hash() {
    let asset = [0x36u8; 32];
    let (chain, owner, spine, seq) = owned_asset(asset).await;
    let mirror = FalconIdentity::generate();
    chain
        .record_mirror_attestation(attest(&mirror, asset, MatrixIndex::new(2, 2, 2), &spine, seq))
        .await
        .expect("test: accepted");

    let receipt = chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: owner may seal");
    let block = receipt.block.clone();

    // The entry's proof_hash really is BLAKE3(serialize(state_proof)) INCLUDING
    // the seal, and the block hash is computed over it.
    let entry = &block.entries[0];
    let bytes = serde_json::to_vec(&entry.state_proof).expect("test: serialize proof");
    assert_eq!(*blake3::hash(&bytes).as_bytes(), entry.proof_hash);
    assert_eq!(block.hash, block.calculate_hash());

    // Alter ONE character of the sealed root and re-derive honestly: the block
    // hash changes. A stripped or swapped seal cannot hide inside a valid block.
    let mut forged = block.clone();
    let mut seal = receipt.seal.clone();
    // Flip the first hex nibble to a GUARANTEED-different value. (Prepending
    // '0' was a no-op ~1/16 of the time — whenever the root already began with
    // '0' — which made this a flaky test, not a real mutation.)
    let mut root_chars: Vec<char> = seal.root.chars().collect();
    root_chars[0] = if root_chars[0] == '0' { '1' } else { '0' };
    seal.root = root_chars.into_iter().collect();
    forged.entries[0].state_proof.mirror_seal = Some(seal);
    let bytes = serde_json::to_vec(&forged.entries[0].state_proof).expect("test: serialize");
    forged.entries[0].proof_hash = *blake3::hash(&bytes).as_bytes();
    assert_ne!(
        forged.calculate_hash(),
        block.hash,
        "the seal reaches the block hash transitively through proof_hash"
    );
}

// ── TAMPER (post-seal) ─────────────────────────────────────────────────────

#[tokio::test]
async fn s3_3_stripped_or_added_attestation_is_detectable_post_seal() {
    let asset = [0x37u8; 32];
    let (chain, owner, spine, seq) = owned_asset(asset).await;
    for i in 0..6 {
        let mirror = FalconIdentity::generate();
        chain
            .record_mirror_attestation(attest(
                &mirror,
                asset,
                MatrixIndex::new(i, 0, i),
                &spine,
                seq,
            ))
            .await
            .expect("test: accepted");
    }

    let receipt = chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: owner may seal");
    let set = receipt.attestations.clone();
    let seal = receipt.seal.clone();
    verify_sealed_set(&asset, &set, &seal).expect("test: intact set verifies");

    // STRIP.
    let mut stripped = set.clone();
    stripped.remove(3);
    assert!(matches!(
        verify_sealed_set(&asset, &stripped, &seal),
        Err(SealBreak::CountMismatch { .. })
    ));

    // ADD.
    let intruder = FalconIdentity::generate();
    let mut added = set.clone();
    added.push(attest(&intruder, asset, MatrixIndex::new(50, 50, 50), &spine, seq));
    assert!(matches!(
        verify_sealed_set(&asset, &added, &seal),
        Err(SealBreak::CountMismatch { .. })
    ));

    // SWAP — same cardinality, different membership: caught by the root.
    let mut swapped = set.clone();
    swapped[2] = attest(&intruder, asset, MatrixIndex::new(2, 0, 2), &spine, seq);
    assert!(matches!(
        verify_sealed_set(&asset, &swapped, &seal),
        Err(SealBreak::RootMismatch { .. })
    ));

    // MEMBERSHIP against the intact set.
    assert_eq!(sealed_set_contains(&asset, &set, &seal, &set[4]), Ok(true));
    let outsider = attest(&intruder, asset, MatrixIndex::new(-1, -1, -1), &spine, seq);
    assert_eq!(sealed_set_contains(&asset, &set, &seal, &outsider), Ok(false));

    // A second seal after one more mirror joins produces a DIFFERENT root, and
    // the older seal still verifies against the set it committed to.
    let latecomer = FalconIdentity::generate();
    chain
        .record_mirror_attestation(attest(
            &latecomer,
            asset,
            MatrixIndex::new(9, 9, 9),
            &spine,
            seq,
        ))
        .await
        .expect("test: accepted");
    let second = chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: owner may re-seal");
    assert_ne!(second.seal.root, seal.root);
    assert_eq!(second.seal.count, 7);
    verify_sealed_set(&asset, &set, &seal).expect("test: the first seal still verifies");
    verify_sealed_set(&asset, &second.attestations, &second.seal)
        .expect("test: the second seal verifies");
}

// ── OWNER GATE ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_3_ownerless_asset_fails_closed_with_the_s3_5_label() {
    // Exactly today's production shape: every AssetRegistration constructor
    // hardcodes AuthorizationSet::default(), so the asset has NO owner.
    let asset = [0x38u8; 32];
    let chain = NodeBlockchain::new(coord());
    chain
        .add_block(vec![local_entry(asset, None)])
        .await
        .expect("test: add_block");

    let err = chain
        .seal_mirror_attestations(&asset, "anyone-at-all")
        .await
        .expect_err("test: an ownerless asset must not be sealable by anyone");
    assert!(err.contains("NO owner"), "error must name the real cause: {err}");
    assert!(err.contains("S3.5"), "error must label the S3.5 gap: {err}");
}

#[tokio::test]
async fn s3_3_non_owner_cannot_seal() {
    let asset = [0x39u8; 32];
    let (chain, owner, spine, seq) = owned_asset(asset).await;
    let mirror = FalconIdentity::generate();
    chain
        .record_mirror_attestation(attest(&mirror, asset, MatrixIndex::new(1, 1, 1), &spine, seq))
        .await
        .expect("test: accepted");

    // A MIRROR is not an owner: access is not the distribution right.
    let err = chain
        .seal_mirror_attestations(&asset, mirror.node_id())
        .await
        .expect_err("test: a mirror must not be able to seal");
    assert!(err.contains("does not hold the distribution right"), "{err}");

    // The owner can.
    chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: the owner may seal");
}

#[tokio::test]
async fn s3_3_seal_must_be_signed_by_the_owner_who_issues_it() {
    // On a chain WITH a signer, sealing as an owner we cannot sign for would
    // produce a block whose H3 envelope asserts someone else's WHO — peers
    // reject that at `signer_binds_to_author`, so we refuse locally.
    let asset = [0x3Au8; 32];
    let identity = Arc::new(FalconIdentity::generate());
    let absent_owner = FalconIdentity::generate().node_id().to_string();

    let chain = NodeBlockchain::new(coord())
        .with_signer(identity.clone() as Arc<dyn NodeSigner + Send + Sync>);
    chain
        .add_block(vec![local_entry(asset, Some(&absent_owner))])
        .await
        .expect("test: add_block");

    let err = chain
        .seal_mirror_attestations(&asset, &absent_owner)
        .await
        .expect_err("test: cannot seal on behalf of an owner we do not sign for");
    assert!(err.contains("this node signs as"), "{err}");
}

#[tokio::test]
async fn s3_3_sealing_an_empty_mirror_set_is_a_well_defined_checkpoint() {
    let asset = [0x3Bu8; 32];
    let (chain, owner, _spine, _seq) = owned_asset(asset).await;

    let receipt = chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: an empty mirror set is sealable");
    assert_eq!(receipt.seal.count, 0);
    assert_eq!(receipt.seal.sealed_by, owner);
    assert_eq!(receipt.seal.root, hex::encode(seal_root(&[])));
    verify_sealed_set(&asset, &[], &receipt.seal).expect("test: the empty set verifies");

    // And an attestation cannot be smuggled under an empty seal.
    let intruder = FalconIdentity::generate();
    let smuggled = vec![attest(&intruder, asset, MatrixIndex::new(0, 0, 0), "x", 0)];
    assert!(verify_sealed_set(&asset, &smuggled, &receipt.seal).is_err());
}

#[tokio::test]
async fn s3_3_seal_binds_to_its_asset() {
    // A seal for asset A must not validate a mirror set for asset B.
    let asset_a = [0x3Cu8; 32];
    let asset_b = [0x3Du8; 32];
    let (chain, owner, spine, seq) = owned_asset(asset_a).await;
    let mirror = FalconIdentity::generate();
    chain
        .record_mirror_attestation(attest(&mirror, asset_a, MatrixIndex::new(1, 2, 3), &spine, seq))
        .await
        .expect("test: accepted");
    let receipt = chain
        .seal_mirror_attestations(&asset_a, &owner)
        .await
        .expect("test: seal");

    assert!(matches!(
        verify_sealed_set(&asset_b, &receipt.attestations, &receipt.seal),
        Err(SealBreak::WrongAsset { .. })
    ));

    // Attestations for a different asset never enter asset A's set.
    let other = attest(&mirror, asset_b, MatrixIndex::new(1, 2, 3), &spine, seq);
    chain
        .record_mirror_attestation(other)
        .await
        .expect("test: accepted for its own asset");
    assert_eq!(chain.mirror_attestation_count(&asset_a).await, 1);
    assert_eq!(chain.mirror_attestation_count(&asset_b).await, 1);
    assert_eq!(
        build_seal(&owner, &chain.mirror_attestations(&asset_a).await).root,
        receipt.seal.root,
        "asset A's set is unchanged by an attestation for asset B"
    );
}

// ── B1: ONE GATE (accept ⊇ audit) ──────────────────────────────────────────

/// QA repro. An attestation with `spine_point: ""` is fully FALCON-valid and
/// correctly identity-bound, so the OLD accept gate (which re-listed
/// `proof_bytes_match` + `binds_to_signer` and omitted the non-empty
/// `spine_point` check) accepted it, pooled it, sealed it, and hash-committed it
/// into an immutable on-chain root — after which `verify_sealed_set` rejected
/// the owner's own set FOREVER with `NotStructurallyValid`. One cheap
/// attestation permanently destroyed the tamper-evidence of that seal.
///
/// The accept gate now DELEGATES to the audit gate, so this can never enter.
#[tokio::test]
async fn s3_3_empty_spine_point_cannot_poison_a_seal() {
    let asset = [0x40u8; 32];
    let (chain, owner, _spine, _seq) = owned_asset(asset).await;
    let mirror = FalconIdentity::generate();

    // Minted honestly in every respect EXCEPT the empty spine point.
    let poison = attest(&mirror, asset, MatrixIndex::new(1, 1, 1), "", 0);
    assert!(poison.proof_bytes_match(), "envelope really does cover these fields");
    assert!(poison.binds_to_signer(), "identity binding really does hold");
    assert!(
        !poison.is_structurally_valid(),
        "the AUDIT gate rejects it — a non-empty spine_point is required"
    );

    // ACCEPT gate must agree with the audit gate.
    assert!(
        !verify_attestation(&poison),
        "B1: the accept gate must reject anything the audit gate rejects"
    );
    chain
        .record_mirror_attestation(poison.clone())
        .await
        .expect_err("B1: an unauditable attestation must never be recorded");
    assert_eq!(chain.mirror_attestation_count(&asset).await, 0);

    // And therefore the seal it would have poisoned is clean and verifiable.
    let receipt = chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: owner may seal");
    assert_eq!(receipt.seal.count, 0);
    verify_sealed_set(&asset, &receipt.attestations, &receipt.seal)
        .expect("B1: the sealed set must remain verifiable");
}

/// The test that closes the CLASS of bug, not the instance: for EVERY
/// attestation shape we can produce, `verify_attestation(a)` implies
/// `a.is_structurally_valid()`. Since the accept gate calls the audit gate, a
/// field added to `is_structurally_valid` tomorrow cannot re-open the hole —
/// and if anyone re-inlines the structural checks, this fails.
#[tokio::test]
async fn s3_3_accept_gate_is_a_superset_of_the_audit_gate() {
    let asset = [0x41u8; 32];
    let (chain, _owner, spine, seq) = owned_asset(asset).await;
    let mirror = FalconIdentity::generate();
    let stranger = FalconIdentity::generate();
    let honest = attest(&mirror, asset, MatrixIndex::new(3, 1, 4), &spine, seq);

    // A corpus spanning every field the two gates could disagree about:
    // signature envelope, identity binding, and each emptiness requirement.
    let mut corpus = vec![honest.clone()];

    let mut empty_spine = honest.clone();
    empty_spine.spine_point = String::new();
    empty_spine.signature.proof_bytes = empty_spine.my_canonical_bytes();
    let digest = MirrorAttestation::signing_digest(
        &empty_spine.signature.proof_bytes,
        &empty_spine.signature.nonce,
    );
    empty_spine.signature.signature = mirror.sign(&digest).expect("test: sign");
    corpus.push(empty_spine);

    let mut empty_mirror = honest.clone();
    empty_mirror.mirror = String::new();
    empty_mirror.signature.proof_bytes = empty_mirror.my_canonical_bytes();
    let digest = MirrorAttestation::signing_digest(
        &empty_mirror.signature.proof_bytes,
        &empty_mirror.signature.nonce,
    );
    empty_mirror.signature.signature = mirror.sign(&digest).expect("test: sign");
    corpus.push(empty_mirror);

    let mut empty_pubkey = honest.clone();
    empty_pubkey.signature.signer_pubkey = Vec::new();
    corpus.push(empty_pubkey);

    let mut empty_signature = honest.clone();
    empty_signature.signature.signature = Vec::new();
    corpus.push(empty_signature);

    let mut wrong_key = honest.clone();
    wrong_key.signature.signer_pubkey = stranger.public_key_bytes().to_vec();
    corpus.push(wrong_key);

    let mut tampered_cell = honest.clone();
    tampered_cell.matrix_index = MatrixIndex::new(0, 0, 0);
    corpus.push(tampered_cell);

    let mut tampered_asset = honest.clone();
    tampered_asset.asset_hash = [0xEEu8; 32];
    corpus.push(tampered_asset);

    let mut tampered_seq = honest.clone();
    tampered_seq.spine_seq = seq.wrapping_add(1);
    corpus.push(tampered_seq);

    let mut rolled_nonce = honest.clone();
    rolled_nonce.signature.nonce[0] ^= 0xFF;
    corpus.push(rolled_nonce);

    // Also cover a cell/seq sweep of otherwise-honest attestations, so the
    // property is exercised on accepted inputs and not only on rejected ones.
    for i in 0..4i64 {
        corpus.push(attest(
            &mirror,
            asset,
            MatrixIndex::new(i, -i, i * 2),
            &spine,
            seq.wrapping_add(i as u64),
        ));
    }

    let mut accepted = 0usize;
    for (position, candidate) in corpus.iter().enumerate() {
        if verify_attestation(candidate) {
            accepted += 1;
            assert!(
                candidate.is_structurally_valid(),
                "SUPERSET VIOLATED at corpus[{position}]: the accept gate admitted an \
                 attestation the audit gate (verify_sealed_set) will reject forever"
            );
            // And the whole record→seal→audit round trip must close for it.
            chain
                .record_mirror_attestation(candidate.clone())
                .await
                .expect("accepted by the gate must be recordable");
        }
    }
    assert!(accepted >= 5, "corpus must exercise the ACCEPTING branch too");

    let owner_seal = build_seal("auditor", &chain.mirror_attestations(&asset).await);
    verify_sealed_set(&asset, &chain.mirror_attestations(&asset).await, &owner_seal)
        .expect("everything the accept gate admitted is auditable");
}

// ── F1: the sealed root is OPENABLE without the set ────────────────────────

#[tokio::test]
async fn s3_3_membership_proof_opens_the_seal_without_the_set() {
    let asset = [0x42u8; 32];
    let (chain, owner, spine, seq) = owned_asset(asset).await;
    let mut minted = Vec::new();
    for i in 0..7i64 {
        let mirror = FalconIdentity::generate();
        let attestation = attest(&mirror, asset, MatrixIndex::new(i, 9 - i, i), &spine, seq);
        chain
            .record_mirror_attestation(attestation.clone())
            .await
            .expect("test: accepted");
        minted.push(attestation);
    }

    let receipt = chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: owner may seal");
    let on_chain_seal = chain
        .latest_mirror_seal(&asset)
        .await
        .expect("test: seal is on the spine")
        .1;

    // Each mirror keeps its own O(log n) witness...
    let witnesses: Vec<_> = minted
        .iter()
        .map(|a| {
            receipt
                .membership_proof(a)
                .expect("test: every sealed attestation has a witness")
        })
        .collect();
    assert!(
        witnesses.iter().all(|w| w.path.len() <= 3),
        "a 7-leaf tree needs at most 3 path steps"
    );

    // ...and the seal opens against the ON-CHAIN root alone. Nothing here holds
    // the sealed set — this is exactly the capability a flat root lacked.
    for (attestation, witness) in minted.iter().zip(&witnesses) {
        verify_membership(&asset, attestation, witness, &on_chain_seal)
            .expect("F1: a member opens the on-chain seal with its own witness");
    }

    // A non-member cannot forge a witness from someone else's path.
    let intruder = attest(
        &FalconIdentity::generate(),
        asset,
        MatrixIndex::new(50, 50, 50),
        &spine,
        seq,
    );
    assert!(receipt.membership_proof(&intruder).is_none());
    assert!(matches!(
        verify_membership(&asset, &intruder, &witnesses[0], &on_chain_seal),
        Err(SealBreak::RootMismatch { .. })
    ));

    // A witness for asset A does not open asset A's seal for a B attestation.
    assert!(matches!(
        verify_membership(&[0x99u8; 32], &minted[0], &witnesses[0], &on_chain_seal),
        Err(SealBreak::WrongAsset { .. })
    ));

    // Altering the attestation invalidates its own witness.
    let mut altered = minted[2].clone();
    altered.signature.nonce[7] ^= 1;
    assert!(verify_membership(&asset, &altered, &witnesses[2], &on_chain_seal).is_err());

    // Whole-set verification still holds — the Merkle change is additive.
    verify_sealed_set(&asset, &receipt.attestations, &on_chain_seal)
        .expect("F1: whole-set verification is unchanged");
}

// ── F2: the checkpoint's own WHEN ──────────────────────────────────────────

#[tokio::test]
async fn s3_3_checkpoint_carries_its_own_time_proof() {
    let asset = [0x43u8; 32];
    let (chain, owner, spine, seq) = owned_asset(asset).await;
    let head_before = chain.asset_history_entries(&asset).await;
    let head_nonce = head_before
        .last()
        .expect("test: head entry")
        .state_proof
        .time_proof
        .nonce;

    chain
        .record_mirror_attestation(attest(
            &FalconIdentity::generate(),
            asset,
            MatrixIndex::new(1, 1, 1),
            &spine,
            seq,
        ))
        .await
        .expect("test: accepted");

    let first = chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: seal");
    let first_time = first.block.entries[0].state_proof.time_proof.clone();

    // F2: the checkpoint does NOT reuse the head's replay nonce...
    assert_ne!(
        first_time.nonce, head_nonce,
        "F2: a checkpoint must not reuse the head's TimeProof nonce"
    );
    assert!(first_time.nonce > 0);
    assert!(
        first_time.is_structurally_valid(),
        "the derived TimeProof must recompute its own proof_hash"
    );

    // ...and it is DERIVED, not clocked: re-sealing the identical set yields the
    // identical nonce (reproducible, no wall-clock read on this path).
    let second = chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: re-seal");
    assert_eq!(second.seal.root, first.seal.root, "same set, same root");
    assert_eq!(
        second.block.entries[0].state_proof.time_proof.nonce, first_time.nonce,
        "F2: the nonce is a pure function of the seal, not of the clock"
    );

    // A different seal (one more mirror) gets a different nonce.
    chain
        .record_mirror_attestation(attest(
            &FalconIdentity::generate(),
            asset,
            MatrixIndex::new(2, 2, 2),
            &spine,
            seq,
        ))
        .await
        .expect("test: accepted");
    let third = chain
        .seal_mirror_attestations(&asset, &owner)
        .await
        .expect("test: seal again");
    assert_ne!(third.seal.root, first.seal.root);
    assert_ne!(
        third.block.entries[0].state_proof.time_proof.nonce, first_time.nonce,
        "F2: the nonce is bound to the seal it stamps"
    );
}
