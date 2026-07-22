// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sync protocol types and helpers
//!
//! Contains the wire-level message types exchanged during blockchain
//! synchronization, the serializable propagation strategy config, and the
//! snapshot-based `BlockProvider` adapter for `NodeBlockchain`.
//!
//! Extracted from `sync_manager` to keep production code under the 500-line gate.

use serde::{Deserialize, Serialize};

use super::block::{Block, BlockHeader};
use super::propagation::PropagationStrategy;
use super::sync_manager::BlockProvider;

/// Serializable wrapper around PropagationStrategy selection.
///
/// We keep this separate from the runtime `PropagationStrategy` enum
/// so that SyncConfig can be serialized/deserialized cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropagationStrategyConfig {
    /// Send to all immediate neighbours
    Broadcast,
    /// Send to closest N neighbours
    NearestN(usize),
    /// Use optimal routing paths
    RoutedPath,
    /// Send to nodes within distance threshold
    DistanceThreshold(f64),
}

impl PropagationStrategyConfig {
    /// Convert to the runtime `PropagationStrategy` used by the propagator
    pub fn to_runtime(&self) -> PropagationStrategy {
        match self {
            Self::Broadcast => PropagationStrategy::Broadcast,
            Self::NearestN(n) => PropagationStrategy::NearestN(*n),
            Self::RoutedPath => PropagationStrategy::RoutedPath,
            Self::DistanceThreshold(d) => PropagationStrategy::DistanceThreshold(*d),
        }
    }
}

/// Messages exchanged during synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Request blocks from a peer starting at a given height
    Request {
        network_id: String,
        from_height: u64,
        max_blocks: u32,
    },
    /// Response containing block hashes and the peer's current height
    Response {
        network_id: String,
        block_hashes: Vec<String>,
        peer_height: u64,
    },
    /// Announce a new block to the network
    Announce {
        network_id: String,
        block_height: u64,
        block_hash: String,
    },
    /// Request the genesis block for a network.
    GenesisRequest {
        network_id: String,
    },
    /// Response with the network's genesis block.
    GenesisResponse {
        network_id: String,
        genesis_block: Block,
    },
    /// Request block headers for lightweight chain verification.
    HeaderRequest {
        network_id: String,
        from_height: u64,
        max_count: u32,
    },
    /// Response with block headers.
    HeaderResponse {
        network_id: String,
        headers: Vec<BlockHeader>,
        peer_height: u64,
    },
    /// Request full blocks by hash (for segments node participates in).
    BlockRequest {
        network_id: String,
        block_hashes: Vec<String>,
    },
    /// Response with full blocks.
    BlockResponse {
        network_id: String,
        blocks: Vec<Block>,
    },
}

/// Snapshot-based [`BlockProvider`] for `NodeBlockchain`.
///
/// Created from a pre-fetched list of blocks, avoiding the need for async
/// inside the synchronous `BlockProvider` trait. Typical usage:
///
/// ```ignore
/// let blocks = node_blockchain.get_chain().await;
/// let provider = NodeBlockchainBlockProvider::from_blocks(&blocks);
/// sync_manager.process_sync_message_with_provider(msg, Some(&provider));
/// ```
pub struct NodeBlockchainBlockProvider {
    block_hashes: Vec<String>,
    chain_height: u64,
    /// S3.0/B3: the FULL genesis block, so a `GenesisRequest` can be answered
    /// with something a peer can actually verify and adopt.
    genesis: Option<super::block::Block>,
}

impl NodeBlockchainBlockProvider {
    /// Create from a slice of [`Block`]s (typically from `NodeBlockchain::get_chain()`).
    pub fn from_blocks(blocks: &[super::block::Block]) -> Self {
        Self {
            block_hashes: blocks.iter().map(|b| b.hash.clone()).collect(),
            chain_height: blocks.len() as u64,
            genesis: blocks.iter().find(|b| b.is_genesis()).cloned(),
        }
    }
}

impl BlockProvider for NodeBlockchainBlockProvider {
    fn get_block_hashes(&self, from_height: u64, max_blocks: u32) -> (Vec<String>, u64) {
        let start = from_height as usize;
        if start >= self.block_hashes.len() {
            return (vec![], self.chain_height);
        }
        let end = (start + max_blocks as usize).min(self.block_hashes.len());
        (self.block_hashes[start..end].to_vec(), self.chain_height)
    }

