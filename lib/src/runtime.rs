// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Unified runtime state types for HyperMesh nodes.
//!
//! These canonical types describe the runtime status of nodes, networks, and
//! assets. Other crates reference these for coordinated state management.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::types::BlockchainScope;

// ---------------------------------------------------------------------------
// NodeState — lifecycle of a mesh node
// ---------------------------------------------------------------------------

/// Runtime lifecycle state of a HyperMesh node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is initializing hardware assessment and genesis block.
    Booting,
    /// Node is fully operational and accepting work.
    Ready,
    /// Node is synchronizing blockchain state with a Network chain.
    Syncing,
    /// Node is operational but with reduced capabilities (e.g. low resources).
    Degraded,
    /// Node is shutting down gracefully.
    Shutdown,
}

impl NodeState {
    /// Whether the node can accept new work in this state.
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Ready | Self::Syncing | Self::Degraded)
    }
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Booting => write!(f, "Booting"),
            Self::Ready => write!(f, "Ready"),
            Self::Syncing => write!(f, "Syncing"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Shutdown => write!(f, "Shutdown"),
        }
    }
}

// ---------------------------------------------------------------------------
// NetworkState — connectivity and chain sync
// ---------------------------------------------------------------------------

/// Runtime network state of a HyperMesh node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkState {
    /// Number of currently connected peers.
    pub connected_peers: u32,
    /// Active blockchain scopes (Device is always present; Network if syncing).
    pub active_chains: Vec<BlockchainScope>,
    /// Current synchronization status.
    pub sync_status: SyncStatus,
}

impl NetworkState {
    /// Create a default Device-only network state with no peers.
    pub fn device_only() -> Self {
        Self {
            connected_peers: 0,
            active_chains: vec![BlockchainScope::Device],
            sync_status: SyncStatus::Idle,
        }
    }

    /// Whether this node is participating in a Network-scope chain.
    pub fn has_network_chain(&self) -> bool {
        self.active_chains.contains(&BlockchainScope::Network)
    }
}

impl fmt::Display for NetworkState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chains: Vec<String> = self.active_chains.iter().map(|c| c.to_string()).collect();
        write!(
            f,
            "Network(peers={}, chains=[{}], sync={})",
            self.connected_peers,
            chains.join(","),
            self.sync_status,
        )
    }
}

/// Synchronization status for network chain participation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncStatus {
    /// Not syncing (Device-only mode or fully caught up).
    Idle,
    /// Actively downloading and applying blocks from peers.
    Downloading,
    /// Caught up and receiving live blocks.
    Live,
    /// Sync stalled (no responsive peers).
    Stalled,
}

impl fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Downloading => write!(f, "Downloading"),
            Self::Live => write!(f, "Live"),
            Self::Stalled => write!(f, "Stalled"),
        }
    }
}

// ---------------------------------------------------------------------------
// RuntimeAssetState — lifecycle of a registered asset
// ---------------------------------------------------------------------------

/// Runtime lifecycle state of an asset on the mesh.
///
/// Distinct from `BaseState` (infrastructure lifecycle) -- this tracks an
/// asset's participation state in the distributed system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeAssetState {
    /// Asset is registered on the local blockchain but not yet active.
    Registered,
    /// Asset is available for allocation and use.
    Available,
    /// Asset is being transferred to another node or scope.
    InTransfer,
    /// Asset is locked (e.g. for a pending cross-scope transfer).
    Locked,
    /// Asset has been revoked and is no longer valid.
    Revoked,
}

impl RuntimeAssetState {
    /// Whether the asset is usable (not locked, in-transfer, or revoked).
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Registered | Self::Available)
    }
}

impl fmt::Display for RuntimeAssetState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registered => write!(f, "Registered"),
            Self::Available => write!(f, "Available"),
            Self::InTransfer => write!(f, "InTransfer"),
            Self::Locked => write!(f, "Locked"),
            Self::Revoked => write!(f, "Revoked"),
        }
    }
}

// ---------------------------------------------------------------------------
// RuntimeSnapshot — combined runtime view
// ---------------------------------------------------------------------------

