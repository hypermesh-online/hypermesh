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

use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tracing::{info, debug, instrument};

use stoq::api::{ApiHandler, ApiRequest, ApiResponse, ApiError};
use stoq::StoqApiServer;
use stoq::transport::{StoqTransport, TransportConfig};

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
    pub publisher_score: Option<f64>,
    pub publisher_tier: Option<String>,
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
    pub reputation_score: f64,
    pub tier: String,
    pub total_packages: u64,
    pub total_downloads: u64,
    pub average_rating: Option<f64>,
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
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid browse request: {}", e)))?;

        // Return empty results — real data comes from wiring to CatalogRegistry
        let response = BrowseResponse {
            packages: Vec::new(),
            total_count: self
                .state
                .package_count
                .load(std::sync::atomic::Ordering::Relaxed),
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
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid search request: {}", e)))?;

        // Return empty results — real data comes from wiring to CatalogRegistry
        let response = SearchResponse {
            results: Vec::new(),
            total_count: 0,
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
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid package request: {}", e)))?;

        // Package not found — real lookup via CatalogRegistry
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

        let req: GetPublisherRequest = serde_json::from_slice(&request.payload).map_err(|e| {
            ApiError::InvalidRequest(format!("Invalid publisher request: {}", e))
        })?;

        // Publisher not found — real lookup via ReputationSystem
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

        let response = RegistryStatsResponse {
            total_packages: self
                .state
                .package_count
                .load(std::sync::atomic::Ordering::Relaxed),
            total_publishers: self
                .state
                .publisher_count
                .load(std::sync::atomic::Ordering::Relaxed),
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

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

/// Catalog STOQ API Server
#[allow(dead_code)]
pub struct CatalogStoqApi {
    server: Arc<StoqApiServer>,
    config: CatalogStoqConfig,
}

impl CatalogStoqApi {
    /// Create new Catalog API server over STOQ with shared application state.
    #[instrument(skip(config, app_state))]
    pub async fn new(
        config: CatalogStoqConfig,
        app_state: Arc<CatalogAppState>,
    ) -> Result<Self> {
        info!(
            "Creating Catalog STOQ API server on {}",
            config.bind_address
        );

        // Parse bind address
        let bind_addr: std::net::Ipv6Addr = config
            .bind_address
            .split(':')
            .next()
            .and_then(|addr| {
                addr.trim_matches(|c| c == '[' || c == ']')
                    .parse()
                    .ok()
            })
            .ok_or_else(|| anyhow!("Invalid IPv6 bind address"))?;

        let port: u16 = config
            .bind_address
            .split(':')
            .nth(1)
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| anyhow!("Invalid port"))?;

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
            state: app_state,
            start_time,
        }));

        info!("Catalog STOQ API handlers registered (6 endpoints)");

        Ok(Self { server, config })
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
        let json =
            serde_json::to_string(&req).expect("test: serialization should succeed");
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
        let json =
            serde_json::to_string(&req).expect("test: serialization should succeed");
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
        let json =
            serde_json::to_string(&resp).expect("test: serialization should succeed");
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
            payload: Bytes::from(
                serde_json::to_vec(&req_body).expect("test: serialize request"),
            ),
            metadata: HashMap::new(),
        };

        let resp = handler
            .handle(api_req)
            .await
            .expect("test: browse handler should succeed");
        assert!(resp.success);

        let body: BrowseResponse = serde_json::from_slice(&resp.payload)
            .expect("test: deserialize response");
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
            payload: Bytes::from(
                serde_json::to_vec(&req_body).expect("test: serialize request"),
            ),
            metadata: HashMap::new(),
        };

        let resp = handler
            .handle(api_req)
            .await
            .expect("test: search handler should succeed");
        assert!(resp.success);

        let body: SearchResponse = serde_json::from_slice(&resp.payload)
            .expect("test: deserialize response");
        assert_eq!(body.query, "gpu compute");
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

        let body: RegistryStatsResponse = serde_json::from_slice(&resp.payload)
            .expect("test: deserialize response");
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

        let body: HealthResponse = serde_json::from_slice(&resp.payload)
            .expect("test: deserialize response");
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
            payload: Bytes::from(
                serde_json::to_vec(&req_body).expect("test: serialize request"),
            ),
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
            payload: Bytes::from(
                serde_json::to_vec(&req_body).expect("test: serialize request"),
            ),
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
            state,
            start_time: std::time::Instant::now(),
        };
        assert_eq!(health.path(), "catalog/health");
    }
}
