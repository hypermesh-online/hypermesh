# API Implementation Guide

## Quick Start

### Gateway Implementation

```rust
// /home/persist/repos/projects/web3/gateway/src/main.rs

use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, HeaderMap},
    response::Json,
    middleware,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize gateway on port 8443
    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1/hypermesh", hypermesh_routes())
        .nest("/api/v1/trustchain", trustchain_routes())
        .nest("/api/v1/stoq", stoq_routes())
        .nest("/api/v1/caesar", caesar_routes())
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn(request_logger))
        .layer(middleware::from_fn(inject_request_id))
        .with_state(AppState::new());

    let addr = SocketAddr::from(([::], 8443));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

### Response Format Helper

```rust
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
    pub request_id: String,
    pub timestamp: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn error(code: &str, message: &str, details: serde_json::Value) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: Some(details),
            }),
            request_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}
```

## Week 1: Core Endpoints Implementation

### 1. HyperMesh System Status

```rust
// /home/persist/repos/projects/web3/blockmatrix/src/api/handlers/system.rs

use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SystemStatusQuery {
    #[serde(default = "default_true")]
    include_matrix: bool,
    #[serde(default = "default_true")]
    include_blockchain: bool,
}

#[derive(Serialize)]
pub struct SystemStatusResponse {
    node_id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    matrix_position: Option<MatrixPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blockchain: Option<BlockchainState>,
    resources: ResourceMetrics,
    network: NetworkMetrics,
}

