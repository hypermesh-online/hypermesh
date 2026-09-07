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

use crate::blockchain::block::StoragePointer;
use crate::blockchain::NodeBlockchain;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::consumer_provider::ConsumerProviderManager;
use crate::network::shard_store::ShardStore;
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
    /// Local shard store — the chain-shard-set seed path consults it to register
    /// this node as a provider ONLY for shards it actually holds (never advertise
    /// bytes we cannot serve).
    shard_store: Arc<ShardStore>,
    /// This node's matrix position (the demand cell recorded for seeded shards).
    position: MatrixPosition,
    /// This node's id (the demand consumer recorded for seeded shards, and the
    /// provider string registered in the location index via [`NodeId::to_hex`]).
    node_id: NodeId,
}

impl SwarmSeeder {
    /// Build a seeder from the daemon's existing handles.
    pub fn new(
        consumer_provider: Arc<ConsumerProviderManager>,
        index: Arc<ShardLocationIndex>,
        analytics: Arc<Mutex<SwarmAnalytics>>,
        shard_store: Arc<ShardStore>,
        coord: MatrixCoordinate,
        node_id: NodeId,
    ) -> Self {
        Self {
            consumer_provider,
            index,
            analytics,
            shard_store,
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

    /// Seed a shard set discovered FROM THE CHAIN entry (hashes only, no bytes).
    ///
    /// This is the propagated-version path: a new asset head arrived via block
    /// propagation carrying a `StoragePointer::Sharded { shard_hashes }`, and the
    /// observer bridges that on-chain set into the LIVE swarm state without any
    /// content bytes and without a catalog dependency:
    ///
    /// - REFLECT: for each shard hash this node actually HOLDS locally
    ///   ([`ShardStore::has`]), register THIS node as a provider in `network` so
    ///   consumers (and the mirror loop's candidate gather) can find it. Shards
    ///   the node does not hold are never advertised.
    /// - MIRROR trigger: feed [`SwarmAnalytics`] for the WHOLE set (demand +
    ///   true replica count) so the [`DmsDriver`](ngauge::DmsDriver) mirror loop
    ///   fires for any shard still below `min_replicas`.
    ///
    /// Returns the shard hashes this node registered itself as a provider for.
    pub async fn seed_onchain_shard_set(
        &self,
        network: NetworkId,
        shard_hashes: &[[u8; 32]],
    ) -> Vec<ContentHash> {
        if shard_hashes.is_empty() {
            return Vec::new();
        }
        let hashes: Vec<ContentHash> = shard_hashes.iter().map(|h| ContentHash(*h)).collect();

        // REFLECT: register self ONLY for shards actually held (byte-free — the
        // bytes are already in the local store, placed there by the node hosting
        // its own asset version).
        let mut held: Vec<ContentHash> = Vec::with_capacity(hashes.len());
        for hash in &hashes {
            if self.shard_store.has(hash).await {
                held.push(*hash);
            }
        }
        if !held.is_empty() {
            self.index
                .register_provider_in_network(network, &self.node_id.to_hex(), &held)
                .await;
        }

        // MIRROR trigger: demand + true replica count for the whole set.
        self.feed_analytics(network, &hashes).await;
        held
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

    /// One diff-and-seed pass driven ENTIRELY by the chain (no content source,
    /// no catalog). For each asset whose head has advanced, read the head entry's
    /// `StoragePointer::Sharded { shard_hashes }` FROM THE CHAIN and seed that set
    /// into every joined `network` via [`SwarmSeeder::seed_onchain_shard_set`].
    ///
    /// This is the daemon-wired path: it covers BOTH the local-author case (the
    /// node stored the bytes, so it registers itself as a provider) and the
    /// propagated case (bytes absent, so only demand is fed and the mirror loop
    /// pulls copies). Heads whose pointer is not `Sharded` (e.g. genesis hardware
    /// assets) are marked examined and skipped so they are not rescanned.
    ///
    /// Returns the asset hashes whose shard set was seeded this pass.
    ///
    /// Lock discipline: the `seeded` guard is taken only for the synchronous diff
    /// and the synchronous mark; the per-head chain read + seed happen with NO
    /// `seeded` guard held (and the seeder never holds the analytics guard across
    /// an `.await`).
    pub async fn poll_once_from_chain(
        &self,
        chain: &NodeBlockchain,
        seeder: &SwarmSeeder,
        networks: &[NetworkId],
    ) -> Vec<[u8; 32]> {
        if networks.is_empty() {
            return Vec::new();
        }
        let snapshot = chain.asset_index_snapshot().await;
        let pending = self.pending_heads(&snapshot);

        let mut seeded = Vec::new();
        for (asset_hash, key) in pending {
            // Mark the head examined regardless of outcome so a non-Sharded head
            // (genesis asset) is not rescanned every tick; a later advance to a
            // Sharded entry changes the key and is re-detected.
            self.mark_seeded(asset_hash, key);

            let Some(shard_hashes) = head_shard_set(chain, &snapshot, &asset_hash).await else {
                continue;
            };
            if shard_hashes.is_empty() {
                continue;
            }
            for network in networks {
                seeder.seed_onchain_shard_set(*network, &shard_hashes).await;
            }
            seeded.push(asset_hash);
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

/// Read the on-chain shard set for `asset_hash`'s current head.
///
/// The `snapshot` locates the head entry `(block_index, entry_ix)`; the block is
/// materialized from `chain` and the entry's [`StoragePointer`] is inspected.
/// Returns the shard hashes iff the pointer is [`StoragePointer::Sharded`],
/// otherwise `None` (genesis / registration entries carry no shard set).
async fn head_shard_set(
    chain: &NodeBlockchain,
    snapshot: &crate::blockchain::asset_index::AssetChainIndex,
    asset_hash: &[u8; 32],
) -> Option<Vec<[u8; 32]>> {
    let head = snapshot.asset_head(asset_hash)?;
    let block = chain.get_block(head.block_index).await?;
    let entry = block.entries.get(head.entry_ix)?;
    match &entry.storage_pointer {
        StoragePointer::Sharded { shard_hashes, .. } => Some(shard_hashes.clone()),
        _ => None,
    }
}

/// Spawn the DMS handoff observer loop (Phase 4, chain-shard-set path).
///
/// Every 20s the observer diffs the asset-chain heads read from `&NodeBlockchain`
/// alone and seeds each newly-headed version's on-chain `StoragePointer::Sharded`
/// set into the swarm the E.2 poll loop already drives — REFLECTing held shards
/// (register self as provider) and feeding demand so the mirror loop converges.
///
/// Reads ONLY `&NodeBlockchain` for versions (they arrive via block propagation),
/// the joined-network set (via the sync manager), and the daemon's existing
/// swarm handles. No content bytes, no catalog dependency.
pub(super) fn spawn(svc: &super::ReplicationService) {
    let chain = svc.blockchain.clone();
    let sync_manager = svc.sync_manager.clone();
    let seeder = Arc::new(SwarmSeeder::new(
        svc.consumer_provider.clone(),
        svc.shard_location_index.clone(),
        svc.ngauge_analytics.clone(),
        svc.shard_store.clone(),
        svc.coord,
        node_id_from_hex(&svc.node_id),
    ));
    let observer = Arc::new(ChainHeadObserver::new());

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(20));
        // Skip the immediate tick so the first pass runs after bring-up settles.
        interval.tick().await;
        loop {
            interval.tick().await;
            let networks = super::joined_networks(&sync_manager).await;
            if networks.is_empty() {
                continue;
            }
            let seeded = observer
                .poll_once_from_chain(&chain, &seeder, &networks)
                .await;
            if !seeded.is_empty() {
                tracing::info!(
                    "dms-handoff: seeded {} on-chain shard set(s) into the swarm",
                    seeded.len()
                );
            }
        }
    });
    tracing::info!("DMS handoff observer loop started (interval=20s)");
}

/// Reconstruct a [`NodeId`] from the daemon's hex node-id string.
///
/// The daemon id is `hex(BLAKE3(FALCON pubkey))`; decoding it yields the SAME
/// 32 bytes whose `to_hex()` the location index is keyed by, so the seeder
/// registers under the identical provider string other components use. A
/// non-hex id (test fixtures) falls back to hashing the string — self-consistent
/// within that process.
fn node_id_from_hex(node_id: &str) -> NodeId {
    hex::decode(node_id)
        .ok()
        .and_then(|v| <[u8; 32]>::try_from(v).ok())
        .map(NodeId::from_bytes)
        .unwrap_or_else(|| NodeId::from_public_key(node_id.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::handler::RequestHandler;
    use crate::ipc::protocol::RpcRequest;
    use hypermesh_lib::DEFAULT_NETWORK;
    use ngauge::{DmsDriver, ShardCandidates};

    /// Phase-4 DMS handoff, daemon/component level, with ZERO catalog import and
    /// NO manual seeding:
    ///
    /// 1. The node HOSTS a version through the real store/host path (the `store`
    ///    IPC entrypoint), which shards + stores locally and records the shard
    ///    set on-chain as `StoragePointer::Sharded`.
    /// 2. The daemon-wired [`ChainHeadObserver::poll_once_from_chain`] reads the
    ///    new asset head from `&NodeBlockchain` ALONE and seeds its on-chain shard
    ///    set into the live swarm handles.
    /// 3. Assert the shard set is registered in the live [`ShardLocationIndex`]
    ///    AND [`DmsDriver::plan`] produces a mirror for it.
    ///
    /// In-process/component (not 2-node QUIC): the handoff seam is entirely local
    /// state (chain read → index/analytics writes → plan), so a component test
    /// exercises the exact daemon code path deterministically without transport
    /// flakiness.
    #[tokio::test]
    async fn daemon_observer_mirrors_a_hosted_version_from_chain() {
        let state = crate::ipc::handlers::tests::test_state().await;

        // (1) Host a version through the node store/host path (IPC `store`).
        let tmp = tempfile::TempDir::new().expect("test: tmpdir");
        let file_path = tmp.path().join("version.bin");
        let data = b"HYPERMESH-DMS-PHASE4-HANDOFF-".repeat(500);
        std::fs::write(&file_path, &data).expect("test: write payload");

        let mut handler = RequestHandler::new();
        crate::ipc::handlers::store::register(&mut handler, &state);
        let resp = handler
            .dispatch(RpcRequest::new(
                "store",
                serde_json::json!({ "path": file_path.to_string_lossy() }),
            ))
            .await;
        assert!(resp.error.is_none(), "store must succeed: {:?}", resp.error);
        let result = resp.result.expect("test: store result present");
        let shard_hashes: Vec<[u8; 32]> = result["shard_hashes"]
            .as_array()
            .expect("test: shard_hashes array")
            .iter()
            .map(|h| {
                let hex = h.as_str().expect("test: shard hash is hex");
                <[u8; 32]>::try_from(hex::decode(hex).expect("test: decode hex"))
                    .expect("test: 32-byte hash")
            })
            .collect();
        assert!(!shard_hashes.is_empty(), "asset must produce shards");

        // Build the daemon's swarm handles (test_state carries none of these).
        let network = DEFAULT_NETWORK;
        let index = Arc::new(ShardLocationIndex::new());
        let analytics = Arc::new(Mutex::new(SwarmAnalytics::new()));
        let consumer_provider = Arc::new(ConsumerProviderManager::new(
            state.shard_store.clone(),
            index.clone(),
            state.node_id.clone(),
            network,
        ));
        let seeder = SwarmSeeder::new(
            consumer_provider,
            index.clone(),
            analytics.clone(),
            state.shard_store.clone(),
            state.coordinate,
            node_id_from_hex(&state.node_id),
        );
        let observer = ChainHeadObserver::new();

        // (2) The EXACT daemon-wired path: read heads from &NodeBlockchain only,
        // seed the on-chain shard set. No manual seeding, no catalog.
        let asset_hash = *blake3::hash(&data).as_bytes();
        let seeded = observer
            .poll_once_from_chain(&state.blockchain, &seeder, &[network])
            .await;
        assert!(
            seeded.iter().any(|a| *a == asset_hash),
            "the hosted version's asset head must be seeded from the chain",
        );

        // (3a) The shard set is registered in the LIVE ShardLocationIndex.
        let provider_id = node_id_from_hex(&state.node_id).to_hex();
        for shard in &shard_hashes {
            let providers = index
                .get_providers_in_network(network, &ContentHash(*shard))
                .await;
            assert!(
                providers.contains(&provider_id),
                "shard {} must register this node as provider",
                hex::encode(&shard[..4]),
            );
        }

        // (3b) DmsDriver::plan mirrors the seeded set. Gather candidates from the
        // SAME live index (no hand-seeding), then plan under the analytics guard.
        let mut bundles = Vec::new();
        for shard in &shard_hashes {
            let providers = index
                .get_providers_in_network(network, &ContentHash(*shard))
                .await;
            bundles.push(ShardCandidates {
                shard_id: ContentHash(*shard),
                positioned: Vec::new(),
                all_ids: providers,
            });
        }
        let plan = {
            let guard = analytics.lock().expect("test: analytics lock");
            DmsDriver::plan(&guard, network, &bundles, 0.5)
        };
        assert!(
            !plan.mirror.is_empty(),
            "DmsDriver must mirror the under-replicated on-chain shard set",
        );
    }
}
