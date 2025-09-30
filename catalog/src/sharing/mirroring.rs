//! Package Mirroring and Replication Module
//!
//! Handles automatic mirroring of popular packages, replication strategies,
//! and geographic distribution for optimal access.

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};
use std::cmp::Ordering;

use crate::{AssetId, AssetPackage, AssetMetadata};
use super::topology::NodeLocation;

/// Mirror strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirrorStrategy {
    /// Mirror based on popularity
    Popularity {
        threshold: f64,
        max_mirrors: u32,
    },
    /// Mirror based on geographic distribution
    Geographic {
        regions: Vec<String>,
        mirrors_per_region: u32,
    },
    /// Mirror based on access patterns
    AccessPattern {
        min_accesses: u64,
        time_window: Duration,
    },
    /// Mirror based on package importance
    Priority {
        min_priority: f64,
        replication_factor: u32,
    },
    /// Adaptive mirroring based on network conditions
    Adaptive {
        target_availability: f64,
        max_latency_ms: u64,
    },
}

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Default replication factor
    pub default_factor: u32,
    /// Maximum replication factor
    pub max_factor: u32,
    /// Minimum nodes for quorum
    pub min_quorum: u32,
    /// Geographic distribution requirements
    pub geo_distribution: bool,
    /// Prefer nodes with high uptime
    pub prefer_stable_nodes: bool,
    /// Replication timeout
    pub replication_timeout: Duration,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            default_factor: 3,
            max_factor: 10,
            min_quorum: 2,
            geo_distribution: true,
            prefer_stable_nodes: true,
            replication_timeout: Duration::from_secs(60),
        }
    }
}

/// Mirror node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorNode {
    /// Node ID
    pub node_id: String,
    /// Node location
    pub location: Option<NodeLocation>,
    /// Storage capacity (bytes)
    pub storage_capacity: u64,
    /// Used storage (bytes)
    pub storage_used: u64,
    /// Node uptime percentage
    pub uptime: f64,
    /// Average response time (ms)
    pub avg_response_time: u64,
    /// Packages mirrored
    pub mirrored_packages: HashSet<AssetId>,
    /// Last health check
    pub last_health_check: SystemTime,
}

/// Package popularity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopularityMetrics {
    /// Total downloads
    pub downloads: u64,
    /// Downloads in last 24 hours
    pub downloads_24h: u64,
    /// Downloads in last 7 days
    pub downloads_7d: u64,
    /// Unique users
    pub unique_users: HashSet<String>,
    /// Average rating
    pub avg_rating: f64,
    /// Popularity score (0-1)
    pub score: f64,
    /// Trend (positive/negative)
    pub trend: f64,
}

/// Mirror status for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorStatus {
    /// Package ID
    pub asset_id: AssetId,
    /// Mirror nodes
    pub mirror_nodes: Vec<String>,
    /// Replication factor achieved
    pub replication_factor: u32,
    /// Geographic coverage
    pub geographic_coverage: HashMap<String, u32>,
    /// Last mirroring operation
    pub last_mirrored: SystemTime,
    /// Mirror health score
    pub health_score: f64,
}

/// Priority queue item for mirroring decisions
#[derive(Debug, Clone)]
struct MirrorCandidate {
    asset_id: AssetId,
    priority: f64,
    size: u64,
}

impl Ord for MirrorCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.partial_cmp(&other.priority).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for MirrorCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MirrorCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for MirrorCandidate {}

/// Mirror manager for package replication
pub struct MirrorManager {
    max_storage: u64,
    replication_factor: u32,
    mirror_nodes: Arc<RwLock<HashMap<String, MirrorNode>>>,
    package_mirrors: Arc<RwLock<HashMap<AssetId, MirrorStatus>>>,
    popularity_metrics: Arc<RwLock<HashMap<AssetId, PopularityMetrics>>>,
    mirror_queue: Arc<RwLock<BinaryHeap<MirrorCandidate>>>,
    replication_config: Arc<ReplicationConfig>,
}

