// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Streaming-first asset pipeline with segment-oriented processing.
//!
//! Processes assets as independently-addressable segments, each compressed,
//! encrypted, and sharded independently. Supports three consumption patterns:
//! - Torrent: download all segments, reconstruct whole file
//! - Streaming: fetch segments sequentially, process as they arrive
//! - Random access: fetch only segments covering a byte range

use crate::assets::pipeline::{
    compression::{CompressionAlgorithm, CompressionConfig, Compressor},
    encryption::Encryptor,
    key_derivation::derive_master_key,
    orchestrator::DecryptionKey,
    segment::{
        AssetManifest, SegmentIndexEntry, FLAG_INDEX_INLINED, FLAG_SEGMENTED,
        MAX_INLINE_SEGMENTS, segment_count, segment_size_for_asset,
    },
    sharding::{Shard, Sharder, ShardingConfig},
    PipelineError, PipelineInputMetadata, PipelineResult,
};
use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Configuration for the streaming pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingPipelineConfig {
    /// Segment size in bytes (0 = auto-detect based on asset size)
    pub segment_size: u32,
    /// Compression algorithm
    pub compression: CompressionAlgorithm,
    /// Compression level (1-11 for Brotli, 1-22 for Zstd)
    pub compression_level: u8,
    /// Reed-Solomon data shards per segment
    pub rs_data_shards: u8,
    /// Reed-Solomon parity shards per segment
    pub rs_parity_shards: u8,
    /// Content type (MIME) for auto-detect
    pub content_type: String,
}

impl Default for StreamingPipelineConfig {
    fn default() -> Self {
        Self {
            segment_size: 0,
            compression: CompressionAlgorithm::Auto,
            compression_level: 3,
            rs_data_shards: 10,
            rs_parity_shards: 4,
            content_type: "application/octet-stream".to_string(),
        }
    }
}

/// Shards produced for a single segment.
#[derive(Debug, Clone)]
pub struct SegmentShardSet {
    pub segment_index: u32,
    pub encrypted_segment_hash: [u8; 32],
    pub compressed_size: u32,
    pub shards: Vec<Shard>,
}

/// The streaming asset pipeline. Processes assets segment-by-segment.
/// Parallel to (not replacing) the existing AssetPipeline.
pub struct StreamingAssetPipeline {
    config: StreamingPipelineConfig,
    compressor: Compressor,
    encryptor: Encryptor,
}

impl StreamingAssetPipeline {
    /// Create a new streaming pipeline with the given configuration.
    pub fn new(config: StreamingPipelineConfig) -> PipelineResult<Self> {
        let compression_config = CompressionConfig {
            algorithm: config.compression,
            level: config.compression_level as u32,
            ..CompressionConfig::default()
        };
        let compressor = Compressor::new(compression_config);
        let encryptor = Encryptor::default();

        Ok(Self {
            config,
            compressor,
            encryptor,
        })
    }

