// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Swarm Distribution Protocol (R12)
//!
//! Implements announce/discover/serve for shards where consuming nodes
//! automatically become providers. Uses a cascade model so per-node
//! load scales O(log N) for N consumers rather than centralizing on a
//! single origin.
//!
//! # Architecture
//!
//! Each `SwarmNode` tracks which shard IDs it can serve. `SwarmProtocol`
//! orchestrates announcement, discovery, and serving across a set of
//! nodes. When a node fetches a shard it auto-announces availability,
//! creating a cascade that distributes load logarithmically.

use hypermesh_lib::{ContentHash, NodeId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::network::shard_transport::ShardTransport;

/// A node participating in the swarm, tracking which shards it can serve.
#[derive(Debug, Clone)]
pub struct SwarmNode {
    /// Identity of this node.
    pub node_id: NodeId,
    /// Set of shard IDs this node can serve.
    available_shards: HashSet<ContentHash>,
}

impl SwarmNode {
    /// Create a new swarm node with no available shards.
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            available_shards: HashSet::new(),
        }
    }

    /// Announce that this node can serve the given shard.
    pub fn announce(&mut self, shard_id: ContentHash) {
        self.available_shards.insert(shard_id);
    }

    /// Check if this node can serve the given shard.
    pub fn has_shard(&self, shard_id: &ContentHash) -> bool {
        self.available_shards.contains(shard_id)
    }

    /// Number of shards this node provides.
    pub fn shard_count(&self) -> usize {
        self.available_shards.len()
    }
}

/// Swarm-wide protocol for shard distribution.
///
/// Maintains a directory of which nodes serve which shards. When a
/// consumer fetches a shard through [`serve_shard`], it is automatically
/// announced as a new provider -- this is the cascade that keeps load
/// O(log N).
pub struct SwarmProtocol<T: ShardTransport> {
    /// Per-node availability records keyed by NodeId.
    nodes: Arc<RwLock<HashMap<NodeId, SwarmNode>>>,
    /// Reverse index: shard_id -> set of NodeIds that have it.
    shard_providers: Arc<RwLock<HashMap<ContentHash, HashSet<NodeId>>>>,
    /// Underlying transport for fetching shard bytes.
    transport: Arc<T>,
}

