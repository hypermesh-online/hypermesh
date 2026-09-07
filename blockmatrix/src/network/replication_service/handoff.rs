// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DMS handoff — Catalog-VCS writes a version, the swarm mirrors + reflects it.
//!
//! Phase 4 of the Catalog-VCS / NGauge-DMS roadmap. Catalog writes a version
//! entry onto the asset's own chain (the bus); blockmatrix OBSERVES the new
//! asset head and SEEDS that version's shard set into the SAME swarm the
//! replication poll loop already drives. No direct catalog → ngauge wire: the
//! asset-chain is the only coupling.
//!
//! Two collaborators, both leveraging existing core systems:
//!
//! - [`SwarmSeeder`] — the seed primitive. Shards content with the ngauge
//!   [`Sharder`], then hands the shards to the EXISTING
//!   [`ConsumerProviderManager::process_fetched_shards`] (store → register-self
//!   as provider in [`ShardLocationIndex`] → build a TAG_SHARD_ANNOUNCE payload
//!   — the REFLECT/seed side), and finally feeds [`SwarmAnalytics`] so the
//!   Phase-1 [`ngauge::DmsDriver`] mirror loop fires on the next tick (the
//!   MIRROR side is the existing `poll.rs` executor — NOT re-implemented here).
//! - [`ChainHeadObserver`] — the trigger. Diffs the [`AssetChainIndex`] head set
//!   against what it last seeded; each newly-advanced head is a new version,
//!   whose content it resolves via [`VersionContentSource`] and seeds. Pure
//!   `poll_once` (deterministic, testable); a background loop is a thin wrapper.
//!
//! Analytics discipline (feed.rs:28): the `std::sync::Mutex<SwarmAnalytics>`
//! guard is taken only for the synchronous demand/replica-count writes and is
//! never held across an `.await`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use async_trait::async_trait;
use hypermesh_lib::{ContentHash, MatrixPosition, NetworkId, NodeId};
use ngauge::{Sharder, SwarmAnalytics};

use crate::blockchain::NodeBlockchain;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::consumer_provider::ConsumerProviderManager;
use crate::network::swarm_provider::ShardLocationIndex;

/// The content of a newly-headed asset version, resolved for seeding.
#[derive(Debug, Clone)]
pub struct VersionContent {
    /// The network the asset participates in — the `(NetworkId, ContentHash)`
    /// key half every swarm index access is scoped by.
    pub network: NetworkId,
    /// The raw version content bytes to shard and seed.
    pub bytes: Vec<u8>,
}

/// Resolves a newly-headed asset version's content for the observer.
///
/// The on-chain entry retains only the version's `content_hash` (not the source
/// bytes), so the observer cannot reconstruct content from the chain alone — it
/// asks a source that DOES hold the bytes. In the daemon that source is backed
/// by the catalog registry (which holds the authored typedef content); the
/// direction stays catalog → blockmatrix (never blockmatrix → catalog), so no
/// dependency cycle is introduced. A `None` return means "not a version this
/// node should seed" (e.g. an asset owned by another producer) and is skipped.
#[async_trait]
pub trait VersionContentSource: Send + Sync {
    /// Return the network + content bytes for the version now at `asset_hash`'s
    /// head, or `None` if this node has no content to seed for it.
    async fn content_for(&self, asset_hash: &[u8; 32]) -> Option<VersionContent>;
}

/// Seeds a version's content into the swarm so NGauge drives its distribution.
///
/// Holds only clones of Arcs the daemon already owns — no new state machine.
pub struct SwarmSeeder {
    /// Consumer-becomes-provider path: store + register-self + announce (R12).
    consumer_provider: Arc<ConsumerProviderManager>,
    /// Shared provider index — queried for the true post-seed replica count.
    index: Arc<ShardLocationIndex>,
    /// Swarm analytics — demand + replica-count feed that fires the mirror loop.
    analytics: Arc<Mutex<SwarmAnalytics>>,
    /// This node's matrix position (the demand cell recorded for seeded shards).
    position: MatrixPosition,
    /// This node's id (the demand consumer recorded for seeded shards).
    node_id: NodeId,
}

impl SwarmSeeder {
    /// Build a seeder from the daemon's existing handles.
    pub fn new(
        consumer_provider: Arc<ConsumerProviderManager>,
        index: Arc<ShardLocationIndex>,
        analytics: Arc<Mutex<SwarmAnalytics>>,
        coord: MatrixCoordinate,
        node_id: NodeId,
    ) -> Self {
        Self {
            consumer_provider,
            index,
            analytics,
            position: MatrixPosition {
                x: coord.x as f64,
                y: coord.y as f64,
                z: coord.z as f64,
            },
            node_id,
        }
    }

    /// Shard `content` (ngauge [`Sharder`]) and seed the resulting shard set into
    /// `network`. Returns the content-addressed hashes of the shards seeded.
    pub async fn seed_content(
        &self,
        network: NetworkId,
        content: &[u8],
    ) -> Result<Vec<ContentHash>> {
        let shards = shard_content(content)?;
        self.seed_shards(network, shards).await
    }

