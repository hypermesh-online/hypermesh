// Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
//
// S3.1 proofs for `AssetChainIndex` — the derived per-asset view that replaces
// the O(blocks × entries × shards) linear scans behind
// `NodeBlockchain::authorizes_shard` and `registration_for_shard`.
//
//   EQUIVALENCE  — index answers == brute-force full-chain scan answers, for
//                  hits and misses, over a randomized populated chain.
//   MANY-TO-MANY — one block carrying entries for N distinct assets is indexed
//                  correctly for all N (this is the genesis hardware-batch
//                  shape: `register_asset_records`).
//   REBUILD      — an index rebuilt from the block set (in memory and via a
//                  real persist → reload round-trip) equals the one that was
//                  maintained incrementally.
//   STALENESS    — after `prune_to_headers` removes full blocks, the index no
//                  longer reports what those blocks contributed.

use blockmatrix::blockchain::block::{Block, BlockAssetEntry, StoragePointer};
use blockmatrix::blockchain::{AssetChainIndex, NodeBlockchain};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::persistence::{BlockQuery, PersistenceConfig, PersistenceManager};
use trustchain::proof_of_state::StateProof;

fn coord() -> MatrixCoordinate {
    MatrixCoordinate::new(2, 3, 4).expect("test: valid coordinate")
}

/// Deterministic PRNG — the randomized equivalence sweep must reproduce
/// byte-identically on every run and on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn hash32(&mut self) -> [u8; 32] {
        let mut out = [0u8; 32];
        for chunk in out.chunks_mut(8) {
            chunk.copy_from_slice(&self.next().to_le_bytes());
        }
        out
    }
}

fn entry(asset_hash: [u8; 32], pointer: StoragePointer) -> BlockAssetEntry {
    BlockAssetEntry::new_bound(
        asset_hash,
        &StateProof::new_for_testing(),
        pointer,
        blockmatrix::assets::core::AssetRegistration::genesis(coord()),
    )
}

fn sharded(asset_hash: [u8; 32], shard_hashes: Vec<[u8; 32]>) -> BlockAssetEntry {
    entry(
        asset_hash,
        StoragePointer::Sharded {
            shard_hashes,
            placements: vec![coord()],
        },
    )
}

// ── brute-force reference implementations ──────────────────────────────────
//
// These are byte-for-byte the pre-S3.1 scans (chain.rs:390-403 and :419-435),
// lifted out so the index can be proven equal to them.

fn scan_authorizes_shard(blocks: &[Block], shard_id: &[u8; 32]) -> bool {
    blocks.iter().any(|block| {
        block.entries.iter().any(|e| match &e.storage_pointer {
            StoragePointer::Sharded { shard_hashes, .. } => {
                shard_hashes.iter().any(|h| h == shard_id)
            }
            _ => false,
        })
    })
}

/// Every entry the old scan could have returned for `shard_id`.
///
/// The old `registration_for_shard` iterated `blocks: HashMap<u64, Block>` and
/// returned the FIRST match in *hash-map* order — i.e. an arbitrary member of
/// this set. Equivalence therefore means: the index returns `Some` exactly when
/// this set is non-empty, and what it returns is a member of the set.
fn scan_registration_candidates(blocks: &[Block], shard_id: &[u8; 32]) -> Vec<BlockAssetEntry> {
    let mut out = Vec::new();
    for block in blocks {
        for e in &block.entries {
            if let StoragePointer::Sharded { shard_hashes, .. } = &e.storage_pointer {
                if shard_hashes.iter().any(|h| h == shard_id) {
                    out.push(e.clone());
                }
            }
        }
    }
    out
}

fn scan_asset_history(blocks: &[Block], asset_hash: &[u8; 32]) -> Vec<(u64, usize)> {
    let mut out: Vec<(u64, usize)> = Vec::new();
    for block in blocks {
        for (ix, e) in block.entries.iter().enumerate() {
            if &e.asset_hash == asset_hash {
                out.push((block.index, ix));
            }
        }
    }
    out.sort();
    out
}

