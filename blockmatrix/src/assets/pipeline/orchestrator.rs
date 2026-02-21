// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Pipeline Orchestrator - Complete asset processing workflow
//!
//! Coordinates all pipeline stages: Compression → Encryption → Sharding → Distribution

use crate::assets::pipeline::{
    Asset, PipelineResult,
    Compressor, CompressionConfig, CompressionStats,
    Encryptor, EncryptionConfig, EncryptionStats, EncryptedData, AesKey,
    KyberEncryptionResult,
    Sharder, ShardingConfig, Shard, ShardingStats,
    MatrixDistributor, DistributionConfig, DistributedAsset, DistributionStats,
};
use serde::{Serialize, Deserialize};

/// Complete pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Compression configuration
    pub compression: CompressionConfig,
    /// Encryption configuration
    pub encryption: EncryptionConfig,
    /// Sharding configuration
    pub sharding: ShardingConfig,
    /// Distribution configuration
    pub distribution: DistributionConfig,
    /// Enable pipeline stages
    pub stages_enabled: PipelineStages,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            compression: CompressionConfig::default(),
            encryption: EncryptionConfig::default(),
            sharding: ShardingConfig::default(),
            distribution: DistributionConfig::default(),
            stages_enabled: PipelineStages::default(),
        }
    }
}

/// Pipeline stages enable/disable flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStages {
    pub compression: bool,
    pub encryption: bool,
    pub sharding: bool,
    pub distribution: bool,
}

impl Default for PipelineStages {
    fn default() -> Self {
        Self {
            compression: true,
            encryption: true,
            sharding: true,
            distribution: true,
        }
    }
}

/// Decryption material stored alongside processed asset.
///
/// For Kyber-1024 (quantum-resistant): stores the KEM ciphertext, nonce,
/// original size, and the Kyber secret key needed for decapsulation.
///
/// For plain AES-256-GCM (fallback): stores the symmetric key and nonce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecryptionKey {
    /// Kyber-1024 KEM + AES-256-GCM (quantum-resistant path)
    Kyber {
        /// Kyber KEM ciphertext (needed for decapsulation)
        #[serde(with = "serde_bytes")]
        ciphertext_kem: Vec<u8>,
        /// AES-GCM nonce used during encryption
        #[serde(with = "serde_bytes")]
        nonce: Vec<u8>,
        /// Pre-encryption size in bytes
        original_size: usize,
        /// Kyber-1024 secret key bytes (for decapsulation)
        #[serde(with = "serde_bytes")]
        secret_key: Vec<u8>,
    },
    /// Plain AES-256-GCM (non-quantum fallback)
    Aes(AesKey),
}

/// Processed asset with all metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedAsset {
    /// Original asset ID
    pub asset_id: String,
    /// Shards of the encrypted blob
    pub shards: Vec<Shard>,
    /// Decryption material for the whole compressed blob
    pub decryption_key: DecryptionKey,
    /// Distribution information
    pub distributed: DistributedAsset,
    /// Complete pipeline statistics
    pub stats: PipelineStats,
}

/// Complete pipeline statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineStats {
    /// Compression statistics
    pub compression: CompressionStats,
    /// Encryption statistics
    pub encryption: EncryptionStats,
    /// Sharding statistics
    pub sharding: ShardingStats,
    /// Distribution statistics
    pub distribution: DistributionStats,
    /// Total pipeline duration (ms)
    pub total_duration_ms: u64,
    /// Total throughput (MB/s)
    pub total_throughput_mbps: f64,
    /// Original asset size
    pub original_size: usize,
    /// Final total size (all shards)
    pub final_size: usize,
}

impl PipelineStats {
    /// Calculate total throughput
    fn calculate_throughput(original_size: usize, duration_ms: u64) -> f64 {
        if duration_ms > 0 {
            (original_size as f64 / (1024.0 * 1024.0)) / (duration_ms as f64 / 1000.0)
        } else if original_size > 0 {
            // If duration is too small to measure, use a minimum of 0.001ms (1 microsecond)
            (original_size as f64 / (1024.0 * 1024.0)) / 0.001
        } else {
            0.0
        }
    }
}

/// Asset processing pipeline
pub struct AssetPipeline {
    config: PipelineConfig,
    compressor: Compressor,
    encryptor: Encryptor,
    sharder: Sharder,
    distributor: MatrixDistributor,
}

