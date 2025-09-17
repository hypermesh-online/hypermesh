//! Asset Registry and Discovery System
//!
//! Provides distributed asset registry capabilities for publishing, discovering,
//! and managing asset packages across the Catalog ecosystem.

use crate::assets::*;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tokio::sync::RwLock;
use std::sync::Arc;

/// Asset registry for managing and discovering asset packages
pub struct AssetRegistry {
    /// Registry configuration
    config: RegistryConfig,
    /// Local asset index
    local_index: Arc<RwLock<AssetIndex>>,
    /// Remote registry clients
    remote_registries: HashMap<String, Box<dyn RegistryClient>>,
    /// Asset cache directory
    cache_dir: PathBuf,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Local registry directory
    pub local_dir: String,
    /// Cache directory for downloaded assets
    pub cache_dir: String,
    /// Remote registries to sync with
    pub remote_registries: Vec<RemoteRegistry>,
    /// Asset indexing configuration
    pub indexing: IndexingConfig,
    /// Asset verification settings
    pub verification: VerificationConfig,
    /// Download and upload settings
    pub network: NetworkConfig,
}

/// Remote registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRegistry {
    /// Registry name
    pub name: String,
    /// Registry URL
    pub url: String,
    /// Authentication token
    pub auth_token: Option<String>,
    /// Whether to sync automatically
    pub auto_sync: bool,
    /// Sync interval in seconds
    pub sync_interval: u64,
    /// Whether this is a trusted registry
    pub trusted: bool,
}

/// Asset indexing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Whether to index content for search
    pub index_content: bool,
    /// Whether to generate search keywords
    pub generate_keywords: bool,
    /// Maximum file size to index (bytes)
    pub max_index_size: u64,
    /// File patterns to exclude from indexing
    pub exclude_patterns: Vec<String>,
}

/// Asset verification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// Whether to verify asset signatures
    pub verify_signatures: bool,
    /// Whether to scan for vulnerabilities
    pub security_scan: bool,
    /// Whether to validate dependencies
    pub validate_dependencies: bool,
    /// Trusted public keys for signature verification
    pub trusted_keys: Vec<String>,
}

/// Network configuration for registry operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// HTTP timeout in seconds
    pub timeout: u64,
    /// Maximum concurrent downloads
    pub max_concurrent_downloads: usize,
    /// Retry attempts for failed downloads
    pub retry_attempts: u32,
    /// User agent for HTTP requests
    pub user_agent: String,
}

/// Asset index for fast lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    /// Indexed assets by ID
    pub assets: HashMap<AssetPackageId, AssetIndexEntry>,
    /// Assets by name
    pub by_name: HashMap<String, Vec<AssetPackageId>>,
    /// Assets by tag
    pub by_tag: HashMap<String, Vec<AssetPackageId>>,
    /// Assets by type
    pub by_type: HashMap<String, Vec<AssetPackageId>>,
    /// Full-text search index
    pub search_index: SearchIndex,
    /// Index metadata
    pub metadata: IndexMetadata,
}

/// Asset index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexEntry {
    /// Asset package ID
    pub id: AssetPackageId,
    /// Asset name
    pub name: String,
    /// Asset version
    pub version: String,
    /// Asset type
    pub asset_type: String,
    /// Asset description
    pub description: Option<String>,
    /// Asset tags
    pub tags: Vec<String>,
    /// Asset keywords for search
    pub keywords: Vec<String>,
    /// Asset file path (local or URL)
    pub location: String,
    /// Asset size in bytes
    pub size: u64,
    /// Asset hash for integrity
    pub hash: String,
    /// Publication timestamp
    pub published_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Registry source
    pub registry: String,
    /// Asset rating/quality score
    pub rating: f64,
    /// Download count
    pub download_count: u64,
    /// Whether asset is verified
    pub verified: bool,
}

