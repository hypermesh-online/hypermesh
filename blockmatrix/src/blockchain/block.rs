// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Block implementation for every-node-blockchain architecture
//!
//! Each block belongs to a specific node's independent blockchain.
//! NO merkle tree consolidation across nodes - fundamental design principle.

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::assets::core::AssetRegistration;
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

    /// Assets contained in this block (Blocks MUST contain Assets)
    pub assets: Vec<AssetRegistration>,

    /// Hash of the previous block in THIS node's chain
    pub previous_hash: String,

    /// This block's hash (BLAKE3)
    pub hash: String,

    /// The node's matrix coordinate that owns this block
    pub node_coordinate: MatrixCoordinate,

    /// Block nonce for additional entropy
    pub nonce: u64,

    /// Optional shard commitment anchoring this block to its spatial shard evidence.
    /// BLAKE3 hash of the canonical shard distribution map (position-based, not identity-based).
    pub shard_commitment: Option<[u8; 32]>,

    /// BLAKE3 hash of the StateProof that authorized this block.
    /// Genesis blocks have None (self-authorized — sovereignty from boot).
    pub state_proof_hash: Option<[u8; 32]>,
}

impl Block {
    /// Create a new block
    pub fn new(
        index: u64,
        assets: Vec<AssetRegistration>,
        previous_hash: String,
        node_coordinate: MatrixCoordinate,
    ) -> Self {
        // Blocks MUST contain at least one Asset
        assert!(!assets.is_empty(), "Block must contain at least one Asset");

        let timestamp = Utc::now();
        let nonce = rand::random::<u64>();

        let mut block = Block {
            index,
            timestamp,
            assets: assets.clone(),
            previous_hash: previous_hash.clone(),
            hash: String::new(),
            node_coordinate,
            nonce,
            shard_commitment: None,
            state_proof_hash: None,
        };

        // Calculate hash
        block.hash = block.calculate_hash();

        block
    }

    /// Create the genesis block for a node
    pub fn genesis(node_coordinate: MatrixCoordinate) -> Self {
        // Create a genesis AssetRegistration for this node
        let genesis_asset = AssetRegistration::genesis(node_coordinate);

        Block::new(
            0,
            vec![genesis_asset],
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

        // Hash all assets in the block
        for asset in &self.assets {
            hasher.update(asset.to_string().as_bytes());
        }

        hasher.update(self.previous_hash.as_bytes());
        hasher.update(&self.node_coordinate.x.to_le_bytes());
        hasher.update(&self.node_coordinate.y.to_le_bytes());
        hasher.update(&self.node_coordinate.z.to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());

        if let Some(commitment) = &self.shard_commitment {
            hasher.update(commitment);
        }

        if let Some(proof_hash) = &self.state_proof_hash {
            hasher.update(proof_hash);
        }

        let hash = hasher.finalize();
        format!("{hash}")
    }

    /// Verify the block's hash is correct
    pub fn verify_hash(&self) -> bool {
        self.hash == self.calculate_hash()
    }

    /// Check if this block belongs to the specified node
    pub fn belongs_to_node(&self, node_coordinate: &MatrixCoordinate) -> bool {
        self.node_coordinate == *node_coordinate
    }

    /// Set the shard commitment and recalculate the block hash.
    pub fn set_shard_commitment(&mut self, commitment: [u8; 32]) {
        self.shard_commitment = Some(commitment);
        self.hash = self.calculate_hash();
    }

    /// Set the state proof hash and recalculate the block hash.
    ///
    /// The state proof hash is the BLAKE3 digest of the `StateProof` that
    /// authorized this block's creation.  Genesis blocks never have one.
    pub fn set_state_proof_hash(&mut self, hash: [u8; 32]) {
        self.state_proof_hash = Some(hash);
        self.hash = self.calculate_hash();
    }

    /// Get the block size in bytes
    pub fn size(&self) -> usize {
        8 + // index
        8 + // timestamp (approximate)
        (self.assets.len() * 32) + // AssetRegistration size estimate
        64 + // previous_hash (hex string)
        64 + // hash (hex string)
        12 + // node_coordinate (3 * i32)
        8 + // nonce
        if self.shard_commitment.is_some() { 32 } else { 0 } + // shard_commitment
        if self.state_proof_hash.is_some() { 32 } else { 0 } // state_proof_hash
    }

    /// Get the assets in this block
    pub fn get_assets(&self) -> &[AssetRegistration] {
        &self.assets
    }

    /// Get the number of assets in this block
    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }

    /// Check if this is a genesis block
    pub fn is_genesis(&self) -> bool {
        self.index == 0
            && self.previous_hash
                == "0000000000000000000000000000000000000000000000000000000000000000"
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
            &self.hash[self.hash.len() - 8..]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block_creation() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let genesis = Block::genesis(coord);

        assert_eq!(genesis.index, 0);
        assert!(genesis.is_genesis());
        assert_eq!(genesis.node_coordinate, coord);
        assert!(genesis.verify_hash());
    }

