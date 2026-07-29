// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Pipeline Orchestrator - Complete asset processing workflow
//!
//! Coordinates all pipeline stages: Compression → Encryption → Sharding → Distribution

use crate::assets::pipeline::{
    AesKey, Asset, CompressionConfig, CompressionStats, Compressor, DistributedAsset,
    DistributionConfig, DistributionStats, EncryptedData, EncryptionConfig, EncryptionStats,
    Encryptor, KyberEncryptionResult, PipelineResult, Shard, Sharder,
    ShardingConfig, ShardingStats,
};
use serde::{Deserialize, Serialize};

/// Complete pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    /// Segment-oriented encryption with HKDF key derivation.
    /// One Kyber KEM per asset, BLAKE3-HKDF per segment.
    KyberSegmented {
        /// Kyber KEM ciphertext for decapsulation
        #[serde(with = "serde_bytes")]
        ciphertext_kem: Vec<u8>,
        /// Kyber secret key
        #[serde(with = "serde_bytes")]
        secret_key: Vec<u8>,
        /// Number of segments
        segment_count: u32,
        /// Original uncompressed asset size
        original_size: u64,
    },
}

/// Processed asset with all metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedAsset {
    /// Original asset ID
    pub asset_id: String,
    /// BLAKE3(Brotli(A)) — content hash of compressed asset, the blockchain address
    pub content_hash: [u8; 32],
    /// BLAKE3(P(A)) — content hash of the proof, the proof's blockchain address
    pub proof_hash: [u8; 32],
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
///
/// Produces content-addressed shards (Compress → Encrypt → Shard). It does
/// NOT decide placement: WHERE shards live requires the live PoS-eligible peer
/// set and their proximity coordinates, which the pipeline (a pure, offline
/// transform) does not have. Placement is the store path's concern — see
/// [`crate::network::placement`] and the single placement authority
/// [`crate::distribution::distribute_shards_pos_aware`]. `ProcessedAsset`
/// therefore carries an always-empty `DistributedAsset` (P4).
pub struct AssetPipeline {
    config: PipelineConfig,
    compressor: Compressor,
    encryptor: Encryptor,
    sharder: Sharder,
}

impl AssetPipeline {
    /// Create new pipeline with configuration
    pub fn new(config: PipelineConfig) -> PipelineResult<Self> {
        let compressor = Compressor::new(config.compression.clone());
        let encryptor = Encryptor::new(config.encryption.clone());
        let sharder = Sharder::new(config.sharding.clone())?;

        Ok(Self {
            config,
            compressor,
            encryptor,
            sharder,
        })
    }

    /// Create pipeline with default configuration
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> PipelineResult<Self> {
        Self::new(PipelineConfig::default())
    }

    /// Process asset through complete pipeline
    ///
    /// Pipeline order per §3 R3:
    /// 1. Compress (Brotli)
    /// 2. Hash: H = BLAKE3(compressed) — content address
    /// 3. Proof: P = PoS(A) — four-proof state proof for asset
    /// 4. Proof hash: PH = BLAKE3(P) — proof content address
    /// 5. Encrypt (Kyber-1024 KEM + AES-256-GCM, whole blob)
    /// 6. Shard (Reed-Solomon)
    /// 7. Distribute (tensor-based placement)
    ///
    /// Ledger entry: L = { H, PH, shard_locations }
    ///
    /// Defaults to the encrypting (Private) path. For Public/Anonymous assets
    /// use [`process_asset_with_privacy`](Self::process_asset_with_privacy),
    /// which produces content-addressed *cleartext* shards (no key to custody).
    pub async fn process_asset(&self, asset: Asset) -> PipelineResult<ProcessedAsset> {
        self.process_asset_inner(asset, true).await
    }

    /// Process an asset honoring its `PrivacyMode` for the encryption stage.
    ///
    /// Encryption gating (R8/R9): Private assets are encrypted (Kyber-1024 KEM
    /// + AES-256-GCM whole-blob) so the key must be custodied and wrapped
    /// per-recipient. Public and Anonymous assets are torrent-style —
    /// content-addressed **cleartext** shards protected by BLAKE3 integrity
    /// and PoS-gated fetch, with NO decryption key. This removes the
    /// "the key must reach everyone" contradiction for open assets.
    pub async fn process_asset_with_privacy(
        &self,
        asset: Asset,
        privacy_mode: hypermesh_lib::PrivacyMode,
    ) -> PipelineResult<ProcessedAsset> {
        // Only Private (bounded, tracked) assets are encrypted. Public and
        // Anonymous (unbounded scope) assets are stored as cleartext shards.
        let encrypt = privacy_mode.scope == hypermesh_lib::AccessScope::Bounded;
        self.process_asset_inner(asset, encrypt).await
    }

