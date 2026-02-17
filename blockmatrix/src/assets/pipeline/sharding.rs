// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sharding Stage - Reed-Solomon erasure coding
//!
//! Provides configurable redundancy with erasure coding (e.g., 10+4).
//!
//! This implementation uses the reed-solomon-erasure crate for production-quality
//! erasure coding with recovery capabilities.

use crate::assets::pipeline::{PipelineError, PipelineResult};
use serde::{Serialize, Deserialize};
use reed_solomon_erasure::galois_8::ReedSolomon;

/// Sharding configuration
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
    /// Total number of shards (data + parity)
    pub fn total_shards(&self) -> usize {
        self.data_shards + self.parity_shards
    }

    /// Minimum shards needed for reconstruction
    pub fn min_shards_for_reconstruction(&self) -> usize {
        self.data_shards
    }

    /// Validate configuration
    pub fn validate(&self) -> PipelineResult<()> {
        if self.data_shards == 0 {
            return Err(PipelineError::InvalidConfig("data_shards must be > 0".to_string()));
        }
        if self.parity_shards == 0 {
            return Err(PipelineError::InvalidConfig("parity_shards must be > 0".to_string()));
        }
        if self.target_shard_size == 0 {
            return Err(PipelineError::InvalidConfig("target_shard_size must be > 0".to_string()));
        }
        Ok(())
    }
}

/// Shard metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for ShardMetadata {
    fn default() -> Self {
        Self {
            index: 0,
            is_parity: false,
            size: 0,
            original_size: 0,
            hash: String::new(),
        }
    }
}

/// Individual shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shard {
    /// Shard data
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    /// Shard metadata
    pub metadata: ShardMetadata,
}

