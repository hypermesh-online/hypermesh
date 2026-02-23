// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AssetDiscovery implementation and search helpers for HyperMeshAssetRegistry.

use crate::assets::*;
use crate::registry::{
    SearchQuery, LegacySearchResults as SearchResults,
    SortCriteria,
    AssetDiscovery, AssetFilters, AssetIndexEntry, AssetSearchResult,
    RecommendationContext
};

use anyhow::Result;
use std::collections::HashMap;

use super::{HyperMeshAssetRegistry, CatalogMetadata, SearchIndex};

#[async_trait::async_trait]
impl AssetDiscovery for HyperMeshAssetRegistry {
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults> {
        let start_time = std::time::Instant::now();
        let cache = self.catalog_cache.read().await;

        let mut results = Vec::new();

        if query.query.is_empty() {
            // Return all assets if no query
            for (package_id, metadata) in &cache.package_metadata {
                if self.matches_filters(metadata, query).await {
                    results.push(AssetSearchResult {
                        entry: self.metadata_to_index_entry(*package_id, metadata).await?,
                        score: 1.0,
                        matched_fields: vec![],
                    });
                }
            }
        } else {
            // Perform text search using inverted index
            let query_terms: Vec<String> = query.query
                .split_whitespace()
                .map(|s| s.to_lowercase())
                .collect();

            let mut scored_results: HashMap<AssetPackageId, f64> = HashMap::new();

            for term in &query_terms {
                if let Some(package_ids) = cache.search_index.inverted_index.get(term) {
                    for &package_id in package_ids {
                        if let Some(metadata) = cache.package_metadata.get(&package_id) {
                            if self.matches_filters(metadata, query).await {
                                let score = self.calculate_relevance_score(
                                    &package_id,
                                    term,
                                    &cache.search_index
                                );
                                *scored_results.entry(package_id).or_insert(0.0) += score;
                            }
                        }
                    }
                }
            }

            // Convert to results and sort by score
            let mut scored_vec: Vec<_> = scored_results.into_iter().collect();
            scored_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (package_id, score) in scored_vec {
                if let Some(metadata) = cache.package_metadata.get(&package_id) {
                    results.push(AssetSearchResult {
                        entry: self.metadata_to_index_entry(package_id, metadata).await?,
                        score: score / query_terms.len() as f64,
                        matched_fields: self.generate_highlights(metadata, &query_terms),
                    });
                }
            }
        }

        // Apply sorting
        self.sort_results(&mut results, &query.sort_by);

        // Apply pagination
        let total_count = results.len();
        let end = std::cmp::min(query.offset + query.limit, results.len());
        if query.offset < results.len() {
            results = results[query.offset..end].to_vec();
        } else {
            results.clear();
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(SearchResults {
            assets: results,
            total_count,
            execution_time_ms: execution_time,
            query: query.query.clone(),
        })
    }

    async fn get_asset(&self, id: &AssetPackageId) -> Result<Option<AssetPackage>> {
        match self.install(id).await {
            Ok(package) => Ok(Some(package)),
            Err(_) => Ok(None),
        }
    }

    async fn list_assets(&self, filters: &AssetFilters) -> Result<Vec<AssetIndexEntry>> {
        let cache = self.catalog_cache.read().await;
        let mut results = Vec::new();

        for (package_id, metadata) in &cache.package_metadata {
            if self.matches_asset_filters(metadata, filters).await {
                results.push(self.metadata_to_index_entry(*package_id, metadata).await?);
            }
        }

        results.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(results)
    }

    async fn get_recommendations(&self, context: &RecommendationContext) -> Result<Vec<AssetIndexEntry>> {
        let cache = self.catalog_cache.read().await;
        let mut recommendations = Vec::new();

        for (package_id, metadata) in &cache.package_metadata {
            if context.current_assets.contains(package_id) {
                continue;
            }

            let mut score = 0.0;

            // Score by preferred tags
            for tag in &metadata.tags {
                if context.preferred_tags.contains(tag) {
                    score += 1.0;
                }
            }

            // Score by rating
            if let Some(stats) = cache.package_stats.get(package_id) {
                score += stats.rating;
            }

            if score > 0.0 {
                recommendations.push((
                    self.metadata_to_index_entry(*package_id, metadata).await?,
                    score
                ));
            }
        }

        recommendations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Default to 10 recommendations if not specified
        const DEFAULT_RECOMMENDATION_COUNT: usize = 10;

        Ok(recommendations.into_iter()
            .take(DEFAULT_RECOMMENDATION_COUNT)
            .map(|(entry, _)| entry)
            .collect())
    }
}

impl HyperMeshAssetRegistry {
    /// Check if metadata matches search filters
    pub(super) async fn matches_filters(&self, metadata: &CatalogMetadata, query: &SearchQuery) -> bool {
        if let Some(_asset_type) = &query.asset_type {
            // Would need to fetch from library to check type
            // For now, assume match
        }

        if !query.tags.is_empty() {
            let has_all_tags = query.tags.iter().all(|tag| metadata.tags.contains(tag));
            if !has_all_tags {
                return false;
            }
        }

        if let Some(author) = &query.author {
            if metadata.author.as_ref() != Some(author) {
                return false;
            }
        }

        true
    }

