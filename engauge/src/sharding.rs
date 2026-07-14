// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation

//! Sharding engine — Reed-Solomon erasure coding (R5, R14).
//!
//! NGauge (engauge) is the sharding authority: this is the production
//! erasure-coding engine that blockmatrix (and any other consumer) calls to
//! turn an encrypted blob into `k`-of-`n` recoverable shards and back.
//!
//! Uses the `reed-solomon-erasure` crate for production-quality erasure coding
//! with recovery. The shared data structures it produces
//! ([`hypermesh_lib::Shard`], [`hypermesh_lib::ShardMetadata`],
//! [`hypermesh_lib::ShardingStats`]) live in `hypermesh-lib` so consumers do
//! not have to depend on engauge for the types (which would create a cycle).
//!
//! Behavior is byte-identical to the previous blockmatrix implementation: the
//! last data shard stores the total original length so reconstruction can trim
//! padding exactly, and each shard carries a BLAKE3 integrity hash.

use hypermesh_lib::{Shard, ShardMetadata, ShardingError, ShardingStats};
use reed_solomon_erasure::galois_8::ReedSolomon;
use serde::{Deserialize, Serialize};

/// Sharding configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    /// Number of data shards
    pub data_shards: usize,
    /// Number of parity shards
    pub parity_shards: usize,
    /// Target shard size in bytes (before padding)
    pub target_shard_size: usize,
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            data_shards: 10,
            parity_shards: 4,
            target_shard_size: 1024 * 1024, // 1MB per shard
        }
    }
}

impl ShardingConfig {
    /// Create a [`ShardingConfig`] with adaptive RS parameters (R14).
    ///
    /// Delegates to [`hypermesh_lib::protocol::ErasureCodingParams::for_asset_size`]
    /// which selects RS(k, n-k) based on the asset size.
    pub fn adaptive_for_size(asset_bytes: u64) -> Self {
        let params = hypermesh_lib::protocol::ErasureCodingParams::for_asset_size(asset_bytes);
        Self {
            data_shards: params.data_shards as usize,
            parity_shards: params.parity_shards as usize,
            target_shard_size: 1024 * 1024,
        }
    }

    /// Total number of shards (data + parity)
    pub fn total_shards(&self) -> usize {
        self.data_shards + self.parity_shards
    }

