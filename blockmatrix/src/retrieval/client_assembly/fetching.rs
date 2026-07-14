// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Shard fetching: real shard retrieval from matrix positions via `ShardTransport`.
//!
//! A2: per-shard location selection goes through the shared two-layer resolver
//! ([`crate::retrieval::location_resolver`]) — live mirrors first, then the
//! plan's canonical matrix placements — so the client-assembly path and the
//! live IPC path share ONE resolution authority. Every fetched + BLAKE3-verified
//! shard is routed through the optional become-provider seeder, so fetching
//! ALWAYS triggers the consumer-becomes-provider re-announce (R12) exactly as
//! the IPC path does.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use hypermesh_lib::NodeId;

use crate::assets::storage::Hash;
use crate::matrix::MatrixCoordinate;
use crate::network::shard_transport::ShardTransport;
use crate::network::swarm_provider::ShardLocationIndex;
use crate::retrieval::location_resolver::{
    coordinate_to_node_id, resolve_shard_locations, ProviderSource,
};

use super::seeding::ShardSeeder;
use super::{AssemblyProgress, AssemblyStats, ClientAssembler, FetchedShard, ShardLocation};

impl ClientAssembler {
    /// Fetch all shards using a real `ShardTransport` implementation.
    ///
    /// Each shard's providers are resolved via the shared two-layer resolver
    /// (live mirrors first, then the plan's canonical matrix placements) and the
    /// transport fetches the bytes, falling through providers on failure. A
    /// fetched, verified shard is re-announced through the seeder when one is
    /// attached (consumer-becomes-provider, R12).
    pub async fn fetch_shards_via_transport(
        &self,
        transport: &dyn ShardTransport,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        let plan_data = {
            let plan = self.plan.read().await;
            let plan = plan
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("No retrieval plan set"))?;

            plan.retrieval_order
                .iter()
                .filter_map(|idx| {
                    plan.shard_map
                        .get_entry(*idx)
                        .map(|entry| (*idx, entry.shard_hash, entry.locations.clone()))
                })
                .collect::<Vec<_>>()
        };

        for (shard_idx, shard_hash, locations) in plan_data {
            Self::fetch_shard_via_transport(
                shard_idx,
                shard_hash,
                locations,
                transport,
                self.live_index.clone(),
                self.seeder.clone(),
                self.fetched_shards.clone(),
                self.progress.clone(),
                self.stats.clone(),
            )
            .await?;
        }

        self.finalize_stats(start.elapsed().as_millis() as u64)
            .await;

