// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! S3.1 — `AssetChainIndex`: a derived, in-memory per-asset view of the chain.
//!
//! # Why
//!
//! Before S3.1 every per-asset question was answered by a full linear scan of
//! the chain: `authorizes_shard` and `registration_for_shard` each walked
//! EVERY block × EVERY entry × every shard hash under a read lock on the whole
//! `blocks` map. That is O(blocks × entries × shards) per lookup, on the shard
//! serve path.
//!
//! This module replaces those scans with an index that is maintained
//! incrementally at the single insert chokepoint
//! (`NodeBlockchain::insert_block`) and rebuilt from the block set on load.
//! It is a **derived view** — it holds no authority and no data that is not
//! already in the blocks. Nothing here changes `Block` or `BlockAssetEntry`,
//! and nothing here is serialized: there is **zero format change**.
//!
//! # D1 — derived cache, never authority
//!
//! The authoritative per-asset object is
//! [`AssetLineage`](super::lineage::AssetLineage), addressed by each entry's
//! `lineage_id` (`= hex(proof_hash)`). `AssetChainIndex` is a **rebuildable
//! accelerator over that authority** — `by_asset` / `heads` / `by_shard` map an
//! asset (or shard) to WHERE its entries currently sit in block storage so a
//! lookup is O(1) instead of a full scan. It carries no truth the blocks do not
//! already hold: [`rebuild`](Self::rebuild) from the same block set always
//! yields an equal value (the invariant the S3.1 rebuild proof asserts), so the
//! whole structure can be thrown away and reconstructed at any time. The
//! [`AssetEntryLocator`] it returns is a **cache pointer into block storage**,
//! not the identity of an entry: `block_index` says which block to read, never
//! "which asset / which position in the lineage" — that is the `lineage_id`.
//!
//! # Shape
//!
//! Modeled on the existing prototype
//! [`ReceiptIndex::rebuild_from_blocks`](crate::assets::cross_chain::receipt_validator)
//! — an in-memory map populated by walking the chain.
//!
//! - `by_asset: asset_hash → Vec<AssetEntryLocator>` — the asset's history
//!   within this node's container, in chain order.
//! - `heads: asset_hash → AssetEntryLocator` — the most recent entry per asset.
//! - `by_shard: shard_hash → Vec<AssetEntryLocator>` — every entry whose
//!   `StoragePointer::Sharded` lists that shard, in chain order.
//! - `by_block: block_index → BlockContribution` — the reverse map that makes
//!   block REMOVAL (pruning) O(entries in that block) instead of O(index).
//!
//! # Many-to-many is mandatory
//!
//! One block can carry entries for N distinct assets — `register_asset_records`
//! batches every hardware asset into a single block at genesis. A naive
//! `asset_hash → block_index` map would be wrong. Every map here is keyed to a
//! `(block_index, entry_ix)` locator and every value is a `Vec`, so N assets in
//! one block and one asset across N blocks are both represented exactly.
//!
//! # Ordering
//!
//! Locators are kept sorted by `(block_index, entry_ix)` — CHAIN order, not
//! arrival order. Blocks do not always arrive in index order (received blocks,
//! orphan drains), so sorting on insert is what makes the incrementally
//! maintained index byte-identical to one rebuilt from the same block set.

use std::collections::HashMap;

use super::block::{Block, StoragePointer};

/// A cache pointer to where one asset entry currently sits in block storage:
/// which block, which entry.
///
/// D1: this is a **storage-fetch detail, never an identity**. `block_index`
/// answers "which block do I read to materialize this entry?", NOT "what is this
/// asset / where is it in its lineage?" — the authoritative address of an entry
/// is its `lineage_id` (`= hex(proof_hash)`), which is spine-offset-free. The
/// block hash is carried alongside so the pointer is self-describing (a caller
/// can name the block without re-reading the `blocks` map). Because the locator
/// is a derived cache value it is **in-memory only** — nothing here is
/// serialized, and it is reconstructed from the blocks on every load.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssetEntryLocator {
    /// Which block to read the entry from — a storage pointer, never identity.
    pub block_index: u64,
    /// Position of the entry within that block's `entries` vector.
    pub entry_ix: usize,
    /// Hash of the block holding the entry.
    pub block_hash: String,
}

