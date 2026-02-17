// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Chain validation logic for every-node-blockchain
//!
//! Validates blocks and chains WITHOUT cross-node merkle consolidation.
//! Each node validates its own chain independently.

use chrono::{DateTime, Utc};
use tracing::{warn, error, debug};

use super::block::Block;
use crate::matrix::coordinate::MatrixCoordinate;

/// Validation rules configuration
#[derive(Debug, Clone)]
pub struct ValidationRules {
    /// Maximum time difference allowed between blocks (milliseconds)
    pub max_block_time_ms: i64,

    /// Minimum time difference required between blocks (milliseconds)
    pub min_block_time_ms: i64,

    /// Maximum block data size in bytes
    pub max_block_size: usize,

    /// Whether to enforce strict index sequencing
    pub strict_indexing: bool,

    /// Whether to validate node ownership
    pub validate_ownership: bool,

    /// Maximum time drift allowed from current time (seconds)
    pub max_time_drift_secs: i64,
}

impl Default for ValidationRules {
    fn default() -> Self {
        ValidationRules {
            max_block_time_ms: 3600000,  // 1 hour
            min_block_time_ms: 0,        // No minimum by default
            max_block_size: 10485760,    // 10 MB
            strict_indexing: true,
            validate_ownership: true,
            max_time_drift_secs: 300,    // 5 minutes
        }
    }
}

/// Chain validator for node-specific blockchains
pub struct ChainValidator {
    rules: ValidationRules,
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

        // Verify signature
        if !block.verify_signature() {
            error!("Block {} has invalid signature", block.index);
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

        // Check timestamp validity
        if !self.validate_timestamp(block.timestamp) {
            error!("Block {} has invalid timestamp", block.index);
            return false;
        }

        debug!("Block {} integrity validation passed", block.index);
        true
    }

    /// Validate block continuity with previous block
    fn validate_block_continuity(&self, block: &Block, previous: &Block) -> bool {
        // Check index sequence
        if self.rules.strict_indexing {
            if block.index != previous.index + 1 {
                error!(
                    "Block index mismatch: expected {}, got {}",
                    previous.index + 1,
                    block.index
                );
                return false;
            }
        }

        // Check previous hash link
        if block.previous_hash != previous.hash {
            error!(
                "Previous hash mismatch in block {}: expected {}, got {}",
                block.index, previous.hash, block.previous_hash
            );
            return false;
        }

        // Check timestamp ordering
        if block.timestamp <= previous.timestamp {
            error!(
                "Block {} timestamp not after previous block",
                block.index
            );
            return false;
        }

        // Check time difference constraints
        let time_diff = (block.timestamp - previous.timestamp).num_milliseconds();

        if time_diff > self.rules.max_block_time_ms {
            warn!(
                "Block {} time difference too large: {} ms",
                block.index, time_diff
            );
            return false;
        }

        if time_diff < self.rules.min_block_time_ms {
            warn!(
                "Block {} time difference too small: {} ms",
                block.index, time_diff
            );
            return false;
        }

        // Validate node ownership if required
        if self.rules.validate_ownership {
            if block.node_coordinate != previous.node_coordinate {
                error!(
                    "Block {} node coordinate mismatch with previous",
                    block.index
                );
                return false;
            }
        }

        debug!("Block {} continuity validation passed", block.index);
        true
    }

    /// Validate a timestamp
    fn validate_timestamp(&self, timestamp: DateTime<Utc>) -> bool {
        let now = Utc::now();
        let diff_secs = (now - timestamp).num_seconds().abs();

        if diff_secs > self.rules.max_time_drift_secs {
            warn!(
                "Timestamp drift too large: {} seconds",
                diff_secs
            );
            return false;
        }

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
        node_coordinate: &MatrixCoordinate
    ) -> bool {
        for block in blocks {
            if !block.belongs_to_node(node_coordinate) {
                error!(
                    "Block {} does not belong to node ({},{},{})",
                    block.index,
                    node_coordinate.x,
                    node_coordinate.y,
                    node_coordinate.z
                );
                return false;
            }
        }
        true
    }