impl AssetPipeline {
    /// Create new pipeline with configuration
    pub fn new(config: PipelineConfig) -> PipelineResult<Self> {
        let compressor = Compressor::new(config.compression.clone());
        let encryptor = Encryptor::new(config.encryption.clone());
        let sharder = Sharder::new(config.sharding.clone())?;
        let distributor = MatrixDistributor::new(config.distribution.clone());

        Ok(Self {
            config,
            compressor,
            encryptor,
            sharder,
            distributor,
        })
    }

    /// Create pipeline with default configuration
    pub fn default() -> PipelineResult<Self> {
        Self::new(PipelineConfig::default())
    }

    /// Process asset through complete pipeline
    ///
    /// Pipeline order: Compress -> Encrypt (whole blob) -> Shard -> Distribute
    ///
    /// This order ensures:
    /// 1. Compression operates on raw data for best ratio
    /// 2. Encryption covers the entire compressed blob (not per-shard)
    /// 3. Sharding splits already-encrypted data into erasure-coded shards
    /// 4. Distribution places shards at optimal matrix positions
    pub async fn process_asset(&self, asset: Asset) -> PipelineResult<ProcessedAsset> {
        let start = std::time::Instant::now();
        let original_size = asset.data.len();

        tracing::info!("Starting pipeline for asset {} ({} bytes)", asset.id, original_size);

        // Stage 1: Compression (Brotli)
        // Compress raw data first for best compression ratio
        let (compressed_data, compression_stats) = if self.config.stages_enabled.compression {
            tracing::debug!("Stage 1: Compression");
            self.compressor.compress(&asset.data)?
        } else {
            (asset.data.clone(), CompressionStats::default())
        };

        tracing::info!("Compressed: {} bytes -> {} bytes (ratio: {:.2})",
            compression_stats.original_size,
            compression_stats.compressed_size,
            compression_stats.ratio
        );

        // Stage 2: Encryption (whole blob)
        // Encrypt the entire compressed blob, NOT per-shard.
        // Uses Kyber-1024 KEM + AES-256-GCM when quantum_resistant is true,
        // otherwise falls back to plain AES-256-GCM.
        let (encrypted_blob, decryption_key, encryption_stats) = if self.config.stages_enabled.encryption {
            tracing::debug!("Stage 2: Encryption (whole blob)");
            if self.config.encryption.quantum_resistant {
                let keypair = self.encryptor.generate_keypair()?;
                let (result, stats) = self.encryptor.encrypt(&compressed_data, &keypair.public_key)?;
                let dk = DecryptionKey::Kyber {
                    ciphertext_kem: result.ciphertext_kem,
                    nonce: result.nonce,
                    original_size: result.original_size,
                    secret_key: keypair.secret_key,
                };
                (result.encrypted_data, dk, stats)
            } else {
                let key = self.encryptor.generate_aes_key()?;
                let (encrypted, stats) = self.encryptor.encrypt_aes(&compressed_data, &key)?;
                (encrypted.ciphertext, DecryptionKey::Aes(key), stats)
            }
        } else {
            let key = self.encryptor.generate_aes_key()?;
            (compressed_data, DecryptionKey::Aes(key), EncryptionStats::default())
        };

        tracing::info!("Encrypted: {} bytes -> {} bytes",
            encryption_stats.original_size,
            encryption_stats.encrypted_size
        );

        // Stage 3: Sharding (Reed-Solomon)
        // Shard the encrypted blob into erasure-coded pieces
        let (shards, sharding_stats) = if self.config.stages_enabled.sharding {
            tracing::debug!("Stage 3: Sharding");
            self.sharder.shard(&encrypted_blob)?
        } else {
            // No sharding - create single shard
            let metadata = crate::assets::pipeline::sharding::ShardMetadata {
                index: 0,
                is_parity: false,
                size: encrypted_blob.len(),
                original_size: encrypted_blob.len(),
                hash: hex::encode(&encrypted_blob),
            };
            let shard = Shard {
                data: encrypted_blob,
                metadata,
            };
            (vec![shard], ShardingStats::default())
        };

        tracing::info!("Sharded: {} data shards + {} parity shards",
            sharding_stats.data_shards,
            sharding_stats.parity_shards
        );

        // Stage 4: Distribution (tensor-based)
        // Place shards at optimal matrix positions
        let (distributed, distribution_stats) = if self.config.stages_enabled.distribution {
            tracing::debug!("Stage 4: Distribution");
            self.distributor.distribute(asset.id.clone(), shards.len())?
        } else {
            // No distribution - return empty stats
            let metadata = crate::assets::pipeline::distribution::DistributionMetadata {
                total_shards: shards.len(),
                networks_used: 1,
                avg_shard_distance: 0.0,
                quality_score: 0.0,
                distributed_at: chrono::Utc::now().timestamp(),
            };
            let distributed = DistributedAsset {
                asset_id: asset.id.clone(),
                placements: vec![],
                metadata,
            };
            (distributed, DistributionStats::default())
        };

        tracing::info!("Distributed: {} shards across {} networks (quality: {:.1})",
            distribution_stats.shards_distributed,
            distribution_stats.networks_used,
            distribution_stats.quality_score
        );

        // Calculate total statistics
        let total_duration_ms = start.elapsed().as_millis() as u64;
        let final_size: usize = shards.iter().map(|s| s.data.len()).sum();

        let stats = PipelineStats {
            compression: compression_stats,
            encryption: encryption_stats,
            sharding: sharding_stats,
            distribution: distribution_stats,
            total_duration_ms,
            total_throughput_mbps: PipelineStats::calculate_throughput(original_size, total_duration_ms),
            original_size,
            final_size,
        };

        tracing::info!(
            "Pipeline complete: {} bytes -> {} bytes in {} ms ({:.2} MB/s)",
            stats.original_size,
            stats.final_size,
            stats.total_duration_ms,
            stats.total_throughput_mbps
        );

        Ok(ProcessedAsset {
            asset_id: asset.id,
            shards,
            decryption_key,
            distributed,
            stats,
        })
    }

