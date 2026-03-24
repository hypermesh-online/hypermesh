// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog STOQ API — package registry operations over STOQ protocol.
//!
//! Provides the catalog.hypermesh.online API surface for package browsing,
//! searching, package details, publisher info, registry stats, and health.
//!
//! All handlers hold a shared [`CatalogAppState`] wrapping the catalog's
//! registry and reputation system behind async-aware locks.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use stoq::api::{ApiError, ApiHandler, ApiRequest, ApiResponse};
use stoq::transport::{StoqTransport, TransportConfig};
use stoq::StoqApiServer;

use crate::registry::SortCriteria;

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

/// Shared application state for Catalog STOQ API handlers.
pub struct CatalogAppState {
    /// Service name
    pub service_name: String,
    /// Catalog version
    pub version: String,
    /// Package count (atomic counter)
    pub package_count: Arc<std::sync::atomic::AtomicU64>,
    /// Publisher count
    pub publisher_count: Arc<std::sync::atomic::AtomicU64>,
    /// Total downloads
    pub total_downloads: Arc<std::sync::atomic::AtomicU64>,
    /// Catalog registry for real lookups (optional for backward compat)
    pub registry: Option<crate::registry::CatalogRegistry>,
}

impl CatalogAppState {
    /// Create new state with defaults
    pub fn new() -> Self {
        Self {
            service_name: "catalog".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            package_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            publisher_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_downloads: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            registry: None,
        }
    }

    /// Create new state with a CatalogRegistry for real lookups
    pub fn with_registry(registry: crate::registry::CatalogRegistry) -> Self {
        Self {
            service_name: "catalog".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            package_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            publisher_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_downloads: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            registry: Some(registry),
        }
    }

    /// Update package count
    pub fn set_package_count(&self, count: u64) {
        self.package_count
            .store(count, std::sync::atomic::Ordering::Relaxed);
    }

    /// Update publisher count
    pub fn set_publisher_count(&self, count: u64) {
        self.publisher_count
            .store(count, std::sync::atomic::Ordering::Relaxed);
    }

    /// Increment download count
    pub fn increment_downloads(&self) {
        self.total_downloads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Sync atomic counters from registry statistics.
    pub async fn sync_from_registry(&self) {
        if let Some(ref registry) = self.registry {
            let stats = registry.get_statistics().await;
            self.set_package_count(stats.total_types as u64);
        }
    }
}

impl Default for CatalogAppState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Catalog STOQ API configuration
#[derive(Debug, Clone)]
pub struct CatalogStoqConfig {
    /// STOQ bind address (IPv6)
    pub bind_address: String,
    /// Service name
    pub service_name: String,
    /// Enable request logging
    pub enable_logging: bool,
}

impl Default for CatalogStoqConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::1]:9295".to_string(),
            service_name: "catalog".to_string(),
            enable_logging: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Browse packages request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseRequest {
    /// Category filter (optional)
    pub category: Option<String>,
    /// Sort by field
    #[serde(default = "default_sort")]
    pub sort_by: String,
    /// Page number (0-indexed)
    #[serde(default)]
    pub page: u64,
    /// Items per page
    #[serde(default = "default_page_size")]
    pub page_size: u64,
    /// Only featured packages
    #[serde(default)]
    pub featured_only: bool,
}

fn default_sort() -> String {
    "relevance".to_string()
}
fn default_page_size() -> u64 {
    20
}

/// Browse packages response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseResponse {
    pub packages: Vec<PackageSummary>,
    pub total_count: u64,
    pub page: u64,
    pub page_size: u64,
}

/// Search packages request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Search query string
    pub query: String,
    /// Tag filters
    #[serde(default)]
    pub tags: Vec<String>,
    /// Author filter
    pub author: Option<String>,
    /// Maximum results
    #[serde(default = "default_page_size")]
    pub limit: u64,
    /// Offset for pagination
    #[serde(default)]
    pub offset: u64,
}

/// Search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<PackageSummary>,
    pub total_count: u64,
    pub query: String,
}

/// Package summary (used in browse/search results)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSummary {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub download_count: u64,
    pub score: f64,
    /// Whether this is a curated/featured type definition
    pub featured: bool,
}

/// Get package details request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPackageRequest {
    /// Package name
    pub name: String,
    /// Version (None = latest)
    pub version: Option<String>,
}

/// Package detail response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPackageResponse {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub tags: Vec<String>,
    pub download_count: u64,
    pub featured: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub dependencies: Vec<String>,
    pub publisher_authenticated: Option<bool>,
    /// Schema for this type definition (JSON Schema)
    pub schema: Option<serde_json::Value>,
    /// Validation rules count
    pub validation_rules_count: u32,
}

/// Get publisher info request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPublisherRequest {
    pub publisher_id: String,
}

/// Publisher info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPublisherResponse {
    pub publisher_id: String,
    pub authenticated: bool,
    pub total_packages: u64,
    pub total_downloads: u64,
    pub member_since: Option<String>,
}

/// Registry stats response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatsResponse {
    pub total_packages: u64,
    pub total_publishers: u64,
    pub total_downloads: u64,
    pub version: String,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub package_count: u64,
    pub uptime_secs: u64,
}

