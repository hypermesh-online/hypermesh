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

/// Address of one asset entry inside the chain: which block, which entry.
///
/// The block hash is carried alongside so a locator is self-describing — a
/// caller can name the block an asset's head lives in without re-reading the
/// `blocks` map, and S3.2's lineage pointers are block hashes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssetEntryLocator {
    /// Index of the block holding the entry.
    pub block_index: u64,
    /// Position of the entry within that block's `entries` vector.
    pub entry_ix: usize,
    /// Hash of the block holding the entry.
    pub block_hash: String,
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

/// In-memory per-asset index over a chain's blocks.
///
/// Derived state only: [`rebuild`](Self::rebuild) from the same block set
/// always yields an equal value, which is what the S3.1 rebuild proof asserts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetChainIndex {
    by_asset: HashMap<[u8; 32], Vec<AssetEntryLocator>>,
    heads: HashMap<[u8; 32], AssetEntryLocator>,
    by_shard: HashMap<[u8; 32], Vec<AssetEntryLocator>>,
    by_block: HashMap<u64, BlockContribution>,
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