    /// Process an asset into segments. Each segment is independently
    /// compressed, encrypted, and sharded.
    ///
    /// Returns (manifest, decryption_key, shard_sets).
    pub fn process_segmented(
        &self,
        data: &[u8],
        metadata: &PipelineInputMetadata,
    ) -> PipelineResult<(AssetManifest, DecryptionKey, Vec<SegmentShardSet>)> {
        let seg_size = if self.config.segment_size == 0 {
            segment_size_for_asset(data.len() as u64)
        } else {
            self.config.segment_size
        };
        let seg_count = segment_count(data.len() as u64, seg_size);

        // Generate Kyber keypair, KEM -> shared secret -> master key
        let keypair = self.encryptor.generate_keypair()?;
        let pk = kyber1024::PublicKey::from_bytes(&keypair.public_key)
            .map_err(|_| PipelineError::EncryptionFailed("Invalid Kyber public key".into()))?;
        let (shared_secret, kem_ciphertext) = kyber1024::encapsulate(&pk);
        let master_key = derive_master_key(shared_secret.as_bytes());
        let kem_ct_bytes = kem_ciphertext.as_bytes().to_vec();
        let kem_ct_hash = *blake3::hash(&kem_ct_bytes).as_bytes();

        // Content hash (BLAKE3 of original uncompressed data)
        let content_hash = *blake3::hash(data).as_bytes();

        // Determine effective compression algorithm
        let effective_algo = match self.config.compression {
            CompressionAlgorithm::Auto => {
                self.compressor
                    .select_algorithm(&self.config.content_type, data.len())
            }
            algo => algo,
        };

        let mut segment_index_entries = Vec::with_capacity(seg_count as usize);
        let mut shard_sets = Vec::with_capacity(seg_count as usize);

        for i in 0..seg_count {
            let (segment_entries, segment_shards) = self.process_single_segment(
                data,
                i,
                seg_size,
                effective_algo,
                &master_key,
            )?;
            segment_index_entries.push(segment_entries);
            shard_sets.push(segment_shards);
        }

        let inline_index = if seg_count as usize <= MAX_INLINE_SEGMENTS {
            Some(segment_index_entries)
        } else {
            None
        };

        let compression_algo_byte = match effective_algo {
            CompressionAlgorithm::None => 0u8,
            CompressionAlgorithm::Brotli => 1u8,
            CompressionAlgorithm::Zstd | CompressionAlgorithm::Auto => 2u8,
        };

        let mut flags = FLAG_SEGMENTED;
        if inline_index.is_some() {
            flags |= FLAG_INDEX_INLINED;
        }

        let manifest = AssetManifest {
            version: 1,
            flags,
            content_hash,
            original_size: data.len() as u64,
            segment_size: seg_size,
            segment_count: seg_count,
            compression_algo: compression_algo_byte,
            compression_level: self.config.compression_level,
            encryption_algo: 1,
            rs_data_shards: self.config.rs_data_shards,
            rs_parity_shards: self.config.rs_parity_shards,
            content_type: metadata.content_type.clone(),
            kem_ciphertext_hash: kem_ct_hash,
            index_root_hash: [0u8; 32],
            inline_index,
        };

        let decryption_key = DecryptionKey::KyberSegmented {
            ciphertext_kem: kem_ct_bytes,
            secret_key: keypair.secret_key,
            segment_count: seg_count,
            original_size: data.len() as u64,
        };

        Ok((manifest, decryption_key, shard_sets))
    }

    /// Reconstruct entire asset from segment shard sets.
    pub fn reconstruct_segmented(
        &self,
        manifest: &AssetManifest,
        decryption_key: &DecryptionKey,
        segment_shards: &[Vec<Shard>],
    ) -> PipelineResult<Vec<u8>> {
        let master_key = self.extract_master_key(decryption_key)?;

        if segment_shards.len() != manifest.segment_count as usize {
            return Err(PipelineError::InvalidData(format!(
                "Expected {} segment shard sets, got {}",
                manifest.segment_count,
                segment_shards.len()
            )));
        }

        let decompress_algo = algo_from_byte(manifest.compression_algo)?;

        let mut output = Vec::with_capacity(manifest.original_size as usize);

        for (i, shards) in segment_shards.iter().enumerate() {
            let segment_data =
                self.reconstruct_single_segment(shards, &master_key, i as u32, manifest, decompress_algo)?;
            output.extend_from_slice(&segment_data);
        }

        output.truncate(manifest.original_size as usize);

        let hash = *blake3::hash(&output).as_bytes();
        if hash != manifest.content_hash {
            return Err(PipelineError::InvalidData(
                "Content hash mismatch after reconstruction".into(),
            ));
        }

        Ok(output)
    }

    /// Reconstruct a byte range from specific segments.
    /// Only fetches and processes the segments covering the requested range.
    pub fn reconstruct_range(
        &self,
        manifest: &AssetManifest,
        decryption_key: &DecryptionKey,
        segment_shards: &[(u32, Vec<Shard>)],
        byte_range: std::ops::Range<u64>,
    ) -> PipelineResult<Vec<u8>> {
        let master_key = self.extract_master_key(decryption_key)?;
        let decompress_algo = algo_from_byte(manifest.compression_algo)?;
        let seg_size = manifest.segment_size as u64;

        let mut result = Vec::new();

        for (seg_idx, shards) in segment_shards {
            let decompressed = self.reconstruct_single_segment(
                shards,
                &master_key,
                *seg_idx,
                manifest,
                decompress_algo,
            )?;

            let seg_start = *seg_idx as u64 * seg_size;
            let seg_end = std::cmp::min(seg_start + seg_size, manifest.original_size);

            let range_start_in_seg = if byte_range.start > seg_start {
                (byte_range.start - seg_start) as usize
            } else {
                0
            };
            let range_end_in_seg = if byte_range.end < seg_end {
                (byte_range.end - seg_start) as usize
            } else {
                (seg_end - seg_start) as usize
            };

            if range_start_in_seg < decompressed.len() && range_start_in_seg < range_end_in_seg {
                let end = std::cmp::min(range_end_in_seg, decompressed.len());
                result.extend_from_slice(&decompressed[range_start_in_seg..end]);
            }
        }

        Ok(result)
    }