    /// Core pipeline implementation shared by the public entry points.
    ///
    /// `encrypt` gates the whole-blob encryption stage independently of
    /// `stages_enabled.encryption`: when `false`, shards are cleartext and the
    /// returned `decryption_key` is an inert placeholder (never used because
    /// callers signal cleartext via an absent key envelope on disk).
    async fn process_asset_inner(
        &self,
        asset: Asset,
        encrypt: bool,
    ) -> PipelineResult<ProcessedAsset> {
        let start = std::time::Instant::now();
        let original_size = asset.data.len();

        tracing::info!(
            "Starting pipeline for asset {} ({} bytes, encrypt={})",
            asset.id,
            original_size,
            encrypt,
        );

        // Stage 1: Compress (Brotli)
        let (compressed_data, compression_stats) = if self.config.stages_enabled.compression {
            tracing::debug!("Stage 1: Compress");
            self.compressor.compress(&asset.data)?
        } else {
            (asset.data.clone(), CompressionStats::default())
        };

        tracing::info!(
            "Compressed: {} bytes -> {} bytes (ratio: {:.2})",
            compression_stats.original_size,
            compression_stats.compressed_size,
            compression_stats.ratio
        );

        // Stage 2: Hash — H = BLAKE3(Brotli(A))
        // Content address of the compressed asset. This is the blockchain identifier.
        let content_hash: [u8; 32] = *blake3::hash(&compressed_data).as_bytes();
        tracing::info!(
            "Content hash: {}",
            hex::encode(&content_hash[..8])
        );

        // Stage 3: Proof — P = PoS(A)
        // Generate a state proof for this asset. The proof attests WHO/WHEN/WHERE/WHAT.
        // For now we hash the asset metadata as a placeholder — the full PoS integration
        // requires the caller to provide a StateProofProvider.
        let proof_data = {
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"proof-of-state:");
            hasher.update(asset.id.as_bytes());
            hasher.update(&content_hash);
            hasher.finalize().as_bytes().to_vec()
        };

        // Stage 4: Proof hash — PH = BLAKE3(P(A))
        // Content address of the proof itself.
        let proof_hash: [u8; 32] = *blake3::hash(&proof_data).as_bytes();
        tracing::info!(
            "Proof hash: {}",
            hex::encode(&proof_hash[..8])
        );

