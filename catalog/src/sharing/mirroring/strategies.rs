// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};

use crate::AssetRegistration;
use super::types::*;

impl super::MirrorManager {
    /// Mirror popular packages
    pub(super) async fn mirror_popular_packages(
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
    pub(super) async fn mirror_by_geography(
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
    pub(super) async fn mirror_by_access_pattern(
        &self,
        min_accesses: u64,
        time_window: Duration,
    ) -> Result<u32> {
        let popularity = self.popularity_metrics.read().await;
        let mut candidates = Vec::new();

        let _cutoff_time = SystemTime::now() - time_window;

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
    pub(super) async fn mirror_by_priority(
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
    pub(super) async fn adaptive_mirroring(
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
    pub(super) async fn select_mirror_nodes(
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

    pub(super) async fn queue_for_mirroring(&self, asset_id: &AssetRegistration, priority: f64) -> Result<bool> {
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

    pub(super) async fn process_mirror_queue(&self) -> Result<u32> {
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

    pub(super) async fn replicate_to_node(
        &self,
        asset_id: &AssetRegistration,
        _metadata: &crate::AssetMetadata,
        node_id: &str,
    ) -> Result<()> {
        // Would implement actual replication protocol
        let mut nodes = self.mirror_nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.mirrored_packages.insert(asset_id.clone());
            let estimated_size = serde_json::to_vec(_metadata)
                .map(|v| v.len() as u64)
                .unwrap_or(256);
            node.storage_used += estimated_size;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Node not found"))
        }
    }

    pub(super) async fn replicate_to_specific_node(
        &self,
        asset_id: &AssetRegistration,
        node_id: &str,
    ) -> Result<()> {
        // Build minimal metadata for the replication call.
        let metadata = crate::AssetMetadata {
            name: asset_id.to_hex_string(),
            version: "0.0.0".to_string(),
            tags: vec![],
            description: None,
            author: None,
            license: None,
            homepage: None,
            repository: None,
            download_count: 0,
            featured: false,
            keywords: vec![],
            created: None,
            updated: None,
        };
        self.replicate_to_node(asset_id, &metadata, node_id).await
    }

    pub(super) async fn calculate_geo_coverage(&self, node_ids: &[String]) -> HashMap<String, u32> {
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

    pub(super) async fn calculate_health_score(&self, node_ids: &[String]) -> f64 {
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

    pub(super) async fn calculate_availability(
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

    pub(super) async fn calculate_average_latency(
        &self,
        status: &MirrorStatus,
        nodes: &HashMap<String, MirrorNode>,
    ) -> u64 {
        let total_latency: u64 = status.mirror_nodes.iter()
            .filter_map(|id| nodes.get(id).map(|n| n.avg_response_time))
            .sum();

        total_latency / status.mirror_nodes.len().max(1) as u64
    }

    pub(super) async fn select_optimal_nodes(
        &self,
        asset_id: &AssetRegistration,
        _target_availability: f64,
        max_latency_ms: u64,
    ) -> Result<Vec<String>> {
        let nodes = self.mirror_nodes.read().await;
        let mirrors = self.package_mirrors.read().await;
        let existing: HashSet<String> = mirrors.get(asset_id)
            .map(|s| s.mirror_nodes.iter().cloned().collect())
            .unwrap_or_default();

        let mut scored: Vec<(String, f64)> = Vec::new();
        for (node_id, node) in nodes.iter() {
            if existing.contains(node_id) { continue; }
            if node.avg_response_time > max_latency_ms { continue; }

            let storage_ratio = if node.storage_capacity > 0 {
                (node.storage_capacity - node.storage_used) as f64
                    / node.storage_capacity as f64
            } else { 0.0 };
            let latency_score = 1.0 / (1.0 + node.avg_response_time as f64 / 1000.0);
            let load_score = 1.0 - (node.mirrored_packages.len() as f64 / 1000.0).min(1.0);
            let score = storage_ratio * 0.3 + node.uptime * 0.3
                + latency_score * 0.2 + load_score * 0.2;
            scored.push((node_id.clone(), score));
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top_n = self.replication_factor as usize;
        Ok(scored.into_iter().take(top_n).map(|(id, _)| id).collect())
    }

    pub(super) async fn get_high_priority_packages(&self, min_priority: f64) -> Result<Vec<AssetRegistration>> {
        let popularity = self.popularity_metrics.read().await;
        let mut scored: Vec<(AssetRegistration, f64)> = popularity.iter()
            .map(|(id, metrics)| (id.clone(), self.calculate_popularity_score(metrics)))
            .filter(|(_, score)| *score >= min_priority)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().map(|(id, _)| id).collect())
    }

    pub(super) async fn select_regional_packages(&self, region: &str) -> Result<Vec<AssetRegistration>> {
        let mirrors = self.package_mirrors.read().await;
        let nodes = self.mirror_nodes.read().await;
        let mut regional: Vec<AssetRegistration> = Vec::new();

        for (asset_id, status) in mirrors.iter() {
            let has_regional_seeder = status.mirror_nodes.iter().any(|nid| {
                nodes.get(nid)
                    .and_then(|n| n.location.as_ref())
                    .map(|loc| loc.region == region)
                    .unwrap_or(false)
            });
            if has_regional_seeder {
                regional.push(asset_id.clone());
            }
        }
        Ok(regional)
    }

    pub(super) fn calculate_popularity_score(&self, metrics: &PopularityMetrics) -> f64 {
        let download_score = (metrics.downloads as f64 / 10000.0).min(1.0);
        let recent_score = (metrics.downloads_24h as f64 / 100.0).min(1.0);
        let user_score = (metrics.unique_users.len() as f64 / 1000.0).min(1.0);
        let rating_score = metrics.avg_rating / 5.0;

        download_score * 0.3 + recent_score * 0.3 + user_score * 0.2 + rating_score * 0.2
    }
}