/// The furthest an asset's chain has ever advanced in THIS container.
///
/// S3.2 QA F1 — the tombstone. Pruning drops an asset's entries from the index
/// (correctly: the index may never name a block the chain no longer holds in
/// full), but the asset's *identity* must outlive its entry bodies. Without
/// this record a pruned asset becomes UNKNOWN, and an unknown asset accepts a
/// fresh `(prev = None, seq = 0)` asset-genesis — a lineage-RESET primitive:
/// prune, then re-root the asset under someone else's history. The high-water
/// mark survives [`AssetChainIndex::remove_block`], so a pruned asset stays
/// KNOWN and a fresh root for it stays REJECTED.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetHighWater {
    /// `lineage_id` of the furthest entry ever indexed for this asset — what a
    /// legitimate continuation must name in `prev_asset_entry`.
    pub lineage_id: String,
    /// `asset_seq` of that entry. A continuation must carry `seq + 1`.
    pub seq: u64,
}

/// What one block contributed to the index — used to undo it on removal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BlockContribution {
    /// Asset hashes this block contributed entries for (with duplicates
    /// preserved: the same asset may appear twice in one block).
    assets: Vec<[u8; 32]>,
    /// Shard hashes this block contributed locators for.
    shards: Vec<[u8; 32]>,
}

/// In-memory per-asset index over a chain's blocks — a derived accelerator, not
/// an authority.
///
/// D1: the authority is [`AssetLineage`](super::lineage::AssetLineage), keyed by
/// `lineage_id`; this index only maps assets and shards to WHERE their entries
/// currently live in block storage. Derived state only:
/// [`rebuild`](Self::rebuild) from the same block set always yields an equal
/// value (the S3.1 rebuild proof), so it can be discarded and reconstructed at
/// will and holds no truth the blocks do not.
#[derive(Clone, Debug, Default, Eq)]
pub struct AssetChainIndex {
    by_asset: HashMap<[u8; 32], Vec<AssetEntryLocator>>,
    heads: HashMap<[u8; 32], AssetEntryLocator>,
    by_shard: HashMap<[u8; 32], Vec<AssetEntryLocator>>,
    by_block: HashMap<u64, BlockContribution>,
    /// F1 tombstones — monotonic per-asset high-water marks. Deliberately NOT
    /// part of [`PartialEq`]; see the impl below.
    high_water: HashMap<[u8; 32], AssetHighWater>,
}

/// Equality is over the DERIVED VIEW of the block set — `by_asset`, `heads`,
/// `by_shard`, `by_block` — and deliberately EXCLUDES `high_water`.
///
/// # Why the tombstone is out of the equality set
///
/// The S3.1 property under test is *"an index maintained incrementally equals
/// one rebuilt from the same blocks"*. `high_water` is not a function of the
/// current block set: it is a monotonic MEMORY of blocks this container once
/// held, and a rebuild from surviving blocks alone cannot reconstruct what
/// pruning erased. Including it would make the property false-by-construction
/// after any prune — the equality assertion would break without anything being
/// wrong, and the only way to restore it would be to drop the tombstone on
/// rebuild, which is exactly the lineage-reset F1 closes.
///
/// So the semantics are stated rather than papered over:
/// **tombstones are runtime-only, and a rebuild SEEDS them from surviving
/// blocks** (the highest seq still present per asset). That seed is a lower
/// bound, never an over-claim — it can only ever be `<=` the true high water,
/// so a rebuild never rejects a legitimate continuation it should accept. The
/// in-process guarantee (prune cannot re-root an asset) holds exactly where
/// the attack exists: `prune_to_headers` is an in-memory operation on a live
/// chain, and the tombstone lives for that chain's lifetime. A process
/// RESTART reloads full blocks from the durable sink, so the rebuilt index
/// recovers its history from the blocks themselves.
impl PartialEq for AssetChainIndex {
    fn eq(&self, other: &Self) -> bool {
        self.by_asset == other.by_asset
            && self.heads == other.heads
            && self.by_shard == other.by_shard
            && self.by_block == other.by_block
    }
}

impl AssetChainIndex {
    /// An empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an index from a block set (load / restart / `from_blocks`).
    ///
    /// Order-independent: because locators sort by `(block_index, entry_ix)`,
    /// feeding the same blocks in any order produces an equal index.
    pub fn rebuild<'a, I>(blocks: I) -> Self
    where
        I: IntoIterator<Item = &'a Block>,
    {
        let mut index = Self::new();
        for block in blocks {
            index.index_block(block);
        }
        index
    }

