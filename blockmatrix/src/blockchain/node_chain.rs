//! Node-specific blockchain implementation
//!
//! Revolutionary architecture: Each node maintains its own independent blockchain.
//! NO merkle tree consolidation, NO shared chain across nodes.
//! Complete node sovereignty over its own ledger.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use tracing::{info, warn, error, debug};

use crate::matrix::coordinate::MatrixCoordinate;
use super::block::Block;
use super::validation::ChainValidator;

/// Statistics about a node's blockchain
#[derive(Debug, Clone, Default)]
pub struct ChainStats {
    /// Total number of blocks
    pub total_blocks: u64,
    /// Current chain height (latest block index)
    pub chain_height: u64,
    /// Chain creation time
    pub chain_start: Option<DateTime<Utc>>,
    /// Average time between blocks
    pub avg_block_time_ms: f64,
    /// Total data size in bytes
    pub total_data_size: usize,
}

/// A node's independent blockchain
///
/// Each BlockMatrix node maintains its own sovereign blockchain.
/// This is NOT a shared chain - each node has complete autonomy.
pub struct NodeBlockchain {
    /// The node's position in the matrix
    node_coordinate: MatrixCoordinate,

    /// The chain of blocks (index -> block)
    blocks: Arc<RwLock<HashMap<u64, Block>>>,

    /// Quick lookup: hash -> block index
    hash_index: Arc<RwLock<HashMap<String, u64>>>,

    /// Current chain head (latest block)
    head: Arc<RwLock<Option<Block>>>,

    /// Chain validator
    validator: ChainValidator,

    /// Chain statistics
    stats: Arc<RwLock<ChainStats>>,
}

impl NodeBlockchain {
    /// Create a new blockchain for a node
    pub fn new(node_coordinate: MatrixCoordinate) -> Self {
        let genesis = Block::genesis(node_coordinate.clone());
        let mut blocks = HashMap::new();
        let mut hash_index = HashMap::new();

        hash_index.insert(genesis.hash.clone(), genesis.index);
        blocks.insert(genesis.index, genesis.clone());

        let mut stats = ChainStats::default();
        stats.total_blocks = 1;
        stats.chain_height = 0;
        stats.chain_start = Some(genesis.timestamp);
        stats.total_data_size = genesis.size();

        info!(
            "Created new blockchain for node at ({}, {}, {})",
            node_coordinate.x, node_coordinate.y, node_coordinate.z
        );

        NodeBlockchain {
            node_coordinate,
            blocks: Arc::new(RwLock::new(blocks)),
            hash_index: Arc::new(RwLock::new(hash_index)),
            head: Arc::new(RwLock::new(Some(genesis))),
            validator: ChainValidator::new(),
            stats: Arc::new(RwLock::new(stats)),
        }
    }

    /// Get the node's matrix coordinate
    pub fn node_coordinate(&self) -> &MatrixCoordinate {
        &self.node_coordinate
    }

    /// Add a new block to this node's chain
    pub async fn add_block(&self, data: Vec<u8>) -> Result<Block, String> {
        let head = self.head.read().await;
        let previous = head.as_ref()
            .ok_or_else(|| "No head block found".to_string())?;

        let new_index = previous.index + 1;
        let new_block = Block::new(
            new_index,
            data,
            previous.hash.clone(),
            self.node_coordinate.clone(),
        );

        let previous_clone = previous.clone();
        drop(head); // Release read lock

        // Validate the new block
        if !self.validator.validate_block(&new_block, Some(&previous_clone)) {
            return Err("Block validation failed".to_string());
        }

        // Add block to chain
        self.insert_block(new_block.clone()).await?;

        info!(
            "Added block #{} to node ({},{},{}) chain",
            new_index,
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z
        );

        Ok(new_block)
    }

    /// Insert a block into the chain (internal helper)
    async fn insert_block(&self, block: Block) -> Result<(), String> {
        let mut blocks = self.blocks.write().await;
        let mut hash_index = self.hash_index.write().await;
        let mut head = self.head.write().await;
        let mut stats = self.stats.write().await;

        // Check for duplicate
        if blocks.contains_key(&block.index) {
            return Err(format!("Block {} already exists", block.index));
        }

        // Update indices
        blocks.insert(block.index, block.clone());
        hash_index.insert(block.hash.clone(), block.index);

        // Update head if this is the latest block
        if head.as_ref().map_or(true, |h| block.index > h.index) {
            *head = Some(block.clone());
            stats.chain_height = block.index;
        }

        // Update statistics
        stats.total_blocks += 1;
        stats.total_data_size += block.size();
        self.update_avg_block_time(&mut stats, &block).await;

        Ok(())
    }

