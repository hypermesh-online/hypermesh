// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Distribution bridge for the user contribution platform.
//!
//! Wires contributed resources to [`SwarmProtocol`] announce and
//! [`ShardTransport`] serving so that user-shared storage actually
//! participates in shard distribution.

use std::sync::Arc;

use hypermesh_lib::{ContentHash, NodeId};
use tracing::{debug, info};

use crate::distribution::swarm::SwarmProtocol;
use crate::network::shard_transport::ShardTransport;
use crate::transport::error::TransportError;

/// Bridges user contribution sessions to the swarm distribution layer.
///
/// When a user starts contributing, [`register_contributor`] announces the
/// node in the swarm. When the user provides shards, [`announce_shard`]
/// makes those shards discoverable network-wide.
pub struct ContributionDistributionBridge<T: ShardTransport> {
    swarm: Arc<SwarmProtocol<T>>,
    local_node_id: NodeId,
}

impl<T: ShardTransport> ContributionDistributionBridge<T> {
    /// Create a new bridge using the given swarm and local node identity.
    pub fn new(swarm: Arc<SwarmProtocol<T>>, local_node_id: NodeId) -> Self {
        Self {
            swarm,
            local_node_id,
        }
    }

    /// Register the local node as a contributor in the swarm.
    ///
    /// Call this when a user starts a contribution session.
    pub async fn register_contributor(&self) {
        self.swarm.register_node(self.local_node_id).await;
        info!(node = %self.local_node_id.to_hex(), "Contributor registered in swarm");
    }

    /// Announce that the local node can serve a specific shard.
    ///
    /// Call this for each shard the user is willing to serve.
    pub async fn announce_shard(&self, shard_id: ContentHash) {
        self.swarm
            .announce_shard(self.local_node_id, shard_id)
            .await;
        debug!(
            shard = %hex::encode(shard_id.0),
            "Contributor announced shard availability"
        );
    }

    /// Announce multiple shards at once (convenience wrapper).
    pub async fn announce_shards(&self, shard_ids: &[ContentHash]) {
        for shard_id in shard_ids {
            self.announce_shard(*shard_id).await;
        }
        info!(count = shard_ids.len(), "Contributor announced batch of shards");
    }

    /// Serve a shard request through the swarm (auto-cascade).
    ///
    /// The local node becomes a provider after successful fetch.
    pub async fn serve_shard(
        &self,
        shard_id: &ContentHash,
    ) -> Result<Vec<u8>, TransportError> {
        self.swarm
            .serve_shard(shard_id, self.local_node_id)
            .await
            .map_err(|e| TransportError::Network(e.to_string()))
    }

    /// Get the number of shards tracked by the swarm.
    pub async fn tracked_shard_count(&self) -> usize {
        self.swarm.tracked_shard_count().await
    }

    /// Get the number of providers for a specific shard.
    pub async fn provider_count(&self, shard_id: &ContentHash) -> usize {
        self.swarm.provider_count(shard_id).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::shard_transport::MockShardTransport;

    fn node(seed: u8) -> NodeId {
        NodeId::from_bytes([seed; 32])
    }

    fn hash(seed: u8) -> ContentHash {
        ContentHash([seed; 32])
    }

    #[tokio::test]
    async fn test_register_and_announce() {
        let transport = Arc::new(MockShardTransport::new());
        let swarm = Arc::new(SwarmProtocol::new(transport));
        let local = node(1);

        let bridge = ContributionDistributionBridge::new(swarm.clone(), local);

        bridge.register_contributor().await;
        assert_eq!(swarm.node_count().await, 1);

        let s1 = hash(10);
        let s2 = hash(20);
        bridge.announce_shards(&[s1, s2]).await;

        assert_eq!(bridge.tracked_shard_count().await, 2);
        assert_eq!(bridge.provider_count(&s1).await, 1);
        assert_eq!(bridge.provider_count(&s2).await, 1);
    }

    #[tokio::test]
    async fn test_serve_through_bridge() {
        let transport = Arc::new(MockShardTransport::new());
        let n1 = node(1);
        let n2 = node(2);
        let s1 = hash(10);
        let data = vec![0xFF; 128];

        // n1 has the shard in transport.
        transport.insert_shard(&n1, &s1, data.clone()).await;

        let swarm = Arc::new(SwarmProtocol::new(Arc::clone(&transport)));
        swarm.announce_shard(n1, s1).await;

        // Bridge for n2 (the contributor/consumer).
        let bridge = ContributionDistributionBridge::new(swarm.clone(), n2);
        bridge.register_contributor().await;

        let fetched = bridge.serve_shard(&s1).await.expect("test: serve should succeed");
        assert_eq!(fetched, data);

        // n2 should now be a provider via cascade.
        assert_eq!(bridge.provider_count(&s1).await, 2);
    }
}
