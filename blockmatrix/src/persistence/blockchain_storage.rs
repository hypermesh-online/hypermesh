// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Blockchain persistence
//!
//! Provides efficient storage for per-node blockchains with append-only logs,
//! indexes, and write-ahead logging for crash recovery.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::{PersistenceError, PersistenceResult};
use crate::blockchain::block::Block;
use crate::blockchain::node_chain::ChainStats;

/// Block file size threshold (1000 blocks per file)
const BLOCKS_PER_FILE: u64 = 1000;

/// Write-ahead log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Operation type
    pub op_type: WalOperation,
    /// Block data
    pub block: Block,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// WAL operation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalOperation {
    /// Add new block
    AddBlock,
    /// Update block (shouldn't happen in blockchain but included for completeness)
    UpdateBlock,
}

/// Blockchain metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMetadata {
    /// Genesis block hash
    pub genesis_hash: String,
    /// Current chain height
    pub chain_height: u64,
    /// Last block hash
    pub last_block_hash: String,
    /// Total blocks
    pub total_blocks: u64,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last modified
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

/// Block query parameters
#[derive(Debug, Clone)]
pub enum BlockQuery {
    /// Query by index
    ByIndex(u64),
    /// Query by hash
    ByHash(String),
    /// Query range of indices
    Range(u64, u64),
    /// Get last N blocks
    Last(u64),
}

/// Manages blockchain storage for a single node
pub struct BlockchainStorage {
    /// Storage directory
    storage_dir: PathBuf,
    /// Node ID
    _node_id: String,
    /// Block index: hash -> (file_id, offset, size)
    block_index: Arc<RwLock<HashMap<String, (u32, u64, u32)>>>,
    /// Index by block number: index -> hash
    index_map: Arc<RwLock<HashMap<u64, String>>>,
    /// Chain metadata
    metadata: Arc<RwLock<ChainMetadata>>,
    /// Write-ahead log
    wal: Arc<RwLock<Option<WalWriter>>>,
}

impl BlockchainStorage {
    /// Create new blockchain storage
    pub async fn new(storage_dir: PathBuf, node_id: String) -> PersistenceResult<Self> {
        // Create directory structure
        let blockchain_dir = storage_dir.join(&node_id).join("blockchain");
        std::fs::create_dir_all(&blockchain_dir)?;
        std::fs::create_dir_all(blockchain_dir.join("blocks"))?;

        // Load or create metadata
        let metadata_path = blockchain_dir.join("metadata.json");
        let metadata = if metadata_path.exists() {
            Self::load_metadata(&metadata_path)?
        } else {
            ChainMetadata {
                genesis_hash: String::new(),
                chain_height: 0,
                last_block_hash: String::new(),
                total_blocks: 0,
                created_at: chrono::Utc::now(),
                last_modified: chrono::Utc::now(),
            }
        };

        // Load block index
        let index_path = blockchain_dir.join("index.db");
        let (block_index, index_map) = if index_path.exists() {
            Self::load_index(&index_path)?
        } else {
            (HashMap::new(), HashMap::new())
        };

        // Initialize WAL
        let wal_path = blockchain_dir.join("wal.log");
        let wal = WalWriter::new(wal_path)?;

        Ok(Self {
            storage_dir: blockchain_dir,
            _node_id: node_id,
            block_index: Arc::new(RwLock::new(block_index)),
            index_map: Arc::new(RwLock::new(index_map)),
            metadata: Arc::new(RwLock::new(metadata)),
            wal: Arc::new(RwLock::new(Some(wal))),
        })
    }