    /// Process asset from an async reader -- never loads the full asset into memory.
    /// Reads segment_size chunks, processes each independently.
    pub async fn process_stream<R: tokio::io::AsyncRead + Unpin>(
        &self,
        mut reader: R,
        total_size: u64,
        metadata: &PipelineInputMetadata,
    ) -> PipelineResult<(AssetManifest, DecryptionKey, Vec<SegmentShardSet>)> {
        let seg_size = if self.config.segment_size == 0 {
            segment_size_for_asset(total_size)
        } else {
            self.config.segment_size
        };
        let seg_count = segment_count(total_size, seg_size);

        // Generate Kyber keypair, KEM -> shared secret -> master key
        let keypair = self.encryptor.generate_keypair()?;
        let pk = kyber1024::PublicKey::from_bytes(&keypair.public_key)
            .map_err(|_| PipelineError::EncryptionFailed("Invalid Kyber public key".into()))?;
        let (shared_secret, kem_ciphertext) = kyber1024::encapsulate(&pk);
        let master_key = derive_master_key(shared_secret.as_bytes());
        let kem_ct_bytes = kem_ciphertext.as_bytes().to_vec();
        let kem_ct_hash = *blake3::hash(&kem_ct_bytes).as_bytes();

        // Streaming BLAKE3 hasher (incremental -- never buffers full asset)
        let mut hasher = blake3::Hasher::new();

        // Determine effective compression algorithm
        let effective_algo = match self.config.compression {
            CompressionAlgorithm::Auto => {
                self.compressor
                    .select_algorithm(&self.config.content_type, total_size as usize)
            }
            algo => algo,
        };

        let mut segment_index_entries = Vec::with_capacity(seg_count as usize);
        let mut shard_sets = Vec::with_capacity(seg_count as usize);

        for i in 0..seg_count {
            let expected_len = if i == seg_count - 1 {
                let remaining = total_size - (i as u64 * seg_size as u64);
                remaining as usize
            } else {
                seg_size as usize
            };

            let mut segment_buf = vec![0u8; expected_len];
            reader
                .read_exact(&mut segment_buf)
                .await
                .map_err(|e| {
                    PipelineError::InvalidData(format!("Failed to read segment {}: {}", i, e))
                })?;

            // Update incremental hash
            hasher.update(&segment_buf);

            // Process this segment: compress -> encrypt -> shard
            let (entry, shard_set) = self.process_segment_buf(
                &segment_buf,
                i,
                effective_algo,
                &master_key,
            )?;
            segment_index_entries.push(entry);
            shard_sets.push(shard_set);
            // segment_buf dropped here -- memory freed before next segment
        }

        let content_hash = *hasher.finalize().as_bytes();

        let inline_index = if seg_count as usize <= MAX_INLINE_SEGMENTS {
            Some(segment_index_entries)
        } else {
            None
        };

        let compression_algo_byte = match effective_algo {
            CompressionAlgorithm::None => 0u8,
            CompressionAlgorithm::Brotli => 1u8,
            CompressionAlgorithm::Zstd | CompressionAlgorithm::Auto => 2u8,
        };

        let mut flags = FLAG_SEGMENTED;
        if inline_index.is_some() {
            flags |= FLAG_INDEX_INLINED;
        }

        let manifest = AssetManifest {
            version: 1,
            flags,
            content_hash,
            original_size: total_size,
            segment_size: seg_size,
            segment_count: seg_count,
            compression_algo: compression_algo_byte,
            compression_level: self.config.compression_level,
            encryption_algo: 1,
            rs_data_shards: self.config.rs_data_shards,
            rs_parity_shards: self.config.rs_parity_shards,
            content_type: metadata.content_type.clone(),
            kem_ciphertext_hash: kem_ct_hash,
            index_root_hash: [0u8; 32],
            inline_index,
        };

        let decryption_key = DecryptionKey::KyberSegmented {
            ciphertext_kem: kem_ct_bytes,
            secret_key: keypair.secret_key,
            segment_count: seg_count,
            original_size: total_size,
        };

        Ok((manifest, decryption_key, shard_sets))
    }