    /// Update average block time statistic
    async fn update_avg_block_time(&self, stats: &mut ChainStats, new_block: &Block) {
        if new_block.index > 0 {
            let blocks = self.blocks.read().await;
            if let Some(prev_block) = blocks.get(&(new_block.index - 1)) {
                let time_diff = new_block.timestamp - prev_block.timestamp;
                let diff_ms = time_diff.num_milliseconds() as f64;

                // Calculate running average
                let n = new_block.index as f64;
                stats.avg_block_time_ms =
                    (stats.avg_block_time_ms * (n - 1.0) + diff_ms) / n;
            }
        }
    }

    /// Get a block by index
    pub async fn get_block(&self, index: u64) -> Option<Block> {
        self.blocks.read().await.get(&index).cloned()
    }

    /// Get a block by hash
    pub async fn get_block_by_hash(&self, hash: &str) -> Option<Block> {
        let hash_index = self.hash_index.read().await;
        if let Some(&index) = hash_index.get(hash) {
            self.get_block(index).await
        } else {
            None
        }
    }

    /// Get the current chain head (latest block)
    pub async fn get_head(&self) -> Option<Block> {
        self.head.read().await.clone()
    }

    /// Get the chain height (index of latest block)
    pub async fn get_height(&self) -> u64 {
        self.head.read().await
            .as_ref()
            .map(|b| b.index)
            .unwrap_or(0)
    }

    /// Get all blocks in order
    pub async fn get_chain(&self) -> Vec<Block> {
        let blocks = self.blocks.read().await;
        let mut chain: Vec<Block> = blocks.values().cloned().collect();
        chain.sort_by_key(|b| b.index);
        chain
    }

    /// Validate the entire chain
    pub async fn validate_chain(&self) -> bool {
        let chain = self.get_chain().await;

        if chain.is_empty() {
            warn!("Empty chain");
            return false;
        }

        // Validate genesis
        if !chain[0].is_genesis() {
            error!("First block is not genesis");
            return false;
        }

        // Validate each block and its link to previous
        for i in 1..chain.len() {
            if !self.validator.validate_block(&chain[i], Some(&chain[i - 1])) {
                error!("Block {} failed validation", chain[i].index);
                return false;
            }
        }

        debug!("Chain validation successful for {} blocks", chain.len());
        true
    }

    /// Get chain statistics
    pub async fn get_stats(&self) -> ChainStats {
        self.stats.read().await.clone()
    }

    /// Check if a block exists in this chain
    pub async fn has_block(&self, hash: &str) -> bool {
        self.hash_index.read().await.contains_key(hash)
    }

    /// Get blocks within a time range
    pub async fn get_blocks_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<Block> {
        let blocks = self.blocks.read().await;
        let mut result: Vec<Block> = blocks
            .values()
            .filter(|b| b.timestamp >= start && b.timestamp <= end)
            .cloned()
            .collect();
        result.sort_by_key(|b| b.index);
        result
    }

    /// Get the last N blocks
    pub async fn get_recent_blocks(&self, count: usize) -> Vec<Block> {
        let chain = self.get_chain().await;
        let start = chain.len().saturating_sub(count);
        chain[start..].to_vec()
    }

