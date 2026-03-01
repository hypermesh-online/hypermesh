// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};

use super::super::SharePermission;
use crate::{PackageSpecMetadata, AssetRegistration};

/// Asset index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    /// Asset ID
    pub asset_id: AssetRegistration,
    /// Asset metadata
    pub metadata: PackageSpecMetadata,
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
    pub dependencies: Vec<AssetRegistration>,
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
    pub asset_id: AssetRegistration,
    /// Recommendation score
    pub score: f64,
    /// Reason for recommendation
    pub reason: RecommendationReason,
    /// Related assets
    pub related: Vec<AssetRegistration>,
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
pub(in crate::sharing) struct IndexCache {
    /// Cached index entries
    pub entries: HashMap<AssetRegistration, AssetIndex>,
    /// Cache timestamp
    pub cached_at: SystemTime,
    /// Cache validity duration
    pub ttl: Duration,
}

/// Index statistics
#[derive(Debug, Clone, Default)]
pub(in crate::sharing) struct IndexStats {
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
    pub _avg_search_time: u64,
}

/// Recommendation engine
pub(in crate::sharing) struct RecommendationEngine {
    /// User interaction history
    pub user_history: HashMap<String, Vec<AssetRegistration>>,
    /// Asset similarity matrix
    pub similarity_matrix: HashMap<AssetRegistration, Vec<(AssetRegistration, f64)>>,
    /// Trending packages
    pub trending: Vec<AssetRegistration>,
    /// Category associations
    pub category_associations: HashMap<String, Vec<AssetRegistration>>,
}