/// Full-text search index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndex {
    /// Inverted index: term -> asset IDs
    pub inverted_index: HashMap<String, Vec<AssetPackageId>>,
    /// Document frequencies for scoring
    pub term_frequencies: HashMap<String, HashMap<AssetPackageId, u32>>,
    /// Total documents indexed
    pub total_documents: usize,
}

/// Index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Index version
    pub version: String,
    /// Index creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Total assets indexed
    pub total_assets: usize,
    /// Index size in bytes
    pub index_size: u64,
}

/// Asset discovery interface
#[async_trait::async_trait]
pub trait AssetDiscovery {
    /// Search for assets by query
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults>;
    
    /// Get asset by ID
    async fn get_asset(&self, id: &AssetPackageId) -> Result<Option<AssetPackage>>;
    
    /// List assets with filters
    async fn list_assets(&self, filters: &AssetFilters) -> Result<Vec<AssetIndexEntry>>;
    
    /// Get asset recommendations
    async fn get_recommendations(&self, context: &RecommendationContext) -> Result<Vec<AssetIndexEntry>>;
}

/// Search query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search terms
    pub query: String,
    /// Asset type filter
    pub asset_type: Option<String>,
    /// Tag filters
    pub tags: Vec<String>,
    /// Author filter
    pub author: Option<String>,
    /// Version constraints
    pub version: Option<String>,
    /// Date range filter
    pub date_range: Option<DateRange>,
    /// Sort criteria
    pub sort_by: SortCriteria,
    /// Maximum results to return
    pub limit: usize,
    /// Offset for pagination
    pub offset: usize,
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    /// Start date
    pub from: DateTime<Utc>,
    /// End date
    pub to: DateTime<Utc>,
}

/// Sort criteria for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortCriteria {
    /// Sort by relevance score
    Relevance,
    /// Sort by creation date (newest first)
    DateCreated,
    /// Sort by update date (newest first)
    DateUpdated,
    /// Sort by download count (highest first)
    Popularity,
    /// Sort by rating (highest first)
    Rating,
    /// Sort by name (alphabetical)
    Name,
}

/// Asset filters for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFilters {
    /// Asset type filter
    pub asset_type: Option<String>,
    /// Tag filters (all must match)
    pub tags: Vec<String>,
    /// Author filter
    pub author: Option<String>,
    /// Verified assets only
    pub verified_only: bool,
    /// Minimum rating
    pub min_rating: Option<f64>,
    /// Registry source filter
    pub registry: Option<String>,
}

/// Recommendation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationContext {
    /// Currently used assets
    pub current_assets: Vec<AssetPackageId>,
    /// User's preferred tags
    pub preferred_tags: Vec<String>,
    /// Asset type preferences
    pub asset_type_preferences: Vec<String>,
    /// Recommendation count
    pub count: usize,
}

/// Search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// Matching assets
    pub assets: Vec<AssetSearchResult>,
    /// Total matching assets (for pagination)
    pub total_count: usize,
    /// Search execution time in milliseconds
    pub execution_time_ms: u64,
    /// Search query that was executed
    pub query: String,
}

/// Individual search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSearchResult {
    /// Asset index entry
    pub asset: AssetIndexEntry,
    /// Relevance score (0.0 - 1.0)
    pub score: f64,
    /// Highlighted search terms in description
    pub highlights: Vec<String>,
}

/// Registry client trait for remote registries
#[async_trait::async_trait]
pub trait RegistryClient: Send + Sync {
    /// Search assets in remote registry
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults>;
    
    /// Download asset package
    async fn download(&self, id: &AssetPackageId) -> Result<AssetPackage>;
    
    /// Upload asset package
    async fn upload(&self, package: &AssetPackage) -> Result<AssetPackageId>;
    
    /// Get asset metadata
    async fn get_metadata(&self, id: &AssetPackageId) -> Result<AssetIndexEntry>;
    
    /// List all assets
    async fn list_all(&self) -> Result<Vec<AssetIndexEntry>>;
    
