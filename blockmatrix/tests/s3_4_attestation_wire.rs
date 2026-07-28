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
//   BYTES (F1)   — the pool has a real BYTE budget, `spine_point` is length-
//                  capped at the one audit gate, and the two together bound the
//                  240 MiB RSS growth QA measured from a single keypair.
//   ORDER (F3)   — capacity is judged BEFORE the FALCON-1024 verification, so a
//                  full pool cannot be turned into a CPU-exhaustion primitive.

use blockmatrix::blockchain::block::{BlockAssetEntry, StoragePointer};
use blockmatrix::blockchain::{
    attestation_footprint_bytes, PresentedAssetChain, MirrorAttestationPool, NodeBlockchain,
    PoolFull, MAX_ATTESTATIONS_PER_ASSET, MAX_ATTESTATION_POOL_BYTES, MAX_TOTAL_ATTESTATIONS,
};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::attestation_wire::{
    decode_mirror_attestation, encode_mirror_attestation, AttestationWireError,
    MAX_ATTESTATION_WIRE_BYTES, MIRROR_ATTEST_TAG,
};
use hypermesh_lib::attestation::{MatrixIndex, MirrorAttestation, MAX_SPINE_POINT_BYTES};
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

/// Deterministic xorshift64* — a fuzz corpus that is reproducible from its
/// seed, so a failure here is re-runnable rather than a one-off sighting.
struct Prng(u64);

impl Prng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
}