impl MirrorManager {
    /// Create new mirror manager
    pub async fn new(max_storage: u64, replication_factor: u32) -> Result<Self> {
        Ok(Self {
            max_storage,
            replication_factor,
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
        asset_id: &AssetId,
        metadata: &AssetMetadata,
    ) -> Result<MirrorStatus> {
        // Check if already mirrored sufficiently
        if let Some(status) = self.get_mirror_status(asset_id).await? {
            if status.replication_factor >= self.replication_factor {
                return Ok(status);
            }
        }

        // Select mirror nodes
        let selected_nodes = self.select_mirror_nodes(
            metadata.size as u64,
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

    /// Mirror popular packages
    async fn mirror_popular_packages(
        &self,
        threshold: f64,
        max_mirrors: u32,
    ) -> Result<u32> {
        let popularity = self.popularity_metrics.read().await;
        let mut candidates = Vec::new();

        // Find packages above threshold
        for (asset_id, metrics) in popularity.iter() {
            if metrics.score >= threshold {
                candidates.push(MirrorCandidate {
                    asset_id: asset_id.clone(),
                    priority: metrics.score,
                    size: 0, // Would get from metadata
                });
            }
        }

        // Sort by priority
        candidates.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());

        // Mirror top packages
        let mut mirrored = 0;
        for candidate in candidates.iter().take(max_mirrors as usize) {
            if self.queue_for_mirroring(&candidate.asset_id, candidate.priority).await? {
                mirrored += 1;
            }
        }

        // Process mirror queue
        self.process_mirror_queue().await?;

        Ok(mirrored)
    }

    /// Mirror based on geographic distribution
    async fn mirror_by_geography(
        &self,
        regions: Vec<String>,
        mirrors_per_region: u32,
    ) -> Result<u32> {
        let nodes = self.mirror_nodes.read().await;
        let mut regional_nodes: HashMap<String, Vec<String>> = HashMap::new();

        // Group nodes by region
        for (node_id, node) in nodes.iter() {
            if let Some(location) = &node.location {
                if regions.contains(&location.region) {
                    regional_nodes.entry(location.region.clone())
                        .or_insert_with(Vec::new)
                        .push(node_id.clone());
                }
            }
        }

        // Select packages to mirror per region
        let mut total_mirrored = 0;
        for (region, node_ids) in regional_nodes {
            let packages_to_mirror = self.select_regional_packages(&region).await?;

            for package_id in packages_to_mirror.iter().take(mirrors_per_region as usize) {
                for node_id in node_ids.iter().take(mirrors_per_region as usize) {
                    if self.replicate_to_specific_node(package_id, node_id).await.is_ok() {
                        total_mirrored += 1;
                    }
                }
            }
        }

        Ok(total_mirrored)
    }

    /// Mirror based on access patterns
    async fn mirror_by_access_pattern(
        &self,
        min_accesses: u64,
        time_window: Duration,
    ) -> Result<u32> {
        let popularity = self.popularity_metrics.read().await;
        let mut candidates = Vec::new();

        let cutoff_time = SystemTime::now() - time_window;

        for (asset_id, metrics) in popularity.iter() {
            // Check recent access count
            if metrics.downloads_24h >= min_accesses {
                candidates.push(asset_id.clone());
            }
        }

        // Mirror frequently accessed packages
        let mut mirrored = 0;
        for asset_id in candidates {
            if self.queue_for_mirroring(&asset_id, 0.8).await? {
                mirrored += 1;
            }
        }

        // Process queue
        self.process_mirror_queue().await?;

        Ok(mirrored)
    }

    /// Mirror based on priority
    async fn mirror_by_priority(
        &self,
        min_priority: f64,
        replication_factor: u32,
    ) -> Result<u32> {
        // Would get package priorities from registry
        let high_priority_packages = self.get_high_priority_packages(min_priority).await?;

        let mut mirrored = 0;
        for asset_id in high_priority_packages {
            // Ensure high replication for priority packages
            let nodes = self.select_mirror_nodes(0, replication_factor).await?;

            for node_id in nodes {
                if self.replicate_to_specific_node(&asset_id, &node_id).await.is_ok() {
                    mirrored += 1;
                }
            }
        }

        Ok(mirrored)
    }

    /// Adaptive mirroring based on network conditions
    async fn adaptive_mirroring(
        &self,
        target_availability: f64,
        max_latency_ms: u64,
    ) -> Result<u32> {
        let mirrors = self.package_mirrors.read().await;
        let nodes = self.mirror_nodes.read().await;

        let mut packages_to_mirror = Vec::new();

        // Find under-replicated packages
        for (asset_id, status) in mirrors.iter() {
            let availability = self.calculate_availability(status, &nodes).await;

            if availability < target_availability {
                // Check latency requirements
                let avg_latency = self.calculate_average_latency(status, &nodes).await;

                if avg_latency > max_latency_ms {
                    packages_to_mirror.push(asset_id.clone());
                }
            }
        }

        // Mirror to improve availability and latency
        let mut mirrored = 0;
        for asset_id in packages_to_mirror {
            // Select nodes to improve metrics
            let optimal_nodes = self.select_optimal_nodes(
                &asset_id,
                target_availability,
                max_latency_ms,
            ).await?;

            for node_id in optimal_nodes {
                if self.replicate_to_specific_node(&asset_id, &node_id).await.is_ok() {
                    mirrored += 1;
                }
            }
        }

        Ok(mirrored)
    }

    /// Select nodes for mirroring
    async fn select_mirror_nodes(
        &self,
        package_size: u64,
        count: u32,
    ) -> Result<Vec<String>> {
        let nodes = self.mirror_nodes.read().await;
        let config = &*self.replication_config;

        // Score and rank nodes
        let mut node_scores: Vec<(String, f64)> = Vec::new();

        for (node_id, node) in nodes.iter() {
            // Check storage capacity
            if node.storage_capacity - node.storage_used < package_size {
                continue;
            }

            let mut score = 0.0;

            // Factor 1: Available storage
            let storage_ratio = (node.storage_capacity - node.storage_used) as f64
                / node.storage_capacity as f64;
            score += storage_ratio * 0.3;

            // Factor 2: Uptime
            if config.prefer_stable_nodes {
                score += node.uptime * 0.3;
            }

            // Factor 3: Response time
            let latency_score = 1.0 / (1.0 + node.avg_response_time as f64 / 1000.0);
            score += latency_score * 0.2;

            // Factor 4: Current load
            let load_score = 1.0 - (node.mirrored_packages.len() as f64 / 1000.0).min(1.0);
            score += load_score * 0.2;

            node_scores.push((node_id.clone(), score));
        }

        // Sort by score
        node_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Select top nodes
        let selected: Vec<String> = node_scores
            .into_iter()
            .take(count as usize)
            .map(|(id, _)| id)
            .collect();

        Ok(selected)
    }

    /// Queue package for mirroring
    async fn queue_for_mirroring(&self, asset_id: &AssetId, priority: f64) -> Result<bool> {
        let mut queue = self.mirror_queue.write().await;

        // Check if already queued
        let already_queued = queue.iter().any(|c| c.asset_id == *asset_id);
        if already_queued {
            return Ok(false);
        }

        queue.push(MirrorCandidate {
            asset_id: asset_id.clone(),
            priority,
            size: 0, // Would get from metadata
        });

        Ok(true)
    }

    /// Process mirror queue
    async fn process_mirror_queue(&self) -> Result<u32> {
        let mut queue = self.mirror_queue.write().await;
        let mut processed = 0;

        while let Some(candidate) = queue.pop() {
            // Check storage capacity
            if self.get_storage_usage().await? + candidate.size > self.max_storage {
                break;
            }

            // Mirror the package
            // Would get full metadata here
            processed += 1;
        }

        Ok(processed)
    }

    /// Replicate package to specific node
    async fn replicate_to_node(
        &self,
        asset_id: &AssetId,
        metadata: &AssetMetadata,
        node_id: &str,
    ) -> Result<()> {
        // Would implement actual replication protocol
        let mut nodes = self.mirror_nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.mirrored_packages.insert(asset_id.clone());
            node.storage_used += metadata.size as u64;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Node not found"))
        }
    }

    async fn replicate_to_specific_node(
        &self,
        asset_id: &AssetId,
        node_id: &str,
    ) -> Result<()> {
        // Would get metadata and call replicate_to_node
        Ok(())
    }

    /// Get mirror status
    pub async fn get_mirror_status(&self, asset_id: &AssetId) -> Result<Option<MirrorStatus>> {
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
        asset_id: &AssetId,
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

        for (node_id, node) in nodes.iter_mut() {
            // Would perform actual health check
            node.last_health_check = now;

            // Update uptime based on response
            // This would be based on actual health check result
            node.uptime = node.uptime * 0.99 + 0.01; // Gradual improvement
        }

        Ok(())
    }

    // Helper methods

    async fn calculate_geo_coverage(&self, node_ids: &[String]) -> HashMap<String, u32> {
        let nodes = self.mirror_nodes.read().await;
        let mut coverage = HashMap::new();

        for node_id in node_ids {
            if let Some(node) = nodes.get(node_id) {
                if let Some(location) = &node.location {
                    *coverage.entry(location.region.clone()).or_insert(0) += 1;
                }
            }
        }

        coverage
    }

    async fn calculate_health_score(&self, node_ids: &[String]) -> f64 {
        let nodes = self.mirror_nodes.read().await;
        let mut total_score = 0.0;
        let mut count = 0;

        for node_id in node_ids {
            if let Some(node) = nodes.get(node_id) {
                let score = node.uptime * 0.5 +
                    (1.0 / (1.0 + node.avg_response_time as f64 / 1000.0)) * 0.5;
                total_score += score;
                count += 1;
            }
        }

        if count > 0 {
            total_score / count as f64
        } else {
            0.0
        }
    }

    async fn calculate_availability(
        &self,
        status: &MirrorStatus,
        nodes: &HashMap<String, MirrorNode>,
    ) -> f64 {
        let online_count = status.mirror_nodes.iter()
            .filter(|id| {
                nodes.get(*id).map(|n| n.uptime > 0.9).unwrap_or(false)
            })
            .count();

        online_count as f64 / status.mirror_nodes.len().max(1) as f64
    }

    async fn calculate_average_latency(
        &self,
        status: &MirrorStatus,
        nodes: &HashMap<String, MirrorNode>,
    ) -> u64 {
        let total_latency: u64 = status.mirror_nodes.iter()
            .filter_map(|id| nodes.get(id).map(|n| n.avg_response_time))
            .sum();

        total_latency / status.mirror_nodes.len().max(1) as u64
    }

    async fn select_optimal_nodes(
        &self,
        _asset_id: &AssetId,
        _target_availability: f64,
        _max_latency_ms: u64,
    ) -> Result<Vec<String>> {
        // Would implement optimal node selection algorithm
        Ok(Vec::new())
    }

    async fn get_high_priority_packages(&self, min_priority: f64) -> Result<Vec<AssetId>> {
        // Would query package registry for priority
        Ok(Vec::new())
    }

    async fn select_regional_packages(&self, _region: &str) -> Result<Vec<AssetId>> {
        // Would select packages popular in region
        Ok(Vec::new())
    }

    fn calculate_popularity_score(&self, metrics: &PopularityMetrics) -> f64 {
        let download_score = (metrics.downloads as f64 / 10000.0).min(1.0);
        let recent_score = (metrics.downloads_24h as f64 / 100.0).min(1.0);
        let user_score = (metrics.unique_users.len() as f64 / 1000.0).min(1.0);
        let rating_score = metrics.avg_rating / 5.0;

        download_score * 0.3 + recent_score * 0.3 + user_score * 0.2 + rating_score * 0.2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mirror_manager_creation() {
        let manager = MirrorManager::new(10 * 1024 * 1024 * 1024, 3).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_node_selection() {
        let manager = MirrorManager::new(10 * 1024 * 1024 * 1024, 3).await.unwrap();
        let nodes = manager.select_mirror_nodes(1024 * 1024, 3).await;
        assert!(nodes.is_ok());
    }
}