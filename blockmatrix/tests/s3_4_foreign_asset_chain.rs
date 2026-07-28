// Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
//
// S3.4 proofs for the THIRD ACCEPT MODE — a foreign asset's verified sub-chain.
//
//   ACCEPT       — a valid, signed, internally-consistent foreign asset-chain is
//                  adopted, and the NODE SPINE does not move: height, head hash,
//                  block count and the S3.1 asset index are all unchanged.
//   FORGERY      — a forged prev-pointer, a sequence gap, and a chain whose root
//                  is not an asset-genesis are each REJECTED.
//   SIGNER       — an unsigned entry, an entry signed by a key that does not
//                  derive its claimed author, and an entry whose proof was
//                  altered after signing are each REJECTED.
//   DOMAINS      — the container-spine rejection domain is UNCHANGED. After a
//                  foreign asset-chain is adopted, a BLOCK carrying the very
//                  same entries is still hard-rejected by `insert_received_block`
//                  (F7) or buffered as an orphan; it never reaches the spine.
//   NON-DESTRUCT — device genesis and previously-adopted chains survive every
//                  accept; a spine-held asset cannot be shadowed; an adopted
//                  chain may only be EXTENDED, never replaced or truncated.
//   BOUND        — the off-spine store is capped and refuses NEW chains at
//                  capacity without evicting anything already adopted.

use blockmatrix::blockchain::block::{Block, BlockAssetEntry, StoragePointer};
use blockmatrix::blockchain::{
    chain_footprint_bytes, entry_footprint_bytes, ForeignAssetChain, ForeignChainReject,
    ForeignChainStore, LineageBreak, NodeBlockchain, StoreBound, MAX_FOREIGN_CHAINS,
    MAX_FOREIGN_CHAIN_ENTRIES, MAX_FOREIGN_STORE_BYTES,
};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use hypermesh_lib::NodeSigner;
use trustchain::identity::FalconIdentity;
use trustchain::proof_of_state::StateProof;

fn coord() -> MatrixCoordinate {
    MatrixCoordinate::new(3, 5, 7).expect("test: valid coordinate")
}

/// An entry exactly as a FOREIGN producer's `add_block` would emit it:
/// content-bound, lineage-stamped, FALCON-signed, claiming `identity` as author.
fn foreign_entry(
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

/// A well-formed foreign chain of `len` entries for `asset`, authored by `by`.
fn foreign_chain(by: &FalconIdentity, asset: [u8; 32], len: usize) -> ForeignAssetChain {
    let mut entries: Vec<BlockAssetEntry> = Vec::new();
    for seq in 0..len as u64 {
        let prev = entries.last().map(BlockAssetEntry::lineage_id);
        entries.push(foreign_entry(by, asset, prev, seq));
    }
    ForeignAssetChain::new(asset, entries)
}

/// A locally-appendable (unsigned) entry — the shape S3.2's tests use for a
/// signer-less test chain.
fn local_entry(asset_hash: [u8; 32]) -> BlockAssetEntry {
    BlockAssetEntry::new_bound(
        asset_hash,
        &StateProof::new_for_testing(),
        StoragePointer::Genesis,
        blockmatrix::assets::core::AssetRegistration::genesis(coord()),
    )
}

/// Everything about the node spine that an accept must leave alone.
#[derive(Debug, PartialEq)]
struct SpineSnapshot {
    height: u64,
    head_hash: String,
    total_blocks: u64,
    indexed_assets: usize,
    asset_index: blockmatrix::blockchain::AssetChainIndex,
}

async fn snapshot(chain: &NodeBlockchain) -> SpineSnapshot {
    SpineSnapshot {
        height: chain.get_height().await,
        head_hash: chain.get_head().await.expect("test: head").hash,
        total_blocks: chain.get_stats().await.total_blocks,
        indexed_assets: chain.indexed_asset_count().await,
        asset_index: chain.asset_index_snapshot().await,
    }
}

// ── ACCEPT ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_4_valid_foreign_chain_is_accepted_and_the_spine_does_not_move() {
    let chain = NodeBlockchain::new(coord());
    let stranger = FalconIdentity::generate();
    let asset = [0x41u8; 32];

    let before = snapshot(&chain).await;
    assert!(!chain.holds_asset(&asset).await);

    let receipt = chain
        .accept_foreign_asset_chain(foreign_chain(&stranger, asset, 3))
        .await
        .expect("test: a valid foreign asset-chain must be accepted");

    assert_eq!(receipt.entries, 3);
    assert_eq!(receipt.added, 3);
    assert_eq!(receipt.head_seq, 2);

    // THE POINT: the node spine is byte-for-byte where it was.
    assert_eq!(
        snapshot(&chain).await,
        before,
        "adopting a foreign asset-chain must not move the node spine"
    );

    // ...and the imported title is queryable and self-verifying.
    let lineage = chain
        .foreign_asset_lineage(&asset)
        .await
        .expect("test: adopted chain is queryable");
    assert_eq!(lineage.sequence(), vec![0, 1, 2]);
    assert_eq!(lineage.verify(), Ok(()));
    assert_eq!(chain.asset_lineage_any(&asset).await, lineage);
    assert!(chain.holds_asset(&asset).await);
    assert_eq!(chain.foreign_chain_count().await, 1);

    // The SPINE's own per-asset view still knows nothing about it — imported
    // history is not local title.
    assert!(chain.asset_lineage(&asset).await.is_empty());
    assert!(!chain.has_ever_seen_asset(&asset).await);
}