    /// Check registry health
    async fn health_check(&self) -> Result<bool>;
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            local_dir: "~/.catalog/registry".to_string(),
            cache_dir: "~/.catalog/cache".to_string(),
            remote_registries: vec![
                RemoteRegistry {
                    name: "hypermesh-official".to_string(),
                    url: "https://registry.hypermesh.online".to_string(),
                    auth_token: None,
                    auto_sync: true,
                    sync_interval: 3600, // 1 hour
                    trusted: true,
                },
            ],
            indexing: IndexingConfig {
                index_content: true,
                generate_keywords: true,
                max_index_size: 10 * 1024 * 1024, // 10MB
                exclude_patterns: vec![
                    "*.bin".to_string(),
                    "*.so".to_string(),
                    "*.dll".to_string(),
                ],
            },
            verification: VerificationConfig {
                verify_signatures: true,
                security_scan: true,
                validate_dependencies: true,
                trusted_keys: vec![],
            },
            network: NetworkConfig {
                timeout: 30,
                max_concurrent_downloads: 4,
                retry_attempts: 3,
                user_agent: "Catalog-Registry/1.0".to_string(),
            },
        }
    }
}

impl AssetRegistry {
    /// Create a new asset registry
    pub async fn new(config: RegistryConfig) -> Result<Self> {
        let cache_dir = shellexpand::tilde(&config.cache_dir).into_owned().into();
        tokio::fs::create_dir_all(&cache_dir).await?;
        
        let local_index = Arc::new(RwLock::new(AssetIndex {
            assets: HashMap::new(),
            by_name: HashMap::new(),
            by_tag: HashMap::new(),
            by_type: HashMap::new(),
            search_index: SearchIndex {
                inverted_index: HashMap::new(),
                term_frequencies: HashMap::new(),
                total_documents: 0,
            },
            metadata: IndexMetadata {
                version: "1.0.0".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                total_assets: 0,
                index_size: 0,
            },
        }));
        
        let mut registry = Self {
            config: config.clone(),
            local_index,
            remote_registries: HashMap::new(),
            cache_dir,
        };
        
        // Initialize remote registry clients
        for remote in &config.remote_registries {
            let client = HttpRegistryClient::new(remote.clone())?;
            registry.remote_registries.insert(remote.name.clone(), Box::new(client));
        }
        
        // Load existing index
        registry.load_index().await?;
        
        Ok(registry)
    }
    
    /// Publish an asset package to the registry
    pub async fn publish(&self, package: AssetPackage) -> Result<AssetPackageId> {
        let package_id = package.get_package_id();
        
        // Store package locally
        let package_dir = self.cache_dir.join(package_id.to_string());
        tokio::fs::create_dir_all(&package_dir).await?;
        
        // Save package files
        let package_file = package_dir.join("package.json");
        let package_json = serde_json::to_string_pretty(&package)?;
        tokio::fs::write(package_file, package_json).await?;
        
        // Create index entry
        let index_entry = AssetIndexEntry {
            id: package_id,
            name: package.spec.metadata.name.clone(),
            version: package.spec.metadata.version.clone(),
            asset_type: package.spec.spec.asset_type.clone(),
            description: package.spec.metadata.description.clone(),
            tags: package.spec.metadata.tags.clone(),
            keywords: self.generate_keywords(&package),
            location: package_dir.to_string_lossy().to_string(),
            size: self.calculate_package_size(&package),
            hash: package.package_hash.clone(),
            published_at: Utc::now(),
            updated_at: Utc::now(),
            registry: "local".to_string(),
            rating: 0.0,
            download_count: 0,
            verified: false, // TODO: Implement verification
        };
        
        // Update index
        self.add_to_index(index_entry).await?;
        
        // Publish to remote registries if configured
        for (name, client) in &self.remote_registries {
            if let Err(e) = client.upload(&package).await {
                tracing::warn!("Failed to upload to registry {}: {}", name, e);
            }
        }
        
        Ok(package_id)
    }
    
