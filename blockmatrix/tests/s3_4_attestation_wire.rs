// Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
//
// S3.4 proofs for the MIRROR-ATTESTATION WIRE SURFACE.
//
//   CODEC        — the `[tag][len][JSON]` framing round-trips, and NO
//                  attacker-controlled byte string can panic the decoder
//                  (the A2 remote-SIGABRT shape).
//   WRONG KEY    — an attestation submitted over the wire under someone else's
//                  identity is REJECTED by the one gate. Relaying an honest
//                  third party's attestation is ACCEPTED, because the signature
//                  — not the sender — names the attestor.
//   ADMISSION    — an attestation about an asset this container does not hold is
//                  refused before any signature work; adopting the asset as an
//                  S3.4 foreign chain makes the same attestation acceptable.
//   BOUND        — the pool is capped per asset and globally, refuses NEWCOMERS
//                  at capacity, EVICTS NOTHING, and still admits a replacement
//                  from an incumbent attestor.

use blockmatrix::blockchain::block::{BlockAssetEntry, StoragePointer};
use blockmatrix::blockchain::{
    ForeignAssetChain, MirrorAttestationPool, NodeBlockchain, PoolFull,
    MAX_ATTESTATIONS_PER_ASSET, MAX_TOTAL_ATTESTATIONS,
};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::attestation_wire::{
    decode_mirror_attestation, encode_mirror_attestation, AttestationWireError,
    MAX_ATTESTATION_WIRE_BYTES, MIRROR_ATTEST_TAG,
};
use hypermesh_lib::attestation::{MatrixIndex, MirrorAttestation};
use hypermesh_lib::{NodeSigner, WireSignedProof};
use trustchain::identity::FalconIdentity;
use trustchain::proof_of_state::StateProof;

fn coord() -> MatrixCoordinate {
    MatrixCoordinate::new(3, 5, 7).expect("test: valid coordinate")
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
    nonce[..8].copy_from_slice(&cell.x.to_le_bytes());
    nonce[8..16].copy_from_slice(&spine_seq.to_le_bytes());
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

/// A chain holding one spine entry for `asset`. Returns the head's spine point.
async fn chain_holding(asset: [u8; 32]) -> (NodeBlockchain, String, u64) {
    let chain = NodeBlockchain::new(coord());
    let entry = BlockAssetEntry::new_bound(
        asset,
        &StateProof::new_for_testing(),
        StoragePointer::Genesis,
        blockmatrix::assets::core::AssetRegistration::genesis(coord()),
    );
    chain.add_block(vec![entry]).await.expect("test: add_block");
    let head = chain
        .asset_lineage(&asset)
        .await
        .head()
        .cloned()
        .expect("test: asset head");
    (chain, head.lineage_id(), head.asset_seq())
}

/// Round-trip an attestation through the wire codec, as the dispatcher does.
fn over_the_wire(attestation: &MirrorAttestation) -> MirrorAttestation {
    let bytes = encode_mirror_attestation(attestation).expect("test: encode");
    decode_mirror_attestation(&bytes).expect("test: decode")
}

// ── CODEC ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_4_attestation_wire_roundtrips() {
    let asset = [0x71u8; 32];
    let (_chain, spine, seq) = chain_holding(asset).await;
    let mirror = FalconIdentity::generate();
    let attestation = attest(&mirror, asset, MatrixIndex::new(1, 2, 3), &spine, seq);

    let bytes = encode_mirror_attestation(&attestation).expect("test: encode");
    assert_eq!(bytes[0], MIRROR_ATTEST_TAG);
    assert_eq!(
        decode_mirror_attestation(&bytes).expect("test: decode"),
        attestation
    );
}