#[tokio::test]
async fn s3_4_empty_and_oversized_presentations_are_refused() {
    let chain = NodeBlockchain::new(coord());
    let stranger = FalconIdentity::generate();

    assert_eq!(
        chain
            .accept_foreign_asset_chain(ForeignAssetChain::new([0x42u8; 32], vec![]))
            .await,
        Err(ForeignChainReject::Empty)
    );

    // Over-length is judged BEFORE any signature work: one cheap entry cloned
    // past the cap is enough to prove the ordering.
    let one = foreign_chain(&stranger, [0x43u8; 32], 1);
    let flooded = ForeignAssetChain::new(
        [0x43u8; 32],
        vec![one.entries[0].clone(); MAX_FOREIGN_CHAIN_ENTRIES + 1],
    );
    assert_eq!(
        chain.accept_foreign_asset_chain(flooded).await,
        Err(ForeignChainReject::TooLong {
            presented: MAX_FOREIGN_CHAIN_ENTRIES + 1,
            limit: MAX_FOREIGN_CHAIN_ENTRIES,
        })
    );
}

// ── FORGERY ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_4_forged_lineage_is_rejected() {
    let stranger = FalconIdentity::generate();

    // (a) FORGED PREV-POINTER — entry 2 names a predecessor that is not entry 1.
    let chain = NodeBlockchain::new(coord());
    let asset = [0x44u8; 32];
    let mut forged = foreign_chain(&stranger, asset, 3);
    forged.entries[2] = foreign_entry(&stranger, asset, Some("de".repeat(32)), 2);
    assert!(matches!(
        chain.accept_foreign_asset_chain(forged).await,
        Err(ForeignChainReject::LineageBroken(
            LineageBreak::PrevPointerMismatch { position: 2, .. }
        ))
    ));
    assert!(!chain.has_foreign_asset_chain(&asset).await);

    // (b) SEQUENCE GAP — correct prev-pointer, skipped sequence number.
    let chain = NodeBlockchain::new(coord());
    let asset = [0x45u8; 32];
    let mut gapped = foreign_chain(&stranger, asset, 3);
    let prev = gapped.entries[1].lineage_id();
    gapped.entries[2] = foreign_entry(&stranger, asset, Some(prev), 9);
    assert!(matches!(
        gapped.entries[2].asset_seq(),
        9,
    ));
    assert!(matches!(
        chain.accept_foreign_asset_chain(gapped).await,
        Err(ForeignChainReject::LineageBroken(LineageBreak::SequenceGap {
            position: 2,
            claimed: 9,
            expected: 2,
        }))
    ));
    assert!(!chain.has_foreign_asset_chain(&asset).await);

    // (c) ROOT IS NOT AN ASSET-GENESIS — a chain presented mid-history, which
    //     is exactly the "unverifiable provenance" S3.2 refused to guess at.
    let chain = NodeBlockchain::new(coord());
    let asset = [0x46u8; 32];
    let full = foreign_chain(&stranger, asset, 3);
    let truncated = ForeignAssetChain::new(asset, full.entries[1..].to_vec());
    assert!(matches!(
        chain.accept_foreign_asset_chain(truncated).await,
        Err(ForeignChainReject::LineageBroken(
            LineageBreak::RootIsNotAssetGenesis { .. }
        ))
    ));
    assert!(!chain.has_foreign_asset_chain(&asset).await);

    // (d) WRONG ASSET — an entry for a different asset smuggled into the run.
    let chain = NodeBlockchain::new(coord());
    let asset = [0x47u8; 32];
    let mut mixed = foreign_chain(&stranger, asset, 2);
    mixed.entries[1] = foreign_entry(
        &stranger,
        [0x48u8; 32],
        Some(mixed.entries[0].lineage_id()),
        1,
    );
    assert!(matches!(
        chain.accept_foreign_asset_chain(mixed).await,
        Err(ForeignChainReject::LineageBroken(LineageBreak::WrongAsset {
            position: 1
        }))
    ));
}

// ── SIGNER ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_4_bad_signers_are_rejected() {
    let stranger = FalconIdentity::generate();
    let impostor = FalconIdentity::generate();

    // (a) UNSIGNED — no envelope at all. Unlike the spine accept mode there is
    //     NO legacy-migration escape hatch here, and setting the spine's compat
    //     flag must not open one.
    std::env::set_var("HYPERMESH_ACCEPT_UNSIGNED_BLOCKS", "1");
    let chain = NodeBlockchain::new(coord());
    let asset = [0x49u8; 32];
    let mut unsigned = foreign_chain(&stranger, asset, 2);
    unsigned.entries[1].signed_proof = None;
    assert_eq!(
        chain.accept_foreign_asset_chain(unsigned).await,
        Err(ForeignChainReject::Unsigned { position: 1 }),
        "the spine's one-release legacy flag must not admit an unsigned foreign import"
    );
    std::env::remove_var("HYPERMESH_ACCEPT_UNSIGNED_BLOCKS");
    assert!(!chain.has_foreign_asset_chain(&asset).await);

    // (b) SIGNER IS NOT THE CLAIMED AUTHOR — a real, valid FALCON signature over
    //     a proof that names somebody ELSE as its author.
    let chain = NodeBlockchain::new(coord());
    let asset = [0x4Au8; 32];
    let mut wrong_author = foreign_chain(&stranger, asset, 2);
    let mut proof = wrong_author.entries[1].state_proof.clone();
    proof.stake_proof.stake_holder_id = stranger.node_id().to_string();
    let mut entry = BlockAssetEntry::new_bound(
        asset,
        &proof,
        StoragePointer::Genesis,
        blockmatrix::assets::core::AssetRegistration::genesis(coord()),
    );
    entry.set_asset_lineage(Some(wrong_author.entries[0].lineage_id()), 1);
    // Signed by the IMPOSTOR while still claiming the stranger as author.
    entry.sign_proof(&impostor).expect("test: FALCON sign");
    assert!(entry.verify_signed_proof().is_ok(), "the signature itself is valid");
    wrong_author.entries[1] = entry;
    assert_eq!(
        chain.accept_foreign_asset_chain(wrong_author).await,
        Err(ForeignChainReject::SignerNotAuthor { position: 1 })
    );
    assert!(!chain.has_foreign_asset_chain(&asset).await);

    // (c) PROOF ALTERED AFTER SIGNING — the envelope no longer wraps the proof
    //     the entry carries.
    let chain = NodeBlockchain::new(coord());
    let asset = [0x4Bu8; 32];
    let mut tampered = foreign_chain(&stranger, asset, 2);
    if let Some(wire) = tampered.entries[0].signed_proof.as_mut() {
        wire.nonce[0] ^= 0xFF;
    }
    assert!(matches!(
        chain.accept_foreign_asset_chain(tampered).await,
        Err(ForeignChainReject::BadSignature { position: 0, .. })
    ));
    assert!(!chain.has_foreign_asset_chain(&asset).await);
}