    /// Calculate chain's total size in bytes
    pub async fn get_total_size(&self) -> usize {
        self.blocks.read().await
            .values()
            .map(|b| b.size())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_blockchain_creation() {
        let coord = MatrixCoordinate::new(1, 2, 3).unwrap();
        let chain = NodeBlockchain::new(coord.clone());

        assert_eq!(chain.node_coordinate(), &coord);
        assert_eq!(chain.get_height().await, 0);

        let head = chain.get_head().await.unwrap();
        assert!(head.is_genesis());
        assert_eq!(head.node_coordinate, coord);
    }

    #[tokio::test]
    async fn test_add_blocks() {
        let coord = MatrixCoordinate::new(5, 5, 5).unwrap();
        let chain = NodeBlockchain::new(coord);

        // Add first block
        let block1 = chain.add_block(b"First block".to_vec()).await.unwrap();
        assert_eq!(block1.index, 1);
        assert_eq!(chain.get_height().await, 1);

        // Add second block
        let block2 = chain.add_block(b"Second block".to_vec()).await.unwrap();
        assert_eq!(block2.index, 2);
        assert_eq!(block2.previous_hash, block1.hash);
        assert_eq!(chain.get_height().await, 2);

        // Verify chain
        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_block_retrieval() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let chain = NodeBlockchain::new(coord);

        let block = chain.add_block(b"Test data".to_vec()).await.unwrap();

        // Get by index
        let retrieved = chain.get_block(1).await.unwrap();
        assert_eq!(retrieved, block);

        // Get by hash
        let retrieved = chain.get_block_by_hash(&block.hash).await.unwrap();
        assert_eq!(retrieved, block);

        // Check existence
        assert!(chain.has_block(&block.hash).await);
        assert!(!chain.has_block("nonexistent").await);
    }

    #[tokio::test]
    async fn test_chain_statistics() {
        let coord = MatrixCoordinate::new(1, 1, 1).unwrap();
        let chain = NodeBlockchain::new(coord);

        // Add some blocks
        for i in 0..5 {
            let data = format!("Block {}", i);
            chain.add_block(data.as_bytes().to_vec()).await.unwrap();
        }

        let stats = chain.get_stats().await;
        assert_eq!(stats.total_blocks, 6); // Including genesis
        assert_eq!(stats.chain_height, 5);
        assert!(stats.chain_start.is_some());
        assert!(stats.total_data_size > 0);
    }

    #[tokio::test]
    async fn test_get_chain() {
        let coord = MatrixCoordinate::new(2, 2, 2).unwrap();
        let chain = NodeBlockchain::new(coord);

        // Add blocks
        for i in 0..3 {
            chain.add_block(vec![i as u8; 10]).await.unwrap();
        }

        let full_chain = chain.get_chain().await;
        assert_eq!(full_chain.len(), 4); // 3 + genesis

        // Verify ordering
        for i in 0..full_chain.len() {
            assert_eq!(full_chain[i].index, i as u64);
        }
    }

    #[tokio::test]
    async fn test_recent_blocks() {
        let coord = MatrixCoordinate::new(3, 3, 3).unwrap();
        let chain = NodeBlockchain::new(coord);

        // Add 10 blocks
        for i in 0..10 {
            chain.add_block(vec![i as u8]).await.unwrap();
        }

        // Get last 5 blocks
        let recent = chain.get_recent_blocks(5).await;
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].index, 6);
        assert_eq!(recent[4].index, 10);
    }

    #[tokio::test]
    async fn test_blocks_in_time_range() {
        let coord = MatrixCoordinate::new(4, 4, 4).unwrap();
        let chain = NodeBlockchain::new(coord);

        let start_time = Utc::now();

        // Add blocks with small delays
        for i in 0..3 {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            chain.add_block(vec![i]).await.unwrap();
        }

        let end_time = Utc::now();

        // Get blocks in range
        let blocks = chain.get_blocks_in_range(start_time, end_time).await;
        assert!(blocks.len() >= 3); // At least the blocks we added
    }

    #[tokio::test]
    async fn test_chain_validation() {
        let coord = MatrixCoordinate::new(6, 6, 6).unwrap();
        let chain = NodeBlockchain::new(coord);

        // Add valid blocks
        for i in 0..5 {
            chain.add_block(vec![i]).await.unwrap();
        }

        // Chain should be valid
        assert!(chain.validate_chain().await);

        // TODO: Test invalid chain scenarios (would need to manipulate internals)
    }

    #[tokio::test]
    async fn test_total_size() {
        let coord = MatrixCoordinate::new(7, 7, 7).unwrap();
        let chain = NodeBlockchain::new(coord);

        // Add blocks with known sizes
        chain.add_block(vec![0u8; 100]).await.unwrap();
        chain.add_block(vec![0u8; 200]).await.unwrap();

        let total_size = chain.get_total_size().await;
        assert!(total_size >= 300); // At least the data we added
    }
}