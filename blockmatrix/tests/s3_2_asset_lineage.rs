// Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
//
// S3.2 proofs for ASSET LINEAGE carried inside the `StateProof` body.
//
//   CONTINUITY   — three sequential entries for one asset get seq 0,1,2, each
//                  naming its predecessor's `lineage_id`; `asset_lineage`
//                  returns them in order and verifies unbroken.
//   TAMPER       — a received block whose entry forges `prev_asset_entry`, or
//                  claims a wrong `asset_seq`, or re-roots the asset as a fresh
//                  genesis, is REJECTED by `insert_received_block`; the honest
//                  continuation of the same shape is ACCEPTED.
//   FOREIGN      — an entry for an asset this container has never seen that is
//                  NOT an asset-genesis is rejected explicitly (S3.4 scope).
//   HASH-SAFETY  — lineage reaches the block hash only TRANSITIVELY, through
//                  `proof_hash`; `calculate_hash` still commits to exactly
//                  `(asset_hash || proof_hash)`.
//   BATCH        — two entries for the SAME asset inside one block continue the
//                  lineage in-block (seq n, n+1).
//   PRUNE (F1)   — pruning away EVERY entry of an asset must not re-open it to a
//                  fresh root. A `(None, 0)` genesis for a pruned asset is
//                  REJECTED; the legitimate continuation at high-water + 1 is
//                  ACCEPTED; local appends continue from the high-water seq.
//   OVERFLOW(F5) — a predecessor at `u64::MAX` has no successor: `succeeds` says
//                  no and `verify` reports `SequenceOverflow` rather than
//                  wrapping to 0.

use blockmatrix::blockchain::block::{Block, BlockAssetEntry, StoragePointer};
use blockmatrix::blockchain::{LineageBreak, NodeBlockchain};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use hypermesh_lib::NodeSigner;
use trustchain::identity::FalconIdentity;
use trustchain::proof_of_state::StateProof;

fn coord() -> MatrixCoordinate {
    MatrixCoordinate::new(3, 5, 7).expect("test: valid coordinate")
}

/// A locally-appendable entry for `asset_hash` (no signature — `add_block` on a
/// signer-less test chain does not require one).
fn local_entry(asset_hash: [u8; 32]) -> BlockAssetEntry {
    BlockAssetEntry::new_bound(
        asset_hash,
        &StateProof::new_for_testing(),
        StoragePointer::Genesis,
        blockmatrix::assets::core::AssetRegistration::genesis(coord()),
    )
}

/// A RECEIVED-shape entry: content-bound, lineage-stamped, FALCON-signed by
/// `identity`, and claiming `identity` as its author — exactly what an honest
/// peer's `add_block` produces.
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

// ── CONTINUITY ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_2_three_sequential_entries_form_an_unbroken_lineage() {
    let chain = NodeBlockchain::new(coord());
    let asset = [0xA7u8; 32];

    for _ in 0..3 {
        chain
            .add_block(vec![local_entry(asset)])
            .await
            .expect("test: add_block");
    }

    let lineage = chain.asset_lineage(&asset).await;
    assert_eq!(lineage.len(), 3, "three entries recorded for the asset");

    // Sequence numbers advance 0,1,2 — the asset's own chain, not the node's.
    assert_eq!(lineage.sequence(), vec![0, 1, 2]);

    // The root is an asset genesis; each later entry names its predecessor.
    assert!(
        lineage.entries[0].is_asset_genesis(),
        "first entry must be the asset's genesis (prev=None, seq=0)",
    );
    for i in 1..lineage.len() {
        assert_eq!(
            lineage.entries[i].prev_asset_entry(),
            Some(lineage.entries[i - 1].lineage_id().as_str()),
            "entry {i} must name entry {}'s lineage_id",
            i - 1,
        );
        assert!(
            lineage.entries[i].succeeds(&lineage.entries[i - 1]),
            "entry {i} must be a well-formed successor",
        );
    }

    // And the whole chain verifies as unbroken.
    lineage.verify().expect("test: lineage must verify unbroken");
    chain
        .verify_asset_lineage(&asset)
        .await
        .expect("test: chain-side verification");

    // A DIFFERENT asset appended in between keeps its own independent chain.
    let other = [0xB8u8; 32];
    chain
        .add_block(vec![local_entry(other)])
        .await
        .expect("test: add_block");
    assert_eq!(chain.asset_lineage(&other).await.sequence(), vec![0]);
    assert_eq!(chain.asset_lineage(&asset).await.sequence(), vec![0, 1, 2]);
}

