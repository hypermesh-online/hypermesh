// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset Pipeline - Compression → Encryption → Sharding → Distribution
//!
//! Revolutionary asset processing pipeline with matrix-aware distribution.
//!
//! ## Pipeline Stages (EXACT ORDER)
//!
//! 1. **Compression** (Brotli): Compress raw data first for best ratio (levels 1-11)
//! 2. **Encryption** (Kyber-1024 KEM + AES-256-GCM): Encrypt the entire compressed blob
//! 3. **Sharding** (Reed-Solomon): Split encrypted data into erasure-coded shards
//! 4. **Distribution** (Matrix-aware): Place shards at optimal matrix positions
//!
//! ## Performance Targets
//!
//! - **Throughput**: 1GB/s end-to-end
//! - **Compression**: Brotli level 4 (balance speed/ratio)
//! - **Encryption**: Kyber-1024 KEM + AES-256-GCM whole-blob
//! - **Sharding**: 10+4 Reed-Solomon (10 data, 4 parity)
//! - **Distribution**: <100ms to calculate placement

#![deny(unsafe_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};

// Pipeline stages
pub mod compression;
pub mod distribution;
pub mod encryption;
pub mod key_derivation;
pub mod orchestrator;
pub mod segment;
pub mod sharding;
pub mod streaming_pipeline;

// Re-exports
pub use compression::{CompressionAlgorithm, CompressionConfig, CompressionStats, Compressor};
pub use distribution::{
    DistributedAsset, DistributionConfig, DistributionStats, MatrixConstraints, MatrixDistributor,
    ShardPlacement,
};
pub use encryption::{
    AesKey, EncryptedData, EncryptionConfig, EncryptionStats, Encryptor, KyberEncryptionResult,
    KyberKeyPair,
};
pub use orchestrator::{
    AssetPipeline, DecryptionKey, PipelineConfig, PipelineStats, ProcessedAsset,
};
pub use sharding::{Shard, ShardMetadata, Sharder, ShardingConfig, ShardingStats};

/// Raw asset data to be processed
#[derive(Clone, Debug)]
pub struct Asset {
    /// Asset identifier
    pub id: String,
    /// Raw asset data
    pub data: Vec<u8>,
    /// Asset metadata
    pub metadata: PipelineInputMetadata,
}

/// Input metadata describing an asset entering the pipeline.
///
/// This is intentionally distinct from `hypermesh_lib::AssetMetadata` (the
/// canonical cross-crate metadata). Pipeline input metadata carries
/// content-type and a free-form custom map, which are only relevant during
/// ingestion into the Compress->Encrypt->Shard->Distribute pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineInputMetadata {
    /// Asset name
    pub name: String,
    /// Content type
    pub content_type: String,
    /// Original size in bytes
    pub size: usize,
    /// Creation timestamp
    pub created_at: i64,
    /// Custom metadata
    pub custom: std::collections::HashMap<String, String>,
}

impl Default for PipelineInputMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            content_type: "application/octet-stream".to_string(),
            size: 0,
            created_at: 0,
            custom: std::collections::HashMap::new(),
        }
    }
}

/// Pipeline error types
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Sharding failed: {0}")]
    ShardingFailed(String),

    #[error("Distribution failed: {0}")]
    DistributionFailed(String),

    #[error("Deduplication failed: {0}")]
    DeduplicationFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

pub type PipelineResult<T> = Result<T, PipelineError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_creation() {
        let asset = Asset {
            id: "test-asset-1".to_string(),
            data: vec![1, 2, 3, 4, 5],
            metadata: PipelineInputMetadata {
                name: "test.bin".to_string(),
                content_type: "application/octet-stream".to_string(),
                size: 5,
                created_at: 1234567890,
                custom: std::collections::HashMap::new(),
            },
        };

        assert_eq!(asset.id, "test-asset-1");
        assert_eq!(asset.data.len(), 5);
        assert_eq!(asset.metadata.size, 5);
    }
}