// ── DOMAINS: the container spine keeps its own rejection rule ───────────────

#[tokio::test]
async fn s3_4_accepting_a_foreign_chain_opens_no_path_onto_the_node_spine() {
    let chain = NodeBlockchain::new(coord());
    let stranger = FalconIdentity::generate();
    let asset = [0x4Cu8; 32];

    let foreign = foreign_chain(&stranger, asset, 2);
    chain
        .accept_foreign_asset_chain(foreign.clone())
        .await
        .expect("test: accepted off-spine");
    let after_accept = snapshot(&chain).await;

    // The producer's OWN block, carrying the exact entries we just adopted,
    // presented to the container-spine accept mode. Its `previous_hash` is the
    // producer's genesis, which we do not hold.
    let graft = Block::new(1, foreign.entries.clone(), "ff".repeat(32));
    let error = chain
        .insert_received_block(graft)
        .await
        .expect_err("test: F7 must still hard-reject a foreign block at index 1");
    assert!(
        error.contains("no chain graft"),
        "expected the F7 hard reject, got: {error}"
    );

    // A block at an index we do not hold is BUFFERED, not inserted — the other
    // half of the spine rule, also unchanged.
    let far = Block::new(9, foreign.entries.clone(), "ee".repeat(32));
    chain
        .insert_received_block(far)
        .await
        .expect("test: an unlinkable block is buffered, not an error");
    assert_eq!(chain.orphan_count().await, 1);
    assert!(chain.get_block(9).await.is_none());

    assert_eq!(
        snapshot(&chain).await,
        after_accept,
        "neither spine path may move the spine on account of an adopted asset"
    );
}

#[tokio::test]
async fn s3_4_a_foreign_chain_cannot_shadow_a_spine_asset() {
    let chain = NodeBlockchain::new(coord());
    let stranger = FalconIdentity::generate();
    let asset = [0x4Du8; 32];

    chain
        .add_block(vec![local_entry(asset)])
        .await
        .expect("test: local append");

    assert_eq!(
        chain
            .accept_foreign_asset_chain(foreign_chain(&stranger, asset, 3))
            .await,
        Err(ForeignChainReject::AlreadyOnSpine),
        "an import must not offer a second opinion about an asset the spine holds"
    );

    // Still true after the spine's entries are pruned away: F1's tombstone
    // keeps the asset OURS.
    chain.prune_to_headers(1..2).await;
    assert!(chain.asset_lineage(&asset).await.is_empty());
    assert_eq!(
        chain
            .accept_foreign_asset_chain(foreign_chain(&stranger, asset, 3))
            .await,
        Err(ForeignChainReject::AlreadyOnSpine)
    );
}

// ── NON-DESTRUCTIVE ────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_4_adoption_is_non_destructive() {
    let chain = NodeBlockchain::new(coord());
    let stranger = FalconIdentity::generate();
    let genesis = chain.get_block(0).await.expect("test: device genesis");
    let local_asset = [0x4Eu8; 32];

    chain
        .add_block(vec![local_entry(local_asset)])
        .await
        .expect("test: local append");
    let before = snapshot(&chain).await;

    for tag in 0x60u8..0x66 {
        chain
            .accept_foreign_asset_chain(foreign_chain(&stranger, [tag; 32], 2))
            .await
            .expect("test: accepted");
    }

    // Device genesis survives, identically.
    assert_eq!(
        chain.get_block(0).await.expect("test: genesis still held"),
        genesis
    );
    // The locally-titled asset survives, with its lineage intact.
    assert_eq!(chain.asset_lineage(&local_asset).await.verify(), Ok(()));
    assert_eq!(snapshot(&chain).await, before);
    // Every adopted chain coexists — an accept adds, it never clears.
    assert_eq!(chain.foreign_chain_count().await, 6);
    for tag in 0x60u8..0x66 {
        assert!(chain.has_foreign_asset_chain(&[tag; 32]).await);
    }
}

