// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cluster-Transport Bridge
//!
//! Wires [`ClusterManager`] to the gossip protocol, mDNS discovery,
//! and [`ShardTransport`] for real node-to-node coordination.
//!
//! [`ClusterTransportBridge`] drives cluster membership from gossip entries
//! and forwards shard reachability checks through the transport layer.

use std::sync::Arc;

use hypermesh_lib::{BlockchainScope, NodeId};
use tracing::{debug, info, warn};

use crate::network::gossip::GossipEntry;
use crate::network::shard_transport::ShardTransport;

use super::{ClusterConfig, ClusterError, ClusterEvent, ClusterManager, NodeStatus};

/// Bridges cluster membership to gossip-discovered peers and transport reachability.
///
/// Call [`sync_from_gossip`] periodically (e.g. on each gossip round) to add/remove
/// nodes from the cluster based on what gossip has learned. Call [`probe_health`] to
/// verify reachable nodes via the shard transport layer.
pub struct ClusterTransportBridge<T: ShardTransport> {
    cluster: ClusterManager,
    transport: Arc<T>,
}

impl<T: ShardTransport> ClusterTransportBridge<T> {
    /// Create a bridge wrapping a fresh cluster and a transport instance.
    pub fn new(config: ClusterConfig, transport: Arc<T>) -> Self {
        Self {
            cluster: ClusterManager::new(config),
            transport,
        }
    }

    /// Synchronise cluster membership with gossip-discovered peers.
    ///
    /// For every gossip entry that is not already in the cluster, add it.
    /// For every cluster node that is no longer present in gossip, remove it.
    /// Returns the list of events generated during sync.
    pub fn sync_from_gossip(
        &mut self,
        gossip_entries: &[GossipEntry],
        local_node_id: &str,
    ) -> Vec<ClusterEvent> {
        // Add new nodes from gossip.
        for entry in gossip_entries {
            // Skip local node.
            if entry.node_id == local_node_id {
                continue;
            }
            if self.cluster.get_node_status(&entry.node_id).is_none() {
                let scope = BlockchainScope::Network; // Gossip peers are network-scoped.
                match self
                    .cluster
                    .add_node(&entry.node_id, entry.coordinate, scope)
                {
                    Ok(()) => {
                        info!(node = %entry.node_id, "Added gossip peer to cluster");
                    }
                    Err(ClusterError::ClusterFull { .. }) => {
                        warn!(node = %entry.node_id, "Cluster full, cannot add gossip peer");
                        break;
                    }
                    Err(e) => {
                        debug!(node = %entry.node_id, error = %e, "Skip gossip peer add");
                    }
                }
            }
        }

        // Remove cluster nodes that are no longer in gossip.
        let gossip_ids: std::collections::HashSet<&str> =
            gossip_entries.iter().map(|e| e.node_id.as_str()).collect();

        let stale_ids: Vec<String> = self
            .cluster
            .list_nodes()
            .iter()
            .filter(|n| n.node_id != local_node_id && !gossip_ids.contains(n.node_id.as_str()))
            .map(|n| n.node_id.clone())
            .collect();

        for id in stale_ids {
            if let Err(e) = self.cluster.remove_node(&id) {
                debug!(node = %id, error = %e, "Failed to remove stale node");
            }
        }

        self.cluster.get_events()
    }

    /// Probe health of cluster nodes through the transport layer.
    ///
    /// For each Healthy/Degraded node, calls `is_reachable` on the transport.
    /// Records heartbeats for reachable nodes and runs a health check cycle.
    pub async fn probe_health(&mut self) -> Vec<ClusterEvent> {
        let node_ids: Vec<(String, NodeStatus)> = self
            .cluster
            .list_nodes()
            .iter()
            .map(|n| (n.node_id.clone(), n.status))
            .collect();

        for (node_id, status) in &node_ids {
            if matches!(
                status,
                NodeStatus::Healthy | NodeStatus::Degraded | NodeStatus::Joining
            ) {
                let lib_id = NodeId::from_bytes(
                    *blake3::hash(node_id.as_bytes()).as_bytes(),
                );
                if self.transport.is_reachable(&lib_id).await {
                    let _ = self.cluster.record_heartbeat(node_id);
                }
            }
        }

        let _ = self.cluster.check_health();
        self.cluster.get_events()
    }

    /// Get a reference to the inner cluster manager.
    pub fn cluster(&self) -> &ClusterManager {
        &self.cluster
    }

    /// Get a mutable reference to the inner cluster manager.
    pub fn cluster_mut(&mut self) -> &mut ClusterManager {
        &mut self.cluster
    }

    /// List healthy node IDs.
    pub fn healthy_node_ids(&self) -> Vec<String> {
        self.cluster
            .list_by_status(NodeStatus::Healthy)
            .iter()
            .map(|n| n.node_id.clone())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;
    use crate::network::shard_transport::MockShardTransport;

    fn test_entry(id: &str, x: i64, y: i64, z: i64) -> GossipEntry {
        GossipEntry {
            node_id: id.to_string(),
            version: 1,
            coordinate: MatrixCoordinate::new(x, y, z).expect("test: valid coord"),
            stoq_port: 9292,
            available_assets: vec![],
            privacy_mode: "Public".to_string(),
            updated_at: 0,
        }
    }

    #[test]
    fn test_sync_adds_gossip_peers() {
        let transport = Arc::new(MockShardTransport::new());
        let config = ClusterConfig {
            max_nodes: 10,
            ..ClusterConfig::default()
        };
        let mut bridge = ClusterTransportBridge::new(config, transport);

        let entries = vec![
            test_entry("local", 0, 0, 0),
            test_entry("peer1", 1, 2, 3),
            test_entry("peer2", 4, 5, 6),
        ];

        let events = bridge.sync_from_gossip(&entries, "local");

        // Two peers added (not the local node).
        let joined: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ClusterEvent::NodeJoined { .. }))
            .collect();
        assert_eq!(joined.len(), 2);

        assert_eq!(bridge.cluster().list_nodes().len(), 2);
    }

    #[tokio::test]
    async fn test_probe_health_records_heartbeat() {
        let transport = Arc::new(MockShardTransport::new());
        let config = ClusterConfig::default();
        let mut bridge = ClusterTransportBridge::new(config, transport);

        // Add a node manually, transition to Healthy via heartbeat.
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: coord");
        bridge
            .cluster_mut()
            .add_node("peer1", coord, BlockchainScope::Network)
            .expect("test: add");
        bridge
            .cluster_mut()
            .record_heartbeat("peer1")
            .expect("test: hb");

        // MockShardTransport returns reachable=true for all by default
        let events = bridge.probe_health().await;

        // Should have a health check completed event.
        let hc = events
            .iter()
            .find(|e| matches!(e, ClusterEvent::HealthCheckCompleted { .. }));
        assert!(hc.is_some());

        // Node should still be healthy.
        let node = bridge.cluster().get_node_status("peer1").expect("test: exists");
        assert_eq!(node.status, NodeStatus::Healthy);
    }
}