impl Shard {
    /// Calculate hash of shard data
    fn calculate_hash(data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Verify shard integrity
    pub fn verify(&self) -> bool {
        Self::calculate_hash(&self.data) == self.metadata.hash
    }
}

/// Sharding statistics
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
    fn calculate(
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

/// Sharder for creating erasure-coded shards
pub struct Sharder {
    config: ShardingConfig,
    reed_solomon: ReedSolomon,
}

impl Sharder {
    /// Create new sharder with configuration
    pub fn new(config: ShardingConfig) -> PipelineResult<Self> {
        config.validate()?;

        let reed_solomon = ReedSolomon::new(config.data_shards, config.parity_shards)
            .map_err(|e| PipelineError::ShardingFailed(format!("Failed to create Reed-Solomon codec: {}", e)))?;

        Ok(Self { config, reed_solomon })
    }

    /// Create sharder with default configuration
    pub fn default() -> PipelineResult<Self> {
        Self::new(ShardingConfig::default())
    }

    /// Shard data into data + parity shards
    pub fn shard(&self, data: &[u8]) -> PipelineResult<(Vec<Shard>, ShardingStats)> {
        let start = std::time::Instant::now();

        // Calculate shard size (round up)
        let shard_size = (data.len() + self.config.data_shards - 1) / self.config.data_shards;
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
        self.reed_solomon.encode(&mut rs_shards)
            .map_err(|e| PipelineError::ShardingFailed(format!("Reed-Solomon encoding failed: {}", e)))?;

        // Convert to Shard structs with metadata
        let mut shards = Vec::with_capacity(self.config.total_shards());

        for (i, shard_data) in rs_shards.into_iter().enumerate() {
            let is_parity = i >= self.config.data_shards;

            let metadata = ShardMetadata {
                index: i,
                is_parity,
                size: shard_data.len(),
                original_size: if !is_parity && i == self.config.data_shards - 1 {
                    // Last data shard - calculate original unpadded size
                    let padding = padded_size.saturating_sub(original_len);
                    shard_data.len().saturating_sub(padding)
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

    /// Reconstruct data from shards (requires at least data_shards available)
    pub fn reconstruct(&self, shards: &[Shard]) -> PipelineResult<Vec<u8>> {
        // Verify we have enough shards
        if shards.len() < self.config.data_shards {
            return Err(PipelineError::ShardingFailed(
                format!("Not enough shards for reconstruction: have {}, need {}",
                    shards.len(), self.config.data_shards)
            ));
        }

        // Verify shard integrity
        for shard in shards {
            if !shard.verify() {
                return Err(PipelineError::ShardingFailed(
                    format!("Shard {} failed integrity check", shard.metadata.index)
                ));
            }
        }

        // Create shard array with Options for missing shards
        let mut rs_shards: Vec<Option<Vec<u8>>> = vec![None; self.config.total_shards()];
        let mut original_size = 0usize;

        // Fill in available shards
        for shard in shards {
            rs_shards[shard.metadata.index] = Some(shard.data.clone());

            // Track original size from last data shard
            if !shard.metadata.is_parity && shard.metadata.index == self.config.data_shards - 1 {
                // Calculate total original size from last shard's original_size
                let shard_size = shard.data.len();
                let full_shards_size = shard_size * (self.config.data_shards - 1);
                original_size = full_shards_size + shard.metadata.original_size;
            }
        }

        // If we don't have original size yet, calculate it
        if original_size == 0 {
            // Assume full size if we can't determine
            let shard_size = shards[0].data.len();
            original_size = shard_size * self.config.data_shards;
        }

        // Reconstruct missing shards using Reed-Solomon
        self.reed_solomon.reconstruct(&mut rs_shards)
            .map_err(|e| PipelineError::ShardingFailed(format!("Reed-Solomon reconstruction failed: {}", e)))?;

        // Combine data shards back into original data
        let mut reconstructed = Vec::with_capacity(original_size);

        for i in 0..self.config.data_shards {
            if let Some(shard_data) = &rs_shards[i] {
                reconstructed.extend_from_slice(shard_data);
            } else {
                return Err(PipelineError::ShardingFailed(
                    format!("Failed to reconstruct data shard {}", i)
                ));
            }
        }

        // Trim to original size (remove padding)
        reconstructed.truncate(original_size);

        Ok(reconstructed)
    }


    /// Get sharding configuration
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
        config.validate().unwrap();
    }

    #[test]
    fn test_shard_and_reconstruct() {
        let sharder = Sharder::default().unwrap();
        let data = b"Hello, World! This is test data for sharding.".repeat(100);

        let (shards, stats) = sharder.shard(&data).unwrap();
        assert_eq!(shards.len(), 14); // 10 data + 4 parity
        assert_eq!(stats.data_shards, 10);
        assert_eq!(stats.parity_shards, 4);

        // Verify all shards
        for shard in &shards {
            assert!(shard.verify());
        }

        // Test 1: Reconstruct from all shards
        let reconstructed = sharder.reconstruct(&shards).unwrap();
        assert_eq!(reconstructed, data);

        // Test 2: Reconstruct from minimum data shards (no parity needed)
        let data_shards: Vec<_> = shards.iter()
            .filter(|s| !s.metadata.is_parity)
            .cloned()
            .collect();

        let reconstructed = sharder.reconstruct(&data_shards).unwrap();
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_reed_solomon_recovery() {
        let sharder = Sharder::default().unwrap();
        let data = b"Reed-Solomon recovery test data! ".repeat(500);

        let (shards, _) = sharder.shard(&data).unwrap();

        // Test recovering with 4 missing shards (using parity)
        // Keep first 10 shards (mix of data and parity)
        let partial_shards: Vec<_> = shards.iter()
            .take(10)
            .cloned()
            .collect();

        let reconstructed = sharder.reconstruct(&partial_shards).unwrap();
        assert_eq!(reconstructed, data);

        // Test with different missing shard patterns
        // Missing shards 2, 5, 8, 11 (3 data, 1 parity)
        let partial_shards: Vec<_> = shards.iter()
            .enumerate()
            .filter(|(i, _)| !matches!(i, 2 | 5 | 8 | 11))
            .map(|(_, s)| s.clone())
            .collect();

        let reconstructed = sharder.reconstruct(&partial_shards).unwrap();
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_maximum_shard_loss() {
        let config = ShardingConfig {
            data_shards: 10,
            parity_shards: 4,
            target_shard_size: 1024,
        };
        let sharder = Sharder::new(config).unwrap();
        let data = vec![42u8; 10240]; // 10KB of data

        let (shards, _) = sharder.shard(&data).unwrap();

        // Test with exactly 4 shards missing (maximum allowed)
        // Drop last 4 shards (all parity shards)
        let partial_shards: Vec<_> = shards.iter()
            .take(10)
            .cloned()
            .collect();

        let reconstructed = sharder.reconstruct(&partial_shards).unwrap();
        assert_eq!(reconstructed, data);

        // Test that we can't recover with 5 shards missing
        let insufficient_shards: Vec<_> = shards.iter()
            .take(9)
            .cloned()
            .collect();

        assert!(sharder.reconstruct(&insufficient_shards).is_err());
    }

    #[test]
    fn test_shard_integrity() {
        let sharder = Sharder::default().unwrap();
        let data = vec![0u8; 10000];

        let (shards, _) = sharder.shard(&data).unwrap();

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
        let sharder = Sharder::default().unwrap();
        let data = b"Test data";

        let (shards, _) = sharder.shard(data).unwrap();

        // Try to reconstruct with too few shards
        let result = sharder.reconstruct(&shards[0..5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_sharding_stats() {
        let sharder = Sharder::default().unwrap();
        let data = vec![0u8; 100000];

        let (shards, stats) = sharder.shard(&data).unwrap();
        assert_eq!(stats.original_size, 100000);
        assert!(stats.total_shard_size > stats.original_size);
        assert!(stats.redundancy_factor > 1.0);
        assert!(stats.throughput_mbps > 0.0);
    }

    #[test]
    fn test_custom_config() {
        let config = ShardingConfig {
            data_shards: 6,
            parity_shards: 2,
            target_shard_size: 1024,
        };

        let sharder = Sharder::new(config).unwrap();
        let data = vec![1u8; 5000];

        let (shards, stats) = sharder.shard(&data).unwrap();
        assert_eq!(shards.len(), 8); // 6 + 2
        assert_eq!(stats.data_shards, 6);
        assert_eq!(stats.parity_shards, 2);
    }
}