    /// Check if metadata matches asset filters
    pub(super) async fn matches_asset_filters(&self, metadata: &CatalogMetadata, filters: &AssetFilters) -> bool {
        if !filters.tags.is_empty() {
            let has_all_tags = filters.tags.iter().all(|tag| metadata.tags.contains(tag));
            if !has_all_tags {
                return false;
            }
        }

        if let Some(author) = &filters.author {
            if metadata.author.as_ref() != Some(author) {
                return false;
            }
        }

        true
    }

    /// Convert metadata to index entry
    pub(super) async fn metadata_to_index_entry(
        &self,
        package_id: AssetPackageId,
        metadata: &CatalogMetadata,
    ) -> Result<AssetIndexEntry> {
        let stats = self.get_package_stats(&package_id).await?;

        // Fetch package info from library for complete data
        let package_info = self.asset_library.get_package(&package_id.to_string()).await
            .ok_or_else(|| anyhow::anyhow!("Package not found in library"))?;

        Ok(AssetIndexEntry {
            id: package_id,
            name: package_info.name.clone(),
            version: package_info.version.clone(),
            asset_type: package_info.asset_type.clone(),
            description: metadata.description.clone(),
            tags: metadata.tags.clone(),
            keywords: metadata.keywords.clone(),
            location: format!("hypermesh://{}", package_id),
            size: package_info.size,
            hash: package_info.hash.clone(),
            published_at: metadata.updated_at,
            updated_at: metadata.updated_at,
            registry: "hypermesh".to_string(),
            rating: stats.rating,
            download_count: stats.download_count,
            verified: true, // All HyperMesh assets are consensus-verified
        })
    }

    /// Calculate relevance score for search
    pub(super) fn calculate_relevance_score(
        &self,
        package_id: &AssetPackageId,
        term: &str,
        search_index: &SearchIndex,
    ) -> f64 {
        let tf = search_index.term_frequencies
            .get(term)
            .and_then(|freqs| freqs.get(package_id))
            .copied()
            .unwrap_or(0) as f64;

        let df = search_index.inverted_index
            .get(term)
            .map(|ids| ids.len())
            .unwrap_or(1) as f64;

        let idf = (search_index.total_documents as f64 / df).ln();

        tf * idf
    }

    /// Generate search highlights
    pub(super) fn generate_highlights(&self, metadata: &CatalogMetadata, query_terms: &[String]) -> Vec<String> {
        let mut highlights = Vec::new();

        if let Some(description) = &metadata.description {
            for term in query_terms {
                if description.to_lowercase().contains(term) {
                    highlights.push(format!("...{}...", term));
                }
            }
        }

        highlights
    }

    /// Sort search results
    pub(super) fn sort_results(&self, results: &mut [AssetSearchResult], sort_by: &SortCriteria) {
        match sort_by {
            SortCriteria::Relevance => {
                results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            }
            SortCriteria::Published => {
                results.sort_by(|a, b| b.entry.published_at.cmp(&a.entry.published_at));
            }
            SortCriteria::Updated => {
                results.sort_by(|a, b| b.entry.updated_at.cmp(&a.entry.updated_at));
            }
            SortCriteria::Downloads => {
                results.sort_by(|a, b| b.entry.download_count.cmp(&a.entry.download_count));
            }
            SortCriteria::Rating => {
                results.sort_by(|a, b| b.entry.rating.partial_cmp(&a.entry.rating).unwrap_or(std::cmp::Ordering::Equal));
            }
            SortCriteria::Name => {
                results.sort_by(|a, b| a.entry.name.cmp(&b.entry.name));
            }
        }
    }
}
