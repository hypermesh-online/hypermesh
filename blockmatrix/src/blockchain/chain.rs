// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Core chain data structures and read-only operations.
//!
//! `NodeBlockchain` holds the in-memory chain state (blocks, indices,
//! statistics) and exposes query methods.  Mutation methods live in
//! [`super::mutations`]; genesis-auth helpers in [`super::genesis_ops`].

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::block::Block;
use super::genesis_auth::GenesisAuthManager;
use super::validation::ChainValidator;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::proof_of_state::validation_service::ValidationService;
use crate::proof_of_state::StateRequirements;

/// Statistics about a node's blockchain.
#[derive(Debug, Clone, Default)]
pub struct ChainStats {
    /// Total number of blocks
    pub total_blocks: u64,
    /// Current chain height (latest block index)
    pub chain_height: u64,
    /// Total data size in bytes
    pub total_data_size: usize,
}

/// A node's independent blockchain.
///
/// Each BlockMatrix node maintains its own sovereign blockchain.
/// This is NOT a shared chain -- each node has complete autonomy.
pub struct NodeBlockchain {
    /// The node's position in the matrix
    pub(crate) node_coordinate: MatrixCoordinate,

    /// The chain of blocks (index -> block)
    pub(crate) blocks: Arc<RwLock<HashMap<u64, Block>>>,

    /// Quick lookup: hash -> block index
    pub(crate) hash_index: Arc<RwLock<HashMap<String, u64>>>,

    /// Current chain head (latest block)
    pub(crate) head: Arc<RwLock<Option<Block>>>,

    /// Chain validator (structural: hash linkage, size)
    pub(crate) validator: ChainValidator,

    /// State proof validation service (four-proof: WHO/WHEN/WHERE/WHAT)
    pub(crate) state_proof_validator: Arc<ValidationService>,

    /// Chain statistics
    pub(crate) stats: Arc<RwLock<ChainStats>>,

    /// Genesis authentication manager (optional MFA)
    pub(crate) genesis_auth: Arc<RwLock<Option<GenesisAuthManager>>>,
}

