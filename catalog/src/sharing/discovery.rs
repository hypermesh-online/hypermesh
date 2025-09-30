//! Cross-Node Asset Discovery Module
//!
//! Provides global asset discovery, federated indexing, and
//! recommendation services across the HyperMesh network.

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

use crate::{AssetId, AssetPackage, AssetMetadata};
use super::{PeerInfo, SharePermission};

/// Asset index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    /// Asset ID
    pub asset_id: AssetId,
    /// Asset metadata
    pub metadata: AssetMetadata,
    /// Nodes that have this asset
    pub available_nodes: HashSet<String>,
    /// Share permissions
    pub permissions: SharePermission,
    /// Index timestamp
    pub indexed_at: SystemTime,
    /// Search keywords
    pub keywords: Vec<String>,
    /// Categories
    pub categories: Vec<String>,
    /// Dependencies
    pub dependencies: Vec<AssetId>,
    /// Usage statistics
    pub usage_stats: UsageStats,
}

/// Usage statistics for assets
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    /// Total downloads
    pub downloads: u64,
    /// Weekly downloads
    pub weekly_downloads: u64,
    /// Monthly downloads
    pub monthly_downloads: u64,
    /// Star count
    pub stars: u32,
    /// Fork count
    pub forks: u32,
    /// Issue count
    pub issues: u32,
    /// Last updated
    pub last_updated: Option<SystemTime>,
}

/// Search capabilities configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCapabilities {
    /// Enable full-text search
    pub full_text: bool,
    /// Enable semantic search
    pub semantic: bool,
    /// Enable fuzzy matching
    pub fuzzy: bool,
    /// Maximum results
    pub max_results: usize,
    /// Enable relevance scoring
    pub relevance_scoring: bool,
    /// Search timeout
    pub timeout: Duration,
}

impl Default for SearchCapabilities {
    fn default() -> Self {
        Self {
            full_text: true,
            semantic: false,
            fuzzy: true,
            max_results: 100,
            relevance_scoring: true,
            timeout: Duration::from_secs(5),
        }
    }
}

/// Search result with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Asset index entry
    pub index: AssetIndex,
    /// Relevance score (0-1)
    pub relevance: f64,
    /// Match highlights
    pub highlights: Vec<String>,
    /// Source nodes
    pub sources: Vec<String>,
}

/// Recommendation based on usage patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Recommended asset
    pub asset_id: AssetId,
    /// Recommendation score
    pub score: f64,
    /// Reason for recommendation
    pub reason: RecommendationReason,
    /// Related assets
    pub related: Vec<AssetId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationReason {
    /// Based on similar assets
    Similar,
    /// Based on user history
    UserHistory,
    /// Based on dependencies
    Dependency,
    /// Based on popularity
    Trending,
    /// Based on category
    Category,
    /// Based on collaborative filtering
    Collaborative,
}

/// Federated index cache
#[derive(Debug, Clone)]
struct IndexCache {
    /// Cached index entries
    entries: HashMap<AssetId, AssetIndex>,
    /// Cache timestamp
    cached_at: SystemTime,
    /// Cache validity duration
    ttl: Duration,
}

/// Discovery service for asset search and indexing
pub struct DiscoveryService {
    cache_ttl: Duration,
    local_index: Arc<RwLock<HashMap<AssetId, AssetIndex>>>,
    federated_cache: Arc<RwLock<IndexCache>>,
    search_capabilities: Arc<SearchCapabilities>,
    recommendation_engine: Arc<RwLock<RecommendationEngine>>,
    index_stats: Arc<RwLock<IndexStats>>,
}

/// Index statistics
#[derive(Debug, Clone, Default)]
struct IndexStats {
    /// Total indexed packages
    pub total_packages: u64,
    /// Local packages
    pub local_packages: u64,
    /// Federated packages
    pub federated_packages: u64,
    /// Total searches
    pub total_searches: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Average search time (ms)
    pub avg_search_time: u64,
}

