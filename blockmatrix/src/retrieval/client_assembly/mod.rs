// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Client Assembly
//!
//! Client-side shard fetching and file reconstruction from retrieval instructions.

mod fetching;
mod pipeline;

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
pub(crate) struct FetchedShard {
    /// Shard hash
    pub(crate) _hash: Hash,

    /// Shard data
    pub(crate) data: Vec<u8>,

    /// Position it was fetched from
    pub(crate) _source: MatrixCoordinate,

    /// Time taken to fetch (milliseconds)
    pub(crate) _fetch_time_ms: u64,
}

/// Client assembler for reconstructing files from instructions
pub struct ClientAssembler {
    /// Current retrieval plan
    pub(crate) plan: Arc<RwLock<Option<RetrievalPlan>>>,

    /// Fetched shards storage
    pub(crate) fetched_shards: Arc<RwLock<HashMap<usize, FetchedShard>>>,

    /// Assembly progress
    pub(crate) progress: Arc<RwLock<AssemblyProgress>>,

    /// Statistics
    pub(crate) stats: Arc<RwLock<AssemblyStats>>,

    /// Maximum parallel fetches
    pub(crate) max_parallel: usize,
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
        plan.validate()?;

        let total_shards = plan.shard_map.entries.len();

        *self.plan.write().await = Some(plan);

        let mut progress = self.progress.write().await;
        progress.total_shards = total_shards;
        progress.fetched_shards = 0;
        progress.in_progress = 0;
        progress.failed_shards = 0;
        progress.percentage = 0.0;

        Ok(())
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
    use crate::retrieval::{
        RetrievalMetadata, CompleteShardMap, ShardMapEntry, ShardLocation,
    };
    use crate::assets::pipeline::EncryptionConfig;

    fn create_test_plan() -> RetrievalPlan {
        let content_hash = [1u8; 32];
        let mut shard_map = CompleteShardMap::new();

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
            encrypted_blob_size: 0,
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
        assert!(progress_after.is_complete(10));
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

    #[tokio::test]
    async fn test_reconstruct_with_pipeline_roundtrip() {
        use crate::assets::pipeline::{
            Asset, AssetMetadata as PipelineAssetMetadata,
            AssetPipeline,
        };

        let original_data = b"Hello, HyperMesh instruction-based retrieval! ".repeat(200);
        let asset = Asset {
            id: "test-pipeline-roundtrip".to_string(),
            data: original_data.clone(),
            metadata: PipelineAssetMetadata {
                name: "test.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: original_data.len(),
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let pipeline = AssetPipeline::default()
            .expect("test: create pipeline");
        let processed = pipeline.process_asset(asset).await
            .expect("test: process asset");

        let content_hash = [42u8; 32];
        let mut shard_map = CompleteShardMap::new();

        for (i, shard) in processed.shards.iter().enumerate() {
            let shard_hash = *blake3::hash(&shard.data).as_bytes();
            let position = MatrixCoordinate::new(i as i64, 0, 0)
                .expect("test: create coordinate");
            let location = ShardLocation::new(position, 1.0);
            let entry = ShardMapEntry::new(shard_hash, vec![location]);
            shard_map.add_entry(entry);
        }

        let metadata = RetrievalMetadata {
            erasure_coding: (10, 4),
            compression: "brotli".to_string(),
            encryption: "kyber-1024".to_string(),
            content_type: "application/octet-stream".to_string(),
            created_at: 1234567890,
            encrypted_blob_size: processed.stats.encryption.encrypted_size,
        };

        let mut plan = RetrievalPlan::new(content_hash, shard_map, metadata);
        plan.original_size = original_data.len();

        let assembler = ClientAssembler::new(4);
        assembler.initialize(plan).await.expect("test: init plan");

        {
            let mut fetched = assembler.fetched_shards.write().await;
            for (i, shard) in processed.shards.iter().enumerate() {
                let position = MatrixCoordinate::new(i as i64, 0, 0)
                    .expect("test: create coordinate");
                fetched.insert(i, FetchedShard {
                    _hash: [0u8; 32],
                    data: shard.data.clone(),
                    _source: position,
                    _fetch_time_ms: 0,
                });
            }

            let mut progress = assembler.progress.write().await;
            progress.fetched_shards = processed.shards.len();
            progress.percentage = 1.0;
        }

        let reconstructed = assembler
            .reconstruct_with_pipeline(&processed.decryption_key)
            .await
            .expect("test: reconstruct with pipeline");

        assert_eq!(
            reconstructed, original_data,
            "Pipeline round-trip: reconstructed data must match original"
        );
    }

    #[tokio::test]
    async fn test_reconstruct_with_pipeline_aes_fallback() {
        use crate::assets::pipeline::{
            Asset, AssetMetadata as PipelineAssetMetadata,
            AssetPipeline, PipelineConfig,
        };

        let original_data = b"AES fallback test data ".repeat(100);
        let asset = Asset {
            id: "test-aes-roundtrip".to_string(),
            data: original_data.clone(),
            metadata: PipelineAssetMetadata {
                name: "test.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: original_data.len(),
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let config = PipelineConfig {
            encryption: EncryptionConfig {
                quantum_resistant: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let pipeline = AssetPipeline::new(config)
            .expect("test: create pipeline");
        let processed = pipeline.process_asset(asset).await
            .expect("test: process asset");

        let mut shard_map = CompleteShardMap::new();
        for i in 0..processed.shards.len() {
            let position = MatrixCoordinate::new(i as i64, 0, 0)
                .expect("test: coordinate");
            let location = ShardLocation::new(position, 1.0);
            let entry = ShardMapEntry::new([i as u8; 32], vec![location]);
            shard_map.add_entry(entry);
        }

        let metadata = RetrievalMetadata {
            erasure_coding: (10, 4),
            compression: "brotli".to_string(),
            encryption: "aes-256-gcm".to_string(),
            content_type: "application/octet-stream".to_string(),
            created_at: 1234567890,
            encrypted_blob_size: processed.stats.encryption.encrypted_size,
        };

        let mut plan = RetrievalPlan::new([0u8; 32], shard_map, metadata);
        plan.original_size = original_data.len();

        let assembler = ClientAssembler::new(4);
        assembler.initialize(plan).await.expect("test: init");

        {
            let mut fetched = assembler.fetched_shards.write().await;
            for (i, shard) in processed.shards.iter().enumerate() {
                let position = MatrixCoordinate::new(i as i64, 0, 0)
                    .expect("test: coordinate");
                fetched.insert(i, FetchedShard {
                    _hash: [0u8; 32],
                    data: shard.data.clone(),
                    _source: position,
                    _fetch_time_ms: 0,
                });
            }

            let mut progress = assembler.progress.write().await;
            progress.fetched_shards = processed.shards.len();
            progress.percentage = 1.0;
        }

        let reconstructed = assembler
            .reconstruct_with_pipeline(&processed.decryption_key)
            .await
            .expect("test: reconstruct with AES pipeline");

        assert_eq!(
            reconstructed, original_data,
            "AES pipeline round-trip: data must match"
        );
    }
}
