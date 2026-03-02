// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Node-specific blockchain implementation
//!
//! Revolutionary architecture: Each node maintains its own independent blockchain.
//! NO merkle tree consolidation, NO shared chain across nodes.
//! Complete node sovereignty over its own ledger.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::block::Block;
use super::genesis_auth::{GenesisAuthManager, GenesisCredentials};
use super::validation::ChainValidator;
use crate::assets::core::AssetRegistration;
use crate::matrix::coordinate::MatrixCoordinate;

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

    /// Genesis authentication manager (optional MFA)
    genesis_auth: Arc<RwLock<Option<GenesisAuthManager>>>,
}

impl NodeBlockchain {
    /// Create a new blockchain for a node
    pub fn new(node_coordinate: MatrixCoordinate) -> Self {
        let genesis = Block::genesis(node_coordinate);
        let mut blocks = HashMap::new();
        let mut hash_index = HashMap::new();

        hash_index.insert(genesis.hash.clone(), genesis.index);
        blocks.insert(genesis.index, genesis.clone());

        let stats = ChainStats {
            total_blocks: 1,
            chain_height: 0,
            chain_start: Some(genesis.timestamp),
            total_data_size: genesis.size(),
            ..ChainStats::default()
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
            stats: Arc::new(RwLock::new(stats)),
            genesis_auth: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the node's matrix coordinate
    pub fn node_coordinate(&self) -> &MatrixCoordinate {
        &self.node_coordinate
    }

    /// Add a new block to this node's chain
    pub async fn add_block(&self, assets: Vec<AssetRegistration>) -> Result<Block, String> {
        let head = self.head.read().await;
        let previous = head
            .as_ref()
            .ok_or_else(|| "No head block found".to_string())?;

        let new_index = previous.index + 1;
        let new_block = Block::new(
            new_index,
            assets,
            previous.hash.clone(),
            self.node_coordinate,
        );

        let previous_clone = previous.clone();
        drop(head); // Release read lock

        // Validate the new block
        if !self
            .validator
            .validate_block(&new_block, Some(&previous_clone))
        {
            return Err("Block validation failed".to_string());
        }

        // Add block to chain
        self.insert_block(new_block.clone()).await?;

        info!(
            "Added block #{} to node ({},{},{}) chain",
            new_index, self.node_coordinate.x, self.node_coordinate.y, self.node_coordinate.z
        );

        Ok(new_block)
    }

    /// Helper: Create asset from data and add block (temporary compatibility method)
    /// TODO: Remove this once all callers properly create Assets
    pub async fn add_block_with_data(&self, data: Vec<u8>) -> Result<Block, String> {
        use crate::assets::core::asset_id::{
            AssetCategory, AssetData, BaseSystemType, NetworkScope,
        };

        // Create an asset from the data
        let asset_data = AssetData {
            config: Vec::new(),
            definition: data.clone(),
            metadata: format!("Block data at {:?}", std::time::SystemTime::now()).into_bytes(),
        };

        let asset_id = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Container),
        );

        self.add_block(vec![asset_id]).await
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

        // Get previous block for time calculation before inserting new block
        let prev_block = if block.index > 0 {
            blocks.get(&(block.index - 1)).cloned()
        } else {
            None
        };

        // Update indices
        blocks.insert(block.index, block.clone());
        hash_index.insert(block.hash.clone(), block.index);

        // Update head if this is the latest block
        if head.as_ref().is_none_or(|h| block.index > h.index) {
            *head = Some(block.clone());
            stats.chain_height = block.index;
        }

        // Update statistics
        stats.total_blocks += 1;
        stats.total_data_size += block.size();

        // Update average block time using the previous block we already have
        if let Some(prev_block) = prev_block {
            let time_diff = block.timestamp - prev_block.timestamp;
            let diff_ms = time_diff.num_milliseconds() as f64;
            let n = block.index as f64;
            stats.avg_block_time_ms = (stats.avg_block_time_ms * (n - 1.0) + diff_ms) / n;
        }

        Ok(())
    }