#[tokio::test]
async fn s3_4_an_adopted_chain_may_only_be_extended() {
    let chain = NodeBlockchain::new(coord());
    let stranger = FalconIdentity::generate();
    let asset = [0x4Fu8; 32];

    // ONE history, presented in growing prefixes. (Every entry carries its own
    // freshness nonce, so two independently built runs are genuinely two
    // different histories — which the conflict case below relies on.)
    let full = foreign_chain(&stranger, asset, 5);
    let three = ForeignAssetChain::new(asset, full.entries[..3].to_vec());

    chain
        .accept_foreign_asset_chain(three.clone())
        .await
        .expect("test: accepted");

    // Re-presenting the same prefix is idempotent: nothing added.
    let receipt = chain
        .accept_foreign_asset_chain(three.clone())
        .await
        .expect("test: re-presentation accepted");
    assert_eq!(receipt.added, 0);
    assert_eq!(receipt.entries, 3);

    // Genuine extension: same history, two more entries.
    let receipt = chain
        .accept_foreign_asset_chain(full)
        .await
        .expect("test: extension accepted");
    assert_eq!(receipt.added, 2);
    assert_eq!(receipt.head_seq, 4);

    // Truncation is not an update.
    assert_eq!(
        chain.accept_foreign_asset_chain(three).await,
        Err(ForeignChainReject::NotAnExtension {
            held: 5,
            presented: 3,
        })
    );

    // A DIFFERENT history of the same length, from a different author, is a
    // conflict — an adopted title is never silently replaced.
    let rival = FalconIdentity::generate();
    assert_eq!(
        chain
            .accept_foreign_asset_chain(foreign_chain(&rival, asset, 5))
            .await,
        Err(ForeignChainReject::Conflict { position: 0 })
    );
    assert_eq!(
        chain
            .foreign_asset_lineage(&asset)
            .await
            .expect("test: still held")
            .len(),
        5,
        "the rival history must not have displaced the adopted one"
    );
}

// ── BOUND ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_4_foreign_store_bound_holds_and_evicts_nothing() {
    let chain = NodeBlockchain::new(coord());
    // One identity, many assets: the store is keyed by asset, so a single
    // keypair is enough to fill it — which is exactly why it must be bounded.
    let stranger = FalconIdentity::generate();

    let mut first: Option<ForeignAssetChain> = None;
    for n in 0..MAX_FOREIGN_CHAINS {
        let mut asset = [0u8; 32];
        asset[..8].copy_from_slice(&(n as u64).to_le_bytes());
        let presented = foreign_chain(&stranger, asset, 2);
        if first.is_none() {
            // Adopt only the FIRST entry of this one, so the same history is
            // available later as a genuine extension.
            chain
                .accept_foreign_asset_chain(ForeignAssetChain::new(
                    asset,
                    presented.entries[..1].to_vec(),
                ))
                .await
                .expect("test: accepted below capacity");
            first = Some(presented);
            continue;
        }
        chain
            .accept_foreign_asset_chain(presented)
            .await
            .expect("test: accepted below capacity");
    }
    assert_eq!(chain.foreign_chain_count().await, MAX_FOREIGN_CHAINS);

    // At capacity a NEW chain is refused...
    let overflow = [0xFFu8; 32];
    assert_eq!(
        chain
            .accept_foreign_asset_chain(foreign_chain(&stranger, overflow, 1))
            .await,
        Err(ForeignChainReject::StoreFull(StoreBound::Chains {
            held: MAX_FOREIGN_CHAINS,
            limit: MAX_FOREIGN_CHAINS,
        }))
    );
    assert!(!chain.has_foreign_asset_chain(&overflow).await);

    // ...and NOTHING already adopted was evicted to make room.
    assert_eq!(chain.foreign_chain_count().await, MAX_FOREIGN_CHAINS);
    let first = first.expect("test: at least one chain");
    assert!(chain.has_foreign_asset_chain(&first.asset_hash).await);

    // Extending an ALREADY-held chain is still admitted at capacity: it takes
    // no new slot.
    let first_asset = first.asset_hash;
    chain
        .accept_foreign_asset_chain(first)
        .await
        .expect("test: extension of a held chain is admitted at capacity");

    // Space is reclaimed only by a LOCAL decision, never by traffic.
    assert_eq!(chain.forget_foreign_asset_chain(&first_asset).await, 2);
    chain
        .accept_foreign_asset_chain(foreign_chain(&stranger, overflow, 1))
        .await
        .expect("test: room was made explicitly");
}

// ── A2: THE BOUND IS A BYTE BUDGET ─────────────────────────────────────────

/// R13's minimum device spec: 4 GB RAM. A bound documented as memory-exhaustion
/// protection that a conforming device cannot survive is not protection.
const R13_MIN_RAM_BYTES: usize = 4 * 1024 * 1024 * 1024;

