//! Client Assembly
//!
//! Client-side shard fetching and file reconstruction from retrieval instructions.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::matrix::MatrixCoordinate;
use crate::assets::storage::Hash;

use super::{RetrievalPlan, ShardLocation};

/// Progress of assembly operation
#[derive(Debug, Clone)]
pub struct AssemblyProgress {
    /// Total shards needed
    pub total_shards: usize,

    /// Shards successfully fetched
    pub fetched_shards: usize,

    /// Shards currently being fetched
    pub in_progress: usize,

    /// Shards failed to fetch
    pub failed_shards: usize,

    /// Percentage complete (0.0 to 1.0)
    pub percentage: f64,

    /// Estimated time remaining (milliseconds)
    pub estimated_remaining_ms: u64,
}

impl AssemblyProgress {
    /// Check if assembly is complete
    pub fn is_complete(&self, min_required: usize) -> bool {
        self.fetched_shards >= min_required
    }

    /// Check if assembly has failed
    pub fn is_failed(&self, min_required: usize) -> bool {
        let available = self.total_shards - self.failed_shards;
        available < min_required
    }
}

/// Statistics for assembly operation
#[derive(Debug, Clone)]
pub struct AssemblyStats {
    /// Total bytes fetched
    pub bytes_fetched: usize,

    /// Total time taken (milliseconds)
    pub total_time_ms: u64,

    /// Average fetch time per shard (milliseconds)
    pub avg_shard_time_ms: u64,

    /// Number of fallback attempts
    pub fallback_attempts: usize,

    /// Number of parallel fetches
    pub parallel_fetches: usize,

    /// Throughput (bytes per second)
    pub throughput_bps: u64,
}

impl AssemblyStats {
    /// Calculate throughput in MB/s
    pub fn throughput_mbps(&self) -> f64 {
        self.throughput_bps as f64 / (1024.0 * 1024.0)
    }
}

/// Fetched shard data
#[derive(Debug, Clone)]
struct FetchedShard {
    /// Shard hash
    hash: Hash,

    /// Shard data
    data: Vec<u8>,

    /// Position it was fetched from
    source: MatrixCoordinate,

    /// Time taken to fetch (milliseconds)
    fetch_time_ms: u64,
}

/// Client assembler for reconstructing files from instructions
pub struct ClientAssembler {
    /// Current retrieval plan
    plan: Arc<RwLock<Option<RetrievalPlan>>>,

    /// Fetched shards storage
    fetched_shards: Arc<RwLock<HashMap<usize, FetchedShard>>>,

    /// Assembly progress
    progress: Arc<RwLock<AssemblyProgress>>,

    /// Statistics
    stats: Arc<RwLock<AssemblyStats>>,

    /// Maximum parallel fetches
    max_parallel: usize,
}

impl ClientAssembler {
    /// Create a new client assembler
    pub fn new(max_parallel: usize) -> Self {
        Self {
            plan: Arc::new(RwLock::new(None)),
            fetched_shards: Arc::new(RwLock::new(HashMap::new())),
            progress: Arc::new(RwLock::new(AssemblyProgress {
                total_shards: 0,
                fetched_shards: 0,
                in_progress: 0,
                failed_shards: 0,
                percentage: 0.0,
                estimated_remaining_ms: 0,
            })),
            stats: Arc::new(RwLock::new(AssemblyStats {
                bytes_fetched: 0,
                total_time_ms: 0,
                avg_shard_time_ms: 0,
                fallback_attempts: 0,
                parallel_fetches: 0,
                throughput_bps: 0,
            })),
            max_parallel,
        }
    }

    /// Initialize with retrieval plan
    pub async fn initialize(&self, plan: RetrievalPlan) -> Result<()> {
        // Validate plan
        plan.validate()?;

        let total_shards = plan.shard_map.entries.len();

        // Set plan
        *self.plan.write().await = Some(plan);

        // Initialize progress
        let mut progress = self.progress.write().await;
        progress.total_shards = total_shards;
        progress.fetched_shards = 0;
        progress.in_progress = 0;
        progress.failed_shards = 0;
        progress.percentage = 0.0;

        Ok(())
    }

