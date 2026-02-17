// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Hash Bucket System
//!
//! Manages hash buckets for content-addressed storage with O(1) lookups.
//! Each bucket represents a range of hash values (256 total buckets).

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::matrix::MatrixCoordinate;
use super::Hash;

/// Bucket ID (00 to ff)
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct BucketId(String);

impl BucketId {
    /// Create bucket ID from hash (first byte)
    pub fn from_hash(hash: &Hash) -> Self {
        Self(format!("{:02x}", hash[0]))
    }

    /// Create bucket ID from string
    pub fn from_str(id: &str) -> Result<Self, String> {
        if id.len() != 2 {
            return Err("Bucket ID must be exactly 2 hex characters".to_string());
        }

        // Verify valid hex
        if !id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Bucket ID must be valid hex".to_string());
        }

        Ok(Self(id.to_lowercase()))
    }

    /// Get all possible bucket IDs (00 to ff)
    pub fn all_buckets() -> Vec<Self> {
        (0u8..=255).map(|i| Self(format!("{:02x}", i))).collect()
    }

    /// Get bucket ID as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Metadata for a stored shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMetadata {
    /// Matrix positions where shard is stored
    pub positions: Vec<MatrixCoordinate>,

    /// Number of times this shard is referenced
    pub reference_count: usize,

    /// Timestamp when first stored
    pub created_at: i64,

    /// Last access timestamp
    pub last_accessed: i64,

    /// Size of the shard in bytes
    pub size: usize,

    /// Popularity score (for replication decisions)
    pub popularity_score: f64,
}

impl ShardMetadata {
    /// Create new shard metadata
    pub fn new(positions: Vec<MatrixCoordinate>, size: usize) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            positions,
            reference_count: 1,
            created_at: now,
            last_accessed: now,
            size,
            popularity_score: 0.0,
        }
    }

    /// Update access time and popularity
    pub fn accessed(&mut self) {
        let now = chrono::Utc::now().timestamp();
        let time_since_creation = (now - self.created_at) as f64;

        // Calculate popularity based on access frequency
        // More accesses in shorter time = higher popularity
        if time_since_creation > 0.0 {
            self.popularity_score = self.reference_count as f64 / time_since_creation.sqrt();
        }

        self.last_accessed = now;
    }

    /// Add reference to this shard
    pub fn add_reference(&mut self) {
        self.reference_count += 1;
        self.accessed();
    }

    /// Add replica positions
    pub fn add_positions(&mut self, positions: Vec<MatrixCoordinate>) {
        for pos in positions {
            if !self.positions.contains(&pos) {
                self.positions.push(pos);
            }
        }
    }

    /// Check if shard should be replicated based on popularity
    pub fn needs_replication(&self, threshold: f64) -> bool {
        self.popularity_score > threshold
    }
}

/// Hash bucket containing shards with similar hash prefixes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashBucket {
    /// Bucket identifier (00 to ff)
    pub bucket_id: BucketId,

    /// HashMap of shard hash -> metadata for O(1) lookups
    pub shard_hashes: HashMap<Hash, ShardMetadata>,

    /// Number of deduplicated occurrences
    pub dedup_count: usize,

    /// Total size of unique shards in this bucket
    pub total_size: usize,

    /// Space saved through deduplication
    pub space_saved: usize,

    /// Bucket creation timestamp
    pub created_at: i64,

    /// Last modification timestamp
    pub updated_at: i64,
}

impl HashBucket {
    /// Create new hash bucket
    pub fn new(bucket_id: BucketId) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            bucket_id,
            shard_hashes: HashMap::new(),
            dedup_count: 0,
            total_size: 0,
            space_saved: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if shard exists (O(1) lookup)
    pub fn contains(&self, hash: &Hash) -> bool {
        self.shard_hashes.contains_key(hash)
    }

    /// Get shard metadata if exists
    pub fn get_metadata(&mut self, hash: &Hash) -> Option<&mut ShardMetadata> {
        if let Some(metadata) = self.shard_hashes.get_mut(hash) {
            metadata.accessed();
            self.updated_at = chrono::Utc::now().timestamp();
            Some(metadata)
        } else {
            None
        }
    }

