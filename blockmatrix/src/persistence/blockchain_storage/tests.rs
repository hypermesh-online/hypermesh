// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Tests for blockchain storage (format + WAL + integrity).

#![cfg(test)]

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};

use crate::blockchain::block::Block;
use crate::matrix::coordinate::MatrixCoordinate;
use tempfile::TempDir;

use super::super::PersistenceError;
use super::format::{
    canonical_hash_bytes, deserialize_block_verified, serialize_block_v1, BLOCK_HEADER_SIZE,
    BLOCK_MAGIC_V1,
};
use super::metadata::BlockQuery;
use super::storage::BlockchainStorage;
use super::wal::{WalEntry, WalOperation, WalWriter};

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
            signed_proof: None,
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
    let (file_id, offset, _size) = {
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