#[tokio::test]
async fn s3_4_the_store_bound_is_a_byte_budget_that_fits_r13() {
    let stranger = FalconIdentity::generate();
    let entry = foreign_entry(&stranger, [0x11u8; 32], None, 0);

    // Runtime measurement of a REAL, FALCON-signed, content-bound entry — the
    // exact shape the wire will carry.
    let footprint = entry_footprint_bytes(&entry);
    println!("measured BlockAssetEntry footprint: {footprint} bytes");
    assert!(
        (2_048..=32_768).contains(&footprint),
        "a signed entry should be a few KiB (FALCON pubkey 1793 B + signature ~1280 B \
         + the StateProof held twice); measured {footprint}"
    );

    // THE DEFECT, quantified: the count caps alone do not bound this store.
    let counts_only = MAX_FOREIGN_CHAINS * MAX_FOREIGN_CHAIN_ENTRIES * footprint;
    println!(
        "count-caps-only worst case: {counts_only} bytes ({} MiB)",
        counts_only / (1024 * 1024)
    );
    assert!(
        counts_only > R13_MIN_RAM_BYTES / 2,
        "this assertion documents WHY the byte budget exists: {MAX_FOREIGN_CHAINS} x \
         {MAX_FOREIGN_CHAIN_ENTRIES} x {footprint} = {counts_only} bytes — more than HALF \
         a minimum-spec node's entire RAM, for an off-spine cache. That is the figure \
         that must NOT be the store's effective bound"
    );

    // THE FIX: the byte budget is the binding bound, and it fits R13 with the
    // rest of the node's working set still in play.
    assert!(
        MAX_FOREIGN_STORE_BYTES < counts_only,
        "the byte budget must bind before the count caps ever could"
    );
    assert!(
        MAX_FOREIGN_STORE_BYTES <= R13_MIN_RAM_BYTES / 32,
        "the off-spine store must not claim more than ~3% of a minimum-spec node's RAM; \
         {MAX_FOREIGN_STORE_BYTES} vs {R13_MIN_RAM_BYTES}"
    );

    // And it is ENFORCED at the byte boundary, not merely declared.
    let store = ForeignChainStore::new();
    assert_eq!(store.bytes_held(), 0);
    assert_eq!(store.admission_check(true, MAX_FOREIGN_STORE_BYTES), Ok(()));
    assert_eq!(
        store.admission_check(true, MAX_FOREIGN_STORE_BYTES + 1),
        Err(ForeignChainReject::StoreFull(StoreBound::Bytes {
            held: 0,
            incoming: MAX_FOREIGN_STORE_BYTES + 1,
            budget: MAX_FOREIGN_STORE_BYTES,
        })),
        "the byte budget must refuse, and must name itself as the bound that refused"
    );

    // A single message can never blow the budget on its own — the per-message
    // entry cap keeps one presentation far below it. The budget is cumulative,
    // which is precisely why it must be accounted rather than assumed.
    let biggest_single_message = MAX_FOREIGN_CHAIN_ENTRIES * footprint;
    assert!(
        biggest_single_message < MAX_FOREIGN_STORE_BYTES,
        "one message ({biggest_single_message} B) must not be able to exhaust the budget"
    );
}

#[tokio::test]
async fn s3_4_byte_accounting_tracks_what_is_actually_held() {
    let chain = NodeBlockchain::new(coord());
    let stranger = FalconIdentity::generate();

    assert_eq!(chain.foreign_chain_bytes().await, 0);

    // One 5-entry history per asset, signed ONCE. A re-signed chain is not
    // byte-identical (nonce and timestamp differ, so `lineage_id` differs), so
    // an extension must be presented from the same entries it extends.
    let histories: Vec<ForeignAssetChain> = (0x70u8..0x76)
        .map(|tag| foreign_chain(&stranger, [tag; 32], 5))
        .collect();

    let mut expected = 0usize;
    for history in &histories {
        let prefix = ForeignAssetChain::new(history.asset_hash, history.entries[..3].to_vec());
        expected += chain_footprint_bytes(&prefix.entries);
        chain
            .accept_foreign_asset_chain(prefix)
            .await
            .expect("test: accepted");
        assert_eq!(
            chain.foreign_chain_bytes().await,
            expected,
            "every adopted entry must be charged exactly once"
        );
    }

    // An extension charges only its TAIL, never the prefix again.
    let long = histories[0].clone();
    let tail_bytes = chain_footprint_bytes(&long.entries[3..]);
    chain
        .accept_foreign_asset_chain(long)
        .await
        .expect("test: extension accepted");
    expected += tail_bytes;
    assert_eq!(chain.foreign_chain_bytes().await, expected);

    // A re-presentation of a chain already held in full charges nothing.
    // (Re-present the entries actually held: a FALCON-1024 signature is
    // variable-length, so a freshly-signed re-run is not byte-identical.)
    let held_71 = chain
        .foreign_asset_lineage(&[0x71u8; 32])
        .await
        .expect("test: 0x71 is held");
    chain
        .accept_foreign_asset_chain(ForeignAssetChain::new(
            [0x71u8; 32],
            held_71.entries.clone(),
        ))
        .await
        .expect("test: re-presentation accepted");
    assert_eq!(chain.foreign_chain_bytes().await, expected);

    // Forgetting releases exactly what it charged.
    chain.forget_foreign_asset_chain(&[0x71u8; 32]).await;
    expected -= chain_footprint_bytes(&held_71.entries);
    assert_eq!(chain.foreign_chain_bytes().await, expected);

    // A refused admission charges nothing at all.
    let before = chain.foreign_chain_bytes().await;
    assert!(chain
        .accept_foreign_asset_chain(ForeignAssetChain::new([0x77u8; 32], Vec::new()))
        .await
        .is_err());
    assert_eq!(chain.foreign_chain_bytes().await, before);
}

// ── A3: CAPACITY IS JUDGED BEFORE THE SIGNATURE WORK ───────────────────────

