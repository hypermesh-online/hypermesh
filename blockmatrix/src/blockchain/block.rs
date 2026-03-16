// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Block implementation for every-node-blockchain architecture
//!
//! Each block belongs to a specific node's independent blockchain.
//! NO merkle tree consolidation across nodes - fundamental design principle.
//!
//! Block structure:
//! ```text
//! Block_i = {
//!     prev_hash,
//!     entries: [
//!         { hA, hπ, state_proof, ptr },
//!         ...
//!     ]
//! }
//! block_hash_i = BLAKE3(Block_i)
//! ```
//!
//! - `hA` = BLAKE3(Brotli(Asset)) — content hash of the compressed asset
//! - `hπ` = BLAKE3(StateProof) — proof integrity hash
//! - `state_proof` — the full four-proof authentication (WHO/WHEN/WHERE/WHAT)
//! - `ptr` — storage pointer (local path or shard placements)
//!
//! Timestamp and node coordinate are NOT block fields — they live inside
//! the state proof (PoTime = WHEN, PoSpace = WHERE).
//!
//! Ledger secures integrity. Storage layer holds data.

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::assets::core::AssetRegistration;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::proof_of_state::genesis_proof::{generate_genesis_proof, HardwareAssessment};
use trustchain::proof_of_state::StateProof;

/// Where the actual asset data lives (the block only stores a pointer).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum StoragePointer {
    /// Asset data stored locally at this path
    Local { path: String },
    /// Asset data sharded across matrix positions
    Sharded {
        /// BLAKE3 hash of each shard
        shard_hashes: Vec<[u8; 32]>,
        /// Matrix positions where shards are placed
        placements: Vec<MatrixCoordinate>,
    },
    /// Genesis assets — no external storage, the registration IS the data
    Genesis,
}

/// A single asset entry within a block.
///
/// Each entry is self-contained: content hash, proof, and storage pointer.
/// Assets within a block can reference each other by content hash (hA).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BlockAssetEntry {
    /// hA = BLAKE3(Brotli(Asset)) — content address of the compressed asset
    pub asset_hash: [u8; 32],

    /// hπ = BLAKE3(StateProof) — integrity hash of the proof
    pub proof_hash: [u8; 32],

    /// The full state proof (PoStake/PoTime/PoSpace/PoWork)
    pub state_proof: StateProof,

    /// Where the actual data lives
    pub storage_pointer: StoragePointer,

    /// Asset registration metadata (category, network scope, etc.)
    pub registration: AssetRegistration,
}

/// Lightweight block header for chain integrity verification.
///
/// Nodes store headers for blocks they don't fully participate in,
/// enabling selective chain reconstruction without full block data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    /// Block index in the chain
    pub index: u64,
    /// This block's hash (BLAKE3 hex)
    pub hash: String,
    /// Hash of the previous block (BLAKE3 hex)
    pub previous_hash: String,
    /// BLAKE3 hash of the serialized entries, proving header matches block content.
    pub entries_hash: [u8; 32],
    /// Number of asset entries in the block
    pub entry_count: usize,
}

impl BlockHeader {
    /// Verify that this header chains to the given previous header.
    pub fn chains_to(&self, previous: &BlockHeader) -> bool {
        self.previous_hash == previous.hash && self.index == previous.index + 1
    }
}

/// A block in a node's independent blockchain.
///
/// The block is purely: hash linkage + asset entries.
/// All metadata (timestamp, location) lives in the state proofs.
/// Same content = same hash = same block. No nonce, no timestamp on the block itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    /// Block index in this node's chain
    pub index: u64,

    /// Hash of the previous block in THIS node's chain (BLAKE3 hex)
    pub previous_hash: String,

    /// This block's hash (BLAKE3 hex)
    pub hash: String,

    /// Asset entries: each contains { hA, hπ, state_proof, ptr, registration }
    pub entries: Vec<BlockAssetEntry>,
}

impl Block {
    /// Create a new block from asset entries.
    pub fn new(
        index: u64,
        entries: Vec<BlockAssetEntry>,
        previous_hash: String,
    ) -> Self {
        assert!(!entries.is_empty(), "Block must contain at least one entry");

        let mut block = Block {
            index,
            previous_hash,
            hash: String::new(),
            entries,
        };

        block.hash = block.calculate_hash();
        block
    }

