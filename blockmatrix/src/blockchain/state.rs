// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Chain state management for node-specific blockchains
//!
//! Manages the state of a node's independent blockchain including
//! storage, querying, and statistics.

use std::collections::{HashMap, BTreeMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use tracing::{info, debug};

use crate::matrix::coordinate::MatrixCoordinate;
use super::block::Block;

/// State snapshot of a blockchain at a specific point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSnapshot {
    /// Timestamp of the snapshot
    pub timestamp: DateTime<Utc>,
    /// Chain height at snapshot
    pub height: u64,
    /// Total blocks
    pub total_blocks: u64,
    /// Hash of the head block
    pub head_hash: String,
    /// Node coordinate
    pub node_coordinate: MatrixCoordinate,
    /// Chain metadata
    pub metadata: HashMap<String, String>,
}

/// Query options for retrieving blocks
#[derive(Debug, Clone)]
pub struct BlockQuery {
    /// Start index (inclusive)
    pub from_index: Option<u64>,
    /// End index (inclusive)
    pub to_index: Option<u64>,
    /// Start time (inclusive)
    pub from_time: Option<DateTime<Utc>>,
    /// End time (inclusive)
    pub to_time: Option<DateTime<Utc>>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Sort order
    pub sort: SortOrder,
}

/// Sort order for query results
#[derive(Debug, Clone)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl Default for BlockQuery {
    fn default() -> Self {
        BlockQuery {
            from_index: None,
            to_index: None,
            from_time: None,
            to_time: None,
            limit: None,
            sort: SortOrder::Ascending,
        }
    }
}

/// Manages the persistent state of a node's blockchain
pub struct ChainStateManager {
    /// Node coordinate
    node_coordinate: MatrixCoordinate,
    /// Storage directory path
    storage_path: PathBuf,
    /// In-memory block cache (most recent blocks)
    block_cache: Arc<RwLock<BTreeMap<u64, Block>>>,
    /// Cache size limit
    cache_size: usize,
    /// Chain snapshots
    snapshots: Arc<RwLock<Vec<ChainSnapshot>>>,
    /// Maximum snapshots to keep
    max_snapshots: usize,
    /// Metadata storage
    metadata: Arc<RwLock<HashMap<String, String>>>,
}