    /// Reconstruct asset from processed components
    ///
    /// Reverse pipeline order: Reconstruct shards -> Decrypt -> Decompress
    pub async fn reconstruct_asset(
        &self,
        processed: &ProcessedAsset,
    ) -> PipelineResult<Vec<u8>> {
        tracing::info!("Reconstructing asset {}", processed.asset_id);

        // Stage 1: Reconstruct encrypted blob from shards
        let encrypted_blob = if self.config.stages_enabled.sharding {
            tracing::debug!("Stage 1: Shard reconstruction");
            self.sharder.reconstruct(&processed.shards)?
        } else {
            processed.shards.first()
                .map(|s| s.data.clone())
                .unwrap_or_default()
        };

        // Stage 2: Decrypt the whole blob
        let compressed_data = if self.config.stages_enabled.encryption {
            tracing::debug!("Stage 2: Decryption (whole blob)");
            match &processed.decryption_key {
                DecryptionKey::Kyber { ciphertext_kem, nonce, original_size, secret_key } => {
                    let kyber_result = KyberEncryptionResult {
                        ciphertext_kem: ciphertext_kem.clone(),
                        encrypted_data: encrypted_blob,
                        nonce: nonce.clone(),
                        original_size: *original_size,
                    };
                    self.encryptor.decrypt(&kyber_result, secret_key)?
                }
                DecryptionKey::Aes(key) => {
                    let encrypted = EncryptedData {
                        ciphertext: encrypted_blob,
                        nonce: key.nonce.clone(),
                        original_size: 0,
                    };
                    self.encryptor.decrypt_aes(&encrypted, key)?
                }
            }
        } else {
            encrypted_blob
        };

        // Stage 3: Decompress
        let original_data = if self.config.stages_enabled.compression {
            tracing::debug!("Stage 3: Decompression");
            self.compressor.decompress(&compressed_data)?
        } else {
            compressed_data
        };

        tracing::info!("Reconstruction complete: {} bytes", original_data.len());

        Ok(original_data)
    }

