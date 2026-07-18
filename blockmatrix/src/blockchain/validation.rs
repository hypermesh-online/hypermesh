// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Chain validation logic for every-node-blockchain
//!
//! Validates blocks and chains WITHOUT cross-node merkle consolidation.
//! Each node validates its own chain independently.

use tracing::{debug, error, warn};

use super::block::Block;
use crate::matrix::coordinate::MatrixCoordinate;

/// Validation rules configuration.
///
/// Timestamp and coordinate checks are NOT here — they belong in
/// Proof of State (PoTime = WHEN, PoSpace = WHERE).
#[derive(Debug, Clone)]
pub struct ValidationRules {
    /// Maximum block data size in bytes
    pub max_block_size: usize,

    /// Whether to enforce strict index sequencing
    pub strict_indexing: bool,
}

impl Default for ValidationRules {
    fn default() -> Self {
        ValidationRules {
            max_block_size: 10485760, // 10 MB
            strict_indexing: true,
        }
    }
}

/// Chain validator for node-specific blockchains
pub struct ChainValidator {
    rules: ValidationRules,
}

impl Default for ChainValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainValidator {
    /// Create a new validator with default rules
    pub fn new() -> Self {
        ChainValidator {
            rules: ValidationRules::default(),
        }
    }

    /// Create a validator with custom rules
    pub fn with_rules(rules: ValidationRules) -> Self {
        ChainValidator { rules }
    }

    /// Validate a single block
    pub fn validate_block(&self, block: &Block, previous: Option<&Block>) -> bool {
        // Basic block validation
        if !self.validate_block_integrity(block) {
            return false;
        }

        // Validate against previous block if provided
        if let Some(prev) = previous {
            if !self.validate_block_continuity(block, prev) {
                return false;
            }
        } else {
            // If no previous block provided, this should be genesis
            if !block.is_genesis() {
                error!("Non-genesis block provided without previous block");
                return false;
            }
        }

        true
    }

    /// Validate block integrity (hash, signature, size, etc.)
    fn validate_block_integrity(&self, block: &Block) -> bool {
        // Verify hash
        if !block.verify_hash() {
            error!("Block {} has invalid hash", block.index);
            return false;
        }

        // Check block size
        if block.size() > self.rules.max_block_size {
            error!(
                "Block {} exceeds maximum size: {} > {}",
                block.index,
                block.size(),
                self.rules.max_block_size
            );
            return false;
        }

        debug!("Block {} integrity validation passed", block.index);
        true
    }

    /// Validate block continuity with previous block
    fn validate_block_continuity(&self, block: &Block, previous: &Block) -> bool {
        // Check index sequence
        if self.rules.strict_indexing && block.index != previous.index + 1 {
            error!(
                "Block index mismatch: expected {}, got {}",
                previous.index + 1,
                block.index
            );
            return false;
        }

        // Check previous hash link
        if block.previous_hash != previous.hash {
            error!(
                "Previous hash mismatch in block {}: expected {}, got {}",
                block.index, previous.hash, block.previous_hash
            );
            return false;
        }

        debug!("Block {} continuity validation passed", block.index);
        true
    }

    /// Validate an entire chain
    pub fn validate_chain(&self, blocks: &[Block]) -> bool {
        if blocks.is_empty() {
            warn!("Empty chain provided for validation");
            return false;
        }

        // First block must be genesis
        if !blocks[0].is_genesis() {
            error!("Chain does not start with genesis block");
            return false;
        }

        // Validate genesis block
        if !self.validate_block(&blocks[0], None) {
            error!("Genesis block validation failed");
            return false;
        }

        // Validate each subsequent block
        for i in 1..blocks.len() {
            if !self.validate_block(&blocks[i], Some(&blocks[i - 1])) {
                error!("Block {} validation failed", blocks[i].index);
                return false;
            }
        }

        debug!("Chain validation passed for {} blocks", blocks.len());
        true
    }

    /// Validate node ownership for entire chain
    pub fn validate_chain_ownership(
        &self,
        blocks: &[Block],
        node_coordinate: &MatrixCoordinate,
    ) -> bool {
        for block in blocks {
            if !block.belongs_to_node(node_coordinate) {
                error!(
                    "Block {} does not belong to node ({},{},{})",
                    block.index, node_coordinate.x, node_coordinate.y, node_coordinate.z
                );
                return false;
            }
        }
        true
    }

    /// Check if a chain is longer than another (for fork resolution)
    /// Note: In our architecture, this is for individual node chain management,
    /// NOT for cross-node agreement (as each node has its own chain)
    pub fn is_longer_chain(chain_a: &[Block], chain_b: &[Block]) -> bool {
        chain_a.len() > chain_b.len()
    }

