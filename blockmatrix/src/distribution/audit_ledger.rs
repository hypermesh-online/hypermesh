// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Bounded in-memory ledger backing the placement-audit trail.
//!
//! Append-only per asset, protected by two eviction bounds so it can never
//! grow without limit — the flaw in the previous unbounded static ledger,
//! which appended on every store/redistribution/migration forever.
//!
//! This is in-memory only. See [`super::audit_trail`] for the public audit
//! API and the FUTURE note on (deliberately un-built) on-chain persistence.

use crate::distribution::audit_trail::AuditRecord;
use std::collections::{HashMap, VecDeque};

/// Per-asset retention cap: max audit records kept per asset before the
/// oldest for that asset is evicted.
///
/// An asset's default Reed-Solomon layout is 14 shards (10+4), so 32 retains
/// roughly the two most-recent placement generations per asset — enough recent
/// history for local verification without unbounded growth.
pub(crate) const PER_ASSET_RECORD_CAP: usize = 32;

/// Global retention cap: max distinct assets tracked before the
/// oldest-first-seen asset is evicted wholesale.
pub(crate) const GLOBAL_ASSET_CAP: usize = 256;

/// Bounded in-memory placement-audit ledger.
///
/// # Memory bound (justified vs R13 4 GB min RAM)
///
/// Worst case = `GLOBAL_ASSET_CAP × PER_ASSET_RECORD_CAP` = 256 × 32 = 8 192
/// records. An `AuditRecord` is small: three short `String`s (asset_id,
/// node_id, tx_hash ≈ 64 B each) plus the `Migration` event's up-to-three
/// `String`s, plus fixed scalar fields — worst case ≈ 640 B/record including
/// allocator and `VecDeque` slack. Total worst case ≈ 5 MiB (~0.13 % of the
/// R13 4 GB minimum). Follows the same measure-and-bound discipline as the
/// F1 pool / foreign store.
#[derive(Default)]
pub(crate) struct BoundedAuditLedger {
    /// Per-asset ring of retained records (front = oldest).
    by_asset: HashMap<String, VecDeque<AuditRecord>>,
    /// Distinct asset keys in first-seen order, for global eviction.
    /// Invariant: its set of keys equals `by_asset`'s keys, no duplicates.
    asset_order: VecDeque<String>,
}

impl BoundedAuditLedger {
    /// Append one record, enforcing both eviction bounds.
    pub(crate) fn append(&mut self, record: AuditRecord) {
        let asset_id = record.asset_id.clone();
        let is_new_asset = !self.by_asset.contains_key(&asset_id);

        let queue = self.by_asset.entry(asset_id.clone()).or_default();
        queue.push_back(record);
        if queue.len() > PER_ASSET_RECORD_CAP {
            queue.pop_front(); // evict this asset's oldest record
        }

        if is_new_asset {
            self.asset_order.push_back(asset_id);
            self.evict_overflow_assets();
        }
    }

    /// Evict oldest-first-seen assets until at most `GLOBAL_ASSET_CAP` remain.
    fn evict_overflow_assets(&mut self) {
        while self.asset_order.len() > GLOBAL_ASSET_CAP {
            if let Some(oldest) = self.asset_order.pop_front() {
                self.by_asset.remove(&oldest);
            }
        }
    }

    /// All retained records for `asset_id`, oldest timestamp first.
    pub(crate) fn query(&self, asset_id: &str) -> Vec<AuditRecord> {
        let mut records: Vec<AuditRecord> = self
            .by_asset
            .get(asset_id)
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default();
        records.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        records
    }

    /// Whether the latest retained record for (`asset_id`, `shard_index`)
    /// places the shard on `expected_node`.
    pub(crate) fn verify(&self, asset_id: &str, shard_index: usize, expected_node: &str) -> bool {
        self.by_asset
            .get(asset_id)
            .and_then(|q| {
                q.iter()
                    .filter(|r| r.shard_index == shard_index)
                    .max_by_key(|r| r.timestamp)
            })
            .map(|r| r.node_id == expected_node)
            .unwrap_or(false)
    }

    /// (distinct assets, total records) — for bound assertions.
    #[cfg(test)]
    pub(crate) fn stats(&self) -> (usize, usize) {
        let records = self.by_asset.values().map(VecDeque::len).sum();
        (self.by_asset.len(), records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distribution::audit_trail::PlacementEvent;
    use crate::distribution::ShardPlacement;
    use crate::matrix::coordinate::MatrixCoordinate;

    /// Build an audit record tagged by `shard` so eviction order is checkable.
    fn tagged_record(asset_id: &str, shard: usize) -> AuditRecord {
        let placement = ShardPlacement {
            shard_index: shard,
            position: MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate"),
            node_id: "test-node".to_string(),
            octant: 0,
            distance_from_origin: 37.4,
        };
        AuditRecord::from_placement(asset_id, &placement, PlacementEvent::InitialPlacement)
    }

    #[test]
    fn test_ledger_bounded_per_asset_evicts_oldest_keeps_newest() {
        let mut ledger = BoundedAuditLedger::default();
        let asset = "hot-asset";
        let n = PER_ASSET_RECORD_CAP * 10; // hammer far past the cap

        for shard in 0..n {
            ledger.append(tagged_record(asset, shard));
        }

        let (assets, records) = ledger.stats();
        assert_eq!(assets, 1);
        assert!(
            records <= PER_ASSET_RECORD_CAP,
            "per-asset bound exceeded: {records} > {PER_ASSET_RECORD_CAP}"
        );

        let retained: Vec<usize> = ledger.query(asset).iter().map(|r| r.shard_index).collect();
        assert_eq!(retained.len(), PER_ASSET_RECORD_CAP);
        // Newest survives, oldest evicted (FIFO front-eviction).
        assert!(retained.contains(&(n - 1)), "newest record was evicted");
        assert!(!retained.contains(&0), "oldest record was not evicted");
    }

    #[test]
    fn test_ledger_bounded_global_assets_evicts_oldest_keeps_newest() {
        let mut ledger = BoundedAuditLedger::default();
        let n = GLOBAL_ASSET_CAP * 4; // hammer far past the asset cap

        for i in 0..n {
            ledger.append(tagged_record(&format!("asset-{i}"), 0));
        }

        let (assets, records) = ledger.stats();
        assert!(
            assets <= GLOBAL_ASSET_CAP,
            "global asset bound exceeded: {assets} > {GLOBAL_ASSET_CAP}"
        );
        assert!(records <= GLOBAL_ASSET_CAP * PER_ASSET_RECORD_CAP);
        // Oldest-first-seen asset evicted, most recent still queryable.
        assert!(ledger.query("asset-0").is_empty(), "oldest asset not evicted");
        assert!(
            !ledger.query(&format!("asset-{}", n - 1)).is_empty(),
            "most recent asset was evicted"
        );
    }
}