#[tokio::test]
async fn s3_2_verify_detects_a_broken_lineage() {
    let chain = NodeBlockchain::new(coord());
    let asset = [0xA9u8; 32];
    for _ in 0..2 {
        chain
            .add_block(vec![local_entry(asset)])
            .await
            .expect("test: add_block");
    }

    let mut lineage = chain.asset_lineage(&asset).await;
    lineage.verify().expect("test: honest lineage verifies");

    // Snip the root out: the remaining chain no longer starts at an asset
    // genesis, which is exactly the "truncated provenance" attack.
    let truncated = blockmatrix::blockchain::AssetLineage {
        asset_hash: asset,
        entries: lineage.entries.split_off(1),
    };
    assert!(matches!(
        truncated.verify(),
        Err(LineageBreak::RootIsNotAssetGenesis { .. }),
    ));
}

// ── TAMPER (the accept-side gate) ───────────────────────────────────────────

/// Set up a chain that already holds ONE entry for `asset`, and return
/// (chain, identity, our head entry, our head block).
async fn chain_with_asset_head(
    asset: [u8; 32],
) -> (NodeBlockchain, FalconIdentity, BlockAssetEntry, Block) {
    let chain = NodeBlockchain::new(coord());
    let block = chain
        .add_block(vec![local_entry(asset)])
        .await
        .expect("test: add_block");
    let head = chain
        .asset_lineage(&asset)
        .await
        .head()
        .cloned()
        .expect("test: asset head");
    (chain, FalconIdentity::generate(), head, block)
}

#[tokio::test]
async fn s3_2_received_honest_continuation_is_accepted() {
    let asset = [0xC1u8; 32];
    let (chain, id, head, head_block) = chain_with_asset_head(asset).await;

    let entry = received_entry(&id, asset, Some(head.lineage_id()), head.asset_seq() + 1);
    let block = Block::new(2, vec![entry], head_block.hash.clone());

    chain
        .insert_received_block(block)
        .await
        .expect("test: honest continuation must be accepted");

    assert_eq!(chain.get_height().await, 2);
    let lineage = chain
        .verify_asset_lineage(&asset)
        .await
        .expect("test: extended lineage verifies");
    assert_eq!(lineage.sequence(), vec![0, 1]);
}

#[tokio::test]
async fn s3_2_received_forged_prev_asset_entry_is_rejected() {
    let asset = [0xC2u8; 32];
    let (chain, id, head, head_block) = chain_with_asset_head(asset).await;

    // FORGERY: a real signer produces a real signature over a proof that names
    // a predecessor which is NOT our recorded head for this asset. Every H3
    // check passes; only the lineage check can catch this.
    let forged_prev = hex::encode([0xEEu8; 32]);
    let entry = received_entry(&id, asset, Some(forged_prev), head.asset_seq() + 1);
    entry
        .verify_signed_proof()
        .expect("test: the forgery is properly signed — H3 cannot catch it");

    let block = Block::new(2, vec![entry], head_block.hash.clone());
    let err = chain
        .insert_received_block(block)
        .await
        .expect_err("test: forged prev_asset_entry must be rejected");
    assert!(
        err.contains("asset lineage broken")
            && err.contains("claims predecessor")
            && err.contains("our recorded head"),
        "error must cite the lineage break, got: {err}",
    );

    assert_eq!(chain.get_height().await, 1, "chain must be untouched");
    assert_eq!(chain.asset_lineage(&asset).await.sequence(), vec![0]);
}

#[tokio::test]
async fn s3_2_received_wrong_asset_seq_is_rejected() {
    let asset = [0xC3u8; 32];
    let (chain, id, head, head_block) = chain_with_asset_head(asset).await;

    // Correct prev-pointer, WRONG sequence number (skips ahead) — a lineage
    // with an invisible gap in it.
    let entry = received_entry(&id, asset, Some(head.lineage_id()), head.asset_seq() + 7);
    let block = Block::new(2, vec![entry], head_block.hash.clone());

    let err = chain
        .insert_received_block(block)
        .await
        .expect_err("test: wrong asset_seq must be rejected");
    assert!(
        err.contains("asset_seq") && err.contains("+ 1"),
        "error must cite the sequence violation, got: {err}",
    );
    assert_eq!(chain.get_height().await, 1, "chain must be untouched");
}

