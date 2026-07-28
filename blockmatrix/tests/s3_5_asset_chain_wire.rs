// Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
//
// D3 proofs for the PRESENTED-ASSET-CHAIN WIRE SURFACE.
//
// D2 produced `accept_asset_chain` — the unified "receive an asset chain"
// operation — but nothing carried a chain to it; only tests invoked it. D3 gives
// it a network ingress modelled on the S3.4 mirror-attestation wire. These tests
// prove the ingress is sound and adds NO second verification.
//
//   TAG       — the chosen byte is 0x55, unused and adjacent to the S3.4
//               mirror-attestation tag 0x54.
//   CODEC     — the `[tag][len][JSON]` framing round-trips, and NO
//               attacker-controlled byte string can panic the decoder (the A2
//               remote-SIGABRT shape). A truncated / oversized / garbage frame
//               is refused by the decoder, never by a slice panic.
//   HANDLER   — a valid presented chain round-trips encode→decode→accept and is
//               adopted, WITHOUT the node spine moving. The handler body IS
//               exactly decode-then-accept, so this path is the handler.
//   REJECT    — a forged-lineage chain and an impostor-signed chain are each
//               refused through the SAME decode→accept path, by
//               `accept_asset_chain`'s one gate — never by a check restated at
//               the wire.
//   AUTH      — the tag is behind the dispatch auth gate, and an unauthenticated
//               peer fails `verify_peer_access`, so it can never reach the accept
//               path. The store is now network-fed; this gate is not optional.
//   INVARIANT — a `PresentedAssetChain` carries entries, never a `Block`; the
//               accept path adopts off-spine and never moves the node's own
//               chain height/head/blocks/index.

use blockmatrix::blockchain::block::{BlockAssetEntry, StoragePointer};
use blockmatrix::blockchain::{
    AcceptReject, LineageBreak, NodeBlockchain, PresentedAssetChain, MAX_RECEIVED_CHAIN_ENTRIES,
};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::asset_chain_wire::{
    decode_presented_asset_chain, encode_presented_asset_chain, AssetChainWireError,
    ASSET_CHAIN_TAG, MAX_ASSET_CHAIN_WIRE_BYTES,
};
use blockmatrix::network::attestation_wire::MIRROR_ATTEST_TAG;
use blockmatrix::network::message_handlers::message_requires_auth;
use blockmatrix::network::peer_auth::{new_authenticated_peers, verify_peer_access};
use hypermesh_lib::NodeSigner;
use trustchain::identity::FalconIdentity;
use trustchain::proof_of_state::StateProof;

fn coord() -> MatrixCoordinate {
    MatrixCoordinate::new(3, 5, 7).expect("test: valid coordinate")
}

/// An entry exactly as a remote producer's `add_block` would emit it:
/// content-bound, lineage-stamped, FALCON-signed, claiming `identity` as author.
fn received_entry(
    identity: &FalconIdentity,
    asset_hash: [u8; 32],
    prev: Option<String>,
    seq: u64,
) -> BlockAssetEntry {
    let mut proof = StateProof::new_for_testing();
    proof.stake_proof.stake_holder_id = identity.node_id().to_string();
    let mut entry = BlockAssetEntry::new_bound(
        asset_hash,
        &proof,
        StoragePointer::Genesis,
        blockmatrix::assets::core::AssetRegistration::genesis(coord()),
    );
    entry.set_asset_lineage(prev, seq);
    entry.sign_proof(identity).expect("test: FALCON sign");
    entry
}

/// A well-formed presented chain of `len` entries for `asset`, authored by `by`.
fn presented_chain(by: &FalconIdentity, asset: [u8; 32], len: usize) -> PresentedAssetChain {
    let mut entries: Vec<BlockAssetEntry> = Vec::new();
    for seq in 0..len as u64 {
        let prev = entries.last().map(BlockAssetEntry::lineage_id);
        entries.push(received_entry(by, asset, prev, seq));
    }
    PresentedAssetChain::new(asset, entries)
}

/// The EXACT work `handle_asset_chain` performs: frame the presentation, decode
/// it off the wire, and feed the decoded value straight into the one accept
/// gate. The handler has no other logic — no second verification list — so this
/// is the full network receive path a peer's presentation travels.
async fn over_the_wire_then_accept(
    chain: &NodeBlockchain,
    presented: &PresentedAssetChain,
) -> Result<blockmatrix::blockchain::AcceptReceipt, AcceptReject> {
    let framed = encode_presented_asset_chain(presented).expect("test: encode");
    let decoded = decode_presented_asset_chain(&framed).expect("test: decode");
    chain.accept_asset_chain(decoded).await
}

// ── TAG ──────────────────────────────────────────────────────────────────────

