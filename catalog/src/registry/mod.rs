// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Registry Module - Asset Type Discovery and Indexing
//!
//! This module provides the registry layer for Catalog, which is a thin
//! indexing/discovery service over BlockMatrix Assets.
//!
//! ARCHITECTURE:
//! - AssetTypeDefinition: Defines asset types (themselves BlockMatrix Assets)
//! - CatalogRegistry: Provides indexing and search functionality
//! - Everything stored as BlockMatrix Assets, not custom storage

pub mod asset_type;
pub mod catalog_registry;

// Re-export main types
pub use asset_type::{
    AssetTypeDefinition, TypeMetadata, ValidationRule, ValidationRuleType, TypeValidationResult,
};

pub use catalog_registry::{
    CatalogRegistry, TrustPolicy, RegistryConfig, SearchQuery, SearchResult, SearchResults,
    RegistryStatistics, SortCriteria, DateRange,
};

// Legacy compatibility exports (DEPRECATED - use new types above)
// These are kept temporarily to avoid breaking existing code during migration

use crate::assets::AssetPackageId;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// DEPRECATED: Use CatalogRegistry instead
///
/// This is a compatibility shim for the old AssetRegistry interface.
/// Will be removed after migration is complete.
#[deprecated(note = "Use CatalogRegistry instead")]
pub struct AssetRegistry {
    catalog_registry: CatalogRegistry,
}

#[allow(deprecated)]
impl AssetRegistry {
    /// Create new asset registry (DEPRECATED)
    pub async fn new(config: RegistryConfig) -> Result<Self> {
        let catalog_registry = CatalogRegistry::new(
            hypermesh_lib::PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            config,
        );

        Ok(Self { catalog_registry })
    }

    /// Publish asset (DEPRECATED)
    pub async fn publish(&self, _package: crate::assets::AssetPackage) -> Result<AssetPackageId> {
        // STUB: Migration compatibility
        Ok(uuid::Uuid::new_v4())
    }

    /// Install asset (DEPRECATED)
    pub async fn install(&self, _id: &AssetPackageId) -> Result<crate::assets::AssetPackage> {
        // STUB: Migration compatibility
        Err(anyhow::anyhow!("Use CatalogRegistry instead"))
    }

    /// Search assets (DEPRECATED)
    pub async fn search(&self, query: &SearchQuery) -> Result<SearchResults> {
        self.catalog_registry.search_types(query).await
    }
}

/// Asset discovery trait (DEPRECATED - for compatibility)
#[async_trait::async_trait]
pub trait AssetDiscovery {
    /// Search for assets by query
    async fn search(&self, query: &SearchQuery) -> Result<LegacySearchResults>;

    /// Get asset by ID
    async fn get_asset(&self, id: &AssetPackageId) -> Result<Option<crate::assets::AssetPackage>>;

    /// List assets with filters
    async fn list_assets(&self, filters: &AssetFilters) -> Result<Vec<AssetIndexEntry>>;

    /// Get asset recommendations
    async fn get_recommendations(&self, context: &RecommendationContext) -> Result<Vec<AssetIndexEntry>>;
}

// Legacy compatibility types (DEPRECATED - for migration only)

use chrono::{DateTime, Utc};

/// Asset filters for search (DEPRECATED)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssetFilters {
    pub asset_type: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub verified_only: bool,
    pub min_rating: Option<f64>,
    pub registry: Option<String>,
}

/// Recommendation context (DEPRECATED)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecommendationContext {
    pub current_assets: Vec<uuid::Uuid>,
    pub preferred_tags: Vec<String>,
    pub usage_history: Vec<String>,
}

/// Asset index entry (DEPRECATED)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexEntry {
    pub id: uuid::Uuid,
    pub name: String,
    pub version: String,
    pub asset_type: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    pub location: String,
    pub size: u64,
    pub hash: String,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub registry: String,
    pub rating: f64,
    pub download_count: u64,
    pub verified: bool,
}

/// Asset search result (DEPRECATED)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSearchResult {
    pub entry: AssetIndexEntry,
    pub score: f64,
    pub matched_fields: Vec<String>,
}

/// Legacy search results for AssetDiscovery trait (DEPRECATED)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacySearchResults {
    /// Matching assets
    pub assets: Vec<AssetSearchResult>,
    /// Total matching assets (for pagination)
    pub total_count: usize,
    /// Search execution time in milliseconds
    pub execution_time_ms: u64,
    /// Search query that was executed
    pub query: String,
}

// SortCriteria and DateRange now re-exported from catalog_registry above

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify new types are exported
        let _ = std::marker::PhantomData::<AssetTypeDefinition>;
        let _ = std::marker::PhantomData::<CatalogRegistry>;
        let _ = std::marker::PhantomData::<ValidationRule>;
    }
}
