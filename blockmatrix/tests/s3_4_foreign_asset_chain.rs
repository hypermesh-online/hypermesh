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
    ForeignAssetChain, ForeignChainReject, LineageBreak, NodeBlockchain, MAX_FOREIGN_CHAINS,
    MAX_FOREIGN_CHAIN_ENTRIES,
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
        Err(ForeignChainReject::StoreFull {
            limit: MAX_FOREIGN_CHAINS
        })
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