pub async fn get_system_status(
    Query(params): Query<SystemStatusQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SystemStatusResponse>>, StatusCode> {
    // Get HyperMesh system instance
    let system = &state.hypermesh_system;

    // Collect matrix position if requested
    let matrix_position = if params.include_matrix {
        let matrix = system.matrix_manager();
        Some(MatrixPosition {
            x: matrix.position.x,
            y: matrix.position.y,
            z: matrix.position.z,
            octant: matrix.calculate_octant(),
            neighbors: matrix.neighbor_count(),
        })
    } else {
        None
    };

    // Collect blockchain state if requested
    let blockchain = if params.include_blockchain {
        let blockchain = system.blockchain_manager();
        Some(BlockchainState {
            height: blockchain.height(),
            hash: blockchain.latest_hash(),
            pending_transactions: blockchain.pending_count(),
            consensus_state: blockchain.consensus_state(),
        })
    } else {
        None
    };

    // Collect resource metrics
    let resources = collect_resource_metrics(&system).await?;
    let network = collect_network_metrics(&system).await?;

    let response = SystemStatusResponse {
        node_id: system.node_id().to_string(),
        status: "operational".to_string(),
        matrix_position,
        blockchain,
        resources,
        network,
    };

    Ok(Json(ApiResponse::success(response)))
}
```

### 2. Asset Listing

```rust
// /home/persist/repos/projects/web3/blockmatrix/src/api/handlers/assets.rs

pub async fn list_assets(
    Query(params): Query<ListAssetsQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ListAssetsResponse>>, StatusCode> {
    let asset_manager = state.hypermesh_system.asset_manager();

    // Get all assets
    let mut assets = asset_manager.list_all_assets().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Apply filters
    if let Some(asset_type) = params.asset_type {
        assets.retain(|a| a.asset_type == asset_type);
    }

    if let Some(status) = params.status {
        assets.retain(|a| a.status == status);
    }

    // Pagination
    let total = assets.len();
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(100).min(1000);

    assets = assets.into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    // Transform to API format
    let api_assets = assets.into_iter()
        .map(|asset| AssetResponse::from_domain(asset))
        .collect();

    Ok(Json(ApiResponse::success(ListAssetsResponse {
        total,
        offset,
        limit,
        assets: api_assets,
    })))
}
```

### 3. STOQ Health Check

```rust
// /home/persist/repos/projects/web3/stoq/src/api/handlers/health.rs

pub async fn get_stoq_health(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<StoqHealthResponse>>, StatusCode> {
    let transport = &state.stoq_transport;

    // Collect metrics
    let connections = ConnectionMetrics {
        active: transport.active_connections(),
        idle: transport.idle_connections(),
        max_capacity: transport.max_connections(),
        total_established: transport.total_established(),
    };

    let performance = PerformanceMetrics {
        avg_latency_ms: transport.avg_latency_ms(),
        throughput_mbps: transport.throughput_mbps(),
        packet_loss_percent: transport.packet_loss_percent(),
    };

    // Get pool statistics
    let pools = collect_pool_stats(&transport).await?;

    let response = StoqHealthResponse {
        protocol_version: "1.0.0".to_string(),
        status: "healthy".to_string(),
        uptime_seconds: transport.uptime_seconds(),
        transport: TransportInfo {
            transport_type: "quic".to_string(),
            ipv6_enabled: true,
            port: 8446,
        },
        connections,
        performance,
        pools,
    };

    Ok(Json(ApiResponse::success(response)))
}
```

### 4. TrustChain Certificate Request

```rust
// /home/persist/repos/projects/web3/trustchain/src/api/handlers/auth.rs

pub async fn request_certificate(
    Json(request): Json<CertificateRequest>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<CertificateResponse>>, StatusCode> {
    // Validate public key
    let public_key = parse_public_key(&request.public_key)
        .map_err(|_| {
            Json(ApiResponse::error(
                "INVALID_PUBLIC_KEY",
                "The provided public key is not valid",
                json!({"error": "Failed to parse PEM encoding"})
            ))
        })?;

    // Validate subject
    validate_subject(&request.subject)?;

    // Generate certificate
    let cert_builder = CertificateBuilder::new()
        .subject(request.subject)
        .public_key(public_key)
        .validity_days(request.validity_days)
        .extensions(request.extensions);

    let certificate = state.trustchain
        .issue_certificate(cert_builder)
        .await
        .map_err(|e| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get certificate chain
    let chain = state.trustchain.get_certificate_chain().await?;

    let response = CertificateResponse {
        certificate: certificate.to_pem(),
        certificate_id: certificate.id(),
        serial_number: certificate.serial_number(),
        issuer: certificate.issuer(),
        subject: certificate.subject(),
        not_before: certificate.not_before(),
        not_after: certificate.not_after(),
        fingerprint: certificate.fingerprint(),
        chain: chain.to_pem_vec(),
    };

    Ok(Json(ApiResponse::success(response)))
}
```

## Validation Middleware

```rust
// /home/persist/repos/projects/web3/gateway/src/middleware/validation.rs

use axum::{
    extract::{Query, rejection::QueryRejection},
    http::StatusCode,
    response::Response,
};

pub async fn validate_pagination<T>(
    query: Result<Query<T>, QueryRejection>,
) -> Result<Query<T>, Response>
where
    T: HasPagination,
{
    let Query(mut params) = query.map_err(|_| {
        ApiResponse::error(
            "INVALID_QUERY_PARAMETERS",
            "Invalid query parameters provided",
            json!({})
        ).into_response()
    })?;

    // Validate limit
    if let Some(limit) = params.limit() {
        if limit < 1 || limit > 1000 {
            return Err(ApiResponse::error(
                "INVALID_LIMIT",
                "Limit must be between 1 and 1000",
                json!({"provided": limit, "valid_range": "1-1000"})
            ).into_response());
        }
    }

    // Validate offset
    if let Some(offset) = params.offset() {
        if offset < 0 {
            return Err(ApiResponse::error(
                "INVALID_OFFSET",
                "Offset must be >= 0",
                json!({"provided": offset})
            ).into_response());
        }
    }

    Ok(Query(params))
}
```

## Authentication Middleware

```rust
// /home/persist/repos/projects/web3/gateway/src/middleware/auth.rs

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn verify_certificate<B>(
    State(state): State<AppState>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Check for development mode
    if state.config.auth.development_mode {
        if request.headers().get("X-Dev-Mode").is_some() {
            return Ok(next.run(request).await);
        }
    }

    // Extract certificate from header
    let cert_header = request.headers()
        .get("X-Client-Certificate")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let cert_pem = cert_header.to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Validate certificate with TrustChain
    let cert = state.trustchain
        .validate_certificate(cert_pem)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add user context to request extensions
    request.extensions_mut().insert(UserContext {
        user_id: cert.subject_id(),
        roles: cert.roles(),
        permissions: cert.permissions(),
    });

    Ok(next.run(request).await)
}
```

## Error Handling

```rust
// /home/persist/repos/projects/web3/gateway/src/error.rs

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Insufficient resources")]
    InsufficientResources,

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                "RESOURCE_NOT_FOUND",
                msg,
            ),
            ApiError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "INVALID_INPUT",
                msg,
            ),
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Authentication required".to_string(),
            ),
            ApiError::InsufficientResources => (
                StatusCode::PAYMENT_REQUIRED,
                "INSUFFICIENT_RESOURCES",
                "Not enough resources available".to_string(),
            ),
            ApiError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "An internal error occurred".to_string(),
            ),
        };

        let response = ApiResponse::<()>::error(
            code,
            &message,
            json!({}),
        );

        (status, Json(response)).into_response()
    }
}
```

## Testing Framework

```rust
// /home/persist/repos/projects/web3/tests/api_integration.rs