    #[test]
    fn test_block_creation() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coord");
        let asset = AssetRegistration::genesis(coord);
        let prev_hash = "abc123".to_string();

        let block = Block::new(1, vec![asset.clone()], prev_hash.clone(), coord);

        assert_eq!(block.index, 1);
        assert_eq!(block.assets.len(), 1);
        assert_eq!(block.assets[0], asset);
        assert_eq!(block.previous_hash, prev_hash);
        assert_eq!(block.node_coordinate, coord);
        assert!(!block.hash.is_empty());
        assert!(block.verify_hash());
    }

    #[test]
    fn test_hash_verification() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let asset = AssetRegistration::genesis(coord);
        let mut block = Block::new(1, vec![asset.clone()], "prev".to_string(), coord);

        assert!(block.verify_hash());

        // Tamper with assets
        let tampered_asset =
            AssetRegistration::genesis(MatrixCoordinate::new(1, 1, 1).expect("test: valid coord"));
        block.assets = vec![tampered_asset];
        assert!(!block.verify_hash());

        // Fix the hash
        block.hash = block.calculate_hash();
        assert!(block.verify_hash());
    }

    #[test]
    fn test_block_belongs_to_node() {
        let coord1 = MatrixCoordinate::new(1, 1, 1).expect("test: valid coord");
        let coord2 = MatrixCoordinate::new(2, 2, 2).expect("test: valid coord");

        let block = Block::genesis(coord1);

        assert!(block.belongs_to_node(&coord1));
        assert!(!block.belongs_to_node(&coord2));
    }

    #[test]
    fn test_block_size() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        // Create multiple assets to test size calculation
        let assets: Vec<AssetRegistration> = (0..10)
            .map(|i| {
                AssetRegistration::genesis(
                    MatrixCoordinate::new(i, i, i).expect("test: valid coord"),
                )
            })
            .collect();

        let block = Block::new(100, assets, "x".repeat(64), coord);

        let size = block.size();
        assert!(size >= 320); // At least 10 assets * 32 bytes
        assert!(size < 1024); // But not too much overhead
    }

    #[test]
    fn test_block_display() {
        let coord = MatrixCoordinate::new(10, 20, 30).expect("test: valid coord");
        let block = Block::genesis(coord);

        let display = format!("{block}");
        assert!(display.contains("Block #0"));
        assert!(display.contains("Node: (10,20,30)"));
    }

    #[test]
    fn test_deterministic_hash() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let asset = AssetRegistration::genesis(coord);

        // Create two blocks with same asset but let nonce be random
        let block1 = Block::new(1, vec![asset.clone()], "prev".to_string(), coord);
        let block2 = Block::new(1, vec![asset.clone()], "prev".to_string(), coord);

        // Hashes should be different due to different nonce/timestamp
        assert_ne!(block1.hash, block2.hash);

        // But if we set same nonce and timestamp, hashes should match
        let mut block3 = block1.clone();
        block3.nonce = block1.nonce;
        block3.timestamp = block1.timestamp;
        block3.hash = block3.calculate_hash();

        assert_eq!(block1.hash, block3.hash);
    }

    #[test]
    fn test_serialization() {
        let coord = MatrixCoordinate::new(7, 8, 9).expect("test: valid coord");
        let block = Block::genesis(coord);

        // Serialize to JSON
        let json = serde_json::to_string(&block).expect("test: valid coord");
        assert!(json.contains("\"index\":0"));

        // Deserialize back
        let decoded: Block = serde_json::from_str(&json).expect("test: valid coord");
        assert_eq!(block, decoded);
    }

    #[test]
    fn test_genesis_block_properties() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let genesis = Block::genesis(coord);

        // Genesis block should have specific properties
        assert_eq!(genesis.index, 0);
        assert!(genesis.is_genesis());
        assert_eq!(
            genesis.previous_hash,
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
        assert!(!genesis.assets.is_empty()); // Has genesis asset
        assert_eq!(genesis.asset_count(), 1); // Exactly one genesis asset
        assert!(genesis.verify_hash());
    }

    #[test]
    fn test_block_must_have_assets() {
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coord");
        let result = std::panic::catch_unwind(|| Block::new(1, vec![], "prev".to_string(), coord));
        assert!(
            result.is_err(),
            "Block creation with empty assets should panic"
        );
    }

    #[test]
    fn test_asset_helpers() {
        let coord = MatrixCoordinate::new(2, 2, 2).expect("test: valid coord");
        let assets: Vec<AssetRegistration> = (0..5)
            .map(|i| {
                AssetRegistration::genesis(
                    MatrixCoordinate::new(i, i, i).expect("test: valid coord"),
                )
            })
            .collect();

        let block = Block::new(1, assets.clone(), "prev".to_string(), coord);

        assert_eq!(block.asset_count(), 5);
        assert_eq!(block.get_assets().len(), 5);
        assert_eq!(block.get_assets(), &assets[..]);
    }
}
