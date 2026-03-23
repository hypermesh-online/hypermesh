// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Reconstruction pipeline: Reed-Solomon reassembly, decryption, decompression.

use anyhow::Result;
use std::collections::HashMap;

use crate::assets::pipeline::{
    encryption::{EncryptedData, KyberEncryptionResult},
    orchestrator::DecryptionKey,
    sharding::{Shard, ShardMetadata},
    CompressionAlgorithm, CompressionConfig, Compressor, EncryptionConfig, Encryptor, Sharder,
    ShardingConfig,
};

use super::super::{RetrievalMetadata, RetrievalPlan};
use super::{ClientAssembler, FetchedShard};

impl ClientAssembler {
    /// Reconstruct file from fetched shards (basic concatenation).
    ///
    /// This method concatenates raw shard data without pipeline processing.
    /// Use `reconstruct_with_pipeline()` for full reverse-pipeline
    /// reconstruction (Reed-Solomon -> decrypt -> decompress).
    pub async fn reconstruct(&self) -> Result<Vec<u8>> {
        let plan = self.plan.read().await;
        let plan = plan
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No retrieval plan set"))?;

        let fetched = self.fetched_shards.read().await;

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
        let plan = plan
            .as_ref()
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

        let encrypted_blob = Self::reconstruct_shards(&pipeline_shards, &plan.metadata)?;
        let compressed_data = Self::decrypt_blob(&encrypted_blob, decryption_key, &plan.metadata)?;
        let original_data = Self::decompress_data(&compressed_data, &plan.metadata)?;

        Ok(original_data)
    }

    /// Convert fetched shards into pipeline Shard structs.
    pub(super) fn build_pipeline_shards(
        &self,
        fetched: &HashMap<usize, FetchedShard>,
        plan: &RetrievalPlan,
    ) -> Vec<Shard> {
        let data_shard_count = plan.metadata.erasure_coding.0;

        let mut shards: Vec<Shard> = Vec::with_capacity(fetched.len());

        for (&idx, fetched_shard) in fetched.iter() {
            let is_parity = idx >= data_shard_count;
            let hash_hex = hex::encode(blake3::hash(&fetched_shard.data).as_bytes());

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

        shards.sort_by_key(|s| s.metadata.index);
        shards
    }

    /// Stage 1: Reed-Solomon reconstruct encrypted blob from shards.
    fn reconstruct_shards(shards: &[Shard], metadata: &RetrievalMetadata) -> Result<Vec<u8>> {
        let config = ShardingConfig {
            data_shards: metadata.erasure_coding.0,
            parity_shards: metadata.erasure_coding.1,
            ..Default::default()
        };
        let sharder =
            Sharder::new(config).map_err(|e| anyhow::anyhow!("Sharder init failed: {e}"))?;

        sharder
            .reconstruct(shards)
            .map_err(|e| anyhow::anyhow!("Shard reconstruction failed: {e}"))
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
                encryptor
                    .decrypt(&kyber_result, secret_key)
                    .map_err(|e| anyhow::anyhow!("Kyber decryption failed: {e}"))
            }
            DecryptionKey::Aes(key) => {
                let encrypted = EncryptedData {
                    ciphertext: encrypted_blob.to_vec(),
                    nonce: key.nonce.clone(),
                    original_size: 0,
                };
                encryptor
                    .decrypt_aes(&encrypted, key)
                    .map_err(|e| anyhow::anyhow!("AES decryption failed: {e}"))
            }
            DecryptionKey::KyberSegmented { .. } => {
                Err(anyhow::anyhow!(
                    "KyberSegmented assets must use StreamingAssetPipeline for reconstruction"
                ))
            }
        }
    }

    /// Stage 3: Decompress data.
    fn decompress_data(compressed_data: &[u8], metadata: &RetrievalMetadata) -> Result<Vec<u8>> {
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

        compressor
            .decompress(compressed_data)
            .map_err(|e| anyhow::anyhow!("Decompression failed: {e}"))
    }
}