    /// Write a block to storage
    pub async fn write_block(&self, block: &Block) -> PersistenceResult<()> {
        // Write to WAL first
        if let Some(wal) = self.wal.write().await.as_mut() {
            wal.write_entry(WalEntry {
                op_type: WalOperation::AddBlock,
                block: block.clone(),
                timestamp: chrono::Utc::now(),
            })?;
        }

        // Determine which file to write to
        let file_id = (block.index / BLOCKS_PER_FILE) as u32;
        let file_path = self
            .storage_dir
            .join("blocks")
            .join(format!("{file_id:08}.blk"));

        // Append block to file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        let offset = file.seek(SeekFrom::End(0))?;
        let serialized = bincode::serialize(block)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        file.write_all(&serialized)?;
        file.sync_all()?;

        // Update indices
        {
            let mut block_index = self.block_index.write().await;
            let mut index_map = self.index_map.write().await;

            block_index.insert(
                block.hash.clone(),
                (file_id, offset, serialized.len() as u32),
            );
            index_map.insert(block.index, block.hash.clone());
        }

        // Update metadata
        {
            let mut metadata = self.metadata.write().await;
            if block.index == 0 {
                metadata.genesis_hash = block.hash.clone();
            }
            metadata.chain_height = block.index;
            metadata.last_block_hash = block.hash.clone();
            metadata.total_blocks = block.index + 1;
            metadata.last_modified = chrono::Utc::now();
        }

        // Save metadata and index
        self.save_metadata().await?;
        self.save_index().await?;

        info!(
            "Wrote block {} to storage (file {}, offset {})",
            block.index, file_id, offset
        );

        Ok(())
    }

    /// Read a block from storage
    pub async fn read_block(&self, query: BlockQuery) -> PersistenceResult<Option<Block>> {
        match query {
            BlockQuery::ByIndex(index) => {
                let hash = {
                    let index_map = self.index_map.read().await;
                    index_map.get(&index).cloned()
                };

                if let Some(hash) = hash {
                    self.read_block_by_hash(&hash).await
                } else {
                    Ok(None)
                }
            }
            BlockQuery::ByHash(hash) => self.read_block_by_hash(&hash).await,
            BlockQuery::Range(start, _end) => {
                // For range queries, return first block (caller should iterate)
                // Use read_block_by_index to avoid recursion
                self.read_block_by_index(start).await
            }
            BlockQuery::Last(n) => {
                let metadata = self.metadata.read().await;
                let index = if metadata.chain_height >= n {
                    metadata.chain_height - n + 1
                } else {
                    0
                };
                drop(metadata);
                // Use read_block_by_index to avoid recursion
                self.read_block_by_index(index).await
            }
        }
    }

    /// Read block by index (non-recursive helper)
    async fn read_block_by_index(&self, index: u64) -> PersistenceResult<Option<Block>> {
        let hash = {
            let index_map = self.index_map.read().await;
            index_map.get(&index).cloned()
        };

        if let Some(hash) = hash {
            self.read_block_by_hash(&hash).await
        } else {
            Ok(None)
        }
    }

    /// Read blocks in range
    pub async fn read_range(&self, start: u64, end: u64) -> PersistenceResult<Vec<Block>> {
        let mut blocks = Vec::new();

        for index in start..=end {
            if let Some(block) = self.read_block(BlockQuery::ByIndex(index)).await? {
                blocks.push(block);
            }
        }

        Ok(blocks)
    }