    /// Reconstruct asset to an async writer -- bounded memory.
    /// Processes one segment at a time, writes output incrementally.
    pub async fn reconstruct_to_writer<W: tokio::io::AsyncWrite + Unpin>(
        &self,
        manifest: &AssetManifest,
        decryption_key: &DecryptionKey,
        segment_shards: &[Vec<Shard>],
        mut writer: W,
    ) -> PipelineResult<()> {
        let master_key = self.extract_master_key(decryption_key)?;
        let decompress_algo = algo_from_byte(manifest.compression_algo)?;

        if segment_shards.len() != manifest.segment_count as usize {
            return Err(PipelineError::InvalidData(format!(
                "Expected {} segment shard sets, got {}",
                manifest.segment_count,
                segment_shards.len()
            )));
        }

        let mut hasher = blake3::Hasher::new();
        let mut bytes_written: u64 = 0;

        for (i, shards) in segment_shards.iter().enumerate() {
            let decompressed = self.reconstruct_single_segment(
                shards,
                &master_key,
                i as u32,
                manifest,
                decompress_algo,
            )?;

            let remaining = manifest.original_size - bytes_written;
            let write_len = std::cmp::min(decompressed.len() as u64, remaining) as usize;

            hasher.update(&decompressed[..write_len]);
            writer
                .write_all(&decompressed[..write_len])
                .await
                .map_err(|e| {
                    PipelineError::InvalidData(format!("Write failed at segment {}: {}", i, e))
                })?;
            bytes_written += write_len as u64;
            // decompressed dropped here -- memory freed
        }

        writer.flush().await.map_err(|e| {
            PipelineError::InvalidData(format!("Flush failed: {}", e))
        })?;

        // Verify content hash
        let hash = *hasher.finalize().as_bytes();
        if hash != manifest.content_hash {
            return Err(PipelineError::InvalidData(
                "Content hash mismatch after reconstruction".into(),
            ));
        }

        Ok(())
    }

    // ── Private helpers ─────────────────────────────────────────────────

    /// Process a single segment: compress, encrypt, shard.
    fn process_single_segment(
        &self,
        data: &[u8],
        segment_index: u32,
        seg_size: u32,
        algo: CompressionAlgorithm,
        master_key: &[u8; 32],
    ) -> PipelineResult<(SegmentIndexEntry, SegmentShardSet)> {
        let start = (segment_index as u64 * seg_size as u64) as usize;
        let end = std::cmp::min(start + seg_size as usize, data.len());
        let segment_data = &data[start..end];

        // Compress
        let compressed = self.compress_segment(segment_data, algo)?;
        let compressed_size = compressed.len() as u32;

        // Encrypt (per-segment HKDF key)
        let (encrypted, _) =
            self.encryptor
                .encrypt_segment(&compressed, master_key, segment_index)?;
        let encrypted_hash = *blake3::hash(&encrypted).as_bytes();

        // Shard (RS per segment)
        let shard_config = ShardingConfig {
            data_shards: self.config.rs_data_shards as usize,
            parity_shards: self.config.rs_parity_shards as usize,
            target_shard_size: 1024 * 1024,
        };
        let sharder = Sharder::new(shard_config)?;
        let (shards, _) = sharder.shard(&encrypted)?;

        let entry = SegmentIndexEntry {
            encrypted_segment_hash: encrypted_hash,
            compressed_size,
        };

        let shard_set = SegmentShardSet {
            segment_index,
            encrypted_segment_hash: encrypted_hash,
            compressed_size,
            shards,
        };

        Ok((entry, shard_set))
    }

    /// Process a single segment from a pre-sliced buffer: compress, encrypt, shard.
    /// Used by `process_stream` where segments are read from a reader one at a time.
    fn process_segment_buf(
        &self,
        segment_data: &[u8],
        segment_index: u32,
        algo: CompressionAlgorithm,
        master_key: &[u8; 32],
    ) -> PipelineResult<(SegmentIndexEntry, SegmentShardSet)> {
        let compressed = self.compress_segment(segment_data, algo)?;
        let compressed_size = compressed.len() as u32;

        let (encrypted, _) =
            self.encryptor
                .encrypt_segment(&compressed, master_key, segment_index)?;
        let encrypted_hash = *blake3::hash(&encrypted).as_bytes();

        let shard_config = ShardingConfig {
            data_shards: self.config.rs_data_shards as usize,
            parity_shards: self.config.rs_parity_shards as usize,
            target_shard_size: 1024 * 1024,
        };
        let sharder = Sharder::new(shard_config)?;
        let (shards, _) = sharder.shard(&encrypted)?;

        let entry = SegmentIndexEntry {
            encrypted_segment_hash: encrypted_hash,
            compressed_size,
        };

        let shard_set = SegmentShardSet {
            segment_index,
            encrypted_segment_hash: encrypted_hash,
            compressed_size,
            shards,
        };

        Ok((entry, shard_set))
    }