    /// Add every entry of `block` to the index.
    ///
    /// Idempotent: re-indexing a block index that is already present first
    /// removes the previous contribution, so the index can never double-count.
    pub fn index_block(&mut self, block: &Block) {
        if self.by_block.contains_key(&block.index) {
            self.remove_block(block.index);
        }

        let mut contribution = BlockContribution::default();

        for (entry_ix, entry) in block.entries.iter().enumerate() {
            let locator = AssetEntryLocator {
                block_index: block.index,
                entry_ix,
                block_hash: block.hash.clone(),
            };

            insert_sorted(
                self.by_asset.entry(entry.asset_hash).or_default(),
                locator.clone(),
            );
            contribution.assets.push(entry.asset_hash);
            self.refresh_head(&entry.asset_hash);
            self.raise_high_water(entry);

            if let StoragePointer::Sharded { shard_hashes, .. } = &entry.storage_pointer {
                for shard_hash in shard_hashes {
                    insert_sorted(
                        self.by_shard.entry(*shard_hash).or_default(),
                        locator.clone(),
                    );
                    contribution.shards.push(*shard_hash);
                }
            }
        }

        self.by_block.insert(block.index, contribution);
    }

    /// Remove every locator contributed by the block at `block_index`.
    ///
    /// Called when a full block leaves the chain (`prune_to_headers`), so the
    /// index can never point at a block the chain no longer holds.
    ///
    /// F1: the per-asset `high_water` tombstones are deliberately NOT removed.
    /// The entry BODIES go (they are no longer held), but the asset's identity
    /// survives, so a pruned asset stays known and cannot be re-rooted with a
    /// fresh `(None, 0)` genesis.
    pub fn remove_block(&mut self, block_index: u64) {
        let Some(contribution) = self.by_block.remove(&block_index) else {
            return;
        };

        for asset_hash in &contribution.assets {
            drop_locators(&mut self.by_asset, asset_hash, block_index);
            self.refresh_head(asset_hash);
        }
        for shard_hash in &contribution.shards {
            drop_locators(&mut self.by_shard, shard_hash, block_index);
        }
    }

    /// Drop all indexed state (used when the chain itself is reseeded).
    pub fn clear(&mut self) {
        self.by_asset.clear();
        self.heads.clear();
        self.by_shard.clear();
        self.by_block.clear();
        // The chain itself is being reseeded from a different root — this
        // container's memory of the OLD chain's assets is not evidence about
        // the new one, so the tombstones go too.
        self.high_water.clear();
    }

    /// F1 — the furthest this container has ever seen `asset_hash`'s chain
    /// advance, INCLUDING through entries that have since been pruned.
    ///
    /// This is what makes a pruned asset still KNOWN: lineage checks fall back
    /// to it when [`asset_head`](Self::asset_head) has nothing.
    pub fn asset_high_water(&self, asset_hash: &[u8; 32]) -> Option<&AssetHighWater> {
        self.high_water.get(asset_hash)
    }

    /// Whether this container has ever recorded an entry for `asset_hash`,
    /// even if every one of them has since been pruned.
    pub fn has_ever_seen_asset(&self, asset_hash: &[u8; 32]) -> bool {
        self.high_water.contains_key(asset_hash)
    }