    /// Install an asset package by ID
    pub async fn install(&self, id: &AssetPackageId) -> Result<AssetPackage> {
        // Check local cache first
        if let Some(package) = self.get_from_cache(id).await? {
            return Ok(package);
        }
        
        // Try to download from remote registries
        for (name, client) in &self.remote_registries {
            match client.download(id).await {
                Ok(package) => {
                    // Cache the downloaded package
                    self.cache_package(&package).await?;
                    return Ok(package);
                }
                Err(e) => {
                    tracing::debug!("Failed to download from {}: {}", name, e);
                }
            }
        }
        
        Err(anyhow::anyhow!("Asset package {} not found in any registry", id))
    }
    
    /// Generate search keywords for an asset
    fn generate_keywords(&self, package: &AssetPackage) -> Vec<String> {
        let mut keywords = Vec::new();
        
        // Add name words
        keywords.extend(
            package.spec.metadata.name
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_lowercase())
        );
        
        // Add description words
        if let Some(desc) = &package.spec.metadata.description {
            keywords.extend(
                desc.split_whitespace()
                    .filter(|s| s.len() > 2) // Skip short words
                    .map(|s| s.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                    .filter(|s| !s.is_empty())
            );
        }
        
        // Add tags
        keywords.extend(package.spec.metadata.tags.iter().map(|t| t.to_lowercase()));
        
        // Add asset type
        keywords.push(package.spec.spec.asset_type.clone());
        
        // Remove duplicates and sort
        keywords.sort();
        keywords.dedup();
        
        keywords
    }
    
    /// Calculate package size
    fn calculate_package_size(&self, package: &AssetPackage) -> u64 {
        let mut size = 0u64;
        
        size += package.content.main_content.len() as u64;
        
        for content in package.content.file_contents.values() {
            size += content.len() as u64;
        }
        
        for content in package.content.binary_contents.values() {
            size += content.len() as u64;
        }
        
        size
    }
    
    /// Add asset to index
    async fn add_to_index(&self, entry: AssetIndexEntry) -> Result<()> {
        let mut index = self.local_index.write().await;
        
        // Add to main index
        index.assets.insert(entry.id, entry.clone());
        
        // Add to name index
        index.by_name.entry(entry.name.clone())
            .or_insert_with(Vec::new)
            .push(entry.id);
        
        // Add to tag index
        for tag in &entry.tags {
            index.by_tag.entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(entry.id);
        }
        
        // Add to type index
        index.by_type.entry(entry.asset_type.clone())
            .or_insert_with(Vec::new)
            .push(entry.id);
        
        // Update search index
        self.update_search_index(&mut index, &entry);
        
        // Update metadata
        index.metadata.updated_at = Utc::now();
        index.metadata.total_assets = index.assets.len();
        
        // Save index
        self.save_index(&index).await?;
        
        Ok(())
    }
    
    /// Update search index for an asset
    fn update_search_index(&self, index: &mut AssetIndex, entry: &AssetIndexEntry) {
        let mut terms = Vec::new();
        
        // Index keywords
        terms.extend(entry.keywords.iter().cloned());
        
        // Index name
        terms.push(entry.name.clone());
        
        // Index description
        if let Some(desc) = &entry.description {
            terms.extend(
                desc.split_whitespace()
                    .map(|s| s.to_lowercase())
                    .filter(|s| s.len() > 2)
            );
        }
        
        // Update inverted index
        for term in &terms {
            index.search_index.inverted_index
                .entry(term.clone())
                .or_insert_with(Vec::new)
                .push(entry.id);
            
            // Update term frequencies
            *index.search_index.term_frequencies
                .entry(term.clone())
                .or_insert_with(HashMap::new)
                .entry(entry.id)
                .or_insert(0) += 1;
        }
        
        index.search_index.total_documents += 1;
    }
    