/// Populate a chain with a randomized-but-deterministic mix of blocks:
/// multi-entry blocks, repeated assets across blocks, repeated shards across
/// entries, and non-sharded (`Genesis`/`Local`) entries that must be ignored by
/// the shard queries.
///
/// Returns `(chain, all_asset_hashes, all_shard_hashes)`.
async fn populate(
    seed: u64,
    block_count: usize,
) -> (NodeBlockchain, Vec<[u8; 32]>, Vec<[u8; 32]>) {
    let chain = NodeBlockchain::new(coord());
    let mut rng = Rng(seed);

    // A pool that entries draw from, so assets recur across blocks and shards
    // recur across entries — the many-to-many and duplicate cases.
    let asset_pool: Vec<[u8; 32]> = (0..24).map(|_| rng.hash32()).collect();
    let shard_pool: Vec<[u8; 32]> = (0..40).map(|_| rng.hash32()).collect();

    let mut used_assets = Vec::new();
    let mut used_shards = Vec::new();

    for _ in 0..block_count {
        let entry_count = 1 + rng.below(4) as usize;
        let mut entries = Vec::with_capacity(entry_count);

        for _ in 0..entry_count {
            let asset = asset_pool[rng.below(asset_pool.len() as u64) as usize];
            used_assets.push(asset);

            match rng.below(4) {
                0 => entries.push(entry(asset, StoragePointer::Genesis)),
                1 => entries.push(entry(
                    asset,
                    StoragePointer::Local {
                        path: "/dev/null".to_string(),
                    },
                )),
                _ => {
                    let shard_count = 1 + rng.below(5) as usize;
                    let shards: Vec<[u8; 32]> = (0..shard_count)
                        .map(|_| shard_pool[rng.below(shard_pool.len() as u64) as usize])
                        .collect();
                    used_shards.extend(shards.iter().copied());
                    entries.push(sharded(asset, shards));
                }
            }
        }

        chain.add_block(entries).await.expect("test: add_block");
    }

    used_assets.sort();
    used_assets.dedup();
    used_shards.sort();
    used_shards.dedup();
    (chain, used_assets, used_shards)
}

// ── EQUIVALENCE ────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_1_index_matches_brute_force_scan_for_hits_and_misses() {
    let (chain, assets, shards) = populate(0x5E3D_1A55, 60).await;
    let blocks = chain.get_chain().await;

    assert!(
        shards.len() > 10,
        "test fixture must produce a meaningful shard set, got {}",
        shards.len()
    );

    // HITS — every shard that actually appears on-chain.
    for shard in &shards {
        let scan_auth = scan_authorizes_shard(&blocks, shard);
        assert!(scan_auth, "fixture invariant: {shard:?} must be on-chain");
        assert_eq!(
            chain.authorizes_shard(shard).await,
            scan_auth,
            "authorizes_shard disagrees with full scan for an on-chain shard",
        );

        let candidates = scan_registration_candidates(&blocks, shard);
        let indexed = chain
            .registration_for_shard(shard)
            .await
            .expect("index must return a registration where the scan finds one");
        assert!(
            candidates.contains(&indexed),
            "registration_for_shard returned an entry the full scan never yields",
        );
    }

    // MISSES — a large sweep of shard ids that are not on-chain.
    let mut rng = Rng(0xDEAD_BEEF);
    let mut misses = 0usize;
    for _ in 0..500 {
        let probe = rng.hash32();
        if shards.contains(&probe) {
            continue;
        }
        misses += 1;
        assert!(
            !scan_authorizes_shard(&blocks, &probe),
            "fixture invariant: random probe must not be on-chain",
        );
        assert!(
            !chain.authorizes_shard(&probe).await,
            "index authorized a shard the full scan rejects",
        );
        assert!(
            chain.registration_for_shard(&probe).await.is_none(),
            "index produced a registration the full scan cannot find",
        );
    }
    assert!(misses > 400, "miss sweep degenerated: {misses} probes");

    // Per-asset history must equal a full scan too, including the zeroed
    // "asset that was never registered" case.
    for asset in &assets {
        let scanned = scan_asset_history(&blocks, asset);
        let indexed: Vec<(u64, usize)> = chain
            .asset_history(asset)
            .await
            .iter()
            .map(|l| (l.block_index, l.entry_ix))
            .collect();
        assert_eq!(indexed, scanned, "asset history disagrees with full scan");
        assert_eq!(
            chain.asset_head(asset).await.map(|l| (l.block_index, l.entry_ix)),
            scanned.last().copied(),
            "asset head is not the last entry the full scan finds",
        );
    }
    assert!(chain.asset_history(&[0u8; 32]).await.is_empty());
    assert!(chain.asset_head(&[0u8; 32]).await.is_none());
}