    /// Get pipeline configuration
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Get mutable reference to distributor (for node registration)
    pub fn distributor_mut(&mut self) -> &mut MatrixDistributor {
        &mut self.distributor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_pipeline() {
        let pipeline = AssetPipeline::default().unwrap();

        let asset = Asset {
            id: "test-asset-1".to_string(),
            data: b"Hello, World! ".repeat(1000),
            metadata: crate::assets::pipeline::AssetMetadata {
                name: "test.txt".to_string(),
                content_type: "text/plain".to_string(),
                size: 14000,
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let original_data = asset.data.clone();
        let processed = pipeline.process_asset(asset).await.unwrap();

        // Verify processing
        assert_eq!(processed.asset_id, "test-asset-1");
        assert!(!processed.shards.is_empty());
        // Default config is quantum_resistant=true → Kyber key
        assert!(matches!(processed.decryption_key, DecryptionKey::Kyber { .. }));
        assert!(processed.stats.total_throughput_mbps > 0.0);

        // Reconstruct and verify
        let reconstructed = pipeline.reconstruct_asset(&processed).await.unwrap();
        assert_eq!(reconstructed, original_data);
    }

    #[tokio::test]
    async fn test_pipeline_with_large_data() {
        let pipeline = AssetPipeline::default().unwrap();

        // 10MB of data
        let data = vec![42u8; 10 * 1024 * 1024];
        let asset = Asset {
            id: "large-asset".to_string(),
            data: data.clone(),
            metadata: crate::assets::pipeline::AssetMetadata {
                name: "large.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: data.len(),
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let processed = pipeline.process_asset(asset).await.unwrap();

        // Verify throughput target (should be > 100 MB/s)
        assert!(processed.stats.total_throughput_mbps > 100.0,
            "Throughput {} MB/s is below target", processed.stats.total_throughput_mbps);

        // Reconstruct and verify
        let reconstructed = pipeline.reconstruct_asset(&processed).await.unwrap();
        assert_eq!(reconstructed, data);
    }

    #[tokio::test]
    async fn test_pipeline_stages_disabled() {
        let config = PipelineConfig {
            stages_enabled: PipelineStages {
                compression: false,
                encryption: false,
                sharding: false,
                distribution: false,
            },
            ..Default::default()
        };

        let pipeline = AssetPipeline::new(config).unwrap();

        let asset = Asset {
            id: "test".to_string(),
            data: b"Test data".to_vec(),
            metadata: crate::assets::pipeline::AssetMetadata {
                name: "test.txt".to_string(),
                content_type: "text/plain".to_string(),
                size: 9,
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let original_data = asset.data.clone();
        let processed = pipeline.process_asset(asset).await.unwrap();

        // Should still work but with minimal processing
        let reconstructed = pipeline.reconstruct_asset(&processed).await.unwrap();
        assert_eq!(reconstructed, original_data);
    }

    #[tokio::test]
    async fn test_pipeline_stats() {
        let pipeline = AssetPipeline::default().unwrap();

        let data = vec![0u8; 100000];
        let asset = Asset {
            id: "stats-test".to_string(),
            data: data.clone(),
            metadata: crate::assets::pipeline::AssetMetadata {
                name: "stats.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: data.len(),
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let processed = pipeline.process_asset(asset).await.unwrap();

        // Verify all stats are populated
        assert_eq!(processed.stats.original_size, 100000);
        assert!(processed.stats.final_size > 0);
        assert!(processed.stats.total_duration_ms > 0);
        assert!(processed.stats.compression.ratio > 0.0);
        assert!(processed.stats.encryption.encrypted_size > 0);
        assert!(processed.stats.sharding.data_shards > 0);
        assert!(processed.stats.distribution.shards_distributed > 0);
    }

    #[tokio::test]
    async fn test_custom_sharding_config() {
        let config = PipelineConfig {
            sharding: ShardingConfig {
                data_shards: 6,
                parity_shards: 2,
                target_shard_size: 1024,
            },
            ..Default::default()
        };

        let pipeline = AssetPipeline::new(config).unwrap();

        let asset = Asset {
            id: "custom-shards".to_string(),
            data: vec![1u8; 5000],
            metadata: crate::assets::pipeline::AssetMetadata {
                name: "custom.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: 5000,
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let original_data = asset.data.clone();
        let processed = pipeline.process_asset(asset).await.unwrap();

        assert_eq!(processed.stats.sharding.data_shards, 6);
        assert_eq!(processed.stats.sharding.parity_shards, 2);
        assert_eq!(processed.shards.len(), 8);

        let reconstructed = pipeline.reconstruct_asset(&processed).await.unwrap();
        assert_eq!(reconstructed, original_data);
    }
}