    /// Every entry recorded for `asset_hash`, in chain order.
    ///
    /// This is the asset's history within this node's container — the
    /// foundation S3.2 (lineage) and S3.5 (transfer) build on.
    pub fn asset_history(&self, asset_hash: &[u8; 32]) -> &[AssetEntryLocator] {
        self.by_asset
            .get(asset_hash)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// The most recent entry recorded for `asset_hash`.
    pub fn asset_head(&self, asset_hash: &[u8; 32]) -> Option<&AssetEntryLocator> {
        self.heads.get(asset_hash)
    }

    /// Number of distinct assets known to the index.
    pub fn asset_count(&self) -> usize {
        self.by_asset.len()
    }

    /// Every asset hash known to the index (unordered).
    pub fn asset_hashes(&self) -> impl Iterator<Item = &[u8; 32]> {
        self.by_asset.keys()
    }

    /// Whether some on-chain asset is stored as shards including `shard_id`.
    ///
    /// O(1) replacement for the full-chain scan behind
    /// `NodeBlockchain::authorizes_shard`.
    pub fn authorizes_shard(&self, shard_id: &[u8; 32]) -> bool {
        self.by_shard.contains_key(shard_id)
    }

    /// Locator of the entry that authorizes `shard_id`, if any.
    ///
    /// Returns the EARLIEST such entry in chain order. The scan this replaces
    /// iterated a `HashMap`, so when several entries listed the same shard it
    /// returned an arbitrary one; picking the earliest is within the old
    /// semantics (same candidate set) and is deterministic, which the old
    /// behaviour was not.
    pub fn locator_for_shard(&self, shard_id: &[u8; 32]) -> Option<&AssetEntryLocator> {
        self.by_shard.get(shard_id).and_then(|v| v.first())
    }

    /// Every entry that lists `shard_id`, in chain order. Used by the
    /// equivalence proof to compare against a brute-force scan.
    pub fn locators_for_shard(&self, shard_id: &[u8; 32]) -> &[AssetEntryLocator] {
        self.by_shard
            .get(shard_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// F1 — advance an asset's tombstone to cover `entry`, monotonically.
    ///
    /// Takes the MAX by `asset_seq`, so it is insensitive to block arrival
    /// order (received blocks and orphan drains do not arrive in index order)
    /// and idempotent under re-indexing the same block.
    fn raise_high_water(&mut self, entry: &super::block::BlockAssetEntry) {
        let seq = entry.asset_seq();
        match self.high_water.get(&entry.asset_hash) {
            Some(existing) if existing.seq >= seq => {}
            _ => {
                self.high_water.insert(
                    entry.asset_hash,
                    AssetHighWater {
                        lineage_id: entry.lineage_id(),
                        seq,
                    },
                );
            }
        }
    }

    /// Recompute the head for one asset from its (sorted) history.
    ///
    /// Derived rather than tracked, so `heads` cannot drift out of sync with
    /// `by_asset` on insert, removal or re-index.
    fn refresh_head(&mut self, asset_hash: &[u8; 32]) {
        match self.by_asset.get(asset_hash).and_then(|v| v.last()) {
            Some(head) => {
                self.heads.insert(*asset_hash, head.clone());
            }
            None => {
                self.heads.remove(asset_hash);
            }
        }
    }
}

/// Insert `locator` keeping the vector sorted by `(block_index, entry_ix)`,
/// skipping an exact duplicate.
fn insert_sorted(slot: &mut Vec<AssetEntryLocator>, locator: AssetEntryLocator) {
    match slot.binary_search(&locator) {
        Ok(_) => {}
        Err(pos) => slot.insert(pos, locator),
    }
}

/// Remove every locator pointing at `block_index` from `map[key]`, dropping the
/// key entirely once it holds nothing.
fn drop_locators(
    map: &mut HashMap<[u8; 32], Vec<AssetEntryLocator>>,
    key: &[u8; 32],
    block_index: u64,
) {
    if let Some(slot) = map.get_mut(key) {
        slot.retain(|loc| loc.block_index != block_index);
        if slot.is_empty() {
            map.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::AssetRegistration;
    use crate::blockchain::block::BlockAssetEntry;
    use crate::matrix::coordinate::MatrixCoordinate;
    use trustchain::proof_of_state::StateProof;

    fn coord() -> MatrixCoordinate {
        MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate")
    }

    fn entry(tag: u8, pointer: StoragePointer) -> BlockAssetEntry {
        let reg = AssetRegistration::genesis(coord());
        BlockAssetEntry::new_bound(
            [tag; 32],
            &StateProof::new_for_testing(),
            pointer,
            reg,
        )
    }

    fn sharded(tag: u8, shards: Vec<[u8; 32]>) -> BlockAssetEntry {
        entry(
            tag,
            StoragePointer::Sharded {
                shard_hashes: shards,
                placements: vec![coord()],
            },
        )
    }

    #[test]
    fn multi_asset_block_indexes_every_asset() {
        let block = Block::new(
            1,
            vec![
                entry(0xA1, StoragePointer::Genesis),
                entry(0xA2, StoragePointer::Genesis),
                entry(0xA3, StoragePointer::Genesis),
            ],
            "prev".to_string(),
        );
        let index = AssetChainIndex::rebuild([&block]);

        assert_eq!(index.asset_count(), 3);
        for (ix, tag) in [0xA1u8, 0xA2, 0xA3].iter().enumerate() {
            let history = index.asset_history(&[*tag; 32]);
            assert_eq!(history.len(), 1);
            assert_eq!(history[0].block_index, 1);
            assert_eq!(history[0].entry_ix, ix);
        }
    }

    #[test]
    fn history_is_chain_ordered_regardless_of_index_order() {
        let b1 = Block::new(1, vec![entry(0xB0, StoragePointer::Genesis)], "p".into());
        let b2 = Block::new(2, vec![entry(0xB0, StoragePointer::Genesis)], "p".into());
        let b3 = Block::new(3, vec![entry(0xB0, StoragePointer::Genesis)], "p".into());

        let forward = AssetChainIndex::rebuild([&b1, &b2, &b3]);
        let shuffled = AssetChainIndex::rebuild([&b3, &b1, &b2]);
        assert_eq!(forward, shuffled);

        let history = forward.asset_history(&[0xB0; 32]);
        assert_eq!(
            history.iter().map(|l| l.block_index).collect::<Vec<_>>(),
            vec![1, 2, 3],
        );
        assert_eq!(
            forward.asset_head(&[0xB0; 32]).map(|l| l.block_index),
            Some(3),
        );
    }

    #[test]
    fn shard_lookup_and_removal() {
        let shard = [0x77u8; 32];
        let block = Block::new(1, vec![sharded(0xC1, vec![shard])], "p".into());
        let mut index = AssetChainIndex::rebuild([&block]);

        assert!(index.authorizes_shard(&shard));
        assert!(!index.authorizes_shard(&[0x00; 32]));
        assert_eq!(
            index.locator_for_shard(&shard).map(|l| l.block_index),
            Some(1),
        );

        index.remove_block(1);
        assert!(!index.authorizes_shard(&shard));
        assert!(index.asset_head(&[0xC1; 32]).is_none());
        assert_eq!(index.asset_count(), 0);
    }

    /// F1 — the tombstone survives what the locators do not.
    #[test]
    fn remove_block_keeps_the_asset_high_water_tombstone() {
        let asset = [0xE1u8; 32];
        let mut e0 = entry(0xE1, StoragePointer::Genesis);
        e0.set_asset_lineage(None, 0);
        let mut e1 = entry(0xE1, StoragePointer::Genesis);
        e1.set_asset_lineage(Some(e0.lineage_id()), 1);
        let expected_id = e1.lineage_id();

        let b1 = Block::new(1, vec![e0], "p".into());
        let b2 = Block::new(2, vec![e1], b1.hash.clone());
        let mut index = AssetChainIndex::rebuild([&b1, &b2]);

        index.remove_block(1);
        index.remove_block(2);

        // Locators are gone — the index must never name a block the chain no
        // longer holds in full.
        assert!(index.asset_head(&asset).is_none());
        assert!(index.asset_history(&asset).is_empty());
        assert_eq!(index.asset_count(), 0);

        // The asset's IDENTITY is not gone: it stays known at its high water,
        // so a fresh `(None, 0)` root for it can still be refused.
        assert!(index.has_ever_seen_asset(&asset));
        let hw = index.asset_high_water(&asset).expect("test: tombstone");
        assert_eq!(hw.seq, 1, "high water is the furthest seq ever indexed");
        assert_eq!(hw.lineage_id, expected_id);

        // `clear` (chain reseeded from a different root) does drop it.
        index.clear();
        assert!(!index.has_ever_seen_asset(&asset));
    }

    /// F1 — the tombstone is monotonic and order-insensitive.
    #[test]
    fn high_water_takes_the_max_regardless_of_arrival_order() {
        let asset = [0xE2u8; 32];
        let mut e0 = entry(0xE2, StoragePointer::Genesis);
        e0.set_asset_lineage(None, 0);
        let mut e1 = entry(0xE2, StoragePointer::Genesis);
        e1.set_asset_lineage(Some(e0.lineage_id()), 1);

        let b1 = Block::new(1, vec![e0], "p".into());
        let b2 = Block::new(2, vec![e1], b1.hash.clone());

        // Out-of-order arrival (received blocks / orphan drains) must not
        // lower the high water.
        let reverse = AssetChainIndex::rebuild([&b2, &b1]);
        assert_eq!(
            reverse.asset_high_water(&asset).map(|hw| hw.seq),
            Some(1),
        );
        // ...and re-indexing is idempotent for the tombstone too.
        let mut again = reverse.clone();
        again.index_block(&b1);
        assert_eq!(again.asset_high_water(&asset).map(|hw| hw.seq), Some(1));
    }

    #[test]
    fn reindexing_the_same_block_is_idempotent() {
        let block = Block::new(1, vec![sharded(0xD1, vec![[0x99; 32]])], "p".into());
        let mut index = AssetChainIndex::new();
        index.index_block(&block);
        let once = index.clone();
        index.index_block(&block);
        assert_eq!(index, once);
        assert_eq!(index.asset_history(&[0xD1; 32]).len(), 1);
    }
}