    /// Get asset from local cache
    async fn get_from_cache(&self, id: &AssetPackageId) -> Result<Option<AssetPackage>> {
        let package_dir = self.cache_dir.join(id.to_string());
        let package_file = package_dir.join("package.json");
        
        if !package_file.exists() {
            return Ok(None);
        }
        
        let package_json = tokio::fs::read_to_string(package_file).await?;
        let package: AssetPackage = serde_json::from_str(&package_json)?;
        
        Ok(Some(package))
    }
    
    /// Cache a downloaded package
    async fn cache_package(&self, package: &AssetPackage) -> Result<()> {
        let package_id = package.get_package_id();
        let package_dir = self.cache_dir.join(package_id.to_string());
        tokio::fs::create_dir_all(&package_dir).await?;
        
        let package_file = package_dir.join("package.json");
        let package_json = serde_json::to_string_pretty(package)?;
        tokio::fs::write(package_file, package_json).await?;
        
        Ok(())
    }
    
    /// Load index from disk
    async fn load_index(&self) -> Result<()> {
        let index_file = self.cache_dir.join("index.json");
        
        if !index_file.exists() {
            return Ok(()); // No existing index
        }
        
        let index_json = tokio::fs::read_to_string(index_file).await?;
        let loaded_index: AssetIndex = serde_json::from_str(&index_json)?;
        
        let mut index = self.local_index.write().await;
        *index = loaded_index;
        
        Ok(())
    }
    