    /// Create the genesis block for a node.
    ///
    /// Genesis entries use `StoragePointer::Genesis` and a hardware-assessed
    /// StateProof (self-authorized — sovereignty from boot).
    ///
    /// Per R1: hardware assessed, not self-reported.
    /// Per section 8.2: "Usage IS verification."
    pub fn genesis(node_coordinate: MatrixCoordinate) -> Self {
        let genesis_reg = AssetRegistration::genesis(node_coordinate);
        let content_hash = {
            let serialized = genesis_reg.to_string();
            *blake3::hash(serialized.as_bytes()).as_bytes()
        };

        let state_proof = Self::build_genesis_proof(node_coordinate);
        let proof_bytes = serde_json::to_vec(&state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();

        let genesis_entry = BlockAssetEntry {
            asset_hash: content_hash,
            proof_hash,
            state_proof,
            storage_pointer: StoragePointer::Genesis,
            registration: genesis_reg,
        };

        Block::new(
            0,
            vec![genesis_entry],
            String::from("0000000000000000000000000000000000000000000000000000000000000000"),
        )
    }

    /// Build a StateProof for the genesis block from real hardware.
    ///
    /// Attempts OS hardware detection; falls back to safe defaults that
    /// still satisfy `StateRequirements::default()` for R13-compliant devices.
    fn build_genesis_proof(coordinate: MatrixCoordinate) -> StateProof {
        let node_id = format!(
            "genesis_({},{},{})",
            coordinate.x, coordinate.y, coordinate.z
        );
        match crate::create_os_abstraction() {
            Ok(os) => {
                let hw = HardwareAssessment::from_os(os.as_ref(), &node_id, coordinate);
                generate_genesis_proof(&hw)
            }
            Err(_) => {
                // Fallback: use num_cpus and conservative estimates
                let hw = HardwareAssessment {
                    cpu_cores: num_cpus::get() as u32,
                    cpu_mhz: 1000,
                    memory_bytes: 4 * 1024 * 1024 * 1024,
                    storage_bytes: 50 * 1024 * 1024 * 1024,
                    storage_available_bytes: 25 * 1024 * 1024 * 1024,
                    node_id,
                    coordinate,
                };
                generate_genesis_proof(&hw)
            }
        }
    }

    /// Compute the BLAKE3 hash of all entries (deterministic commitment).
    ///
    /// Hashes the concatenation of `(asset_hash || proof_hash)` for each entry.
    /// This is deterministic regardless of serialization format.
    pub fn compute_entries_hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        for entry in &self.entries {
            hasher.update(&entry.asset_hash);
            hasher.update(&entry.proof_hash);
        }
        *hasher.finalize().as_bytes()
    }

    /// Extract a lightweight header from this block.
    pub fn header(&self) -> BlockHeader {
        BlockHeader {
            index: self.index,
            hash: self.hash.clone(),
            previous_hash: self.previous_hash.clone(),
            entries_hash: self.compute_entries_hash(),
            entry_count: self.entries.len(),
        }
    }

    /// Verify that this block matches a given header.
    pub fn verify_against_header(&self, header: &BlockHeader) -> bool {
        self.index == header.index
            && self.hash == header.hash
            && self.previous_hash == header.previous_hash
            && self.entries.len() == header.entry_count
            && self.compute_entries_hash() == header.entries_hash
    }

    /// Calculate the hash of this block using BLAKE3.
    ///
    /// `block_hash = BLAKE3(index || prev_hash || entries...)`
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Hasher::new();

        hasher.update(&self.index.to_le_bytes());
        hasher.update(self.previous_hash.as_bytes());

        for entry in &self.entries {
            hasher.update(&entry.asset_hash);
            hasher.update(&entry.proof_hash);
        }