    /// Read block by hash
    async fn read_block_by_hash(&self, hash: &str) -> PersistenceResult<Option<Block>> {
        let location = {
            let block_index = self.block_index.read().await;
            block_index.get(hash).cloned()
        };

        if let Some((file_id, offset, size)) = location {
            let file_path = self
                .storage_dir
                .join("blocks")
                .join(format!("{file_id:08}.blk"));

            let mut file = File::open(&file_path)?;
            file.seek(SeekFrom::Start(offset))?;

            let mut buffer = vec![0u8; size as usize];
            file.read_exact(&mut buffer)?;

            let block: Block = bincode::deserialize(&buffer)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    /// Get chain metadata
    pub async fn get_metadata(&self) -> ChainMetadata {
        self.metadata.read().await.clone()
    }

    /// Get chain statistics
    pub async fn get_stats(&self) -> ChainStats {
        let metadata = self.metadata.read().await;

        ChainStats {
            total_blocks: metadata.total_blocks,
            chain_height: metadata.chain_height,
            chain_start: Some(metadata.created_at),
            avg_block_time_ms: 0.0, // Would need to calculate from blocks
            total_data_size: 0,     // Would need to sum from index
        }
    }

    /// Save metadata to disk
    async fn save_metadata(&self) -> PersistenceResult<()> {
        let metadata_path = self.storage_dir.join("metadata.json");
        let metadata = self.metadata.read().await;

        let json = serde_json::to_string_pretty(&*metadata)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        std::fs::write(metadata_path, json)?;
        Ok(())
    }

    /// Load metadata from disk
    fn load_metadata(path: &Path) -> PersistenceResult<ChainMetadata> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(|e| PersistenceError::Deserialization(e.to_string()))
    }

    /// Save block index to disk
    async fn save_index(&self) -> PersistenceResult<()> {
        let index_path = self.storage_dir.join("index.db");
        let block_index = self.block_index.read().await;
        let index_map = self.index_map.read().await;

        let index_data = (block_index.clone(), index_map.clone());
        let serialized = bincode::serialize(&index_data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        std::fs::write(index_path, serialized)?;
        Ok(())
    }

    /// Load block index from disk
    #[allow(clippy::type_complexity)]
    fn load_index(
        path: &Path,
    ) -> PersistenceResult<(HashMap<String, (u32, u64, u32)>, HashMap<u64, String>)> {
        let data = std::fs::read(path)?;
        bincode::deserialize(&data).map_err(|e| PersistenceError::Deserialization(e.to_string()))
    }

    /// Replay WAL entries
    pub async fn replay_wal(&self) -> PersistenceResult<u32> {
        let wal_path = self.storage_dir.join("wal.log");
        if !wal_path.exists() {
            return Ok(0);
        }

        info!("Replaying WAL from {:?}", wal_path);
        let entries = WalReader::read_all(&wal_path)?;
        let count = entries.len() as u32;

        for entry in entries {
            if entry.op_type == WalOperation::AddBlock {
                // Re-apply the block write (without WAL)
                let mut wal = self.wal.write().await;
                *wal = None; // Temporarily disable WAL
                drop(wal);

                self.write_block(&entry.block).await?;

                // Re-enable WAL
                let mut wal = self.wal.write().await;
                *wal = Some(WalWriter::new(wal_path.clone())?);
            }
        }

        info!("Replayed {} WAL entries", count);
        Ok(count)
    }

    /// Flush WAL to storage
    pub async fn flush_wal(&self) -> PersistenceResult<()> {
        if let Some(wal) = self.wal.write().await.as_mut() {
            wal.flush()?;
        }
        Ok(())
    }
}

/// Write-ahead log writer
struct WalWriter {
    file: BufWriter<File>,
}

impl WalWriter {
    fn new(path: PathBuf) -> PersistenceResult<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;

        Ok(Self {
            file: BufWriter::new(file),
        })
    }

    fn write_entry(&mut self, entry: WalEntry) -> PersistenceResult<()> {
        let serialized = bincode::serialize(&entry)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // Write length prefix
        let len = serialized.len() as u32;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.write_all(&serialized)?;
        self.file.flush()?;

        Ok(())
    }

    fn flush(&mut self) -> PersistenceResult<()> {
        self.file.flush()?;
        Ok(())
    }
}

/// Write-ahead log reader
struct WalReader;

impl WalReader {
    fn read_all(path: &Path) -> PersistenceResult<Vec<WalEntry>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();

        loop {
            // Read length prefix
            let mut len_bytes = [0u8; 4];
            if reader.read_exact(&mut len_bytes).is_err() {
                break; // End of file
            }

            let len = u32::from_le_bytes(len_bytes) as usize;
            let mut buffer = vec![0u8; len];
            reader.read_exact(&mut buffer)?;

            let entry: WalEntry = bincode::deserialize(&buffer)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

            entries.push(entry);
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_blockchain_storage_creation() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let metadata = storage.get_metadata().await;
        assert_eq!(metadata.total_blocks, 0);
        assert_eq!(metadata.chain_height, 0);
    }

