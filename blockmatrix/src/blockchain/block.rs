//! Block implementation for every-node-blockchain architecture
//!
//! Each block belongs to a specific node's independent blockchain.
//! NO merkle tree consolidation across nodes - fundamental design principle.

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::matrix::coordinate::MatrixCoordinate;
use rand;

/// A block in a node's independent blockchain
///
/// Each node maintains its own blockchain without cross-node merkle consolidation.
/// This is a revolutionary architecture where every node is sovereign.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    /// Block index in this node's chain
    pub index: u64,

    /// Timestamp of block creation
    pub timestamp: DateTime<Utc>,

    /// Block data (transactions, state changes, etc.)
    pub data: Vec<u8>,

    /// Hash of the previous block in THIS node's chain
    pub previous_hash: String,

    /// This block's hash (BLAKE3)
    pub hash: String,

    /// The node's matrix coordinate that owns this block
    pub node_coordinate: MatrixCoordinate,

    /// Node signature (placeholder for now)
    pub node_signature: Vec<u8>,

    /// Block nonce for additional entropy
    pub nonce: u64,
}

impl Block {
    /// Create a new block
    pub fn new(
        index: u64,
        data: Vec<u8>,
        previous_hash: String,
        node_coordinate: MatrixCoordinate,
    ) -> Self {
        let timestamp = Utc::now();
        let nonce = rand::random::<u64>();

        let mut block = Block {
            index,
            timestamp,
            data: data.clone(),
            previous_hash: previous_hash.clone(),
            hash: String::new(),
            node_coordinate: node_coordinate.clone(),
            node_signature: Vec::new(),
            nonce,
        };

        // Calculate hash
        block.hash = block.calculate_hash();

        // Generate signature (placeholder - would use real cryptography in production)
        block.node_signature = block.generate_signature();

        block
    }

    /// Create the genesis block for a node
    pub fn genesis(node_coordinate: MatrixCoordinate) -> Self {
        let genesis_data = format!(
            "Genesis block for node at ({}, {}, {})",
            node_coordinate.x, node_coordinate.y, node_coordinate.z
        );

        Block::new(
            0,
            genesis_data.as_bytes().to_vec(),
            String::from("0000000000000000000000000000000000000000000000000000000000000000"),
            node_coordinate,
        )
    }

    /// Calculate the hash of this block using BLAKE3
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Hasher::new();

        // Hash all block components
        hasher.update(&self.index.to_le_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(&self.data);
        hasher.update(self.previous_hash.as_bytes());
        hasher.update(&self.node_coordinate.x.to_le_bytes());
        hasher.update(&self.node_coordinate.y.to_le_bytes());
        hasher.update(&self.node_coordinate.z.to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());

        let hash = hasher.finalize();
        format!("{}", hash)
    }

    /// Verify the block's hash is correct
    pub fn verify_hash(&self) -> bool {
        self.hash == self.calculate_hash()
    }

    /// Generate a signature for the block (placeholder implementation)
    fn generate_signature(&self) -> Vec<u8> {
        // In production, this would use real cryptographic signatures
        // For now, we'll use a simple hash-based pseudo-signature
        let mut hasher = Hasher::new();
        hasher.update(self.hash.as_bytes());
        hasher.update(b"node_signature");
        hasher.finalize().as_bytes().to_vec()
    }

    /// Verify the block's signature
    pub fn verify_signature(&self) -> bool {
        // Placeholder verification - always returns true for now
        // In production, this would verify using the node's public key
        !self.node_signature.is_empty()
    }

    /// Check if this block belongs to the specified node
    pub fn belongs_to_node(&self, node_coordinate: &MatrixCoordinate) -> bool {
        self.node_coordinate == *node_coordinate
    }

    /// Get the block size in bytes
    pub fn size(&self) -> usize {
        8 + // index
        8 + // timestamp (approximate)
        self.data.len() +
        64 + // previous_hash (hex string)
        64 + // hash (hex string)
        12 + // node_coordinate (3 * i32)
        self.node_signature.len() +
        8 // nonce
    }