// ── MANY-TO-MANY ───────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_1_single_block_with_n_distinct_assets_is_fully_indexed() {
    const N: usize = 16;
    let chain = NodeBlockchain::new(coord());

    let assets: Vec<[u8; 32]> = (0..N).map(|i| [(i as u8) + 1; 32]).collect();
    let shards: Vec<[u8; 32]> = (0..N).map(|i| [0x80 + (i as u8); 32]).collect();

    // One block, N entries, N distinct assets — the `register_asset_records`
    // genesis hardware-batch shape. A naive asset→block_index map is wrong here.
    let entries: Vec<BlockAssetEntry> = assets
        .iter()
        .zip(shards.iter())
        .map(|(a, s)| sharded(*a, vec![*s]))
        .collect();
    let block = chain.add_block(entries).await.expect("test: add_block");
    assert_eq!(block.entries.len(), N);

    assert_eq!(
        chain.indexed_asset_count().await,
        N + 1,
        "N batch assets + the genesis block's own asset",
    );

    for (ix, (asset, shard)) in assets.iter().zip(shards.iter()).enumerate() {
        let history = chain.asset_history(asset).await;
        assert_eq!(history.len(), 1, "asset {ix} indexed {} times", history.len());
        assert_eq!(history[0].block_index, block.index);
        assert_eq!(history[0].entry_ix, ix, "asset {ix} indexed at wrong entry");
        assert_eq!(history[0].block_hash, block.hash);

        assert!(chain.authorizes_shard(shard).await, "shard {ix} not authorized");
        let reg = chain
            .registration_for_shard(shard)
            .await
            .expect("registration for a shard of a batched asset");
        assert_eq!(
            &reg.asset_hash, asset,
            "shard {ix} resolved to the wrong asset in a multi-asset block",
        );
    }

    // And the same asset appearing again in a LATER block extends its history
    // rather than replacing it.
    let repeat = chain
        .add_block(vec![sharded(assets[0], vec![[0xF0; 32]])])
        .await
        .expect("test: add_block");
    let history = chain.asset_history(&assets[0]).await;
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].block_index, block.index);
    assert_eq!(history[1].block_index, repeat.index);
    assert_eq!(
        chain.asset_head(&assets[0]).await.map(|l| l.block_index),
        Some(repeat.index),
    );
}

// ── REBUILD ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_1_rebuild_from_blocks_equals_incremental_index() {
    let (chain, _, _) = populate(0x0BAD_F00D, 40).await;

    let incremental = chain.asset_index_snapshot().await;
    let blocks = chain.get_chain().await;

    // Direct rebuild from the block set.
    assert_eq!(
        AssetChainIndex::rebuild(blocks.iter()),
        incremental,
        "index rebuilt from the same blocks differs from the maintained one",
    );

    // Rebuild through the real `from_blocks` restore path.
    let restored =
        NodeBlockchain::from_blocks(coord(), blocks.clone()).expect("test: from_blocks");
    assert_eq!(
        restored.asset_index_snapshot().await,
        incremental,
        "from_blocks produced a different index than incremental maintenance",
    );

    // Rebuild is order-independent: blocks do not always arrive in index order
    // (received blocks, orphan drains).
    let mut reversed = blocks.clone();
    reversed.reverse();
    assert_eq!(AssetChainIndex::rebuild(reversed.iter()), incremental);
}

/// D1 — the reframed semantics: `lineage_id` is the authoritative address of an
/// entry; the `AssetChainIndex` locator (`block_index`) is a throwaway cache
/// pointer. Two things this proves, over a populated chain:
///
/// 1. **Cache agrees with authority.** For every asset, the head the derived
///    index points at (by `block_index`) resolves to the SAME entry the
///    authoritative [`AssetLineage`] names (by `lineage_id`).
/// 2. **Identity survives cache loss.** Throw the whole index away and rebuild
///    it from the blocks: the authoritative `lineage_id` of every asset's head
///    is unchanged. The cache is reconstructable; the identity is not derived
///    from it.
#[tokio::test]
async fn s3_1_lineage_id_is_authority_locator_is_a_throwaway_cache() {
    let (chain, assets, _) = populate(0xD1_CAC4E_0001, 40).await;
    assert!(!assets.is_empty(), "fixture must register assets");

    let blocks = chain.get_chain().await;
    let rebuilt = AssetChainIndex::rebuild(blocks.iter());

    for asset in &assets {
        // Authoritative head: the last entry of the asset's own lineage,
        // addressed by lineage_id (spine-offset-free).
        let lineage = chain.asset_lineage(asset).await;
        let Some(authoritative_head) = lineage.head() else {
            continue;
        };
        let authoritative_id = authoritative_head.lineage_id();

        // (1) The live cache locator resolves to the same entry identity.
        let cache_locator = chain
            .asset_head(asset)
            .await
            .expect("test: indexed head for a held asset");
        let via_cache = chain
            .entry_at(&cache_locator)
            .await
            .expect("test: locator resolves to its entry");
        assert_eq!(
            via_cache.lineage_id(),
            authoritative_id,
            "cache locator and authoritative lineage disagree on the head entry",
        );

        // (2) A rebuilt-from-scratch cache names the same identity — the
        // block_index pointer may be regenerated, the lineage_id may not change.
        let rebuilt_head = rebuilt
            .asset_head(asset)
            .expect("test: rebuilt cache head for a held asset");
        let via_rebuilt = blocks
            .iter()
            .find(|b| b.index == rebuilt_head.block_index)
            .and_then(|b| b.entries.get(rebuilt_head.entry_ix))
            .expect("test: rebuilt locator resolves to its entry");
        assert_eq!(
            via_rebuilt.lineage_id(),
            authoritative_id,
            "authoritative identity changed when the cache was rebuilt",
        );
    }
}