/// Publish a new type definition request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypePublishRequest {
    /// Type name (e.g. "Message", "Invoice")
    pub type_name: String,
    /// JSON Schema defining the type structure
    pub schema: serde_json::Value,
    /// Semantic version (defaults to "1.0.0")
    #[serde(default = "default_version")]
    pub version: String,
    /// Optional author identifier
    pub author: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Optional tags
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// Type publish response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypePublishResponse {
    /// Human-readable type name
    pub type_name: String,
    /// Content-addressed BLAKE3 hash of the schema
    pub type_hash: String,
    /// Version that was registered
    pub version: String,
    /// Registration status
    pub status: String,
}

/// Look up a type definition request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeLookupRequest {
    /// Look up by name (optional)
    pub name: Option<String>,
    /// Look up by content hash (optional)
    pub hash: Option<String>,
}

/// Type lookup response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeLookupResponse {
    /// Whether the type was found
    pub status: String,
    /// Type name (if found)
    pub type_name: Option<String>,
    /// Content-addressed hash (if found)
    pub type_hash: Option<String>,
    /// Schema (if found)
    pub schema: Option<serde_json::Value>,
    /// Version (if found)
    pub version: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers — map registry types to API response types
// ---------------------------------------------------------------------------

use crate::registry::asset_type::AssetTypeDefinition;

/// Map a sort string from the API request to the registry's SortCriteria.
fn parse_sort_criteria(s: &str) -> SortCriteria {
    match s {
        "name" => SortCriteria::Name,
        "downloads" => SortCriteria::Downloads,
        "rating" => SortCriteria::Rating,
        "updated" => SortCriteria::Updated,
        "published" => SortCriteria::Published,
        _ => SortCriteria::Relevance,
    }
}

/// Build a [`PackageSummary`] from a search result, enriched with type metadata
/// when available.
fn to_package_summary(
    type_name: &str,
    score: f64,
    type_def: Option<&AssetTypeDefinition>,
) -> PackageSummary {
    match type_def {
        Some(def) => PackageSummary {
            name: type_name.to_string(),
            version: def.metadata.version.clone(),
            description: def.metadata.description.clone(),
            author: def.metadata.author.clone(),
            tags: def.metadata.tags.clone(),
            download_count: 0,
            score,
            featured: false,
        },
        None => PackageSummary {
            name: type_name.to_string(),
            version: String::new(),
            description: None,
            author: None,
            tags: Vec::new(),
            download_count: 0,
            score,
            featured: false,
        },
    }
}

/// Build a full [`GetPackageResponse`] from a found type, enriched with
/// metadata from the [`AssetTypeDefinition`] when available.
fn build_package_response(
    req: &GetPackageRequest,
    state: &CatalogAppState,
    type_def: Option<&AssetTypeDefinition>,
) -> GetPackageResponse {
    match type_def {
        Some(def) => GetPackageResponse {
            name: req.name.clone(),
            version: req
                .version
                .clone()
                .unwrap_or_else(|| def.metadata.version.clone()),
            description: def.metadata.description.clone(),
            author: def.metadata.author.clone(),
            license: def.metadata.license.clone(),
            homepage: None,
            repository: None,
            tags: def.metadata.tags.clone(),
            download_count: state
                .total_downloads
                .load(std::sync::atomic::Ordering::Relaxed),
            featured: false,
            created_at: Some(def.metadata.created_at.to_rfc3339()),
            updated_at: Some(def.metadata.updated_at.to_rfc3339()),
            dependencies: def.dependencies.clone(),
            publisher_authenticated: None,
            schema: Some(def.schema.clone()),
            validation_rules_count: def.validation_rules.len() as u32,
        },
        None => GetPackageResponse {
            name: req.name.clone(),
            version: req
                .version
                .clone()
                .unwrap_or_else(|| "latest".to_string()),
            description: None,
            author: None,
            license: None,
            homepage: None,
            repository: None,
            tags: Vec::new(),
            download_count: state
                .total_downloads
                .load(std::sync::atomic::Ordering::Relaxed),
            featured: false,
            created_at: None,
            updated_at: None,
            dependencies: Vec::new(),
            publisher_authenticated: None,
            schema: None,
            validation_rules_count: 0,
        },
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Browse packages handler
pub struct BrowseHandler {
    pub state: Arc<CatalogAppState>,
}

#[async_trait]
impl ApiHandler for BrowseHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling catalog/browse: {}", request.id);

        let req: BrowseRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid browse request: {e}")))?;

        // Query CatalogRegistry if available
        let sort = parse_sort_criteria(&req.sort_by);
        let (packages, total_count) = if let Some(ref registry) = self.state.registry {
            let search_query = crate::registry::SearchQuery {
                query: String::new(),
                sort_by: sort,
                limit: req.page_size as usize,
                offset: (req.page * req.page_size) as usize,
                ..Default::default()
            };
            match registry.search_types(&search_query).await {
                Ok(search_results) => {
                    let mut summaries = Vec::with_capacity(search_results.results.len());
                    for r in &search_results.results {
                        let meta = registry.get_type_definition(&r.type_name).await;
                        summaries.push(to_package_summary(
                            &r.type_name,
                            r.score,
                            meta.as_ref(),
                        ));
                    }
                    let count = search_results.total_count as u64;
                    (summaries, count)
                }
                Err(_) => {
                    let count = self
                        .state
                        .package_count
                        .load(std::sync::atomic::Ordering::Relaxed);
                    (Vec::new(), count)
                }
            }
        } else {
            let count = self
                .state
                .package_count
                .load(std::sync::atomic::Ordering::Relaxed);
            (Vec::new(), count)
        };

        let response = BrowseResponse {
            packages,
            total_count,
            page: req.page,
            page_size: req.page_size,
        };

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "catalog/browse"
    }
}