/// Point-in-time snapshot of a node's runtime state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSnapshot {
    /// Node lifecycle state.
    pub node: NodeState,
    /// Network connectivity and chain sync state.
    pub network: NetworkState,
    /// Total number of registered assets on this node.
    pub asset_count: u64,
    /// Number of assets currently available for use.
    pub available_asset_count: u64,
    /// Snapshot timestamp (UTC milliseconds since epoch).
    pub timestamp_ms: i64,
}

impl RuntimeSnapshot {
    /// Create a snapshot for a freshly booted node with no assets.
    pub fn initial(timestamp_ms: i64) -> Self {
        Self {
            node: NodeState::Booting,
            network: NetworkState::device_only(),
            asset_count: 0,
            available_asset_count: 0,
            timestamp_ms,
        }
    }
}

impl fmt::Display for RuntimeSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Runtime(node={}, {}, assets={}/{})",
            self.node, self.network, self.available_asset_count, self.asset_count,
        )
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_state_operational() {
        assert!(!NodeState::Booting.is_operational());
        assert!(NodeState::Ready.is_operational());
        assert!(NodeState::Syncing.is_operational());
        assert!(NodeState::Degraded.is_operational());
        assert!(!NodeState::Shutdown.is_operational());
    }

    #[test]
    fn node_state_display() {
        assert_eq!(NodeState::Ready.to_string(), "Ready");
        assert_eq!(NodeState::Syncing.to_string(), "Syncing");
    }

    #[test]
    fn network_state_device_only() {
        let ns = NetworkState::device_only();
        assert_eq!(ns.connected_peers, 0);
        assert!(!ns.has_network_chain());
        assert_eq!(ns.sync_status, SyncStatus::Idle);
    }

    #[test]
    fn network_state_with_network_chain() {
        let ns = NetworkState {
            connected_peers: 5,
            active_chains: vec![BlockchainScope::Device, BlockchainScope::Network],
            sync_status: SyncStatus::Live,
        };
        assert!(ns.has_network_chain());
        assert_eq!(ns.connected_peers, 5);
    }

    #[test]
    fn runtime_asset_state_usable() {
        assert!(RuntimeAssetState::Registered.is_usable());
        assert!(RuntimeAssetState::Available.is_usable());
        assert!(!RuntimeAssetState::InTransfer.is_usable());
        assert!(!RuntimeAssetState::Locked.is_usable());
        assert!(!RuntimeAssetState::Revoked.is_usable());
    }

    #[test]
    fn runtime_snapshot_initial() {
        let snap = RuntimeSnapshot::initial(1700000000000);
        assert_eq!(snap.node, NodeState::Booting);
        assert!(!snap.network.has_network_chain());
        assert_eq!(snap.asset_count, 0);
        assert_eq!(snap.timestamp_ms, 1700000000000);
    }

    #[test]
    fn runtime_snapshot_display() {
        let snap = RuntimeSnapshot::initial(0);
        let s = snap.to_string();
        assert!(s.contains("Booting"), "got: {s}");
        assert!(s.contains("assets=0/0"), "got: {s}");
    }

    #[test]
    fn runtime_snapshot_serde_roundtrip() {
        let snap = RuntimeSnapshot {
            node: NodeState::Ready,
            network: NetworkState {
                connected_peers: 3,
                active_chains: vec![BlockchainScope::Device, BlockchainScope::Network],
                sync_status: SyncStatus::Live,
            },
            asset_count: 42,
            available_asset_count: 30,
            timestamp_ms: 1700000000000,
        };
        let json = serde_json::to_string(&snap).expect("test: serialize");
        let back: RuntimeSnapshot = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(snap, back);
    }

    #[test]
    fn sync_status_display() {
        assert_eq!(SyncStatus::Idle.to_string(), "Idle");
        assert_eq!(SyncStatus::Downloading.to_string(), "Downloading");
        assert_eq!(SyncStatus::Live.to_string(), "Live");
        assert_eq!(SyncStatus::Stalled.to_string(), "Stalled");
    }
}