#[tokio::test]
async fn s3_1_index_survives_a_real_persist_reload_round_trip() {
    let dir = tempfile::tempdir().expect("test: temp dir");
    let config = PersistenceConfig {
        storage_dir: dir.path().to_path_buf(),
        enable_background: false,
        ..PersistenceConfig::default()
    };
    let persistence = std::sync::Arc::new(
        PersistenceManager::new(config.clone(), "s3-1-index-node".to_string())
            .await
            .expect("test: persistence manager"),
    );

    let genesis = Block::genesis(coord());
    persistence
        .save_block(&genesis)
        .await
        .expect("test: persist genesis");

    let chain = NodeBlockchain::from_genesis(coord(), genesis.clone())
        .with_persistence(persistence.clone());

    let shard_a = [0x1Au8; 32];
    let shard_b = [0x2Bu8; 32];
    chain
        .add_block(vec![
            sharded([0xAA; 32], vec![shard_a]),
            sharded([0xBB; 32], vec![shard_b]),
        ])
        .await
        .expect("test: add_block");
    chain
        .add_block(vec![sharded([0xAA; 32], vec![shard_a, [0x3C; 32]])])
        .await
        .expect("test: add_block");

    let before = chain.asset_index_snapshot().await;
    drop(chain);

    // Reload exactly as the node binary's resume path does.
    let persistence = std::sync::Arc::new(
        PersistenceManager::new(config, "s3-1-index-node".to_string())
            .await
            .expect("test: persistence manager"),
    );
    let mut loaded = Vec::new();
    for idx in 0..=2u64 {
        if let Some(block) = persistence
            .load_block(BlockQuery::ByIndex(idx))
            .await
            .expect("test: load block")
        {
            loaded.push(block);
        }
    }
    assert_eq!(loaded.len(), 3, "genesis + 2 runtime blocks must be on disk");

    let restored = NodeBlockchain::from_blocks(coord(), loaded).expect("test: from_blocks");
    assert_eq!(
        restored.asset_index_snapshot().await,
        before,
        "index rebuilt from disk differs from the pre-restart index",
    );
    assert!(restored.authorizes_shard(&shard_a).await);
    assert!(restored.authorizes_shard(&shard_b).await);
    assert_eq!(restored.asset_history(&[0xAA; 32]).await.len(), 2);
}