#[tokio::test]
async fn s3_2_received_reroot_of_a_known_asset_is_rejected() {
    let asset = [0xC4u8; 32];
    let (chain, id, _head, head_block) = chain_with_asset_head(asset).await;

    // Claim a FRESH asset genesis for an asset we already hold — this would
    // silently discard the asset's existing provenance.
    let entry = received_entry(&id, asset, None, 0);
    let block = Block::new(2, vec![entry], head_block.hash.clone());

    let err = chain
        .insert_received_block(block)
        .await
        .expect_err("test: re-rooting a known asset must be rejected");
    assert!(
        err.contains("asset-genesis for an asset this container has already seen"),
        "error must cite the re-root, got: {err}",
    );
    assert_eq!(chain.get_height().await, 1, "chain must be untouched");
}

// ── FOREIGN ASSET-CHAIN (S3.4 scope) ───────────────────────────────────────

#[tokio::test]
async fn s3_2_received_unknown_asset_genesis_is_accepted() {
    let chain = NodeBlockchain::new(coord());
    let genesis = chain.get_head().await.expect("test: genesis");
    let id = FalconIdentity::generate();

    let entry = received_entry(&id, [0xD0u8; 32], None, 0);
    let block = Block::new(1, vec![entry], genesis.hash.clone());

    chain
        .insert_received_block(block)
        .await
        .expect("test: an asset-genesis for a new asset is accepted");
    assert_eq!(
        chain.asset_lineage(&[0xD0u8; 32]).await.sequence(),
        vec![0],
    );
}

#[tokio::test]
async fn s3_2_received_foreign_asset_chain_is_rejected_not_silently_accepted() {
    let chain = NodeBlockchain::new(coord());
    let genesis = chain.get_head().await.expect("test: genesis");
    let id = FalconIdentity::generate();

    // seq 5 for an asset whose first four entries we have never seen: the
    // provenance is unverifiable in the block-accept path. It must be REJECTED
    // and routed to the asset-chain accept path, never accepted with a hole.
    let entry = received_entry(&id, [0xD1u8; 32], Some(hex::encode([0x42u8; 32])), 5);
    let block = Block::new(1, vec![entry], genesis.hash.clone());

    let err = chain
        .insert_received_block(block)
        .await
        .expect_err("test: an unverified asset history must be rejected by the block path");
    assert!(
        err.contains("asset-chain accept path") && err.contains("accept_asset_chain"),
        "rejection must route to the asset-chain accept path, got: {err}",
    );
    assert!(chain.asset_lineage(&[0xD1u8; 32]).await.is_empty());
}

// ── HASH SAFETY ────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_2_lineage_reaches_the_block_hash_only_through_proof_hash() {
    let asset = [0xE5u8; 32];
    let mut entry = local_entry(asset);
    let before_proof_hash = entry.proof_hash;
    let before_block = Block::new(4, vec![entry.clone()], "prev".to_string());

    entry.set_asset_lineage(Some(hex::encode([0x01u8; 32])), 3);

    // The proof body changed, so proof_hash MUST change...
    assert_ne!(
        entry.proof_hash, before_proof_hash,
        "lineage must be covered by proof_hash",
    );
    // ...and the entry stays content-bound (P1) by construction.
    assert!(entry.content_binding_ok());

    // ...which is the ONLY way it reaches the block hash: hold proof_hash
    // fixed and the block hash is unchanged by the proof body.
    let after_block = Block::new(4, vec![entry.clone()], "prev".to_string());
    assert_ne!(before_block.hash, after_block.hash);

    let mut restored = after_block.clone();
    restored.entries[0].proof_hash = before_proof_hash;
    assert_eq!(
        restored.calculate_hash(),
        before_block.hash,
        "calculate_hash must commit to exactly (asset_hash || proof_hash)",
    );
}

// ── IN-BLOCK BATCH CONTINUATION ────────────────────────────────────────────

