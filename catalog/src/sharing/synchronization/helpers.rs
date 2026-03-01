// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::Result;
use std::collections::HashMap;
use std::time::SystemTime;
// BLAKE3 used for sync hashing
use chrono::{DateTime, Utc};

use super::super::ConflictResolution;
use super::types::*;
use crate::{PackageSpecMetadata, AssetPackage};

impl super::SyncManager {
    pub(super) async fn update_sync_state(&self, peer_id: &str, state: SyncState) -> Result<()> {
        let mut peer_states = self.peer_states.write().await;
        let metadata = peer_states
            .entry(peer_id.to_string())
            .or_insert_with(|| SyncMetadata {
                peer_id: peer_id.to_string(),
                last_sync: None,
                state: SyncState::NotSynced,
                merkle_root: String::new(),
                package_versions: HashMap::new(),
                conflicts_resolved: 0,
                bytes_transferred: 0,
            });
        metadata.state = state;
        Ok(())
    }

    pub(super) async fn record_event(&self, event: SyncEvent) {
        let mut history = self.sync_history.write().await;
        history.push(event);

        // Keep only last 1000 events
        if history.len() > 1000 {
            let to_remove = history.len() - 1000;
            history.drain(0..to_remove);
        }
    }

    pub(super) fn suggest_resolution(
        &self,
        local: &PackageSpecMetadata,
        remote: &PackageSpecMetadata,
    ) -> ConflictResolution {
        // Simple heuristic for suggesting resolution
        // Compare updated timestamps if they exist
        match (local.updated, remote.updated) {
            (Some(local_time), Some(remote_time)) => {
                if remote_time > local_time {
                    ConflictResolution::NewestWins
                } else {
                    ConflictResolution::ConsensusWins
                }
            }
            _ => ConflictResolution::ConsensusWins,
        }
    }

