// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

use super::super::ConflictResolution;
use crate::{AssetMetadata, AssetRegistration};

/// Synchronization strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStrategy {
    /// Full synchronization - sync all packages
    Full,
    /// Incremental - sync only changes since last sync
    Incremental { since: SystemTime },
    /// Selective - sync only specific categories
    Selective { categories: Vec<String> },
    /// Priority - sync based on package priority
    Priority { min_priority: f64 },
    /// Differential - sync based on merkle tree differences
    Differential { merkle_root: String },
}

/// Synchronization state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncState {
    /// Not synchronized
    NotSynced,
    /// Synchronization in progress
    Syncing {
        started_at: SystemTime,
        progress: f64,
    },
    /// Synchronized
    Synced {
        last_sync: SystemTime,
        packages_synced: u32,
    },
    /// Synchronization failed
    Failed {
        last_attempt: SystemTime,
        error: String,
    },
}

/// Synchronization metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    /// Peer node ID
    pub peer_id: String,
    /// Last successful sync
    pub last_sync: Option<SystemTime>,
    /// Sync state
    pub state: SyncState,
    /// Merkle root of package tree
    pub merkle_root: String,
    /// Package versions
    pub package_versions: HashMap<AssetRegistration, String>,
    /// Conflict count
    pub conflicts_resolved: u32,
    /// Bytes transferred
    pub bytes_transferred: u64,
}

/// Package delta for synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDelta {
    /// Packages to add
    pub additions: Vec<crate::AssetPackage>,
    /// Packages to update
    pub updates: Vec<crate::AssetPackage>,
    /// Packages to remove (package names, not BlockMatrix AssetRegistrations)
    pub deletions: Vec<String>,
    /// Conflicting packages
    pub conflicts: Vec<ConflictInfo>,
}

/// Conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Package ID with conflict (catalog package name, not BlockMatrix AssetRegistration)
    pub asset_id: String,
    /// Local version
    pub local_version: String,
    /// Remote version
    pub remote_version: String,
    /// Local metadata
    pub local_metadata: AssetMetadata,
    /// Remote metadata
    pub remote_metadata: AssetMetadata,
    /// Suggested resolution
    pub suggested_resolution: ConflictResolution,
}

/// Merkle tree node for efficient sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// Node hash
    pub hash: String,
    /// Left child hash
    pub left: Option<String>,
    /// Right child hash
    pub right: Option<String>,
    /// Package IDs in this node (for leaf nodes)
    pub packages: Vec<AssetRegistration>,
}

/// Synchronization event for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    /// Timestamp of event
    pub timestamp: SystemTime,
    /// Peer involved
    pub peer_id: String,
    /// Event type
    pub event_type: SyncEventType,
    /// Packages affected
    pub packages_affected: u32,
    /// Data transferred
    pub bytes_transferred: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncEventType {
    Started,
    Completed,
    Failed { error: String },
    ConflictResolved { resolution: ConflictResolution },
}