#[test]
fn s3_5_asset_chain_tag_is_unused_and_adjacent_to_mirror_attest() {
    assert_eq!(ASSET_CHAIN_TAG, 0x55, "the chosen tag byte");
    assert_eq!(MIRROR_ATTEST_TAG, 0x54, "the S3.4 tag it sits next to");
    assert_ne!(
        ASSET_CHAIN_TAG, MIRROR_ATTEST_TAG,
        "the two per-asset receive surfaces must not collide"
    );
}

// ── CODEC ──────────────────────────────────────────────────────────────────

#[test]
fn s3_5_asset_chain_wire_roundtrips() {
    let author = FalconIdentity::generate();
    let asset = [0x51u8; 32];
    let presented = presented_chain(&author, asset, 3);

    let framed = encode_presented_asset_chain(&presented).expect("test: encode");
    assert_eq!(framed[0], ASSET_CHAIN_TAG);
    assert_eq!(
        decode_presented_asset_chain(&framed).expect("test: decode"),
        presented
    );
}

/// The A2 shape: a length prefix trusted into a slice expression is a
/// remote-triggerable abort. Nothing here may panic on any input.
#[test]
fn s3_5_asset_chain_wire_never_panics_on_hostile_bytes() {
    let author = FalconIdentity::generate();
    let good =
        encode_presented_asset_chain(&presented_chain(&author, [0x52u8; 32], 2)).expect("test");

    // Every truncation of a valid message is refused, never panics.
    for cut in 0..good.len() {
        assert!(decode_presented_asset_chain(&good[..cut]).is_err());
    }

    // A declared length beyond the cap.
    let mut huge = good.clone();
    huge[1..5].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        decode_presented_asset_chain(&huge),
        Err(AssetChainWireError::TooLarge { .. })
    ));

    // A declared length beyond what is present but under the cap.
    let mut lying = good.clone();
    lying[1..5].copy_from_slice(&((MAX_ASSET_CHAIN_WIRE_BYTES - 1) as u32).to_le_bytes());
    assert!(matches!(
        decode_presented_asset_chain(&lying),
        Err(AssetChainWireError::LengthMismatch { .. })
    ));

    // Wrong tag, empty input, framed garbage.
    let mut wrong = good.clone();
    wrong[0] = 0x00;
    assert!(matches!(
        decode_presented_asset_chain(&wrong),
        Err(AssetChainWireError::WrongTag { got: 0x00 })
    ));
    assert!(decode_presented_asset_chain(&[]).is_err());
    for seed in 0..64u8 {
        let mut noise = vec![ASSET_CHAIN_TAG];
        noise.extend_from_slice(&32u32.to_le_bytes());
        noise.extend_from_slice(&[seed; 32]);
        assert!(decode_presented_asset_chain(&noise).is_err());
    }
}

/// Deterministic xorshift64* — a fuzz corpus reproducible from its seed.
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

