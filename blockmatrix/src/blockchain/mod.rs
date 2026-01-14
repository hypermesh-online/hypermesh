//! Revolutionary Every-Node-Blockchain Architecture
//!
//! This module implements BlockMatrix's revolutionary concept where EVERY node
//! maintains its own independent blockchain. There is NO merkle tree consolidation
//! across nodes, NO shared chain, and complete node sovereignty.
//!
//! Key Concepts:
//! - Each node has ONE blockchain instance (not shared)
//! - Genesis block includes the node's MatrixCoordinate
//! - Chain validation is per-node (no cross-node merkle validation)
//! - Block propagation uses matrix topology for neighbor discovery
//! - Complete autonomy - nodes don't need consensus to add blocks
//!
//! This fundamentally differs from traditional blockchain architectures where
//! all nodes share a single chain and use consensus mechanisms like PoW or PoS.

pub mod block;
pub mod node_chain;
pub mod validation;
pub mod propagation;
pub mod state;
pub mod errors;

pub use block::Block;
pub use node_chain::{NodeBlockchain, ChainStats};
pub use validation::{ChainValidator, ValidationRules};
pub use propagation::{BlockPropagator, PropagationStrategy, PropagationResult};
pub use state::{ChainStateManager, ChainSnapshot, BlockQuery, SortOrder, StorageStats};
pub use errors::{BlockchainError, StateError, PropagationError, Result};

use crate::matrix::coordinate::MatrixCoordinate;
use std::path::Path;

/// Create a new node blockchain with all components initialized
pub async fn create_node_blockchain(
    node_coordinate: MatrixCoordinate,
    storage_path: impl AsRef<Path>,
) -> std::result::Result<(NodeBlockchain, ChainStateManager, BlockPropagator), String> {
    // Create the blockchain
    let blockchain = NodeBlockchain::new(node_coordinate.clone());

    // Create state manager
    let state_manager = ChainStateManager::new(
        node_coordinate.clone(),
        storage_path,
    );
    state_manager.initialize().await?;

    // Create propagator with default broadcast strategy
    let propagator = BlockPropagator::new(
        node_coordinate,
        PropagationStrategy::Broadcast,
    );

    Ok((blockchain, state_manager, propagator))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_full_blockchain_creation() {
        let temp_dir = TempDir::new().unwrap();
        let coord = MatrixCoordinate::new(1, 2, 3).unwrap();

        let result = create_node_blockchain(
            coord.clone(),
            temp_dir.path(),
        ).await;

        assert!(result.is_ok());
        let (blockchain, state_manager, propagator) = result.unwrap();

        // Verify components are initialized
        assert_eq!(blockchain.node_coordinate(), &coord);
        assert_eq!(blockchain.get_height().await, 0); // Genesis only

        // Verify state manager created directories
        assert!(temp_dir.path().join("blocks").exists());
        assert!(temp_dir.path().join("snapshots").exists());
    }
}