impl<T: ShardTransport> SwarmProtocol<T> {
    /// Create a new swarm protocol backed by the given transport.
    pub fn new(transport: Arc<T>) -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            shard_providers: Arc::new(RwLock::new(HashMap::new())),
            transport,
        }
    }

    /// Register a node in the swarm (does not announce any shards yet).
    pub async fn register_node(&self, node_id: NodeId) {
        let mut nodes = self.nodes.write().await;
        nodes.entry(node_id).or_insert_with(|| SwarmNode::new(node_id));
    }

    /// Announce that `node_id` can serve `shard_id`.
    ///
    /// Updates both the node record and the reverse provider index.
    pub async fn announce_shard(&self, node_id: NodeId, shard_id: ContentHash) {
        // Update node record.
        {
            let mut nodes = self.nodes.write().await;
            let node = nodes
                .entry(node_id)
                .or_insert_with(|| SwarmNode::new(node_id));
            node.announce(shard_id);
        }
        // Update reverse index.
        {
            let mut providers = self.shard_providers.write().await;
            providers
                .entry(shard_id)
                .or_insert_with(HashSet::new)
                .insert(node_id);
        }
    }

    /// Discover which nodes can serve `shard_id`.
    ///
    /// Returns a list of `NodeId`s that have announced availability. Only
    /// reachable nodes (according to the transport layer) are included.
    pub async fn discover_shard(&self, shard_id: &ContentHash) -> Vec<NodeId> {
        let providers = self.shard_providers.read().await;
        let Some(node_set) = providers.get(shard_id) else {
            return Vec::new();
        };

        let mut reachable = Vec::new();
        for node_id in node_set {
            if self.transport.is_reachable(node_id).await {
                reachable.push(*node_id);
            }
        }
        reachable
    }

    /// Serve a shard to `requester` by fetching from an available provider.
    ///
    /// After a successful fetch the requester is automatically announced
    /// as a new provider (cascade behavior).
    ///
    /// Returns the raw shard data, or an error if no provider is reachable.
    pub async fn serve_shard(
        &self,
        shard_id: &ContentHash,
        requester: NodeId,
    ) -> Result<Vec<u8>, SwarmError> {
        let providers = self.discover_shard(shard_id).await;
        if providers.is_empty() {
            return Err(SwarmError::NoProviders {
                shard: *shard_id,
            });
        }

        // Try providers in order until one succeeds.
        let mut last_err = None;
        for provider in &providers {
            match self.transport.fetch_shard(provider, shard_id).await {
                Ok(data) => {
                    // Cascade: requester becomes a provider.
                    self.announce_shard(requester, *shard_id).await;
                    return Ok(data);
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        Err(SwarmError::AllProvidersFailed {
            shard: *shard_id,
            last_error: format!("{}", last_err.map_or_else(|| "unknown".to_string(), |e| e.to_string())),
        })
    }

    /// Number of registered nodes.
    pub async fn node_count(&self) -> usize {
        self.nodes.read().await.len()
    }

    /// Number of distinct shards tracked by the swarm.
    pub async fn tracked_shard_count(&self) -> usize {
        self.shard_providers.read().await.len()
    }

    /// Number of providers for a given shard.
    pub async fn provider_count(&self, shard_id: &ContentHash) -> usize {
        self.shard_providers
            .read()
            .await
            .get(shard_id)
            .map_or(0, |s| s.len())
    }
}

/// Errors specific to the swarm protocol.
#[derive(Debug, thiserror::Error)]
pub enum SwarmError {
    #[error("no providers found for shard {shard}")]
    NoProviders { shard: ContentHash },
    #[error("all providers failed for shard {shard}: {last_error}")]
    AllProvidersFailed { shard: ContentHash, last_error: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::shard_transport::MockShardTransport;

    fn node(seed: u8) -> NodeId {
        NodeId::from_bytes([seed; 32])
    }

    fn shard_hash(seed: u8) -> ContentHash {
        ContentHash([seed; 32])
    }

    #[tokio::test]
    async fn test_announce_and_discover() {
        let transport = Arc::new(MockShardTransport::new());
        let swarm = SwarmProtocol::new(transport);

        let n1 = node(1);
        let s1 = shard_hash(10);

        swarm.announce_shard(n1, s1).await;

        let providers = swarm.discover_shard(&s1).await;
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0], n1);

        // Unknown shard returns empty.
        let unknown = shard_hash(99);
        assert!(swarm.discover_shard(&unknown).await.is_empty());
    }

    #[tokio::test]
    async fn test_discover_filters_unreachable() {
        let transport = Arc::new(MockShardTransport::new());
        let n1 = node(1);
        let n2 = node(2);
        let s1 = shard_hash(10);

        // Mark n2 as unreachable.
        transport.set_unreachable(&n2).await;

        let swarm = SwarmProtocol::new(Arc::clone(&transport));
        swarm.announce_shard(n1, s1).await;
        swarm.announce_shard(n2, s1).await;

        let providers = swarm.discover_shard(&s1).await;
        // Only n1 should be returned since n2 is unreachable.
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0], n1);
    }

    #[tokio::test]
    async fn test_cascade_on_serve() {
        let transport = Arc::new(MockShardTransport::new());
        let n1 = node(1);
        let n2 = node(2);
        let s1 = shard_hash(10);
        let data = vec![0xAB; 512];

        // Pre-populate shard on n1 in the mock transport.
        transport.insert_shard(&n1, &s1, data.clone()).await;

        let swarm = SwarmProtocol::new(Arc::clone(&transport));
        swarm.announce_shard(n1, s1).await;

        // Before serve: only n1 provides s1.
        assert_eq!(swarm.provider_count(&s1).await, 1);

        // n2 fetches the shard.
        let fetched = swarm.serve_shard(&s1, n2).await.expect("test: serve should succeed");
        assert_eq!(fetched, data);

        // After serve: n2 was auto-announced as provider (cascade).
        assert_eq!(swarm.provider_count(&s1).await, 2);

        // Verify n2 is now in the provider list.
        let providers = swarm.discover_shard(&s1).await;
        assert!(providers.contains(&n2));
    }

    #[tokio::test]
    async fn test_serve_no_providers() {
        let transport = Arc::new(MockShardTransport::new());
        let swarm = SwarmProtocol::new(transport);

        let s1 = shard_hash(10);
        let requester = node(1);

        let result = swarm.serve_shard(&s1, requester).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multi_node_cascade_chain() {
        // Verifies that cascading scales: n1 -> n2 -> n3
        let transport = Arc::new(MockShardTransport::new());
        let n1 = node(1);
        let n2 = node(2);
        let n3 = node(3);
        let s1 = shard_hash(10);
        let data = vec![0xCD; 256];

        // n1 has the shard.
        transport.insert_shard(&n1, &s1, data.clone()).await;

        let swarm = SwarmProtocol::new(Arc::clone(&transport));
        swarm.announce_shard(n1, s1).await;

        // n2 fetches from n1 -> cascade announces n2.
        let fetched = swarm.serve_shard(&s1, n2).await.expect("test: n2 serve");
        assert_eq!(fetched, data);
        assert_eq!(swarm.provider_count(&s1).await, 2);

        // Simulate n2 now having the shard in transport.
        transport.insert_shard(&n2, &s1, data.clone()).await;

        // n3 fetches -> can get from either n1 or n2, cascade announces n3.
        let fetched = swarm.serve_shard(&s1, n3).await.expect("test: n3 serve");
        assert_eq!(fetched, data);
        assert_eq!(swarm.provider_count(&s1).await, 3);
    }
}
