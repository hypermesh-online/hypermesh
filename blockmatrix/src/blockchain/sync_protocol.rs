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
}

impl NodeBlockchainBlockProvider {
    /// Create from a slice of [`Block`]s (typically from `NodeBlockchain::get_chain()`).
    pub fn from_blocks(blocks: &[super::block::Block]) -> Self {
        Self {
            block_hashes: blocks.iter().map(|b| b.hash.clone()).collect(),
            chain_height: blocks.len() as u64,
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
}