        // Stage 5: Encrypt (whole blob)
        // Encrypt the entire compressed blob, NOT per-shard.
        // Uses Kyber-1024 KEM + AES-256-GCM when quantum_resistant is true,
        // otherwise falls back to plain AES-256-GCM.
        let (encrypted_blob, decryption_key, encryption_stats) =
            if self.config.stages_enabled.encryption && encrypt {
                tracing::debug!("Stage 5: Encrypt (whole blob)");
                if self.config.encryption.quantum_resistant {
                    let keypair = self.encryptor.generate_keypair()?;
                    let (result, stats) = self
                        .encryptor
                        .encrypt(&compressed_data, &keypair.public_key)?;
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
                (
                    compressed_data,
                    DecryptionKey::Aes(key),
                    EncryptionStats::default(),
                )
            };

        tracing::info!(
            "Encrypted: {} bytes -> {} bytes",
            encryption_stats.original_size,
            encryption_stats.encrypted_size
        );

        // Stage 6: Shard (Reed-Solomon)
        // Shard the encrypted blob into erasure-coded pieces.
        // When the pipeline config uses the default RS(10,4) AND the asset
        // size warrants different parameters, we use adaptive RS (R14).
        let (shards, sharding_stats) = if self.config.stages_enabled.sharding {
            tracing::debug!("Stage 6: Shard");
            let adaptive_config =
                ShardingConfig::adaptive_for_size(original_size as u64);
            let use_adaptive = self.config.sharding.data_shards
                == ShardingConfig::default().data_shards
                && self.config.sharding.parity_shards
                    == ShardingConfig::default().parity_shards;

            if use_adaptive
                && (adaptive_config.data_shards != self.config.sharding.data_shards
                    || adaptive_config.parity_shards != self.config.sharding.parity_shards)
            {
                tracing::info!(
                    "Adaptive RS: RS({},{}) for {} bytes (R14)",
                    adaptive_config.data_shards,
                    adaptive_config.parity_shards,
                    original_size,
                );
                let adaptive_sharder = Sharder::new(adaptive_config)?;
                adaptive_sharder.shard(&encrypted_blob)?
            } else {
                self.sharder.shard(&encrypted_blob)?
            }
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

        tracing::info!(
            "Sharded: {} data shards + {} parity shards",
            sharding_stats.data_shards,
            sharding_stats.parity_shards
        );

        // Stage 7: Distribute — placement is NOT computed here.
        //
        // P4: the pipeline no longer fabricates matrix positions. Placement
        // (WHERE) requires the live PoS-eligible peer set + proximity
        // coordinates, which only the store path has. The carrier is filled
        // empty; the store path computes real placements via
        // `crate::network::placement::place_shards` →
        // `crate::distribution::distribute_shards_pos_aware`.
        let distributed = DistributedAsset {
            asset_id: asset.id.clone(),
            placements: vec![],
            metadata: crate::assets::pipeline::distribution::DistributionMetadata {
                total_shards: shards.len(),
                networks_used: 0,
                avg_shard_distance: 0.0,
                quality_score: 0.0,
                distributed_at: chrono::Utc::now().timestamp(),
            },
        };
        let distribution_stats = DistributionStats::default();

        // Calculate total statistics
        let total_duration_ms = start.elapsed().as_millis() as u64;
        let final_size: usize = shards.iter().map(|s| s.data.len()).sum();

        let stats = PipelineStats {
            compression: compression_stats,
            encryption: encryption_stats,
            sharding: sharding_stats,
            distribution: distribution_stats,
            total_duration_ms,
            total_throughput_mbps: PipelineStats::calculate_throughput(
                original_size,
                total_duration_ms,
            ),
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
            content_hash,
            proof_hash,
            shards,
            decryption_key,
            distributed,
            stats,
        })
    }

