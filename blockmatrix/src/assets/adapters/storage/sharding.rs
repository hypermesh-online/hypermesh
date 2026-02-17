// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sharding management, deduplication, and bucket logic
//!
//! Handles content-aware sharding, bucket deduplication, and shard metadata

use serde::{Deserialize, Serialize};

/// Sharding configuration for distributed storage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShardingConfig {
    /// Shard size in bytes
    pub shard_size_bytes: u64,
    /// Number of shards
    pub shard_count: u32,
    /// Sharding algorithm
    pub algorithm: ShardingAlgorithm,
    /// Content-aware sharding enabled
    pub content_aware: bool,
    /// Deduplication enabled
    pub deduplication_enabled: bool,
    /// Compression enabled
    pub compression_enabled: bool,
}

/// Sharding algorithms
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShardingAlgorithm {
    /// Round-robin distribution
    RoundRobin,
    /// Hash-based distribution
    HashBased,
    /// Content-aware distribution
    ContentAware,
    /// RAID-like striping
    Striping,
}

impl ShardingConfig {
    /// Configure sharding for storage allocation
    pub fn configure(size_bytes: u64, device_count: u32) -> Self {
        // Calculate optimal shard size (aim for 64MB shards)
        let target_shard_size = 64 * 1024 * 1024; // 64MB
        let shard_count = (size_bytes / target_shard_size).max(1) as u32;
        let actual_shard_size = size_bytes / shard_count as u64;

        Self {
            shard_size_bytes: actual_shard_size,
            shard_count,
            algorithm: if device_count > 1 {
                ShardingAlgorithm::Striping
            } else {
                ShardingAlgorithm::ContentAware
            },
            content_aware: true,
            deduplication_enabled: true,
            compression_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharding_config() {
        let size_bytes = 500 * 1024 * 1024; // 500MB
        let config = ShardingConfig::configure(size_bytes, 3);

        assert!(config.shard_count > 0);
        assert!(config.content_aware);
        assert_eq!(config.algorithm, ShardingAlgorithm::Striping);
    }

    #[test]
    fn test_single_device_sharding() {
        let size_bytes = 100 * 1024 * 1024; // 100MB
        let config = ShardingConfig::configure(size_bytes, 1);

        assert_eq!(config.algorithm, ShardingAlgorithm::ContentAware);
    }
}
