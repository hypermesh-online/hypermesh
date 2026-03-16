// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Blockchain persistence with versioned format and integrity verification
//!
//! Provides efficient storage for per-node blockchains with append-only logs,
//! indexes, write-ahead logging for crash recovery, and tamper detection.
//!
//! ## Storage format (v1)
//!
//! Each block is stored with a header for version identification and integrity:
//! ```text
//! [4 bytes: magic "HMB\x01"]  -- HyperMesh Block format version 1
//! [4 bytes: payload_size as u32 LE]
//! [32 bytes: canonical_hash (raw BLAKE3 of block's canonical fields)]
//! [payload_size bytes: bincode-serialized Block]
//! ```
//!
//! SECURITY REVIEW REQUIRED: The canonical hash is verified on every read.
//! Block hashes are PERMANENT and format-independent. They are computed from
//! canonical fields only (index, prev_hash, entries[].asset_hash,
//! entries[].proof_hash). Blocks are NEVER re-hashed during format migration.
//! See papers/HYPERMESH.md Section 6.2 and Section 7.2.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use super::{PersistenceError, PersistenceResult};
use crate::blockchain::block::Block;
use crate::blockchain::chain::ChainStats;

/// Block file size threshold (1000 blocks per file)
const BLOCKS_PER_FILE: u64 = 1000;

// SECURITY REVIEW REQUIRED: Format version header definition.
// The magic bytes identify the storage format version. Any change to the
// serialization format MUST use a new magic value (e.g., "HMB\x02").
// See papers/HYPERMESH.md Section 6.2 for the block integrity model.
/// Magic bytes for HyperMesh Block format version 1
const BLOCK_MAGIC_V1: [u8; 4] = [b'H', b'M', b'B', 0x01];

/// Total header size: 4 (magic) + 4 (payload_size) + 32 (canonical_hash)
const BLOCK_HEADER_SIZE: usize = 40;

/// Compute the raw 32-byte BLAKE3 canonical hash from a block's hex hash string.
///
/// The block's `calculate_hash()` returns a hex-encoded BLAKE3 hash. We parse
/// that back to raw bytes for compact storage in the header.
fn canonical_hash_bytes(block: &Block) -> [u8; 32] {
    let hex_hash = block.calculate_hash();
    match blake3::Hash::from_hex(&hex_hash) {
        Ok(hash) => *hash.as_bytes(),
        Err(_) => {
            // Fallback: hash the hex string directly (should never happen with
            // valid BLAKE3 output, but we must not panic in production)
            *blake3::hash(hex_hash.as_bytes()).as_bytes()
        }
    }
}

/// Serialize a block with the v1 format header.
///
/// Returns the full byte sequence: magic + payload_size + canonical_hash + payload.
fn serialize_block_v1(block: &Block) -> PersistenceResult<Vec<u8>> {
    let payload = bincode::serialize(block)
        .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
    let payload_size = payload.len() as u32;
    let hash_bytes = canonical_hash_bytes(block);

    let mut buf = Vec::with_capacity(BLOCK_HEADER_SIZE + payload.len());
    buf.extend_from_slice(&BLOCK_MAGIC_V1);
    buf.extend_from_slice(&payload_size.to_le_bytes());
    buf.extend_from_slice(&hash_bytes);
    buf.extend_from_slice(&payload);
    Ok(buf)
}