    /// Seed an already-sharded set: store + register-self + announce (REFLECT)
    /// via the existing consumer-provider path, then feed analytics (MIRROR
    /// trigger). Returns the seeded shard hashes.
    pub async fn seed_shards(
        &self,
        network: NetworkId,
        shards: Vec<(ContentHash, Vec<u8>)>,
    ) -> Result<Vec<ContentHash>> {
        if shards.is_empty() {
            return Ok(Vec::new());
        }
        let hashes: Vec<ContentHash> = shards.iter().map(|(h, _)| *h).collect();

        // REFLECT/seed side: store locally, register THIS node as a provider in
        // the asset's network, and produce the TAG_SHARD_ANNOUNCE payload a
        // consumer resolves against. Reuses the existing R12 primitive as-is.
        self.consumer_provider
            .process_fetched_shards_in_network(network, shards)
            .await;

        self.feed_analytics(network, &hashes).await;
        Ok(hashes)
    }

    /// Feed [`SwarmAnalytics`]: record demand for each seeded shard (so the
    /// [`ReplicationTrigger`](ngauge::ReplicationTrigger) tracks it) and set its
    /// replica count to the true provider count (below `min_replicas` on a fresh
    /// seed → the mirror loop fires and converges to the device pool target).
    ///
    /// The provider counts are read (async, no lock) BEFORE the analytics guard
    /// is taken; the guard then covers only synchronous writes and is dropped
    /// before returning — never held across an `.await`.
    async fn feed_analytics(&self, network: NetworkId, hashes: &[ContentHash]) {
        let mut counts: Vec<(ContentHash, u32)> = Vec::with_capacity(hashes.len());
        for hash in hashes {
            let n = self.index.get_providers_in_network(network, hash).await.len() as u32;
            counts.push((*hash, n));
        }
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        if let Ok(mut guard) = self.analytics.lock() {
            for (hash, replicas) in counts {
                guard.record_request_in_network(network, hash, self.node_id, self.position, ts);
                guard.set_replica_count_in_network(network, hash, replicas);
            }
        }
    }
}

/// Shard `content` with the default ngauge [`Sharder`] into content-addressed
/// `(hash, bytes)` pairs. The hash is BLAKE3 of the shard bytes — the same key
/// the store, provider index, and analytics are all keyed by.
pub fn shard_content(content: &[u8]) -> Result<Vec<(ContentHash, Vec<u8>)>> {
    let sharder = Sharder::default()
        .map_err(|e| anyhow::anyhow!("failed to build sharder: {e}"))?;
    let (shards, _stats) = sharder
        .shard(content)
        .map_err(|e| anyhow::anyhow!("sharding failed: {e}"))?;
    Ok(shards
        .into_iter()
        .map(|s| {
            let hash = ContentHash(*blake3::hash(&s.data).as_bytes());
            (hash, s.data)
        })
        .collect())
}

/// Observes the asset-chain for newly-advanced heads and seeds each new version.
///
/// The asset-chain is the handoff bus: Catalog appends a version entry, the head
/// for that asset advances, and this observer notices and seeds. `poll_once` is
/// the deterministic unit (one diff-and-seed pass); a production loop simply
/// calls it on an interval.
#[derive(Default)]
pub struct ChainHeadObserver {
    /// The head locator last SEEDED per asset. A head that differs from the
    /// recorded one is a new version to seed; equal means already handled.
    seeded: Mutex<HashMap<[u8; 32], (u64, usize)>>,
}

impl ChainHeadObserver {
    /// A fresh observer that has seeded nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// One diff-and-seed pass. Snapshots the [`AssetChainIndex`], seeds every
    /// asset whose head has advanced since the last pass (resolving content via
    /// `source`), and records the heads actually seeded. Returns the asset
    /// hashes seeded this pass.
    ///
    /// Lock discipline: the `seeded` guard is taken only for the synchronous
    /// diff (to build the work list) and again for the synchronous mark; the
    /// per-version content resolution + seed happen with NO lock held.
    pub async fn poll_once(
        &self,
        chain: &NodeBlockchain,
        seeder: &SwarmSeeder,
        source: &dyn VersionContentSource,
    ) -> Vec<[u8; 32]> {
        let snapshot = chain.asset_index_snapshot().await;
        let pending = self.pending_heads(&snapshot);

        let mut seeded = Vec::new();
        for (asset_hash, key) in pending {
            let Some(content) = source.content_for(&asset_hash).await else {
                continue;
            };
            match seeder.seed_content(content.network, &content.bytes).await {
                Ok(hashes) if !hashes.is_empty() => {
                    self.mark_seeded(asset_hash, key);
                    seeded.push(asset_hash);
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!(
                        "dms-handoff: seeding {} failed: {e}",
                        hex::encode(&asset_hash[..4])
                    );
                }
            }
        }
        seeded
    }

    /// Build the list of `(asset_hash, head_key)` whose head has advanced since
    /// the last seeded pass. Pure + synchronous — holds only the `seeded` guard.
    fn pending_heads(
        &self,
        snapshot: &crate::blockchain::asset_index::AssetChainIndex,
    ) -> Vec<([u8; 32], (u64, usize))> {
        let guard = match self.seeded.lock() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };
        let mut pending = Vec::new();
        for asset_hash in snapshot.asset_hashes() {
            let Some(head) = snapshot.asset_head(asset_hash) else {
                continue;
            };
            let key = (head.block_index, head.entry_ix);
            if guard.get(asset_hash) != Some(&key) {
                pending.push((*asset_hash, key));
            }
        }
        pending
    }

    /// Record `key` as the head last seeded for `asset_hash`.
    fn mark_seeded(&self, asset_hash: [u8; 32], key: (u64, usize)) {
        if let Ok(mut guard) = self.seeded.lock() {
            guard.insert(asset_hash, key);
        }
    }
}