// ── STALENESS ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn s3_1_prune_to_headers_removes_pruned_blocks_from_the_index() {
    let chain = NodeBlockchain::new(coord());

    // Blocks 1..=5, each with a distinct shard; the recurring asset 0xEE spans
    // blocks 1, 3 and 5 so pruning must trim its history, not erase it.
    let mut shards = Vec::new();
    for i in 1..=5u8 {
        let shard = [0xC0 + i; 32];
        shards.push(shard);
        let asset = if i % 2 == 1 { [0xEE; 32] } else { [0xD0 + i; 32] };
        chain
            .add_block(vec![sharded(asset, vec![shard])])
            .await
            .expect("test: add_block");
    }

    assert_eq!(chain.asset_history(&[0xEE; 32]).await.len(), 3);
    for shard in &shards {
        assert!(chain.authorizes_shard(shard).await);
    }

    // Prune blocks 1, 2, 3 to headers.
    chain.prune_to_headers(1..4).await;

    let remaining = chain.get_chain().await;
    assert_eq!(remaining.len(), 3, "genesis + blocks 4 and 5 remain in full");

    // Every query must now agree with a brute-force scan of what is LEFT.
    for shard in &shards {
        let scanned = scan_authorizes_shard(&remaining, shard);
        assert_eq!(
            chain.authorizes_shard(shard).await,
            scanned,
            "index is stale for {shard:?} after prune",
        );
        assert_eq!(
            chain.registration_for_shard(shard).await.is_some(),
            scanned,
            "registration_for_shard is stale for {shard:?} after prune",
        );
    }
    assert!(!chain.authorizes_shard(&shards[0]).await, "pruned shard 1");
    assert!(!chain.authorizes_shard(&shards[1]).await, "pruned shard 2");
    assert!(!chain.authorizes_shard(&shards[2]).await, "pruned shard 3");
    assert!(chain.authorizes_shard(&shards[3]).await, "retained shard 4");
    assert!(chain.authorizes_shard(&shards[4]).await, "retained shard 5");

    // The recurring asset keeps only its surviving entry (block 5).
    let history = chain.asset_history(&[0xEE; 32]).await;
    assert_eq!(
        history.iter().map(|l| l.block_index).collect::<Vec<_>>(),
        scan_asset_history(&remaining, &[0xEE; 32])
            .iter()
            .map(|(b, _)| *b)
            .collect::<Vec<_>>(),
    );
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].block_index, 5);
    assert_eq!(chain.asset_head(&[0xEE; 32]).await.map(|l| l.block_index), Some(5));

    // A fully-pruned asset disappears entirely.
    assert!(chain.asset_history(&[0xD0 + 2; 32]).await.is_empty());
    assert!(chain.asset_head(&[0xD0 + 2; 32]).await.is_none());

    // And the pruned index equals a rebuild over the surviving blocks.
    assert_eq!(
        chain.asset_index_snapshot().await,
        AssetChainIndex::rebuild(remaining.iter()),
        "index after prune differs from a rebuild over the surviving blocks",
    );
}

/// S3.1 QA follow-up: a shard listed by BOTH a pruned block and a surviving one
/// must STILL authorize after `prune_to_headers`, and must resolve to the
/// SURVIVING block's asset.
///
/// This is the case where a naive "drop the key when a contributing block goes"
/// implementation silently de-authorizes a shard this node still serves — or,
/// worse, keeps a locator pointing at a block that is no longer held in full.
#[tokio::test]
async fn s3_1_shard_shared_by_pruned_and_surviving_block_still_authorizes() {
    let chain = NodeBlockchain::new(coord());

    // The SAME shard is listed by two DIFFERENT assets in two different blocks.
    let shared_shard = [0x5Au8; 32];
    let pruned_asset = [0x01u8; 32];
    let surviving_asset = [0x02u8; 32];

    chain
        .add_block(vec![sharded(pruned_asset, vec![shared_shard])])
        .await
        .expect("test: add_block 1");
    chain
        .add_block(vec![sharded(surviving_asset, vec![shared_shard])])
        .await
        .expect("test: add_block 2");

    // Before pruning, the earliest (block 1, the soon-to-be-pruned one) wins.
    assert!(chain.authorizes_shard(&shared_shard).await);
    assert_eq!(
        chain
            .registration_for_shard(&shared_shard)
            .await
            .expect("test: registration before prune")
            .asset_hash,
        pruned_asset,
    );

    // Prune block 1 only.
    chain.prune_to_headers(1..2).await;
    let remaining = chain.get_chain().await;
    assert_eq!(remaining.len(), 2, "genesis + block 2 remain in full");

    // The shard is STILL authorized — block 2 still lists it.
    assert!(
        chain.authorizes_shard(&shared_shard).await,
        "a shard still listed by a surviving block must stay authorized",
    );
    assert_eq!(
        chain.authorizes_shard(&shared_shard).await,
        scan_authorizes_shard(&remaining, &shared_shard),
    );

    // And it now resolves to the SURVIVING block's asset, not the pruned one.
    let registration = chain
        .registration_for_shard(&shared_shard)
        .await
        .expect("test: registration must still resolve after prune");
    assert_eq!(
        registration.asset_hash, surviving_asset,
        "must resolve to the surviving block's asset",
    );

    // The pruned asset is gone from the index; the surviving one is intact.
    assert!(chain.asset_history(&pruned_asset).await.is_empty());
    assert_eq!(chain.asset_history(&surviving_asset).await.len(), 1);

    // The pruned index still equals a rebuild over the surviving blocks.
    assert_eq!(
        chain.asset_index_snapshot().await,
        AssetChainIndex::rebuild(remaining.iter()),
    );
}