impl NodeBlockchain {
    /// Create a new blockchain for a node.
    pub fn new(node_coordinate: MatrixCoordinate) -> Self {
        let genesis = Block::genesis(node_coordinate);
        let mut blocks = HashMap::new();
        let mut hash_index = HashMap::new();

        hash_index.insert(genesis.hash.clone(), genesis.index);
        blocks.insert(genesis.index, genesis.clone());

        let stats = ChainStats {
            total_blocks: 1,
            chain_height: 0,
            total_data_size: genesis.size(),
        };

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
            state_proof_validator: Arc::new(ValidationService::new()),
            stats: Arc::new(RwLock::new(stats)),
            genesis_auth: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new blockchain with custom state proof requirements.
    ///
    /// Use `StateRequirements::production()` for production deployments,
    /// `StateRequirements::default()` for development/testing.
    pub fn with_requirements(
        node_coordinate: MatrixCoordinate,
        requirements: StateRequirements,
    ) -> Self {
        let mut chain = Self::new(node_coordinate);
        chain.state_proof_validator = Arc::new(ValidationService::with_requirements(requirements));
        chain
    }

    /// Reconstruct a blockchain from persisted blocks.
    ///
    /// Used on restart to rebuild the in-memory chain from blocks loaded
    /// from disk via `BlockchainStorage`. Validates chain integrity
    /// (previous_hash linkage) and rebuilds all indices.
    pub fn from_blocks(
        node_coordinate: MatrixCoordinate,
        mut blocks: Vec<Block>,
    ) -> Result<Self, String> {
        if blocks.is_empty() {
            return Err("Cannot create blockchain from empty block list".to_string());
        }

        blocks.sort_by_key(|b| b.index);

        if blocks[0].index != 0 {
            return Err(format!(
                "First block must be genesis (index 0), got index {}",
                blocks[0].index,
            ));
        }

        // Validate chain integrity: each block's previous_hash must match
        // the prior block's hash (skip genesis which has no predecessor)
        for i in 1..blocks.len() {
            if blocks[i].previous_hash != blocks[i - 1].hash {
                return Err(format!(
                    "Chain integrity violation at block {}: previous_hash {} != block {}'s hash {}",
                    blocks[i].index,
                    blocks[i].previous_hash,
                    blocks[i - 1].index,
                    blocks[i - 1].hash,
                ));
            }
        }

        // Rebuild indices
        let mut block_map = HashMap::with_capacity(blocks.len());
        let mut hash_index = HashMap::with_capacity(blocks.len());
        let mut total_data_size = 0usize;

        for block in &blocks {
            block_map.insert(block.index, block.clone());
            hash_index.insert(block.hash.clone(), block.index);
            total_data_size += block.size();
        }

        let head = blocks.last().cloned();
        let chain_height = head.as_ref().map(|b| b.index).unwrap_or(0);

        let stats = ChainStats {
            total_blocks: blocks.len() as u64,
            chain_height,
            total_data_size,
        };

        info!(
            "Restored blockchain for node ({},{},{}) -- {} blocks, height {}",
            node_coordinate.x,
            node_coordinate.y,
            node_coordinate.z,
            blocks.len(),
            chain_height,
        );

        Ok(NodeBlockchain {
            node_coordinate,
            blocks: Arc::new(RwLock::new(block_map)),
            hash_index: Arc::new(RwLock::new(hash_index)),
            head: Arc::new(RwLock::new(head)),
            validator: ChainValidator::new(),
            state_proof_validator: Arc::new(ValidationService::new()),
            stats: Arc::new(RwLock::new(stats)),
            genesis_auth: Arc::new(RwLock::new(None)),
        })
    }

    /// Get the node's matrix coordinate.
    pub fn node_coordinate(&self) -> &MatrixCoordinate {
        &self.node_coordinate
    }

    /// Get a block by index.
    pub async fn get_block(&self, index: u64) -> Option<Block> {
        self.blocks.read().await.get(&index).cloned()
    }

    /// Get a block by hash.
    pub async fn get_block_by_hash(&self, hash: &str) -> Option<Block> {
        let hash_index = self.hash_index.read().await;
        if let Some(&index) = hash_index.get(hash) {
            drop(hash_index);
            self.get_block(index).await
        } else {
            None
        }
    }

    /// Get the current chain head (latest block).
    pub async fn get_head(&self) -> Option<Block> {
        self.head.read().await.clone()
    }

    /// Get the chain height (index of latest block).
    pub async fn get_height(&self) -> u64 {
        self.head
            .read()
            .await
            .as_ref()
            .map(|b| b.index)
            .unwrap_or(0)
    }

    /// Get all blocks in order.
    pub async fn get_chain(&self) -> Vec<Block> {
        let blocks = self.blocks.read().await;
        let mut chain: Vec<Block> = blocks.values().cloned().collect();
        chain.sort_by_key(|b| b.index);
        chain
    }

    /// Validate the entire chain.
    pub async fn validate_chain(&self) -> bool {
        let chain = self.get_chain().await;

        if chain.is_empty() {
            warn!("Empty chain");
            return false;
        }

        if !chain[0].is_genesis() {
            error!("First block is not genesis");
            return false;
        }

        for i in 1..chain.len() {
            if !self
                .validator
                .validate_block(&chain[i], Some(&chain[i - 1]))
            {
                error!("Block {} failed validation", chain[i].index);
                return false;
            }
        }

        debug!("Chain validation successful for {} blocks", chain.len());
        true
    }

    /// Get chain statistics.
    pub async fn get_stats(&self) -> ChainStats {
        self.stats.read().await.clone()
    }

    /// Check if a block exists in this chain.
    pub async fn has_block(&self, hash: &str) -> bool {
        self.hash_index.read().await.contains_key(hash)
    }

    /// Get the last N blocks.
    pub async fn get_recent_blocks(&self, count: usize) -> Vec<Block> {
        let chain = self.get_chain().await;
        let start = chain.len().saturating_sub(count);
        chain[start..].to_vec()
    }

    /// Calculate chain's total size in bytes.
    pub async fn get_total_size(&self) -> usize {
        self.blocks.read().await.values().map(|b| b.size()).sum()
    }

    /// Insert a block into the chain (internal helper).
    pub(crate) async fn insert_block(&self, block: Block) -> Result<(), String> {
        let mut blocks = self.blocks.write().await;
        let mut hash_index = self.hash_index.write().await;
        let mut head = self.head.write().await;
        let mut stats = self.stats.write().await;

        if blocks.contains_key(&block.index) {
            return Err(format!("Block {} already exists", block.index));
        }

        blocks.insert(block.index, block.clone());
        hash_index.insert(block.hash.clone(), block.index);

        if head.as_ref().is_none_or(|h| block.index > h.index) {
            *head = Some(block.clone());
            stats.chain_height = block.index;
        }

        stats.total_blocks += 1;
        stats.total_data_size += block.size();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::block::{BlockAssetEntry, StoragePointer};
    use crate::assets::core::AssetRegistration;
    use trustchain::proof_of_state::StateProof;

    fn test_entry(coord: MatrixCoordinate) -> BlockAssetEntry {
        let reg = AssetRegistration::genesis(coord);
        let content_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        let state_proof = StateProof::default();
        let proof_bytes = serde_json::to_vec(&state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();
        BlockAssetEntry {
            asset_hash: content_hash,
            proof_hash,
            state_proof,
            storage_pointer: StoragePointer::Genesis,
            registration: reg,
        }
    }

    #[tokio::test]
    async fn test_blockchain_creation() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        assert_eq!(chain.node_coordinate(), &coord);
        assert_eq!(chain.get_height().await, 0);

        let head = chain.get_head().await.expect("test: block retrieval");
        assert!(head.is_genesis());
    }

    #[tokio::test]
    async fn test_block_retrieval() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Insert a block manually via insert_block
        let entry = test_entry(coord);
        let genesis = chain.get_head().await.expect("test: genesis");
        let block = Block::new(1, vec![entry], genesis.hash.clone());

        chain.insert_block(block.clone()).await.expect("test: insert");

        let retrieved = chain.get_block(1).await.expect("test: block retrieval");
        assert_eq!(retrieved, block);

        let retrieved = chain
            .get_block_by_hash(&block.hash)
            .await
            .expect("test: block retrieval");
        assert_eq!(retrieved, block);

        assert!(chain.has_block(&block.hash).await);
        assert!(!chain.has_block("nonexistent").await);
    }

    #[tokio::test]
    async fn test_chain_statistics() {
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Insert 5 blocks manually
        for i in 1..=5u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        let stats = chain.get_stats().await;
        assert_eq!(stats.total_blocks, 6); // Including genesis
        assert_eq!(stats.chain_height, 5);
        assert!(stats.total_data_size > 0);
    }

    #[tokio::test]
    async fn test_get_chain() {
        let coord = MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        for i in 1..=3u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        let full_chain = chain.get_chain().await;
        assert_eq!(full_chain.len(), 4); // 3 + genesis

        for (i, block) in full_chain.iter().enumerate() {
            assert_eq!(block.index, i as u64);
        }
    }

    #[tokio::test]
    async fn test_recent_blocks() {
        let coord = MatrixCoordinate::new(3, 3, 3).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        for i in 1..=10u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        let recent = chain.get_recent_blocks(5).await;
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].index, 6);
        assert_eq!(recent[4].index, 10);
    }

    #[tokio::test]
    async fn test_chain_validation() {
        let coord = MatrixCoordinate::new(6, 6, 6).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        for i in 1..=5u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_total_size() {
        let coord = MatrixCoordinate::new(7, 7, 7).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        for i in 1..=2u64 {
            let prev = chain.get_head().await.expect("test: head");
            let entry = test_entry(coord);
            let block = Block::new(i, vec![entry], prev.hash.clone());
            chain.insert_block(block).await.expect("test: insert");
        }

        let total_size = chain.get_total_size().await;
        assert!(total_size > 0);
    }

    #[tokio::test]
    async fn test_from_blocks() {
        let coord = MatrixCoordinate::new(4, 4, 4).expect("test: valid coordinate");

        // Build a chain manually
        let genesis = Block::genesis(coord);
        let entry1 = test_entry(coord);
        let block1 = Block::new(1, vec![entry1], genesis.hash.clone());
        let entry2 = test_entry(coord);
        let block2 = Block::new(2, vec![entry2], block1.hash.clone());

        let blocks = vec![genesis, block1, block2];
        let chain =
            NodeBlockchain::from_blocks(coord, blocks).expect("test: from_blocks");

        assert_eq!(chain.get_height().await, 2);
        assert_eq!(chain.get_stats().await.total_blocks, 3);
        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_from_blocks_empty_fails() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let result = NodeBlockchain::from_blocks(coord, vec![]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_from_blocks_broken_chain_fails() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");

        let genesis = Block::genesis(coord);
        let entry = test_entry(coord);
        // Wrong previous_hash
        let bad_block = Block::new(1, vec![entry], "wrong_hash".to_string());

        let result = NodeBlockchain::from_blocks(coord, vec![genesis, bad_block]);
        assert!(result.is_err());
    }
}
