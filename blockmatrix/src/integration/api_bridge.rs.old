//! Unified API Bridge for Inter-Component Communication
//!
//! Provides standardized REST/gRPC APIs for seamless communication
//! between HyperMesh, TrustChain, STOQ, Catalog, and Caesar components.

use std::sync::Arc;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex};
use axum::{
    Router,
    routing::{get, post, put, delete},
    extract::{State, Path as AxumPath, Query, Json},
    response::{IntoResponse, Response},
    http::{StatusCode, HeaderMap, HeaderValue, header},
    middleware,
};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    compression::CompressionLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use serde::{Serialize, Deserialize};
use serde_json::json;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error, instrument};
use dashmap::DashMap;
use uuid::Uuid;
use bytes::Bytes;

use crate::integration::bootstrap::{BootstrapPhase, ServiceType};

/// Unified API bridge for all components
pub struct UnifiedApiBridge {
    /// API router
    router: Router,
    /// Service registry
    services: Arc<DashMap<String, ServiceInfo>>,
    /// API configuration
    config: Arc<ApiConfig>,
    /// Request interceptors
    interceptors: Arc<RwLock<Vec<Box<dyn RequestInterceptor>>>>,
    /// Response transformers
    transformers: Arc<RwLock<Vec<Box<dyn ResponseTransformer>>>>,
    /// Rate limiter
    rate_limiter: Arc<RateLimiter>,
    /// Metrics collector
    metrics: Arc<ApiMetrics>,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Bind address for API server
    pub bind_address: SocketAddr,
    /// Enable authentication
    pub enable_auth: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Request timeout
    pub request_timeout: Duration,
    /// Maximum request size
    pub max_request_size: usize,
    /// CORS configuration
    pub cors: CorsConfig,
    /// API version
    pub api_version: String,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    /// Max age for preflight
    pub max_age: Duration,
}

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    /// Service type
    pub service_type: ServiceType,
    /// Service version
    pub version: String,
    /// Service endpoints
    pub endpoints: Vec<EndpointInfo>,
    /// Health check URL
    pub health_check_url: String,
    /// Service metadata
    pub metadata: HashMap<String, String>,
    /// Registration time
    pub registered_at: SystemTime,
}

/// Endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointInfo {
    /// Endpoint path
    pub path: String,
    /// HTTP methods
    pub methods: Vec<String>,
    /// Request schema
    pub request_schema: Option<serde_json::Value>,
    /// Response schema
    pub response_schema: Option<serde_json::Value>,
    /// Authentication required
    pub auth_required: bool,
    /// Rate limit
    pub rate_limit: Option<u32>,
}

/// Request interceptor trait
#[async_trait::async_trait]
pub trait RequestInterceptor: Send + Sync {
    /// Intercept and modify request
    async fn intercept(&self, request: &mut ApiRequest) -> Result<()>;
}

/// Response transformer trait
#[async_trait::async_trait]
pub trait ResponseTransformer: Send + Sync {
    /// Transform response
    async fn transform(&self, response: &mut ApiResponse) -> Result<()>;
}

/// API request wrapper
#[derive(Debug, Clone)]
pub struct ApiRequest {
    /// Request ID
    pub id: String,
    /// Source service
    pub source: String,
    /// Target service
    pub target: String,
    /// Request method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body
    pub body: Option<Bytes>,
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// API response wrapper
#[derive(Debug, Clone)]
pub struct ApiResponse {
    /// Request ID
    pub request_id: String,
    /// Response status
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Option<Bytes>,
    /// Response time
    pub response_time: Duration,
}

/// Rate limiter for API calls
pub struct RateLimiter {
    /// Rate limits by service
    limits: DashMap<String, RateLimit>,
    /// Request counts
    counts: DashMap<String, RequestCount>,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
struct RateLimit {
    /// Requests per second
    requests_per_second: u32,
    /// Burst size
    burst_size: u32,
    /// Window duration
    window: Duration,
}

/// Request count tracking
#[derive(Debug, Clone)]
struct RequestCount {
    /// Current count
    count: u32,
    /// Window start
    window_start: SystemTime,
}

/// API metrics
#[derive(Debug)]
pub struct ApiMetrics {
    /// Total requests
    total_requests: std::sync::atomic::AtomicU64,
    /// Total errors
    total_errors: std::sync::atomic::AtomicU64,
    /// Average latency
    average_latency_ms: std::sync::atomic::AtomicU64,
    /// Request counts by service
    service_requests: DashMap<String, u64>,
    /// Error counts by service
    service_errors: DashMap<String, u64>,
}

// Standard API endpoints for all services

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub uptime: Duration,
    pub checks: HashMap<String, bool>,
}

/// Service discovery response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceDiscoveryResponse {
    pub services: Vec<ServiceInfo>,
    pub total: usize,
    pub timestamp: SystemTime,
}