#[tokio::test]
async fn s3_4_capacity_is_refused_before_any_falcon_verification() {
    let chain = NodeBlockchain::new(coord());
    let stranger = FalconIdentity::generate();

    // Keep one 3-entry history back, adopting only its first entry, so a
    // genuine EXTENSION of a held chain is available to present at capacity.
    let mut extendable = [0u8; 32];
    extendable[..8].copy_from_slice(&0u64.to_le_bytes());
    let extendable_history = foreign_chain(&stranger, extendable, 3);

    for n in 0..MAX_FOREIGN_CHAINS {
        let mut asset = [0u8; 32];
        asset[..8].copy_from_slice(&(n as u64).to_le_bytes());
        let presented = if n == 0 {
            ForeignAssetChain::new(extendable, extendable_history.entries[..1].to_vec())
        } else {
            foreign_chain(&stranger, asset, 1)
        };
        chain
            .accept_foreign_asset_chain(presented)
            .await
            .expect("test: accepted below capacity");
    }
    assert_eq!(chain.foreign_chain_count().await, MAX_FOREIGN_CHAINS);

    // A NEW chain, at capacity, whose LAST entry carries a corrupted signature.
    // The two possible verdicts are an ordering oracle:
    //   StoreFull     => capacity was judged first, as documented;
    //   BadSignature  => every entry was FALCON-verified before the refusal,
    //                    i.e. the CPU-exhaustion primitive is live.
    let overflow = [0xFEu8; 32];
    let mut poisoned = foreign_chain(&stranger, overflow, MAX_FOREIGN_CHAIN_ENTRIES);
    if let Some(last) = poisoned.entries.last_mut() {
        if let Some(envelope) = last.signed_proof.as_mut() {
            envelope.signature[0] ^= 0xFF;
        }
    }

    assert_eq!(
        chain.accept_foreign_asset_chain(poisoned).await,
        Err(ForeignChainReject::StoreFull(StoreBound::Chains {
            held: MAX_FOREIGN_CHAINS,
            limit: MAX_FOREIGN_CHAINS,
        })),
        "at capacity the refusal must cost O(1), not {MAX_FOREIGN_CHAIN_ENTRIES} FALCON \
         verifications"
    );

    // The early probe and the authoritative check are the SAME function asked
    // the same question, so they cannot name different bounds.
    let store_view = ForeignChainStore::new();
    assert_eq!(
        store_view.admission_check(true, MAX_FOREIGN_STORE_BYTES + 1),
        Err(ForeignChainReject::StoreFull(StoreBound::Bytes {
            held: 0,
            incoming: MAX_FOREIGN_STORE_BYTES + 1,
            budget: MAX_FOREIGN_STORE_BYTES,
        }))
    );

    // An EXTENSION of a held chain is not growth and is still admitted at
    // capacity — the early probe must not have turned into a blanket refusal.
    let receipt = chain
        .accept_foreign_asset_chain(extendable_history)
        .await
        .expect("test: extending a held chain is admitted at capacity");
    assert_eq!(receipt.added, 2);
    assert_eq!(chain.foreign_chain_count().await, MAX_FOREIGN_CHAINS);
}

// ── A4: THE INVERSE ORDERING (adopt foreign X, then spine X) ───────────────

#[tokio::test]
async fn s3_4_a_spine_block_supersedes_an_already_adopted_foreign_chain() {
    let chain = NodeBlockchain::new(coord());
    let stranger = FalconIdentity::generate();
    let asset = [0x4Fu8; 32];

    let foreign = foreign_chain(&stranger, asset, 3);
    let foreign_head_id = foreign.entries[2].lineage_id();
    chain
        .accept_foreign_asset_chain(foreign)
        .await
        .expect("test: adopted off-spine");

    assert!(chain.foreign_asset_lineage(&asset).await.is_some());
    assert!(!chain.foreign_chain_is_shadowed(&asset).await);

    // THE INVERSE ORDERING: the spine now acquires the same asset. Nothing in
    // the spine accept path consults `foreign_chains`, and it must not — a
    // remote import able to veto a local block would be a censorship primitive.
    chain
        .add_block(vec![local_entry(asset)])
        .await
        .expect("test: the spine takes the asset on its own merits");

    // CONFIRMED: the spine wins every read path.
    let spine = chain.asset_lineage(&asset).await;
    assert_eq!(spine.entries.len(), 1);
    assert_eq!(chain.asset_lineage_any(&asset).await, spine);
    assert_ne!(
        spine.entries[0].lineage_id(),
        foreign_head_id,
        "the spine's answer must be the spine's, not the import's"
    );

    // A4 CLOSED: the superseded import stops answering as a live title, and
    // says so explicitly instead of silently coexisting.
    assert!(chain.foreign_chain_is_shadowed(&asset).await);
    assert_eq!(
        chain.foreign_asset_lineage(&asset).await,
        None,
        "a shadowed import must not keep serving a competing history"
    );
    assert_eq!(chain.foreign_asset_head(&asset).await, None);

    // The bytes are still held and still accounted — shadowing is not a free
    // deletion — and are released by an explicit local decision.
    assert!(chain.has_foreign_asset_chain(&asset).await);
    assert!(chain.foreign_chain_bytes().await > 0);
    assert_eq!(chain.forget_foreign_asset_chain(&asset).await, 3);
    assert_eq!(chain.foreign_chain_bytes().await, 0);
    assert!(!chain.foreign_chain_is_shadowed(&asset).await);
}