impl ChainStateManager {
    /// Create a new state manager
    pub fn new(
        node_coordinate: MatrixCoordinate,
        storage_path: impl AsRef<Path>,
    ) -> Self {
        let storage_path = storage_path.as_ref().to_path_buf();

        ChainStateManager {
            node_coordinate,
            storage_path,
            block_cache: Arc::new(RwLock::new(BTreeMap::new())),
            cache_size: 1000, // Keep last 1000 blocks in memory
            snapshots: Arc::new(RwLock::new(Vec::new())),
            max_snapshots: 10,
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize storage directory
    pub async fn initialize(&self) -> Result<(), String> {
        // Create storage directory if it doesn't exist
        fs::create_dir_all(&self.storage_path).await
            .map_err(|e| format!("Failed to create storage directory: {}", e))?;

        // Create subdirectories
        let blocks_dir = self.storage_path.join("blocks");
        let snapshots_dir = self.storage_path.join("snapshots");

        fs::create_dir_all(&blocks_dir).await
            .map_err(|e| format!("Failed to create blocks directory: {}", e))?;
        fs::create_dir_all(&snapshots_dir).await
            .map_err(|e| format!("Failed to create snapshots directory: {}", e))?;

        info!(
            "Initialized chain state storage at {:?} for node ({},{},{})",
            self.storage_path,
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z
        );

        Ok(())
    }

    /// Store a block persistently
    pub async fn store_block(&self, block: &Block) -> Result<(), String> {
        // Add to cache
        self.update_cache(block).await;

        // Write to disk
        let block_path = self.get_block_path(block.index);
        let json = serde_json::to_string_pretty(block)
            .map_err(|e| format!("Failed to serialize block: {}", e))?;

        fs::write(&block_path, json).await
            .map_err(|e| format!("Failed to write block to disk: {}", e))?;

        debug!("Stored block {} to disk", block.index);
        Ok(())
    }

    /// Load a block from storage
    pub async fn load_block(&self, index: u64) -> Result<Block, String> {
        // Check cache first
        if let Some(block) = self.get_from_cache(index).await {
            return Ok(block);
        }

        // Load from disk
        let block_path = self.get_block_path(index);

        if !block_path.exists() {
            return Err(format!("Block {} not found", index));
        }

        let json = fs::read_to_string(&block_path).await
            .map_err(|e| format!("Failed to read block from disk: {}", e))?;

        let block: Block = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize block: {}", e))?;

        // Update cache
        self.update_cache(&block).await;

        Ok(block)
    }

    /// Query blocks based on criteria
    pub async fn query_blocks(&self, query: BlockQuery) -> Result<Vec<Block>, String> {
        let mut results = Vec::new();

        // Determine index range
        let start_index = query.from_index.unwrap_or(0);

        // If no end_index specified and no limit, scan from cache to determine max index
        let max_index = if query.to_index.is_none() && query.limit.is_none() {
            let cache = self.block_cache.read().await;
            cache.keys().max().copied().unwrap_or(start_index + 1000)
        } else {
            query.to_index.unwrap_or(start_index + 1000)
        };
        let end_index = max_index;

        // Load blocks from cache and disk
        let cache = self.block_cache.read().await;

        for index in start_index..=end_index {
            // Try cache first
            let block = if let Some(cached) = cache.get(&index) {
                cached.clone()
            } else if self.get_block_path(index).exists() {
                // Load from disk if not in cache
                match self.load_block(index).await {
                    Ok(b) => b,
                    Err(_) => continue,
                }
            } else {
                continue;
            };

            // Apply time filters
            if let Some(from_time) = query.from_time {
                if block.timestamp < from_time {
                    continue;
                }
            }
            if let Some(to_time) = query.to_time {
                if block.timestamp > to_time {
                    continue;
                }
            }

            results.push(block);

            // Apply limit
            if let Some(limit) = query.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }

        // Sort results
        match query.sort {
            SortOrder::Ascending => results.sort_by_key(|b| b.index),
            SortOrder::Descending => results.sort_by_key(|b| std::cmp::Reverse(b.index)),
        }

        Ok(results)
    }

    /// Create a snapshot of the current chain state
    pub async fn create_snapshot(
        &self,
        height: u64,
        head_hash: String,
    ) -> Result<ChainSnapshot, String> {
        let snapshot = ChainSnapshot {
            timestamp: Utc::now(),
            height,
            total_blocks: height + 1,
            head_hash,
            node_coordinate: self.node_coordinate.clone(),
            metadata: self.metadata.read().await.clone(),
        };

        // Store snapshot
        self.store_snapshot(&snapshot).await?;

        Ok(snapshot)
    }

    /// Store a snapshot
    async fn store_snapshot(&self, snapshot: &ChainSnapshot) -> Result<(), String> {
        let mut snapshots = self.snapshots.write().await;

        // Add new snapshot
        snapshots.push(snapshot.clone());

        // Trim old snapshots
        if snapshots.len() > self.max_snapshots {
            let drain_count = snapshots.len() - self.max_snapshots;
            snapshots.drain(0..drain_count);
        }

        // Write to disk
        let snapshot_path = self.storage_path
            .join("snapshots")
            .join(format!("snapshot_{}.json", snapshot.timestamp.timestamp()));

        let json = serde_json::to_string_pretty(snapshot)
            .map_err(|e| format!("Failed to serialize snapshot: {}", e))?;

        fs::write(&snapshot_path, json).await
            .map_err(|e| format!("Failed to write snapshot to disk: {}", e))?;

        info!("Created snapshot at height {}", snapshot.height);
        Ok(())
    }

    /// Get the latest snapshot
    pub async fn get_latest_snapshot(&self) -> Option<ChainSnapshot> {
        self.snapshots.read().await.last().cloned()
    }

    /// Get all snapshots
    pub async fn get_snapshots(&self) -> Vec<ChainSnapshot> {
        self.snapshots.read().await.clone()
    }

    /// Update block cache
    async fn update_cache(&self, block: &Block) {
        let mut cache = self.block_cache.write().await;

        cache.insert(block.index, block.clone());

        // Evict old blocks if cache is too large
        while cache.len() > self.cache_size {
            if let Some((&oldest_index, _)) = cache.iter().next() {
                cache.remove(&oldest_index);
            }
        }
    }

    /// Get block from cache
    async fn get_from_cache(&self, index: u64) -> Option<Block> {
        self.block_cache.read().await.get(&index).cloned()
    }

    /// Get block file path
    fn get_block_path(&self, index: u64) -> PathBuf {
        self.storage_path
            .join("blocks")
            .join(format!("block_{:010}.json", index))
    }

    /// Calculate storage statistics
    pub async fn get_storage_stats(&self) -> Result<StorageStats, String> {
        let blocks_dir = self.storage_path.join("blocks");
        let mut total_size = 0u64;
        let mut block_count = 0u64;

        // Count blocks and calculate size
        let mut entries = fs::read_dir(&blocks_dir).await
            .map_err(|e| format!("Failed to read blocks directory: {}", e))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| format!("Failed to read directory entry: {}", e))? {
            if let Ok(metadata) = entry.metadata().await {
                total_size += metadata.len();
                block_count += 1;
            }
        }

        let cache_size = self.block_cache.read().await.len();

        Ok(StorageStats {
            total_blocks: block_count,
            total_size_bytes: total_size,
            cached_blocks: cache_size,
            snapshots_count: self.snapshots.read().await.len(),
        })
    }

    /// Set metadata value
    pub async fn set_metadata(&self, key: String, value: String) {
        self.metadata.write().await.insert(key, value);
    }

    /// Get metadata value
    pub async fn get_metadata(&self, key: &str) -> Option<String> {
        self.metadata.read().await.get(key).cloned()
    }

    /// Clear all metadata
    pub async fn clear_metadata(&self) {
        self.metadata.write().await.clear();
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Total blocks stored on disk
    pub total_blocks: u64,
    /// Total storage size in bytes
    pub total_size_bytes: u64,
    /// Number of blocks in cache
    pub cached_blocks: usize,
    /// Number of snapshots
    pub snapshots_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::test_utils::test_asset_ids;

    async fn create_test_manager() -> (ChainStateManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let coord = MatrixCoordinate::new(1, 2, 3).unwrap();
        let manager = ChainStateManager::new(coord, temp_dir.path());
        manager.initialize().await.unwrap();
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_state_manager_initialization() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Verify directories were created
        assert!(manager.storage_path.join("blocks").exists());
        assert!(manager.storage_path.join("snapshots").exists());
    }

    #[tokio::test]
    async fn test_block_storage_and_retrieval() {
        let (manager, _temp_dir) = create_test_manager().await;

        let coord = MatrixCoordinate::new(1, 2, 3).unwrap();
        let block = Block::genesis(coord);

        // Store block
        manager.store_block(&block).await.unwrap();

        // Retrieve block
        let loaded = manager.load_block(0).await.unwrap();
        assert_eq!(loaded, block);
    }

    #[tokio::test]
    async fn test_cache_management() {
        let (mut manager, _temp_dir) = create_test_manager().await;
        manager.cache_size = 3; // Small cache for testing

        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();

        // Add blocks beyond cache size
        for i in 0..5 {
            let block = Block::new(
                i,
                test_asset_ids(1),
                format!("prev_{}", i),
                coord.clone(),
            );
            manager.store_block(&block).await.unwrap();
        }

        // Check cache size
        let cache = manager.block_cache.read().await;
        assert!(cache.len() <= 3);
    }

    #[tokio::test]
    async fn test_block_queries() {
        let (manager, _temp_dir) = create_test_manager().await;

        let coord = MatrixCoordinate::new(5, 5, 5).unwrap();

        // Add some blocks
        for i in 0..10 {
            let block = Block::new(
                i,
                test_asset_ids(1),
                format!("prev_{}", i),
                coord.clone(),
            );
            manager.store_block(&block).await.unwrap();
        }

        // Query range
        let query = BlockQuery {
            from_index: Some(3),
            to_index: Some(7),
            ..Default::default()
        };

        let results = manager.query_blocks(query).await.unwrap();
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].index, 3);
        assert_eq!(results[4].index, 7);
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let (manager, _temp_dir) = create_test_manager().await;

        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();

        // Add blocks
        for i in 0..10 {
            let block = Block::new(i, test_asset_ids(1), format!("prev_{}", i), coord.clone());
            manager.store_block(&block).await.unwrap();
        }

        // Query with limit
        let query = BlockQuery {
            limit: Some(3),
            ..Default::default()
        };

        let results = manager.query_blocks(query).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_snapshots() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create snapshot
        let snapshot = manager.create_snapshot(
            10,
            "head_hash_123".to_string(),
        ).await.unwrap();

        assert_eq!(snapshot.height, 10);
        assert_eq!(snapshot.total_blocks, 11);
        assert_eq!(snapshot.head_hash, "head_hash_123");

        // Get latest snapshot
        let latest = manager.get_latest_snapshot().await.unwrap();
        assert_eq!(latest.height, snapshot.height);
    }