    /// Save index to disk
    async fn save_index(&self, index: &AssetIndex) -> Result<()> {
        let index_file = self.cache_dir.join("index.json");
        let index_json = serde_json::to_string_pretty(index)?;
        tokio::fs::write(index_file, index_json).await?;
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl AssetDiscovery for AssetRegistry {
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults> {
        let start_time = std::time::Instant::now();
        let index = self.local_index.read().await;
        
        let mut results = Vec::new();
        
        if query.query.is_empty() {
            // Return all assets if no query
            for entry in index.assets.values() {
                if self.matches_filters(entry, query) {
                    results.push(AssetSearchResult {
                        asset: entry.clone(),
                        score: 1.0,
                        highlights: vec![],
                    });
                }
            }
        } else {
            // Perform text search
            let query_terms: Vec<String> = query.query
                .split_whitespace()
                .map(|s| s.to_lowercase())
                .collect();
            
            let mut scored_results: HashMap<AssetPackageId, f64> = HashMap::new();
            
            for term in &query_terms {
                if let Some(asset_ids) = index.search_index.inverted_index.get(term) {
                    for &asset_id in asset_ids {
                        if let Some(entry) = index.assets.get(&asset_id) {
                            if self.matches_filters(entry, query) {
                                let score = self.calculate_relevance_score(entry, term, &index.search_index);
                                *scored_results.entry(asset_id).or_insert(0.0) += score;
                            }
                        }
                    }
                }
            }
            
            // Convert to results and sort by score
            let mut scored_vec: Vec<_> = scored_results.into_iter().collect();
            scored_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            for (asset_id, score) in scored_vec {
                if let Some(entry) = index.assets.get(&asset_id) {
                    results.push(AssetSearchResult {
                        asset: entry.clone(),
                        score: score / query_terms.len() as f64, // Normalize score
                        highlights: self.generate_highlights(entry, &query_terms),
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
        self.get_from_cache(id).await
    }
    
    async fn list_assets(&self, filters: &AssetFilters) -> Result<Vec<AssetIndexEntry>> {
        let index = self.local_index.read().await;
        let mut results = Vec::new();
        
        for entry in index.assets.values() {
            if self.matches_asset_filters(entry, filters) {
                results.push(entry.clone());
            }
        }
        
        // Sort by name
        results.sort_by(|a, b| a.name.cmp(&b.name));
        
        Ok(results)
    }
    
    async fn get_recommendations(&self, context: &RecommendationContext) -> Result<Vec<AssetIndexEntry>> {
        let index = self.local_index.read().await;
        let mut recommendations = Vec::new();
        
        // Simple recommendation based on tags and types
        for entry in index.assets.values() {
            if context.current_assets.contains(&entry.id) {
                continue; // Skip already used assets
            }
            
            let mut score = 0.0;
            
            // Score by preferred tags
            for tag in &entry.tags {
                if context.preferred_tags.contains(tag) {
                    score += 1.0;
                }
            }
            
            // Score by asset type preferences
            if context.asset_type_preferences.contains(&entry.asset_type) {
                score += 2.0;
            }
            
            // Score by rating
            score += entry.rating;
            
            if score > 0.0 {
                recommendations.push((entry.clone(), score));
            }
        }
        
        // Sort by score
        recommendations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Return top recommendations
        Ok(recommendations.into_iter()
            .take(context.count)
            .map(|(entry, _)| entry)
            .collect())
    }
}

impl AssetRegistry {
    /// Check if entry matches search query filters
    fn matches_filters(&self, entry: &AssetIndexEntry, query: &SearchQuery) -> bool {
        if let Some(asset_type) = &query.asset_type {
            if entry.asset_type != *asset_type {
                return false;
            }
        }
        
        if !query.tags.is_empty() {
            let has_all_tags = query.tags.iter().all(|tag| entry.tags.contains(tag));
            if !has_all_tags {
                return false;
            }
        }
        
        if let Some(date_range) = &query.date_range {
            if entry.published_at < date_range.from || entry.published_at > date_range.to {
                return false;
            }
        }
        
        true
    }
    
    /// Check if entry matches asset filters
    fn matches_asset_filters(&self, entry: &AssetIndexEntry, filters: &AssetFilters) -> bool {
        if let Some(asset_type) = &filters.asset_type {
            if entry.asset_type != *asset_type {
                return false;
            }
        }
        
        if !filters.tags.is_empty() {
            let has_all_tags = filters.tags.iter().all(|tag| entry.tags.contains(tag));
            if !has_all_tags {
                return false;
            }
        }
        
        if filters.verified_only && !entry.verified {
            return false;
        }
        
        if let Some(min_rating) = filters.min_rating {
            if entry.rating < min_rating {
                return false;
            }
        }
        
        if let Some(registry) = &filters.registry {
            if entry.registry != *registry {
                return false;
            }
        }
        
        true
    }
    
    /// Calculate relevance score for a search term
    fn calculate_relevance_score(&self, entry: &AssetIndexEntry, term: &str, search_index: &SearchIndex) -> f64 {
        let tf = search_index.term_frequencies
            .get(term)
            .and_then(|freqs| freqs.get(&entry.id))
            .copied()
            .unwrap_or(0) as f64;
        
        let df = search_index.inverted_index
            .get(term)
            .map(|ids| ids.len())
            .unwrap_or(1) as f64;
        
        let idf = (search_index.total_documents as f64 / df).ln();
        
        tf * idf
    }
    
    /// Generate highlights for search results
    fn generate_highlights(&self, entry: &AssetIndexEntry, query_terms: &[String]) -> Vec<String> {
        let mut highlights = Vec::new();
        
        if let Some(description) = &entry.description {
            for term in query_terms {
                if description.to_lowercase().contains(term) {
                    // Simple highlighting - could be improved
                    highlights.push(format!("...{}...", term));
                }
            }
        }
        
        highlights
    }
    
    /// Sort search results
    fn sort_results(&self, results: &mut [AssetSearchResult], sort_by: &SortCriteria) {
        match sort_by {
            SortCriteria::Relevance => {
                results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            }
            SortCriteria::DateCreated => {
                results.sort_by(|a, b| b.asset.published_at.cmp(&a.asset.published_at));
            }
            SortCriteria::DateUpdated => {
                results.sort_by(|a, b| b.asset.updated_at.cmp(&a.asset.updated_at));
            }
            SortCriteria::Popularity => {
                results.sort_by(|a, b| b.asset.download_count.cmp(&a.asset.download_count));
            }
            SortCriteria::Rating => {
                results.sort_by(|a, b| b.asset.rating.partial_cmp(&a.asset.rating).unwrap_or(std::cmp::Ordering::Equal));
            }
            SortCriteria::Name => {
                results.sort_by(|a, b| a.asset.name.cmp(&b.asset.name));
            }
        }
    }
}

/// HTTP-based registry client implementation
pub struct HttpRegistryClient {
    /// Remote registry configuration
    config: RemoteRegistry,
    /// HTTP client
    client: reqwest::Client,
}

impl HttpRegistryClient {
    /// Create a new HTTP registry client
    pub fn new(config: RemoteRegistry) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Catalog-Registry/1.0")
            .build()?;
        
        Ok(Self { config, client })
    }
}

#[async_trait::async_trait]
impl RegistryClient for HttpRegistryClient {
    async fn search(&self, query: &SearchQuery) -> Result<SearchResults> {
        let url = format!("{}/api/v1/search", self.config.url);
        
        let mut request = self.client.post(&url).json(query);
        
        if let Some(token) = &self.config.auth_token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        let results: SearchResults = response.json().await?;
        
        Ok(results)
    }
    
    async fn download(&self, id: &AssetPackageId) -> Result<AssetPackage> {
        let url = format!("{}/api/v1/assets/{}", self.config.url, id);
        
        let mut request = self.client.get(&url);
        
        if let Some(token) = &self.config.auth_token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        let package: AssetPackage = response.json().await?;
        
        Ok(package)
    }
    
    async fn upload(&self, package: &AssetPackage) -> Result<AssetPackageId> {
        let url = format!("{}/api/v1/assets", self.config.url);
        
        let mut request = self.client.post(&url).json(package);
        
        if let Some(token) = &self.config.auth_token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        let result: serde_json::Value = response.json().await?;
        
        let id_str = result["id"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response from registry"))?;
        
        let id = Uuid::parse_str(id_str)?;
        
        Ok(id)
    }
    
    async fn get_metadata(&self, id: &AssetPackageId) -> Result<AssetIndexEntry> {
        let url = format!("{}/api/v1/assets/{}/metadata", self.config.url, id);
        
        let mut request = self.client.get(&url);
        
        if let Some(token) = &self.config.auth_token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        let metadata: AssetIndexEntry = response.json().await?;
        
        Ok(metadata)
    }
    
    async fn list_all(&self) -> Result<Vec<AssetIndexEntry>> {
        let url = format!("{}/api/v1/assets", self.config.url);
        
        let mut request = self.client.get(&url);
        
        if let Some(token) = &self.config.auth_token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await?;
        let assets: Vec<AssetIndexEntry> = response.json().await?;
        
        Ok(assets)
    }
    
    async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/v1/health", self.config.url);
        
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_registry_creation() {
        let temp_dir = TempDir::new().unwrap();
        
        let config = RegistryConfig {
            local_dir: temp_dir.path().join("registry").to_string_lossy().to_string(),
            cache_dir: temp_dir.path().join("cache").to_string_lossy().to_string(),
            remote_registries: vec![],
            indexing: IndexingConfig::default(),
            verification: VerificationConfig::default(),
            network: NetworkConfig::default(),
        };
        
        let registry = AssetRegistry::new(config).await.unwrap();
        
        // Test empty search
        let query = SearchQuery {
            query: "".to_string(),
            asset_type: None,
            tags: vec![],
            author: None,
            version: None,
            date_range: None,
            sort_by: SortCriteria::Relevance,
            limit: 10,
            offset: 0,
        };
        
        let results = registry.search(&query).await.unwrap();
        assert_eq!(results.total_count, 0);
    }
}