    fn get_genesis_block(&self) -> Option<super::block::Block> {
        self.genesis.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;

    #[test]
    fn test_genesis_request_serialization() {
        let msg = SyncMessage::GenesisRequest {
            network_id: "net-alpha".to_string(),
        };
        let json = serde_json::to_string(&msg).expect("test: serialize");
        let parsed: SyncMessage = serde_json::from_str(&json).expect("test: deserialize");
        match parsed {
            SyncMessage::GenesisRequest { network_id } => {
                assert_eq!(network_id, "net-alpha");
            }
            other => unreachable!("test: expected GenesisRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_genesis_response_serialization() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coord");
        let genesis = Block::genesis(coord);

        let msg = SyncMessage::GenesisResponse {
            network_id: "net-beta".to_string(),
            genesis_block: genesis.clone(),
        };
        let json = serde_json::to_string(&msg).expect("test: serialize");
        let parsed: SyncMessage = serde_json::from_str(&json).expect("test: deserialize");
        match parsed {
            SyncMessage::GenesisResponse {
                network_id,
                genesis_block,
            } => {
                assert_eq!(network_id, "net-beta");
                assert_eq!(genesis_block.index, 0);
                assert_eq!(genesis_block.hash, genesis.hash);
                assert!(genesis_block.verify_hash());
            }
            other => unreachable!("test: expected GenesisResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_header_request_serialization() {
        let msg = SyncMessage::HeaderRequest {
            network_id: "net-gamma".to_string(),
            from_height: 100,
            max_count: 50,
        };
        let json = serde_json::to_string(&msg).expect("test: serialize");
        let parsed: SyncMessage = serde_json::from_str(&json).expect("test: deserialize");
        match parsed {
            SyncMessage::HeaderRequest {
                network_id,
                from_height,
                max_count,
            } => {
                assert_eq!(network_id, "net-gamma");
                assert_eq!(from_height, 100);
                assert_eq!(max_count, 50);
            }
            other => unreachable!("test: expected HeaderRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_header_response_serialization() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let block = Block::genesis(coord);
        let header = block.header();

        let msg = SyncMessage::HeaderResponse {
            network_id: "net-delta".to_string(),
            headers: vec![header.clone()],
            peer_height: 42,
        };
        let json = serde_json::to_string(&msg).expect("test: serialize");
        let parsed: SyncMessage = serde_json::from_str(&json).expect("test: deserialize");
        match parsed {
            SyncMessage::HeaderResponse {
                network_id,
                headers,
                peer_height,
            } => {
                assert_eq!(network_id, "net-delta");
                assert_eq!(headers.len(), 1);
                assert_eq!(headers[0].index, header.index);
                assert_eq!(headers[0].hash, header.hash);
                assert_eq!(peer_height, 42);
            }
            other => unreachable!("test: expected HeaderResponse, got {:?}", other),
        }
    }

    #[test]
    fn test_block_request_serialization() {
        let msg = SyncMessage::BlockRequest {
            network_id: "net-epsilon".to_string(),
            block_hashes: vec!["abc123".to_string(), "def456".to_string()],
        };
        let json = serde_json::to_string(&msg).expect("test: serialize");
        let parsed: SyncMessage = serde_json::from_str(&json).expect("test: deserialize");
        match parsed {
            SyncMessage::BlockRequest {
                network_id,
                block_hashes,
            } => {
                assert_eq!(network_id, "net-epsilon");
                assert_eq!(block_hashes.len(), 2);
            }
            other => unreachable!("test: expected BlockRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_block_response_serialization() {
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coord");
        let block = Block::genesis(coord);

        let msg = SyncMessage::BlockResponse {
            network_id: "net-zeta".to_string(),
            blocks: vec![block.clone()],
        };
        let json = serde_json::to_string(&msg).expect("test: serialize");
        let parsed: SyncMessage = serde_json::from_str(&json).expect("test: deserialize");
        match parsed {
            SyncMessage::BlockResponse {
                network_id,
                blocks,
            } => {
                assert_eq!(network_id, "net-zeta");
                assert_eq!(blocks.len(), 1);
                assert_eq!(blocks[0].hash, block.hash);
            }
            other => unreachable!("test: expected BlockResponse, got {:?}", other),
        }
    }
}