/// A REAL adversarial pass over `decode_mirror_attestation`.
///
/// The previous proof reasoned about the code's SHAPE (checked slicing, no
/// `unwrap`). This one drives bytes. The shape being ruled out is the A2
/// shard-locate parser's remote-triggerable `SIGABRT`: a length prefix trusted
/// into a slice expression, reachable from the wire. Any panic or abort in
/// here fails the test process.
#[test]
fn s3_4_attestation_wire_survives_a_real_fuzz_pass() {
    const RANDOM_CASES: usize = 200_000;

    let mirror = FalconIdentity::generate();
    // A realistic message: real FALCON-1024 key and signature material, so the
    // corpus mutates something the size and shape of a live attestation.
    let sample = attest(&mirror, [0x5Au8; 32], MatrixIndex::new(4, -5, 6), "spine", 11);
    let good = encode_mirror_attestation(&sample).expect("test: encode");
    assert!(good.len() > 2000, "the fuzz corpus must be realistically sized");

    let mut decoded_ok = 0usize;
    let mut refused = 0usize;
    let mut note = |ok: bool| {
        if ok {
            decoded_ok += 1;
        } else {
            refused += 1;
        }
    };

    // (1) EVERY truncation, including the empty slice and the full message.
    // Only the untruncated message may decode.
    let mut truncations_decoded = 0usize;
    for cut in 0..=good.len() {
        let ok = decode_mirror_attestation(&good[..cut]).is_ok();
        if ok {
            truncations_decoded += 1;
            assert_eq!(cut, good.len(), "a truncated message decoded at {cut} bytes");
        }
        note(ok);
    }
    assert_eq!(truncations_decoded, 1);

    // (2) Every possible tag byte on an otherwise valid message.
    for tag in 0..=u8::MAX {
        let mut framed = good.clone();
        framed[0] = tag;
        let out = decode_mirror_attestation(&framed);
        if tag == MIRROR_ATTEST_TAG {
            assert!(out.is_ok(), "the real tag must still decode");
        } else {
            assert_eq!(out, Err(AttestationWireError::WrongTag { got: tag }));
        }
        note(out.is_ok());
    }

    // (3) Lying lengths: the extremes, the cap boundary, and every power of two
    //     up to u32::MAX — declared over a body that does not match.
    let mut lengths: Vec<u32> = vec![
        0,
        1,
        u32::MAX,
        u32::MAX - 1,
        MAX_ATTESTATION_WIRE_BYTES as u32,
        (MAX_ATTESTATION_WIRE_BYTES as u32).wrapping_add(1),
        (MAX_ATTESTATION_WIRE_BYTES as u32).wrapping_sub(1),
        good.len() as u32,
        (good.len() as u32).wrapping_sub(5),
        i32::MAX as u32,
    ];
    lengths.extend((0..32).map(|shift| 1u32 << shift));
    for declared in lengths {
        let mut framed = good.clone();
        framed[1..5].copy_from_slice(&declared.to_le_bytes());
        note(decode_mirror_attestation(&framed).is_ok());

        // ...and the same lie over a body of every small size, including none.
        for body in [0usize, 1, 4, 37] {
            let mut short = vec![MIRROR_ATTEST_TAG];
            short.extend_from_slice(&declared.to_le_bytes());
            short.extend(std::iter::repeat(0xABu8).take(body));
            note(decode_mirror_attestation(&short).is_ok());
        }
    }

    // (4) Single-bit flips across the whole valid message — the mutations most
    //     likely to land inside the length prefix or the JSON structure.
    let mut prng = Prng(0x5EED_1234_ABCD_0001);
    for _ in 0..20_000 {
        let mut framed = good.clone();
        let at = prng.below(framed.len());
        framed[at] ^= 1u8 << (prng.byte() % 8);
        note(decode_mirror_attestation(&framed).is_ok());
    }

    // (5) Bulk random payloads of random length, framed and unframed.
    for _ in 0..RANDOM_CASES {
        let len = prng.below(96);
        let mut noise: Vec<u8> = (0..len).map(|_| prng.byte()).collect();
        note(decode_mirror_attestation(&noise).is_ok());

        // Correctly framed, random body — drives the JSON parser directly.
        let body_len = prng.below(64);
        noise.clear();
        noise.push(MIRROR_ATTEST_TAG);
        noise.extend_from_slice(&(body_len as u32).to_le_bytes());
        noise.extend((0..body_len).map(|_| prng.byte()));
        note(decode_mirror_attestation(&noise).is_ok());
    }

    // The claim this test makes is ZERO PANICS and ZERO ABORTS — a panic
    // anywhere above fails the process, so reaching here IS the result.
    //
    // Some mutants legitimately decode: a single bit flipped inside a JSON
    // string or digit still yields a well-formed attestation, and the decoder
    // deliberately performs NO verification (that is
    // `record_mirror_attestation`'s job). What must never decode is a
    // TRUNCATED message, a WRONG-TAG message, or a message whose declared
    // length is not the length present — each asserted individually above.
    assert!(
        refused > RANDOM_CASES,
        "fuzz corpus was too small to be evidence: {refused} refusals"
    );
    println!(
        "fuzz: {} inputs driven, {refused} refused, {decoded_ok} decoded, zero panics/aborts",
        refused + decoded_ok
    );
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
        .accept_asset_chain(PresentedAssetChain::new(unknown, vec![entry]))
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

// ── BYTES (F1) ─────────────────────────────────────────────────────────────

/// A synthetic attestation with realistically-sized FALCON material. No
/// signature work: the pool performs no verification, and the property under
/// test is its ACCOUNTING.
fn weighed(asset: [u8; 32], cell: i64, spine_point: &str) -> MirrorAttestation {
    MirrorAttestation {
        asset_hash: asset,
        matrix_index: MatrixIndex::new(cell, 0, 0),
        mirror: format!("{cell:064x}"),
        spine_point: spine_point.to_string(),
        spine_seq: 0,
        signature: WireSignedProof {
            // Sized as `my_canonical_bytes` would be: framing + spine_point.
            proof_bytes: vec![0u8; 176 + spine_point.len()],
            signature: vec![0u8; 1280],
            signer_pubkey: vec![0u8; 1793],
            nonce: [0u8; 32],
        },
    }
}

/// F1 — the pool's bound is a BYTE budget, it fits R13, and the worst case is
/// stated in real numbers rather than estimated.
///
/// QA's reproduction, driven through the shipped wire path with ONE FALCON
/// keypair and `spine_point = "S".repeat(13_400)`:
///
/// ```text
/// attestations accepted   : 8192 (cap 8192)
/// victim RSS before/after : 4568 KiB -> 250632 KiB   (+240.3 MiB)
/// resident per attestation: 30,758 bytes
/// ```
///
/// The count caps were satisfied throughout. This test states what the caps now
/// actually bound.
#[test]
fn s3_4_the_pool_bound_is_a_byte_budget_that_fits_r13() {
    // Honest material: a `lineage_id` is hex of a BLAKE3 digest — 64 bytes.
    let honest = attestation_footprint_bytes(&weighed([1u8; 32], 0, &"ab".repeat(32)));
    // The heaviest attestation the audit gate will now admit.
    let worst = attestation_footprint_bytes(&weighed(
        [1u8; 32],
        0,
        &"c".repeat(MAX_SPINE_POINT_BYTES),
    ));
    // What the pool used to allow per slot, before the field cap.
    let unbounded = attestation_footprint_bytes(&weighed([1u8; 32], 0, &"S".repeat(13_400)));

    println!("honest attestation footprint : {honest} B");
    println!("worst-case footprint (capped) : {worst} B");
    println!("pre-fix footprint (uncapped)  : {unbounded} B  <- QA measured 30,758 B resident");
    println!(
        "pool budget                   : {} MiB ({:.1} % of R13's 4 GB)",
        MAX_ATTESTATION_POOL_BYTES / (1024 * 1024),
        100.0 * MAX_ATTESTATION_POOL_BYTES as f64 / (4.0 * 1024.0 * 1024.0 * 1024.0)
    );
    println!(
        "pre-fix worst case (count only): {:.1} MiB",
        (MAX_TOTAL_ATTESTATIONS * unbounded) as f64 / (1024.0 * 1024.0)
    );

    // The charged footprint is an OVER-estimate of what QA measured resident.
    assert!(
        unbounded >= 30_758,
        "the charge must not under-count real memory: {unbounded} < 30758"
    );

    // The count caps alone permitted ~240 MiB. The byte budget is 32 MiB.
    assert!(
        MAX_TOTAL_ATTESTATIONS * unbounded > 200 * 1024 * 1024,
        "the pre-fix count-only bound really was ~240 MiB"
    );

    // R13: 4 GB RAM. The budget is well under 1 % of it.
    const R13_RAM: usize = 4 * 1024 * 1024 * 1024;
    assert!(
        MAX_ATTESTATION_POOL_BYTES * 100 / R13_RAM < 1,
        "the pool must not claim 1 % of a minimum-spec device's RAM"
    );

    // And the budget BINDS: it is reached before the count cap, for honest and
    // for worst-case material alike. A byte budget that never fires is
    // decoration.
    assert!(
        MAX_TOTAL_ATTESTATIONS * honest > MAX_ATTESTATION_POOL_BYTES,
        "the byte budget must be the operative bound, not the count"
    );
    assert!(worst < 8 * 1024, "a capped attestation is small: {worst} B");
}

/// F1 — the byte budget refuses at capacity, evicts nothing, and admits a
/// replacement that is not growth.
///
/// Driven at the pool, for the same reason the global-count test is: reaching
/// the budget with real FALCON-1024 signatures would mean minting thousands of
/// them, and the property under test is the ACCOUNTING. `record_mirror_attestation`
/// is the only production door to `try_insert`.
#[test]
fn s3_4_pool_byte_budget_refuses_and_evicts_nothing() {
    let spine = "c".repeat(MAX_SPINE_POINT_BYTES);
    let per = attestation_footprint_bytes(&weighed([0u8; 32], 0, &spine));

    let mut pool = MirrorAttestationPool::new();
    let mut admitted = 0usize;
    let mut refusal = None;
    'fill: for asset in 0..(MAX_TOTAL_ATTESTATIONS / MAX_ATTESTATIONS_PER_ASSET) as u16 {
        let mut asset_hash = [0u8; 32];
        asset_hash[..2].copy_from_slice(&asset.to_le_bytes());
        for cell in 0..MAX_ATTESTATIONS_PER_ASSET as i64 {
            match pool.try_insert(weighed(asset_hash, cell, &spine)) {
                Ok(_) => admitted += 1,
                Err(full) => {
                    refusal = Some(full);
                    break 'fill;
                }
            }
        }
    }

    let full = refusal.expect("test: the byte budget must be reached before the count cap");
    assert!(
        matches!(full, PoolFull::Bytes { .. }),
        "the BYTE budget must be what refuses, got {full:?}"
    );
    assert!(
        admitted < MAX_TOTAL_ATTESTATIONS,
        "the count cap must not be what stopped us: {admitted}"
    );
    assert!(
        pool.bytes_held() <= MAX_ATTESTATION_POOL_BYTES,
        "the budget must hold exactly"
    );
    assert_eq!(pool.bytes_held(), pool.recomputed_bytes(), "accounting must not drift");
    assert_eq!(pool.total(), admitted);
    println!("byte budget reached after {admitted} attestations of {per} B each");

    // NOTHING was evicted: the very first attestation is still there.
    let mut first_asset = [0u8; 32];
    first_asset[..2].copy_from_slice(&0u16.to_le_bytes());
    assert_eq!(pool.count_for(&first_asset), MAX_ATTESTATIONS_PER_ASSET);

    // A same-size REPLACEMENT is admitted at capacity — it is not growth.
    let mut renewed = weighed(first_asset, 0, &spine);
    renewed.spine_seq = 99;
    let held_before = pool.bytes_held();
    assert!(pool
        .try_insert(renewed)
        .expect("test: a replacement is not growth")
        .is_some());
    assert_eq!(pool.bytes_held(), held_before, "same size, same charge");

    // A replacement that GROWS is charged for its growth, and is refused when
    // the growth does not fit. The byte budget applies to replacements too,
    // which is why it cannot be walked past one slot at a time — the count
    // guards, which exempt replacements entirely, could never see this.
    let headroom = MAX_ATTESTATION_POOL_BYTES.saturating_sub(pool.bytes_held());
    let mut too_fat = weighed(first_asset, 1, &spine);
    too_fat
        .signature
        .proof_bytes
        .extend(std::iter::repeat(0u8).take(headroom + 1));
    assert!(
        matches!(pool.try_insert(too_fat), Err(PoolFull::Bytes { .. })),
        "a replacement that grows past the budget must be refused"
    );

    // Growing by EXACTLY the headroom is admitted, and charged.
    let mut exact = weighed(first_asset, 1, &spine);
    exact
        .signature
        .proof_bytes
        .extend(std::iter::repeat(0u8).take(headroom));
    let before_growth = pool.bytes_held();
    assert!(pool
        .try_insert(exact)
        .expect("test: it fits exactly")
        .is_some());
    assert_eq!(
        pool.bytes_held(),
        before_growth + headroom,
        "a replacement's growth must be charged"
    );
    assert_eq!(pool.bytes_held(), MAX_ATTESTATION_POOL_BYTES);
    assert_eq!(pool.bytes_held(), pool.recomputed_bytes());

    // Releasing an asset gives the budget back — the reclaim path, by local
    // decision only.
    let released = pool.clear_asset(&first_asset);
    assert_eq!(released, MAX_ATTESTATIONS_PER_ASSET);
    assert_eq!(pool.bytes_held(), pool.recomputed_bytes());
    assert!(pool.bytes_held() < held_before, "bytes must come back");
    assert_eq!(pool.total(), admitted - MAX_ATTESTATIONS_PER_ASSET);
    pool.try_insert(weighed(first_asset, 0, &spine))
        .expect("test: room was made");
}