/// Search packages handler
pub struct SearchHandler {
    pub state: Arc<CatalogAppState>,
}

#[async_trait]
impl ApiHandler for SearchHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling catalog/search: {}", request.id);

        let req: SearchRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid search request: {e}")))?;

        // Query CatalogRegistry if available
        let (results, total_count) = if let Some(ref registry) = self.state.registry {
            let search_query = crate::registry::SearchQuery {
                query: req.query.clone(),
                tags: req.tags.clone(),
                author: req.author.clone(),
                limit: req.limit as usize,
                offset: req.offset as usize,
                ..Default::default()
            };
            match registry.search_types(&search_query).await {
                Ok(search_results) => {
                    let mut summaries = Vec::with_capacity(search_results.results.len());
                    for r in &search_results.results {
                        let meta = registry.get_type_definition(&r.type_name).await;
                        summaries.push(to_package_summary(
                            &r.type_name,
                            r.score,
                            meta.as_ref(),
                        ));
                    }
                    let count = search_results.total_count as u64;
                    (summaries, count)
                }
                Err(_) => (Vec::new(), 0),
            }
        } else {
            (Vec::new(), 0)
        };

        let response = SearchResponse {
            results,
            total_count,
            query: req.query,
        };

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "catalog/search"
    }
}

/// Get package details handler
pub struct GetPackageHandler {
    pub state: Arc<CatalogAppState>,
}

#[async_trait]
impl ApiHandler for GetPackageHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling catalog/package: {}", request.id);

        let req: GetPackageRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid package request: {e}")))?;

        // Look up via CatalogRegistry if available
        if let Some(ref registry) = self.state.registry {
            if let Ok(_asset_id) = registry.find_type(&req.name).await {
                self.state.increment_downloads();
                let type_def = registry.get_type_definition(&req.name).await;
                let response = build_package_response(&req, &self.state, type_def.as_ref());

                let payload = serde_json::to_vec(&response)
                    .map_err(|e| ApiError::SerializationError(e.to_string()))?;

                return Ok(ApiResponse {
                    request_id: request.id,
                    success: true,
                    payload: payload.into(),
                    error: None,
                    metadata: HashMap::new(),
                });
            }
        }

        Err(ApiError::NotFound(format!(
            "Package '{}' not found",
            req.name
        )))
    }

    fn path(&self) -> &str {
        "catalog/package"
    }
}

/// Get publisher info handler
pub struct GetPublisherHandler {
    pub state: Arc<CatalogAppState>,
}

#[async_trait]
impl ApiHandler for GetPublisherHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling catalog/publisher: {}", request.id);

        let req: GetPublisherRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid publisher request: {e}")))?;

        // Search registry for types published by this author
        if let Some(ref registry) = self.state.registry {
            let search_query = crate::registry::SearchQuery {
                query: String::new(),
                author: Some(req.publisher_id.clone()),
                limit: 10_000,
                ..Default::default()
            };
            if let Ok(search_results) = registry.search_types(&search_query).await {
                if !search_results.results.is_empty() {
                    let response = GetPublisherResponse {
                        publisher_id: req.publisher_id,
                        authenticated: true,
                        total_packages: search_results.total_count as u64,
                        total_downloads: self
                            .state
                            .total_downloads
                            .load(std::sync::atomic::Ordering::Relaxed),
                        member_since: None,
                    };

                    let payload = serde_json::to_vec(&response)
                        .map_err(|e| ApiError::SerializationError(e.to_string()))?;

                    return Ok(ApiResponse {
                        request_id: request.id,
                        success: true,
                        payload: payload.into(),
                        error: None,
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Err(ApiError::NotFound(format!(
            "Publisher '{}' not found",
            req.publisher_id
        )))
    }

    fn path(&self) -> &str {
        "catalog/publisher"
    }
}

/// Registry statistics handler
pub struct RegistryStatsHandler {
    pub state: Arc<CatalogAppState>,
}

#[async_trait]
impl ApiHandler for RegistryStatsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling catalog/stats: {}", request.id);

        // Pull live counts from the registry when available
        let (total_packages, total_publishers) =
            if let Some(ref registry) = self.state.registry {
                let stats = registry.get_statistics().await;
                (stats.total_types as u64, self
                    .state
                    .publisher_count
                    .load(std::sync::atomic::Ordering::Relaxed))
            } else {
                (
                    self.state
                        .package_count
                        .load(std::sync::atomic::Ordering::Relaxed),
                    self.state
                        .publisher_count
                        .load(std::sync::atomic::Ordering::Relaxed),
                )
            };

        let response = RegistryStatsResponse {
            total_packages,
            total_publishers,
            total_downloads: self
                .state
                .total_downloads
                .load(std::sync::atomic::Ordering::Relaxed),
            version: self.state.version.clone(),
        };

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "catalog/stats"
    }
}

/// Health check handler
pub struct CatalogHealthHandler {
    pub state: Arc<CatalogAppState>,
    pub start_time: std::time::Instant,
}

#[async_trait]
impl ApiHandler for CatalogHealthHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        let uptime = self.start_time.elapsed().as_secs();

        let response = HealthResponse {
            status: "healthy".to_string(),
            service: self.state.service_name.clone(),
            version: self.state.version.clone(),
            package_count: self
                .state
                .package_count
                .load(std::sync::atomic::Ordering::Relaxed),
            uptime_secs: uptime,
        };

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "catalog/health"
    }
}