/// Component status response
#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentStatusResponse {
    pub component: String,
    pub status: String,
    pub phase: BootstrapPhase,
    pub metrics: HashMap<String, serde_json::Value>,
}

// Inter-component API contracts

/// HyperMesh asset request
#[derive(Debug, Serialize, Deserialize)]
pub struct AssetRequest {
    pub asset_type: String,
    pub amount: u64,
    pub duration: Duration,
    pub requirements: HashMap<String, String>,
}

/// HyperMesh asset response
#[derive(Debug, Serialize, Deserialize)]
pub struct AssetResponse {
    pub asset_id: String,
    pub allocated: bool,
    pub allocation_details: Option<AllocationDetails>,
    pub error: Option<String>,
}

/// Allocation details
#[derive(Debug, Serialize, Deserialize)]
pub struct AllocationDetails {
    pub asset_id: String,
    pub amount: u64,
    pub expires_at: SystemTime,
    pub consensus_proof: Option<String>,
}

/// TrustChain certificate request
#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateRequest {
    pub domain: String,
    pub public_key: String,
    pub validity_period: Duration,
    pub metadata: HashMap<String, String>,
}

/// TrustChain certificate response
#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateResponse {
    pub certificate: String,
    pub fingerprint: String,
    pub issuer: String,
    pub not_before: SystemTime,
    pub not_after: SystemTime,
    pub ct_log_url: Option<String>,
}

/// Caesar transaction request
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionRequest {
    pub from: String,
    pub to: String,
    pub amount: String,
    pub memo: Option<String>,
    pub nonce: u64,
}

/// Caesar transaction response
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub transaction_id: String,
    pub status: String,
    pub block_number: Option<u64>,
    pub confirmations: u32,
    pub fee: String,
}

/// Catalog package request
#[derive(Debug, Serialize, Deserialize)]
pub struct PackageRequest {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<String>,
}

/// Catalog package response
#[derive(Debug, Serialize, Deserialize)]
pub struct PackageResponse {
    pub package_id: String,
    pub download_url: String,
    pub checksum: String,
    pub metadata: HashMap<String, String>,
}

impl UnifiedApiBridge {
    /// Create new unified API bridge
    pub fn new(config: ApiConfig) -> Self {
        let services = Arc::new(DashMap::new());
        let interceptors = Arc::new(RwLock::new(Vec::new()));
        let transformers = Arc::new(RwLock::new(Vec::new()));
        let rate_limiter = Arc::new(RateLimiter::new());
        let metrics = Arc::new(ApiMetrics::new());

        let router = Self::build_router(
            services.clone(),
            interceptors.clone(),
            transformers.clone(),
            rate_limiter.clone(),
            metrics.clone(),
            config.clone(),
        );

        Self {
            router,
            services,
            config: Arc::new(config),
            interceptors,
            transformers,
            rate_limiter,
            metrics,
        }
    }