/// Deserialize a block from a buffer, handling both v1 (with header) and
/// legacy (raw bincode) formats.
///
/// SECURITY REVIEW REQUIRED: This function verifies the canonical hash on
/// every read. If the stored hash does not match the computed hash, the block
/// is rejected with `IntegrityViolation`. An attacker who modifies persisted
/// data on disk will be detected here. Blocks are NEVER re-hashed.
/// See papers/HYPERMESH.md Section 6.2 and Section 7.2.
fn deserialize_block_verified(buffer: &[u8]) -> PersistenceResult<Block> {
    if buffer.len() >= BLOCK_HEADER_SIZE && buffer[..4] == BLOCK_MAGIC_V1 {
        // V1 format: parse header, deserialize payload, verify hash
        let payload_size =
            u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]) as usize;
        let stored_hash_bytes: [u8; 32] = buffer[8..40]
            .try_into()
            .map_err(|_| PersistenceError::Deserialization(
                "invalid canonical hash in header".to_string(),
            ))?;

        let payload_end = BLOCK_HEADER_SIZE + payload_size;
        if buffer.len() < payload_end {
            return Err(PersistenceError::Deserialization(format!(
                "truncated block: header says {} bytes but only {} available",
                payload_size,
                buffer.len() - BLOCK_HEADER_SIZE
            )));
        }

        let block: Block =
            bincode::deserialize(&buffer[BLOCK_HEADER_SIZE..payload_end])
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

        // Verify canonical hash
        let computed = canonical_hash_bytes(&block);
        if computed != stored_hash_bytes {
            let stored_hex = blake3::Hash::from_bytes(stored_hash_bytes).to_hex();
            let computed_hex = blake3::Hash::from_bytes(computed).to_hex();
            return Err(PersistenceError::IntegrityViolation {
                index: block.index,
                stored_hash: stored_hex.to_string(),
                computed_hash: computed_hex.to_string(),
            });
        }

        Ok(block)
    } else {
        // SECURITY REVIEW REQUIRED: Legacy format detection.
        // Old data written without a header is raw bincode. We still verify
        // the block's stored hash matches its canonical hash to detect
        // tampering. An attacker who modifies legacy blocks will be caught
        // here because the hash field inside the Block struct was set at
        // creation time and is compared against a fresh calculate_hash().
        // See papers/HYPERMESH.md Section 7.2.
        let block: Block = bincode::deserialize(buffer)
            .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

        let computed = block.calculate_hash();
        if computed != block.hash {
            return Err(PersistenceError::IntegrityViolation {
                index: block.index,
                stored_hash: block.hash.clone(),
                computed_hash: computed,
            });
        }

        Ok(block)
    }
}

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

        // Write additional blocks (each must contain at least one entry)
        for i in 1..10 {
            use crate::blockchain::block::{BlockAssetEntry, StoragePointer};
            use trustchain::proof_of_state::StateProof;
            let reg = AssetRegistration::genesis(coord);
            let content_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
            let state_proof = StateProof::default();
            let proof_bytes = serde_json::to_vec(&state_proof).unwrap_or_default();
            let proof_hash = *blake3::hash(&proof_bytes).as_bytes();
            let entry = BlockAssetEntry {
                asset_hash: content_hash,
                proof_hash,
                state_proof,
                storage_pointer: StoragePointer::Genesis,
                registration: reg,
            };
            let mut block = Block::new(i, vec![entry], prev_block.hash.clone());
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
    async fn test_wal_replay_crash_recovery() {
        // Simulate a crash: WAL has an entry but storage was never written.
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let block = Block::genesis(coord);

        // Set up storage directory and write a WAL entry directly (simulating
        // a crash between WAL write and storage commit).
        let blockchain_dir = temp_dir
            .path()
            .join("test_node")
            .join("blockchain");
        std::fs::create_dir_all(blockchain_dir.join("blocks"))
            .expect("test: create dirs");

        let wal_path = blockchain_dir.join("wal.log");
        {
            let mut wal_writer = WalWriter::new(wal_path).expect("test: wal writer");
            wal_writer
                .write_entry(WalEntry {
                    op_type: WalOperation::AddBlock,
                    block: block.clone(),
                    timestamp: chrono::Utc::now(),
                })
                .expect("test: wal write");
        }

        // Open storage (block NOT in index) and replay WAL
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let replayed = storage.replay_wal().await.expect("test: async operation");
        assert_eq!(replayed, 1, "should replay the one missing block");

        // Verify block is now in storage
        let read = storage
            .read_block(BlockQuery::ByIndex(0))
            .await
            .expect("test: async operation");
        assert!(read.is_some());
        assert_eq!(read.expect("test: assertion value").hash, block.hash);

        // Verify WAL is truncated after replay
        let wal_path = blockchain_dir.join("wal.log");
        let wal_size = std::fs::metadata(&wal_path)
            .expect("test: wal metadata")
            .len();
        assert_eq!(wal_size, 0, "WAL should be truncated after replay");
    }

    #[tokio::test]
    async fn test_wal_replay_skips_already_persisted() {
        // Write a block normally (WAL truncated after commit), then manually
        // re-add the same block to WAL. Replay should skip it.
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let block = Block::genesis(coord);

        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        // Write block normally (commits to storage + truncates WAL)
        storage
            .write_block(&block)
            .await
            .expect("test: async operation");

        // Manually append the same block to WAL (simulates stale WAL from old code)
        {
            let mut wal = storage.wal.write().await;
            if let Some(w) = wal.as_mut() {
                w.write_entry(WalEntry {
                    op_type: WalOperation::AddBlock,
                    block: block.clone(),
                    timestamp: chrono::Utc::now(),
                })
                .expect("test: wal write");
            }
        }

        // Replay should skip the already-persisted block
        let replayed = storage.replay_wal().await.expect("test: async operation");
        assert_eq!(replayed, 0, "should skip already-persisted block");
    }

    #[tokio::test]
    async fn test_wal_truncated_after_write() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let block = Block::genesis(coord);

        storage
            .write_block(&block)
            .await
            .expect("test: async operation");

        // WAL should be truncated (0 bytes) after successful write
        let wal_path = temp_dir
            .path()
            .join("test_node")
            .join("blockchain")
            .join("wal.log");
        let wal_size = std::fs::metadata(&wal_path)
            .expect("test: wal metadata")
            .len();
        assert_eq!(wal_size, 0, "WAL should be empty after committed write");
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

    // --- New tests for versioned persistence and integrity verification ---

    #[tokio::test]
    async fn test_v1_write_and_read_integrity() {
        // Write a block with v1 format, read it back, verify integrity passes
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let coord = MatrixCoordinate::new(5, 10, 15).expect("test: valid coordinate");
        let block = Block::genesis(coord);

        // Verify the genesis block has a valid hash
        assert!(block.verify_hash(), "genesis block hash should be valid");

        storage.write_block(&block).await.expect("test: write block");

        // Read back and verify integrity check passed (no IntegrityViolation)
        let read = storage
            .read_block(BlockQuery::ByIndex(0))
            .await
            .expect("test: read block should succeed with integrity check");
        let read_block = read.expect("test: block should exist");
        assert_eq!(read_block.hash, block.hash);
        assert_eq!(read_block.index, block.index);
        assert_eq!(read_block.entries.len(), block.entries.len());
    }

    #[tokio::test]
    async fn test_tampered_block_detected() {
        // Write a block, then tamper with the on-disk payload. Read should
        // return IntegrityViolation.
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let block = Block::genesis(coord);
        assert!(block.verify_hash(), "genesis block hash should be valid");

        storage.write_block(&block).await.expect("test: write block");

        // Get the location of the block on disk
        let (file_id, offset, size) = {
            let idx = storage.block_index.read().await;
            idx.get(&block.hash).cloned().expect("test: block in index")
        };

        // Tamper with a byte in the payload (past the header)
        let file_path = temp_dir
            .path()
            .join("test_node")
            .join("blockchain")
            .join("blocks")
            .join(format!("{file_id:08}.blk"));

        {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&file_path)
                .expect("test: open file");
            // Flip a byte in the payload area (after the 40-byte header)
            let tamper_offset = offset + BLOCK_HEADER_SIZE as u64 + 5;
            file.seek(SeekFrom::Start(tamper_offset))
                .expect("test: seek");
            // Read current byte, flip it
            let mut byte = [0u8; 1];
            file.read_exact(&mut byte).expect("test: read byte");
            byte[0] ^= 0xFF;
            file.seek(SeekFrom::Start(tamper_offset))
                .expect("test: seek back");
            file.write_all(&byte).expect("test: write tampered byte");
        }

        // Reading should fail with IntegrityViolation or Deserialization.
        // Tampering a payload byte may corrupt bincode structure (Deserialization)
        // or produce a deserializable block with wrong fields (IntegrityViolation).
        // In either case the block is rejected -- never silently accepted.
        let result = storage.read_block(BlockQuery::ByIndex(0)).await;
        match result {
            Err(PersistenceError::IntegrityViolation { .. }) => {
                // Tampered block detected via hash mismatch
            }
            Err(PersistenceError::Deserialization(_)) => {
                // Tampered byte broke bincode structure
            }
            other => unreachable!(
                "expected IntegrityViolation or Deserialization, got: {:?}",
                other.map(|o| format!("{:?}", o))
            ),
        }
    }

    #[tokio::test]
    async fn test_legacy_format_read() {
        // Write raw bincode (no header) directly to a .blk file, then read
        // it through the storage layer. Should succeed for a valid block
        // (legacy compat) because the hash matches.
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let block = Block::genesis(coord);
        assert!(block.verify_hash(), "genesis block hash should be valid");

        // Write raw bincode directly (legacy format, no header)
        let file_id = 0u32;
        let file_path = temp_dir
            .path()
            .join("test_node")
            .join("blockchain")
            .join("blocks")
            .join(format!("{file_id:08}.blk"));

        let serialized = bincode::serialize(&block).expect("test: serialize");
        std::fs::write(&file_path, &serialized).expect("test: write legacy file");

        // Manually insert into index so read_block can find it
        {
            let mut block_index = storage.block_index.write().await;
            let mut index_map = storage.index_map.write().await;
            block_index.insert(
                block.hash.clone(),
                (file_id, 0, serialized.len() as u32),
            );
            index_map.insert(block.index, block.hash.clone());
        }

        // Read should succeed via legacy path with hash verification
        let read = storage
            .read_block(BlockQuery::ByIndex(0))
            .await
            .expect("test: legacy read should succeed");
        let read_block = read.expect("test: block should exist");
        assert_eq!(read_block.hash, block.hash);
    }

    #[tokio::test]
    async fn test_legacy_tampered_block_detected() {
        // Write raw bincode (legacy) with a tampered hash field inside the
        // Block struct. The hash won't match calculate_hash() -> IntegrityViolation.
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let mut block = Block::genesis(coord);
        let original_hash = block.hash.clone();
        // Tamper with the block's hash field
        block.hash = "tampered_hash_value_that_does_not_match".to_string();

        // Write raw bincode (legacy format)
        let file_id = 0u32;
        let file_path = temp_dir
            .path()
            .join("test_node")
            .join("blockchain")
            .join("blocks")
            .join(format!("{file_id:08}.blk"));

        let serialized = bincode::serialize(&block).expect("test: serialize");
        std::fs::write(&file_path, &serialized).expect("test: write legacy file");

        // Insert with tampered hash so we can look it up
        {
            let mut block_index = storage.block_index.write().await;
            let mut index_map = storage.index_map.write().await;
            block_index.insert(
                block.hash.clone(),
                (file_id, 0, serialized.len() as u32),
            );
            index_map.insert(block.index, block.hash.clone());
        }

        // Read should fail with IntegrityViolation
        let result = storage.read_block(BlockQuery::ByIndex(0)).await;
        match result {
            Err(PersistenceError::IntegrityViolation {
                index,
                stored_hash,
                computed_hash,
            }) => {
                assert_eq!(index, 0);
                assert_eq!(stored_hash, "tampered_hash_value_that_does_not_match");
                // computed_hash should be the real hash
                assert_eq!(computed_hash, original_hash);
            }
            other => unreachable!(
                "expected IntegrityViolation, got: {:?}",
                other.map(|o| format!("{:?}", o))
            ),
        }
    }

    #[tokio::test]
    async fn test_wal_replay_integrity_check() {
        // Write a WAL entry with a tampered block. Replay should skip it.
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let mut block = Block::genesis(coord);
        // Tamper: set hash to something that won't match calculate_hash()
        block.hash = "wal_tampered_hash".to_string();

        let blockchain_dir = temp_dir
            .path()
            .join("test_node")
            .join("blockchain");
        std::fs::create_dir_all(blockchain_dir.join("blocks"))
            .expect("test: create dirs");

        let wal_path = blockchain_dir.join("wal.log");
        {
            let mut wal_writer = WalWriter::new(wal_path).expect("test: wal writer");
            wal_writer
                .write_entry(WalEntry {
                    op_type: WalOperation::AddBlock,
                    block: block.clone(),
                    timestamp: chrono::Utc::now(),
                })
                .expect("test: wal write");
        }

        let storage =
            BlockchainStorage::new(temp_dir.path().to_path_buf(), "test_node".to_string())
                .await
                .expect("test: expected success");

        // Replay should skip the tampered entry (0 replayed)
        let replayed = storage.replay_wal().await.expect("test: replay should succeed");
        assert_eq!(replayed, 0, "tampered WAL entry should be skipped");

        // Verify the block was NOT written to storage
        let read = storage.read_block(BlockQuery::ByIndex(0)).await.expect("test: read");
        assert!(read.is_none(), "tampered block should not be in storage");
    }

    #[test]
    fn test_v1_header_structure() {
        // Verify the v1 serialization produces the expected header layout
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let block = Block::genesis(coord);

        let serialized = serialize_block_v1(&block).expect("test: serialize v1");

        // Check magic bytes
        assert_eq!(&serialized[0..4], &BLOCK_MAGIC_V1);

        // Check payload size
        let payload_size =
            u32::from_le_bytes([serialized[4], serialized[5], serialized[6], serialized[7]])
                as usize;
        assert_eq!(serialized.len(), BLOCK_HEADER_SIZE + payload_size);

        // Check canonical hash matches block's hash
        let stored_hash: [u8; 32] = serialized[8..40]
            .try_into()
            .expect("test: hash slice");
        let expected = canonical_hash_bytes(&block);
        assert_eq!(stored_hash, expected);
    }

    #[test]
    fn test_deserialize_v1_round_trip() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let block = Block::genesis(coord);

        let serialized = serialize_block_v1(&block).expect("test: serialize");
        let deserialized =
            deserialize_block_verified(&serialized).expect("test: deserialize");

        assert_eq!(deserialized.hash, block.hash);
        assert_eq!(deserialized.index, block.index);
        assert_eq!(deserialized.previous_hash, block.previous_hash);
    }

    #[test]
    fn test_deserialize_legacy_round_trip() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let block = Block::genesis(coord);

        // Raw bincode (legacy format)
        let serialized = bincode::serialize(&block).expect("test: serialize");
        let deserialized =
            deserialize_block_verified(&serialized).expect("test: deserialize legacy");

        assert_eq!(deserialized.hash, block.hash);
        assert_eq!(deserialized.index, block.index);
    }
}
