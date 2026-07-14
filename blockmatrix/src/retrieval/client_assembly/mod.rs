// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Client Assembly
//!
//! Client-side shard fetching and file reconstruction from retrieval instructions.

pub mod fetching;
mod pipeline;
pub mod seeding;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::assets::storage::Hash;
use crate::matrix::MatrixCoordinate;
use crate::network::swarm_provider::ShardLocationIndex;

use super::{RetrievalPlan, ShardLocation};

pub use seeding::{ConsumerProviderSeeder, ShardSeeder};

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

    /// A2: optional live-mirror index. When set, per-shard location selection
    /// consults the live swarm FIRST (via the shared two-layer resolver) before
    /// falling back to the plan's canonical matrix placements. `None` in pure
    /// tests / Private mode — the path degrades to canonical placement only.
    pub(crate) live_index: Option<Arc<ShardLocationIndex>>,

    /// A2: optional become-provider seeder. When set, every fetched +
    /// BLAKE3-verified shard is re-announced to the swarm (consumer becomes
    /// provider, R12), exactly as the live IPC path does. `None` in pure tests
    /// — the fetch still succeeds and shards are held locally, matching the IPC
    /// path's "no manager wired" fallback (cache without announce).
    pub(crate) seeder: Option<Arc<dyn ShardSeeder>>,
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
            live_index: None,
            seeder: None,
        }
    }

    /// Attach a live-mirror index so per-shard resolution consults the swarm
    /// first (A2 two-layer resolve). Builder-style; returns `self`.
    pub fn with_live_index(mut self, index: Arc<ShardLocationIndex>) -> Self {
        self.live_index = Some(index);
        self
    }

    /// Attach a become-provider seeder so verified fetches re-announce to the
    /// swarm (A2 unification — same become-provider behaviour as the live IPC
    /// path). Builder-style; returns `self`.
    pub fn with_seeder(mut self, seeder: Arc<dyn ShardSeeder>) -> Self {
        self.seeder = Some(seeder);
        self
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
    use crate::assets::pipeline::EncryptionConfig;
    use crate::retrieval::{CompleteShardMap, RetrievalMetadata, ShardLocation, ShardMapEntry};

    fn create_test_plan() -> RetrievalPlan {
        let content_hash = [1u8; 32];
        let mut shard_map = CompleteShardMap::new();

        for i in 0..14 {
            let shard_hash = [i as u8; 32];
            let locations = vec![ShardLocation::new(
                MatrixCoordinate::new(i as i64, 0, 0).expect("test: valid coordinate"),
                0.9,
            )];
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
    async fn test_reset() {
        let assembler = ClientAssembler::new(4);
        let plan = create_test_plan();

        assembler.initialize(plan).await.expect("test: async operation");

        let progress_before = assembler.get_progress().await;
        assert_eq!(progress_before.total_shards, 14);

        assembler.reset().await;

        let progress_after = assembler.get_progress().await;
        assert_eq!(progress_after.fetched_shards, 0);
        assert_eq!(progress_after.total_shards, 0);
    }

    #[tokio::test]
    async fn test_reconstruct_with_pipeline_roundtrip() {
        use crate::assets::pipeline::{
            Asset, PipelineInputMetadata, AssetPipeline,
        };

        let original_data = b"Hello, HyperMesh instruction-based retrieval! ".repeat(200);
        let asset = Asset {
            id: "test-pipeline-roundtrip".to_string(),
            data: original_data.clone(),
            metadata: PipelineInputMetadata {
                name: "test.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: original_data.len(),
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let pipeline = AssetPipeline::default().expect("test: create pipeline");
        let processed = pipeline
            .process_asset(asset)
            .await
            .expect("test: process asset");

        let content_hash = [42u8; 32];
        let mut shard_map = CompleteShardMap::new();

        for (i, shard) in processed.shards.iter().enumerate() {
            let shard_hash = *blake3::hash(&shard.data).as_bytes();
            let position = MatrixCoordinate::new(i as i64, 0, 0).expect("test: create coordinate");
            let location = ShardLocation::new(position, 1.0);
            let entry = ShardMapEntry::new(shard_hash, vec![location]);
            shard_map.add_entry(entry);
        }

        // Derive actual RS parameters from processed shards (adaptive RS per R14)
        let data_shards = processed.shards.iter().filter(|s| !s.metadata.is_parity).count();
        let parity_shards = processed.shards.iter().filter(|s| s.metadata.is_parity).count();

        let metadata = RetrievalMetadata {
            erasure_coding: (data_shards, parity_shards),
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
                let position =
                    MatrixCoordinate::new(i as i64, 0, 0).expect("test: create coordinate");
                fetched.insert(
                    i,
                    FetchedShard {
                        _hash: [0u8; 32],
                        data: shard.data.clone(),
                        _source: position,
                        _fetch_time_ms: 0,
                    },
                );
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
            Asset, PipelineInputMetadata, AssetPipeline, PipelineConfig,
        };

        let original_data = b"AES fallback test data ".repeat(100);
        let asset = Asset {
            id: "test-aes-roundtrip".to_string(),
            data: original_data.clone(),
            metadata: PipelineInputMetadata {
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
        let pipeline = AssetPipeline::new(config).expect("test: create pipeline");
        let processed = pipeline
            .process_asset(asset)
            .await
            .expect("test: process asset");

        let mut shard_map = CompleteShardMap::new();
        for i in 0..processed.shards.len() {
            let position = MatrixCoordinate::new(i as i64, 0, 0).expect("test: coordinate");
            let location = ShardLocation::new(position, 1.0);
            let entry = ShardMapEntry::new([i as u8; 32], vec![location]);
            shard_map.add_entry(entry);
        }

        // Derive actual RS parameters from processed shards (adaptive RS per R14)
        let data_shards = processed.shards.iter().filter(|s| !s.metadata.is_parity).count();
        let parity_shards = processed.shards.iter().filter(|s| s.metadata.is_parity).count();

        let metadata = RetrievalMetadata {
            erasure_coding: (data_shards, parity_shards),
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
                let position = MatrixCoordinate::new(i as i64, 0, 0).expect("test: coordinate");
                fetched.insert(
                    i,
                    FetchedShard {
                        _hash: [0u8; 32],
                        data: shard.data.clone(),
                        _source: position,
                        _fetch_time_ms: 0,
                    },
                );
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

    /// 5.1 Test: Full retrieval via MockShardTransport (Kyber pipeline).
    ///
    /// Processes an asset through the pipeline, pre-populates shards on a
    /// MockShardTransport, then uses `retrieve_asset()` to fetch + reconstruct.
    #[tokio::test]
    async fn test_retrieve_asset_via_mock_transport_kyber() {
        use crate::assets::pipeline::{Asset, AssetPipeline, PipelineInputMetadata};
        use crate::network::shard_transport::MockShardTransport;
        use crate::retrieval::client_assembly::fetching::node_id_from_coordinate;

        let original_data = b"Instruction-based retrieval via transport! ".repeat(200);
        let asset = Asset {
            id: "transport-roundtrip".to_string(),
            data: original_data.clone(),
            metadata: PipelineInputMetadata {
                name: "test.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: original_data.len(),
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let pipeline = AssetPipeline::default().expect("test: create pipeline");
        let processed = pipeline
            .process_asset(asset)
            .await
            .expect("test: process asset");

        // Build retrieval plan from processed shards
        let content_hash = [42u8; 32];
        let mut shard_map = CompleteShardMap::new();
        let mock_transport = MockShardTransport::new();

        for (i, shard) in processed.shards.iter().enumerate() {
            let shard_hash = *blake3::hash(&shard.data).as_bytes();
            let position =
                MatrixCoordinate::new(i as i64, 0, 0).expect("test: coordinate");
            let location = ShardLocation::new(position, 1.0);
            shard_map.add_entry(ShardMapEntry::new(shard_hash, vec![location]));

            // Pre-populate the mock transport with shard data
            let node_id = node_id_from_coordinate(&position);
            let content_hash_val = hypermesh_lib::ContentHash(shard_hash);
            mock_transport
                .insert_shard(&node_id, &content_hash_val, shard.data.clone())
                .await;
        }

        let data_shards = processed
            .shards
            .iter()
            .filter(|s| !s.metadata.is_parity)
            .count();
        let parity_shards = processed
            .shards
            .iter()
            .filter(|s| s.metadata.is_parity)
            .count();

        let metadata = RetrievalMetadata {
            erasure_coding: (data_shards, parity_shards),
            compression: "brotli".to_string(),
            encryption: "kyber-1024".to_string(),
            content_type: "application/octet-stream".to_string(),
            created_at: 1234567890,
            encrypted_blob_size: processed.stats.encryption.encrypted_size,
        };

        let mut plan = RetrievalPlan::new(content_hash, shard_map, metadata);
        plan.original_size = original_data.len();

        // Use retrieve_asset() for the full pipeline
        let assembler = ClientAssembler::new(4);
        assembler
            .initialize(plan)
            .await
            .expect("test: init plan");

        let reconstructed = assembler
            .retrieve_asset(&mock_transport, &processed.decryption_key)
            .await
            .expect("test: retrieve_asset should succeed");

        assert_eq!(
            reconstructed, original_data,
            "retrieve_asset round-trip: data must match original"
        );

        // Verify progress was tracked
        let progress = assembler.get_progress().await;
        assert_eq!(
            progress.fetched_shards,
            data_shards + parity_shards,
            "All shards should be marked fetched"
        );
        assert!(
            (progress.percentage - 1.0).abs() < f64::EPSILON,
            "Progress should be 100%"
        );
    }

    /// FORGED MIRROR (a): a fetched shard whose data does NOT match its claimed
    /// content hash is REJECTED at reconstruct — a corrupt/forged shard never
    /// reaches the Reed-Solomon decoder (mirror invariant #1, F4).
    #[tokio::test]
    async fn test_reconstruct_rejects_forged_shard() {
        let assembler = ClientAssembler::new(4);
        let plan = create_test_plan();
        assembler.initialize(plan).await.expect("test: init plan");

        {
            let mut fetched = assembler.fetched_shards.write().await;
            // Insert enough shards to clear the min_shards_required threshold
            // (10 data shards for the test plan). Shards 0..9 are honest; shard 5
            // is FORGED (data does not hash to its claimed _hash).
            for i in 0..10usize {
                let (data, hash) = if i == 5 {
                    // FORGED: claimed _hash != BLAKE3(data)
                    (vec![9u8, 9, 9, 9], [0xAAu8; 32])
                } else {
                    let d = vec![i as u8, 2, 3, 4];
                    let h = *blake3::hash(&d).as_bytes();
                    (d, h)
                };
                fetched.insert(
                    i,
                    FetchedShard {
                        _hash: hash,
                        data,
                        _source: MatrixCoordinate::new(i as i64, 0, 0).expect("test: coord"),
                        _fetch_time_ms: 0,
                    },
                );
            }

            let mut progress = assembler.progress.write().await;
            progress.fetched_shards = 10;
            progress.percentage = 1.0;
        }

        let result = assembler
            .reconstruct_with_pipeline(&crate::assets::pipeline::orchestrator::DecryptionKey::Aes(
                crate::assets::pipeline::encryption::AesKey {
                    key: vec![0u8; 32],
                    nonce: vec![0u8; 12],
                },
            ))
            .await;

        assert!(result.is_err(), "forged shard must be rejected at reconstruct");
        assert!(
            result.unwrap_err().to_string().contains("content-hash mismatch"),
            "error should cite the content-hash mismatch",
        );
    }

    /// 5.1 Test: Retrieval with fallback when some locations are unreachable.
    ///
    /// Sets up a shard with two locations — one unreachable, one reachable.
    /// Verifies the transport falls back to the second location.
    #[tokio::test]
    async fn test_retrieve_asset_transport_fallback() {
        use crate::assets::pipeline::{Asset, AssetPipeline, PipelineInputMetadata};
        use crate::network::shard_transport::MockShardTransport;
        use crate::retrieval::client_assembly::fetching::node_id_from_coordinate;

        let original_data = b"Fallback transport test data ".repeat(200);
        let asset = Asset {
            id: "fallback-test".to_string(),
            data: original_data.clone(),
            metadata: PipelineInputMetadata {
                name: "fallback.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: original_data.len(),
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let pipeline = AssetPipeline::default().expect("test: create pipeline");
        let processed = pipeline
            .process_asset(asset)
            .await
            .expect("test: process asset");

        let content_hash = [55u8; 32];
        let mut shard_map = CompleteShardMap::new();
        let mock_transport = MockShardTransport::new();

        for (i, shard) in processed.shards.iter().enumerate() {
            let shard_hash = *blake3::hash(&shard.data).as_bytes();

            // Primary location: unreachable node at (i, 100, 0)
            let primary_pos =
                MatrixCoordinate::new(i as i64, 100, 0).expect("test: coordinate");
            let primary_node = node_id_from_coordinate(&primary_pos);

            // Secondary location: reachable node at (i, 0, 0)
            let secondary_pos =
                MatrixCoordinate::new(i as i64, 0, 0).expect("test: coordinate");
            let secondary_node = node_id_from_coordinate(&secondary_pos);

            // Mark primary as unreachable
            mock_transport.set_unreachable(&primary_node).await;

            // Pre-populate secondary with shard data
            let ch = hypermesh_lib::ContentHash(shard_hash);
            mock_transport
                .insert_shard(&secondary_node, &ch, shard.data.clone())
                .await;

            let locations = vec![
                ShardLocation::new(primary_pos, 0.9),
                ShardLocation::new(secondary_pos, 0.8),
            ];
            shard_map.add_entry(ShardMapEntry::new(shard_hash, locations));
        }

        let data_shards = processed
            .shards
            .iter()
            .filter(|s| !s.metadata.is_parity)
            .count();
        let parity_shards = processed
            .shards
            .iter()
            .filter(|s| s.metadata.is_parity)
            .count();

        let metadata = RetrievalMetadata {
            erasure_coding: (data_shards, parity_shards),
            compression: "brotli".to_string(),
            encryption: "kyber-1024".to_string(),
            content_type: "application/octet-stream".to_string(),
            created_at: 1234567890,
            encrypted_blob_size: processed.stats.encryption.encrypted_size,
        };

        let mut plan = RetrievalPlan::new(content_hash, shard_map, metadata);
        plan.original_size = original_data.len();

        let assembler = ClientAssembler::new(4);
        assembler
            .initialize(plan)
            .await
            .expect("test: init plan");

        let reconstructed = assembler
            .retrieve_asset(&mock_transport, &processed.decryption_key)
            .await
            .expect("test: retrieve with fallback should succeed");

        assert_eq!(
            reconstructed, original_data,
            "Fallback retrieval: data must match original"
        );

        // Verify fallback attempts were recorded
        let stats = assembler.get_stats().await;
        assert!(
            stats.fallback_attempts > 0,
            "Should have recorded fallback attempts (was {})",
            stats.fallback_attempts
        );
    }
}
