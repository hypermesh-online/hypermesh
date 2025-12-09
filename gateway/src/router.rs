use anyhow::{anyhow, Result};
use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

use crate::config::{GatewayConfig, RetryConfig};
use crate::middleware::{CircuitBreaker, CorsMiddleware, LoggingMiddleware, RequestIdMiddleware};
use crate::pool::ConnectionPool;
use crate::proxy::{transform_backend_path, Http3Proxy};

/// Gateway router for routing requests to backend services
pub struct GatewayRouter {
    trustchain_pool: ConnectionPool,
    blockmatrix_pool: ConnectionPool,
    trustchain_proxy: Http3Proxy,
    blockmatrix_proxy: Http3Proxy,
    trustchain_addr: SocketAddr,
    blockmatrix_addr: SocketAddr,
    cors: CorsMiddleware,
    retry_config: RetryConfig,
    trustchain_breaker: Arc<CircuitBreaker>,
    blockmatrix_breaker: Arc<CircuitBreaker>,
}

impl GatewayRouter {
    pub async fn new(config: &GatewayConfig) -> Result<Self> {
        // Create connection pools
        let trustchain_pool = ConnectionPool::new(
            config.trustchain_addr,
            config.pool.max_connections,
            config.pool.idle_timeout,
        )
        .await?;

        let blockmatrix_pool = ConnectionPool::new(
            config.blockmatrix_addr,
            config.pool.max_connections,
            config.pool.idle_timeout,
        )
        .await?;

        // Create proxies
        let trustchain_proxy = Http3Proxy::new(
            trustchain_pool.clone(),
            config.pool.connect_timeout,
        );

        let blockmatrix_proxy = Http3Proxy::new(
            blockmatrix_pool.clone(),
            config.pool.connect_timeout,
        );

        // Create circuit breakers
        let trustchain_breaker = Arc::new(CircuitBreaker::new(
            5, // 5 failures before opening
            Duration::from_secs(30), // 30 second timeout
        ));

        let blockmatrix_breaker = Arc::new(CircuitBreaker::new(
            5,
            Duration::from_secs(30),
        ));

        Ok(Self {
            trustchain_pool,
            blockmatrix_pool,
            trustchain_proxy,
            blockmatrix_proxy,
            trustchain_addr: config.trustchain_addr,
            blockmatrix_addr: config.blockmatrix_addr,
            cors: CorsMiddleware::new(config.cors.clone()),
            retry_config: config.retry.clone(),
            trustchain_breaker,
            blockmatrix_breaker,
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
        RequestIdMiddleware::add_request_id(
            req.headers_mut(),
            logger.request_id(),
        );

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
                format!("{}?{}", transformed_path, q)
            } else {
                transformed_path.clone()
            };
            parts.path_and_query = Some(path_and_query.parse()?);
            *req.uri_mut() = http::Uri::from_parts(parts)?;

            // Forward request with retry
            match proxy.forward_with_retry(
                req,
                body,
                self.retry_config.max_attempts,
                self.retry_config.base_delay,
            ).await {
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
            Ok((&self.trustchain_proxy, &self.trustchain_breaker, "/api/v1/trustchain"))
        } else if path.starts_with("/api/v1/blockmatrix") {
            Ok((&self.blockmatrix_proxy, &self.blockmatrix_breaker, "/api/v1/blockmatrix"))
        } else if path.starts_with("/api/v1/hypermesh") {
            Ok((&self.blockmatrix_proxy, &self.blockmatrix_breaker, "/api/v1/hypermesh"))
        } else if path.starts_with("/api/v1/stoq") {
            Ok((&self.blockmatrix_proxy, &self.blockmatrix_breaker, "/api/v1/stoq"))
        } else if path.starts_with("/api/v1/caesar") {
            Ok((&self.blockmatrix_proxy, &self.blockmatrix_breaker, "/api/v1/caesar"))
        } else {
            Err(anyhow!("No backend found for path: {}", path))
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

        // Get pool statistics
        let trustchain_stats = self.trustchain_pool.stats();
        let blockmatrix_stats = self.blockmatrix_pool.stats();

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
            .unwrap()
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
            .unwrap()
    }
}