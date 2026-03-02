// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Gossip protocol for mesh coordination.
//!
//! Nodes periodically exchange state (membership, matrix position, available
//! assets) with a random subset of known peers. Each piece of state carries a
//! lamport-style version counter; only newer versions propagate.
//!
//! Transport: messages are serialized as JSON and sent over STOQ streams.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::matrix::coordinate::MatrixCoordinate;

/// Maximum number of peers to gossip to per round.
const GOSSIP_FANOUT: usize = 3;

/// Gossip round interval in seconds.
const GOSSIP_INTERVAL_SECS: u64 = 15;

/// A versioned piece of node state shared via gossip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipEntry {
    /// Node ID that owns this state
    pub node_id: String,
    /// Lamport version counter (monotonically increasing)
    pub version: u64,
    /// Matrix coordinate of the node
    pub coordinate: MatrixCoordinate,
    /// STOQ port
    pub stoq_port: u16,
    /// Available asset IDs on this node
    pub available_assets: Vec<String>,
    /// Privacy mode (serialized string)
    pub privacy_mode: String,
    /// Unix timestamp of last update
    pub updated_at: u64,
}

/// Gossip message exchanged between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    /// Sender node ID
    pub sender: String,
    /// Entries the sender knows about
    pub entries: Vec<GossipEntry>,
}

/// Gossip state for a single node.
#[derive(Debug, Clone)]
pub struct GossipState {
    /// All known node states, keyed by node ID
    pub entries: HashMap<String, GossipEntry>,
    /// Local node ID
    pub local_node_id: String,
    /// Current local version counter
    pub local_version: u64,
}

impl GossipState {
    /// Create initial gossip state for this node.
    pub fn new(
        node_id: String,
        coordinate: MatrixCoordinate,
        stoq_port: u16,
        privacy_mode: String,
    ) -> Self {
        let entry = GossipEntry {
            node_id: node_id.clone(),
            version: 1,
            coordinate,
            stoq_port,
            available_assets: Vec::new(),
            privacy_mode,
            updated_at: current_timestamp(),
        };

        let mut entries = HashMap::new();
        entries.insert(node_id.clone(), entry);

        Self {
            entries,
            local_node_id: node_id,
            local_version: 1,
        }
    }

    /// Update the local node's entry (increments version).
    pub fn update_local(
        &mut self,
        coordinate: MatrixCoordinate,
        available_assets: Vec<String>,
    ) {
        self.local_version += 1;

        if let Some(entry) = self.entries.get_mut(&self.local_node_id) {
            entry.version = self.local_version;
            entry.coordinate = coordinate;
            entry.available_assets = available_assets;
            entry.updated_at = current_timestamp();
        }
    }

    /// Merge incoming gossip entries (only if newer).
    ///
    /// Returns the number of entries that were updated.
    pub fn merge(&mut self, incoming: &[GossipEntry]) -> usize {
        let mut updated = 0;

        for entry in incoming {
            let dominated = self
                .entries
                .get(&entry.node_id)
                .map(|existing| entry.version > existing.version)
                .unwrap_or(true);

            if dominated {
                self.entries.insert(entry.node_id.clone(), entry.clone());
                updated += 1;
            }
        }

        updated
    }

    /// Build a gossip message containing all known entries.
    pub fn build_message(&self) -> GossipMessage {
        GossipMessage {
            sender: self.local_node_id.clone(),
            entries: self.entries.values().cloned().collect(),
        }
    }

    /// Get all known node IDs except the local node.
    pub fn remote_node_ids(&self) -> Vec<String> {
        self.entries
            .keys()
            .filter(|id| *id != &self.local_node_id)
            .cloned()
            .collect()
    }

    /// Remove entries older than `max_age_secs`.
    pub fn prune_stale(&mut self, max_age_secs: u64) {
        let cutoff = current_timestamp().saturating_sub(max_age_secs);
        self.entries
            .retain(|id, entry| *id == self.local_node_id || entry.updated_at >= cutoff);
    }
}

