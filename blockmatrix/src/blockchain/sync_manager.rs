// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Blockchain synchronization manager for Device and Network scopes
//!
//! Coordinates synchronization between a node's local Device chain and
//! any Network scope chains it participates in. A node ALWAYS has a Device
//! chain (starts on boot). It can OPTIONALLY join one or more Network
//! scope chains by syncing with other participating nodes.
//!
//! PrivacyMode controls WHO can participate in a network.
//! BlockchainScope controls WHETHER chains synchronize.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::blockchain::propagation::PropagationStrategy;
use crate::bootstrap::PrivacyMode;
use hypermesh_lib::BlockchainScope;

/// Manages blockchain synchronization between Device and Network scopes.
///
/// Each node has exactly one Device chain (always present, created at boot).
/// The SyncManager tracks zero or more Network memberships and coordinates
/// synchronization state for each.
pub struct SyncManager {
    /// Identifier for this node's device chain
    device_chain_id: String,
    /// Network chains this node participates in (keyed by network_id)
    network_memberships: HashMap<String, NetworkMembership>,
    /// Sync state per network
    sync_states: HashMap<String, SyncState>,
    /// Configuration
    config: SyncConfig,
}

/// Represents membership in a Network scope chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMembership {
    /// Unique identifier for this network
    pub network_id: String,
    /// Always `Network` for memberships (Device is implicit)
    pub scope: BlockchainScope,
    /// Privacy mode controlling participation rules
    pub privacy_mode: PrivacyMode,
    /// Timestamp (unix seconds) when this node joined the network
    pub joined_at: u64,
    /// Timestamp of the last successful sync, if any
    pub last_sync: Option<u64>,
}

/// Tracks the synchronization state for a specific network
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncState {
    /// Not syncing -- Device scope only
    Disconnected,
    /// Searching for peers in the network
    Discovering,
    /// Actively syncing chain state with network peers
    Syncing {
        /// Progress as a fraction (0.0 to 1.0)
        progress: f64,
        /// Number of peers currently syncing with
        peer_count: usize,
    },
    /// Fully synchronized with the network
    Synchronized {
        /// Height of the last synced block
        last_block_height: u64,
    },
    /// Sync failed, can be retried
    Failed {
        /// Human-readable failure reason
        reason: String,
    },
}

/// Configuration for the sync manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Maximum simultaneous network memberships
    pub max_networks: usize,
    /// Milliseconds between sync checks
    pub sync_interval_ms: u64,
    /// Maximum blocks behind before forcing a full sync
    pub max_block_lag: u64,
    /// Propagation strategy for block announcements
    pub propagation_strategy: PropagationStrategyConfig,
}

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

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_networks: 8,
            sync_interval_ms: 5_000,
            max_block_lag: 100,
            propagation_strategy: PropagationStrategyConfig::Broadcast,
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

impl SyncManager {
    /// Create a new sync manager for the given device chain
    pub fn new(device_chain_id: String, config: SyncConfig) -> Self {
        info!(
            device_chain = %device_chain_id,
            max_networks = config.max_networks,
            "SyncManager created"
        );

        Self {
            device_chain_id,
            network_memberships: HashMap::new(),
            sync_states: HashMap::new(),
            config,
        }
    }

    /// Get the device chain identifier
    pub fn device_chain_id(&self) -> &str {
        &self.device_chain_id
    }

    /// Get the sync configuration
    pub fn config(&self) -> &SyncConfig {
        &self.config
    }

    /// Join a Network scope chain
    ///
    /// Returns an error string if the maximum number of networks is reached
    /// or if the node is already a member.
    pub fn join_network(
        &mut self,
        network_id: String,
        privacy_mode: PrivacyMode,
        now_unix_secs: u64,
    ) -> Result<(), String> {
        if self.network_memberships.contains_key(&network_id) {
            return Err(format!("Already a member of network {}", network_id));
        }

        if self.network_memberships.len() >= self.config.max_networks {
            return Err(format!(
                "Maximum network memberships ({}) reached",
                self.config.max_networks
            ));
        }

        let membership = NetworkMembership {
            network_id: network_id.clone(),
            scope: BlockchainScope::Network,
            privacy_mode,
            joined_at: now_unix_secs,
            last_sync: None,
        };

        info!(
            network = %network_id,
            privacy = %privacy_mode,
            "Joined network"
        );

        self.network_memberships
            .insert(network_id.clone(), membership);
        self.sync_states
            .insert(network_id, SyncState::Discovering);

        Ok(())
    }