#[tokio::test]
async fn s3_2_two_entries_for_one_asset_in_one_block_continue_the_lineage() {
    let chain = NodeBlockchain::new(coord());
    let asset = [0xF1u8; 32];

    // The genesis hardware batch shape: one block, several entries — here two
    // of them for the SAME asset.
    chain
        .add_block(vec![local_entry(asset), local_entry([0xF2u8; 32]), local_entry(asset)])
        .await
        .expect("test: add_block");

    let lineage = chain
        .verify_asset_lineage(&asset)
        .await
        .expect("test: in-block continuation must verify");
    assert_eq!(lineage.sequence(), vec![0, 1]);
    assert!(lineage.entries[1].succeeds(&lineage.entries[0]));

    // The other asset in the same block is independent.
    assert_eq!(chain.asset_lineage(&[0xF2u8; 32]).await.sequence(), vec![0]);
}

// ── PRUNE / LINEAGE-RESET (S3.2 QA F1) ─────────────────────────────────────
//
// `prune_to_headers` correctly drops an asset's entries from the S3.1 index —
// the index may never name a block the chain no longer holds in full. Before
// F1 that also made the asset UNKNOWN, and an unknown asset accepts a fresh
// `(prev = None, seq = 0)` asset-genesis: prune, then re-root the asset under
// someone else's history. The high-water TOMBSTONE closes it — the asset's
// identity survives pruning even though its entry bodies do not.

/// Build a chain where asset `A` has TWO entries (blocks 1 and 2) and an
/// unrelated asset occupies block 3, then prune blocks 1..3 away.
///
/// Block 3 is deliberately left FULL: a received block N is only judged (rather
/// than orphan-buffered) when block N-1 is held in full, so the test needs a
/// live predecessor to actually reach the lineage gate.
///
/// Returns `(chain, high_water_lineage_id, high_water_seq, block_3)`.
async fn chain_with_pruned_asset(asset: [u8; 32]) -> (NodeBlockchain, String, u64, Block) {
    let chain = NodeBlockchain::new(coord());
    chain.add_block(vec![local_entry(asset)]).await.expect("test: block 1");
    chain.add_block(vec![local_entry(asset)]).await.expect("test: block 2");
    let block_3 = chain
        .add_block(vec![local_entry([0x5Au8; 32])])
        .await
        .expect("test: block 3 (unrelated asset)");

    let head = chain
        .asset_lineage(&asset)
        .await
        .head()
        .cloned()
        .expect("test: asset head before pruning");
    let (high_water_id, high_water_seq) = (head.lineage_id(), head.asset_seq());
    assert_eq!(high_water_seq, 1, "asset advanced to seq 1 before pruning");

    // (a) Prune away EVERY entry of the asset.
    chain.prune_to_headers(1..3).await;

    assert!(
        chain.asset_head(&asset).await.is_none(),
        "pruned asset has no held head entry",
    );
    assert!(
        chain.asset_lineage(&asset).await.is_empty(),
        "pruned asset has no reproducible history",
    );
    // ...but the container has NOT forgotten that it saw the asset.
    assert!(
        chain.has_ever_seen_asset(&asset).await,
        "F1: the asset's identity must survive pruning of its entry bodies",
    );

    (chain, high_water_id, high_water_seq, block_3)
}

#[tokio::test]
async fn s3_2_f1_pruning_does_not_re_open_an_asset_to_a_fresh_root() {
    let asset = [0x9Au8; 32];
    let (chain, high_water_id, high_water_seq, block_3) =
        chain_with_pruned_asset(asset).await;
    let id = FalconIdentity::generate();

    // (b) A properly signed FRESH asset-genesis for the pruned asset. Every
    //     other check passes — H3 signature, content binding, proof integrity.
    //     Only the tombstone can catch it.
    let reroot = received_entry(&id, asset, None, 0);
    reroot
        .verify_signed_proof()
        .expect("test: the re-root is properly signed — H3 cannot catch it");
    let block = Block::new(4, vec![reroot], block_3.hash.clone());

    let err = chain
        .insert_received_block(block)
        .await
        .expect_err("F1: a fresh root for a PRUNED asset must be rejected");
    assert!(
        err.contains("asset-genesis for an asset this container has already seen"),
        "rejection must cite the re-root of a seen asset, got: {err}",
    );
    assert_eq!(chain.get_height().await, 3, "chain must be untouched");

    // (c) The LEGITIMATE continuation — naming the retained high-water
    //     lineage_id at high-water + 1 — is still accepted after pruning.
    let continuation = received_entry(
        &id,
        asset,
        Some(high_water_id.clone()),
        high_water_seq + 1,
    );
    let block = Block::new(4, vec![continuation], block_3.hash.clone());
    chain
        .insert_received_block(block)
        .await
        .expect("F1: a correct continuation at high-water + 1 must still be accepted");

    assert_eq!(chain.get_height().await, 4);
    assert_eq!(
        chain.asset_lineage(&asset).await.sequence(),
        vec![high_water_seq + 1],
        "only the un-pruned entry is reproducible, and it continues the sequence",
    );
}