/// A REAL adversarial pass over `decode_presented_asset_chain`. The shape being
/// ruled out is the A2 shard-locate parser's remote `SIGABRT`: a length prefix
/// trusted into a slice. Any panic or abort here fails the test process.
#[test]
fn s3_5_asset_chain_wire_survives_a_real_fuzz_pass() {
    const RANDOM_CASES: usize = 200_000;

    let author = FalconIdentity::generate();
    let good =
        encode_presented_asset_chain(&presented_chain(&author, [0x5Au8; 32], 2)).expect("test");
    assert!(good.len() > 2000, "the fuzz corpus must be realistically sized");

    let mut refused = 0usize;
    let mut decoded_ok = 0usize;
    let mut note = |ok: bool| {
        if ok {
            decoded_ok += 1;
        } else {
            refused += 1;
        }
    };

    // (1) Every truncation; only the full message may decode.
    let mut truncations_decoded = 0usize;
    for cut in 0..=good.len() {
        let ok = decode_presented_asset_chain(&good[..cut]).is_ok();
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
        let out = decode_presented_asset_chain(&framed);
        if tag == ASSET_CHAIN_TAG {
            assert!(out.is_ok(), "the real tag must still decode");
        } else {
            assert_eq!(out, Err(AssetChainWireError::WrongTag { got: tag }));
        }
        note(out.is_ok());
    }

    // (3) Lying lengths: extremes, cap boundary, every power of two.
    let mut lengths: Vec<u32> = vec![
        0,
        1,
        u32::MAX,
        u32::MAX - 1,
        MAX_ASSET_CHAIN_WIRE_BYTES as u32,
        (MAX_ASSET_CHAIN_WIRE_BYTES as u32).wrapping_add(1),
        (MAX_ASSET_CHAIN_WIRE_BYTES as u32).wrapping_sub(1),
        good.len() as u32,
        (good.len() as u32).wrapping_sub(5),
        i32::MAX as u32,
    ];
    lengths.extend((0..32).map(|shift| 1u32 << shift));
    for declared in lengths {
        let mut framed = good.clone();
        framed[1..5].copy_from_slice(&declared.to_le_bytes());
        note(decode_presented_asset_chain(&framed).is_ok());
        for body in [0usize, 1, 4, 37] {
            let mut short = vec![ASSET_CHAIN_TAG];
            short.extend_from_slice(&declared.to_le_bytes());
            short.extend(std::iter::repeat(0xABu8).take(body));
            note(decode_presented_asset_chain(&short).is_ok());
        }
    }

    // (4) Single-bit flips across the whole valid message.
    let mut prng = Prng(0x5EED_D3D3_ABCD_0001);
    for _ in 0..20_000 {
        let mut framed = good.clone();
        let at = prng.below(framed.len());
        framed[at] ^= 1u8 << (prng.byte() % 8);
        note(decode_presented_asset_chain(&framed).is_ok());
    }

    // (5) Bulk random payloads, framed and unframed.
    for _ in 0..RANDOM_CASES {
        let len = prng.below(96);
        let mut noise: Vec<u8> = (0..len).map(|_| prng.byte()).collect();
        note(decode_presented_asset_chain(&noise).is_ok());

        let body_len = prng.below(64);
        noise.clear();
        noise.push(ASSET_CHAIN_TAG);
        noise.extend_from_slice(&(body_len as u32).to_le_bytes());
        noise.extend((0..body_len).map(|_| prng.byte()));
        note(decode_presented_asset_chain(&noise).is_ok());
    }

    // Reaching here IS the result: zero panics, zero aborts. The decoder does NO
    // verification, so a bit flipped inside a JSON string may still decode — what
    // must never decode is a truncated, wrong-tag, or length-lying frame, each
    // asserted individually above.
    assert!(
        refused > RANDOM_CASES,
        "fuzz corpus too small to be evidence: {refused} refusals"
    );
    println!(
        "fuzz: {} inputs driven, {refused} refused, {decoded_ok} decoded, zero panics/aborts",
        refused + decoded_ok
    );
}

#[test]
fn s3_5_oversized_chain_is_refused_at_both_ends() {
    // A frame whose declared length exceeds the wire cap is refused on decode
    // before any body is sliced — the allocation lever is closed.
    let mut framed = vec![ASSET_CHAIN_TAG];
    framed.extend_from_slice(&((MAX_ASSET_CHAIN_WIRE_BYTES + 1) as u32).to_le_bytes());
    assert!(matches!(
        decode_presented_asset_chain(&framed),
        Err(AssetChainWireError::TooLarge {
            declared,
            limit,
        }) if declared == MAX_ASSET_CHAIN_WIRE_BYTES + 1 && limit == MAX_ASSET_CHAIN_WIRE_BYTES
    ));
}

// ── HANDLER: the accept-happy path over the wire ────────────────────────────

#[tokio::test]
async fn s3_5_valid_presented_chain_round_trips_and_is_adopted_without_moving_the_spine() {
    let chain = NodeBlockchain::new(coord());
    let author = FalconIdentity::generate();
    let asset = [0x53u8; 32];

    let height_before = chain.get_height().await;
    let head_before = chain.get_head().await.expect("test: head").hash;
    let blocks_before = chain.get_stats().await.total_blocks;
    let indexed_before = chain.indexed_asset_count().await;
    assert!(!chain.holds_asset(&asset).await);

    let receipt = over_the_wire_then_accept(&chain, &presented_chain(&author, asset, 3))
        .await
        .expect("test: a valid presented chain must be adopted over the wire");
    assert_eq!(receipt.entries, 3);
    assert_eq!(receipt.added, 3);
    assert_eq!(receipt.head_seq, 2);

    // THE INVARIANT: the node's own block chain did not move. A wire-received
    // asset chain became an off-spine received title, never a spine block.
    assert_eq!(chain.get_height().await, height_before);
    assert_eq!(chain.get_head().await.expect("test: head").hash, head_before);
    assert_eq!(chain.get_stats().await.total_blocks, blocks_before);
    assert_eq!(chain.indexed_asset_count().await, indexed_before);

    // ...and the adopted title is queryable and self-verifying.
    let lineage = chain
        .received_asset_lineage(&asset)
        .await
        .expect("test: adopted chain is queryable");
    assert_eq!(lineage.verify(), Ok(()));
    assert!(chain.holds_asset(&asset).await);
    assert!(!chain.has_ever_seen_asset(&asset).await);
}

// ── REJECT: the same wire path, refused by the one gate ─────────────────────

