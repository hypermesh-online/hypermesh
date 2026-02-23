// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-Node Asset Discovery Module
//!
//! Provides global asset discovery, federated indexing, and
//! recommendation services across the HyperMesh network.

mod types;
mod search;

pub use types::*;

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime};

use crate::{AssetRegistration, AssetPackage, AssetMetadata};
use super::{PeerInfo, SharePermission};

/// Discovery service for asset search and indexing
pub struct DiscoveryService {
    pub(super) cache_ttl: Duration,
    pub(super) local_index: Arc<RwLock<HashMap<AssetRegistration, AssetIndex>>>,
    pub(super) federated_cache: Arc<RwLock<IndexCache>>,
    pub(super) search_capabilities: Arc<SearchCapabilities>,
    pub(super) recommendation_engine: Arc<RwLock<RecommendationEngine>>,
    pub(super) index_stats: Arc<RwLock<IndexStats>>,
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
        asset_id: &AssetRegistration,
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
            // STUB: AssetMetadata doesn't have category field, use tags as categories
            categories: metadata.tags.clone(),
            // STUB: AssetMetadata doesn't have dependencies field
            dependencies: vec![],
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
    pub async fn search_local(&self, query: &str) -> Result<Vec<(AssetRegistration, AssetMetadata)>> {
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

    /// Get package from local index
    pub async fn get_local_package(&self, asset_id: &AssetRegistration) -> Result<Option<AssetPackage>> {
        let index = self.local_index.read().await;
        let entry = match index.get(asset_id) {
            Some(e) => e.clone(),
            None => return Ok(None),
        };
        Ok(Some(Self::build_package_from_metadata(
            &entry.metadata,
            &asset_id.to_hex_string(),
        )))
    }

    /// Construct a minimal AssetPackage from metadata and hash.
    fn build_package_from_metadata(metadata: &AssetMetadata, hash: &str) -> AssetPackage {
        AssetPackage {
            spec: crate::AssetSpec {
                api_version: "v1".to_string(),
                kind: "Asset".to_string(),
                metadata: metadata.clone(),
                spec: Self::default_asset_specification(),
            },
            content: Self::default_content_resolved(),
            validation: Self::default_validation_status(),
            package_hash: hash.to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            signature: None,
        }
    }

    fn default_asset_specification() -> crate::assets::types::AssetSpecification {
        crate::assets::types::AssetSpecification {
            asset_type: "package".to_string(),
            content: crate::AssetContent {
                main: String::new(), files: vec![], inline: None,
                binary: vec![], templates: vec![],
            },
            security: crate::AssetSecurity {
                consensus_required: false, certificate_pinning: false,
                hash_validation: "sha256".to_string(),
                sandbox_level: "standard".to_string(), allowed_syscalls: vec![],
                network_access: crate::assets::types::NetworkAccess {
                    enabled: false, allowed_domains: vec![],
                    allowed_ports: vec![], require_tls: true,
                },
                file_access: crate::assets::types::FileAccess {
                    level: "read".to_string(), allowed_paths: vec![],
                    denied_paths: vec![], allow_temp: false,
                },
                permissions: vec![],
            },
            resources: crate::AssetResources {
                cpu_limit: "1".to_string(), memory_limit: "128M".to_string(),
                execution_timeout: "60s".to_string(), storage_required: None,
                network_bandwidth: None, gpu_required: false,
                hardware_requirements: vec![],
            },
            execution: crate::AssetExecution {
                delegation_strategy: "any".to_string(), minimum_consensus: 1,
                retry_policy: "none".to_string(), max_concurrent: None,
                priority: "normal".to_string(),
                timeout_config: crate::assets::types::TimeoutConfig {
                    execution: "60s".to_string(), network: "30s".to_string(),
                    io: "30s".to_string(), compilation: None,
                },
                scheduling: crate::assets::types::SchedulingConfig {
                    timing: "immediate".to_string(),
                    allocation_strategy: "best-fit".to_string(),
                    node_affinity: vec![], anti_affinity: vec![],
                },
            },
            dependencies: vec![],
            environment: std::collections::HashMap::new(),
            config_schema: None,
        }
    }

    fn default_content_resolved() -> crate::assets::types::AssetContentResolved {
        crate::assets::types::AssetContentResolved {
            main_content: String::new(),
            file_contents: std::collections::HashMap::new(),
            binary_contents: std::collections::HashMap::new(),
            template_content: std::collections::HashMap::new(),
            resolved_dependencies: vec![],
        }
    }

    fn default_validation_status() -> crate::assets::registry::AssetValidationStatus {
        crate::assets::registry::AssetValidationStatus {
            is_valid: false,
            validated_at: chrono::Utc::now(),
            errors: vec![],
            warnings: vec![],
            security_results: crate::assets::registry::SecurityScanResults {
                security_score: 0,
                vulnerabilities: vec![],
                recommendations: vec![],
                scanned_at: chrono::Utc::now(),
            },
            dependency_results: Default::default(),
        }
    }

    /// Check if package exists locally
    pub async fn has_package(&self, asset_id: &AssetRegistration) -> Result<bool> {
        let index = self.local_index.read().await;
        Ok(index.contains_key(asset_id))
    }

    /// Get popular packages
    pub async fn get_popular_packages(&self, threshold: f64) -> Result<Vec<(AssetRegistration, AssetMetadata)>> {
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

    pub(super) fn get_local_node_id(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"catalog-discovery-local-node");
        hasher.update(self.cache_ttl.as_secs().to_le_bytes());
        format!("node_{}", hex::encode(&hasher.finalize()[..8]))
    }

    pub(super) fn extract_keywords(&self, metadata: &AssetMetadata) -> Vec<String> {
        let mut keywords = Vec::new();

        // Extract from name
        keywords.extend(metadata.name.split_whitespace().map(|s| s.to_lowercase()));

        // Extract from description
        if let Some(desc) = &metadata.description {
            keywords.extend(desc.split_whitespace()
                .filter(|s| s.len() > 3)
                .map(|s| s.to_lowercase())
                .take(20));
        }

        // Add tags
        keywords.extend(metadata.tags.clone());

        // Deduplicate
        keywords.sort();
        keywords.dedup();

        keywords
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