#[tokio::test]
async fn s3_2_f1_local_appends_continue_from_the_high_water_not_from_zero() {
    let asset = [0x9Bu8; 32];
    let (chain, high_water_id, high_water_seq, _block_3) =
        chain_with_pruned_asset(asset).await;

    // The local stamp path takes the SAME tombstone fallback as the receive
    // path: a local write for a pruned asset CONTINUES its chain rather than
    // silently restarting it at 0. Restarting would let a node launder an
    // asset's provenance by pruning its own history.
    chain
        .add_block(vec![local_entry(asset)])
        .await
        .expect("test: local append after prune");

    let head = chain
        .asset_lineage(&asset)
        .await
        .head()
        .cloned()
        .expect("test: new head");
    assert_eq!(
        head.asset_seq(),
        high_water_seq + 1,
        "local append must continue from the high-water seq, not restart at 0",
    );
    assert_eq!(
        head.prev_asset_entry(),
        Some(high_water_id.as_str()),
        "local append must name the retained high-water lineage_id",
    );
    assert!(
        !head.is_asset_genesis(),
        "a pruned asset must never be re-rooted by a local write",
    );
}

#[tokio::test]
async fn s3_2_f1_index_rebuild_equality_still_holds_across_a_prune() {
    // The S3.1 property — incrementally maintained == rebuilt from the same
    // blocks — must survive F1. Tombstones are runtime-only and EXCLUDED from
    // `PartialEq`, precisely because a rebuild from surviving blocks cannot
    // know what pruning erased; including them would make the property false
    // by construction after any prune.
    let asset = [0x9Cu8; 32];
    let (chain, _id, _seq, _b3) = chain_with_pruned_asset(asset).await;

    let incremental = chain.asset_index_snapshot().await;
    let rebuilt = blockmatrix::blockchain::AssetChainIndex::rebuild(
        chain.get_chain().await.iter(),
    );
    assert_eq!(
        incremental, rebuilt,
        "S3.1 rebuild == incremental must still hold after a prune",
    );

    // And the tombstone itself is present in the live index (it is simply not
    // part of the equality set).
    assert!(
        incremental.has_ever_seen_asset(&asset),
        "the live index retains the tombstone",
    );
    assert!(
        !rebuilt.has_ever_seen_asset(&asset),
        "a rebuild from surviving blocks alone cannot know the pruned asset — \
         which is exactly why tombstones are excluded from equality",
    );
}

// ── SEQUENCE OVERFLOW (S3.2 QA F5) ─────────────────────────────────────────

#[tokio::test]
async fn s3_2_f5_sequence_overflow_fails_closed_instead_of_wrapping() {
    let asset = [0x9Du8; 32];

    let mut root = local_entry(asset);
    root.set_asset_lineage(None, 0);

    let mut predecessor = local_entry(asset);
    predecessor.set_asset_lineage(Some(root.lineage_id()), u64::MAX);

    // A wrapping `+ 1` would make seq 0 the "expected" successor — i.e. a
    // fresh root would read as a valid continuation.
    let mut wrapped = local_entry(asset);
    wrapped.set_asset_lineage(Some(predecessor.lineage_id()), 0);
    assert!(
        !wrapped.succeeds(&predecessor),
        "F5: u64::MAX has no successor — must not wrap to 0",
    );

    // Sanity: without the overflow, the SAME shape is a plain sequence gap —
    // so the assertion above is about the overflow, not about `succeeds`
    // rejecting everything.
    let mut normal = local_entry(asset);
    normal.set_asset_lineage(Some(hex::encode([0x11u8; 32])), 41);
    let mut next = local_entry(asset);
    next.set_asset_lineage(Some(normal.lineage_id()), 42);
    assert!(next.succeeds(&normal), "sanity: 41 → 42 is a valid successor");

    // `AssetLineage::verify` and `check_entry_lineage` carry the same
    // `checked_add`. Their overflow arms are defense-in-depth rather than
    // separately reachable: `verify` requires position 0 to be an asset
    // genesis (`seq == 0`) and rejects the first sequence jump, so no
    // well-formed prefix can present a predecessor at `u64::MAX`; and the
    // receive path's head seq is read from an entry this container already
    // accepted through that same gate. What the assertion above proves is the
    // shared invariant: the successor sequence is COMPUTED, never wrapped.
    let lineage = blockmatrix::blockchain::AssetLineage {
        asset_hash: asset,
        entries: vec![root, predecessor],
    };
    assert!(
        matches!(lineage.verify(), Err(LineageBreak::SequenceGap { .. })),
        "a jump to u64::MAX is caught as a gap before any successor is computed",
    );
}