/// F4 — a store full of SHADOWED chains has a reachable reclaim path.
///
/// `ForeignChainStore::asset_hashes` is `pub`, but the store is `pub(crate)` and
/// nothing on `NodeBlockchain` returned the adopted set — so
/// `forget_foreign_asset_chain` only worked for a hash the caller already
/// remembered. A store whose chains have all been superseded by the spine
/// answers no query (`foreign_asset_lineage` returns `None`), still charges the
/// byte budget, and refused every new admission with no enumerable way out.
///
/// The reclaim stays a LOCAL decision: it acts only on chains the spine has
/// already taken over, and no remote input reaches it.
#[tokio::test]
async fn s3_4_shadowed_foreign_chains_are_enumerable_and_reclaimable() {
    let chain = NodeBlockchain::new(coord());
    let stranger = FalconIdentity::generate();

    // Three adopted chains; the spine later takes two of them over.
    let assets: Vec<[u8; 32]> = (0..3u8).map(|i| [0x60 + i; 32]).collect();
    for asset in &assets {
        chain
            .accept_foreign_asset_chain(foreign_chain(&stranger, *asset, 2))
            .await
            .expect("test: adopted off-spine");
    }
    let charged = chain.foreign_chain_bytes().await;
    assert_eq!(chain.foreign_chain_count().await, 3);

    // ENUMERATION: the adopted set is reachable without remembering the hashes.
    let mut enumerated = chain.foreign_asset_hashes().await;
    enumerated.sort_unstable();
    let mut expected = assets.clone();
    expected.sort_unstable();
    assert_eq!(enumerated, expected, "every adopted chain must be enumerable");
    assert!(
        chain.shadowed_foreign_asset_chains().await.is_empty(),
        "nothing is shadowed yet"
    );

    for asset in &assets[..2] {
        chain
            .add_block(vec![local_entry(*asset)])
            .await
            .expect("test: the spine takes the asset");
    }

    let mut shadowed = chain.shadowed_foreign_asset_chains().await;
    shadowed.sort_unstable();
    let mut expected_shadowed = assets[..2].to_vec();
    expected_shadowed.sort_unstable();
    assert_eq!(shadowed, expected_shadowed);

    // RECLAIM: only the shadowed entries go, and the live one is untouched.
    assert_eq!(chain.forget_shadowed_foreign_asset_chains().await, 2);
    assert_eq!(chain.foreign_chain_count().await, 1);
    assert!(chain.has_foreign_asset_chain(&assets[2]).await);
    assert!(chain.foreign_asset_lineage(&assets[2]).await.is_some());
    assert!(
        chain.foreign_chain_bytes().await < charged,
        "the byte budget must actually come back"
    );
    assert!(chain.shadowed_foreign_asset_chains().await.is_empty());

    // Reclaiming again is a no-op — it can only ever act on what the spine has
    // already superseded, so it can never delete a live answer.
    assert_eq!(chain.forget_shadowed_foreign_asset_chains().await, 0);
    assert!(chain.foreign_asset_lineage(&assets[2]).await.is_some());
}

// ── CONCURRENCY ────────────────────────────────────────────────────────────

/// S3.0 had a head->insert race that only appeared under concurrency: 2 of 8
/// concurrent writers succeeded and the rest were silently dropped. The same
/// probe, on the third accept mode.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn s3_4_concurrent_accepts_of_distinct_assets_are_all_accounted() {
    const WRITERS: usize = 32;
    const ENTRIES: usize = 3;

    let chain = std::sync::Arc::new(NodeBlockchain::new(coord()));
    let stranger = std::sync::Arc::new(FalconIdentity::generate());

    // Sign OUTSIDE the race so the window under test is the accept, not keygen.
    let presented: Vec<ForeignAssetChain> = (0..WRITERS)
        .map(|n| {
            let mut asset = [0xA0u8; 32];
            asset[..8].copy_from_slice(&(n as u64).to_le_bytes());
            foreign_chain(&stranger, asset, ENTRIES)
        })
        .collect();
    let expected_bytes: usize = presented
        .iter()
        .map(|c| chain_footprint_bytes(&c.entries))
        .sum();

    let mut tasks = Vec::new();
    for one in presented {
        let chain = chain.clone();
        tasks.push(tokio::spawn(
            async move { chain.accept_foreign_asset_chain(one).await },
        ));
    }

    let mut accepted = 0usize;
    let mut refused = 0usize;
    let mut added_total = 0usize;
    for task in tasks {
        // A task that neither returned Ok nor Err is the silent-drop failure.
        match task.await.expect("test: no accept task may panic or be dropped") {
            Ok(receipt) => {
                accepted += 1;
                added_total += receipt.added;
            }
            Err(_) => refused += 1,
        }
    }

    assert_eq!(accepted + refused, WRITERS, "every writer must be accounted");
    assert_eq!(refused, 0, "nothing here contends for the same key or bound");
    assert_eq!(accepted, WRITERS);
    assert_eq!(added_total, WRITERS * ENTRIES);

    // The store's accounting matches EXACTLY what was admitted.
    assert_eq!(chain.foreign_chain_count().await, WRITERS);
    assert_eq!(chain.foreign_chain_bytes().await, expected_bytes);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn s3_4_concurrent_accepts_of_the_same_asset_never_drop_or_double_count() {
    const WRITERS: usize = 8;

    let chain = std::sync::Arc::new(NodeBlockchain::new(coord()));
    let stranger = std::sync::Arc::new(FalconIdentity::generate());
    let asset = [0xB1u8; 32];

    // Prefixes 1..=8 of ONE history, presented concurrently. Every ordering is
    // legitimate: a longer presentation extends, a shorter one is refused as
    // NotAnExtension. Neither may be dropped, and the union must be exact.
    let full = foreign_chain(&stranger, asset, WRITERS);
    let presented: Vec<ForeignAssetChain> = (1..=WRITERS)
        .map(|len| ForeignAssetChain::new(asset, full.entries[..len].to_vec()))
        .collect();

    let mut tasks = Vec::new();
    for one in presented {
        let chain = chain.clone();
        tasks.push(tokio::spawn(
            async move { chain.accept_foreign_asset_chain(one).await },
        ));
    }

    let mut outcomes = 0usize;
    let mut added_total = 0usize;
    for task in tasks {
        match task.await.expect("test: no accept task may panic or be dropped") {
            Ok(receipt) => {
                outcomes += 1;
                added_total += receipt.added;
            }
            Err(reject) => {
                outcomes += 1;
                // The ONLY legitimate refusal here: a shorter prefix arriving
                // after a longer one. Anything else is a real defect.
                assert!(
                    matches!(reject, ForeignChainReject::NotAnExtension { .. }),
                    "unexpected refusal under same-asset contention: {reject}"
                );
            }
        }
    }

    assert_eq!(outcomes, WRITERS, "every writer must be accounted");

    // Exactly one chain, holding the longest history presented, with each entry
    // counted exactly once across all the accepts that added anything.
    assert_eq!(chain.foreign_chain_count().await, 1);
    let held = chain
        .foreign_asset_lineage(&asset)
        .await
        .expect("test: the chain is held");
    assert_eq!(held.entries.len(), WRITERS);
    assert_eq!(held.verify(), Ok(()), "the merged history must still verify");
    assert_eq!(
        added_total, WRITERS,
        "each entry must be charged as added exactly once — no double count, no gap"
    );
    assert_eq!(
        chain.foreign_chain_bytes().await,
        chain_footprint_bytes(&held.entries),
        "byte accounting must equal the entries actually held"
    );
}