    /// Add new shard to bucket
    pub fn add_shard(&mut self, hash: Hash, positions: Vec<MatrixCoordinate>, size: usize) -> ShardMetadata {
        let metadata = ShardMetadata::new(positions, size);
        self.shard_hashes.insert(hash, metadata.clone());
        self.total_size += size;
        self.updated_at = chrono::Utc::now().timestamp();
        metadata
    }

    /// Record deduplication
    pub fn record_deduplication(&mut self, hash: &Hash, size: usize) -> Option<&mut ShardMetadata> {
        if let Some(metadata) = self.shard_hashes.get_mut(hash) {
            metadata.add_reference();
            self.dedup_count += 1;
            self.space_saved += size;
            self.updated_at = chrono::Utc::now().timestamp();
            Some(metadata)
        } else {
            None
        }
    }

    /// Get bucket statistics
    pub fn get_stats(&self) -> BucketStats {
        BucketStats {
            bucket_id: self.bucket_id.clone(),
            unique_shards: self.shard_hashes.len(),
            dedup_count: self.dedup_count,
            total_size: self.total_size,
            space_saved: self.space_saved,
            deduplication_rate: if self.dedup_count > 0 {
                self.dedup_count as f64 / (self.shard_hashes.len() + self.dedup_count) as f64
            } else {
                0.0
            },
        }
    }

    /// Get shards that need replication
    pub fn get_popular_shards(&self, threshold: f64) -> Vec<(Hash, ShardMetadata)> {
        self.shard_hashes
            .iter()
            .filter(|(_, metadata)| metadata.needs_replication(threshold))
            .map(|(hash, metadata)| (*hash, metadata.clone()))
            .collect()
    }
}

/// Bucket statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketStats {
    pub bucket_id: BucketId,
    pub unique_shards: usize,
    pub dedup_count: usize,
    pub total_size: usize,
    pub space_saved: usize,
    pub deduplication_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_id_creation() {
        // Test from hash
        let hash: Hash = [0xab; 32];
        let bucket_id = BucketId::from_hash(&hash);
        assert_eq!(bucket_id.as_str(), "ab");

        // Test from string
        let bucket_id = BucketId::from_str("ff").unwrap();
        assert_eq!(bucket_id.as_str(), "ff");

        // Test invalid
        assert!(BucketId::from_str("g1").is_err());
        assert!(BucketId::from_str("abc").is_err());
    }

    #[test]
    fn test_all_buckets() {
        let buckets = BucketId::all_buckets();
        assert_eq!(buckets.len(), 256);
        assert_eq!(buckets[0].as_str(), "00");
        assert_eq!(buckets[255].as_str(), "ff");
    }

    #[test]
    fn test_shard_metadata_popularity() {
        let positions = vec![MatrixCoordinate::new(1, 2, 3).unwrap()];
        let mut metadata = ShardMetadata::new(positions, 1024);

        // Add references
        for _ in 0..10 {
            metadata.add_reference();
        }

        assert_eq!(metadata.reference_count, 11); // 1 initial + 10 added
        assert!(metadata.popularity_score > 0.0);
    }

    #[test]
    fn test_hash_bucket_operations() {
        let bucket_id = BucketId::from_str("ab").unwrap();
        let mut bucket = HashBucket::new(bucket_id);

        let hash: Hash = [0xab; 32];
        let positions = vec![MatrixCoordinate::new(1, 2, 3).unwrap()];

        // Add shard
        bucket.add_shard(hash, positions.clone(), 1024);
        assert!(bucket.contains(&hash));
        assert_eq!(bucket.total_size, 1024);

        // Deduplicate
        bucket.record_deduplication(&hash, 1024);
        assert_eq!(bucket.dedup_count, 1);
        assert_eq!(bucket.space_saved, 1024);

        // Check stats
        let stats = bucket.get_stats();
        assert_eq!(stats.unique_shards, 1);
        assert_eq!(stats.dedup_count, 1);
        assert_eq!(stats.deduplication_rate, 0.5); // 1 dedup / (1 unique + 1 dedup)
    }
}