/// Publish (register) a type definition handler
pub struct TypePublishHandler {
    pub state: Arc<CatalogAppState>,
}

#[async_trait]
impl ApiHandler for TypePublishHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling catalog/type.publish: {}", request.id);

        let req: TypePublishRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid type publish request: {e}")))?;

        let registry = self
            .state
            .registry
            .as_ref()
            .ok_or_else(|| ApiError::HandlerError("Registry not configured".to_string()))?;

        // Build a type definition from the request
        let state_proof = crate::registry::CatalogRegistry::builtin_state_proof();
        let mut type_def =
            crate::registry::asset_type::AssetTypeDefinition::new(
                req.type_name.clone(),
                req.schema.clone(),
                state_proof,
            );

        // Apply optional metadata
        type_def.metadata.version = req.version.clone();
        type_def.metadata.author = req.author;
        type_def.metadata.description = req.description;
        type_def.metadata.tags = req.tags;

        // Register via the registry
        registry
            .register_type(type_def)
            .await
            .map_err(|e| ApiError::HandlerError(format!("Registration failed: {e}")))?;

        // Compute the type hash for the response (same algorithm as registry)
        let schema_json = serde_json::to_string(&req.schema)
            .map_err(|e| ApiError::SerializationError(format!("Schema serialization: {e}")))?;
        let type_hash = hex::encode(blake3::hash(schema_json.as_bytes()).as_bytes());

        let response = TypePublishResponse {
            type_name: req.type_name,
            type_hash,
            version: req.version,
            status: "published".to_string(),
        };

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "catalog/type.publish"
    }
}

/// Look up a type definition by name or content hash
pub struct TypeLookupHandler {
    pub state: Arc<CatalogAppState>,
}

#[async_trait]
impl ApiHandler for TypeLookupHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling catalog/type.lookup: {}", request.id);

        let req: TypeLookupRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid type lookup request: {e}")))?;

        let registry = self
            .state
            .registry
            .as_ref()
            .ok_or_else(|| ApiError::HandlerError("Registry not configured".to_string()))?;

        // Try lookup by hash first (exact), then by name
        let registration = if let Some(ref hash) = req.hash {
            registry.lookup_type_by_hash(hash).await
        } else if let Some(ref name) = req.name {
            registry.lookup_type(name).await
        } else {
            None
        };

        let response = match registration {
            Some(reg) => TypeLookupResponse {
                status: "found".to_string(),
                type_name: Some(reg.type_name),
                type_hash: Some(reg.type_hash),
                schema: Some(reg.schema),
                version: Some(reg.version),
            },
            None => TypeLookupResponse {
                status: "not_found".to_string(),
                type_name: None,
                type_hash: None,
                schema: None,
                version: None,
            },
        };

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "catalog/type.lookup"
    }
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

/// Catalog STOQ API Server
pub struct CatalogStoqApi {
    server: Arc<StoqApiServer>,
    _config: CatalogStoqConfig,
}