    // Removed: update_avg_block_time - logic moved inline to avoid deadlock

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
        self.head
            .read()
            .await
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
        self.blocks.read().await.values().map(|b| b.size()).sum()
    }

    // === Asset Registration ===

    /// Add a new block with an automatic shard commitment (R12).
    ///
    /// When a block carries shard distribution data, the commitment
    /// `BLAKE3(sorted placements)` is computed and anchored in the block
    /// header before validation and insertion.
    ///
    /// `shard_placement_data` is the canonical byte serialization of the
    /// sorted placement list (caller is responsible for sorting by shard
    /// index before serializing).
    pub async fn add_block_with_shard_commitment(
        &self,
        assets: Vec<AssetRegistration>,
        shard_placement_data: &[u8],
    ) -> Result<Block, String> {
        let head = self.head.read().await;
        let previous = head
            .as_ref()
            .ok_or_else(|| "No head block found".to_string())?;

        let new_index = previous.index + 1;
        let mut new_block = Block::new(
            new_index,
            assets,
            previous.hash.clone(),
            self.node_coordinate,
        );

        // Compute shard commitment and set it (recalculates block hash)
        let commitment = hypermesh_lib::protocol::ShardCommitment::compute(shard_placement_data);
        new_block.set_shard_commitment(*commitment.as_bytes());

        let previous_clone = previous.clone();
        drop(head);

        if !self
            .validator
            .validate_block(&new_block, Some(&previous_clone))
        {
            return Err("Block validation failed".to_string());
        }

        self.insert_block(new_block.clone()).await?;

        info!(
            "Added block #{} with shard commitment to node ({},{},{}) chain",
            new_index,
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z,
        );

        Ok(new_block)
    }

    /// Register an asset record on this node's blockchain.
    ///
    /// Creates a new block containing the [`AssetRegistration`], validates
    /// it against the chain, and appends it.  Returns the block that was
    /// produced (callers can inspect its hash for receipts).
    pub async fn register_asset_record(
        &self,
        registration: AssetRegistration,
    ) -> Result<Block, String> {
        info!(
            "Registering asset on blockchain at ({},{},{})",
            self.node_coordinate.x, self.node_coordinate.y, self.node_coordinate.z,
        );
        self.add_block(vec![registration]).await
    }

    /// Register multiple asset records in a single block.
    ///
    /// Useful during genesis to batch all hardware assets into one block.
    pub async fn register_asset_records(
        &self,
        registrations: Vec<AssetRegistration>,
    ) -> Result<Block, String> {
        if registrations.is_empty() {
            return Err("cannot register empty asset list".to_string());
        }
        info!(
            "Registering {} assets on blockchain at ({},{},{})",
            registrations.len(),
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z,
        );
        self.add_block(registrations).await
    }

    // === MFA Genesis Authentication Methods ===

    /// Initialize MFA-protected genesis authentication
    ///
    /// # Arguments
    /// * `user_id` - User identifier (username/email)
    /// * `passphrase` - User passphrase for key derivation
    ///
    /// # Returns
    /// Tuple of (TOTP secret for user to save, recovery codes)
    pub async fn initialize_genesis_auth(
        &self,
        user_id: String,
        passphrase: &str,
    ) -> Result<(String, Vec<String>), String> {
        let mut auth_guard = self.genesis_auth.write().await;

        if auth_guard.is_some() {
            return Err("Genesis authentication already initialized".to_string());
        }

        let mut auth_manager = GenesisAuthManager::new();
        let (totp_secret, recovery_codes) = auth_manager
            .initialize(user_id, passphrase, self.node_coordinate)
            .map_err(|e| format!("Failed to initialize genesis auth: {e}"))?;

        *auth_guard = Some(auth_manager);

        info!(
            "Genesis authentication initialized for node ({}, {}, {})",
            self.node_coordinate.x, self.node_coordinate.y, self.node_coordinate.z
        );

        Ok((totp_secret, recovery_codes))
    }

    /// Authenticate and unlock genesis block (MFA required)
    ///
    /// # Arguments
    /// * `passphrase` - User passphrase
    /// * `totp_code` - Current TOTP code (6 digits)
    ///
    /// # Returns
    /// Decrypted private key if authentication successful
    pub async fn authenticate_genesis(
        &self,
        passphrase: &str,
        totp_code: &str,
    ) -> Result<Vec<u8>, String> {
        let mut auth_guard = self.genesis_auth.write().await;

        let auth_manager = auth_guard
            .as_mut()
            .ok_or_else(|| "Genesis authentication not initialized".to_string())?;

        auth_manager
            .authenticate(passphrase, totp_code)
            .map_err(|e| format!("Authentication failed: {e}"))
    }

    /// Recover genesis access using recovery code
    ///
    /// # Arguments
    /// * `passphrase` - User passphrase
    /// * `recovery_code` - One of the recovery codes
    ///
    /// # Returns
    /// New TOTP secret (user must save this)
    pub async fn recover_genesis(
        &self,
        passphrase: &str,
        recovery_code: &str,
    ) -> Result<String, String> {
        let mut auth_guard = self.genesis_auth.write().await;

        let auth_manager = auth_guard
            .as_mut()
            .ok_or_else(|| "Genesis authentication not initialized".to_string())?;

        auth_manager
            .recover_with_code(passphrase, recovery_code)
            .map_err(|e| format!("Recovery failed: {e}"))
    }

    /// Get genesis credentials for serialization/storage
    pub async fn get_genesis_credentials(&self) -> Option<GenesisCredentials> {
        let auth_guard = self.genesis_auth.read().await;
        auth_guard
            .as_ref()
            .and_then(|auth| auth.get_credentials().cloned())
    }

    /// Load genesis credentials from external storage
    pub async fn load_genesis_credentials(
        &self,
        credentials: GenesisCredentials,
    ) -> Result<(), String> {
        let mut auth_guard = self.genesis_auth.write().await;

        if auth_guard.is_some() {
            return Err("Genesis authentication already loaded".to_string());
        }

        let mut auth_manager = GenesisAuthManager::new();
        auth_manager
            .load_credentials(credentials)
            .map_err(|e| format!("Failed to load credentials: {e}"))?;

        *auth_guard = Some(auth_manager);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_blockchain_creation() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        assert_eq!(chain.node_coordinate(), &coord);
        assert_eq!(chain.get_height().await, 0);

        let head = chain.get_head().await.expect("test: block retrieval");
        assert!(head.is_genesis());
        assert_eq!(head.node_coordinate, coord);
    }

    #[tokio::test]
    async fn test_add_blocks() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Add first block
        let block1 = chain
            .add_block_with_data(b"First block".to_vec())
            .await
            .expect("test: expected success");
        assert_eq!(block1.index, 1);
        assert_eq!(chain.get_height().await, 1);

        // Add second block
        let block2 = chain
            .add_block_with_data(b"Second block".to_vec())
            .await
            .expect("test: expected success");
        assert_eq!(block2.index, 2);
        assert_eq!(block2.previous_hash, block1.hash);
        assert_eq!(chain.get_height().await, 2);

        // Verify chain
        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_block_retrieval() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let block = chain
            .add_block_with_data(b"Test data".to_vec())
            .await
            .expect("test: expected success");

        // Get by index
        let retrieved = chain.get_block(1).await.expect("test: block retrieval");
        assert_eq!(retrieved, block);

        // Get by hash
        let retrieved = chain.get_block_by_hash(&block.hash).await.expect("test: block retrieval");
        assert_eq!(retrieved, block);

        // Check existence
        assert!(chain.has_block(&block.hash).await);
        assert!(!chain.has_block("nonexistent").await);
    }

    #[tokio::test]
    async fn test_chain_statistics() {
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Add some blocks
        for i in 0..5 {
            let data = format!("Block {i}");
            chain
                .add_block_with_data(data.as_bytes().to_vec())
                .await
                .expect("test: expected success");
        }

        let stats = chain.get_stats().await;
        assert_eq!(stats.total_blocks, 6); // Including genesis
        assert_eq!(stats.chain_height, 5);
        assert!(stats.chain_start.is_some());
        assert!(stats.total_data_size > 0);
    }

    #[tokio::test]
    async fn test_get_chain() {
        let coord = MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Add blocks
        for i in 0..3 {
            chain.add_block_with_data(vec![i as u8; 10]).await.expect("test: block addition");
        }

        let full_chain = chain.get_chain().await;
        assert_eq!(full_chain.len(), 4); // 3 + genesis

        // Verify ordering
        for (i, block) in full_chain.iter().enumerate() {
            assert_eq!(block.index, i as u64);
        }
    }

    #[tokio::test]
    async fn test_recent_blocks() {
        let coord = MatrixCoordinate::new(3, 3, 3).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Add 10 blocks
        for i in 0..10 {
            chain.add_block_with_data(vec![i as u8]).await.expect("test: block addition");
        }

        // Get last 5 blocks
        let recent = chain.get_recent_blocks(5).await;
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].index, 6);
        assert_eq!(recent[4].index, 10);
    }

    #[tokio::test]
    async fn test_blocks_in_time_range() {
        let coord = MatrixCoordinate::new(4, 4, 4).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let start_time = Utc::now();

        // Add blocks with small delays
        for i in 0..3 {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            chain.add_block_with_data(vec![i]).await.expect("test: block addition");
        }

        let end_time = Utc::now();

        // Get blocks in range
        let blocks = chain.get_blocks_in_range(start_time, end_time).await;
        assert!(blocks.len() >= 3); // At least the blocks we added
    }

    #[tokio::test]
    async fn test_chain_validation() {
        let coord = MatrixCoordinate::new(6, 6, 6).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Add valid blocks
        for i in 0..5 {
            chain.add_block_with_data(vec![i]).await.expect("test: block addition");
        }

        // Chain should be valid
        assert!(chain.validate_chain().await);

        // TODO: Test invalid chain scenarios (would need to manipulate internals)
    }

    // === Item 4.2: Asset registration tests ===

    #[tokio::test]
    async fn test_register_asset_record() {
        let coord = MatrixCoordinate::new(8, 8, 8).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let asset = AssetRegistration::genesis(coord);
        let block = chain
            .register_asset_record(asset.clone())
            .await
            .expect("test: registration");

        assert_eq!(block.index, 1);
        assert_eq!(block.assets.len(), 1);
        assert_eq!(block.assets[0], asset);
        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_register_multiple_asset_records() {
        let coord = MatrixCoordinate::new(9, 9, 9).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let assets = vec![
            AssetRegistration::genesis(coord),
            AssetRegistration::genesis(
                MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate"),
            ),
        ];
        let block = chain
            .register_asset_records(assets.clone())
            .await
            .expect("test: batch registration");

        assert_eq!(block.index, 1);
        assert_eq!(block.assets.len(), 2);
        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_register_empty_assets_fails() {
        let coord = MatrixCoordinate::new(10, 10, 10).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let result = chain.register_asset_records(vec![]).await;
        assert!(result.is_err());
    }

    // === Item 4.3: Shard commitment tests ===

    #[tokio::test]
    async fn test_add_block_with_shard_commitment() {
        let coord = MatrixCoordinate::new(11, 11, 11).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let asset = AssetRegistration::genesis(coord);
        let placement_data = b"shard0:node-a,shard1:node-b";

        let block = chain
            .add_block_with_shard_commitment(vec![asset], placement_data)
            .await
            .expect("test: block with commitment");

        assert!(
            block.shard_commitment.is_some(),
            "block must have shard commitment"
        );

        // Verify the commitment matches BLAKE3 of the placement data
        let expected = hypermesh_lib::protocol::ShardCommitment::compute(placement_data);
        assert_eq!(
            block.shard_commitment.expect("test: commitment present"),
            *expected.as_bytes(),
        );

        assert!(block.verify_hash(), "hash must be valid after commitment");
        assert!(chain.validate_chain().await);
    }

    #[tokio::test]
    async fn test_shard_commitment_changes_block_hash() {
        let coord = MatrixCoordinate::new(12, 12, 12).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        let asset = AssetRegistration::genesis(coord);

        // Block without commitment
        let block_a = chain
            .add_block(vec![asset.clone()])
            .await
            .expect("test: block without commitment");

        // Block with commitment (different chain to avoid index collision)
        let chain2 = NodeBlockchain::new(coord);
        let block_b = chain2
            .add_block_with_shard_commitment(vec![asset], b"test data")
            .await
            .expect("test: block with commitment");

        assert_ne!(
            block_a.hash, block_b.hash,
            "commitment must change the block hash"
        );
    }

    #[tokio::test]
    async fn test_total_size() {
        let coord = MatrixCoordinate::new(7, 7, 7).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);

        // Add blocks with known sizes
        chain.add_block_with_data(vec![0u8; 100]).await.expect("test: block addition");
        chain.add_block_with_data(vec![0u8; 200]).await.expect("test: block addition");

        let total_size = chain.get_total_size().await;
        assert!(total_size >= 300); // At least the data we added
    }
}
