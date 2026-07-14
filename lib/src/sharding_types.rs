// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation

//! Shared sharding types (R5, R14).
//!
//! Canonical cross-crate definitions for erasure-coded shards. The Reed-Solomon
//! sharding ENGINE lives in the `engauge` crate (NGauge is the sharding
//! authority); these are the shared data structures it produces and that
//! blockmatrix (and any other crate) consumes. Keeping them in `hypermesh-lib`
//! — which every crate already depends on — lets the engine live in engauge
//! without forcing a dependency cycle.
//!
//! The shard is an atomic content-addressed unit: after creation it must NEVER
//! be sub-divided (that would break the BLAKE3 hash mappings used by dedup,
//! buckets, shard maps, and commitments — see R14).

use serde::{Deserialize, Serialize};

/// Errors from the sharding engine (config validation + Reed-Solomon codec).
///
/// Defined here in `hypermesh-lib` so the engine (engauge) and its consumers
/// (blockmatrix) share one error type without a dependency cycle. blockmatrix's
/// pipeline `PipelineError` provides a `From<ShardingError>` conversion so
/// existing call sites keep returning `PipelineError`.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ShardingError {
    /// The Reed-Solomon encode/decode step failed, or reconstruction was
    /// requested with insufficient/corrupt shards.
    #[error("Sharding failed: {0}")]
    ShardingFailed(String),

    /// The sharding configuration was invalid (e.g. zero data/parity shards).
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Shard metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShardMetadata {
    /// Shard index
    pub index: usize,
    /// Is this a parity shard?
    pub is_parity: bool,
    /// Shard size in bytes
    pub size: usize,
    /// Original data size (before padding)
    pub original_size: usize,
    /// Hash of shard data (for integrity)
    pub hash: String,
}

/// Individual shard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shard {
    /// Shard data
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    /// Shard metadata
    pub metadata: ShardMetadata,
}

impl Shard {
    /// Calculate the BLAKE3 hash of shard data (hex-encoded).
    ///
    /// Public so the sharding engine (engauge) can stamp the integrity hash
    /// into [`ShardMetadata::hash`] when it produces shards.
    pub fn calculate_hash(data: &[u8]) -> String {
        hex::encode(blake3::hash(data).as_bytes())
    }

    /// Verify shard integrity by recomputing the data hash.
    pub fn verify(&self) -> bool {
        Self::calculate_hash(&self.data) == self.metadata.hash
    }
}

/// Sharding statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShardingStats {
    /// Original data size
    pub original_size: usize,
    /// Total sharded size (all shards)
    pub total_shard_size: usize,
    /// Number of data shards
    pub data_shards: usize,
    /// Number of parity shards
    pub parity_shards: usize,
    /// Redundancy factor (total/original)
    pub redundancy_factor: f64,
    /// Sharding time in milliseconds
    pub duration_ms: u64,
    /// Throughput in MB/s
    pub throughput_mbps: f64,
}

impl ShardingStats {
    /// Compute derived statistics for a completed sharding operation.
    ///
    /// Public so the sharding engine (engauge) can build the stats it returns.
    pub fn calculate(
        original_size: usize,
        total_shard_size: usize,
        data_shards: usize,
        parity_shards: usize,
        duration_ms: u64,
    ) -> Self {
        let redundancy_factor = if original_size > 0 {
            total_shard_size as f64 / original_size as f64
        } else {
            0.0
        };

        let throughput_mbps = if duration_ms > 0 {
            (original_size as f64 / (1024.0 * 1024.0)) / (duration_ms as f64 / 1000.0)
        } else if original_size > 0 {
            // If duration is too small to measure, use a minimum of 0.001ms (1 microsecond)
            (original_size as f64 / (1024.0 * 1024.0)) / 0.001
        } else {
            0.0
        };

        Self {
            original_size,
            total_shard_size,
            data_shards,
            parity_shards,
            redundancy_factor,
            duration_ms,
            throughput_mbps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_hash_and_verify() {
        let data = b"integrity test payload".to_vec();
        let hash = Shard::calculate_hash(&data);
        let shard = Shard {
            data: data.clone(),
            metadata: ShardMetadata {
                index: 0,
                is_parity: false,
                size: data.len(),
                original_size: data.len(),
                hash,
            },
        };
        assert!(shard.verify());

        let mut corrupted = shard.clone();
        corrupted.data[0] ^= 1;
        assert!(!corrupted.verify());
    }

    #[test]
    fn test_sharding_stats_calculate() {
        let stats = ShardingStats::calculate(1_000_000, 1_400_000, 10, 4, 5);
        assert_eq!(stats.original_size, 1_000_000);
        assert_eq!(stats.total_shard_size, 1_400_000);
        assert_eq!(stats.data_shards, 10);
        assert_eq!(stats.parity_shards, 4);
        assert!(stats.redundancy_factor > 1.0);
        assert!(stats.throughput_mbps > 0.0);
    }

    #[test]
    fn test_shard_serde_roundtrip() {
        let shard = Shard {
            data: vec![1, 2, 3, 4, 5],
            metadata: ShardMetadata {
                index: 3,
                is_parity: true,
                size: 5,
                original_size: 5,
                hash: Shard::calculate_hash(&[1, 2, 3, 4, 5]),
            },
        };
        let bytes = serde_json::to_vec(&shard).expect("test: serialize shard");
        let back: Shard = serde_json::from_slice(&bytes).expect("test: deserialize shard");
        assert_eq!(back.data, shard.data);
        assert_eq!(back.metadata.index, 3);
        assert!(back.metadata.is_parity);
    }
}