/// F1 — the `spine_point` length cap, proved against the measured attack AND
/// against every value a production producer emits.
///
/// The only thing that constructs a `spine_point` is a mirror naming the
/// `lineage_id` of the spine entry it validated, and `lineage_id` is
/// `hex(proof_hash)` of a BLAKE3 digest — 64 bytes, always. This test takes that
/// value from a REAL chain head rather than asserting it.
#[tokio::test]
async fn s3_4_oversized_spine_point_is_refused_at_the_one_gate() {
    let asset = [0x78u8; 32];
    let (chain, spine, seq) = chain_holding(asset).await;
    let mirror = FalconIdentity::generate();

    // (a) The honest producer's value, straight off the spine.
    assert_eq!(spine.len(), 64, "a lineage_id is hex of a BLAKE3 digest");
    assert!(spine.len() <= MAX_SPINE_POINT_BYTES);
    let honest = attest(&mirror, asset, MatrixIndex::new(1, 0, 0), &spine, seq);
    chain
        .accept_wire_attestation(over_the_wire(&honest))
        .await
        .expect("test: an honest lineage_id must be accepted");

    // (b) Exactly at the cap: still accepted. The cap is inclusive and generous
    //     — four times the only length any producer emits.
    let at_cap = attest(
        &mirror,
        asset,
        MatrixIndex::new(2, 0, 0),
        &"c".repeat(MAX_SPINE_POINT_BYTES),
        seq,
    );
    chain
        .accept_wire_attestation(over_the_wire(&at_cap))
        .await
        .expect("test: the cap is inclusive");

    // (c) The measured attack: 13,400 bytes, the largest that fits the wire cap.
    //     It encodes, it decodes, its envelope is sound and its FALCON signature
    //     is real — the SIZE is what refuses it.
    let attack = attest(
        &mirror,
        asset,
        MatrixIndex::new(3, 0, 0),
        &"S".repeat(13_400),
        seq,
    );
    let framed = encode_mirror_attestation(&attack).expect("test: it fits the wire cap");
    assert!(framed.len() <= MAX_ATTESTATION_WIRE_BYTES);
    let decoded = decode_mirror_attestation(&framed).expect("test: it decodes");
    assert!(decoded.proof_bytes_match() && decoded.binds_to_signer());
    chain
        .accept_wire_attestation(decoded)
        .await
        .expect_err("test: an oversized spine_point must be refused");

    // One byte over the cap is refused for the same reason.
    let over = attest(
        &mirror,
        asset,
        MatrixIndex::new(4, 0, 0),
        &"c".repeat(MAX_SPINE_POINT_BYTES + 1),
        seq,
    );
    chain
        .accept_wire_attestation(over_the_wire(&over))
        .await
        .expect_err("test: one byte over the cap must be refused");

    assert_eq!(chain.mirror_attestation_count(&asset).await, 2);
    // The whole point: 8192 slots can no longer be made 30 KiB each.
    assert!(
        chain.mirror_attestation_bytes().await < 2 * 8 * 1024,
        "two capped attestations must weigh kilobytes, not tens of kilobytes"
    );
}