    /// Fetch all shards according to retrieval plan
    pub async fn fetch_shards(&self) -> Result<()> {
        let start = std::time::Instant::now();

        // Get plan data
        let plan_data = {
            let plan = self.plan.read().await;
            let plan = plan.as_ref()
                .ok_or_else(|| anyhow::anyhow!("No retrieval plan set"))?;

            plan.retrieval_order.iter()
                .filter_map(|idx| {
                    plan.shard_map.get_entry(*idx).map(|entry| {
                        (*idx, entry.shard_hash, entry.locations.clone())
                    })
                })
                .collect::<Vec<_>>()
        };

        // Fetch shards sequentially for simplicity (parallel version would need futures::FuturesUnordered)
        for (shard_idx, shard_hash, locations) in plan_data {
            Self::fetch_shard_from_locations(
                shard_idx,
                shard_hash,
                locations,
                self.fetched_shards.clone(),
                self.progress.clone(),
                self.stats.clone(),
            ).await?;
        }

        // Update final stats
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
    async fn fetch_shard_from_locations(
        shard_idx: usize,
        shard_hash: Hash,
        locations: Vec<ShardLocation>,
        fetched_shards: Arc<RwLock<HashMap<usize, FetchedShard>>>,
        progress: Arc<RwLock<AssemblyProgress>>,
        stats: Arc<RwLock<AssemblyStats>>,
    ) -> Result<()> {
        // Mark as in progress
        {
            let mut prog = progress.write().await;
            prog.in_progress += 1;
        }

        // Try each location in order
        let mut last_error = None;

        for (attempt, location) in locations.iter().enumerate() {
            let fetch_start = std::time::Instant::now();

            match Self::fetch_from_location(&location.position, &shard_hash).await {
                Ok(data) => {
                    let fetch_time = fetch_start.elapsed().as_millis() as u64;

                    // Store fetched shard
                    let fetched = FetchedShard {
                        hash: shard_hash,
                        data: data.clone(),
                        source: location.position.clone(),
                        fetch_time_ms: fetch_time,
                    };

                    let data_size = data.len();

                    fetched_shards.write().await.insert(shard_idx, fetched);

                    // Update progress
                    {
                        let mut prog = progress.write().await;
                        prog.fetched_shards += 1;
                        prog.in_progress -= 1;
                        prog.percentage = prog.fetched_shards as f64 / prog.total_shards as f64;
                    }

                    // Update stats
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

        // All locations failed
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
        // In production, this would:
        // 1. Connect to node at position
        // 2. Request shard by hash
        // 3. Verify received data matches hash
        // 4. Return shard data

        // For now, simulate with dummy data
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        Ok(vec![0u8; 1024]) // 1KB dummy shard
    }

    /// Reconstruct file from fetched shards
    pub async fn reconstruct(&self) -> Result<Vec<u8>> {
        let plan = self.plan.read().await;
        let plan = plan.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No retrieval plan set"))?;

        let fetched = self.fetched_shards.read().await;

        // Check if we have enough shards
        if fetched.len() < plan.min_shards_required {
            return Err(anyhow::anyhow!(
                "Insufficient shards: have {}, need {}",
                fetched.len(),
                plan.min_shards_required
            ));
        }

        // In production, this would:
        // 1. Decrypt shards if encrypted
        // 2. Use Reed-Solomon to reconstruct from available shards
        // 3. Decompress if compressed
        // 4. Verify content hash

        // For now, concatenate dummy data
        let mut reconstructed = Vec::new();
        for i in 0..plan.min_shards_required {
            if let Some(shard) = fetched.get(&i) {
                reconstructed.extend_from_slice(&shard.data);
            }
        }

        Ok(reconstructed)
    }

    /// Get current progress
    pub async fn get_progress(&self) -> AssemblyProgress {
        self.progress.read().await.clone()
    }

    /// Get statistics
    pub async fn get_stats(&self) -> AssemblyStats {
        self.stats.read().await.clone()
    }

    /// Reset assembler for new retrieval
    pub async fn reset(&self) {
        *self.plan.write().await = None;
        self.fetched_shards.write().await.clear();

        let mut progress = self.progress.write().await;
        *progress = AssemblyProgress {
            total_shards: 0,
            fetched_shards: 0,
            in_progress: 0,
            failed_shards: 0,
            percentage: 0.0,
            estimated_remaining_ms: 0,
        };

        let mut stats = self.stats.write().await;
        *stats = AssemblyStats {
            bytes_fetched: 0,
            total_time_ms: 0,
            avg_shard_time_ms: 0,
            fallback_attempts: 0,
            parallel_fetches: 0,
            throughput_bps: 0,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retrieval::{CompleteShardMap, RetrievalMetadata, ShardMapEntry};

    fn create_test_plan() -> RetrievalPlan {
        let content_hash = [1u8; 32];
        let mut shard_map = CompleteShardMap::new();

        // Create 14 shards
        for i in 0..14 {
            let shard_hash = [i as u8; 32];
            let locations = vec![
                ShardLocation::new(MatrixCoordinate::new(i as i64, 0, 0).unwrap(), 0.9),
            ];
            let entry = ShardMapEntry::new(shard_hash, locations);
            shard_map.add_entry(entry);
        }

        let metadata = RetrievalMetadata {
            erasure_coding: (10, 4),
            compression: "brotli".to_string(),
            encryption: "aes-256-gcm".to_string(),
            content_type: "application/octet-stream".to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };

        RetrievalPlan::new(content_hash, shard_map, metadata)
    }

    #[tokio::test]
    async fn test_assembler_creation() {
        let assembler = ClientAssembler::new(4);
        let progress = assembler.get_progress().await;
        assert_eq!(progress.total_shards, 0);
    }

    #[tokio::test]
    async fn test_initialize() {
        let assembler = ClientAssembler::new(4);
        let plan = create_test_plan();

        let result = assembler.initialize(plan).await;
        assert!(result.is_ok());

        let progress = assembler.get_progress().await;
        assert_eq!(progress.total_shards, 14);
    }

    #[tokio::test]
    async fn test_fetch_shards() {
        let assembler = ClientAssembler::new(4);
        let plan = create_test_plan();

        assembler.initialize(plan).await.unwrap();
        let result = assembler.fetch_shards().await;
        assert!(result.is_ok());

        let progress = assembler.get_progress().await;
        assert!(progress.fetched_shards > 0);
    }

    #[tokio::test]
    async fn test_reconstruct() {
        let assembler = ClientAssembler::new(4);
        let plan = create_test_plan();

        assembler.initialize(plan).await.unwrap();
        assembler.fetch_shards().await.unwrap();

        let reconstructed = assembler.reconstruct().await;
        assert!(reconstructed.is_ok());

        let data = reconstructed.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_progress_tracking() {
        let assembler = ClientAssembler::new(4);
        let plan = create_test_plan();

        assembler.initialize(plan).await.unwrap();

        let progress_before = assembler.get_progress().await;
        assert_eq!(progress_before.percentage, 0.0);

        assembler.fetch_shards().await.unwrap();

        let progress_after = assembler.get_progress().await;
        assert!(progress_after.percentage > 0.0);
        assert!(progress_after.is_complete(10)); // Min required is 10
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let assembler = ClientAssembler::new(4);
        let plan = create_test_plan();

        assembler.initialize(plan).await.unwrap();
        assembler.fetch_shards().await.unwrap();

        let stats = assembler.get_stats().await;
        assert!(stats.bytes_fetched > 0);
        assert!(stats.total_time_ms > 0);
        assert_eq!(stats.parallel_fetches, 4);
    }

    #[tokio::test]
    async fn test_reset() {
        let assembler = ClientAssembler::new(4);
        let plan = create_test_plan();

        assembler.initialize(plan).await.unwrap();
        assembler.fetch_shards().await.unwrap();

        let progress_before = assembler.get_progress().await;
        assert!(progress_before.fetched_shards > 0);

        assembler.reset().await;

        let progress_after = assembler.get_progress().await;
        assert_eq!(progress_after.fetched_shards, 0);
        assert_eq!(progress_after.total_shards, 0);
    }
}