impl CatalogStoqApi {
    /// Create new Catalog API server over STOQ with shared application state.
    #[instrument(skip(config, app_state))]
    pub async fn new(config: CatalogStoqConfig, app_state: Arc<CatalogAppState>) -> Result<Self> {
        info!(
            "Creating Catalog STOQ API server on {}",
            config.bind_address
        );

        // Parse bind address — supports [::1]:9295 and ::1:9295 formats
        let sock_addr: std::net::SocketAddrV6 = if config.bind_address.starts_with('[') {
            // Bracketed format: [::1]:9295
            let s = config.bind_address.trim_start_matches('[');
            let (addr_str, port_str) = s
                .split_once("]:")
                .ok_or_else(|| anyhow!("Invalid bind address: expected [addr]:port"))?;
            let addr: std::net::Ipv6Addr = addr_str
                .parse()
                .map_err(|e| anyhow!("Invalid IPv6 address '{}': {}", addr_str, e))?;
            let port: u16 = port_str
                .parse()
                .map_err(|e| anyhow!("Invalid port '{}': {}", port_str, e))?;
            std::net::SocketAddrV6::new(addr, port, 0, 0)
        } else {
            // Try parsing as SocketAddrV6 directly
            config
                .bind_address
                .parse::<std::net::SocketAddrV6>()
                .map_err(|e| anyhow!("Invalid bind address '{}': {}", config.bind_address, e))?
        };
        let bind_addr = *sock_addr.ip();
        let port = sock_addr.port();

        // Create STOQ transport
        let transport_config = TransportConfig {
            bind_address: bind_addr,
            port,
            ..Default::default()
        };

        let transport = Arc::new(StoqTransport::new(transport_config).await?);

        // Create API server and register handlers
        let server = Arc::new(StoqApiServer::new(transport));

        let start_time = std::time::Instant::now();

        server.register_handler(Arc::new(BrowseHandler {
            state: app_state.clone(),
        }));
        server.register_handler(Arc::new(SearchHandler {
            state: app_state.clone(),
        }));
        server.register_handler(Arc::new(GetPackageHandler {
            state: app_state.clone(),
        }));
        server.register_handler(Arc::new(GetPublisherHandler {
            state: app_state.clone(),
        }));
        server.register_handler(Arc::new(RegistryStatsHandler {
            state: app_state.clone(),
        }));
        server.register_handler(Arc::new(CatalogHealthHandler {
            state: app_state.clone(),
            start_time,
        }));
        server.register_handler(Arc::new(TypePublishHandler {
            state: app_state.clone(),
        }));
        server.register_handler(Arc::new(TypeLookupHandler {
            state: app_state,
        }));

        info!("Catalog STOQ API handlers registered (8 endpoints)");

        Ok(Self {
            server,
            _config: config,
        })
    }

    /// Start the API server
    #[instrument(skip(self))]
    pub async fn serve(self: Arc<Self>) -> Result<()> {
        info!("Starting Catalog STOQ API server...");
        self.server.listen().await
    }