/// The A2 shape: a length prefix trusted into a slice expression is a
/// remote-triggerable abort. Nothing here may panic on any input.
#[test]
fn s3_4_attestation_wire_never_panics_on_hostile_bytes() {
    let mut sample = MirrorAttestation {
        asset_hash: [7u8; 32],
        matrix_index: MatrixIndex::new(1, -2, 3),
        mirror: "a".repeat(64),
        spine_point: "b".repeat(64),
        spine_seq: 9,
        signature: WireSignedProof {
            proof_bytes: vec![1, 2, 3],
            signature: vec![4, 5, 6],
            signer_pubkey: vec![7, 8, 9],
            nonce: [3u8; 32],
        },
    };
    let good = encode_mirror_attestation(&sample).expect("test: encode");

    // Every truncation of a valid message.
    for cut in 0..good.len() {
        assert!(decode_mirror_attestation(&good[..cut]).is_err());
    }

    // A declared length beyond the cap, and beyond what is present.
    let mut huge = good.clone();
    huge[1..5].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        decode_mirror_attestation(&huge),
        Err(AttestationWireError::TooLarge { .. })
    ));

    let mut lying = good.clone();
    lying[1..5].copy_from_slice(&((MAX_ATTESTATION_WIRE_BYTES - 1) as u32).to_le_bytes());
    assert!(matches!(
        decode_mirror_attestation(&lying),
        Err(AttestationWireError::LengthMismatch { .. })
    ));

    // Wrong tag, empty input, garbage payloads.
    let mut wrong = good.clone();
    wrong[0] = 0x00;
    assert!(matches!(
        decode_mirror_attestation(&wrong),
        Err(AttestationWireError::WrongTag { got: 0x00 })
    ));
    assert!(decode_mirror_attestation(&[]).is_err());
    for seed in 0..64u8 {
        let mut noise = vec![MIRROR_ATTEST_TAG];
        noise.extend_from_slice(&32u32.to_le_bytes());
        noise.extend_from_slice(&[seed; 32]);
        assert!(decode_mirror_attestation(&noise).is_err());
    }

    // Oversized payloads are refused at BOTH ends, so a giant string field can
    // never reach `lib`'s u32 length-prefix builder.
    sample.spine_point = "x".repeat(MAX_ATTESTATION_WIRE_BYTES + 1);
    assert!(matches!(
        encode_mirror_attestation(&sample),
        Err(AttestationWireError::TooLarge { .. })
    ));
}

// ── WRONG KEY ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_4_wire_attestation_from_the_wrong_key_is_rejected() {
    let asset = [0x72u8; 32];
    let (chain, spine, seq) = chain_holding(asset).await;
    let mirror = FalconIdentity::generate();
    let stranger = FalconIdentity::generate();
    let honest = attest(&mirror, asset, MatrixIndex::new(4, 4, 4), &spine, seq);

    // (a) IMPERSONATION — a real signature from the stranger's key, presented
    //     under the honest mirror's identity string. This is the exact thing the
    //     wire surface must not let a submitter do.
    let mut impersonation =
        attest(&stranger, asset, MatrixIndex::new(4, 4, 4), &spine, seq);
    impersonation.mirror = mirror.node_id().to_string();
    chain
        .accept_wire_attestation(over_the_wire(&impersonation))
        .await
        .expect_err("test: attesting AS somebody else must be refused");

    // (b) SWAPPED PUBKEY — the envelope's key replaced with the honest mirror's,
    //     so the claimed identity derives correctly but the signature does not.
    let mut swapped = attest(&stranger, asset, MatrixIndex::new(5, 5, 5), &spine, seq);
    swapped.mirror = mirror.node_id().to_string();
    swapped.signature.signer_pubkey = mirror.public_key_bytes().to_vec();
    swapped.signature.proof_bytes = swapped.my_canonical_bytes();
    chain
        .accept_wire_attestation(over_the_wire(&swapped))
        .await
        .expect_err("test: a borrowed public key must not validate");

    // (c) TAMPERED SPINE POINT — altered after signing.
    let mut moved = honest.clone();
    moved.spine_point = "de".repeat(32);
    chain
        .accept_wire_attestation(over_the_wire(&moved))
        .await
        .expect_err("test: a tampered spine point must be refused");

    assert_eq!(chain.mirror_attestation_count(&asset).await, 0);

    // (d) RELAY IS NOT IMPERSONATION — the honest attestation is accepted no
    //     matter who put it on the wire, because the SIGNATURE names the
    //     attestor. That is what makes an owner able to learn about a mirror it
    //     is not directly connected to.
    chain
        .accept_wire_attestation(over_the_wire(&honest))
        .await
        .expect("test: an honest attestation relayed by anyone must be accepted");
    assert_eq!(chain.mirror_attestation_count(&asset).await, 1);
    assert_eq!(chain.mirror_attestations(&asset).await[0], honest);
}

