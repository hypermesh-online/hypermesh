// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Package Mirroring and Replication Module
//!
//! Migrated to use BlockMatrix instruction-based retrieval

mod types;
mod strategies;

pub use types::*;

use anyhow::Result;
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

use crate::{AssetRegistration, AssetMetadata};
use crate::registry::CatalogRegistry;

/// Mirror manager for package replication
pub struct MirrorManager {
    pub(super) max_storage: u64,
    pub(super) replication_factor: u32,
    pub(super) _registry: Arc<CatalogRegistry>,
    pub(super) mirror_nodes: Arc<RwLock<HashMap<String, MirrorNode>>>,
    pub(super) package_mirrors: Arc<RwLock<HashMap<AssetRegistration, MirrorStatus>>>,
    pub(super) popularity_metrics: Arc<RwLock<HashMap<AssetRegistration, PopularityMetrics>>>,
    pub(super) mirror_queue: Arc<RwLock<BinaryHeap<MirrorCandidate>>>,
    pub(super) replication_config: Arc<ReplicationConfig>,
}

impl MirrorManager {
    /// Create new mirror manager with registry integration
    pub async fn new(
        max_storage: u64,
        replication_factor: u32,
        registry: Arc<CatalogRegistry>,
    ) -> Result<Self> {
        Ok(Self {
            max_storage,
            replication_factor,
            _registry: registry,
            mirror_nodes: Arc::new(RwLock::new(HashMap::new())),
            package_mirrors: Arc::new(RwLock::new(HashMap::new())),
            popularity_metrics: Arc::new(RwLock::new(HashMap::new())),
            mirror_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            replication_config: Arc::new(ReplicationConfig::default()),
        })
    }

    /// Mirror a package
    pub async fn mirror_package(
        &self,
        asset_id: &AssetRegistration,
        metadata: &AssetMetadata,
    ) -> Result<MirrorStatus> {
        // Check if already mirrored sufficiently
        if let Some(status) = self.get_mirror_status(asset_id).await? {
            if status.replication_factor >= self.replication_factor {
                return Ok(status);
            }
        }

        // Select mirror nodes
        let metadata_size = serde_json::to_vec(metadata)
            .map(|v| v.len() as u64)
            .unwrap_or(256);
        let selected_nodes = self.select_mirror_nodes(
            metadata_size,
            self.replication_factor,
        ).await?;

        // Replicate to selected nodes
        let mut successful_mirrors = Vec::new();
        for node_id in &selected_nodes {
            if self.replicate_to_node(asset_id, metadata, node_id).await.is_ok() {
                successful_mirrors.push(node_id.clone());
            }
        }

        // Update mirror status
        let status = MirrorStatus {
            asset_id: asset_id.clone(),
            mirror_nodes: successful_mirrors.clone(),
            replication_factor: successful_mirrors.len() as u32,
            geographic_coverage: self.calculate_geo_coverage(&successful_mirrors).await,
            last_mirrored: SystemTime::now(),
            health_score: self.calculate_health_score(&successful_mirrors).await,
        };

        // Store mirror status
        let mut mirrors = self.package_mirrors.write().await;
        mirrors.insert(asset_id.clone(), status.clone());

        Ok(status)
    }

    /// Apply mirroring strategy
    pub async fn apply_strategy(&self, strategy: MirrorStrategy) -> Result<u32> {
        match strategy {
            MirrorStrategy::Popularity { threshold, max_mirrors } => {
                self.mirror_popular_packages(threshold, max_mirrors).await
            }
            MirrorStrategy::Geographic { regions, mirrors_per_region } => {
                self.mirror_by_geography(regions, mirrors_per_region).await
            }
            MirrorStrategy::AccessPattern { min_accesses, time_window } => {
                self.mirror_by_access_pattern(min_accesses, time_window).await
            }
            MirrorStrategy::Priority { min_priority, replication_factor } => {
                self.mirror_by_priority(min_priority, replication_factor).await
            }
            MirrorStrategy::Adaptive { target_availability, max_latency_ms } => {
                self.adaptive_mirroring(target_availability, max_latency_ms).await
            }
        }
    }

    /// Get mirror status
    pub async fn get_mirror_status(&self, asset_id: &AssetRegistration) -> Result<Option<MirrorStatus>> {
        let mirrors = self.package_mirrors.read().await;
        Ok(mirrors.get(asset_id).cloned())
    }

    /// Get storage usage
    pub async fn get_storage_usage(&self) -> Result<u64> {
        let nodes = self.mirror_nodes.read().await;
        let local_node = nodes.get("local").map(|n| n.storage_used).unwrap_or(0);
        Ok(local_node)
    }

    /// Update popularity metrics
    pub async fn update_popularity(
        &self,
        asset_id: &AssetRegistration,
        download_event: bool,
        user_id: Option<String>,
    ) -> Result<()> {
        let mut metrics = self.popularity_metrics.write().await;
        let entry = metrics.entry(asset_id.clone()).or_insert_with(|| {
            PopularityMetrics {
                downloads: 0,
                downloads_24h: 0,
                downloads_7d: 0,
                unique_users: HashSet::new(),
                avg_rating: 0.0,
                score: 0.0,
                trend: 0.0,
            }
        });

        if download_event {
            entry.downloads += 1;
            entry.downloads_24h += 1;
            entry.downloads_7d += 1;

            if let Some(user) = user_id {
                entry.unique_users.insert(user);
            }

            // Recalculate score
            entry.score = self.calculate_popularity_score(entry);
        }

        Ok(())
    }

    /// Health check for mirror nodes
    pub async fn health_check(&self) -> Result<()> {
        let mut nodes = self.mirror_nodes.write().await;
        let now = SystemTime::now();
        let stale_threshold = Duration::from_secs(300);

        for (_node_id, node) in nodes.iter_mut() {
            let elapsed = now.duration_since(node.last_health_check)
                .unwrap_or(Duration::from_secs(0));
            let is_stale = elapsed > stale_threshold;

            let response_health = 1.0 / (1.0 + node.avg_response_time as f64 / 500.0);
            let storage_health = if node.storage_capacity > 0 {
                1.0 - (node.storage_used as f64 / node.storage_capacity as f64)
            } else {
                0.0
            };

            if is_stale {
                node.uptime = (node.uptime * 0.95).max(0.0);
            } else {
                let health = response_health * 0.6 + storage_health * 0.4;
                node.uptime = node.uptime * 0.9 + health * 0.1;
            }

            node.last_health_check = now;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mirror_manager_creation() {
        use crate::registry::{CatalogRegistry, TrustPolicy, RegistryConfig};

        let registry = Arc::new(CatalogRegistry::new(
            hypermesh_lib::PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        ));

        let manager = MirrorManager::new(10 * 1024 * 1024 * 1024, 3, registry).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_node_selection() {
        use crate::registry::{CatalogRegistry, TrustPolicy, RegistryConfig};

        let registry = Arc::new(CatalogRegistry::new(
            hypermesh_lib::PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        ));

        let manager = MirrorManager::new(10 * 1024 * 1024 * 1024, 3, registry).await.unwrap();
        let nodes = manager.select_mirror_nodes(1024 * 1024, 3).await;
        assert!(nodes.is_ok());
    }
}