    /// Check if this is a genesis block
    pub fn is_genesis(&self) -> bool {
        self.index == 0 &&
        self.previous_hash == "0000000000000000000000000000000000000000000000000000000000000000"
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Block #{} at {} | Node: ({},{},{}) | Hash: {}...{}",
            self.index,
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z,
            &self.hash[..8],
            &self.hash[self.hash.len()-8..]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block_creation() {
        let coord = MatrixCoordinate::new(1, 2, 3).unwrap();
        let genesis = Block::genesis(coord.clone());

        assert_eq!(genesis.index, 0);
        assert!(genesis.is_genesis());
        assert_eq!(genesis.node_coordinate, coord);
        assert!(genesis.verify_hash());
        assert!(genesis.verify_signature());
    }

    #[test]
    fn test_block_creation() {
        let coord = MatrixCoordinate::new(5, 5, 5).unwrap();
        let data = b"Test block data".to_vec();
        let prev_hash = "abc123".to_string();

        let block = Block::new(1, data.clone(), prev_hash.clone(), coord.clone());

        assert_eq!(block.index, 1);
        assert_eq!(block.data, data);
        assert_eq!(block.previous_hash, prev_hash);
        assert_eq!(block.node_coordinate, coord);
        assert!(!block.hash.is_empty());
        assert!(block.verify_hash());
    }

    #[test]
    fn test_hash_verification() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let mut block = Block::new(
            1,
            b"data".to_vec(),
            "prev".to_string(),
            coord,
        );

        assert!(block.verify_hash());

        // Tamper with data
        block.data = b"tampered".to_vec();
        assert!(!block.verify_hash());

        // Fix the hash
        block.hash = block.calculate_hash();
        assert!(block.verify_hash());
    }

    #[test]
    fn test_block_belongs_to_node() {
        let coord1 = MatrixCoordinate::new(1, 1, 1).unwrap();
        let coord2 = MatrixCoordinate::new(2, 2, 2).unwrap();

        let block = Block::genesis(coord1.clone());

        assert!(block.belongs_to_node(&coord1));
        assert!(!block.belongs_to_node(&coord2));
    }

    #[test]
    fn test_block_size() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let block = Block::new(
            100,
            vec![0u8; 1024], // 1KB of data
            "x".repeat(64),
            coord,
        );

        let size = block.size();
        assert!(size >= 1024); // At least the data size
        assert!(size < 2048);  // But not too much overhead
    }

    #[test]
    fn test_block_display() {
        let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
        let block = Block::genesis(coord);

        let display = format!("{}", block);
        assert!(display.contains("Block #0"));
        assert!(display.contains("Node: (10,20,30)"));
    }

    #[test]
    fn test_deterministic_hash() {
        let coord = MatrixCoordinate::new(1, 2, 3).unwrap();
        let data = b"test".to_vec();

        // Create two blocks with same data but let nonce be random
        let block1 = Block::new(1, data.clone(), "prev".to_string(), coord.clone());
        let block2 = Block::new(1, data.clone(), "prev".to_string(), coord.clone());

        // Hashes should be different due to different nonce
        assert_ne!(block1.hash, block2.hash);

        // But if we set same nonce, hashes should match
        let mut block3 = block1.clone();
        block3.nonce = block1.nonce;
        block3.timestamp = block1.timestamp;
        block3.hash = block3.calculate_hash();

        assert_eq!(block1.hash, block3.hash);
    }

    #[test]
    fn test_serialization() {
        let coord = MatrixCoordinate::new(7, 8, 9).unwrap();
        let block = Block::genesis(coord);

        // Serialize to JSON
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"index\":0"));

        // Deserialize back
        let decoded: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(block, decoded);
    }

    #[test]
    fn test_genesis_block_properties() {
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let genesis = Block::genesis(coord.clone());

        // Genesis block should have specific properties
        assert_eq!(genesis.index, 0);
        assert!(genesis.is_genesis());
        assert_eq!(
            genesis.previous_hash,
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
        assert!(genesis.data.len() > 0); // Has genesis message
        assert!(genesis.verify_hash());
        assert!(genesis.verify_signature());
    }
}