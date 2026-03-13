// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Routing table implementation for Kademlia DHT

use super::{DhtConfig, DhtNodeId, NodeInfo};

/// Routing table for Kademlia
pub(crate) struct RoutingTable {
    /// K-buckets indexed by distance
    pub(crate) buckets: Vec<KBucket>,
    /// Configuration
    pub(crate) config: DhtConfig,
}

/// K-bucket for storing nodes at a specific distance
pub(crate) struct KBucket {
    /// Nodes in the bucket (most recently seen last)
    pub(crate) nodes: Vec<NodeInfo>,
    /// Replacement cache for full buckets
    pub(crate) replacements: Vec<NodeInfo>,
}

impl RoutingTable {
    pub(crate) fn new(config: DhtConfig) -> Self {
        let mut buckets = Vec::with_capacity(256);
        for _ in 0..256 {
            buckets.push(KBucket {
                nodes: Vec::new(),
                replacements: Vec::new(),
            });
        }

        Self { buckets, config }
    }

    /// Compute the XOR-distance-based bucket index for a node relative to
    /// a local node ID. Returns 0..255 based on the position of the highest
    /// differing bit.
    pub(crate) fn bucket_for_key(local_id: &DhtNodeId, remote_id: &DhtNodeId) -> usize {
        let distance = local_id.distance(remote_id);
        distance.bucket_index()
    }

    pub(crate) fn add_node(&mut self, node: NodeInfo) {
        let bucket_idx = Self::bucket_for_key(&DhtNodeId { id: [0u8; 32] }, &node.id);
        let bucket = &mut self.buckets[bucket_idx];

        // Check if node already exists -- update last_seen
        if let Some(existing) = bucket.nodes.iter_mut().find(|n| n.id == node.id) {
            existing.last_seen = node.last_seen;
            existing.rtt = node.rtt;
            return;
        }

        // Add new node
        if bucket.nodes.len() < self.config.k {
            bucket.nodes.push(node);
        } else {
            // Bucket full, add to replacements
            bucket.replacements.push(node);
            if bucket.replacements.len() > self.config.k {
                bucket.replacements.remove(0);
            }
        }
    }

    pub(crate) fn add_node_with_local_id(&mut self, local_id: &DhtNodeId, node: NodeInfo) {
        let bucket_idx = Self::bucket_for_key(local_id, &node.id);
        let bucket = &mut self.buckets[bucket_idx];

        if let Some(existing) = bucket.nodes.iter_mut().find(|n| n.id == node.id) {
            existing.last_seen = node.last_seen;
            existing.rtt = node.rtt;
            return;
        }

        if bucket.nodes.len() < self.config.k {
            bucket.nodes.push(node);
        } else {
            bucket.replacements.push(node);
            if bucket.replacements.len() > self.config.k {
                bucket.replacements.remove(0);
            }
        }
    }

    pub(crate) fn get_closest_nodes(&self, target: &DhtNodeId, k: usize) -> Vec<NodeInfo> {
        let mut nodes = Vec::new();

        // Collect all nodes
        for bucket in &self.buckets {
            nodes.extend(bucket.nodes.clone());
        }

        // Sort by distance
        nodes.sort_by_key(|n| target.distance(&n.id));
        nodes.truncate(k);

        nodes
    }
}
