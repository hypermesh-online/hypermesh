//! Deduplication Engine
//!
//! Core deduplication logic with O(1) HashMap lookups and matrix-aware shard placement.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use crate::assets::pipeline::PipelineError;

use crate::matrix::MatrixCoordinate;
use crate::assets::pipeline::Shard;
use super::{Hash, BucketId, HashBucket, ShardMetadata, BucketMapper, compute_hash};

/// Deduplication result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationResult {
    /// Whether the shard was deduplicated (already existed)
    pub deduplicated: bool,

    /// Matrix positions where shard is stored
    pub positions: Vec<MatrixCoordinate>,

    /// Space saved (or used if new shard)
    pub space_saved: usize,

    /// Shard hash
    pub shard_hash: Hash,

    /// Bucket ID where stored
    pub bucket_id: BucketId,

    /// Reference count after this operation
    pub reference_count: usize,
}

/// Deduplication statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeduplicationStats {
    /// Total shards processed
    pub total_processed: usize,

    /// Number of deduplicated shards
    pub deduplicated_count: usize,

    /// Number of unique shards
    pub unique_count: usize,

    /// Total space saved (bytes)
    pub space_saved: usize,

    /// Total space used (bytes)
    pub space_used: usize,

    /// Deduplication rate (0.0 to 1.0)
    pub deduplication_rate: f64,

    /// Average lookup time (microseconds)
    pub avg_lookup_time_us: u64,

    /// Number of active buckets
    pub active_buckets: usize,
}

/// Deduplication engine with O(1) lookups
pub struct DeduplicationEngine {
    /// Hash buckets (256 total, 00 to ff)
    buckets: Arc<RwLock<HashMap<BucketId, HashBucket>>>,

    /// Bucket mapper for matrix placement
    mapper: Arc<BucketMapper>,

    /// Content hash to shard hashes mapping
    content_map: Arc<RwLock<HashMap<Hash, Vec<Hash>>>>,

    /// Statistics
    stats: Arc<RwLock<DeduplicationStats>>,

    /// Performance metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

/// Performance metrics for monitoring
#[derive(Debug, Clone, Default)]
struct PerformanceMetrics {
    /// Histogram of lookup times
    lookup_times: Vec<u64>,

    /// Cache hit rate
    cache_hits: usize,
    cache_misses: usize,

    /// Bucket access counts
    bucket_accesses: HashMap<BucketId, usize>,
}

impl DeduplicationEngine {
    /// Create new deduplication engine
    pub fn new(mapper: Arc<BucketMapper>) -> Self {
        // Pre-create all 256 buckets for O(1) access
        let mut buckets = HashMap::new();
        for bucket_id in BucketId::all_buckets() {
            buckets.insert(bucket_id.clone(), HashBucket::new(bucket_id));
        }

        Self {
            buckets: Arc::new(RwLock::new(buckets)),
            mapper,
            content_map: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(DeduplicationStats::default())),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }

    /// Process a shard with deduplication
    pub async fn process_shard(&mut self, shard: Shard) -> Result<DeduplicationResult> {
        let start = std::time::Instant::now();

        // Compute shard hash
        let shard_hash = compute_hash(&shard.data);
        let bucket_id = BucketId::from_hash(&shard_hash);
        let shard_size = shard.data.len();

        // O(1) bucket lookup
        let mut buckets = self.buckets.write().await;
        let bucket = buckets.get_mut(&bucket_id)
            .ok_or_else(|| anyhow::anyhow!(
                "Bucket {:?} not found - deduplication engine may not be properly initialized", bucket_id
            ))?;

        // Record metrics
        let mut metrics = self.metrics.write().await;
        *metrics.bucket_accesses.entry(bucket_id.clone()).or_insert(0) += 1;

        // Check if shard exists (O(1) HashMap lookup)
        let result = if bucket.contains(&shard_hash) {
            // DEDUPLICATED! Shard already exists
            metrics.cache_hits += 1;

            // Record deduplication
            let metadata = bucket.record_deduplication(&shard_hash, shard_size)
                .ok_or_else(|| anyhow::anyhow!(
                    "Failed to record deduplication for shard - hash may not exist in bucket"
                ))?;

            DeduplicationResult {
                deduplicated: true,
                positions: metadata.positions.clone(),
                space_saved: shard_size,
                shard_hash,
                bucket_id: bucket_id.clone(),
                reference_count: metadata.reference_count,
            }
        } else {
            // NEW SHARD - find optimal positions
            metrics.cache_misses += 1;

            // Use bucket mapper to find optimal positions (14 for Reed-Solomon 10+4)
            let positions = self.mapper.optimal_positions(&bucket_id, 14).await?;

            // Add to bucket
            let metadata = bucket.add_shard(shard_hash, positions.clone(), shard_size);

            // Store actual shard data (in production, this would write to disk/network)
            // For now, we're just tracking metadata

            DeduplicationResult {
                deduplicated: false,
                positions: positions.clone(),
                space_saved: shard_size, // Actually space used for new shard
                shard_hash,
                bucket_id: bucket_id.clone(),
                reference_count: 1,
            }
        };

        // Update statistics
        let elapsed = start.elapsed().as_micros() as u64;
        metrics.lookup_times.push(elapsed);

        let mut stats = self.stats.write().await;
        stats.total_processed += 1;
        if result.deduplicated {
            stats.deduplicated_count += 1;
            stats.space_saved += result.space_saved;
        } else {
            stats.unique_count += 1;
            stats.space_used += result.space_saved; // Actually space used
        }

        // Calculate deduplication rate
        if stats.total_processed > 0 {
            stats.deduplication_rate = stats.deduplicated_count as f64 / stats.total_processed as f64;
        }

        // Update average lookup time
        stats.avg_lookup_time_us = if metrics.lookup_times.is_empty() {
            0
        } else {
            metrics.lookup_times.iter().sum::<u64>() / metrics.lookup_times.len() as u64
        };

        // Count active buckets
        stats.active_buckets = buckets.values()
            .filter(|b| !b.shard_hashes.is_empty())
            .count();

        Ok(result)
    }