/// Gossip protocol manager.
///
/// Maintains local gossip state and provides methods for building/processing
/// gossip messages. The actual network I/O is delegated to the caller
/// (typically `NetworkManager`) so the gossip logic is testable without
/// real connections.
pub struct GossipProtocol {
    /// Shared gossip state
    state: Arc<RwLock<GossipState>>,
    /// Whether the gossip loop is running
    running: Arc<RwLock<bool>>,
}

impl GossipProtocol {
    /// Create a new gossip protocol instance.
    pub fn new(
        node_id: String,
        coordinate: MatrixCoordinate,
        stoq_port: u16,
        privacy_mode: String,
    ) -> Self {
        let state = GossipState::new(node_id, coordinate, stoq_port, privacy_mode);
        Self {
            state: Arc::new(RwLock::new(state)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get shared state reference.
    pub fn state(&self) -> Arc<RwLock<GossipState>> {
        self.state.clone()
    }

    /// Mark the gossip loop as running.
    pub async fn start(&self) {
        *self.running.write().await = true;
        info!("Gossip protocol started");
    }

    /// Stop the gossip loop.
    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("Gossip protocol stopped");
    }

    /// Check if gossip is running.
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Build a gossip message to send to peers.
    pub async fn build_outgoing_message(&self) -> GossipMessage {
        self.state.read().await.build_message()
    }

    /// Process an incoming gossip message.
    ///
    /// Returns the number of entries that were updated.
    pub async fn process_incoming(&self, message: GossipMessage) -> usize {
        let mut state = self.state.write().await;
        let updated = state.merge(&message.entries);

        if updated > 0 {
            debug!(
                "Gossip: merged {} new/updated entries from {}",
                updated, message.sender
            );
        }

        updated
    }

    /// Update local state (coordinate and assets).
    pub async fn update_local(
        &self,
        coordinate: MatrixCoordinate,
        available_assets: Vec<String>,
    ) {
        self.state
            .write()
            .await
            .update_local(coordinate, available_assets);
    }

    /// Get all known peers (excluding local node).
    pub async fn known_peers(&self) -> Vec<GossipEntry> {
        let state = self.state.read().await;
        state
            .entries
            .values()
            .filter(|e| e.node_id != state.local_node_id)
            .cloned()
            .collect()
    }

    /// Get known peer count (excluding local node).
    pub async fn peer_count(&self) -> usize {
        let state = self.state.read().await;
        state.entries.len().saturating_sub(1)
    }

    /// Select random peers for gossip fanout.
    pub async fn select_gossip_targets(&self) -> Vec<String> {
        let state = self.state.read().await;
        let remote_ids = state.remote_node_ids();

        if remote_ids.len() <= GOSSIP_FANOUT {
            return remote_ids;
        }

        // Random selection without replacement
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let mut selected = remote_ids;
        selected.shuffle(&mut rng);
        selected.truncate(GOSSIP_FANOUT);
        selected
    }

    /// Remove stale entries.
    pub async fn prune_stale(&self, max_age_secs: u64) {
        self.state.write().await.prune_stale(max_age_secs);
    }

    /// Get the gossip interval.
    pub fn gossip_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(GOSSIP_INTERVAL_SECS)
    }

    /// Get the fanout count.
    pub fn fanout(&self) -> usize {
        GOSSIP_FANOUT
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_coord(x: i64, y: i64, z: i64) -> MatrixCoordinate {
        MatrixCoordinate::new(x, y, z).expect("test: valid coordinate")
    }

    #[test]
    fn test_gossip_state_creation() {
        let state = GossipState::new(
            "node1".to_string(),
            test_coord(10, 20, 30),
            9292,
            "Public".to_string(),
        );

        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.local_version, 1);

        let entry = state.entries.get("node1").expect("test: entry should exist");
        assert_eq!(entry.stoq_port, 9292);
    }

    #[test]
    fn test_gossip_merge_newer() {
        let mut state = GossipState::new(
            "node1".to_string(),
            test_coord(0, 0, 0),
            9292,
            "Public".to_string(),
        );

        // Merge an entry for a new node
        let remote_entry = GossipEntry {
            node_id: "node2".to_string(),
            version: 5,
            coordinate: test_coord(10, 10, 10),
            stoq_port: 9293,
            available_assets: vec!["asset1".to_string()],
            privacy_mode: "Private".to_string(),
            updated_at: current_timestamp(),
        };

        let updated = state.merge(&[remote_entry]);
        assert_eq!(updated, 1);
        assert_eq!(state.entries.len(), 2);
    }

    #[test]
    fn test_gossip_merge_older_ignored() {
        let mut state = GossipState::new(
            "node1".to_string(),
            test_coord(0, 0, 0),
            9292,
            "Public".to_string(),
        );

        // Insert a remote entry at version 10
        let entry_v10 = GossipEntry {
            node_id: "node2".to_string(),
            version: 10,
            coordinate: test_coord(5, 5, 5),
            stoq_port: 9293,
            available_assets: vec![],
            privacy_mode: "Public".to_string(),
            updated_at: current_timestamp(),
        };
        state.merge(&[entry_v10]);

        // Try to merge an older version (should be ignored)
        let entry_v5 = GossipEntry {
            node_id: "node2".to_string(),
            version: 5,
            coordinate: test_coord(99, 99, 99),
            stoq_port: 9999,
            available_assets: vec![],
            privacy_mode: "Private".to_string(),
            updated_at: current_timestamp(),
        };

        let updated = state.merge(&[entry_v5]);
        assert_eq!(updated, 0);

        // State should still have version 10
        let entry = state.entries.get("node2").expect("test: entry exists");
        assert_eq!(entry.version, 10);
        assert_eq!(entry.stoq_port, 9293);
    }

    #[tokio::test]
    async fn test_gossip_protocol_roundtrip() {
        let proto1 = GossipProtocol::new(
            "node1".to_string(),
            test_coord(0, 0, 0),
            9292,
            "Public".to_string(),
        );

        let proto2 = GossipProtocol::new(
            "node2".to_string(),
            test_coord(10, 10, 10),
            9293,
            "Private".to_string(),
        );

        // Node1 sends gossip to Node2
        let msg1 = proto1.build_outgoing_message().await;
        let updated = proto2.process_incoming(msg1).await;
        assert_eq!(updated, 1); // Node2 learns about Node1

        // Node2 sends gossip to Node1
        let msg2 = proto2.build_outgoing_message().await;
        let updated = proto1.process_incoming(msg2).await;
        assert_eq!(updated, 1); // Node1 learns about Node2

        // Both nodes now know about each other
        assert_eq!(proto1.peer_count().await, 1);
        assert_eq!(proto2.peer_count().await, 1);
    }

    #[tokio::test]
    async fn test_gossip_update_local() {
        let proto = GossipProtocol::new(
            "node1".to_string(),
            test_coord(0, 0, 0),
            9292,
            "Public".to_string(),
        );

        proto
            .update_local(
                test_coord(5, 5, 5),
                vec!["asset-a".to_string(), "asset-b".to_string()],
            )
            .await;

        let msg = proto.build_outgoing_message().await;
        let local_entry = msg
            .entries
            .iter()
            .find(|e| e.node_id == "node1")
            .expect("test: local entry should exist");

        assert_eq!(local_entry.version, 2);
        assert_eq!(local_entry.available_assets.len(), 2);
        assert_eq!(local_entry.coordinate, test_coord(5, 5, 5));
    }

    #[tokio::test]
    async fn test_gossip_prune_stale() {
        let proto = GossipProtocol::new(
            "node1".to_string(),
            test_coord(0, 0, 0),
            9292,
            "Public".to_string(),
        );

        // Add a stale entry
        let stale = GossipEntry {
            node_id: "stale-node".to_string(),
            version: 1,
            coordinate: test_coord(99, 99, 99),
            stoq_port: 1234,
            available_assets: vec![],
            privacy_mode: "Public".to_string(),
            updated_at: 1, // Ancient timestamp
        };

        proto
            .process_incoming(GossipMessage {
                sender: "stale-node".to_string(),
                entries: vec![stale],
            })
            .await;

        assert_eq!(proto.peer_count().await, 1);

        // Prune with short threshold
        proto.prune_stale(60).await;
        assert_eq!(proto.peer_count().await, 0);
    }
}
