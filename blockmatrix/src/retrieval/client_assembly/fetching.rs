// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Shard fetching: sequential/parallel shard retrieval from matrix positions.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::assets::storage::Hash;
use crate::matrix::MatrixCoordinate;

use super::{AssemblyProgress, AssemblyStats, ClientAssembler, FetchedShard, ShardLocation};

impl ClientAssembler {
    /// Fetch all shards according to retrieval plan
    pub async fn fetch_shards(&self) -> Result<()> {
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
            Self::fetch_shard_from_locations(
                shard_idx,
                shard_hash,
                locations,
                self.fetched_shards.clone(),
                self.progress.clone(),
                self.stats.clone(),
            )
            .await?;
        }

        let elapsed = start.elapsed().as_millis() as u64;
        let mut stats = self.stats.write().await;
        stats.total_time_ms = elapsed;
        stats.parallel_fetches = self.max_parallel;

        if elapsed > 0 {
            stats.throughput_bps = (stats.bytes_fetched as u64 * 1000) / elapsed;
        }

        Ok(())
    }

    /// Fetch a single shard from available locations
    pub(super) async fn fetch_shard_from_locations(
        shard_idx: usize,
        shard_hash: Hash,
        locations: Vec<ShardLocation>,
        fetched_shards: Arc<RwLock<HashMap<usize, FetchedShard>>>,
        progress: Arc<RwLock<AssemblyProgress>>,
        stats: Arc<RwLock<AssemblyStats>>,
    ) -> Result<()> {
        {
            let mut prog = progress.write().await;
            prog.in_progress += 1;
        }

        let mut last_error = None;

        for (attempt, location) in locations.iter().enumerate() {
            let fetch_start = std::time::Instant::now();

            match Self::fetch_from_location(&location.position, &shard_hash).await {
                Ok(data) => {
                    let fetch_time = fetch_start.elapsed().as_millis() as u64;

                    let fetched = FetchedShard {
                        _hash: shard_hash,
                        data: data.clone(),
                        _source: location.position,
                        _fetch_time_ms: fetch_time,
                    };

                    let data_size = data.len();

                    fetched_shards.write().await.insert(shard_idx, fetched);

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

                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
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

    /// Fetch shard data from a specific location (placeholder)
    async fn fetch_from_location(
        _position: &MatrixCoordinate,
        _shard_hash: &Hash,
    ) -> Result<Vec<u8>> {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        Ok(vec![0u8; 1024])
    }
}