        Ok(())
    }

    /// Fetch a single shard via transport, trying resolved providers in order.
    #[allow(clippy::too_many_arguments)]
    async fn fetch_shard_via_transport(
        shard_idx: usize,
        shard_hash: Hash,
        locations: Vec<ShardLocation>,
        transport: &dyn ShardTransport,
        live_index: Option<Arc<ShardLocationIndex>>,
        seeder: Option<Arc<dyn ShardSeeder>>,
        fetched_shards: Arc<RwLock<HashMap<usize, FetchedShard>>>,
        progress: Arc<RwLock<AssemblyProgress>>,
        stats: Arc<RwLock<AssemblyStats>>,
    ) -> Result<()> {
        {
            let mut prog = progress.write().await;
            prog.in_progress += 1;
        }

        let content_hash = hypermesh_lib::ContentHash(shard_hash);

        // A2 two-layer resolve: live mirrors first, then canonical placements.
        // The plan's `ShardLocation`s ARE the canonical matrix coordinates.
        let canonical_coords: Vec<MatrixCoordinate> =
            locations.iter().map(|loc| loc.position).collect();
        let resolved =
            resolve_shard_locations(&content_hash, live_index.as_deref(), &canonical_coords).await;

        // Map each resolved provider to (NodeId, source-coordinate). Live-mirror
        // and upstream providers carry a hex node id we parse directly; canonical
        // placements carry the coordinate whose owning node id we derive.
        let mut ordered: Vec<(NodeId, MatrixCoordinate)> = Vec::with_capacity(resolved.len());
        for provider in &resolved {
            match &provider.source {
                ProviderSource::CanonicalPlacement { coordinate } => {
                    ordered.push((coordinate_to_node_id(coordinate), *coordinate));
                }
                ProviderSource::LiveMirror | ProviderSource::UpstreamTracker => {
                    if let Some(node_id) = node_id_from_hex(&provider.node_id) {
                        // Live-mirror providers are real peers with no matrix
                        // coordinate of their own; record the origin coordinate
                        // for provenance (used only for the FetchedShard source
                        // tag, never for addressing).
                        ordered.push((node_id, MatrixCoordinate::origin()));
                    }
                }
            }
        }

        let mut last_error = None;

        for (attempt, (node_id, source_coord)) in ordered.into_iter().enumerate() {
            let fetch_start = std::time::Instant::now();

            match transport.fetch_shard(&node_id, &content_hash).await {
                Ok(data) => {
                    // Content-validity gate (mirror invariant #1, F4): the
                    // received shard MUST hash to its claimed content address.
                    // A forged/corrupt shard (data != claimed hash) is rejected
                    // and treated as a fetch failure so fallback locations are
                    // tried — never stored, never fed to reconstruction, never
                    // re-announced.
                    let computed = *blake3::hash(&data).as_bytes();
                    if computed != shard_hash {
                        last_error = Some(anyhow::anyhow!(
                            "shard content-hash mismatch: expected {}, got {}",
                            hex::encode(shard_hash),
                            hex::encode(computed),
                        ));
                        continue;
                    }

                    let fetch_time = fetch_start.elapsed().as_millis() as u64;

                    let fetched = FetchedShard {
                        _hash: shard_hash,
                        data: data.clone(),
                        _source: source_coord,
                        _fetch_time_ms: fetch_time,
                    };

                    let data_size = data.len();
                    fetched_shards.write().await.insert(shard_idx, fetched);

                    // A2 unification: consumer-becomes-provider re-announce.
                    // Only AFTER the BLAKE3 content gate passes. When no seeder
                    // is wired (pure tests / Private mode) the shard is simply
                    // held locally — matching the IPC path's fallback.
                    if let Some(ref seeder) = seeder {
                        seeder.seed(content_hash, data).await;
                    }

                    Self::update_progress_success(
                        &progress, &stats, data_size, fetch_time, attempt,
                    )
                    .await;

                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(anyhow::anyhow!("{}", e));
                    continue;
                }
            }
        }

        {
            let mut prog = progress.write().await;
            prog.failed_shards += 1;
            prog.in_progress -= 1;
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All locations failed")))
    }

    /// Update progress and stats after a successful fetch.
    async fn update_progress_success(
        progress: &Arc<RwLock<AssemblyProgress>>,
        stats: &Arc<RwLock<AssemblyStats>>,
        data_size: usize,
        fetch_time: u64,
        attempt: usize,
    ) {
        {
            let mut prog = progress.write().await;
            prog.fetched_shards += 1;
            prog.in_progress -= 1;
            prog.percentage = prog.fetched_shards as f64 / prog.total_shards as f64;
        }

        {
            let mut st = stats.write().await;
            st.bytes_fetched += data_size;
            st.avg_shard_time_ms =
                (st.avg_shard_time_ms * (st.fallback_attempts as u64) + fetch_time)
                    / (st.fallback_attempts as u64 + 1);
            if attempt > 0 {
                st.fallback_attempts += attempt;
            }
        }
    }

    /// Finalize stats after all shards are fetched.
    async fn finalize_stats(&self, elapsed_ms: u64) {
        let mut stats = self.stats.write().await;
        stats.total_time_ms = elapsed_ms;
        stats.parallel_fetches = self.max_parallel;

        if elapsed_ms > 0 {
            stats.throughput_bps = (stats.bytes_fetched as u64 * 1000) / elapsed_ms;
        }
    }

    /// End-to-end asset retrieval via `ShardTransport`.
    ///
    /// Performs the full reconstruction pipeline:
    /// 1. Fetch shards from matrix nodes via transport
    /// 2. Reed-Solomon reconstruct the encrypted blob
    /// 3. Decrypt using the provided key
    /// 4. Decompress to recover original data
    ///
    /// This is the primary entry point for instruction-based retrieval.
    pub async fn retrieve_asset(
        &self,
        transport: &dyn ShardTransport,
        decryption_key: &crate::assets::pipeline::orchestrator::DecryptionKey,
    ) -> Result<Vec<u8>> {
        // Step 1: Fetch all shards via the real transport
        self.fetch_shards_via_transport(transport).await?;

        // Step 2-4: Reconstruct (RS decode -> decrypt -> decompress)
        self.reconstruct_with_pipeline(decryption_key).await
    }
}

/// Derive a `NodeId` from a matrix coordinate (deterministic).
///
/// Delegates to [`crate::retrieval::location_resolver::coordinate_to_node_id`]
/// so the transport fetch path and the two-layer location resolver share ONE
/// coordinate→node derivation authority (A2). Kept as a re-export here for the
/// existing callers/tests that reference it by this path.
pub fn node_id_from_coordinate(coord: &MatrixCoordinate) -> hypermesh_lib::NodeId {
    crate::retrieval::location_resolver::coordinate_to_node_id(coord)
}

/// Parse a 64-char hex node id (as held by the live-mirror index) into a
/// [`NodeId`]. Returns `None` for malformed ids so a garbled index entry is
/// skipped rather than crashing the fetch.
fn node_id_from_hex(hex_id: &str) -> Option<NodeId> {
    let bytes = hex::decode(hex_id).ok()?;
    let arr: [u8; 32] = bytes.try_into().ok()?;
    Some(NodeId::from_bytes(arr))
}