#[tokio::test]
async fn s3_5_forged_lineage_is_rejected_through_the_wire_path() {
    let chain = NodeBlockchain::new(coord());
    let author = FalconIdentity::generate();
    let asset = [0x54u8; 32];

    // Entry 2 names a predecessor that is not entry 1 — a forged prev-pointer.
    let mut forged = presented_chain(&author, asset, 3);
    forged.entries[2] = received_entry(&author, asset, Some("de".repeat(32)), 2);

    assert!(matches!(
        over_the_wire_then_accept(&chain, &forged).await,
        Err(AcceptReject::LineageBroken(
            LineageBreak::PrevPointerMismatch { position: 2, .. }
        ))
    ));
    // A refused chain leaves NO partial state behind.
    assert!(!chain.has_received_asset_chain(&asset).await);
    assert_eq!(chain.received_chain_bytes().await, 0);
}

#[tokio::test]
async fn s3_5_impostor_signed_chain_is_rejected_through_the_wire_path() {
    let chain = NodeBlockchain::new(coord());
    let author = FalconIdentity::generate();
    let impostor = FalconIdentity::generate();
    let asset = [0x55u8; 32];

    // A real, valid FALCON signature over a proof that names the AUTHOR, but
    // produced by the impostor's key — the signer does not derive the claimed
    // author. The wire adds no check of its own; `accept_asset_chain` refuses it.
    let mut chain_p = presented_chain(&author, asset, 2);
    let mut proof = chain_p.entries[1].state_proof.clone();
    proof.stake_proof.stake_holder_id = author.node_id().to_string();
    let mut entry = BlockAssetEntry::new_bound(
        asset,
        &proof,
        StoragePointer::Genesis,
        blockmatrix::assets::core::AssetRegistration::genesis(coord()),
    );
    entry.set_asset_lineage(Some(chain_p.entries[0].lineage_id()), 1);
    entry.sign_proof(&impostor).expect("test: FALCON sign");
    assert!(entry.verify_signed_proof().is_ok(), "the signature itself is valid");
    chain_p.entries[1] = entry;

    assert_eq!(
        over_the_wire_then_accept(&chain, &chain_p).await,
        Err(AcceptReject::SignerNotAuthor { position: 1 })
    );
    assert!(!chain.has_received_asset_chain(&asset).await);
    assert_eq!(chain.received_chain_bytes().await, 0);
}

#[tokio::test]
async fn s3_5_an_oversized_presentation_is_refused_before_the_signature_work() {
    let chain = NodeBlockchain::new(coord());
    let author = FalconIdentity::generate();
    let asset = [0x56u8; 32];

    // One cheap entry cloned past the entry cap: the accept path judges the
    // entry-count bound BEFORE any FALCON verification.
    let one = presented_chain(&author, asset, 1);
    let flooded = PresentedAssetChain::new(
        asset,
        vec![one.entries[0].clone(); MAX_RECEIVED_CHAIN_ENTRIES + 1],
    );
    assert_eq!(
        over_the_wire_then_accept(&chain, &flooded).await,
        Err(AcceptReject::TooLong {
            presented: MAX_RECEIVED_CHAIN_ENTRIES + 1,
            limit: MAX_RECEIVED_CHAIN_ENTRIES,
        })
    );
    assert!(!chain.has_received_asset_chain(&asset).await);
}

// ── AUTH: an unauthenticated peer never reaches the accept path ─────────────

#[test]
fn s3_5_asset_chain_tag_is_behind_the_dispatch_auth_gate() {
    // The single source of truth for the gate says this tag requires auth.
    assert!(
        message_requires_auth(ASSET_CHAIN_TAG),
        "the accept path is network-fed — its tag MUST be auth-gated"
    );
    // A tag that is not gated, for contrast — the predicate is not vacuously true.
    assert!(!message_requires_auth(0xFF));
}

#[tokio::test]
async fn s3_5_an_unauthenticated_peer_fails_the_gate_that_guards_the_accept_path() {
    // The dispatch gate is `message_requires_auth(tag) && verify_peer_access(..)`.
    // For the asset-chain tag the first half is true (asserted above), so the
    // second half is what stands between a connection and the accept path. An
    // empty authenticated-peers map is exactly an unauthenticated connection.
    let peers = new_authenticated_peers();
    assert!(
        !verify_peer_access(&peers, "an-unauthenticated-peer", "net").await,
        "an unauthenticated peer must fail the gate, so it never reaches accept_asset_chain"
    );

    // Requiring auth AND failing the access check ⟹ the dispatcher returns
    // before the handler. The two facts together are the refusal.
    assert!(message_requires_auth(ASSET_CHAIN_TAG));
}