/// Recommendation engine
struct RecommendationEngine {
    /// User interaction history
    user_history: HashMap<String, Vec<AssetId>>,
    /// Asset similarity matrix
    similarity_matrix: HashMap<AssetId, Vec<(AssetId, f64)>>,
    /// Trending packages
    trending: Vec<AssetId>,
    /// Category associations
    category_associations: HashMap<String, Vec<AssetId>>,
}

impl DiscoveryService {
    /// Create new discovery service
    pub async fn new(cache_ttl: Duration) -> Result<Self> {
        Ok(Self {
            cache_ttl,
            local_index: Arc::new(RwLock::new(HashMap::new())),
            federated_cache: Arc::new(RwLock::new(IndexCache {
                entries: HashMap::new(),
                cached_at: SystemTime::now(),
                ttl: cache_ttl,
            })),
            search_capabilities: Arc::new(SearchCapabilities::default()),
            recommendation_engine: Arc::new(RwLock::new(RecommendationEngine {
                user_history: HashMap::new(),
                similarity_matrix: HashMap::new(),
                trending: Vec::new(),
                category_associations: HashMap::new(),
            })),
            index_stats: Arc::new(RwLock::new(IndexStats::default())),
        })
    }

    /// Register package in local index
    pub async fn register_package(
        &self,
        asset_id: &AssetId,
        metadata: &AssetMetadata,
        permissions: SharePermission,
    ) -> Result<()> {
        let index_entry = AssetIndex {
            asset_id: asset_id.clone(),
            metadata: metadata.clone(),
            available_nodes: HashSet::from([self.get_local_node_id()]),
            permissions,
            indexed_at: SystemTime::now(),
            keywords: self.extract_keywords(metadata),
            categories: vec![metadata.category.clone()],
            dependencies: metadata.dependencies.clone(),
            usage_stats: UsageStats::default(),
        };

        // Add to local index
        let mut index = self.local_index.write().await;
        index.insert(asset_id.clone(), index_entry.clone());

        // Update stats
        let mut stats = self.index_stats.write().await;
        stats.local_packages += 1;
        stats.total_packages += 1;

        // Update recommendation engine
        self.update_recommendations(&index_entry).await?;

        Ok(())
    }

    /// Search local index
    pub async fn search_local(&self, query: &str) -> Result<Vec<(AssetId, AssetMetadata)>> {
        let index = self.local_index.read().await;
        let mut results = Vec::new();

        for (asset_id, entry) in index.iter() {
            if self.matches_query(&entry, query) {
                results.push((asset_id.clone(), entry.metadata.clone()));
            }
        }

        // Sort by relevance
        if self.search_capabilities.relevance_scoring {
            results.sort_by(|a, b| {
                let score_a = self.calculate_relevance(&a.1, query);
                let score_b = self.calculate_relevance(&b.1, query);
                score_b.partial_cmp(&score_a).unwrap()
            });
        }

        // Limit results
        results.truncate(self.search_capabilities.max_results);

        // Update stats
        let mut stats = self.index_stats.write().await;
        stats.total_searches += 1;
        stats.cache_hits += 1;

        Ok(results)
    }

