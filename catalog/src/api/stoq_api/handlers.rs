// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! API handlers for the Catalog STOQ API (8 endpoints).

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use stoq::api::{ApiError, ApiHandler, ApiRequest, ApiResponse};

use crate::registry::SortCriteria;
use crate::registry::asset_type::AssetTypeDefinition;

use super::config_state::CatalogAppState;
use super::message_types::{
    BrowseRequest, BrowseResponse, GetPackageRequest, GetPackageResponse, GetPublisherRequest,
    GetPublisherResponse, HealthResponse, PackageSummary, RegistryStatsResponse, SearchRequest,
    SearchResponse, TypeLookupRequest, TypeLookupResponse, TypePublishRequest, TypePublishResponse,
};

// ---------------------------------------------------------------------------
// Helpers — map registry types to API response types
// ---------------------------------------------------------------------------

/// Map a sort string from the API request to the registry's SortCriteria.
pub(crate) fn parse_sort_criteria(s: &str) -> SortCriteria {
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
pub(crate) fn to_package_summary(
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
pub(crate) fn build_package_response(
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
            // ContentHash deps are encoded as full hex for the wire format.
            dependencies: def
                .dependencies
                .iter()
                .map(|h| hex::encode(h.as_bytes()))
                .collect(),
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
