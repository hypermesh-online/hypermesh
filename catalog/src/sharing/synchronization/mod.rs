// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Library Synchronization Module
//!
//! Migrated to use BlockMatrix instruction-based retrieval and Asset Registry

mod types;
mod helpers;

pub use types::*;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

use crate::{AssetRegistration, AssetPackage};
use crate::registry::CatalogRegistry;
use super::{PeerInfo, ConflictResolution};

/// Synchronization manager
pub struct SyncManager {
    pub(super) _node_id: String,
    pub(super) _sync_interval: Duration,
    pub(super) _registry: Arc<CatalogRegistry>,
    pub(super) peer_states: Arc<RwLock<HashMap<String, SyncMetadata>>>,
    pub(super) merkle_tree: Arc<RwLock<HashMap<String, MerkleNode>>>,
    pub(super) package_index: Arc<RwLock<HashMap<String, AssetPackage>>>,
    pub(super) sync_history: Arc<RwLock<Vec<SyncEvent>>>,
}

impl SyncManager {
    /// Create new sync manager with registry integration
    pub async fn new(
        node_id: String,
        sync_interval: Duration,
        registry: Arc<CatalogRegistry>,
    ) -> Result<Self> {
        Ok(Self {
            _node_id: node_id,
            _sync_interval: sync_interval,
            _registry: registry,
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
            if peer.available_packages.contains(package.id()) {
                // Check if peer has older version
                if self.needs_update(&package.id(), &peer.node_id).await? {
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
            if !peer.available_packages.contains(package.id()) {
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
            if !peer.available_packages.contains(package.id()) {
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
                    let package = self.get_package(&package_id.to_hex_string()).await?;
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
            if let Some(local_package) = our_packages.get(id.as_str()) {
                if local_package.spec.metadata.version != remote_meta.version {
                    // Version conflict
                    delta.conflicts.push(ConflictInfo {
                        asset_id: id.clone(),
                        local_version: local_package.spec.metadata.version.clone(),
                        remote_version: remote_meta.version.clone(),
                        local_metadata: local_package.spec.metadata.clone(),
                        remote_metadata: remote_meta.clone(),
                        suggested_resolution: self.suggest_resolution(
                            &local_package.spec.metadata,
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
                    // Compare timestamps using updated_at field
                    if conflict.remote_metadata.updated > conflict.local_metadata.updated {
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
            package_index.insert(package.package_hash.clone(), package);
            packages_synced += 1;
        }

        // Apply updates
        for package in delta.updates {
            package_index.insert(package.package_hash.clone(), package);
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
    pub(super) async fn rebuild_merkle_tree(&self) -> Result<()> {
        let packages = self.package_index.read().await;
        let mut tree = HashMap::new();

        // Create leaf nodes
        let mut leaves = Vec::new();
        for (id, package) in packages.iter() {
            let hash = self.hash_package(package);
            // Parse string ID to AssetRegistration
            let asset_id = AssetRegistration::from_hex_string(id)
                .unwrap_or_else(|_| {
                    // Fallback: create from package hash
                    let mut hash_bytes = [0u8; 32];
                    if let Ok(bytes) = hex::decode(&package.package_hash) {
                        hash_bytes[..bytes.len().min(32)].copy_from_slice(&bytes[..bytes.len().min(32)]);
                    }
                    AssetRegistration::new_from_hash(&hash_bytes)
                });
            let node = MerkleNode {
                hash: hash.clone(),
                left: None,
                right: None,
                packages: vec![asset_id],
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_manager_creation() {
        use crate::registry::{CatalogRegistry, TrustPolicy, RegistryConfig};

        let registry = Arc::new(CatalogRegistry::new(
            hypermesh_lib::PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        ));

        let manager = SyncManager::new(
            "test-node".to_string(),
            Duration::from_secs(300),
            registry,
        ).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_merkle_tree_building() {
        use crate::registry::{CatalogRegistry, TrustPolicy, RegistryConfig};

        let registry = Arc::new(CatalogRegistry::new(
            hypermesh_lib::PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        ));

        let manager = SyncManager::new(
            "test-node".to_string(),
            Duration::from_secs(300),
            registry,
        ).await.unwrap();

        let result = manager.rebuild_merkle_tree().await;
        assert!(result.is_ok());
    }
}