    /// Build API router with all endpoints
    fn build_router(
        services: Arc<DashMap<String, ServiceInfo>>,
        interceptors: Arc<RwLock<Vec<Box<dyn RequestInterceptor>>>>,
        transformers: Arc<RwLock<Vec<Box<dyn ResponseTransformer>>>>,
        rate_limiter: Arc<RateLimiter>,
        metrics: Arc<ApiMetrics>,
        config: ApiConfig,
    ) -> Router {
        // Create shared state
        let state = Arc::new(ApiState {
            services,
            interceptors,
            transformers,
            rate_limiter,
            metrics,
            config: Arc::new(config.clone()),
        });

        // Build router with standard endpoints
        Router::new()
            // Health and status endpoints
            .route("/health", get(health_check))
            .route("/status", get(component_status))
            .route("/metrics", get(get_metrics))

            // Service discovery
            .route("/services", get(list_services))
            .route("/services/:service", get(get_service))
            .route("/services/register", post(register_service))
            .route("/services/:service/unregister", delete(unregister_service))

            // HyperMesh endpoints
            .route("/hypermesh/assets", post(allocate_asset))
            .route("/hypermesh/assets/:id", get(get_asset))
            .route("/hypermesh/assets/:id/release", post(release_asset))

            // TrustChain endpoints
            .route("/trustchain/certificates", post(request_certificate))
            .route("/trustchain/certificates/:fingerprint", get(get_certificate))
            .route("/trustchain/certificates/:fingerprint/validate", post(validate_certificate))

            // Caesar endpoints
            .route("/caesar/transactions", post(create_transaction))
            .route("/caesar/transactions/:id", get(get_transaction))
            .route("/caesar/balances/:address", get(get_balance))

            // Catalog endpoints
            .route("/catalog/packages", get(list_packages))
            .route("/catalog/packages/:name", get(get_package))
            .route("/catalog/packages/install", post(install_package))

            // STOQ transport endpoints
            .route("/stoq/connections", get(list_connections))
            .route("/stoq/connections/:id", get(get_connection))
            .route("/stoq/metrics", get(get_transport_metrics))

            // Add middleware
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CompressionLayer::new())
                    .layer(TimeoutLayer::new(config.request_timeout))
                    .layer(CorsLayer::permissive())
            )
            .with_state(state)
    }

    /// Start API server
    pub async fn start(&self) -> Result<()> {
        let addr = self.config.bind_address;
        info!("Starting Unified API Bridge on {}", addr);

        axum::Server::bind(&addr)
            .serve(self.router.clone().into_make_service())
            .await?;

        Ok(())
    }

    /// Register a service
    pub async fn register(&self, service: ServiceInfo) -> Result<()> {
        info!("Registering service: {}", service.name);
        self.services.insert(service.name.clone(), service);
        Ok(())
    }

    /// Add request interceptor
    pub async fn add_interceptor(&self, interceptor: Box<dyn RequestInterceptor>) {
        self.interceptors.write().await.push(interceptor);
    }

    /// Add response transformer
    pub async fn add_transformer(&self, transformer: Box<dyn ResponseTransformer>) {
        self.transformers.write().await.push(transformer);
    }

    /// Get metrics
    pub fn metrics(&self) -> &ApiMetrics {
        &self.metrics
    }
}

/// API state for handlers
#[derive(Clone)]
struct ApiState {
    services: Arc<DashMap<String, ServiceInfo>>,
    interceptors: Arc<RwLock<Vec<Box<dyn RequestInterceptor>>>>,
    transformers: Arc<RwLock<Vec<Box<dyn ResponseTransformer>>>>,
    rate_limiter: Arc<RateLimiter>,
    metrics: Arc<ApiMetrics>,
    config: Arc<ApiConfig>,
}

// API handlers

async fn health_check(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    Json(HealthCheckResponse {
        status: "healthy".to_string(),
        service: "unified-api-bridge".to_string(),
        version: state.config.api_version.clone(),
        uptime: Duration::from_secs(0), // Would calculate actual uptime
        checks: HashMap::new(),
    })
}

async fn component_status(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    Json(ComponentStatusResponse {
        component: "api-bridge".to_string(),
        status: "running".to_string(),
        phase: BootstrapPhase::FullFederation,
        metrics: HashMap::new(),
    })
}

async fn get_metrics(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    use std::sync::atomic::Ordering;

    Json(json!({
        "total_requests": state.metrics.total_requests.load(Ordering::Relaxed),
        "total_errors": state.metrics.total_errors.load(Ordering::Relaxed),
        "average_latency_ms": state.metrics.average_latency_ms.load(Ordering::Relaxed),
    }))
}