// ── ADMISSION ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_4_wire_attestations_are_only_cached_for_assets_we_hold() {
    let held = [0x73u8; 32];
    let (chain, spine, seq) = chain_holding(held).await;
    let mirror = FalconIdentity::generate();

    // An asset nobody here has ever heard of: the pool key space must not be
    // the sender's to choose.
    let unknown = [0x74u8; 32];
    let orphaned = attest(&mirror, unknown, MatrixIndex::new(1, 1, 1), &spine, seq);
    let error = chain
        .accept_wire_attestation(over_the_wire(&orphaned))
        .await
        .expect_err("test: an attestation about an unheld asset must not be cached");
    assert!(
        error.contains("holds no asset"),
        "expected the held-asset admission rule, got: {error}"
    );
    assert_eq!(chain.mirror_attestation_total().await, 0);

    // Adopting the asset as an S3.4 FOREIGN chain makes the very same
    // attestation acceptable — "hold" spans both sides of the container.
    let author = FalconIdentity::generate();
    let mut proof = StateProof::new_for_testing();
    proof.stake_proof.stake_holder_id = author.node_id().to_string();
    let mut entry = BlockAssetEntry::new_bound(
        unknown,
        &proof,
        StoragePointer::Genesis,
        blockmatrix::assets::core::AssetRegistration::genesis(coord()),
    );
    entry.set_asset_lineage(None, 0);
    entry.sign_proof(&author).expect("test: FALCON sign");
    chain
        .accept_foreign_asset_chain(ForeignAssetChain::new(unknown, vec![entry]))
        .await
        .expect("test: foreign chain adopted");

    chain
        .accept_wire_attestation(over_the_wire(&orphaned))
        .await
        .expect("test: the asset is now held, so the attestation is cached");
    assert_eq!(chain.mirror_attestation_count(&unknown).await, 1);
    assert_eq!(chain.mirror_attestation_count(&held).await, 0);
}

// ── BOUND ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_4_per_asset_attestation_bound_holds_under_flood_and_evicts_nothing() {
    let asset = [0x75u8; 32];
    let (chain, spine, seq) = chain_holding(asset).await;

    // ONE keypair is enough to flood: the pool is keyed by (matrix cell,
    // mirror), so a single identity can claim every cell in the lattice. That
    // is precisely why identity count cannot be the bound.
    let flooder = FalconIdentity::generate();

    let first = attest(&flooder, asset, MatrixIndex::new(0, 0, 0), &spine, seq);
    chain
        .accept_wire_attestation(over_the_wire(&first))
        .await
        .expect("test: first attestation accepted");

    let mut refusals = 0usize;
    for n in 1..(MAX_ATTESTATIONS_PER_ASSET + 32) as i64 {
        let candidate = attest(&flooder, asset, MatrixIndex::new(n, 0, 0), &spine, seq);
        if let Err(e) = chain.accept_wire_attestation(over_the_wire(&candidate)).await {
            assert!(
                e.contains("mirror attestations"),
                "expected the pool bound, got: {e}"
            );
            refusals += 1;
        }
    }

    assert_eq!(
        chain.mirror_attestation_count(&asset).await,
        MAX_ATTESTATIONS_PER_ASSET,
        "the per-asset bound must hold exactly"
    );
    assert_eq!(refusals, 32, "everything past the cap must be refused");

    // NOTHING was evicted: the very first attestation, which an owner may
    // already have sealed, is still there.
    assert_eq!(
        chain
            .mirror_attestations(&asset)
            .await
            .iter()
            .filter(|a| **a == first)
            .count(),
        1,
        "an incumbent attestation must survive a flood"
    );

    // A REPLACEMENT from an incumbent attestor is still admitted at capacity:
    // it takes no new slot.
    let renewed = attest(
        &flooder,
        asset,
        MatrixIndex::new(0, 0, 0),
        "a-newer-spine-point",
        seq + 1,
    );
    chain
        .accept_wire_attestation(over_the_wire(&renewed))
        .await
        .expect("test: a replacement is not growth and must be admitted");
    assert_eq!(
        chain.mirror_attestation_count(&asset).await,
        MAX_ATTESTATIONS_PER_ASSET
    );
    assert_eq!(chain.mirror_attestations(&asset).await[0], renewed);
}

