// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Client Assembly
//!
//! Client-side shard fetching and file reconstruction from retrieval instructions.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::matrix::MatrixCoordinate;
use crate::assets::storage::Hash;
use crate::assets::pipeline::{
    Compressor, CompressionConfig, CompressionAlgorithm,
    Encryptor, EncryptionConfig,
    Sharder, ShardingConfig,
    sharding::{Shard, ShardMetadata},
    encryption::{KyberEncryptionResult, EncryptedData},
    orchestrator::DecryptionKey,
};

use super::{RetrievalPlan, RetrievalMetadata, ShardLocation};

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
    _hash: Hash,

    /// Shard data
    data: Vec<u8>,

    /// Position it was fetched from
    _source: MatrixCoordinate,

    /// Time taken to fetch (milliseconds)
    _fetch_time_ms: u64,
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
                        _hash: shard_hash,
                        data: data.clone(),
                        _source: location.position.clone(),
                        _fetch_time_ms: fetch_time,
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

    /// Reconstruct file from fetched shards (basic concatenation).
    ///
    /// This method concatenates raw shard data without pipeline processing.
    /// Use `reconstruct_with_pipeline()` for full reverse-pipeline
    /// reconstruction (Reed-Solomon -> decrypt -> decompress).
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

        let mut reconstructed = Vec::new();
        for i in 0..plan.min_shards_required {
            if let Some(shard) = fetched.get(&i) {
                reconstructed.extend_from_slice(&shard.data);
            }
        }

        Ok(reconstructed)
    }

    /// Reconstruct file using the full reverse pipeline.
    ///
    /// Applies the reverse of the asset processing pipeline:
    /// 1. Reed-Solomon reconstruct encrypted blob from shards
    /// 2. Decrypt blob using the provided decryption key
    /// 3. Decompress to recover original data
    pub async fn reconstruct_with_pipeline(
        &self,
        decryption_key: &DecryptionKey,
    ) -> Result<Vec<u8>> {
        let plan = self.plan.read().await;
        let plan = plan.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No retrieval plan set"))?;

        let fetched = self.fetched_shards.read().await;

        if fetched.len() < plan.min_shards_required {
            return Err(anyhow::anyhow!(
                "Insufficient shards: have {}, need {}",
                fetched.len(),
                plan.min_shards_required
            ));
        }

        let pipeline_shards = self.build_pipeline_shards(&fetched, plan);

        // Reverse pipeline: reconstruct -> decrypt -> decompress
        let encrypted_blob = Self::reconstruct_shards(
            &pipeline_shards,
            &plan.metadata,
        )?;
        let compressed_data = Self::decrypt_blob(
            &encrypted_blob,
            decryption_key,
            &plan.metadata,
        )?;
        let original_data = Self::decompress_data(
            &compressed_data,
            &plan.metadata,
        )?;

        Ok(original_data)
    }

    /// Convert fetched shards into pipeline Shard structs.
    fn build_pipeline_shards(
        &self,
        fetched: &HashMap<usize, FetchedShard>,
        plan: &RetrievalPlan,
    ) -> Vec<Shard> {
        let data_shard_count = plan.metadata.erasure_coding.0;

        let mut shards: Vec<Shard> = Vec::with_capacity(fetched.len());

        for (&idx, fetched_shard) in fetched.iter() {
            let is_parity = idx >= data_shard_count;
            let hash_hex = {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(&fetched_shard.data);
                hex::encode(hasher.finalize())
            };

            // Last data shard stores pre-sharding blob size for Reed-Solomon
            // padding truncation. This is the encrypted blob size, NOT the
            // original uncompressed data size.
            let original_size = if !is_parity && idx == data_shard_count - 1 {
                plan.metadata.encrypted_blob_size
            } else {
                fetched_shard.data.len()
            };

            let metadata = ShardMetadata {
                index: idx,
                is_parity,
                size: fetched_shard.data.len(),
                original_size,
                hash: hash_hex,
            };

            shards.push(Shard {
                data: fetched_shard.data.clone(),
                metadata,
            });
        }

        // Sort by index for consistent reconstruction
        shards.sort_by_key(|s| s.metadata.index);
        shards
    }

    /// Stage 1: Reed-Solomon reconstruct encrypted blob from shards.
    fn reconstruct_shards(
        shards: &[Shard],
        metadata: &RetrievalMetadata,
    ) -> Result<Vec<u8>> {
        let config = ShardingConfig {
            data_shards: metadata.erasure_coding.0,
            parity_shards: metadata.erasure_coding.1,
            ..Default::default()
        };
        let sharder = Sharder::new(config)
            .map_err(|e| anyhow::anyhow!("Sharder init failed: {}", e))?;

        sharder.reconstruct(shards)
            .map_err(|e| anyhow::anyhow!("Shard reconstruction failed: {}", e))
    }

    /// Stage 2: Decrypt blob using decryption key.
    fn decrypt_blob(
        encrypted_blob: &[u8],
        decryption_key: &DecryptionKey,
        metadata: &RetrievalMetadata,
    ) -> Result<Vec<u8>> {
        if metadata.encryption.is_empty() || metadata.encryption == "none" {
            return Ok(encrypted_blob.to_vec());
        }

        let encryptor = Encryptor::new(EncryptionConfig::default());

        match decryption_key {
            DecryptionKey::Kyber {
                ciphertext_kem,
                nonce,
                original_size,
                secret_key,
            } => {
                let kyber_result = KyberEncryptionResult {
                    ciphertext_kem: ciphertext_kem.clone(),
                    encrypted_data: encrypted_blob.to_vec(),
                    nonce: nonce.clone(),
                    original_size: *original_size,
                };
                encryptor.decrypt(&kyber_result, secret_key)
                    .map_err(|e| anyhow::anyhow!("Kyber decryption failed: {}", e))
            }
            DecryptionKey::Aes(key) => {
                let encrypted = EncryptedData {
                    ciphertext: encrypted_blob.to_vec(),
                    nonce: key.nonce.clone(),
                    original_size: 0,
                };
                encryptor.decrypt_aes(&encrypted, key)
                    .map_err(|e| anyhow::anyhow!("AES decryption failed: {}", e))
            }
        }
    }

    /// Stage 3: Decompress data.
    fn decompress_data(
        compressed_data: &[u8],
        metadata: &RetrievalMetadata,
    ) -> Result<Vec<u8>> {
        if metadata.compression.is_empty() || metadata.compression == "none" {
            return Ok(compressed_data.to_vec());
        }

        let algorithm = match metadata.compression.as_str() {
            "brotli" => CompressionAlgorithm::Brotli,
            _ => CompressionAlgorithm::None,
        };

        let compressor = Compressor::new(CompressionConfig {
            algorithm,
            ..Default::default()
        });

        compressor.decompress(compressed_data)
            .map_err(|e| anyhow::anyhow!("Decompression failed: {}", e))
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
    use crate::retrieval::{CompleteShardMap, ShardMapEntry};

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

    #[tokio::test]
    async fn test_reconstruct_with_pipeline_roundtrip() {
        use crate::assets::pipeline::{
            Asset, AssetMetadata as PipelineAssetMetadata,
            AssetPipeline,
        };

        // 1. Process data through the forward pipeline
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

        // 2. Build a retrieval plan from the processed asset
        let content_hash = [42u8; 32];
        let mut shard_map = CompleteShardMap::new();

        for (i, shard) in processed.shards.iter().enumerate() {
            let shard_hash = {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(&shard.data);
                let result = hasher.finalize();
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&result);
                hash
            };
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

        // 3. Manually inject processed shards as fetched shards
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

        // 4. Reconstruct using the real pipeline
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

        // Use AES-only (non-quantum) pipeline
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

        // Build plan and inject shards
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