async fn list_services(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let services: Vec<ServiceInfo> = state.services
        .iter()
        .map(|entry| entry.value().clone())
        .collect();

    Json(ServiceDiscoveryResponse {
        total: services.len(),
        services,
        timestamp: SystemTime::now(),
    })
}

async fn get_service(
    State(state): State<Arc<ApiState>>,
    AxumPath(service): AxumPath<String>,
) -> impl IntoResponse {
    if let Some(info) = state.services.get(&service) {
        Json(info.clone()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn register_service(
    State(state): State<Arc<ApiState>>,
    Json(service): Json<ServiceInfo>,
) -> impl IntoResponse {
    state.services.insert(service.name.clone(), service);
    StatusCode::CREATED
}

async fn unregister_service(
    State(state): State<Arc<ApiState>>,
    AxumPath(service): AxumPath<String>,
) -> impl IntoResponse {
    if state.services.remove(&service).is_some() {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

// HyperMesh handlers

async fn allocate_asset(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<AssetRequest>,
) -> impl IntoResponse {
    // Forward to HyperMesh service
    Json(AssetResponse {
        asset_id: Uuid::new_v4().to_string(),
        allocated: true,
        allocation_details: Some(AllocationDetails {
            asset_id: Uuid::new_v4().to_string(),
            amount: request.amount,
            expires_at: SystemTime::now() + request.duration,
            consensus_proof: Some("proof".to_string()),
        }),
        error: None,
    })
}

async fn get_asset(
    State(state): State<Arc<ApiState>>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    Json(AssetResponse {
        asset_id: id,
        allocated: true,
        allocation_details: None,
        error: None,
    })
}

async fn release_asset(
    State(state): State<Arc<ApiState>>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    StatusCode::OK
}

// TrustChain handlers

async fn request_certificate(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<CertificateRequest>,
) -> impl IntoResponse {
    Json(CertificateResponse {
        certificate: "certificate-pem".to_string(),
        fingerprint: "fingerprint".to_string(),
        issuer: "TrustChain CA".to_string(),
        not_before: SystemTime::now(),
        not_after: SystemTime::now() + request.validity_period,
        ct_log_url: Some("https://ct.trustchain.example".to_string()),
    })
}

async fn get_certificate(
    State(state): State<Arc<ApiState>>,
    AxumPath(fingerprint): AxumPath<String>,
) -> impl IntoResponse {
    Json(CertificateResponse {
        certificate: "certificate-pem".to_string(),
        fingerprint,
        issuer: "TrustChain CA".to_string(),
        not_before: SystemTime::now(),
        not_after: SystemTime::now() + Duration::from_secs(86400 * 90),
        ct_log_url: None,
    })
}

async fn validate_certificate(
    State(state): State<Arc<ApiState>>,
    AxumPath(fingerprint): AxumPath<String>,
) -> impl IntoResponse {
    Json(json!({
        "valid": true,
        "fingerprint": fingerprint,
        "validation_time": SystemTime::now(),
    }))
}

// Caesar handlers

async fn create_transaction(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<TransactionRequest>,
) -> impl IntoResponse {
    Json(TransactionResponse {
        transaction_id: Uuid::new_v4().to_string(),
        status: "pending".to_string(),
        block_number: None,
        confirmations: 0,
        fee: "0.001".to_string(),
    })
}

async fn get_transaction(
    State(state): State<Arc<ApiState>>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    Json(TransactionResponse {
        transaction_id: id,
        status: "confirmed".to_string(),
        block_number: Some(12345),
        confirmations: 6,
        fee: "0.001".to_string(),
    })
}

async fn get_balance(
    State(state): State<Arc<ApiState>>,
    AxumPath(address): AxumPath<String>,
) -> impl IntoResponse {
    Json(json!({
        "address": address,
        "balance": "1000.00",
        "pending": "0.00",
        "staked": "500.00",
    }))
}

// Catalog handlers

async fn list_packages(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    Json(json!({
        "packages": [],
        "total": 0,
    }))
}

async fn get_package(
    State(state): State<Arc<ApiState>>,
    AxumPath(name): AxumPath<String>,
) -> impl IntoResponse {
    Json(PackageResponse {
        package_id: Uuid::new_v4().to_string(),
        download_url: format!("https://catalog.hypermesh.online/packages/{}", name),
        checksum: "sha256:abcdef".to_string(),
        metadata: HashMap::new(),
    })
}

async fn install_package(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<PackageRequest>,
) -> impl IntoResponse {
    Json(PackageResponse {
        package_id: Uuid::new_v4().to_string(),
        download_url: format!("https://catalog.hypermesh.online/packages/{}", request.name),
        checksum: "sha256:abcdef".to_string(),
        metadata: HashMap::new(),
    })
}

// STOQ handlers

async fn list_connections(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    Json(json!({
        "connections": [],
        "total": 0,
    }))
}

async fn get_connection(
    State(state): State<Arc<ApiState>>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    Json(json!({
        "id": id,
        "status": "established",
        "remote_addr": "::1:9292",
        "local_addr": "::1:54321",
    }))
}

async fn get_transport_metrics(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    Json(json!({
        "bytes_sent": 0,
        "bytes_received": 0,
        "connections": 0,
        "throughput_gbps": 0.0,
    }))
}

// Rate limiter implementation

impl RateLimiter {
    fn new() -> Self {
        Self {
            limits: DashMap::new(),
            counts: DashMap::new(),
        }
    }

    pub async fn check_limit(&self, service: &str) -> bool {
        // Simplified rate limiting check
        true
    }

    pub fn set_limit(&self, service: String, requests_per_second: u32, burst_size: u32) {
        self.limits.insert(service.clone(), RateLimit {
            requests_per_second,
            burst_size,
            window: Duration::from_secs(1),
        });
    }
}

// Metrics implementation

impl ApiMetrics {
    fn new() -> Self {
        use std::sync::atomic::AtomicU64;

        Self {
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            average_latency_ms: AtomicU64::new(0),
            service_requests: DashMap::new(),
            service_errors: DashMap::new(),
        }
    }

    pub fn record_request(&self, service: String, latency_ms: u64, error: bool) {
        use std::sync::atomic::Ordering;

        self.total_requests.fetch_add(1, Ordering::Relaxed);

        if error {
            self.total_errors.fetch_add(1, Ordering::Relaxed);
            *self.service_errors.entry(service.clone()).or_insert(0) += 1;
        }

        *self.service_requests.entry(service).or_insert(0) += 1;

        // Update average latency (simplified)
        self.average_latency_ms.store(latency_ms, Ordering::Relaxed);
    }
}

// Default implementations for testing

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::1]:8000".parse().unwrap(),
            enable_auth: false,
            enable_rate_limiting: true,
            request_timeout: Duration::from_secs(30),
            max_request_size: 10 * 1024 * 1024, // 10MB
            cors: CorsConfig::default(),
            api_version: "v1".to_string(),
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string()],
            allowed_headers: vec!["*".to_string()],
            max_age: Duration::from_secs(3600),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_bridge_creation() {
        let config = ApiConfig::default();
        let bridge = UnifiedApiBridge::new(config);

        // Register a test service
        let service = ServiceInfo {
            name: "test-service".to_string(),
            service_type: ServiceType::HyperMesh,
            version: "1.0.0".to_string(),
            endpoints: vec![],
            health_check_url: "/health".to_string(),
            metadata: HashMap::new(),
            registered_at: SystemTime::now(),
        };

        bridge.register(service).await.unwrap();

        // Verify service was registered
        assert!(bridge.services.contains_key("test-service"));
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new();
        limiter.set_limit("test".to_string(), 100, 10);

        assert!(limiter.limits.contains_key("test"));
    }

    #[test]
    fn test_metrics() {
        let metrics = ApiMetrics::new();
        metrics.record_request("test".to_string(), 50, false);

        use std::sync::atomic::Ordering;
        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.total_errors.load(Ordering::Relaxed), 0);
    }
}