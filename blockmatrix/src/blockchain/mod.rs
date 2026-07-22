// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

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
//! - Complete autonomy - nodes don't need external validation to add blocks
//!
//! This fundamentally differs from traditional blockchain architectures where
//! all nodes share a single chain and use Proof of States like PoW or PoS.

pub mod asset_index;
pub mod block;
pub mod block_sink;
pub mod chain;
pub mod errors;
pub mod genesis_assessor;
pub mod genesis_auth;
pub mod genesis_crypto;
pub mod genesis_ops;
pub mod lineage;
pub mod mutations;
pub mod node_chain;
pub mod propagation;
pub mod state;
pub mod stoq_transport;
pub mod sync_manager;
pub mod sync_protocol;
pub mod validation;

pub use asset_index::{AssetChainIndex, AssetEntryLocator, AssetHighWater};
pub use block::{Block, BlockHeader};
pub use block_sink::BlockSink;
pub use errors::{BlockchainError, PropagationError, Result, StateError};
pub use genesis_assessor::{
    GenesisAssessor, HardwareProbe, RealHardwareProbe, SyntheticHardwareProbe,
};
pub use genesis_auth::{GenesisAuthManager, GenesisCredentials};
pub use lineage::{AssetLineage, LineageBreak};
pub use node_chain::{ChainStats, NodeBlockchain};
pub use propagation::{
    BlockPropagator, BlockTransport, PropagationResult, PropagationStrategy, SimulatedTransport,
};
pub use stoq_transport::StoqBlockTransportAdapter;
pub use state::{BlockQuery, ChainSnapshot, ChainStateManager, SortOrder, StorageStats};
pub use sync_manager::{
    BlockProvider, NetworkMembership, NodeBlockchainBlockProvider, SyncConfig, SyncManager,
    SyncMessage, SyncObserver, SyncState,
};
pub use validation::{ChainValidator, ValidationRules};

use crate::matrix::coordinate::MatrixCoordinate;
use std::path::Path;

/// Create a new node blockchain with all components initialized
pub async fn create_node_blockchain(
    node_coordinate: MatrixCoordinate,
    storage_path: impl AsRef<Path>,
) -> std::result::Result<(NodeBlockchain, ChainStateManager, BlockPropagator), String> {
    // Create the blockchain
    let blockchain = NodeBlockchain::new(node_coordinate);

    // Create state manager
    let state_manager = ChainStateManager::new(node_coordinate, storage_path);
    state_manager.initialize().await?;

    // Create propagator with default broadcast strategy
    let propagator = BlockPropagator::new(node_coordinate, PropagationStrategy::Broadcast);

    Ok((blockchain, state_manager, propagator))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_full_blockchain_creation() {
        let temp_dir = TempDir::new().expect("test: create temp dir");
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: create coordinate");

        let result = create_node_blockchain(coord, temp_dir.path()).await;

        assert!(result.is_ok());
        let (blockchain, _state_manager, _propagator) = result.expect("test: create blockchain");

        // Verify components are initialized
        assert_eq!(blockchain.node_coordinate(), &coord);
        assert_eq!(blockchain.get_height().await, 0); // Genesis only

        // Verify state manager created directories
        assert!(temp_dir.path().join("blocks").exists());
        assert!(temp_dir.path().join("snapshots").exists());
    }
}
