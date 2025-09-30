//! Library Synchronization Module
//!
//! Handles cross-node library synchronization with conflict resolution,
//! incremental updates, and selective synchronization.

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};
use sha2::{Sha256, Digest};

use crate::{AssetId, AssetPackage, AssetMetadata};
use super::{PeerInfo, ConflictResolution};

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
    pub package_versions: HashMap<AssetId, String>,
    /// Conflict count
    pub conflicts_resolved: u32,
    /// Bytes transferred
    pub bytes_transferred: u64,
}

/// Package delta for synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDelta {
    /// Packages to add
    pub additions: Vec<AssetPackage>,
    /// Packages to update
    pub updates: Vec<AssetPackage>,
    /// Packages to remove
    pub deletions: Vec<AssetId>,
    /// Conflicting packages
    pub conflicts: Vec<ConflictInfo>,
}

/// Conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Asset ID with conflict
    pub asset_id: AssetId,
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
    pub packages: Vec<AssetId>,
}

/// Synchronization manager
pub struct SyncManager {
    node_id: String,
    sync_interval: Duration,
    peer_states: Arc<RwLock<HashMap<String, SyncMetadata>>>,
    merkle_tree: Arc<RwLock<HashMap<String, MerkleNode>>>,
    package_index: Arc<RwLock<HashMap<AssetId, AssetPackage>>>,
    sync_history: Arc<RwLock<Vec<SyncEvent>>>,
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

impl SyncManager {
    /// Create new sync manager
    pub async fn new(node_id: String, sync_interval: Duration) -> Result<Self> {
        Ok(Self {
            node_id,
            sync_interval,
            peer_states: Arc::new(RwLock::new(HashMap::new())),
            merkle_tree: Arc::new(RwLock::new(HashMap::new())),
            package_index: Arc::new(RwLock::new(HashMap::new())),
            sync_history: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Synchronize with a peer
    pub async fn sync_with_peer(
        &self,
        peer: &PeerInfo,
        resolution: ConflictResolution,
    ) -> Result<u32> {
        // Update sync state
        self.update_sync_state(&peer.node_id, SyncState::Syncing {
            started_at: SystemTime::now(),
            progress: 0.0,
        }).await?;

        // Record sync start
        self.record_event(SyncEvent {
            timestamp: SystemTime::now(),
            peer_id: peer.node_id.clone(),
            event_type: SyncEventType::Started,
            packages_affected: 0,
            bytes_transferred: 0,
        }).await;

        // Get peer's merkle root
        let peer_merkle = self.request_merkle_root(&peer.node_id).await?;

        // Compare merkle trees to find differences
        let delta = self.compute_delta(&peer_merkle).await?;

        // Resolve conflicts
        let resolved_delta = self.resolve_conflicts(delta, resolution.clone()).await?;

        // Apply changes
        let packages_synced = self.apply_delta(resolved_delta, &peer.node_id).await?;

        // Update sync state
        self.update_sync_state(&peer.node_id, SyncState::Synced {
            last_sync: SystemTime::now(),
            packages_synced,
        }).await?;

        // Record sync completion
        self.record_event(SyncEvent {
            timestamp: SystemTime::now(),
            peer_id: peer.node_id.clone(),
            event_type: SyncEventType::Completed,
            packages_affected: packages_synced,
            bytes_transferred: 0, // Would be tracked during actual transfer
        }).await;

        Ok(packages_synced)
    }

    /// Perform selective synchronization
    pub async fn selective_sync(
        &self,
        peer: &PeerInfo,
        strategy: SyncStrategy,
    ) -> Result<u32> {
        match strategy {
            SyncStrategy::Full => {
                self.sync_with_peer(peer, ConflictResolution::ConsensusWins).await
            }
            SyncStrategy::Incremental { since } => {
                self.incremental_sync(peer, since).await
            }
            SyncStrategy::Selective { categories } => {
                self.category_sync(peer, categories).await
            }
            SyncStrategy::Priority { min_priority } => {
                self.priority_sync(peer, min_priority).await
            }
            SyncStrategy::Differential { merkle_root } => {
                self.differential_sync(peer, merkle_root).await
            }
        }
    }

    /// Incremental synchronization since timestamp
    async fn incremental_sync(
        &self,
        peer: &PeerInfo,
        since: SystemTime,
    ) -> Result<u32> {
        let packages = self.get_packages_since(since).await?;
        let mut synced_count = 0;

        for package in packages {
            if peer.available_packages.contains(&package.metadata.id) {
                // Check if peer has older version
                if self.needs_update(&package.metadata.id, &peer.node_id).await? {
                    self.send_package_update(&package, &peer.node_id).await?;
                    synced_count += 1;
                }
            } else {
                // Peer doesn't have this package
                self.send_package(&package, &peer.node_id).await?;
                synced_count += 1;
            }
        }

        Ok(synced_count)
    }

    /// Category-based synchronization
    async fn category_sync(
        &self,
        peer: &PeerInfo,
        categories: Vec<String>,
    ) -> Result<u32> {
        let packages = self.get_packages_by_category(categories).await?;
        let mut synced_count = 0;

        for package in packages {
            if !peer.available_packages.contains(&package.metadata.id) {
                self.send_package(&package, &peer.node_id).await?;
                synced_count += 1;
            }
        }

        Ok(synced_count)
    }

    /// Priority-based synchronization
    async fn priority_sync(
        &self,
        peer: &PeerInfo,
        min_priority: f64,
    ) -> Result<u32> {
        let packages = self.get_high_priority_packages(min_priority).await?;
        let mut synced_count = 0;

        for package in packages {
            if !peer.available_packages.contains(&package.metadata.id) {
                self.send_package(&package, &peer.node_id).await?;
                synced_count += 1;
            }
        }

        Ok(synced_count)
    }

    /// Differential synchronization using merkle trees
    async fn differential_sync(
        &self,
        peer: &PeerInfo,
        peer_merkle_root: String,
    ) -> Result<u32> {
        let our_merkle = self.merkle_tree.read().await;
        let our_root = self.calculate_merkle_root(&*our_merkle);

        if our_root == peer_merkle_root {
            // Already in sync
            return Ok(0);
        }

        // Find differing branches
        let diff_nodes = self.find_merkle_differences(&peer_merkle_root).await?;
        let mut synced_count = 0;

        for node_hash in diff_nodes {
            if let Some(node) = our_merkle.get(&node_hash) {
                for package_id in &node.packages {
                    let package = self.get_package(package_id).await?;
                    self.send_package(&package, &peer.node_id).await?;
                    synced_count += 1;
                }
            }
        }

        Ok(synced_count)
    }

    /// Compute delta between local and remote state
    async fn compute_delta(&self, peer_merkle: &str) -> Result<PackageDelta> {
        let our_packages = self.package_index.read().await;
        let peer_packages = self.request_package_list(peer_merkle).await?;

        let mut delta = PackageDelta {
            additions: Vec::new(),
            updates: Vec::new(),
            deletions: Vec::new(),
            conflicts: Vec::new(),
        };

        // Find additions and updates
        for (id, remote_meta) in peer_packages.iter() {
            if let Some(local_package) = our_packages.get(id) {
                if local_package.metadata.version != remote_meta.version {
                    // Version conflict
                    delta.conflicts.push(ConflictInfo {
                        asset_id: id.clone(),
                        local_version: local_package.metadata.version.clone(),
                        remote_version: remote_meta.version.clone(),
                        local_metadata: local_package.metadata.clone(),
                        remote_metadata: remote_meta.clone(),
                        suggested_resolution: self.suggest_resolution(
                            &local_package.metadata,
                            remote_meta,
                        ),
                    });
                }
            } else {
                // New package from peer
                if let Ok(package) = self.request_package(id, peer_merkle).await {
                    delta.additions.push(package);
                }
            }
        }

        // Find deletions
        for (id, _) in our_packages.iter() {
            if !peer_packages.contains_key(id) {
                delta.deletions.push(id.clone());
            }
        }

        Ok(delta)
    }

    /// Resolve conflicts in delta
    async fn resolve_conflicts(
        &self,
        mut delta: PackageDelta,
        resolution: ConflictResolution,
    ) -> Result<PackageDelta> {
        let mut resolved_conflicts = Vec::new();

        for conflict in delta.conflicts.drain(..) {
            match resolution {
                ConflictResolution::NewestWins => {
                    // Compare timestamps
                    if conflict.remote_metadata.timestamp > conflict.local_metadata.timestamp {
                        // Use remote version
                        if let Ok(package) = self.request_package(
                            &conflict.asset_id,
                            &conflict.remote_version,
                        ).await {
                            delta.updates.push(package);
                        }
                    }
                    // Otherwise keep local version
                }
                ConflictResolution::ConsensusWins => {
                    // Check consensus scores
                    if self.get_consensus_score(&conflict.remote_metadata).await? >
                       self.get_consensus_score(&conflict.local_metadata).await? {
                        // Use remote version
                        if let Ok(package) = self.request_package(
                            &conflict.asset_id,
                            &conflict.remote_version,
                        ).await {
                            delta.updates.push(package);
                        }
                    }
                }
                ConflictResolution::Merge => {
                    // Attempt to merge changes
                    if let Ok(merged) = self.merge_packages(
                        &conflict.local_metadata,
                        &conflict.remote_metadata,
                    ).await {
                        delta.updates.push(merged);
                    } else {
                        // Merge failed, keep as conflict
                        resolved_conflicts.push(conflict);
                    }
                }
                ConflictResolution::KeepBoth => {
                    // Create versioned copies of both
                    // This would create package variants
                    resolved_conflicts.push(conflict);
                }
                ConflictResolution::Manual => {
                    // Keep as unresolved conflict
                    resolved_conflicts.push(conflict);
                }
            }

            // Record conflict resolution
            self.record_event(SyncEvent {
                timestamp: SystemTime::now(),
                peer_id: String::new(), // Would be set from context
                event_type: SyncEventType::ConflictResolved {
                    resolution: resolution.clone(),
                },
                packages_affected: 1,
                bytes_transferred: 0,
            }).await;
        }

        delta.conflicts = resolved_conflicts;
        Ok(delta)
    }

    /// Apply synchronization delta
    async fn apply_delta(&self, delta: PackageDelta, peer_id: &str) -> Result<u32> {
        let mut packages_synced = 0;
        let mut package_index = self.package_index.write().await;

        // Apply additions
        for package in delta.additions {
            package_index.insert(package.metadata.id.clone(), package);
            packages_synced += 1;
        }

        // Apply updates
        for package in delta.updates {
            package_index.insert(package.metadata.id.clone(), package);
            packages_synced += 1;
        }

        // Apply deletions (with caution)
        for id in delta.deletions {
            if self.can_safely_delete(&id).await? {
                package_index.remove(&id);
                packages_synced += 1;
            }
        }

        // Update merkle tree
        self.rebuild_merkle_tree().await?;

        // Update peer state
        let mut peer_states = self.peer_states.write().await;
        if let Some(state) = peer_states.get_mut(peer_id) {
            state.last_sync = Some(SystemTime::now());
            state.merkle_root = self.calculate_current_merkle_root().await?;
        }

        Ok(packages_synced)
    }

    /// Build/rebuild merkle tree from packages
    async fn rebuild_merkle_tree(&self) -> Result<()> {
        let packages = self.package_index.read().await;
        let mut tree = HashMap::new();

        // Create leaf nodes
        let mut leaves = Vec::new();
        for (id, package) in packages.iter() {
            let hash = self.hash_package(package);
            let node = MerkleNode {
                hash: hash.clone(),
                left: None,
                right: None,
                packages: vec![id.clone()],
            };
            tree.insert(hash.clone(), node);
            leaves.push(hash);
        }

        // Build tree bottom-up
        let mut current_level = leaves;
        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                let left = &chunk[0];
                let right = chunk.get(1).unwrap_or(left);

                let parent_hash = self.hash_pair(left, right);
                let parent_node = MerkleNode {
                    hash: parent_hash.clone(),
                    left: Some(left.clone()),
                    right: Some(right.clone()),
                    packages: Vec::new(),
                };

                tree.insert(parent_hash.clone(), parent_node);
                next_level.push(parent_hash);
            }

            current_level = next_level;
        }

        *self.merkle_tree.write().await = tree;
        Ok(())
    }

    // Helper methods

    async fn update_sync_state(&self, peer_id: &str, state: SyncState) -> Result<()> {
        let mut peer_states = self.peer_states.write().await;
        let metadata = peer_states.entry(peer_id.to_string()).or_insert_with(|| {
            SyncMetadata {
                peer_id: peer_id.to_string(),
                last_sync: None,
                state: SyncState::NotSynced,
                merkle_root: String::new(),
                package_versions: HashMap::new(),
                conflicts_resolved: 0,
                bytes_transferred: 0,
            }
        });
        metadata.state = state;
        Ok(())
    }

    async fn record_event(&self, event: SyncEvent) {
        let mut history = self.sync_history.write().await;
        history.push(event);

        // Keep only last 1000 events
        if history.len() > 1000 {
            history.drain(0..history.len() - 1000);
        }
    }

    fn suggest_resolution(
        &self,
        local: &AssetMetadata,
        remote: &AssetMetadata,
    ) -> ConflictResolution {
        // Simple heuristic for suggesting resolution
        if remote.timestamp > local.timestamp {
            ConflictResolution::NewestWins
        } else {
            ConflictResolution::ConsensusWins
        }
    }

    fn hash_package(&self, package: &AssetPackage) -> String {
        let mut hasher = Sha256::new();
        hasher.update(package.metadata.id.to_string().as_bytes());
        hasher.update(package.metadata.version.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn hash_pair(&self, left: &str, right: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(left.as_bytes());
        hasher.update(right.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn calculate_merkle_root(&self, tree: &HashMap<String, MerkleNode>) -> String {
        // Find root node (node with no parent)
        for node in tree.values() {
            let is_root = !tree.values().any(|n| {
                n.left.as_ref() == Some(&node.hash) || n.right.as_ref() == Some(&node.hash)
            });
            if is_root && node.left.is_some() {
                return node.hash.clone();
            }
        }
        String::new()
    }

    async fn calculate_current_merkle_root(&self) -> Result<String> {
        let tree = self.merkle_tree.read().await;
        Ok(self.calculate_merkle_root(&*tree))
    }

    // Stub methods for actual network operations

    async fn request_merkle_root(&self, _peer_id: &str) -> Result<String> {
        // Would make actual network request
        Ok(String::new())
    }

    async fn request_package_list(&self, _peer_merkle: &str) -> Result<HashMap<AssetId, AssetMetadata>> {
        // Would make actual network request
        Ok(HashMap::new())
    }

    async fn request_package(&self, _id: &AssetId, _version: &str) -> Result<AssetPackage> {
        // Would make actual network request
        Err(anyhow::anyhow!("Not implemented"))
    }

    async fn get_packages_since(&self, since: SystemTime) -> Result<Vec<AssetPackage>> {
        let packages = self.package_index.read().await;
        Ok(packages.values()
            .filter(|p| p.metadata.timestamp > since)
            .cloned()
            .collect())
    }

    async fn get_packages_by_category(&self, categories: Vec<String>) -> Result<Vec<AssetPackage>> {
        let packages = self.package_index.read().await;
        Ok(packages.values()
            .filter(|p| categories.contains(&p.metadata.category))
            .cloned()
            .collect())
    }

    async fn get_high_priority_packages(&self, min_priority: f64) -> Result<Vec<AssetPackage>> {
        let packages = self.package_index.read().await;
        Ok(packages.values()
            .filter(|p| p.metadata.priority >= min_priority)
            .cloned()
            .collect())
    }

    async fn get_package(&self, id: &AssetId) -> Result<AssetPackage> {
        let packages = self.package_index.read().await;
        packages.get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Package not found"))
    }

    async fn needs_update(&self, _id: &AssetId, _peer_id: &str) -> Result<bool> {
        // Would check version comparison
        Ok(false)
    }

    async fn send_package(&self, _package: &AssetPackage, _peer_id: &str) -> Result<()> {
        // Would send over network
        Ok(())
    }

    async fn send_package_update(&self, _package: &AssetPackage, _peer_id: &str) -> Result<()> {
        // Would send update over network
        Ok(())
    }

    async fn find_merkle_differences(&self, _peer_root: &str) -> Result<Vec<String>> {
        // Would compare merkle trees
        Ok(Vec::new())
    }

    async fn get_consensus_score(&self, _metadata: &AssetMetadata) -> Result<f64> {
        // Would calculate consensus score
        Ok(0.5)
    }

    async fn merge_packages(
        &self,
        _local: &AssetMetadata,
        _remote: &AssetMetadata,
    ) -> Result<AssetPackage> {
        // Would attempt to merge package versions
        Err(anyhow::anyhow!("Merge not supported"))
    }

    async fn can_safely_delete(&self, _id: &AssetId) -> Result<bool> {
        // Would check if package can be safely deleted
        Ok(false) // Conservative default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_manager_creation() {
        let manager = SyncManager::new(
            "test-node".to_string(),
            Duration::from_secs(300),
        ).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_merkle_tree_building() {
        let manager = SyncManager::new(
            "test-node".to_string(),
            Duration::from_secs(300),
        ).await.unwrap();

        let result = manager.rebuild_merkle_tree().await;
        assert!(result.is_ok());
    }
}