    /// Compress segment data with the given algorithm.
    fn compress_segment(
        &self,
        segment_data: &[u8],
        algo: CompressionAlgorithm,
    ) -> PipelineResult<Vec<u8>> {
        match algo {
            CompressionAlgorithm::None => Ok(segment_data.to_vec()),
            _ => {
                let comp_config = CompressionConfig {
                    algorithm: algo,
                    level: self.config.compression_level as u32,
                    ..CompressionConfig::default()
                };
                let temp_compressor = Compressor::new(comp_config);
                let (compressed_data, _) = temp_compressor.compress(segment_data)?;
                Ok(compressed_data)
            }
        }
    }

    /// Reconstruct a single segment from its shards: unshard, decrypt, decompress.
    fn reconstruct_single_segment(
        &self,
        shards: &[Shard],
        master_key: &[u8; 32],
        segment_index: u32,
        manifest: &AssetManifest,
        decompress_algo: CompressionAlgorithm,
    ) -> PipelineResult<Vec<u8>> {
        let shard_config = ShardingConfig {
            data_shards: manifest.rs_data_shards as usize,
            parity_shards: manifest.rs_parity_shards as usize,
            target_shard_size: 1024 * 1024,
        };
        let sharder = Sharder::new(shard_config)?;
        let encrypted = sharder.reconstruct(shards)?;

        let compressed =
            self.encryptor
                .decrypt_segment(&encrypted, master_key, segment_index)?;

        match decompress_algo {
            CompressionAlgorithm::None => Ok(compressed),
            algo => {
                let comp_config = CompressionConfig {
                    algorithm: algo,
                    level: manifest.compression_level as u32,
                    ..CompressionConfig::default()
                };
                Compressor::new(comp_config).decompress(&compressed)
            }
        }
    }

    /// Extract and derive the master key from a DecryptionKey.
    fn extract_master_key(&self, decryption_key: &DecryptionKey) -> PipelineResult<[u8; 32]> {
        let (kem_ct, secret_key) = match decryption_key {
            DecryptionKey::KyberSegmented {
                ciphertext_kem,
                secret_key,
                ..
            } => (ciphertext_kem, secret_key),
            _ => {
                return Err(PipelineError::InvalidData(
                    "Segmented reconstruction requires DecryptionKey::KyberSegmented".into(),
                ))
            }
        };

        let sk = kyber1024::SecretKey::from_bytes(secret_key)
            .map_err(|_| PipelineError::EncryptionFailed("Invalid Kyber secret key".into()))?;
        let ct = kyber1024::Ciphertext::from_bytes(kem_ct)
            .map_err(|_| PipelineError::EncryptionFailed("Invalid Kyber ciphertext".into()))?;
        let shared_secret = kyber1024::decapsulate(&ct, &sk);
        Ok(derive_master_key(shared_secret.as_bytes()))
    }
}