    #[tokio::test]
    async fn test_write_and_read_block() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let block = Block::genesis(coord);

        // Write block
        storage.write_block(&block).await.expect("test: async operation");

        // Read by index
        let read_block = storage.read_block(BlockQuery::ByIndex(0)).await.expect("test: async operation");
        assert!(read_block.is_some());
        assert_eq!(read_block.expect("test: assertion value").hash, block.hash);

        // Read by hash
        let read_block = storage
            .read_block(BlockQuery::ByHash(block.hash.clone()))
            .await
            .expect("test: expected success");
        assert!(read_block.is_some());
        assert_eq!(read_block.expect("test: assertion value").index, 0);
    }

    #[tokio::test]
    async fn test_multiple_blocks() {
        use crate::assets::core::AssetRegistration;

        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let mut prev_block = Block::genesis(coord);
        storage.write_block(&prev_block).await.expect("test: async operation");

        // Write additional blocks (each must contain at least one asset)
        for i in 1..10 {
            let asset = AssetRegistration::genesis(coord);
            let mut block = Block::new(i, vec![asset], prev_block.hash.clone(), coord);
            block.hash = format!("hash_{i}"); // Simplified for testing
            storage.write_block(&block).await.expect("test: async operation");
            prev_block = block;
        }

        // Check metadata
        let metadata = storage.get_metadata().await;
        assert_eq!(metadata.total_blocks, 10);
        assert_eq!(metadata.chain_height, 9);

        // Read range
        let blocks = storage.read_range(0, 5).await.expect("test: async operation");
        assert_eq!(blocks.len(), 6);
    }

    #[tokio::test]
    async fn test_wal_replay() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");

        {
            let storage =
                BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                    .await
                    .expect("test: expected success");

            let block = Block::genesis(coord);
            storage.write_block(&block).await.expect("test: async operation");
        }

        // Create new storage instance and replay WAL
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let replayed = storage.replay_wal().await.expect("test: async operation");
        assert!(replayed > 0);

        // Verify block exists after replay
        let block = storage.read_block(BlockQuery::ByIndex(0)).await.expect("test: async operation");
        assert!(block.is_some());
    }

    #[tokio::test]
    async fn test_metadata_persistence() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");

        // Write some blocks
        {
            let storage =
                BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                    .await
                    .expect("test: expected success");

            for i in 0..5 {
                let mut block = Block::genesis(coord);
                block.index = i;
                block.hash = format!("hash_{i}");
                storage.write_block(&block).await.expect("test: async operation");
            }
        }

        // Load storage again and verify metadata
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let metadata = storage.get_metadata().await;
        assert_eq!(metadata.total_blocks, 5);
        assert_eq!(metadata.chain_height, 4);
    }

    #[tokio::test]
    async fn test_last_n_blocks_query() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");

        // Write 10 blocks
        for i in 0..10 {
            let mut block = Block::genesis(coord);
            block.index = i;
            block.hash = format!("hash_{i}");
            storage.write_block(&block).await.expect("test: async operation");
        }

        // Query last 3 blocks
        let block = storage.read_block(BlockQuery::Last(3)).await.expect("test: async operation");
        assert!(block.is_some());
        assert_eq!(block.expect("test: assertion value").index, 7);
    }

    #[tokio::test]
    async fn test_flush_wal() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let block = Block::genesis(coord);

        storage.write_block(&block).await.expect("test: async operation");
        storage.flush_wal().await.expect("test: async operation");

        // Verify WAL file exists
        let wal_path = temp_dir
            .path()
            .join("test_node")
            .join("blockchain")
            .join("wal.log");
        assert!(wal_path.exists());
    }
}
