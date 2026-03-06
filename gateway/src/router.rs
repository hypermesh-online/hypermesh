// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::{anyhow, Result};
use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

use crate::config::{GatewayConfig, RetryConfig};
use crate::middleware::{CircuitBreaker, CorsMiddleware, LoggingMiddleware, RequestIdMiddleware};
use crate::pool::ConnectionPool;
use crate::proxy::{transform_backend_path, Http3Proxy};

use crate::dashboard_server::{DashboardScope, DashboardServer};
use crate::onboarding::{GATEWAY_ADMIN_HTML, GATEWAY_PRIVATE_HTML, GATEWAY_PUBLIC_HTML};

/// Gateway router for routing requests to backend services
pub struct GatewayRouter {
    trustchain_pool: ConnectionPool,
    blockmatrix_pool: ConnectionPool,
    caesar_pool: ConnectionPool,
    catalog_pool: ConnectionPool,
    engauge_pool: ConnectionPool,
    trustchain_proxy: Http3Proxy,
    blockmatrix_proxy: Http3Proxy,
    caesar_proxy: Http3Proxy,
    catalog_proxy: Http3Proxy,
    engauge_proxy: Http3Proxy,
    /// TrustChain service address (retained for health check routing)
    _trustchain_addr: SocketAddr,
    /// BlockMatrix service address (retained for health check routing)
    _blockmatrix_addr: SocketAddr,
    /// Caesar service address (retained for health check routing)
    _caesar_addr: SocketAddr,
    /// Catalog service address (retained for health check routing)
    _catalog_addr: SocketAddr,
    /// engauge service address (retained for health check routing)
    _engauge_addr: SocketAddr,
    dashboard: DashboardServer,
    cors: CorsMiddleware,
    retry_config: RetryConfig,
    trustchain_breaker: Arc<CircuitBreaker>,
    blockmatrix_breaker: Arc<CircuitBreaker>,
    caesar_breaker: Arc<CircuitBreaker>,
    catalog_breaker: Arc<CircuitBreaker>,
    engauge_breaker: Arc<CircuitBreaker>,
}

impl GatewayRouter {
    pub async fn new(config: &GatewayConfig) -> Result<Self> {
        // Create connection pools
        let trustchain_pool = ConnectionPool::new(
            config.trustchain_addr,
            &config.trustchain_server_name,
            config.pool.max_connections,
            config.pool.idle_timeout,
        )
        .await?;

        let blockmatrix_pool = ConnectionPool::new(
            config.blockmatrix_addr,
            &config.blockmatrix_server_name,
            config.pool.max_connections,
            config.pool.idle_timeout,
        )
        .await?;

        let caesar_pool = ConnectionPool::new(
            config.caesar_addr,
            &config.caesar_server_name,
            config.pool.max_connections,
            config.pool.idle_timeout,
        )
        .await?;

        let catalog_pool = ConnectionPool::new(
            config.catalog_addr,
            &config.catalog_server_name,
            config.pool.max_connections,
            config.pool.idle_timeout,
        )
        .await?;

        let engauge_pool = ConnectionPool::new(
            config.engauge_addr,
            &config.engauge_server_name,
            config.pool.max_connections,
            config.pool.idle_timeout,
        )
        .await?;

        // Create proxies
        let trustchain_proxy =
            Http3Proxy::new(trustchain_pool.clone(), config.pool.connect_timeout);

        let blockmatrix_proxy =
            Http3Proxy::new(blockmatrix_pool.clone(), config.pool.connect_timeout);

        let caesar_proxy = Http3Proxy::new(caesar_pool.clone(), config.pool.connect_timeout);

        let catalog_proxy = Http3Proxy::new(catalog_pool.clone(), config.pool.connect_timeout);

        let engauge_proxy = Http3Proxy::new(engauge_pool.clone(), config.pool.connect_timeout);

        // Create circuit breakers
        let trustchain_breaker = Arc::new(CircuitBreaker::new(
            5,                       // 5 failures before opening
            Duration::from_secs(30), // 30 second timeout
        ));

        let blockmatrix_breaker = Arc::new(CircuitBreaker::new(5, Duration::from_secs(30)));

        let caesar_breaker = Arc::new(CircuitBreaker::new(5, Duration::from_secs(30)));

        let catalog_breaker = Arc::new(CircuitBreaker::new(5, Duration::from_secs(30)));

        let engauge_breaker = Arc::new(CircuitBreaker::new(5, Duration::from_secs(30)));

        // Initialize dashboard server with onboarding HTML
        let dashboard = DashboardServer::new(Duration::from_secs(3600));
        dashboard
            .load_defaults(
                "trust.hypermesh.online",
                "gateway-node",
                GATEWAY_PUBLIC_HTML,
                GATEWAY_PRIVATE_HTML,
                GATEWAY_ADMIN_HTML,
            )
            .await;

        Ok(Self {
            trustchain_pool,
            blockmatrix_pool,
            caesar_pool,
            catalog_pool,
            engauge_pool,
            trustchain_proxy,
            blockmatrix_proxy,
            caesar_proxy,
            catalog_proxy,
            engauge_proxy,
            _trustchain_addr: config.trustchain_addr,
            _blockmatrix_addr: config.blockmatrix_addr,
            _caesar_addr: config.caesar_addr,
            _catalog_addr: config.catalog_addr,
            _engauge_addr: config.engauge_addr,
            dashboard,
            cors: CorsMiddleware::new(config.cors.clone()),
            retry_config: config.retry.clone(),
            trustchain_breaker,
            blockmatrix_breaker,
            caesar_breaker,
            catalog_breaker,
            engauge_breaker,
        })
    }