        format!("{}", hasher.finalize())
    }

    /// Verify the block's hash is correct
    pub fn verify_hash(&self) -> bool {
        self.hash == self.calculate_hash()
    }

    /// Check if this is a genesis block
    pub fn is_genesis(&self) -> bool {
        self.index == 0
            && self.previous_hash
                == "0000000000000000000000000000000000000000000000000000000000000000"
    }

    /// Get the block size in bytes (estimate)
    pub fn size(&self) -> usize {
        8 + // index
        64 + // previous_hash
        64 + // hash
        self.entries.len() * (32 + 32 + 256 + 64) // entry estimate (hA + hπ + proof + ptr)
    }

    /// Get asset registrations from entries (compatibility helper)
    pub fn get_assets(&self) -> Vec<&AssetRegistration> {
        self.entries.iter().map(|e| &e.registration).collect()
    }

    /// Get the number of asset entries
    pub fn asset_count(&self) -> usize {
        self.entries.len()
    }

    /// Check if this block belongs to the specified node.
    ///
    /// Checks the first entry's PoSpace proof `node_id` which encodes
    /// the node coordinate as `"(x,y,z)"`.
    pub fn belongs_to_node(&self, node_coordinate: &MatrixCoordinate) -> bool {
        if let Some(first) = self.entries.first() {
            let coord_str = format!(
                "({},{},{})",
                node_coordinate.x, node_coordinate.y, node_coordinate.z
            );
            first.state_proof.space_proof.node_id == coord_str
        } else {
            false
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Block #{} | {} entries | Hash: {}...{}",
            self.index,
            self.entries.len(),
            &self.hash[..8.min(self.hash.len())],
            &self.hash[self.hash.len().saturating_sub(8)..]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_genesis_entry(coord: MatrixCoordinate) -> BlockAssetEntry {
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

    #[test]
    fn test_genesis_block_creation() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let genesis = Block::genesis(coord);

        assert_eq!(genesis.index, 0);
        assert!(genesis.is_genesis());
        assert!(genesis.verify_hash());
        assert_eq!(genesis.asset_count(), 1);
    }

    #[test]
    fn test_block_creation() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coord");
        let entry = test_genesis_entry(coord);
        let prev_hash = "abc123".to_string();

        let block = Block::new(1, vec![entry.clone()], prev_hash.clone());

        assert_eq!(block.index, 1);
        assert_eq!(block.entries.len(), 1);
        assert_eq!(block.entries[0], entry);
        assert_eq!(block.previous_hash, prev_hash);
        assert!(!block.hash.is_empty());
        assert!(block.verify_hash());
    }

    #[test]
    fn test_hash_verification() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let entry = test_genesis_entry(coord);
        let mut block = Block::new(1, vec![entry], "prev".to_string());

        assert!(block.verify_hash());

        // Tamper with an entry's asset hash
        block.entries[0].asset_hash = [0xFFu8; 32];
        assert!(!block.verify_hash());

        // Fix the hash
        block.hash = block.calculate_hash();
        assert!(block.verify_hash());
    }

    #[test]
    fn test_deterministic_hash() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let entry = test_genesis_entry(coord);

        // Same content = same hash (no nonce, no timestamp on block)
        let block1 = Block::new(1, vec![entry.clone()], "prev".to_string());
        let block2 = Block::new(1, vec![entry.clone()], "prev".to_string());

        assert_eq!(block1.hash, block2.hash);
    }

    #[test]
    fn test_block_size() {
        let entries: Vec<BlockAssetEntry> = (0..10)
            .map(|i| {
                test_genesis_entry(
                    MatrixCoordinate::new(i, i, i).expect("test: valid coord"),
                )
            })
            .collect();

        let block = Block::new(100, entries, "x".repeat(64));

        let size = block.size();
        assert!(size >= 320);
    }

    #[test]
    fn test_block_display() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coord");
        let block = Block::genesis(coord);

        let display = format!("{block}");
        assert!(display.contains("Block #0"));
        assert!(display.contains("1 entries"));
    }

    #[test]
    fn test_serialization() {
        let coord = MatrixCoordinate::new(7, 8, 9).expect("test: valid coord");
        let block = Block::genesis(coord);

        let json = serde_json::to_string(&block).expect("test: serialize");
        assert!(json.contains("\"index\":0"));

        let decoded: Block = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(block, decoded);
    }

    #[test]
    fn test_genesis_block_properties() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let genesis = Block::genesis(coord);

        assert_eq!(genesis.index, 0);
        assert!(genesis.is_genesis());
        assert_eq!(
            genesis.previous_hash,
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(genesis.asset_count(), 1);
        assert!(genesis.verify_hash());
    }

    #[test]
    fn test_block_must_have_entries() {
        let result = std::panic::catch_unwind(|| Block::new(1, vec![], "prev".to_string()));
        assert!(
            result.is_err(),
            "Block creation with empty entries should panic"
        );
    }

    #[test]
    fn test_multiple_entries() {
        let entries: Vec<BlockAssetEntry> = (0..5)
            .map(|i| {
                test_genesis_entry(
                    MatrixCoordinate::new(i, i, i).expect("test: valid coord"),
                )
            })
            .collect();

        let block = Block::new(1, entries, "prev".to_string());

        assert_eq!(block.asset_count(), 5);
        assert_eq!(block.get_assets().len(), 5);
    }

    // --- BlockHeader tests ---

    #[test]
    fn test_block_header_round_trip() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let entry = test_genesis_entry(coord);
        let block = Block::new(1, vec![entry], "prev".to_string());

        let header = block.header();
        assert!(block.verify_against_header(&header));
    }

    #[test]
    fn test_block_header_verify_fails_different_entries() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let entry1 = test_genesis_entry(coord);
        let block = Block::new(1, vec![entry1], "prev".to_string());
        let header = block.header();

        // Build a different block with same index/prev but different entry
        let coord2 = MatrixCoordinate::new(4, 5, 6).expect("test: valid coord");
        let entry2 = test_genesis_entry(coord2);
        let block2 = Block::new(1, vec![entry2], "prev".to_string());

        assert!(!block2.verify_against_header(&header));
    }

    #[test]
    fn test_block_header_chains_to_sequential() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let entry = test_genesis_entry(coord);

        let block0 = Block::new(0, vec![entry.clone()], "genesis".to_string());
        let block1 = Block::new(1, vec![entry], block0.hash.clone());

        let h0 = block0.header();
        let h1 = block1.header();

        assert!(h1.chains_to(&h0));
    }

    #[test]
    fn test_block_header_chains_to_fails_non_sequential() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let entry = test_genesis_entry(coord);

        let block0 = Block::new(0, vec![entry.clone()], "genesis".to_string());
        let block2 = Block::new(2, vec![entry], block0.hash.clone());

        let h0 = block0.header();
        let h2 = block2.header();

        // Index gap: 2 != 0 + 1
        assert!(!h2.chains_to(&h0));
    }

    #[test]
    fn test_block_header_chains_to_fails_wrong_hash() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let entry = test_genesis_entry(coord);

        let block0 = Block::new(0, vec![entry.clone()], "genesis".to_string());
        let block1 = Block::new(1, vec![entry], "wrong_prev_hash".to_string());

        let h0 = block0.header();
        let h1 = block1.header();

        assert!(!h1.chains_to(&h0));
    }

    #[test]
    fn test_block_header_entries_hash_deterministic() {
        let coord = MatrixCoordinate::new(3, 3, 3).expect("test: valid coord");
        let entry = test_genesis_entry(coord);

        let block = Block::new(1, vec![entry], "prev".to_string());
        let hash1 = block.compute_entries_hash();
        let hash2 = block.compute_entries_hash();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_block_header_genesis_block() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let genesis = Block::genesis(coord);
        let header = genesis.header();

        assert_eq!(header.index, 0);
        assert_eq!(header.entry_count, 1);
        assert_eq!(header.hash, genesis.hash);
        assert!(genesis.verify_against_header(&header));
    }

    #[test]
    fn test_storage_pointer_variants() {
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coord");
        let mut entry = test_genesis_entry(coord);

        // Test Local pointer
        entry.storage_pointer = StoragePointer::Local {
            path: "/data/assets/abc123".to_string(),
        };
        let block = Block::new(1, vec![entry.clone()], "prev".to_string());
        assert!(block.verify_hash());

        // Test Sharded pointer
        entry.storage_pointer = StoragePointer::Sharded {
            shard_hashes: vec![[1u8; 32], [2u8; 32]],
            placements: vec![
                MatrixCoordinate::new(1, 0, 0).expect("test: valid"),
                MatrixCoordinate::new(0, 1, 0).expect("test: valid"),
            ],
        };
        let block2 = Block::new(2, vec![entry], "prev2".to_string());
        assert!(block2.verify_hash());
    }
}
