// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! `CatalogStoqApi` server setup — binds STOQ transport and registers handlers.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::{info, instrument};

use stoq::transport::{StoqTransport, TransportConfig};
use stoq::StoqApiServer;

use super::config_state::{CatalogAppState, CatalogStoqConfig};
use super::handlers::{
    BrowseHandler, CatalogHealthHandler, GetPackageHandler, GetPublisherHandler,
    RegistryStatsHandler, SearchHandler, TypeLookupHandler, TypePublishHandler,
};

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
    use super::super::*;
    use bytes::Bytes;
    use std::collections::HashMap;
    use stoq::api::{ApiError, ApiHandler, ApiRequest};

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
        let state = std::sync::Arc::new(CatalogAppState::new());
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
        let state = std::sync::Arc::new(CatalogAppState::new());
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
            SpaceProof, StakeProof, TimeProof, WorkProof,
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
        let stake = StakeProof::new("h".into(), "i".into());
        let space = SpaceProof::new("n".into(), "/t".into(), 1024);
        let work = WorkProof::from_work("o".into(), "w".into(), b"work");
        let time = TimeProof::new(std::time::Duration::from_secs(10));
        let proof = StateProof::new(stake, time, space, work);
        let type_def = AssetTypeDefinition::new("GpuCompute".to_string(), schema, proof);
        registry.register_type(type_def).await
            .expect("test: register type");

        let state = std::sync::Arc::new(CatalogAppState::with_registry(registry));
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
            SpaceProof, StakeProof, TimeProof, WorkProof,
        };
        use blockmatrix::assets::StateProof;
        use hypermesh_lib::PrivacyMode;

        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        let schema = serde_json::json!({ "type": "object" });
        let stake = StakeProof::new("h".into(), "i".into());
        let space = SpaceProof::new("n".into(), "/t".into(), 1024);
        let work = WorkProof::from_work("o".into(), "w".into(), b"work");
        let time = TimeProof::new(std::time::Duration::from_secs(10));
        let proof = StateProof::new(stake, time, space, work);
        let type_def = AssetTypeDefinition::new("MyPackage".to_string(), schema, proof);
        registry.register_type(type_def).await
            .expect("test: register type");

        let state = std::sync::Arc::new(CatalogAppState::with_registry(registry));
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
        let state = std::sync::Arc::new(CatalogAppState::new());
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
        let state = std::sync::Arc::new(CatalogAppState::new());
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
        let state = std::sync::Arc::new(CatalogAppState::new());
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
        let state = std::sync::Arc::new(CatalogAppState::new());
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
        let state = std::sync::Arc::new(CatalogAppState::new());
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
        let state = std::sync::Arc::new(CatalogAppState::new());

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
                allowed_publishers: Vec::new(),
                require_certificate: false,
            },
            RegistryConfig::default(),
        )
    }

    #[tokio::test]
    async fn test_type_publish_handler() {
        let registry = make_registry_with_no_pos();
        let state = std::sync::Arc::new(CatalogAppState::with_registry(registry));
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
        let state = std::sync::Arc::new(CatalogAppState::with_registry(registry));
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
        let state = std::sync::Arc::new(CatalogAppState::with_registry(registry));

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
        let state = std::sync::Arc::new(CatalogAppState::with_registry(registry));
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