    /// Check if shard exists (O(1) lookup)
    pub async fn check_exists(&self, shard_hash: Hash) -> Option<Vec<MatrixCoordinate>> {
        let bucket_id = BucketId::from_hash(&shard_hash);
        let mut buckets = self.buckets.write().await;

        if let Some(bucket) = buckets.get_mut(&bucket_id) {
            bucket.get_metadata(&shard_hash).map(|m| m.positions.clone())
        } else {
            None
        }
    }

    /// Get shard positions
    pub async fn get_shard_positions(&self, shard_hash: Hash) -> Result<Vec<MatrixCoordinate>> {
        self.check_exists(shard_hash).await
            .ok_or_else(|| anyhow::anyhow!("Shard not found"))
    }

    /// Add replica positions for a shard
    pub async fn add_replica_positions(
        &mut self,
        shard_hash: Hash,
        new_positions: Vec<MatrixCoordinate>,
    ) -> Result<()> {
        let bucket_id = BucketId::from_hash(&shard_hash);
        let mut buckets = self.buckets.write().await;

        if let Some(bucket) = buckets.get_mut(&bucket_id) {
            if let Some(metadata) = bucket.shard_hashes.get_mut(&shard_hash) {
                metadata.add_positions(new_positions);
                Ok(())
            } else {
                Err(anyhow::anyhow!("Shard not found in bucket"))
            }
        } else {
            Err(anyhow::anyhow!("Bucket not found"))
        }
    }

    /// Store content to shard mapping
    pub async fn store_content_mapping(&mut self, content_hash: Hash, shard_hashes: Vec<Hash>) -> Result<()> {
        let mut content_map = self.content_map.write().await;
        content_map.insert(content_hash, shard_hashes);
        Ok(())
    }

    /// Get retrieval instructions for content
    pub async fn get_retrieval_instructions(&self, content_hash: Hash) -> Result<super::RetrievalInstructions> {
        let content_map = self.content_map.read().await;

        let shard_hashes = content_map.get(&content_hash)
            .ok_or_else(|| anyhow::anyhow!("Content not found"))?;

        let mut shard_map = Vec::new();
        for shard_hash in shard_hashes {
            let positions = self.get_shard_positions(*shard_hash).await?;
            shard_map.push((*shard_hash, positions));
        }

        Ok(super::RetrievalInstructions::new(shard_map))
    }