    /// Route incoming request to appropriate backend
    pub async fn route(
        &self,
        mut req: Request<()>,
        body: Option<Bytes>,
    ) -> Result<Response<Bytes>> {
        // Create logging middleware
        let logger = LoggingMiddleware::new();

        // Log request before any mutations
        logger.log_request(&req);

        // Add request ID if not present
        RequestIdMiddleware::add_request_id(req.headers_mut(), logger.request_id());

        // Handle CORS preflight
        if req.method() == Method::OPTIONS {
            let response = self.cors.handle_preflight();
            logger.log_response(&response);
            return Ok(response);
        }

        // Extract path before any mutations
        let path = req.uri().path().to_string();

        let mut response = if path == "/health" {
            self.handle_health_check().await?
        } else if path == "/" || path == "/index.html" || path.starts_with("/dashboard") {
            // Serve dashboard content for root and dashboard paths
            let scope = DashboardScope::Public;
            match self
                .dashboard
                .serve("trust.hypermesh.online", &path, scope)
                .await
            {
                Some(served) => Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", &served.content_type)
                    .body(Bytes::from(served.content))
                    .map_err(|e| anyhow!("failed to build dashboard response: {e}"))?,
                None => {
                    let body = json!({"error": "Not found", "message": "Dashboard content not available"});
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header("content-type", "application/json")
                        .body(Bytes::from(serde_json::to_vec(&body).unwrap_or_default()))
                        .map_err(|e| anyhow!("failed to build 404 response: {e}"))?
                }
            }
        } else {
            let (proxy, breaker, backend_prefix) = self.select_backend(&path)?;

            // Check circuit breaker
            if breaker.is_open() {
                return Ok(self.service_unavailable_response());
            }

            // Transform path for backend
            let transformed_path = transform_backend_path(&path, backend_prefix);

            // Extract query before mutating URI
            let query = req.uri().query().map(|q| q.to_string());

            // Update request path
            let mut parts = req.uri().clone().into_parts();
            let path_and_query = if let Some(q) = query {
                format!("{transformed_path}?{q}")
            } else {
                transformed_path.clone()
            };
            parts.path_and_query = Some(path_and_query.parse()?);
            *req.uri_mut() = http::Uri::from_parts(parts)?;

            // Forward request with retry
            match proxy
                .forward_with_retry(
                    req,
                    body,
                    self.retry_config.max_attempts,
                    self.retry_config.base_delay,
                )
                .await
            {
                Ok(resp) => {
                    breaker.record_success();
                    resp
                }
                Err(e) => {
                    breaker.record_failure();
                    error!("Failed to forward request: {}", e);
                    self.gateway_error_response(e)
                }
            }
        };

        // Apply CORS headers
        self.cors.apply_cors(&mut response);

        // Log response
        logger.log_response(&response);

        Ok(response)
    }

    /// Select backend based on request path
    fn select_backend(&self, path: &str) -> Result<(&Http3Proxy, &Arc<CircuitBreaker>, &str)> {
        if path.starts_with("/api/v1/trustchain") {
            Ok((
                &self.trustchain_proxy,
                &self.trustchain_breaker,
                "/api/v1/trustchain",
            ))
        } else if path.starts_with("/api/v1/blockmatrix") {
            Ok((
                &self.blockmatrix_proxy,
                &self.blockmatrix_breaker,
                "/api/v1/blockmatrix",
            ))
        } else if path.starts_with("/api/v1/hypermesh") {
            Ok((
                &self.blockmatrix_proxy,
                &self.blockmatrix_breaker,
                "/api/v1/hypermesh",
            ))
        } else if path.starts_with("/api/v1/stoq") {
            Ok((
                &self.blockmatrix_proxy,
                &self.blockmatrix_breaker,
                "/api/v1/stoq",
            ))
        } else if path.starts_with("/api/v1/caesar") {
            Ok((&self.caesar_proxy, &self.caesar_breaker, "/api/v1/caesar"))
        } else if path.starts_with("/api/v1/catalog") {
            Ok((
                &self.catalog_proxy,
                &self.catalog_breaker,
                "/api/v1/catalog",
            ))
        } else if path.starts_with("/api/v1/engauge") {
            Ok((
                &self.engauge_proxy,
                &self.engauge_breaker,
                "/api/v1/engauge",
            ))
        } else {
            Err(anyhow!("No backend found for path: {path}"))
        }
    }