    /// Leave a Network scope chain
    ///
    /// Returns an error string if the node is not a member of the network.
    pub fn leave_network(&mut self, network_id: &str) -> Result<(), String> {
        if self.network_memberships.remove(network_id).is_none() {
            return Err(format!("Not a member of network {}", network_id));
        }

        self.sync_states.remove(network_id);

        info!(network = %network_id, "Left network");

        Ok(())
    }

    /// Get the current sync state for a network
    pub fn sync_state(&self, network_id: &str) -> Option<&SyncState> {
        self.sync_states.get(network_id)
    }

    /// Get all active network memberships
    pub fn active_networks(&self) -> Vec<&NetworkMembership> {
        self.network_memberships.values().collect()
    }

    /// Get the count of active network memberships
    pub fn active_network_count(&self) -> usize {
        self.network_memberships.len()
    }

    /// Check if the node is a member of a specific network
    pub fn is_member(&self, network_id: &str) -> bool {
        self.network_memberships.contains_key(network_id)
    }

    /// Update the sync state for a network
    pub fn update_sync_state(
        &mut self,
        network_id: &str,
        state: SyncState,
    ) -> Result<(), String> {
        if !self.network_memberships.contains_key(network_id) {
            return Err(format!("Not a member of network {}", network_id));
        }

        debug!(
            network = %network_id,
            state = ?state,
            "Sync state updated"
        );

        self.sync_states.insert(network_id.to_string(), state);

        Ok(())
    }

    /// Record a successful sync timestamp for a network
    pub fn record_sync(&mut self, network_id: &str, now_unix_secs: u64) {
        if let Some(membership) = self.network_memberships.get_mut(network_id) {
            membership.last_sync = Some(now_unix_secs);
        }
    }

    /// Process an incoming sync message and return an optional response
    pub fn process_sync_message(
        &mut self,
        msg: SyncMessage,
    ) -> Option<SyncMessage> {
        match msg {
            SyncMessage::Request {
                network_id,
                from_height,
                max_blocks,
            } => {
                if !self.is_member(&network_id) {
                    warn!(
                        network = %network_id,
                        "Received sync request for unknown network"
                    );
                    return None;
                }

                debug!(
                    network = %network_id,
                    from = from_height,
                    max = max_blocks,
                    "Processing sync request"
                );

                // Return an empty response -- actual block retrieval is
                // handled by the caller using the blockchain storage layer
                Some(SyncMessage::Response {
                    network_id,
                    block_hashes: Vec::new(),
                    peer_height: from_height,
                })
            }

            SyncMessage::Announce {
                network_id,
                block_height,
                block_hash,
            } => {
                if !self.is_member(&network_id) {
                    return None;
                }

                debug!(
                    network = %network_id,
                    height = block_height,
                    hash = %block_hash,
                    "Received block announcement"
                );

                // Check if we need to sync (are we lagging?)
                if let Some(SyncState::Synchronized { last_block_height }) =
                    self.sync_states.get(&network_id)
                {
                    if block_height > last_block_height + self.config.max_block_lag
                    {
                        self.sync_states.insert(
                            network_id.clone(),
                            SyncState::Syncing {
                                progress: 0.0,
                                peer_count: 1,
                            },
                        );
                    }
                }

                None
            }

            SyncMessage::Response {
                network_id,
                block_hashes,
                peer_height,
            } => {
                if !self.is_member(&network_id) {
                    return None;
                }

                debug!(
                    network = %network_id,
                    blocks = block_hashes.len(),
                    peer_height = peer_height,
                    "Received sync response"
                );

                // If we received blocks, update sync progress
                if block_hashes.is_empty() {
                    self.sync_states.insert(
                        network_id,
                        SyncState::Synchronized {
                            last_block_height: peer_height,
                        },
                    );
                }

                None
            }
        }
    }

