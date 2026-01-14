//! Asset Pipeline - Compression → Encryption → Sharding → Distribution
//!
//! Revolutionary asset processing pipeline with matrix-aware distribution.
//!
//! ## Pipeline Stages
//!
//! 1. **Compression** (Brotli): Configurable compression levels 1-11 with streaming support
//! 2. **Encryption** (Kyber-1024 + AES-256-GCM): Quantum-resistant encryption
//! 3. **Sharding** (Reed-Solomon): Erasure coding with configurable redundancy
//! 4. **Distribution** (Matrix-aware): Optimal shard placement using tensor operations
//!
//! ## Performance Targets
//!
//! - **Throughput**: 1GB/s end-to-end
//! - **Compression**: Brotli level 4 (balance speed/ratio)
//! - **Encryption**: AES-256-GCM with hardware acceleration
//! - **Sharding**: 10+4 Reed-Solomon (10 data, 4 parity)
//! - **Distribution**: <100ms to calculate placement

#![deny(unsafe_code)]

use std::sync::Arc;
use anyhow::Result;
use serde::{Serialize, Deserialize};

// Pipeline stages
pub mod compression;
pub mod encryption;
pub mod sharding;
pub mod distribution;
pub mod orchestrator;

// Re-exports
pub use compression::{Compressor, CompressionConfig, CompressionAlgorithm, CompressionStats};
pub use encryption::{Encryptor, EncryptionConfig, EncryptionStats, EncryptedData, ShardKey};
pub use sharding::{Sharder, ShardingConfig, Shard, ShardMetadata, ShardingStats};
pub use distribution::{
    MatrixDistributor, DistributionConfig, DistributedAsset, ShardPlacement,
    MatrixConstraints, DistributionStats,
};
pub use orchestrator::{AssetPipeline, PipelineConfig, PipelineStats, ProcessedAsset};

/// Raw asset data to be processed
#[derive(Clone, Debug)]
pub struct Asset {
    /// Asset identifier
    pub id: String,
    /// Raw asset data
    pub data: Vec<u8>,
    /// Asset metadata
    pub metadata: AssetMetadata,
}

/// Asset metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetMetadata {
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

impl Default for AssetMetadata {
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
            metadata: AssetMetadata {
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