    /// Handle health check endpoint
    async fn handle_health_check(&self) -> Result<Response<Bytes>> {
        let mut backends = serde_json::Map::new();

        // Check TrustChain health
        let trustchain_status = match self.trustchain_pool.health_check().await {
            Ok(latency) => {
                json!({
                    "status": "up",
                    "latency_ms": latency.as_millis()
                })
            }
            Err(_) => {
                json!({
                    "status": "down",
                    "latency_ms": null
                })
            }
        };
        backends.insert("trustchain".to_string(), trustchain_status);

        // Check BlockMatrix health
        let blockmatrix_status = match self.blockmatrix_pool.health_check().await {
            Ok(latency) => {
                json!({
                    "status": "up",
                    "latency_ms": latency.as_millis()
                })
            }
            Err(_) => {
                json!({
                    "status": "down",
                    "latency_ms": null
                })
            }
        };
        backends.insert("blockmatrix".to_string(), blockmatrix_status);

        // Check Caesar health
        let caesar_status = match self.caesar_pool.health_check().await {
            Ok(latency) => {
                json!({
                    "status": "up",
                    "latency_ms": latency.as_millis()
                })
            }
            Err(_) => {
                json!({
                    "status": "down",
                    "latency_ms": null
                })
            }
        };
        backends.insert("caesar".to_string(), caesar_status);

        // Check Catalog health
        let catalog_status = match self.catalog_pool.health_check().await {
            Ok(latency) => {
                json!({
                    "status": "up",
                    "latency_ms": latency.as_millis()
                })
            }
            Err(_) => {
                json!({
                    "status": "down",
                    "latency_ms": null
                })
            }
        };
        backends.insert("catalog".to_string(), catalog_status);

        // Check engauge health
        let engauge_status = match self.engauge_pool.health_check().await {
            Ok(latency) => {
                json!({
                    "status": "up",
                    "latency_ms": latency.as_millis()
                })
            }
            Err(_) => {
                json!({
                    "status": "down",
                    "latency_ms": null
                })
            }
        };
        backends.insert("engauge".to_string(), engauge_status);

        // Get pool statistics
        let trustchain_stats = self.trustchain_pool.stats();
        let blockmatrix_stats = self.blockmatrix_pool.stats();
        let caesar_stats = self.caesar_pool.stats();
        let catalog_stats = self.catalog_pool.stats();
        let engauge_stats = self.engauge_pool.stats();

        let response_body = json!({
            "status": "healthy",
            "backends": backends,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_seconds": 0, // Would need to track from startup
            "statistics": {
                "trustchain": {
                    "total_connections": trustchain_stats.total_connections,
                    "active_connections": trustchain_stats.active_connections,
                    "requests_served": trustchain_stats.requests_served,
                },
                "blockmatrix": {
                    "total_connections": blockmatrix_stats.total_connections,
                    "active_connections": blockmatrix_stats.active_connections,
                    "requests_served": blockmatrix_stats.requests_served,
                },
                "caesar": {
                    "total_connections": caesar_stats.total_connections,
                    "active_connections": caesar_stats.active_connections,
                    "requests_served": caesar_stats.requests_served,
                },
                "catalog": {
                    "total_connections": catalog_stats.total_connections,
                    "active_connections": catalog_stats.active_connections,
                    "requests_served": catalog_stats.requests_served,
                },
                "engauge": {
                    "total_connections": engauge_stats.total_connections,
                    "active_connections": engauge_stats.active_connections,
                    "requests_served": engauge_stats.requests_served,
                }
            }
        });

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Bytes::from(serde_json::to_vec(&response_body)?))?)
    }

    /// Create service unavailable response
    fn service_unavailable_response(&self) -> Response<Bytes> {
        let body = json!({
            "error": "Service temporarily unavailable",
            "message": "The backend service is experiencing issues. Please try again later."
        });

        Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("content-type", "application/json")
            .body(Bytes::from(serde_json::to_vec(&body).unwrap_or_default()))
            .expect("response builder with valid status and headers should not fail")
    }

    /// Create gateway error response
    fn gateway_error_response(&self, error: anyhow::Error) -> Response<Bytes> {
        let body = json!({
            "error": "Gateway error",
            "message": error.to_string()
        });

        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header("content-type", "application/json")
            .body(Bytes::from(serde_json::to_vec(&body).unwrap_or_default()))
            .expect("response builder with valid status and headers should not fail")
    }
}