// ── ORDER (F3) ─────────────────────────────────────────────────────────────

/// F3 — capacity is judged BEFORE the FALCON-1024 verification.
///
/// Stated as a behavioural fact rather than a timing measurement: at capacity, a
/// submission whose signature is GARBAGE is refused with the POOL-FULL
/// diagnosis. It could only report that if the bound was consulted before the
/// signature was. Below capacity the same garbage is refused by the audit gate,
/// which is what makes this evidence of ORDER rather than of the pool swallowing
/// invalid material.
#[tokio::test]
async fn s3_4_capacity_is_judged_before_the_signature() {
    let asset = [0x79u8; 32];
    let (chain, spine, seq) = chain_holding(asset).await;
    let flooder = FalconIdentity::generate();

    // Below capacity: junk is refused by the AUDIT GATE.
    let mut junk = attest(&flooder, asset, MatrixIndex::new(-1, 0, 0), &spine, seq);
    junk.signature.signature = vec![0xAB; 1280];
    let early = chain
        .accept_wire_attestation(over_the_wire(&junk))
        .await
        .expect_err("test: a garbage signature must be refused");
    assert!(
        early.contains("audit gate"),
        "below capacity the signature is what refuses: {early}"
    );

    // Fill this asset's slots with real, valid attestations.
    for cell in 0..MAX_ATTESTATIONS_PER_ASSET as i64 {
        let a = attest(&flooder, asset, MatrixIndex::new(cell, 0, 0), &spine, seq);
        chain
            .accept_wire_attestation(over_the_wire(&a))
            .await
            .expect("test: below the cap");
    }
    assert_eq!(
        chain.mirror_attestation_count(&asset).await,
        MAX_ATTESTATIONS_PER_ASSET
    );

    // At capacity: the SAME garbage is refused by the BOUND, not the signature.
    // A junk submission therefore costs no FALCON verification.
    let at_capacity = chain
        .accept_wire_attestation(over_the_wire(&junk))
        .await
        .expect_err("test: at capacity a newcomer is refused");
    assert!(
        at_capacity.contains("not recorded"),
        "at capacity the BOUND must refuse first: {at_capacity}"
    );
    assert!(
        !at_capacity.contains("audit gate"),
        "no signature work may be spent on a submission the bound already refuses: \
         {at_capacity}"
    );

    // The early probe and the authoritative check are the same rule, so a
    // REPLACEMENT still gets through at capacity — it is not growth, and the
    // probe must not diverge from `try_insert` about that.
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
        .expect("test: the early probe must let a replacement through");
    assert_eq!(
        chain.mirror_attestation_count(&asset).await,
        MAX_ATTESTATIONS_PER_ASSET
    );
}