    /// Stop the server gracefully
    pub fn stop(&self) {
        info!("Stopping Catalog STOQ API server");
        self.server.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_browse_request_serialization() {
        let req = BrowseRequest {
            category: Some("compute".to_string()),
            sort_by: "downloads".to_string(),
            page: 0,
            page_size: 20,
            featured_only: false,
        };
        let json = serde_json::to_string(&req).expect("test: serialization should succeed");
        assert!(json.contains("compute"));
    }

    #[test]
    fn test_search_request_serialization() {
        let req = SearchRequest {
            query: "gpu compute".to_string(),
            tags: vec!["compute".to_string()],
            author: None,
            limit: 10,
            offset: 0,
        };
        let json = serde_json::to_string(&req).expect("test: serialization should succeed");
        assert!(json.contains("gpu compute"));
    }

    #[test]
    fn test_health_response_serialization() {
        let resp = HealthResponse {
            status: "healthy".to_string(),
            service: "catalog".to_string(),
            version: "0.1.0".to_string(),
            package_count: 42,
            uptime_secs: 3600,
        };
        let json = serde_json::to_string(&resp).expect("test: serialization should succeed");
        assert!(json.contains("healthy"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_catalog_app_state() {
        let state = CatalogAppState::new();
        assert_eq!(state.service_name, "catalog");

        state.set_package_count(100);
        assert_eq!(
            state
                .package_count
                .load(std::sync::atomic::Ordering::Relaxed),
            100
        );

        state.increment_downloads();
        state.increment_downloads();
        assert_eq!(
            state
                .total_downloads
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );
    }

    #[test]
    fn test_catalog_app_state_default() {
        let state = CatalogAppState::default();
        assert_eq!(state.service_name, "catalog");
        assert_eq!(
            state
                .package_count
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[test]
    fn test_catalog_stoq_config_default() {
        let config = CatalogStoqConfig::default();
        assert_eq!(config.bind_address, "[::1]:9295");
        assert_eq!(config.service_name, "catalog");
        assert!(config.enable_logging);
    }

    #[tokio::test]
    async fn test_browse_handler() {
        let state = Arc::new(CatalogAppState::new());
        state.set_package_count(50);

        let handler = BrowseHandler { state };

        let req_body = BrowseRequest {
            category: None,
            sort_by: "relevance".to_string(),
            page: 0,
            page_size: 10,
            featured_only: false,
        };

        let api_req = ApiRequest {
            id: "test-browse-1".to_string(),
            service: "catalog".to_string(),
            method: "browse".to_string(),
            payload: Bytes::from(serde_json::to_vec(&req_body).expect("test: serialize request")),
            metadata: HashMap::new(),
        };

        let resp = handler
            .handle(api_req)
            .await
            .expect("test: browse handler should succeed");
        assert!(resp.success);

        let body: BrowseResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize response");
        assert_eq!(body.total_count, 50);
        assert_eq!(body.page, 0);
    }

    #[tokio::test]
    async fn test_search_handler() {
        let state = Arc::new(CatalogAppState::new());
        let handler = SearchHandler { state };

        let req_body = SearchRequest {
            query: "gpu compute".to_string(),
            tags: vec![],
            author: None,
            limit: 20,
            offset: 0,
        };

        let api_req = ApiRequest {
            id: "test-search-1".to_string(),
            service: "catalog".to_string(),
            method: "search".to_string(),
            payload: Bytes::from(serde_json::to_vec(&req_body).expect("test: serialize request")),
            metadata: HashMap::new(),
        };

        let resp = handler
            .handle(api_req)
            .await
            .expect("test: search handler should succeed");
        assert!(resp.success);

        let body: SearchResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize response");
        assert_eq!(body.query, "gpu compute");
    }

    #[tokio::test]
    async fn test_search_handler_with_registry() {
        use crate::registry::{CatalogRegistry, RegistryConfig, TrustPolicy};
        use crate::registry::asset_type::AssetTypeDefinition;
        use blockmatrix::proof_of_state::proof_of_state_integration::{
            SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
        };
        use blockmatrix::assets::StateProof;
        use hypermesh_lib::PrivacyMode;

        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        // Register a type
        let schema = serde_json::json!({ "type": "object" });
        let stake = StakeProof::new("h".into(), "i".into(), 1000);
        let space = SpaceProof::new("n".into(), "/t".into(), 1024);
        let work = WorkProof::new(
            "o".into(), "w".into(), 12345, 100,
            WorkloadType::Compute, WorkState::Completed,
        );
        let time = TimeProof::new(std::time::Duration::from_secs(10));
        let proof = StateProof::new(stake, time, space, work);
        let type_def = AssetTypeDefinition::new("GpuCompute".to_string(), schema, proof);
        registry.register_type(type_def).await
            .expect("test: register type");

        let state = Arc::new(CatalogAppState::with_registry(registry));
        let handler = SearchHandler { state };

        let req_body = SearchRequest {
            query: "Gpu".to_string(),
            tags: vec![],
            author: None,
            limit: 20,
            offset: 0,
        };

        let api_req = ApiRequest {
            id: "test-search-reg-1".to_string(),
            service: "catalog".to_string(),
            method: "search".to_string(),
            payload: Bytes::from(serde_json::to_vec(&req_body).expect("test: serialize")),
            metadata: HashMap::new(),
        };

        let resp = handler
            .handle(api_req)
            .await
            .expect("test: search with registry should succeed");
        assert!(resp.success);

        let body: SearchResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.total_count, 1);
        assert_eq!(body.results[0].name, "GpuCompute");
    }

    #[tokio::test]
    async fn test_get_package_with_registry() {
        use crate::registry::{CatalogRegistry, RegistryConfig, TrustPolicy};
        use crate::registry::asset_type::AssetTypeDefinition;
        use blockmatrix::proof_of_state::proof_of_state_integration::{
            SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
        };
        use blockmatrix::assets::StateProof;
        use hypermesh_lib::PrivacyMode;

        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        let schema = serde_json::json!({ "type": "object" });
        let stake = StakeProof::new("h".into(), "i".into(), 1000);
        let space = SpaceProof::new("n".into(), "/t".into(), 1024);
        let work = WorkProof::new(
            "o".into(), "w".into(), 12345, 100,
            WorkloadType::Compute, WorkState::Completed,
        );
        let time = TimeProof::new(std::time::Duration::from_secs(10));
        let proof = StateProof::new(stake, time, space, work);
        let type_def = AssetTypeDefinition::new("MyPackage".to_string(), schema, proof);
        registry.register_type(type_def).await
            .expect("test: register type");

        let state = Arc::new(CatalogAppState::with_registry(registry));
        let handler = GetPackageHandler { state };

        // Found case
        let req_body = GetPackageRequest {
            name: "MyPackage".to_string(),
            version: None,
        };
        let api_req = ApiRequest {
            id: "test-pkg-found".to_string(),
            service: "catalog".to_string(),
            method: "package".to_string(),
            payload: Bytes::from(serde_json::to_vec(&req_body).expect("test: serialize")),
            metadata: HashMap::new(),
        };
        let resp = handler.handle(api_req).await
            .expect("test: package should be found");
        assert!(resp.success);
        let body: GetPackageResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.name, "MyPackage");

        // Not found case
        let req_body2 = GetPackageRequest {
            name: "NonExistent".to_string(),
            version: None,
        };
        let api_req2 = ApiRequest {
            id: "test-pkg-miss".to_string(),
            service: "catalog".to_string(),
            method: "package".to_string(),
            payload: Bytes::from(serde_json::to_vec(&req_body2).expect("test: serialize")),
            metadata: HashMap::new(),
        };
        let result = handler.handle(api_req2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stats_handler() {
        let state = Arc::new(CatalogAppState::new());
        state.set_package_count(100);
        state.set_publisher_count(25);

        let handler = RegistryStatsHandler { state };

        let api_req = ApiRequest {
            id: "test-stats-1".to_string(),
            service: "catalog".to_string(),
            method: "stats".to_string(),
            payload: Bytes::from("{}"),
            metadata: HashMap::new(),
        };

        let resp = handler
            .handle(api_req)
            .await
            .expect("test: stats handler should succeed");
        assert!(resp.success);

        let body: RegistryStatsResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize response");
        assert_eq!(body.total_packages, 100);
        assert_eq!(body.total_publishers, 25);
    }

    #[tokio::test]
    async fn test_health_handler() {
        let state = Arc::new(CatalogAppState::new());
        let handler = CatalogHealthHandler {
            state,
            start_time: std::time::Instant::now(),
        };

        let api_req = ApiRequest {
            id: "test-health-1".to_string(),
            service: "catalog".to_string(),
            method: "health".to_string(),
            payload: Bytes::from("{}"),
            metadata: HashMap::new(),
        };

        let resp = handler
            .handle(api_req)
            .await
            .expect("test: health handler should succeed");
        assert!(resp.success);

        let body: HealthResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize response");
        assert_eq!(body.status, "healthy");
        assert_eq!(body.service, "catalog");
    }

    #[tokio::test]
    async fn test_package_not_found() {
        let state = Arc::new(CatalogAppState::new());
        let handler = GetPackageHandler { state };

        let req_body = GetPackageRequest {
            name: "nonexistent-pkg".to_string(),
            version: None,
        };

        let api_req = ApiRequest {
            id: "test-pkg-1".to_string(),
            service: "catalog".to_string(),
            method: "package".to_string(),
            payload: Bytes::from(serde_json::to_vec(&req_body).expect("test: serialize request")),
            metadata: HashMap::new(),
        };

        let result = handler.handle(api_req).await;
        assert!(result.is_err());
        match result {
            Err(ApiError::NotFound(msg)) => {
                assert!(msg.contains("nonexistent-pkg"));
            }
            other => unreachable!("test: expected NotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_publisher_not_found() {
        let state = Arc::new(CatalogAppState::new());
        let handler = GetPublisherHandler { state };

        let req_body = GetPublisherRequest {
            publisher_id: "unknown-pub".to_string(),
        };

        let api_req = ApiRequest {
            id: "test-pub-1".to_string(),
            service: "catalog".to_string(),
            method: "publisher".to_string(),
            payload: Bytes::from(serde_json::to_vec(&req_body).expect("test: serialize request")),
            metadata: HashMap::new(),
        };

        let result = handler.handle(api_req).await;
        assert!(result.is_err());
        match result {
            Err(ApiError::NotFound(msg)) => {
                assert!(msg.contains("unknown-pub"));
            }
            other => unreachable!("test: expected NotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_browse_invalid_payload() {
        let state = Arc::new(CatalogAppState::new());
        let handler = BrowseHandler { state };

        let api_req = ApiRequest {
            id: "test-bad-1".to_string(),
            service: "catalog".to_string(),
            method: "browse".to_string(),
            payload: Bytes::from("not valid json"),
            metadata: HashMap::new(),
        };

        let result = handler.handle(api_req).await;
        assert!(result.is_err());
        match result {
            Err(ApiError::InvalidRequest(msg)) => {
                assert!(msg.contains("Invalid browse request"));
            }
            other => unreachable!("test: expected InvalidRequest, got {:?}", other),
        }
    }

    #[test]
    fn test_handler_paths() {
        let state = Arc::new(CatalogAppState::new());

        let browse = BrowseHandler {
            state: state.clone(),
        };
        assert_eq!(browse.path(), "catalog/browse");

        let search = SearchHandler {
            state: state.clone(),
        };
        assert_eq!(search.path(), "catalog/search");

        let package = GetPackageHandler {
            state: state.clone(),
        };
        assert_eq!(package.path(), "catalog/package");

        let publisher = GetPublisherHandler {
            state: state.clone(),
        };
        assert_eq!(publisher.path(), "catalog/publisher");

        let stats = RegistryStatsHandler {
            state: state.clone(),
        };
        assert_eq!(stats.path(), "catalog/stats");

        let health = CatalogHealthHandler {
            state: state.clone(),
            start_time: std::time::Instant::now(),
        };
        assert_eq!(health.path(), "catalog/health");

        let type_publish = TypePublishHandler {
            state: state.clone(),
        };
        assert_eq!(type_publish.path(), "catalog/type.publish");

        let type_lookup = TypeLookupHandler { state };
        assert_eq!(type_lookup.path(), "catalog/type.lookup");
    }

    #[test]
    fn test_with_registry_constructor() {
        use crate::registry::{CatalogRegistry, RegistryConfig, TrustPolicy};
        use hypermesh_lib::PrivacyMode;

        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );
        let state = CatalogAppState::with_registry(registry);
        assert!(state.registry.is_some());
        assert_eq!(state.service_name, "catalog");
    }

    fn make_registry_with_no_pos() -> crate::registry::CatalogRegistry {
        use crate::registry::{CatalogRegistry, RegistryConfig, TrustPolicy};
        use hypermesh_lib::PrivacyMode;

        CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy {
                require_state_proof: false,
                minimum_stake: 0,
                allowed_publishers: Vec::new(),
                require_certificate: false,
            },
            RegistryConfig::default(),
        )
    }

    #[tokio::test]
    async fn test_type_publish_handler() {
        let registry = make_registry_with_no_pos();
        let state = Arc::new(CatalogAppState::with_registry(registry));
        let handler = TypePublishHandler { state };

        let req_body = TypePublishRequest {
            type_name: "Invoice".to_string(),
            schema: serde_json::json!({
                "type": "object",
                "required": ["amount"],
                "properties": { "amount": { "type": "number" } }
            }),
            version: "1.0.0".to_string(),
            author: Some("test-author".to_string()),
            description: Some("An invoice type".to_string()),
            tags: vec!["finance".to_string()],
        };

        let api_req = ApiRequest {
            id: "test-type-pub-1".to_string(),
            service: "catalog".to_string(),
            method: "type.publish".to_string(),
            payload: Bytes::from(serde_json::to_vec(&req_body).expect("test: serialize")),
            metadata: HashMap::new(),
        };

        let resp = handler
            .handle(api_req)
            .await
            .expect("test: type publish should succeed");
        assert!(resp.success);

        let body: TypePublishResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.type_name, "Invoice");
        assert_eq!(body.status, "published");
        assert!(!body.type_hash.is_empty());

        // Verify BLAKE3 hash matches
        let schema_json = serde_json::to_string(&req_body.schema).expect("test: json");
        let expected_hash = hex::encode(blake3::hash(schema_json.as_bytes()).as_bytes());
        assert_eq!(body.type_hash, expected_hash);
    }

    #[tokio::test]
    async fn test_type_publish_duplicate_fails() {
        let registry = make_registry_with_no_pos();
        let state = Arc::new(CatalogAppState::with_registry(registry));
        let handler = TypePublishHandler { state };

        let req_body = TypePublishRequest {
            type_name: "DupApi".to_string(),
            schema: serde_json::json!({ "type": "object" }),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
            tags: vec![],
        };

        let api_req1 = ApiRequest {
            id: "dup-1".to_string(),
            service: "catalog".to_string(),
            method: "type.publish".to_string(),
            payload: Bytes::from(serde_json::to_vec(&req_body).expect("test: serialize")),
            metadata: HashMap::new(),
        };
        handler.handle(api_req1).await.expect("test: first publish");

        let api_req2 = ApiRequest {
            id: "dup-2".to_string(),
            service: "catalog".to_string(),
            method: "type.publish".to_string(),
            payload: Bytes::from(serde_json::to_vec(&req_body).expect("test: serialize")),
            metadata: HashMap::new(),
        };
        let result = handler.handle(api_req2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_type_lookup_handler_by_name() {
        let registry = make_registry_with_no_pos();
        let state = Arc::new(CatalogAppState::with_registry(registry));

        // First publish a type
        let pub_handler = TypePublishHandler {
            state: state.clone(),
        };
        let pub_req = TypePublishRequest {
            type_name: "LookupTest".to_string(),
            schema: serde_json::json!({ "type": "object", "id": "lookup-test" }),
            version: "2.0.0".to_string(),
            author: None,
            description: None,
            tags: vec![],
        };
        let api_req = ApiRequest {
            id: "pub-for-lookup".to_string(),
            service: "catalog".to_string(),
            method: "type.publish".to_string(),
            payload: Bytes::from(serde_json::to_vec(&pub_req).expect("test: serialize")),
            metadata: HashMap::new(),
        };
        pub_handler.handle(api_req).await.expect("test: publish");

        // Look up by name
        let lookup_handler = TypeLookupHandler { state };
        let lookup_req = TypeLookupRequest {
            name: Some("LookupTest".to_string()),
            hash: None,
        };
        let api_req = ApiRequest {
            id: "lookup-by-name".to_string(),
            service: "catalog".to_string(),
            method: "type.lookup".to_string(),
            payload: Bytes::from(serde_json::to_vec(&lookup_req).expect("test: serialize")),
            metadata: HashMap::new(),
        };
        let resp = lookup_handler.handle(api_req).await.expect("test: lookup");
        assert!(resp.success);

        let body: TypeLookupResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.status, "found");
        assert_eq!(body.type_name.as_deref(), Some("LookupTest"));
        assert!(body.type_hash.is_some());
        assert!(body.schema.is_some());
    }

    #[tokio::test]
    async fn test_type_lookup_handler_not_found() {
        let registry = make_registry_with_no_pos();
        let state = Arc::new(CatalogAppState::with_registry(registry));
        let handler = TypeLookupHandler { state };

        let lookup_req = TypeLookupRequest {
            name: Some("DoesNotExist".to_string()),
            hash: None,
        };
        let api_req = ApiRequest {
            id: "lookup-miss".to_string(),
            service: "catalog".to_string(),
            method: "type.lookup".to_string(),
            payload: Bytes::from(serde_json::to_vec(&lookup_req).expect("test: serialize")),
            metadata: HashMap::new(),
        };
        let resp = handler.handle(api_req).await.expect("test: lookup");
        assert!(resp.success);

        let body: TypeLookupResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.status, "not_found");
        assert!(body.type_name.is_none());
    }
}
