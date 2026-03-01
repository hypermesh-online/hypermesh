// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};

use super::super::{PeerInfo, SharePermission};
use super::types::*;
use crate::{AssetMetadata, AssetRegistration};

impl super::DiscoveryService {
    /// Search across network
    pub async fn search_network(
        &self,
        query: &str,
        peers: &HashMap<String, PeerInfo>,
    ) -> Result<Vec<(AssetRegistration, AssetMetadata)>> {
        // Check federated cache first
        if let Some(cached_results) = self.search_cache(query).await? {
            return Ok(cached_results);
        }

        // Search across peers
        let mut all_results = Vec::new();
        let mut search_tasks = Vec::new();

        for (peer_id, peer_info) in peers.iter() {
            let query = query.to_string();
            let peer_id = peer_id.clone();
            let peer_address = peer_info.address.clone();

            // Spawn parallel search tasks
            let task =
                tokio::spawn(
                    async move { Self::search_peer(&peer_id, &peer_address, &query).await },
                );
            search_tasks.push(task);
        }

        // Collect results
        for task in search_tasks {
            if let Ok(Ok(results)) = task.await {
                all_results.extend(results);
            }
        }

        // Deduplicate and rank
        all_results = self.deduplicate_results(all_results);
        all_results = self.rank_results(all_results, query);

        // Cache results
        self.cache_search_results(query, &all_results).await?;

        // Update stats
        let mut stats = self.index_stats.write().await;
        stats.total_searches += 1;
        stats.cache_misses += 1;

        Ok(all_results)
    }

    /// Perform full-text search
    pub async fn full_text_search(&self, query: &str) -> Result<Vec<SearchResult>> {
        if !self.search_capabilities.full_text {
            return Ok(Vec::new());
        }

        let index = self.local_index.read().await;
        let mut results = Vec::new();

        for (_asset_id, entry) in index.iter() {
            let (matches, highlights) = self.full_text_match(entry, query);
            if matches {
                results.push(SearchResult {
                    index: entry.clone(),
                    relevance: self.calculate_relevance(&entry.metadata, query),
                    highlights,
                    sources: entry.available_nodes.iter().cloned().collect(),
                });
            }
        }

        Ok(results)
    }

    /// Fuzzy search for approximate matches
    pub async fn fuzzy_search(
        &self,
        query: &str,
        max_distance: usize,
    ) -> Result<Vec<SearchResult>> {
        if !self.search_capabilities.fuzzy {
            return Ok(Vec::new());
        }

        let index = self.local_index.read().await;
        let mut results = Vec::new();

        for (_asset_id, entry) in index.iter() {
            if let Some(distance) = self.fuzzy_distance(&entry.metadata.name, query) {
                if distance <= max_distance {
                    let relevance = 1.0 - (distance as f64 / max_distance as f64);
                    results.push(SearchResult {
                        index: entry.clone(),
                        relevance,
                        highlights: vec![entry.metadata.name.clone()],
                        sources: entry.available_nodes.iter().cloned().collect(),
                    });
                }
            }
        }

        Ok(results)
    }

    pub(super) fn matches_query(&self, entry: &AssetIndex, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        // Check name
        if entry.metadata.name.to_lowercase().contains(&query_lower) {
            return true;
        }

        // Check description
        if entry
            .metadata
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(&query_lower)
        {
            return true;
        }

        // Check keywords
        for keyword in &entry.keywords {
            if keyword.contains(&query_lower) {
                return true;
            }
        }

        // Check tags
        for tag in &entry.metadata.tags {
            if tag.to_lowercase().contains(&query_lower) {
                return true;
            }
        }

        false
    }

    pub(super) fn calculate_relevance(&self, metadata: &AssetMetadata, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let mut score: f64 = 0.0;

        // Name match (highest weight)
        if metadata.name.to_lowercase() == query_lower {
            score += 1.0;
        } else if metadata.name.to_lowercase().contains(&query_lower) {
            score += 0.7;
        }

        // Tag match
        for tag in &metadata.tags {
            if tag.to_lowercase() == query_lower {
                score += 0.5;
            } else if tag.to_lowercase().contains(&query_lower) {
                score += 0.3;
            }
        }

        // Description match
        if metadata
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(&query_lower)
        {
            score += 0.2;
        }

        score.min(1.0_f64)
    }

    pub(super) fn calculate_popularity(&self, stats: &UsageStats) -> f64 {
        let download_score = (stats.downloads as f64 / 10000.0).min(1.0_f64);
        let weekly_score = (stats.weekly_downloads as f64 / 1000.0).min(1.0_f64);
        let star_score = (stats.stars as f64 / 100.0).min(1.0_f64);

        download_score * 0.4 + weekly_score * 0.4 + star_score * 0.2
    }

    pub(super) fn get_cached_popularity(&self, asset_id: &AssetRegistration) -> f64 {
        if let Ok(cache) = self.federated_cache.try_read() {
            if let Some(entry) = cache.entries.get(asset_id) {
                return self.calculate_popularity(&entry.usage_stats);
            }
        }
        if let Ok(index) = self.local_index.try_read() {
            if let Some(entry) = index.get(asset_id) {
                return self.calculate_popularity(&entry.usage_stats);
            }
        }
        0.0
    }