    /// Search across network
    pub async fn search_network(
        &self,
        query: &str,
        peers: &HashMap<String, PeerInfo>,
    ) -> Result<Vec<(AssetId, AssetMetadata)>> {
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
            let task = tokio::spawn(async move {
                Self::search_peer(&peer_id, &peer_address, &query).await
            });
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

    /// Get package from local index
    pub async fn get_local_package(&self, asset_id: &AssetId) -> Result<Option<AssetPackage>> {
        // Would retrieve from local storage
        // This is a placeholder implementation
        Ok(None)
    }

    /// Check if package exists locally
    pub async fn has_package(&self, asset_id: &AssetId) -> Result<bool> {
        let index = self.local_index.read().await;
        Ok(index.contains_key(asset_id))
    }

    /// Get popular packages
    pub async fn get_popular_packages(&self, threshold: f64) -> Result<Vec<(AssetId, AssetMetadata)>> {
        let index = self.local_index.read().await;
        let mut popular = Vec::new();

        for (asset_id, entry) in index.iter() {
            let popularity = self.calculate_popularity(&entry.usage_stats);
            if popularity >= threshold {
                popular.push((asset_id.clone(), entry.metadata.clone()));
            }
        }

        // Sort by popularity
        popular.sort_by(|a, b| {
            let pop_a = self.get_cached_popularity(&a.0);
            let pop_b = self.get_cached_popularity(&b.0);
            pop_b.partial_cmp(&pop_a).unwrap()
        });

        Ok(popular)
    }

    /// Get recommendations for user
    pub async fn get_recommendations(
        &self,
        user_id: &str,
        count: usize,
    ) -> Result<Vec<Recommendation>> {
        let engine = self.recommendation_engine.read().await;
        let mut recommendations = Vec::new();

        // Get user history
        if let Some(history) = engine.user_history.get(user_id) {
            // Find similar assets
            for asset_id in history.iter().take(10) {
                if let Some(similar) = engine.similarity_matrix.get(asset_id) {
                    for (similar_id, score) in similar.iter().take(5) {
                        recommendations.push(Recommendation {
                            asset_id: similar_id.clone(),
                            score: *score,
                            reason: RecommendationReason::Similar,
                            related: vec![asset_id.clone()],
                        });
                    }
                }
            }
        }

        // Add trending packages
        for (i, trending_id) in engine.trending.iter().take(count / 2).enumerate() {
            recommendations.push(Recommendation {
                asset_id: trending_id.clone(),
                score: 0.9 - (i as f64 * 0.1),
                reason: RecommendationReason::Trending,
                related: Vec::new(),
            });
        }

        // Sort by score and limit
        recommendations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        recommendations.truncate(count);

        Ok(recommendations)
    }

    /// Update index with peer information
    pub async fn update_from_peer(&self, peer: &PeerInfo) -> Result<()> {
        // Request peer's index
        let peer_index = self.request_peer_index(&peer.node_id, &peer.address).await?;

        // Update federated cache
        let mut cache = self.federated_cache.write().await;
        for (asset_id, index_entry) in peer_index {
            cache.entries.insert(asset_id, index_entry);
        }
        cache.cached_at = SystemTime::now();

        // Update stats
        let mut stats = self.index_stats.write().await;
        stats.federated_packages = cache.entries.len() as u64;
        stats.total_packages = stats.local_packages + stats.federated_packages;

        Ok(())
    }

    /// Perform full-text search
    pub async fn full_text_search(&self, query: &str) -> Result<Vec<SearchResult>> {
        if !self.search_capabilities.full_text {
            return Ok(Vec::new());
        }

        let index = self.local_index.read().await;
        let mut results = Vec::new();

        for (_asset_id, entry) in index.iter() {
            let (matches, highlights) = self.full_text_match(&entry, query);
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
    pub async fn fuzzy_search(&self, query: &str, max_distance: usize) -> Result<Vec<SearchResult>> {
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

    // Helper methods

    fn get_local_node_id(&self) -> String {
        // Would get actual node ID
        "local".to_string()
    }

    fn extract_keywords(&self, metadata: &AssetMetadata) -> Vec<String> {
        let mut keywords = Vec::new();

        // Extract from name
        keywords.extend(metadata.name.split_whitespace().map(|s| s.to_lowercase()));

        // Extract from description
        keywords.extend(metadata.description.split_whitespace()
            .filter(|s| s.len() > 3)
            .map(|s| s.to_lowercase())
            .take(20));

        // Add tags
        keywords.extend(metadata.tags.clone());

        // Deduplicate
        keywords.sort();
        keywords.dedup();

        keywords
    }

    fn matches_query(&self, entry: &AssetIndex, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        // Check name
        if entry.metadata.name.to_lowercase().contains(&query_lower) {
            return true;
        }

        // Check description
        if entry.metadata.description.to_lowercase().contains(&query_lower) {
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

    fn calculate_relevance(&self, metadata: &AssetMetadata, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let mut score = 0.0;

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
        if metadata.description.to_lowercase().contains(&query_lower) {
            score += 0.2;
        }

        score.min(1.0)
    }

    fn calculate_popularity(&self, stats: &UsageStats) -> f64 {
        let download_score = (stats.downloads as f64 / 10000.0).min(1.0);
        let weekly_score = (stats.weekly_downloads as f64 / 1000.0).min(1.0);
        let star_score = (stats.stars as f64 / 100.0).min(1.0);

        download_score * 0.4 + weekly_score * 0.4 + star_score * 0.2
    }

    fn get_cached_popularity(&self, _asset_id: &AssetId) -> f64 {
        // Would get from cache
        0.5
    }

    async fn search_cache(&self, _query: &str) -> Result<Option<Vec<(AssetId, AssetMetadata)>>> {
        // Would check cache
        Ok(None)
    }

    async fn cache_search_results(
        &self,
        _query: &str,
        _results: &[(AssetId, AssetMetadata)],
    ) -> Result<()> {
        // Would cache results
        Ok(())
    }

    fn deduplicate_results(
        &self,
        mut results: Vec<(AssetId, AssetMetadata)>,
    ) -> Vec<(AssetId, AssetMetadata)> {
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results.dedup_by(|a, b| a.0 == b.0);
        results
    }

    fn rank_results(
        &self,
        mut results: Vec<(AssetId, AssetMetadata)>,
        query: &str,
    ) -> Vec<(AssetId, AssetMetadata)> {
        results.sort_by(|a, b| {
            let rel_a = self.calculate_relevance(&a.1, query);
            let rel_b = self.calculate_relevance(&b.1, query);
            rel_b.partial_cmp(&rel_a).unwrap()
        });
        results
    }

    async fn update_recommendations(&self, entry: &AssetIndex) -> Result<()> {
        let mut engine = self.recommendation_engine.write().await;

        // Update category associations
        for category in &entry.categories {
            engine.category_associations
                .entry(category.clone())
                .or_insert_with(Vec::new)
                .push(entry.asset_id.clone());
        }

        // Update similarity matrix (simplified)
        // Would use more sophisticated similarity calculation
        for dep in &entry.dependencies {
            engine.similarity_matrix
                .entry(entry.asset_id.clone())
                .or_insert_with(Vec::new)
                .push((dep.clone(), 0.8));
        }

        Ok(())
    }

    fn full_text_match(&self, entry: &AssetIndex, query: &str) -> (bool, Vec<String>) {
        let mut highlights = Vec::new();
        let query_lower = query.to_lowercase();

        // Check all text fields
        if entry.metadata.name.to_lowercase().contains(&query_lower) {
            highlights.push(entry.metadata.name.clone());
            return (true, highlights);
        }

        if entry.metadata.description.to_lowercase().contains(&query_lower) {
            // Extract matching portion
            let desc = &entry.metadata.description;
            if let Some(start) = desc.to_lowercase().find(&query_lower) {
                let end = (start + 100).min(desc.len());
                highlights.push(desc[start..end].to_string());
            }
            return (true, highlights);
        }

        (false, highlights)
    }

    fn fuzzy_distance(&self, s1: &str, s2: &str) -> Option<usize> {
        // Simplified Levenshtein distance
        // Would use proper fuzzy matching library
        if s1 == s2 {
            return Some(0);
        }

        if s1.len().abs_diff(s2.len()) > 3 {
            return None;
        }

        // Very simplified distance calculation
        let mut distance = 0;
        for (c1, c2) in s1.chars().zip(s2.chars()) {
            if c1 != c2 {
                distance += 1;
            }
        }
        distance += s1.len().abs_diff(s2.len());

        Some(distance)
    }

    // Network operation stubs

    async fn search_peer(
        _peer_id: &str,
        _peer_address: &str,
        _query: &str,
    ) -> Result<Vec<(AssetId, AssetMetadata)>> {
        // Would perform actual network search
        Ok(Vec::new())
    }

    async fn request_peer_index(
        &self,
        _peer_id: &str,
        _peer_address: &str,
    ) -> Result<HashMap<AssetId, AssetIndex>> {
        // Would request index from peer
        Ok(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discovery_service_creation() {
        let service = DiscoveryService::new(Duration::from_secs(3600)).await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_local_search() {
        let service = DiscoveryService::new(Duration::from_secs(3600)).await.unwrap();
        let results = service.search_local("test").await;
        assert!(results.is_ok());
    }
}