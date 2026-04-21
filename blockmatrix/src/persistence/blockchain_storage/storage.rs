// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! `BlockchainStorage` — public API for per-node blockchain storage.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::blockchain::block::Block;
use crate::blockchain::chain::ChainStats;

use super::super::{PersistenceError, PersistenceResult};
use super::format::{deserialize_block_verified, serialize_block_v1};
use super::metadata::{BlockQuery, ChainMetadata};
use super::wal::{WalEntry, WalOperation, WalReader, WalWriter};

/// Block file size threshold (1000 blocks per file)
const BLOCKS_PER_FILE: u64 = 1000;

/// Manages blockchain storage for a single node
pub struct BlockchainStorage {
    /// Storage directory
    pub(super) storage_dir: PathBuf,
    /// Node ID
    pub(super) _node_id: String,
    /// Block index: hash -> (file_id, offset, size)
    pub(super) block_index: Arc<RwLock<HashMap<String, (u32, u64, u32)>>>,
    /// Index by block number: index -> hash
    pub(super) index_map: Arc<RwLock<HashMap<u64, String>>>,
    /// Chain metadata
    pub(super) metadata: Arc<RwLock<ChainMetadata>>,
    /// Write-ahead log
    pub(super) wal: Arc<RwLock<Option<WalWriter>>>,
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

    /// Write a block to storage (WAL + storage files)
    pub async fn write_block(&self, block: &Block) -> PersistenceResult<()> {
        // Write to WAL first for crash recovery
        if let Some(wal) = self.wal.write().await.as_mut() {
            wal.write_entry(WalEntry {
                op_type: WalOperation::AddBlock,
                block: block.clone(),
                timestamp: chrono::Utc::now(),
            })?;
        }

        // Write to storage files
        self.write_block_to_storage(block).await?;

        // WAL entry is no longer needed -- data is committed to storage
        self.truncate_wal().await?;

        Ok(())
    }

    /// Write block to storage files without WAL logging.
    ///
    /// Used internally by `write_block` (after WAL write) and by `replay_wal`
    /// (where re-logging to WAL would be circular).
    ///
    /// Writes the block in v1 format with a header containing magic bytes,
    /// payload size, and canonical BLAKE3 hash for integrity verification.
    async fn write_block_to_storage(&self, block: &Block) -> PersistenceResult<()> {
        let file_id = (block.index / BLOCKS_PER_FILE) as u32;
        let file_path = self
            .storage_dir
            .join("blocks")
            .join(format!("{file_id:08}.blk"));

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        let offset = file.seek(SeekFrom::End(0))?;
        let serialized = serialize_block_v1(block)?;

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

    /// Truncate the WAL after blocks have been committed to storage.
    ///
    /// Drops the current WAL writer, truncates the file to zero bytes,
    /// then re-opens a fresh WAL writer.
    async fn truncate_wal(&self) -> PersistenceResult<()> {
        let wal_path = self.storage_dir.join("wal.log");
        let mut wal = self.wal.write().await;
        // Drop existing writer so the file handle is released
        *wal = None;
        // Truncate file to zero bytes
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&wal_path)?;
        // Re-open fresh WAL writer
        *wal = Some(WalWriter::new(wal_path)?);
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

    /// Read block by hash with integrity verification.
    ///
    /// SECURITY REVIEW REQUIRED: Every block read from disk is verified
    /// against its canonical hash. Both v1 (header) and legacy (raw bincode)
    /// formats are supported. Tampered blocks produce `IntegrityViolation`.
    /// See papers/HYPERMESH.md Section 6.2 and Section 7.2.
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

            let block = deserialize_block_verified(&buffer)?;

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
            total_data_size: 0, // Would need to sum from index
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

    /// Replay WAL entries for crash recovery.
    ///
    /// Skips blocks already present in the index (committed before crash)
    /// and only writes blocks that are genuinely missing from storage.
    /// Truncates the WAL after successful replay.
    ///
    /// SECURITY REVIEW REQUIRED: WAL replay integrity check.
    /// Each deserialized block's canonical hash is verified against block.hash.
    /// If they do not match, the entry is logged and skipped -- corrupted or
    /// tampered WAL entries must never propagate into storage.
    /// See papers/HYPERMESH.md Section 7.2.
    pub async fn replay_wal(&self) -> PersistenceResult<u32> {
        let wal_path = self.storage_dir.join("wal.log");
        if !wal_path.exists() {
            return Ok(0);
        }

        info!("Replaying WAL from {:?}", wal_path);
        let entries = WalReader::read_all(&wal_path)?;

        if entries.is_empty() {
            return Ok(0);
        }

        let total = entries.len() as u32;
        let mut replayed = 0u32;

        for entry in &entries {
            if entry.op_type == WalOperation::AddBlock {
                // SECURITY REVIEW REQUIRED: WAL replay integrity check.
                // Verify the block's canonical hash matches its stored hash
                // before committing to storage. A tampered WAL entry will
                // have a mismatched hash and must be rejected.
                let computed = entry.block.calculate_hash();
                if computed != entry.block.hash {
                    error!(
                        "WAL replay: block {} integrity violation \
                         (stored={}, computed={}) -- skipping corrupted entry, \
                         SECURITY REVIEW REQUIRED",
                        entry.block.index, entry.block.hash, computed
                    );
                    continue;
                }

                // Skip blocks already committed to storage
                let already_exists = self
                    .block_index
                    .read()
                    .await
                    .contains_key(&entry.block.hash);
                if already_exists {
                    info!(
                        "WAL replay: block {} already in storage, skipping",
                        entry.block.index
                    );
                    continue;
                }

                self.write_block_to_storage(&entry.block).await?;
                replayed += 1;
            }
        }

        // Truncate WAL -- all entries are now committed
        self.truncate_wal().await?;

        info!(
            "Replayed {} WAL entries ({} skipped, already in storage)",
            replayed,
            total - replayed
        );
        Ok(replayed)
    }

    /// Flush WAL to storage
    pub async fn flush_wal(&self) -> PersistenceResult<()> {
        if let Some(wal) = self.wal.write().await.as_mut() {
            wal.flush()?;
        }
        Ok(())
    }
}