// ── REJECTIONS ARE VISIBLE AT THE DEFAULT LOG LEVEL (S3.2 QA F2) ───────────
//
// The node binary installs `tracing_subscriber::fmt().with_max_level(INFO)`
// unless `--debug` is passed (bin/node/main.rs), so anything logged at
// `debug!` is DISCARDED in normal operation. Both production block-receive
// paths logged their rejection at `debug!` — the carefully-worded lineage
// rejection was invisible exactly when it mattered. Both are now `warn!`,
// matching the orphan-drain path.
//
// This test drives the orphan-drain rejection (the one path reachable from a
// bare `NodeBlockchain`, with no peer or transport) through a subscriber
// capped at INFO — the binary's default — and asserts the rejection text is
// actually emitted and names the lineage break.

/// A `MakeWriter` that appends every log line to a shared buffer.
#[derive(Clone)]
struct CapturedLog(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for CapturedLog {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.0.lock() {
            Ok(mut sink) => {
                sink.extend_from_slice(buf);
                Ok(buf.len())
            }
            Err(_) => Ok(buf.len()),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLog {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[tokio::test]
async fn s3_2_f2_a_lineage_rejection_is_visible_at_the_default_log_level() {
    let asset = [0xB2u8; 32];
    let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let subscriber = tracing_subscriber::fmt()
        // Exactly what `hypermesh` installs without `--debug`.
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false)
        .with_writer(CapturedLog(sink.clone()))
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        futures::executor::block_on(async {
            let chain = NodeBlockchain::new(coord());
            let id = FalconIdentity::generate();
            let block_1 = chain
                .add_block(vec![local_entry(asset)])
                .await
                .expect("test: block 1");

            // Block 2 is honest and unrelated; build it but hold it back.
            let honest = received_entry(&id, [0x77u8; 32], None, 0);
            let block_2 = Block::new(2, vec![honest], block_1.hash.clone());

            // Block 3 forges the asset's predecessor. It arrives FIRST, so it
            // is buffered as an orphan and only judged on drain.
            let forged = received_entry(&id, asset, Some(hex::encode([0xEEu8; 32])), 1);
            let block_3 = Block::new(3, vec![forged], block_2.hash.clone());
            chain
                .insert_received_block(block_3)
                .await
                .expect("test: orphan is buffered, not judged, on arrival");

            // Linking block 2 drains the orphan — and rejects it.
            chain
                .insert_received_block(block_2)
                .await
                .expect("test: honest block 2 links");

            assert_eq!(
                chain.get_height().await,
                2,
                "the forged orphan must NOT have entered the chain",
            );
        });
    });

    let captured = String::from_utf8_lossy(
        &sink.lock().expect("test: log sink").clone(),
    )
    .to_string();

    assert!(
        captured.contains("Dropping orphan block 3"),
        "F2: the drop must be visible at the binary's default level (INFO); \
         captured:\n{captured}",
    );
    assert!(
        captured.contains("asset lineage broken") && captured.contains("claims predecessor"),
        "F2: the visible message must name the lineage break, not just 'failed'; \
         captured:\n{captured}",
    );
    assert!(
        captured.contains("WARN"),
        "F2: the rejection must be logged at WARN, not DEBUG; captured:\n{captured}",
    );
}