use axum_test::TestServer;

#[tokio::test]
async fn test_system_status() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/api/v1/hypermesh/system/status")
        .await;

    assert_eq!(response.status_code(), 200);

    let json: ApiResponse<SystemStatusResponse> = response.json();
    assert!(json.success);
    assert!(json.data.is_some());

    let data = json.data.unwrap();
    assert_eq!(data.status, "operational");
}

#[tokio::test]
async fn test_asset_pagination() {
    let server = create_test_server().await;

    // Test pagination
    let response = server
        .get("/api/v1/hypermesh/assets?limit=10&offset=0")
        .await;

    assert_eq!(response.status_code(), 200);

    let json: ApiResponse<ListAssetsResponse> = response.json();
    assert!(json.data.unwrap().assets.len() <= 10);
}

#[tokio::test]
async fn test_invalid_asset_type() {
    let server = create_test_server().await;

    let response = server
        .get("/api/v1/hypermesh/assets?asset_type=invalid")
        .await;

    assert_eq!(response.status_code(), 400);

    let json: ApiResponse<()> = response.json();
    assert!(!json.success);
    assert_eq!(json.error.unwrap().code, "INVALID_ASSET_TYPE");
}
```

## Performance Testing

```rust
// /home/persist/repos/projects/web3/benches/api_performance.rs

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_system_status(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let server = runtime.block_on(create_test_server());

    c.bench_function("system_status", |b| {
        b.to_async(&runtime).iter(|| async {
            server.get("/api/v1/hypermesh/system/status").await
        });
    });
}