    /// Minimum shards needed for reconstruction
    pub fn min_shards_for_reconstruction(&self) -> usize {
        self.data_shards
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ShardingError> {
        if self.data_shards == 0 {
            return Err(ShardingError::InvalidConfig(
                "data_shards must be > 0".to_string(),
            ));
        }
        if self.parity_shards == 0 {
            return Err(ShardingError::InvalidConfig(
                "parity_shards must be > 0".to_string(),
            ));
        }
        if self.target_shard_size == 0 {
            return Err(ShardingError::InvalidConfig(
                "target_shard_size must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Sharder for creating erasure-coded shards.
pub struct Sharder {
    config: ShardingConfig,
    reed_solomon: ReedSolomon,
}

impl Sharder {
    /// Create new sharder with configuration.
    pub fn new(config: ShardingConfig) -> Result<Self, ShardingError> {
        config.validate()?;

        let reed_solomon =
            ReedSolomon::new(config.data_shards, config.parity_shards).map_err(|e| {
                ShardingError::ShardingFailed(format!("Failed to create Reed-Solomon codec: {e}"))
            })?;

        Ok(Self {
            config,
            reed_solomon,
        })
    }

    /// Create sharder with default configuration.
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Result<Self, ShardingError> {
        Self::new(ShardingConfig::default())
    }

    /// Shard data into data + parity shards.
    pub fn shard(&self, data: &[u8]) -> Result<(Vec<Shard>, ShardingStats), ShardingError> {
        let start = std::time::Instant::now();

        // Calculate shard size (round up)
        let shard_size = data.len().div_ceil(self.config.data_shards);
        let padded_size = shard_size * self.config.data_shards;

        // Pad data to align with shard boundaries
        let mut padded_data = data.to_vec();
        let original_len = data.len();
        padded_data.resize(padded_size, 0);

        // Create Reed-Solomon shard buffers
        let mut rs_shards: Vec<Vec<u8>> = (0..self.config.data_shards)
            .map(|i| {
                let start_idx = i * shard_size;
                let end_idx = start_idx + shard_size;
                padded_data[start_idx..end_idx].to_vec()
            })
            .collect();

        // Add empty parity shard buffers
        for _ in 0..self.config.parity_shards {
            rs_shards.push(vec![0u8; shard_size]);
        }

        // Encode using Reed-Solomon to generate parity shards
        self.reed_solomon.encode(&mut rs_shards).map_err(|e| {
            ShardingError::ShardingFailed(format!("Reed-Solomon encoding failed: {e}"))
        })?;

        // Convert to Shard structs with metadata
        let mut shards = Vec::with_capacity(self.config.total_shards());

        for (i, shard_data) in rs_shards.into_iter().enumerate() {
            let is_parity = i >= self.config.data_shards;

            let metadata = ShardMetadata {
                index: i,
                is_parity,
                size: shard_data.len(),
                original_size: if !is_parity && i == self.config.data_shards - 1 {
                    // Last data shard stores the TOTAL original data length
                    // so reconstruct() can truncate padding exactly.
                    original_len
                } else {
                    shard_data.len()
                },
                hash: Shard::calculate_hash(&shard_data),
            };

            shards.push(Shard {
                data: shard_data,
                metadata,
            });
        }

        let total_size: usize = shards.iter().map(|s| s.data.len()).sum();
        let duration_ms = start.elapsed().as_millis() as u64;

        let stats = ShardingStats::calculate(
            original_len,
            total_size,
            self.config.data_shards,
            self.config.parity_shards,
            duration_ms,
        );

        Ok((shards, stats))
    }

    /// Reconstruct data from shards (requires at least data_shards available).
    pub fn reconstruct(&self, shards: &[Shard]) -> Result<Vec<u8>, ShardingError> {
        // Verify we have enough shards
        if shards.len() < self.config.data_shards {
            return Err(ShardingError::ShardingFailed(format!(
                "Not enough shards for reconstruction: have {}, need {}",
                shards.len(),
                self.config.data_shards
            )));
        }

        // Verify shard integrity
        for shard in shards {
            if !shard.verify() {
                return Err(ShardingError::ShardingFailed(format!(
                    "Shard {} failed integrity check",
                    shard.metadata.index
                )));
            }
        }

        // Create shard array with Options for missing shards
        let mut rs_shards: Vec<Option<Vec<u8>>> = vec![None; self.config.total_shards()];
        let mut original_size = 0usize;

        // Fill in available shards
        for shard in shards {
            rs_shards[shard.metadata.index] = Some(shard.data.clone());

            // Last data shard stores the total original data length
            if !shard.metadata.is_parity && shard.metadata.index == self.config.data_shards - 1 {
                original_size = shard.metadata.original_size;
            }
        }

        // If we don't have original size yet, calculate it
        if original_size == 0 {
            // Assume full size if we can't determine
            let shard_size = shards[0].data.len();
            original_size = shard_size * self.config.data_shards;
        }

        // Reconstruct missing shards using Reed-Solomon
        self.reed_solomon.reconstruct(&mut rs_shards).map_err(|e| {
            ShardingError::ShardingFailed(format!("Reed-Solomon reconstruction failed: {e}"))
        })?;

        // Combine data shards back into original data
        let mut reconstructed = Vec::with_capacity(original_size);

        for (i, rs_shard) in rs_shards.iter().enumerate().take(self.config.data_shards) {
            if let Some(shard_data) = rs_shard {
                reconstructed.extend_from_slice(shard_data);
            } else {
                return Err(ShardingError::ShardingFailed(format!(
                    "Failed to reconstruct data shard {i}"
                )));
            }
        }

        // Trim to original size (remove padding)
        reconstructed.truncate(original_size);

        Ok(reconstructed)
    }

    /// Get sharding configuration.
    pub fn config(&self) -> &ShardingConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharding_config() {
        let config = ShardingConfig::default();
        assert_eq!(config.total_shards(), 14); // 10 + 4
        assert_eq!(config.min_shards_for_reconstruction(), 10);
        config.validate().expect("test: validate config");
    }

    #[test]
    fn test_shard_and_reconstruct() {
        let sharder = Sharder::default().expect("test: create sharder");
        let data = b"Hello, World! This is test data for sharding.".repeat(100);

        let (shards, stats) = sharder.shard(&data).expect("test: shard data");
        assert_eq!(shards.len(), 14); // 10 data + 4 parity
        assert_eq!(stats.data_shards, 10);
        assert_eq!(stats.parity_shards, 4);

        // Verify all shards
        for shard in &shards {
            assert!(shard.verify());
        }

        // Test 1: Reconstruct from all shards
        let reconstructed = sharder
            .reconstruct(&shards)
            .expect("test: reconstruct from all shards");
        assert_eq!(reconstructed, data);

        // Test 2: Reconstruct from minimum data shards (no parity needed)
        let data_shards: Vec<_> = shards
            .iter()
            .filter(|s| !s.metadata.is_parity)
            .cloned()
            .collect();

        let reconstructed = sharder
            .reconstruct(&data_shards)
            .expect("test: reconstruct from data shards");
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_reed_solomon_recovery() {
        let sharder = Sharder::default().expect("test: create sharder");
        let data = b"Reed-Solomon recovery test data! ".repeat(500);

        let (shards, _) = sharder.shard(&data).expect("test: shard data");

        // Test recovering with 4 missing shards (using parity)
        // Keep first 10 shards (mix of data and parity)
        let partial_shards: Vec<_> = shards.iter().take(10).cloned().collect();

        let reconstructed = sharder
            .reconstruct(&partial_shards)
            .expect("test: reconstruct from partial");
        assert_eq!(reconstructed, data);

        // Test with different missing shard patterns
        // Missing shards 2, 5, 8, 11 (3 data, 1 parity)
        let partial_shards: Vec<_> = shards
            .iter()
            .enumerate()
            .filter(|(i, _)| !matches!(i, 2 | 5 | 8 | 11))
            .map(|(_, s)| s.clone())
            .collect();

        let reconstructed = sharder
            .reconstruct(&partial_shards)
            .expect("test: reconstruct from pattern");
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_maximum_shard_loss() {
        let config = ShardingConfig {
            data_shards: 10,
            parity_shards: 4,
            target_shard_size: 1024,
        };
        let sharder = Sharder::new(config).expect("test: create sharder with config");
        let data = vec![42u8; 10240]; // 10KB of data

        let (shards, _) = sharder.shard(&data).expect("test: shard data");

        // Test with exactly 4 shards missing (maximum allowed)
        // Drop last 4 shards (all parity shards)
        let partial_shards: Vec<_> = shards.iter().take(10).cloned().collect();

        let reconstructed = sharder
            .reconstruct(&partial_shards)
            .expect("test: reconstruct from max loss");
        assert_eq!(reconstructed, data);

        // Test that we can't recover with 5 shards missing
        let insufficient_shards: Vec<_> = shards.iter().take(9).cloned().collect();

        assert!(sharder.reconstruct(&insufficient_shards).is_err());
    }

    #[test]
    fn test_shard_integrity() {
        let sharder = Sharder::default().expect("test: create sharder");
        let data = vec![0u8; 10000];

        let (shards, _) = sharder.shard(&data).expect("test: shard data");

        // All shards should verify
        for shard in &shards {
            assert!(shard.verify());
        }

        // Corrupt a shard
        let mut corrupted = shards[0].clone();
        corrupted.data[0] ^= 1;
        assert!(!corrupted.verify());
    }

    #[test]
    fn test_insufficient_shards() {
        let sharder = Sharder::default().expect("test: create sharder");
        let data = b"Test data";

        let (shards, _) = sharder.shard(data).expect("test: shard data");

        // Try to reconstruct with too few shards
        let result = sharder.reconstruct(&shards[0..5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_sharding_stats() {
        let sharder = Sharder::default().expect("test: create sharder");
        let data = vec![0u8; 100000];

        let (_shards, stats) = sharder.shard(&data).expect("test: shard data");
        assert_eq!(stats.original_size, 100000);
        assert!(stats.total_shard_size > stats.original_size);
        assert!(stats.redundancy_factor > 1.0);
        assert!(stats.throughput_mbps > 0.0);
    }

    #[test]
    fn test_shard_reconstruct_small_data() {
        // Regression: when data.len() < data_shards, padding spans multiple
        // shards and the old per-shard original_size calculation via
        // saturating_sub produced the wrong total, corrupting reconstruction.
        let sharder = Sharder::default().expect("test: sharder"); // 10+4
        for size in [1, 3, 5, 7, 9, 10, 11, 15, 20, 25, 30, 35, 40, 50] {
            let data = vec![0xABu8; size];
            let (shards, _) = sharder.shard(&data).expect("test: shard");
            let reconstructed = sharder.reconstruct(&shards).expect("test: reconstruct");
            assert_eq!(
                reconstructed.len(),
                data.len(),
                "Size {} length mismatch: got {} expected {}",
                size,
                reconstructed.len(),
                data.len()
            );
            assert_eq!(reconstructed, data, "Size {size} data mismatch");
        }
    }

    #[test]
    fn test_adaptive_sharding_small_asset() {
        // < 1MB should get RS(4,2)
        let config = ShardingConfig::adaptive_for_size(500_000);
        assert_eq!(config.data_shards, 4);
        assert_eq!(config.parity_shards, 2);

        let sharder = Sharder::new(config).expect("test: adaptive sharder");
        let data = vec![0xABu8; 500];
        let (shards, stats) = sharder.shard(&data).expect("test: shard small");
        assert_eq!(shards.len(), 6);
        assert_eq!(stats.data_shards, 4);
        assert_eq!(stats.parity_shards, 2);

        let reconstructed = sharder.reconstruct(&shards).expect("test: reconstruct");
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_adaptive_sharding_medium_asset() {
        // 50MB should get RS(10,4)
        let config = ShardingConfig::adaptive_for_size(50_000_000);
        assert_eq!(config.data_shards, 10);
        assert_eq!(config.parity_shards, 4);
    }

    #[test]
    fn test_adaptive_sharding_large_asset() {
        // 500MB should get RS(20,8)
        let config = ShardingConfig::adaptive_for_size(500_000_000);
        assert_eq!(config.data_shards, 20);
        assert_eq!(config.parity_shards, 8);
    }

    #[test]
    fn test_custom_config() {
        let config = ShardingConfig {
            data_shards: 6,
            parity_shards: 2,
            target_shard_size: 1024,
        };

        let sharder = Sharder::new(config).expect("test: create sharder with config");
        let data = vec![1u8; 5000];

        let (shards, stats) = sharder.shard(&data).expect("test: shard data");
        assert_eq!(shards.len(), 8); // 6 + 2
        assert_eq!(stats.data_shards, 6);
        assert_eq!(stats.parity_shards, 2);
    }
}