    /// Generate a sync request for a specific network
    ///
    /// Returns None if the node is not a member or already synchronized.
    pub fn generate_sync_request(
        &self,
        network_id: &str,
        local_height: u64,
    ) -> Option<SyncMessage> {
        if !self.is_member(network_id) {
            return None;
        }

        let state = self.sync_states.get(network_id)?;

        match state {
            SyncState::Discovering | SyncState::Syncing { .. } => {
                Some(SyncMessage::Request {
                    network_id: network_id.to_string(),
                    from_height: local_height,
                    max_blocks: 50,
                })
            }
            _ => None,
        }
    }

    /// Get networks that need syncing (Discovering or Syncing state)
    pub fn networks_needing_sync(&self) -> Vec<&str> {
        self.sync_states
            .iter()
            .filter(|(_, state)| {
                matches!(
                    state,
                    SyncState::Discovering | SyncState::Syncing { .. }
                )
            })
            .map(|(id, _)| id.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> SyncConfig {
        SyncConfig {
            max_networks: 4,
            ..SyncConfig::default()
        }
    }

    #[test]
    fn test_create_sync_manager() {
        let mgr = SyncManager::new("device-chain-1".to_string(), default_config());

        assert_eq!(mgr.device_chain_id(), "device-chain-1");
        assert_eq!(mgr.active_network_count(), 0);
        assert!(mgr.active_networks().is_empty());
    }

    #[test]
    fn test_join_and_leave_network() {
        let mut mgr = SyncManager::new("dev-chain".to_string(), default_config());

        // Join a network
        let result = mgr.join_network(
            "net-alpha".to_string(),
            PrivacyMode::PUBLIC,
            1000,
        );
        assert!(result.is_ok());
        assert_eq!(mgr.active_network_count(), 1);
        assert!(mgr.is_member("net-alpha"));

        // Verify initial state is Discovering
        let state = mgr.sync_state("net-alpha")
            .expect("test: sync state should exist");
        assert_eq!(*state, SyncState::Discovering);

        // Leave the network
        let result = mgr.leave_network("net-alpha");
        assert!(result.is_ok());
        assert_eq!(mgr.active_network_count(), 0);
        assert!(!mgr.is_member("net-alpha"));
    }

    #[test]
    fn test_duplicate_join_rejected() {
        let mut mgr = SyncManager::new("dev-chain".to_string(), default_config());

        mgr.join_network("net-1".to_string(), PrivacyMode::PRIVATE, 100)
            .expect("test: first join should succeed");

        let result = mgr.join_network("net-1".to_string(), PrivacyMode::PRIVATE, 200);
        assert!(result.is_err());
        assert!(result
            .err()
            .expect("test: should have error")
            .contains("Already a member"));
    }

    #[test]
    fn test_max_networks_enforced() {
        let config = SyncConfig {
            max_networks: 2,
            ..SyncConfig::default()
        };
        let mut mgr = SyncManager::new("dev".to_string(), config);

        mgr.join_network("n1".to_string(), PrivacyMode::PUBLIC, 1)
            .expect("test: join 1");
        mgr.join_network("n2".to_string(), PrivacyMode::PUBLIC, 2)
            .expect("test: join 2");

        let result = mgr.join_network("n3".to_string(), PrivacyMode::PUBLIC, 3);
        assert!(result.is_err());
        assert!(result
            .err()
            .expect("test: should have error")
            .contains("Maximum network memberships"));
    }

    #[test]
    fn test_leave_unknown_network_fails() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        let result = mgr.leave_network("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_state_transitions() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        mgr.join_network("net".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        // Transition: Discovering -> Syncing
        mgr.update_sync_state("net", SyncState::Syncing {
            progress: 0.5,
            peer_count: 3,
        })
        .expect("test: update to syncing");

        if let Some(SyncState::Syncing { progress, peer_count }) = mgr.sync_state("net") {
            assert!((*progress - 0.5).abs() < f64::EPSILON);
            assert_eq!(*peer_count, 3);
        } else {
            unreachable!("test: expected Syncing state");
        }

        // Transition: Syncing -> Synchronized
        mgr.update_sync_state("net", SyncState::Synchronized {
            last_block_height: 42,
        })
        .expect("test: update to synchronized");

        assert_eq!(
            mgr.sync_state("net"),
            Some(&SyncState::Synchronized { last_block_height: 42 })
        );
    }

    #[test]
    fn test_process_sync_announce_triggers_resync() {
        let config = SyncConfig {
            max_block_lag: 10,
            ..default_config()
        };
        let mut mgr = SyncManager::new("dev".to_string(), config);

        mgr.join_network("net".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");
        mgr.update_sync_state("net", SyncState::Synchronized {
            last_block_height: 50,
        })
        .expect("test: set synchronized");

        // Announce a block far ahead -- should trigger resync
        let msg = SyncMessage::Announce {
            network_id: "net".to_string(),
            block_height: 200,
            block_hash: "abc123".to_string(),
        };

        let _response = mgr.process_sync_message(msg);

        // State should have transitioned to Syncing
        match mgr.sync_state("net") {
            Some(SyncState::Syncing { .. }) => {}
            other => unreachable!("test: expected Syncing, got {:?}", other),
        }
    }

    #[test]
    fn test_generate_sync_request() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        mgr.join_network("net".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        // Should generate request when Discovering
        let req = mgr.generate_sync_request("net", 10);
        assert!(req.is_some());

        if let Some(SyncMessage::Request {
            network_id,
            from_height,
            max_blocks,
        }) = req
        {
            assert_eq!(network_id, "net");
            assert_eq!(from_height, 10);
            assert_eq!(max_blocks, 50);
        } else {
            unreachable!("test: expected Request message");
        }

        // Should NOT generate request when Synchronized
        mgr.update_sync_state("net", SyncState::Synchronized {
            last_block_height: 42,
        })
        .expect("test: set synchronized");

        let req = mgr.generate_sync_request("net", 42);
        assert!(req.is_none());
    }

    #[test]
    fn test_networks_needing_sync() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        mgr.join_network("n1".to_string(), PrivacyMode::PUBLIC, 1)
            .expect("test: join n1");
        mgr.join_network("n2".to_string(), PrivacyMode::PRIVATE, 2)
            .expect("test: join n2");

        // Both start as Discovering
        let needing = mgr.networks_needing_sync();
        assert_eq!(needing.len(), 2);

        // Synchronize n1
        mgr.update_sync_state("n1", SyncState::Synchronized {
            last_block_height: 100,
        })
        .expect("test: synchronize n1");

        let needing = mgr.networks_needing_sync();
        assert_eq!(needing.len(), 1);
        assert_eq!(needing[0], "n2");
    }

    #[test]
    fn test_record_sync_timestamp() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        mgr.join_network("net".to_string(), PrivacyMode::PUBLIC, 100)
            .expect("test: join");

        // Initially no last_sync
        let membership = &mgr.active_networks()[0];
        assert!(membership.last_sync.is_none());

        // Record sync
        mgr.record_sync("net", 200);

        let membership = &mgr.active_networks()[0];
        assert_eq!(membership.last_sync, Some(200));
    }

    #[test]
    fn test_membership_scope_is_network() {
        let mut mgr = SyncManager::new("dev".to_string(), default_config());

        mgr.join_network("net".to_string(), PrivacyMode::ANONYMOUS, 50)
            .expect("test: join");

        let membership = &mgr.active_networks()[0];
        assert_eq!(membership.scope, BlockchainScope::Network);
        assert_eq!(membership.privacy_mode, PrivacyMode::ANONYMOUS);
        assert_eq!(membership.joined_at, 50);
    }
}