    #[tokio::test]
    async fn test_metadata() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Set metadata
        manager.set_metadata("key1".to_string(), "value1".to_string()).await;
        manager.set_metadata("key2".to_string(), "value2".to_string()).await;

        // Get metadata
        assert_eq!(manager.get_metadata("key1").await, Some("value1".to_string()));
        assert_eq!(manager.get_metadata("key2").await, Some("value2".to_string()));
        assert_eq!(manager.get_metadata("nonexistent").await, None);

        // Clear metadata
        manager.clear_metadata().await;
        assert_eq!(manager.get_metadata("key1").await, None);
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let (manager, _temp_dir) = create_test_manager().await;

        let coord = MatrixCoordinate::new(1, 1, 1).unwrap();

        // Add some blocks
        for i in 0..5 {
            let block = Block::new(i, test_asset_ids(1), format!("prev_{}", i), coord.clone());
            manager.store_block(&block).await.unwrap();
        }

        // Get stats
        let stats = manager.get_storage_stats().await.unwrap();
        assert_eq!(stats.total_blocks, 5);
        assert!(stats.total_size_bytes > 0);
        assert!(stats.cached_blocks <= 5);
    }

    #[tokio::test]
    async fn test_descending_sort() {
        let (manager, _temp_dir) = create_test_manager().await;

        let coord = MatrixCoordinate::new(2, 2, 2).unwrap();

        // Add blocks
        for i in 0..5 {
            let block = Block::new(i, test_asset_ids(1), format!("prev_{}", i), coord.clone());
            manager.store_block(&block).await.unwrap();
        }

        // Query with descending sort
        let query = BlockQuery {
            sort: SortOrder::Descending,
            ..Default::default()
        };

        let results = manager.query_blocks(query).await.unwrap();
        assert!(results.len() >= 5);
        assert!(results[0].index > results[1].index);
    }
}