    /// Get deduplication statistics
    pub fn get_stats(&self) -> DeduplicationStats {
        // Using block_on to avoid async in a non-async context
        // In production, this should be made async
        futures::executor::block_on(async {
            self.stats.read().await.clone()
        })
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> EngineMetrics {
        let metrics = self.metrics.read().await;
        let stats = self.stats.read().await;

        let cache_hit_rate = if metrics.cache_hits + metrics.cache_misses > 0 {
            metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64
        } else {
            0.0
        };

        EngineMetrics {
            total_lookups: metrics.lookup_times.len(),
            avg_lookup_time_us: stats.avg_lookup_time_us,
            cache_hit_rate,
            deduplication_rate: stats.deduplication_rate,
            active_buckets: stats.active_buckets,
            space_saved: stats.space_saved,
            space_used: stats.space_used,
        }
    }

    /// Find popular shards for replication
    pub async fn find_popular_shards(&self, threshold: f64) -> Vec<(Hash, ShardMetadata)> {
        let mut popular = Vec::new();
        let buckets = self.buckets.read().await;

        for bucket in buckets.values() {
            popular.extend(bucket.get_popular_shards(threshold));
        }

        // Sort by popularity score
        popular.sort_by(|a, b| {
            b.1.popularity_score.partial_cmp(&a.1.popularity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        popular
    }

    /// Verify O(1) lookup performance
    pub async fn verify_o1_performance(&self) -> bool {
        let metrics = self.metrics.read().await;
        if metrics.lookup_times.len() < 100 {
            return true; // Not enough data
        }

        // Calculate standard deviation
        let avg = metrics.lookup_times.iter().sum::<u64>() as f64 / metrics.lookup_times.len() as f64;
        let variance = metrics.lookup_times.iter()
            .map(|&t| {
                let diff = t as f64 - avg;
                diff * diff
            })
            .sum::<f64>() / metrics.lookup_times.len() as f64;
        let std_dev = variance.sqrt();

        // O(1) means lookup time should be relatively constant
        // Allow 50% coefficient of variation as threshold
        let coefficient_of_variation = std_dev / avg;
        coefficient_of_variation < 0.5
    }
}

/// Engine metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineMetrics {
    pub total_lookups: usize,
    pub avg_lookup_time_us: u64,
    pub cache_hit_rate: f64,
    pub deduplication_rate: f64,
    pub active_buckets: usize,
    pub space_saved: usize,
    pub space_used: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::phase1_foundation::{MatrixFoundation, MatrixFoundationConfig};

    async fn create_test_engine() -> DeduplicationEngine {
        let foundation = Arc::new(MatrixFoundation::new(MatrixFoundationConfig::default()).await.unwrap());
        let mapper = Arc::new(BucketMapper::new(foundation).await.unwrap());
        DeduplicationEngine::new(mapper)
    }

    #[tokio::test]
    async fn test_deduplication_new_shard() {
        let mut engine = create_test_engine().await;

        let shard = Shard {
            data: vec![1, 2, 3, 4],
            metadata: Default::default(),
        };

        let result = engine.process_shard(shard).await.unwrap();
        assert!(!result.deduplicated);
        assert_eq!(result.reference_count, 1);
        assert!(!result.positions.is_empty());
    }

    #[tokio::test]
    async fn test_deduplication_duplicate_shard() {
        let mut engine = create_test_engine().await;

        let shard1 = Shard {
            data: vec![1, 2, 3, 4],
            metadata: Default::default(),
        };

        let shard2 = Shard {
            data: vec![1, 2, 3, 4], // Same data
            metadata: Default::default(),
        };

        // First shard - new
        let result1 = engine.process_shard(shard1).await.unwrap();
        assert!(!result1.deduplicated);

        // Second shard - deduplicated
        let result2 = engine.process_shard(shard2).await.unwrap();
        assert!(result2.deduplicated);
        assert_eq!(result2.reference_count, 2);
        assert_eq!(result2.positions, result1.positions); // Same positions
    }

    #[tokio::test]
    async fn test_deduplication_rate_calculation() {
        let mut engine = create_test_engine().await;

        // Process 10 shards, 5 unique
        for i in 0..10 {
            let data = if i < 5 { vec![i] } else { vec![i - 5] }; // Duplicate last 5
            let shard = Shard {
                data,
                metadata: Default::default(),
            };
            engine.process_shard(shard).await.unwrap();
        }

        let stats = engine.get_stats();
        assert_eq!(stats.total_processed, 10);
        assert_eq!(stats.unique_count, 5);
        assert_eq!(stats.deduplicated_count, 5);
        assert_eq!(stats.deduplication_rate, 0.5); // 50% deduplication
    }

    #[tokio::test]
    async fn test_bucket_distribution() {
        let mut engine = create_test_engine().await;

        // Create shards that will go to different buckets
        for i in 0u8..=10 {
            let mut data = vec![i; 32];
            data[0] = i; // Different first byte = different bucket
            let shard = Shard {
                data,
                metadata: Default::default(),
            };
            engine.process_shard(shard).await.unwrap();
        }

        let stats = engine.get_stats();
        assert!(stats.active_buckets > 1); // Should use multiple buckets
    }

    #[tokio::test]
    async fn test_o1_lookup_verification() {
        let mut engine = create_test_engine().await;

        // Process many shards to get performance data
        for i in 0..200 {
            let shard = Shard {
                data: vec![i as u8; 100],
                metadata: Default::default(),
            };
            engine.process_shard(shard).await.unwrap();
        }

        // Verify O(1) performance
        assert!(engine.verify_o1_performance().await);
    }
}