/// The accept path takes `asset_index` (read) and holds it across the
/// `foreign_chains` write — the documented order
/// (`append_lock -> blocks -> headers -> hash_index -> head -> stats ->
/// asset_index -> mirror_attestations -> foreign_chains`). Spine appends take
/// those locks the other way round only in that same order, so the two cannot
/// deadlock. Run them against each other and prove it, under a hard timeout:
/// an inversion would hang here rather than fail.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn s3_4_accepts_and_spine_appends_interleave_without_deadlock() {
    const ROUNDS: usize = 24;

    let chain = std::sync::Arc::new(NodeBlockchain::new(coord()));
    let stranger = std::sync::Arc::new(FalconIdentity::generate());

    let foreigns: Vec<ForeignAssetChain> = (0..ROUNDS)
        .map(|n| {
            let mut asset = [0xC2u8; 32];
            asset[..8].copy_from_slice(&(n as u64).to_le_bytes());
            foreign_chain(&stranger, asset, 2)
        })
        .collect();

    let accepting = {
        let chain = chain.clone();
        tokio::spawn(async move {
            for one in foreigns {
                let _ = chain.accept_foreign_asset_chain(one).await;
            }
        })
    };
    let appending = {
        let chain = chain.clone();
        tokio::spawn(async move {
            for n in 0..ROUNDS {
                let mut asset = [0xD3u8; 32];
                asset[..8].copy_from_slice(&(n as u64).to_le_bytes());
                let _ = chain.add_block(vec![local_entry(asset)]).await;
            }
        })
    };

    let both = async {
        accepting.await.expect("test: accept loop");
        appending.await.expect("test: append loop");
    };
    tokio::time::timeout(std::time::Duration::from_secs(60), both)
        .await
        .expect("test: lock-order inversion would hang here");

    assert_eq!(chain.foreign_chain_count().await, ROUNDS);
    assert_eq!(chain.get_height().await, ROUNDS as u64);
    assert_eq!(chain.asset_lineage_any(&[0xD3u8; 32]).await.verify(), Ok(()));
}

/// The TOCTOU QA noted: `has_ever_seen_asset` is read WITHOUT `append_lock`,
/// then `foreign_chains` is write-locked. A spine append landing in that window
/// must not be able to leave the container holding two live titles for one
/// asset. The authoritative re-check now runs while `asset_index` is held.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn s3_4_a_spine_append_racing_an_accept_leaves_exactly_one_live_title() {
    const ROUNDS: usize = 48;

    for round in 0..ROUNDS {
        let chain = std::sync::Arc::new(NodeBlockchain::new(coord()));
        let stranger = FalconIdentity::generate();
        let mut asset = [0xE4u8; 32];
        asset[..8].copy_from_slice(&(round as u64).to_le_bytes());
        let foreign = foreign_chain(&stranger, asset, 2);

        let accepting = {
            let chain = chain.clone();
            tokio::spawn(async move { chain.accept_foreign_asset_chain(foreign).await })
        };
        let appending = {
            let chain = chain.clone();
            tokio::spawn(async move { chain.add_block(vec![local_entry(asset)]).await })
        };

        let accepted = accepting.await.expect("test: accept task").is_ok();
        appending
            .await
            .expect("test: append task")
            .expect("test: a local append must never be blocked by an import");

        // Whichever order won, the container serves exactly ONE title.
        let spine = chain.asset_lineage(&asset).await;
        assert!(!spine.is_empty(), "the spine append always lands");
        assert_eq!(
            chain.foreign_asset_lineage(&asset).await,
            None,
            "round {round}: the import must not be readable alongside a spine title \
             (accept returned Ok = {accepted})"
        );
        assert_eq!(chain.asset_lineage_any(&asset).await, spine);
    }
}