fn bench_asset_listing(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let server = runtime.block_on(create_test_server());

    let mut group = c.benchmark_group("asset_listing");

    for limit in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(limit),
            limit,
            |b, &limit| {
                b.to_async(&runtime).iter(|| async {
                    server.get(&format!("/api/v1/hypermesh/assets?limit={}", limit)).await
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_system_status, bench_asset_listing);
criterion_main!(benches);
```

## Deployment Configuration

```yaml
# docker-compose.yml
version: '3.8'

services:
  gateway:
    build: ./gateway
    ports:
      - "[::]:8443:8443"
    environment:
      - RUST_LOG=info
      - AUTH_MODE=development
    depends_on:
      - hypermesh
      - trustchain
      - stoq
      - caesar

  hypermesh:
    build: ./blockmatrix
    ports:
      - "[::1]:8446:8446"
    environment:
      - RUST_LOG=debug
      - NODE_ID=node-001

  trustchain:
    build: ./trustchain
    ports:
      - "[::1]:50053:50053"
    environment:
      - RUST_LOG=info

  stoq:
    build: ./stoq
    environment:
      - RUST_LOG=info
      - ADAPTIVE_POOLS=true

  caesar:
    build: ./caesar
    environment:
      - RUST_LOG=info
```

## Monitoring Setup

```rust
// /home/persist/repos/projects/web3/gateway/src/metrics.rs

use prometheus::{
    register_histogram_vec, register_int_counter_vec,
    HistogramVec, IntCounterVec,
};

lazy_static! {
    static ref REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "api_request_duration_seconds",
        "API request duration in seconds",
        &["endpoint", "method", "status"]
    ).unwrap();

    static ref REQUEST_COUNT: IntCounterVec = register_int_counter_vec!(
        "api_request_total",
        "Total API requests",
        &["endpoint", "method", "status"]
    ).unwrap();
}

pub async fn metrics_middleware<B>(
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string();
    let method = request.method().to_string();
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status().as_u16().to_string();

    REQUEST_DURATION
        .with_label_values(&[&path, &method, &status])
        .observe(duration.as_secs_f64());

    REQUEST_COUNT
        .with_label_values(&[&path, &method, &status])
        .inc();

    Ok(response)
}
```

## Development Commands

```bash
# Start development gateway
cargo run --bin gateway -- --dev-mode

# Run API tests
cargo test --package api-tests

# Run performance benchmarks
cargo bench --package api-benchmarks

# Generate OpenAPI spec
cargo run --bin openapi-gen > openapi.yaml

# Start with hot reload
cargo watch -x 'run --bin gateway'

# Check API compliance
cargo run --bin api-validator -- --spec ./API_SPECIFICATIONS.md
```

## Common Patterns

### Async Handler Pattern
```rust
pub async fn handler_name(
    Query(params): Query<QueryParams>,
    Json(body): Json<RequestBody>,
    State(state): State<AppState>,
    Extension(user): Extension<UserContext>,
) -> Result<Json<ApiResponse<ResponseType>>, ApiError> {
    // 1. Validate input
    validate_input(&params, &body)?;

    // 2. Check permissions
    check_permissions(&user, "required_permission")?;

    // 3. Business logic
    let result = perform_operation(&state, params, body).await?;

    // 4. Return response
    Ok(Json(ApiResponse::success(result)))
}
```

### Caching Pattern
```rust
use moka::future::Cache;

pub struct CachedEndpoint {
    cache: Cache<String, Bytes>,
}

impl CachedEndpoint {
    pub async fn get_cached_or_fetch(&self, key: &str) -> Result<Bytes> {
        self.cache
            .get_with(key.to_string(), async move {
                // Fetch fresh data
                fetch_data(key).await
            })
            .await
    }
}
```

### Rate Limiting Pattern
```rust
use governor::{Quota, RateLimiter};

pub struct RateLimitedEndpoint {
    limiter: RateLimiter<String, DefaultKeyedStateStore<String>>,
}

pub async fn rate_limited_handler(
    State(limiter): State<Arc<RateLimiter>>,
    Extension(user): Extension<UserContext>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    limiter.check_key(&user.user_id)
        .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;

    // Process request
    Ok(Json(ApiResponse::success(())))
}
```

## Troubleshooting

### Common Issues

1. **Port Already in Use**
   ```bash
   # Find process using port 8443
   lsof -i :8443
   # Kill process
   kill -9 <PID>
   ```

2. **Certificate Validation Fails**
   ```bash
   # Enable development mode
   export AUTH_MODE=development
   ```

3. **CORS Issues**
   - Ensure CorsLayer is added to router
   - Check allowed origins configuration

4. **Performance Issues**
   - Enable connection pooling
   - Increase worker threads
   - Add caching layer

5. **Memory Leaks**
   - Use Arc for shared state
   - Avoid circular references
   - Monitor with valgrind

## Next Steps

1. **Week 1 Priority**:
   - Implement gateway with health check
   - Add P0 endpoints (system status, assets, health checks)
   - Set up authentication middleware
   - Add P1 endpoints

2. **Week 2 Priority**:
   - Implement P2 endpoints
   - Add comprehensive error handling
   - Performance optimization
   - Load testing

3. **Production Readiness**:
   - Add monitoring/metrics
   - Implement rate limiting
   - Set up CI/CD pipeline
   - Security audit

---

**Document Version**: 1.0.0
**Last Updated**: 2025-12-09