    /// Check if a chain is longer than another (for fork resolution)
    /// Note: In our architecture, this is for individual node chain management,
    /// NOT for cross-node consensus (as each node has its own chain)
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
            if blocks[i].timestamp <= blocks[i - 1].timestamp {
                error!("Block timestamps not increasing at index {}", i);
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_block(index: u64, previous_hash: String, coord: MatrixCoordinate) -> Block {
        Block::new(index, vec![0u8; 100], previous_hash, coord)
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
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let genesis = Block::genesis(coord);

        assert!(validator.validate_block(&genesis, None));
    }

    #[test]
    fn test_block_integrity_validation() {
        let validator = ChainValidator::new();
        let coord = MatrixCoordinate::new(1, 1, 1).unwrap();
        let mut block = create_test_block(1, "prev_hash".to_string(), coord);

        // Valid block
        assert!(validator.validate_block_integrity(&block));

        // Tamper with hash
        block.hash = "invalid_hash".to_string();
        assert!(!validator.validate_block_integrity(&block));

        // Fix hash
        block.hash = block.calculate_hash();
        assert!(validator.validate_block_integrity(&block));
    }

    #[test]
    fn test_block_continuity_validation() {
        let validator = ChainValidator::new();
        let coord = MatrixCoordinate::new(2, 2, 2).unwrap();

        let block1 = create_test_block(1, "genesis".to_string(), coord.clone());
        let block2 = Block::new(2, vec![1u8; 50], block1.hash.clone(), coord.clone());

        assert!(validator.validate_block_continuity(&block2, &block1));

        // Wrong previous hash
        let bad_block = Block::new(2, vec![2u8; 50], "wrong_hash".to_string(), coord.clone());
        assert!(!validator.validate_block_continuity(&bad_block, &block1));
    }

    #[test]
    fn test_index_sequence_validation() {
        let validator = ChainValidator::new();
        let coord = MatrixCoordinate::new(3, 3, 3).unwrap();

        let block1 = create_test_block(1, "genesis".to_string(), coord.clone());

        // Correct sequence
        let block2 = Block::new(2, vec![1u8; 50], block1.hash.clone(), coord.clone());
        assert!(validator.validate_block(&block2, Some(&block1)));

        // Wrong index
        let bad_block = Block::new(3, vec![2u8; 50], block1.hash.clone(), coord.clone());
        assert!(!validator.validate_block(&bad_block, Some(&block1)));
    }

    #[test]
    fn test_timestamp_validation() {
        let mut rules = ValidationRules::default();
        rules.max_time_drift_secs = 60; // 1 minute
        let validator = ChainValidator::with_rules(rules);

        let current_time = Utc::now();
        assert!(validator.validate_timestamp(current_time));

        // Future timestamp (beyond drift)
        let future_time = current_time + Duration::seconds(120);
        assert!(!validator.validate_timestamp(future_time));

        // Past timestamp (beyond drift)
        let past_time = current_time - Duration::seconds(120);
        assert!(!validator.validate_timestamp(past_time));
    }

    #[test]
    fn test_chain_validation() {
        let validator = ChainValidator::new();
        let coord = MatrixCoordinate::new(4, 4, 4).unwrap();

        let mut chain = vec![Block::genesis(coord.clone())];

        // Build valid chain
        for i in 1..5 {
            let prev_hash = chain.last().unwrap().hash.clone();
            chain.push(Block::new(i, vec![i as u8; 100], prev_hash, coord.clone()));
        }

        assert!(validator.validate_chain(&chain));

        // Empty chain
        assert!(!validator.validate_chain(&[]));

        // Non-genesis first block
        let bad_chain = vec![create_test_block(1, "hash".to_string(), coord.clone())];
        assert!(!validator.validate_chain(&bad_chain));
    }

    #[test]
    fn test_chain_ownership_validation() {
        let validator = ChainValidator::new();
        let coord1 = MatrixCoordinate::new(5, 5, 5).unwrap();
        let coord2 = MatrixCoordinate::new(6, 6, 6).unwrap();

        let chain = vec![
            Block::genesis(coord1.clone()),
            create_test_block(1, "hash1".to_string(), coord1.clone()),
            create_test_block(2, "hash2".to_string(), coord1.clone()),
        ];

        // Correct ownership
        assert!(validator.validate_chain_ownership(&chain, &coord1));

        // Wrong ownership
        assert!(!validator.validate_chain_ownership(&chain, &coord2));
    }

    #[test]
    fn test_block_ordering_validation() {
        let coord = MatrixCoordinate::new(7, 7, 7).unwrap();

        // Properly ordered chain
        let good_chain = vec![
            Block::genesis(coord.clone()),
            create_test_block(1, "hash1".to_string(), coord.clone()),
            create_test_block(2, "hash2".to_string(), coord.clone()),
        ];
        assert!(ChainValidator::validate_block_ordering(&good_chain));

        // Improperly ordered (by index)
        let bad_chain = vec![
            Block::genesis(coord.clone()),
            create_test_block(2, "hash2".to_string(), coord.clone()),
            create_test_block(1, "hash1".to_string(), coord.clone()),
        ];
        assert!(!ChainValidator::validate_block_ordering(&bad_chain));
    }

    #[test]
    fn test_chain_length_comparison() {
        let coord = MatrixCoordinate::new(8, 8, 8).unwrap();

        let short_chain = vec![
            Block::genesis(coord.clone()),
            create_test_block(1, "hash".to_string(), coord.clone()),
        ];

        let long_chain = vec![
            Block::genesis(coord.clone()),
            create_test_block(1, "hash1".to_string(), coord.clone()),
            create_test_block(2, "hash2".to_string(), coord.clone()),
            create_test_block(3, "hash3".to_string(), coord.clone()),
        ];

        assert!(ChainValidator::is_longer_chain(&long_chain, &short_chain));
        assert!(!ChainValidator::is_longer_chain(&short_chain, &long_chain));
    }
}