/// Convert a compression algorithm byte from the manifest to enum.
fn algo_from_byte(byte: u8) -> PipelineResult<CompressionAlgorithm> {
    match byte {
        0 => Ok(CompressionAlgorithm::None),
        1 => Ok(CompressionAlgorithm::Brotli),
        2 => Ok(CompressionAlgorithm::Zstd),
        other => Err(PipelineError::InvalidData(format!(
            "Unknown compression algorithm: {}",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_metadata() -> PipelineInputMetadata {
        PipelineInputMetadata {
            name: "test.bin".to_string(),
            content_type: "application/octet-stream".to_string(),
            size: 0,
            created_at: 0,
            custom: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_process_segmented_small_asset() {
        let data = vec![42u8; 1000];
        let config = StreamingPipelineConfig {
            segment_size: 4 * 1024 * 1024,
            compression: CompressionAlgorithm::None,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: create pipeline");
        let (manifest, key, shard_sets) = pipeline
            .process_segmented(&data, &test_metadata())
            .expect("test: process");

        assert_eq!(manifest.segment_count, 1);
        assert_eq!(manifest.version, 1);
        assert_eq!(shard_sets.len(), 1);
        assert!(manifest.inline_index.is_some());

        match &key {
            DecryptionKey::KyberSegmented {
                segment_count,
                original_size,
                ..
            } => {
                assert_eq!(*segment_count, 1);
                assert_eq!(*original_size, 1000);
            }
            _ => unreachable!("test: expected KyberSegmented"),
        }
    }

    #[test]
    fn test_process_segmented_multi_segment() {
        let data = vec![0xABu8; 12_000];
        let config = StreamingPipelineConfig {
            segment_size: 4000,
            compression: CompressionAlgorithm::None,
            rs_data_shards: 4,
            rs_parity_shards: 2,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: create pipeline");
        let (manifest, _, shard_sets) = pipeline
            .process_segmented(&data, &test_metadata())
            .expect("test: process");

        assert_eq!(manifest.segment_count, 3);
        assert_eq!(shard_sets.len(), 3);

        for set in &shard_sets {
            assert_eq!(set.shards.len(), 6);
        }
    }

    #[test]
    fn test_process_and_reconstruct_roundtrip() {
        let data: Vec<u8> = (0..20_000).map(|i| (i % 256) as u8).collect();
        let config = StreamingPipelineConfig {
            segment_size: 5000,
            compression: CompressionAlgorithm::None,
            rs_data_shards: 4,
            rs_parity_shards: 2,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: create pipeline");
        let (manifest, key, shard_sets) = pipeline
            .process_segmented(&data, &test_metadata())
            .expect("test: process");

        let all_shards: Vec<Vec<Shard>> = shard_sets.into_iter().map(|s| s.shards).collect();
        let reconstructed = pipeline
            .reconstruct_segmented(&manifest, &key, &all_shards)
            .expect("test: reconstruct");

        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_process_and_reconstruct_with_zstd() {
        let data = b"Hello zstd streaming pipeline! ".repeat(1000);
        let config = StreamingPipelineConfig {
            segment_size: 10_000,
            compression: CompressionAlgorithm::Zstd,
            compression_level: 3,
            rs_data_shards: 4,
            rs_parity_shards: 2,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: create pipeline");
        let (manifest, key, shard_sets) = pipeline
            .process_segmented(&data, &test_metadata())
            .expect("test: process");

        assert_eq!(manifest.compression_algo, 2);

        let all_shards: Vec<Vec<Shard>> = shard_sets.into_iter().map(|s| s.shards).collect();
        let reconstructed = pipeline
            .reconstruct_segmented(&manifest, &key, &all_shards)
            .expect("test: reconstruct");

        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_reconstruct_range_single_segment() {
        let data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        let config = StreamingPipelineConfig {
            segment_size: 5000,
            compression: CompressionAlgorithm::None,
            rs_data_shards: 4,
            rs_parity_shards: 2,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: create pipeline");
        let (manifest, key, shard_sets) = pipeline
            .process_segmented(&data, &test_metadata())
            .expect("test: process");

        let range_shards = vec![(0u32, shard_sets[0].shards.clone())];
        let range_data = pipeline
            .reconstruct_range(&manifest, &key, &range_shards, 100..200)
            .expect("test: range");

        assert_eq!(range_data, &data[100..200]);
    }

    #[test]
    fn test_reconstruct_range_cross_segment() {
        let data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        let config = StreamingPipelineConfig {
            segment_size: 5000,
            compression: CompressionAlgorithm::None,
            rs_data_shards: 4,
            rs_parity_shards: 2,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: create pipeline");
        let (manifest, key, shard_sets) = pipeline
            .process_segmented(&data, &test_metadata())
            .expect("test: process");

        let range_shards = vec![
            (0u32, shard_sets[0].shards.clone()),
            (1u32, shard_sets[1].shards.clone()),
        ];
        let range_data = pipeline
            .reconstruct_range(&manifest, &key, &range_shards, 4900..5100)
            .expect("test: range");

        assert_eq!(range_data, &data[4900..5100]);
    }

    #[test]
    fn test_manifest_content_hash_matches() {
        let data = vec![0xFFu8; 8000];
        let config = StreamingPipelineConfig {
            segment_size: 4000,
            compression: CompressionAlgorithm::None,
            rs_data_shards: 4,
            rs_parity_shards: 2,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: create pipeline");
        let (manifest, _, _) = pipeline
            .process_segmented(&data, &test_metadata())
            .expect("test: process");

        let expected_hash = *blake3::hash(&data).as_bytes();
        assert_eq!(manifest.content_hash, expected_hash);
    }

    #[test]
    fn test_segment_independence() {
        let data: Vec<u8> = (0..15_000).map(|i| (i % 256) as u8).collect();
        let config = StreamingPipelineConfig {
            segment_size: 5000,
            compression: CompressionAlgorithm::None,
            rs_data_shards: 4,
            rs_parity_shards: 2,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: create pipeline");
        let (manifest, key, shard_sets) = pipeline
            .process_segmented(&data, &test_metadata())
            .expect("test: process");

        // Reconstruct only segment 1 (middle segment)
        let range_shards = vec![(1u32, shard_sets[1].shards.clone())];
        let seg1_data = pipeline
            .reconstruct_range(&manifest, &key, &range_shards, 5000..10000)
            .expect("test: reconstruct segment 1");

        assert_eq!(seg1_data, &data[5000..10000]);
    }

    #[tokio::test]
    async fn test_process_stream_matches_process_segmented() {
        let data: Vec<u8> = (0..20_000).map(|i| (i % 256) as u8).collect();
        let config = StreamingPipelineConfig {
            segment_size: 5000,
            compression: CompressionAlgorithm::None,
            rs_data_shards: 4,
            rs_parity_shards: 2,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: pipeline");
        let meta = test_metadata();

        // Process via streaming
        let cursor = tokio::io::BufReader::new(&data[..]);
        let (manifest_stream, key_stream, shards_stream) = pipeline
            .process_stream(cursor, data.len() as u64, &meta)
            .await
            .expect("test: stream process");

        // Reconstruct from streaming result
        let all_shards: Vec<Vec<Shard>> =
            shards_stream.into_iter().map(|s| s.shards).collect();
        let reconstructed = pipeline
            .reconstruct_segmented(&manifest_stream, &key_stream, &all_shards)
            .expect("test: reconstruct");

        assert_eq!(reconstructed, data);
        assert_eq!(
            manifest_stream.content_hash,
            *blake3::hash(&data).as_bytes()
        );
    }

    #[tokio::test]
    async fn test_reconstruct_to_writer_matches_reconstruct_segmented() {
        let data: Vec<u8> = (0..15_000).map(|i| (i % 256) as u8).collect();
        let config = StreamingPipelineConfig {
            segment_size: 5000,
            compression: CompressionAlgorithm::None,
            rs_data_shards: 4,
            rs_parity_shards: 2,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: pipeline");
        let (manifest, key, shard_sets) = pipeline
            .process_segmented(&data, &test_metadata())
            .expect("test: process");

        let all_shards: Vec<Vec<Shard>> =
            shard_sets.into_iter().map(|s| s.shards).collect();

        // Reconstruct to writer
        let mut output = Vec::new();
        pipeline
            .reconstruct_to_writer(&manifest, &key, &all_shards, &mut output)
            .await
            .expect("test: reconstruct to writer");

        assert_eq!(output, data);
    }

    #[tokio::test]
    async fn test_stream_large_asset() {
        // 500KB through streaming (larger than segment size)
        let data: Vec<u8> = (0..500_000).map(|i| (i % 256) as u8).collect();
        let config = StreamingPipelineConfig {
            segment_size: 50_000, // 10 segments
            compression: CompressionAlgorithm::Zstd,
            compression_level: 1,
            rs_data_shards: 4,
            rs_parity_shards: 2,
            ..Default::default()
        };
        let pipeline = StreamingAssetPipeline::new(config).expect("test: pipeline");
        let meta = test_metadata();

        let cursor = tokio::io::BufReader::new(&data[..]);
        let (manifest, key, shard_sets) = pipeline
            .process_stream(cursor, data.len() as u64, &meta)
            .await
            .expect("test: stream process");

        assert_eq!(manifest.segment_count, 10);

        let all_shards: Vec<Vec<Shard>> =
            shard_sets.into_iter().map(|s| s.shards).collect();
        let mut output = Vec::new();
        pipeline
            .reconstruct_to_writer(&manifest, &key, &all_shards, &mut output)
            .await
            .expect("test: reconstruct");

        assert_eq!(output, data);
    }
}