    pub(super) fn hash_package(&self, package: &AssetPackage) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(package.id().to_string().as_bytes());
        hasher.update(package.metadata().version.as_bytes());
        hasher.finalize().to_hex().to_string()
    }

    pub(super) fn hash_pair(&self, left: &str, right: &str) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(left.as_bytes());
        hasher.update(right.as_bytes());
        hasher.finalize().to_hex().to_string()
    }

    pub(super) fn calculate_merkle_root(&self, tree: &HashMap<String, MerkleNode>) -> String {
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

    pub(super) async fn calculate_current_merkle_root(&self) -> Result<String> {
        let tree = self.merkle_tree.read().await;
        Ok(self.calculate_merkle_root(&tree))
    }

    pub(super) async fn request_merkle_root(&self, _peer_id: &str) -> Result<String> {
        // Local-first: compute and return our own merkle root.
        // For P2P sync the remote peer would do the same on their side.
        let tree = self.merkle_tree.read().await;
        if tree.is_empty() {
            drop(tree);
            self.rebuild_merkle_tree().await?;
        }
        self.calculate_current_merkle_root().await
    }

    pub(super) async fn request_package_list(
        &self,
        _peer_merkle: &str,
    ) -> Result<HashMap<String, PackageSpecMetadata>> {
        // Local-first: return local package index as id->metadata map.
        // A real P2P call would fetch the remote peer's list over STOQ.
        let packages = self.package_index.read().await;
        let mut result = HashMap::new();
        for (id, package) in packages.iter() {
            result.insert(id.clone(), package.spec.metadata.clone());
        }
        Ok(result)
    }

    pub(super) async fn request_package(&self, id: &str, _version: &str) -> Result<AssetPackage> {
        // Local-first: look up in local index. If not found, the caller
        // would need a P2P fetch over STOQ (network-dependent).
        let packages = self.package_index.read().await;
        packages.get(id).cloned().ok_or_else(|| {
            anyhow::anyhow!("Package '{id}' not found locally; requires P2P fetch over STOQ")
        })
    }

    pub(super) async fn get_packages_since(&self, since: SystemTime) -> Result<Vec<AssetPackage>> {
        let packages = self.package_index.read().await;
        // Filter by updated field from PackageSpecMetadata (if it exists)
        // Convert SystemTime to DateTime<Utc> for comparison
        let since_dt = DateTime::<Utc>::from(since);
        Ok(packages
            .values()
            .filter(|p| p.metadata().updated.map(|u| u > since_dt).unwrap_or(false))
            .cloned()
            .collect())
    }

    pub(super) async fn get_packages_by_category(
        &self,
        categories: Vec<String>,
    ) -> Result<Vec<AssetPackage>> {
        let packages = self.package_index.read().await;
        // Filter by tags since PackageSpecMetadata doesn't have category field
        Ok(packages
            .values()
            .filter(|p| p.metadata().tags.iter().any(|tag| categories.contains(tag)))
            .cloned()
            .collect())
    }

    pub(super) async fn get_high_priority_packages(
        &self,
        min_priority: f64,
    ) -> Result<Vec<AssetPackage>> {
        let packages = self.package_index.read().await;
        let mut scored: Vec<(f64, AssetPackage)> = packages
            .values()
            .map(|pkg| {
                let meta = &pkg.spec.metadata;
                // Score from metadata completeness: author, license, description, tags
                let mut score = 0.0;
                if meta.author.is_some() {
                    score += 0.2;
                }
                if meta.license.is_some() {
                    score += 0.2;
                }
                if meta.description.is_some() {
                    score += 0.2;
                }
                if !meta.tags.is_empty() {
                    score += 0.2;
                }
                if !meta.keywords.is_empty() {
                    score += 0.2;
                }
                (score, pkg.clone())
            })
            .filter(|(score, _)| *score >= min_priority)
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().map(|(_, pkg)| pkg).collect())
    }

    pub(super) async fn get_package(&self, id: &str) -> Result<AssetPackage> {
        let packages = self.package_index.read().await;
        packages
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Package not found"))
    }

    pub(super) async fn needs_update(&self, id: &str, peer_id: &str) -> Result<bool> {
        // Compare local version against peer's tracked version.
        let packages = self.package_index.read().await;
        let local_version = match packages.get(id) {
            Some(pkg) => pkg.spec.metadata.version.clone(),
            None => return Ok(false),
        };
        let peer_states = self.peer_states.read().await;
        let peer_version = peer_states
            .get(peer_id)
            .and_then(|state| {
                state
                    .package_versions
                    .iter()
                    .find(|(reg, _)| reg.to_hex_string() == id)
                    .map(|(_, v)| v.clone())
            })
            .unwrap_or_else(|| "0.0.0".to_string());
        // Update needed if local is newer than peer's copy
        Ok(local_version != peer_version && local_version > peer_version)
    }

    pub(super) async fn send_package(&self, package: &AssetPackage, _peer_id: &str) -> Result<()> {
        // Local-first: store in local index. Network send deferred.
        let mut index = self.package_index.write().await;
        index.insert(package.package_hash.clone(), package.clone());
        tracing::debug!(
            package_id = %package.id(),
            "Package stored locally; network send deferred (requires STOQ)"
        );
        Ok(())
    }

    pub(super) async fn send_package_update(
        &self,
        package: &AssetPackage,
        _peer_id: &str,
    ) -> Result<()> {
        // Local-first: update in local index. Network send deferred.
        let mut index = self.package_index.write().await;
        index.insert(package.package_hash.clone(), package.clone());
        tracing::debug!(
            package_id = %package.id(),
            "Package update stored locally; network send deferred (requires STOQ)"
        );
        Ok(())
    }

    pub(super) async fn find_merkle_differences(&self, peer_root: &str) -> Result<Vec<String>> {
        // Compare our merkle root against the peer's. If they differ, return
        // all local leaf node hashes as potential differences.
        let our_root = self.calculate_current_merkle_root().await?;
        if our_root == peer_root {
            return Ok(Vec::new());
        }
        let tree = self.merkle_tree.read().await;
        let diff_hashes: Vec<String> = tree
            .values()
            .filter(|node| !node.packages.is_empty())
            .map(|node| node.hash.clone())
            .collect();
        Ok(diff_hashes)
    }

    pub(super) async fn get_consensus_score(&self, metadata: &PackageSpecMetadata) -> Result<f64> {
        // Score based on metadata completeness and trust signals.
        let mut score = 0.0;
        let mut factors = 0;
        if metadata.author.is_some() {
            score += 1.0;
        }
        factors += 1;
        if metadata.license.is_some() {
            score += 1.0;
        }
        factors += 1;
        if metadata.description.is_some() {
            score += 1.0;
        }
        factors += 1;
        if !metadata.tags.is_empty() {
            score += 1.0;
        }
        factors += 1;
        if metadata.homepage.is_some() {
            score += 1.0;
        }
        factors += 1;
        if metadata.repository.is_some() {
            score += 1.0;
        }
        factors += 1;
        // Version maturity: higher major versions imply more trust
        let version_parts: Vec<&str> = metadata.version.split('.').collect();
        if let Some(major) = version_parts.first().and_then(|v| v.parse::<u32>().ok()) {
            if major >= 1 {
                score += 1.0;
            }
        }
        factors += 1;
        Ok(score / factors as f64)
    }

    pub(super) async fn merge_packages(
        &self,
        local: &PackageSpecMetadata,
        remote: &PackageSpecMetadata,
    ) -> Result<AssetPackage> {
        // Last-writer-wins: take the package with the newer timestamp.
        let winner_name = match (local.updated, remote.updated) {
            (Some(local_ts), Some(remote_ts)) if remote_ts > local_ts => &remote.name,
            (None, Some(_)) => &remote.name,
            _ => &local.name,
        };
        let packages = self.package_index.read().await;
        for pkg in packages.values() {
            if pkg.spec.metadata.name == *winner_name {
                return Ok(pkg.clone());
            }
        }
        Err(anyhow::anyhow!(
            "Merge winner '{winner_name}' not found in local index"
        ))
    }

    pub(super) async fn can_safely_delete(&self, id: &str) -> Result<bool> {
        // Check if any other packages depend on this one.
        let packages = self.package_index.read().await;
        let target_name = packages.get(id).map(|p| p.spec.metadata.name.clone());
        let target_name = match target_name {
            Some(name) => name,
            None => return Ok(true), // Already absent
        };
        for (pkg_id, pkg) in packages.iter() {
            if pkg_id == id {
                continue;
            }
            for dep in &pkg.spec.spec.dependencies {
                if dep.name == target_name {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}