    /// Validate that blocks are properly ordered
    pub fn validate_block_ordering(blocks: &[Block]) -> bool {
        if blocks.is_empty() {
            return true;
        }

        for i in 1..blocks.len() {
            if blocks[i].index <= blocks[i - 1].index {
                error!("Blocks not properly ordered at index {}", i);
                return false;
            }
        }

        true
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
        let mut state_proof = StateProof::default();
        // Set space proof node_id to match coordinate (for belongs_to_node checks)
        state_proof.space_proof.node_id =
            format!("({},{},{})", coord.x, coord.y, coord.z);
        let proof_bytes = serde_json::to_vec(&state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();
        BlockAssetEntry {
            asset_hash: content_hash,
            proof_hash,
            state_proof,
            signed_proof: None,
            storage_pointer: StoragePointer::Genesis,
            registration: reg,
        }
    }

    fn create_test_block(index: u64, previous_hash: String, coord: MatrixCoordinate) -> Block {
        Block::new(index, vec![test_entry(coord)], previous_hash)
    }

    #[test]
    fn test_validator_creation() {
        let validator = ChainValidator::new();
        assert!(validator.rules.strict_indexing);

        let custom_rules = ValidationRules {
            strict_indexing: false,
            ..Default::default()
        };
        let custom_validator = ChainValidator::with_rules(custom_rules);
        assert!(!custom_validator.rules.strict_indexing);
    }

    #[test]
    fn test_genesis_block_validation() {
        let validator = ChainValidator::new();
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let genesis = Block::genesis(coord);

        assert!(validator.validate_block(&genesis, None));
    }

    #[test]
    fn test_block_integrity_validation() {
        let validator = ChainValidator::new();
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let mut block = create_test_block(1, "prev_hash".to_string(), coord);

        assert!(validator.validate_block_integrity(&block));

        block.hash = "invalid_hash".to_string();
        assert!(!validator.validate_block_integrity(&block));

        block.hash = block.calculate_hash();
        assert!(validator.validate_block_integrity(&block));
    }

    #[test]
    fn test_block_continuity_validation() {
        let validator = ChainValidator::new();
        let coord = MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate");

        let block1 = create_test_block(1, "genesis".to_string(), coord);
        let block2 = Block::new(2, vec![test_entry(coord)], block1.hash.clone());

        assert!(validator.validate_block_continuity(&block2, &block1));

        let bad_block = Block::new(2, vec![test_entry(coord)], "wrong_hash".to_string());
        assert!(!validator.validate_block_continuity(&bad_block, &block1));
    }

    #[test]
    fn test_index_sequence_validation() {
        let validator = ChainValidator::new();
        let coord = MatrixCoordinate::new(3, 3, 3).expect("test: valid coordinate");

        let block1 = create_test_block(1, "genesis".to_string(), coord);

        let block2 = Block::new(2, vec![test_entry(coord)], block1.hash.clone());
        assert!(validator.validate_block(&block2, Some(&block1)));

        let bad_block = Block::new(3, vec![test_entry(coord)], block1.hash.clone());
        assert!(!validator.validate_block(&bad_block, Some(&block1)));
    }

    #[test]
    fn test_chain_validation() {
        let validator = ChainValidator::new();
        let coord = MatrixCoordinate::new(4, 4, 4).expect("test: valid coordinate");

        let mut chain = vec![Block::genesis(coord)];

        for i in 1..5u64 {
            let prev_hash = chain.last().expect("test: expected success").hash.clone();
            chain.push(Block::new(i, vec![test_entry(coord)], prev_hash));
        }

        assert!(validator.validate_chain(&chain));
        assert!(!validator.validate_chain(&[]));

        let bad_chain = vec![create_test_block(1, "hash".to_string(), coord)];
        assert!(!validator.validate_chain(&bad_chain));
    }

    #[test]
    fn test_chain_ownership_validation() {
        let validator = ChainValidator::new();
        let coord1 = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let coord2 = MatrixCoordinate::new(6, 6, 6).expect("test: valid coordinate");

        let chain = vec![
            Block::genesis(coord1),
            create_test_block(1, "hash1".to_string(), coord1),
            create_test_block(2, "hash2".to_string(), coord1),
        ];

        assert!(validator.validate_chain_ownership(&chain, &coord1));
        assert!(!validator.validate_chain_ownership(&chain, &coord2));
    }

    #[test]
    fn test_block_ordering_validation() {
        let coord = MatrixCoordinate::new(7, 7, 7).expect("test: valid coordinate");

        let good_chain = vec![
            Block::genesis(coord),
            create_test_block(1, "hash1".to_string(), coord),
            create_test_block(2, "hash2".to_string(), coord),
        ];
        assert!(ChainValidator::validate_block_ordering(&good_chain));

        let bad_chain = vec![
            Block::genesis(coord),
            create_test_block(2, "hash2".to_string(), coord),
            create_test_block(1, "hash1".to_string(), coord),
        ];
        assert!(!ChainValidator::validate_block_ordering(&bad_chain));
    }

    #[test]
    fn test_chain_length_comparison() {
        let coord = MatrixCoordinate::new(8, 8, 8).expect("test: valid coordinate");

        let short_chain = vec![
            Block::genesis(coord),
            create_test_block(1, "hash".to_string(), coord),
        ];

        let long_chain = vec![
            Block::genesis(coord),
            create_test_block(1, "hash1".to_string(), coord),
            create_test_block(2, "hash2".to_string(), coord),
            create_test_block(3, "hash3".to_string(), coord),
        ];

        assert!(ChainValidator::is_longer_chain(&long_chain, &short_chain));
        assert!(!ChainValidator::is_longer_chain(&short_chain, &long_chain));
    }
}