    /// Reconstruct asset from processed components
    ///
    /// Reverse pipeline order: Reconstruct shards -> Decrypt -> Decompress
    pub async fn reconstruct_asset(&self, processed: &ProcessedAsset) -> PipelineResult<Vec<u8>> {
        tracing::info!("Reconstructing asset {}", processed.asset_id);

        // Stage 1: Reconstruct encrypted blob from shards
        let encrypted_blob = if self.config.stages_enabled.sharding {
            tracing::debug!("Stage 1: Shard reconstruction");
            // Detect actual RS parameters from the processed shards (may differ
            // from pipeline default due to adaptive sizing per R14).
            // Only override when we have a complete set indicating different RS
            // config -- partial shard sets (missing parity) should use
            // the existing sharder which handles reconstruction with fewer shards.
            let total = processed.shards.len();
            let data_count = processed.shards.iter().filter(|s| !s.metadata.is_parity).count();
            let parity_count = total - data_count;
            let sharder = if parity_count > 0
                && (data_count != self.config.sharding.data_shards
                    || parity_count != self.config.sharding.parity_shards)
            {
                tracing::info!(
                    "Adaptive RS reconstruction: RS({},{}) from shard metadata",
                    data_count,
                    parity_count,
                );
                &Sharder::new(ShardingConfig {
                    data_shards: data_count,
                    parity_shards: parity_count,
                    target_shard_size: self.config.sharding.target_shard_size,
                })?
            } else {
                &self.sharder
            };
            sharder.reconstruct(&processed.shards)?
        } else {
            processed
                .shards
                .first()
                .map(|s| s.data.clone())
                .unwrap_or_default()
        };

        // Stage 2: Decrypt the whole blob
        let compressed_data = if self.config.stages_enabled.encryption {
            tracing::debug!("Stage 2: Decryption (whole blob)");
            match &processed.decryption_key {
                DecryptionKey::Kyber {
                    ciphertext_kem,
                    nonce,
                    original_size,
                    secret_key,
                } => {
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
                DecryptionKey::KyberSegmented { .. } => {
                    return Err(crate::assets::pipeline::PipelineError::InvalidData(
                        "KyberSegmented assets must use StreamingAssetPipeline for reconstruction".into(),
                    ));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_pipeline() {
        let pipeline = AssetPipeline::default().expect("test: expected success");

        let asset = Asset {
            id: "test-asset-1".to_string(),
            data: b"Hello, World! ".repeat(1000),
            metadata: crate::assets::pipeline::PipelineInputMetadata {
                name: "test.txt".to_string(),
                content_type: "text/plain".to_string(),
                size: 14000,
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let original_data = asset.data.clone();
        let processed = pipeline.process_asset(asset).await.expect("test: async operation");

        // Verify processing
        assert_eq!(processed.asset_id, "test-asset-1");
        assert!(!processed.shards.is_empty());
        // Default config is quantum_resistant=true → Kyber key
        assert!(matches!(
            processed.decryption_key,
            DecryptionKey::Kyber { .. }
        ));
        assert!(processed.stats.total_throughput_mbps > 0.0);

        // Reconstruct and verify
        let reconstructed = pipeline.reconstruct_asset(&processed).await.expect("test: async operation");
        assert_eq!(reconstructed, original_data);
    }

    #[tokio::test]
    async fn test_pipeline_with_large_data() {
        let pipeline = AssetPipeline::default().expect("test: expected success");

        // 10MB of data
        let data = vec![42u8; 10 * 1024 * 1024];
        let asset = Asset {
            id: "large-asset".to_string(),
            data: data.clone(),
            metadata: crate::assets::pipeline::PipelineInputMetadata {
                name: "large.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: data.len(),
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let processed = pipeline.process_asset(asset).await.expect("test: async operation");

        // Verify throughput target (should be > 100 MB/s)
        assert!(
            processed.stats.total_throughput_mbps > 100.0,
            "Throughput {} MB/s is below target",
            processed.stats.total_throughput_mbps
        );

        // Reconstruct and verify
        let reconstructed = pipeline.reconstruct_asset(&processed).await.expect("test: async operation");
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

        let pipeline = AssetPipeline::new(config).expect("test: creation");

        let asset = Asset {
            id: "test".to_string(),
            data: b"Test data".to_vec(),
            metadata: crate::assets::pipeline::PipelineInputMetadata {
                name: "test.txt".to_string(),
                content_type: "text/plain".to_string(),
                size: 9,
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let original_data = asset.data.clone();
        let processed = pipeline.process_asset(asset).await.expect("test: async operation");

        // Should still work but with minimal processing
        let reconstructed = pipeline.reconstruct_asset(&processed).await.expect("test: async operation");
        assert_eq!(reconstructed, original_data);
    }

    #[tokio::test]
    async fn test_pipeline_stats() {
        let pipeline = AssetPipeline::default().expect("test: expected success");

        let data = vec![0u8; 100000];
        let asset = Asset {
            id: "stats-test".to_string(),
            data: data.clone(),
            metadata: crate::assets::pipeline::PipelineInputMetadata {
                name: "stats.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: data.len(),
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let processed = pipeline.process_asset(asset).await.expect("test: async operation");

        // Verify all stats are populated
        assert_eq!(processed.stats.original_size, 100000);
        assert!(processed.stats.final_size > 0);
        // total_duration_ms is a wall-clock measurement: a small asset through
        // Compress→Encrypt→Shard can complete in under a millisecond, so 0 is a
        // valid reading. Assert throughput is a finite, non-negative number
        // instead of a fragile `> 0` on the millisecond clock.
        assert!(processed.stats.total_throughput_mbps.is_finite());
        assert!(processed.stats.total_throughput_mbps >= 0.0);
        assert!(processed.stats.compression.ratio > 0.0);
        assert!(processed.stats.encryption.encrypted_size > 0);
        assert!(processed.stats.sharding.data_shards > 0);
        // P4: the pipeline no longer computes placement, so distribution stats
        // are empty here — placement is the store path's concern.
        assert_eq!(processed.stats.distribution.shards_distributed, 0);
        assert!(processed.distributed.placements.is_empty());
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

        let pipeline = AssetPipeline::new(config).expect("test: creation");

        let asset = Asset {
            id: "custom-shards".to_string(),
            data: vec![1u8; 5000],
            metadata: crate::assets::pipeline::PipelineInputMetadata {
                name: "custom.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: 5000,
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        let original_data = asset.data.clone();
        let processed = pipeline.process_asset(asset).await.expect("test: async operation");

        assert_eq!(processed.stats.sharding.data_shards, 6);
        assert_eq!(processed.stats.sharding.parity_shards, 2);
        assert_eq!(processed.shards.len(), 8);

        let reconstructed = pipeline.reconstruct_asset(&processed).await.expect("test: async operation");
        assert_eq!(reconstructed, original_data);
    }

    /// Build a fixed test asset with `data` (helper for the privacy tests).
    fn privacy_test_asset(id: &str, data: Vec<u8>) -> Asset {
        let size = data.len();
        Asset {
            id: id.to_string(),
            data,
            metadata: crate::assets::pipeline::PipelineInputMetadata {
                name: "clip.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size,
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        }
    }

    /// Concatenate all shard bytes (index order) for comparing whether the
    /// underlying blob changed between privacy modes.
    fn concat_shards(processed: &ProcessedAsset) -> Vec<u8> {
        let mut shards: Vec<&Shard> = processed.shards.iter().collect();
        shards.sort_by_key(|s| (s.metadata.is_parity, s.metadata.index));
        let mut blob = Vec::new();
        for s in shards {
            blob.extend_from_slice(&s.data);
        }
        blob
    }

    /// F5 regression: a Public (Unbounded) asset is processed as
    /// content-addressed *cleartext* shards — no encryption key to custody.
    /// Proof: (1) the same input yields DIFFERENT shard bytes for PUBLIC vs
    /// PRIVATE (encryption changed them), and (2) the PUBLIC asset reconstructs
    /// through the decryption-disabled reverse pipeline with NO key material.
    #[tokio::test]
    async fn test_public_asset_produces_cleartext_shards() {
        let data = {
            let marker = b"HYPERMESH-CLEARTEXT-MARKER-0123456789";
            let mut d = Vec::new();
            for _ in 0..200 {
                d.extend_from_slice(marker);
            }
            d
        };

        let pipeline = AssetPipeline::default().expect("test: pipeline");

        let public = pipeline
            .process_asset_with_privacy(
                privacy_test_asset("public-cleartext", data.clone()),
                hypermesh_lib::PrivacyMode::PUBLIC,
            )
            .await
            .expect("test: public pipeline");

        let private = pipeline
            .process_asset_with_privacy(
                privacy_test_asset("private-encrypted", data.clone()),
                hypermesh_lib::PrivacyMode::PRIVATE,
            )
            .await
            .expect("test: private pipeline");

        // Identical input, identical RS parameters — the only difference is the
        // encryption stage. Cleartext (Public) and ciphertext (Private) shard
        // bytes MUST differ.
        assert_ne!(
            concat_shards(&public),
            concat_shards(&private),
            "cleartext (Public) and encrypted (Private) shards must differ"
        );
        // Private produced a real Kyber key to custody; Public did not encrypt.
        assert!(matches!(private.decryption_key, DecryptionKey::Kyber { .. }));

        // The cleartext fetch path reconstructs with encryption disabled and
        // NO key material — the defining property of a Public asset.
        let mut config = PipelineConfig::default();
        config.stages_enabled.encryption = false;
        let cleartext_pipeline = AssetPipeline::new(config).expect("test: cleartext pipeline");
        let reconstructed = cleartext_pipeline
            .reconstruct_asset(&public)
            .await
            .expect("test: reconstruct cleartext");
        assert_eq!(reconstructed, data);
    }

    /// A Private (Bounded) asset takes the encrypting path — a real Kyber
    /// `DecryptionKey` is produced and the asset round-trips through the
    /// encrypting reverse pipeline.
    #[tokio::test]
    async fn test_private_asset_encrypts_shards() {
        let data = vec![7u8; 8000];
        let pipeline = AssetPipeline::default().expect("test: pipeline");

        let processed = pipeline
            .process_asset_with_privacy(
                privacy_test_asset("private-encrypted", data.clone()),
                hypermesh_lib::PrivacyMode::PRIVATE,
            )
            .await
            .expect("test: private pipeline");

        assert!(matches!(processed.decryption_key, DecryptionKey::Kyber { .. }));

        let reconstructed = pipeline
            .reconstruct_asset(&processed)
            .await
            .expect("test: reconstruct private");
        assert_eq!(reconstructed, data);
    }
}