/// The GLOBAL bound, exercised at the pool — the bounded primitive itself.
///
/// Deliberately not driven through `accept_wire_attestation`: reaching
/// [`MAX_TOTAL_ATTESTATIONS`] would mean minting that many FALCON-1024
/// signatures, and the property under test is the pool's accounting, not the
/// signature scheme's. `try_insert` performs no verification, which is exactly
/// what makes it usable here — and why `record_mirror_attestation` is the only
/// production door to it.
#[test]
fn s3_4_global_attestation_bound_holds_and_evicts_nothing() {
    fn synthetic(asset: u16, cell: i64) -> MirrorAttestation {
        let mut asset_hash = [0u8; 32];
        asset_hash[..2].copy_from_slice(&asset.to_le_bytes());
        MirrorAttestation {
            asset_hash,
            matrix_index: MatrixIndex::new(cell, 0, 0),
            mirror: format!("{cell:064x}"),
            spine_point: "spine".to_string(),
            spine_seq: 0,
            signature: WireSignedProof {
                proof_bytes: Vec::new(),
                signature: Vec::new(),
                signer_pubkey: Vec::new(),
                nonce: [0u8; 32],
            },
        }
    }

    let mut pool = MirrorAttestationPool::new();
    let per_asset = MAX_ATTESTATIONS_PER_ASSET;
    let assets = MAX_TOTAL_ATTESTATIONS / per_asset;

    for asset in 0..assets as u16 {
        for cell in 0..per_asset as i64 {
            pool.try_insert(synthetic(asset, cell))
                .expect("test: accepted below the global cap");
        }
    }
    assert_eq!(pool.total(), MAX_TOTAL_ATTESTATIONS);

    // A fresh asset, well under its own per-asset cap, is now refused by the
    // GLOBAL bound.
    assert_eq!(
        pool.try_insert(synthetic(assets as u16, 0)),
        Err(PoolFull::Global {
            limit: MAX_TOTAL_ATTESTATIONS
        })
    );
    assert_eq!(pool.total(), MAX_TOTAL_ATTESTATIONS);
    // The refused asset left NO empty slot behind — a refusal must not itself
    // create an attacker-keyed entry.
    assert_eq!(pool.asset_count(), assets);

    // Replacement still works at the global cap.
    let mut renewed = synthetic(0, 0);
    renewed.spine_seq = 99;
    assert!(pool.try_insert(renewed).expect("test: replacement").is_some());
    assert_eq!(pool.total(), MAX_TOTAL_ATTESTATIONS);

    // Releasing an asset gives the budget back, and nothing else was evicted.
    assert_eq!(pool.clear_asset(&synthetic(0, 0).asset_hash), per_asset);
    assert_eq!(pool.total(), MAX_TOTAL_ATTESTATIONS - per_asset);
    assert_eq!(pool.count_for(&synthetic(1, 0).asset_hash), per_asset);
    pool.try_insert(synthetic(assets as u16, 0))
        .expect("test: room was made");
}

/// The per-asset bound at the pool, where the exact error is visible.
#[test]
fn s3_4_per_asset_bound_reports_itself_and_leaves_no_empty_slot() {
    let mut pool = MirrorAttestationPool::new();
    let asset = [0x76u8; 32];
    let make = |cell: i64| MirrorAttestation {
        asset_hash: asset,
        matrix_index: MatrixIndex::new(cell, 0, 0),
        mirror: format!("{cell:064x}"),
        spine_point: "spine".to_string(),
        spine_seq: 0,
        signature: WireSignedProof {
            proof_bytes: Vec::new(),
            signature: Vec::new(),
            signer_pubkey: Vec::new(),
            nonce: [0u8; 32],
        },
    };

    for cell in 0..MAX_ATTESTATIONS_PER_ASSET as i64 {
        pool.try_insert(make(cell)).expect("test: below cap");
    }
    assert_eq!(
        pool.try_insert(make(MAX_ATTESTATIONS_PER_ASSET as i64)),
        Err(PoolFull::Asset {
            limit: MAX_ATTESTATIONS_PER_ASSET
        })
    );
    assert_eq!(pool.count_for(&asset), MAX_ATTESTATIONS_PER_ASSET);
    assert_eq!(pool.total(), MAX_ATTESTATIONS_PER_ASSET);

    // A refusal for an entirely UNKNOWN asset creates no entry for it.
    let mut other = make(0);
    other.asset_hash = [0x77u8; 32];
    assert_eq!(pool.asset_count(), 1);
    pool.try_insert(other).expect("test: a different asset has its own budget");
    assert_eq!(pool.asset_count(), 2);
}