    pub(super) async fn search_cache(
        &self,
        query: &str,
    ) -> Result<Option<Vec<(AssetRegistration, AssetMetadata)>>> {
        let cache = self.federated_cache.read().await;
        let elapsed = SystemTime::now()
            .duration_since(cache.cached_at)
            .unwrap_or(Duration::from_secs(u64::MAX));
        if elapsed > cache.ttl || cache.entries.is_empty() {
            return Ok(None);
        }
        let mut results = Vec::new();
        for (asset_id, entry) in cache.entries.iter() {
            if self.matches_query(entry, query) {
                results.push((asset_id.clone(), entry.metadata.clone()));
            }
        }
        if results.is_empty() {
            Ok(None)
        } else {
            Ok(Some(results))
        }
    }

    pub(super) async fn cache_search_results(
        &self,
        _query: &str,
        results: &[(AssetRegistration, AssetMetadata)],
    ) -> Result<()> {
        let mut cache = self.federated_cache.write().await;
        for (asset_id, metadata) in results {
            if !cache.entries.contains_key(asset_id) {
                cache.entries.insert(
                    asset_id.clone(),
                    AssetIndex {
                        asset_id: asset_id.clone(),
                        metadata: metadata.clone(),
                        available_nodes: HashSet::new(),
                        permissions: SharePermission::Public,
                        indexed_at: SystemTime::now(),
                        keywords: self.extract_keywords(metadata),
                        categories: metadata.tags.clone(),
                        dependencies: vec![],
                        usage_stats: UsageStats::default(),
                    },
                );
            }
        }
        cache.cached_at = SystemTime::now();
        Ok(())
    }

    pub(super) fn deduplicate_results(
        &self,
        mut results: Vec<(AssetRegistration, AssetMetadata)>,
    ) -> Vec<(AssetRegistration, AssetMetadata)> {
        results.sort_by(|a, b| a.0.to_hex_string().cmp(&b.0.to_hex_string()));
        results.dedup_by(|a, b| a.0 == b.0);
        results
    }

    pub(super) fn rank_results(
        &self,
        mut results: Vec<(AssetRegistration, AssetMetadata)>,
        query: &str,
    ) -> Vec<(AssetRegistration, AssetMetadata)> {
        results.sort_by(|a, b| {
            let rel_a = self.calculate_relevance(&a.1, query);
            let rel_b = self.calculate_relevance(&b.1, query);
            rel_b.partial_cmp(&rel_a).expect("relevance scores should be valid for comparison")
        });
        results
    }

    pub(super) async fn update_recommendations(&self, entry: &AssetIndex) -> Result<()> {
        let mut engine = self.recommendation_engine.write().await;

        // Update category associations
        for category in &entry.categories {
            engine
                .category_associations
                .entry(category.clone())
                .or_insert_with(Vec::new)
                .push(entry.asset_id.clone());
        }

        // Update similarity matrix (simplified)
        // Would use more sophisticated similarity calculation
        for dep in &entry.dependencies {
            engine
                .similarity_matrix
                .entry(entry.asset_id.clone())
                .or_insert_with(Vec::new)
                .push((dep.clone(), 0.8));
        }

        Ok(())
    }

    pub(super) fn full_text_match(&self, entry: &AssetIndex, query: &str) -> (bool, Vec<String>) {
        let mut highlights = Vec::new();
        let query_lower = query.to_lowercase();

        // Check all text fields
        if entry.metadata.name.to_lowercase().contains(&query_lower) {
            highlights.push(entry.metadata.name.clone());
            return (true, highlights);
        }

        if entry
            .metadata
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(&query_lower)
        {
            // Extract matching portion
            if let Some(ref desc) = entry.metadata.description {
                if let Some(start) = desc.to_lowercase().find(&query_lower) {
                    let end = (start + 100).min(desc.len());
                    highlights.push(desc[start..end].to_string());
                }
            }
            return (true, highlights);
        }

        (false, highlights)
    }

    pub(super) fn fuzzy_distance(&self, s1: &str, s2: &str) -> Option<usize> {
        // Levenshtein edit distance via dynamic programming.
        let a: Vec<char> = s1.to_lowercase().chars().collect();
        let b: Vec<char> = s2.to_lowercase().chars().collect();
        let (m, n) = (a.len(), b.len());

        if m.abs_diff(n) > m.max(n) / 2 + 3 {
            return None;
        }

        let mut prev = (0..=n).collect::<Vec<usize>>();
        let mut curr = vec![0usize; n + 1];

        for i in 1..=m {
            curr[0] = i;
            for j in 1..=n {
                let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
                curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
            }
            std::mem::swap(&mut prev, &mut curr);
        }

        Some(prev[n])
    }

    // Network operation stubs

    async fn search_peer(
        peer_id: &str,
        peer_address: &str,
        query: &str,
    ) -> Result<Vec<(AssetRegistration, AssetMetadata)>> {
        // Network-dependent: requires STOQ wire protocol.
        tracing::debug!(
            peer_id = %peer_id,
            peer_address = %peer_address,
            query = %query,
            "Search request built for peer; requires STOQ transport for execution"
        );
        Ok(Vec::new())
    }

    pub(super) async fn request_peer_index(
        &self,
        peer_id: &str,
        peer_address: &str,
    ) -> Result<HashMap<AssetRegistration, AssetIndex>> {
        // Network-dependent: requires STOQ wire protocol.
        tracing::debug!(
            peer_id = %peer_id,
            peer_address = %peer_address,
            "Index request built for peer; requires STOQ transport for execution"
        );
        Ok(HashMap::new())
    }
}