/// F1 — the reclaim path exists and is reachable: forgetting a foreign chain
/// releases the attestations pooled for it, because the asset is then no longer
/// held and a NEW attestation about it would be refused.
#[tokio::test]
async fn s3_4_forgetting_a_foreign_chain_releases_its_pooled_attestations() {
    let spine_asset = [0x7Au8; 32];
    let (chain, spine, seq) = chain_holding(spine_asset).await;
    let mirror = FalconIdentity::generate();

    // Attest to the SPINE asset — this must survive everything below.
    let on_spine = attest(&mirror, spine_asset, MatrixIndex::new(1, 1, 1), &spine, seq);
    chain
        .accept_wire_attestation(over_the_wire(&on_spine))
        .await
        .expect("test: spine asset attestation");

    // Adopt a foreign chain and attest to THAT asset.
    let foreign_asset = [0x7Bu8; 32];
    let author = FalconIdentity::generate();
    let mut proof = StateProof::new_for_testing();
    proof.stake_proof.stake_holder_id = author.node_id().to_string();
    let mut entry = BlockAssetEntry::new_bound(
        foreign_asset,
        &proof,
        StoragePointer::Genesis,
        blockmatrix::assets::core::AssetRegistration::genesis(coord()),
    );
    entry.set_asset_lineage(None, 0);
    entry.sign_proof(&author).expect("test: FALCON sign");
    chain
        .accept_asset_chain(PresentedAssetChain::new(foreign_asset, vec![entry]))
        .await
        .expect("test: foreign chain adopted");

    let on_foreign = attest(&mirror, foreign_asset, MatrixIndex::new(2, 2, 2), &spine, seq);
    chain
        .accept_wire_attestation(over_the_wire(&on_foreign))
        .await
        .expect("test: foreign asset attestation");
    assert_eq!(chain.mirror_attestation_total().await, 2);
    let charged = chain.mirror_attestation_bytes().await;
    assert!(charged > 0);

    // Forgetting the foreign chain — a LOCAL decision — releases the pool slot
    // for an asset this container no longer holds, and touches nothing else.
    assert_eq!(chain.forget_received_asset_chain(&foreign_asset).await, 1);
    assert_eq!(chain.mirror_attestation_count(&foreign_asset).await, 0);
    assert_eq!(chain.mirror_attestation_count(&spine_asset).await, 1);
    assert_eq!(chain.mirror_attestation_total().await, 1);
    assert!(chain.mirror_attestation_bytes().await < charged);

    // The same attestation is now refused: we do not hold the asset.
    chain
        .accept_wire_attestation(over_the_wire(&on_foreign))
        .await
        .expect_err("test: the asset is no longer held");

    // The explicit operator release works on a spine asset too, and is the only
    // other caller — nothing remote can reach either.
    assert_eq!(chain.clear_mirror_attestations(&spine_asset).await, 1);
    assert_eq!(chain.mirror_attestation_total().await, 0);
    assert_eq!(chain.mirror_attestation_bytes().await, 0);
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
