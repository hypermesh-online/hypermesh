//! TrustChain API Server
//! 
//! REST API server for TrustChain services with endpoints for CA, CT, and DNS
//! operations. Provides integration points for STOQ and HyperMesh systems.

use std::sync::Arc;
use std::time::SystemTime;
use std::net::{Ipv6Addr, SocketAddrV6};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, debug, warn, error};

use axum::{
    Router, Json, extract::{Query, Path, State},
    http::{StatusCode, HeaderMap, Method},
    response::{Response, Json as JsonResponse},
    middleware::{self, Next},
    routing::{get, post, put, delete},
};
use tower::ServiceBuilder;
use tower_http::{
    cors::{CorsLayer, Any, AllowMethods},
    trace::{TraceLayer, DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse},
};

use crate::config::ApiConfig;
use crate::consensus::{ConsensusProof, ConsensusContext};
use crate::ca::{CertificateRequest, IssuedCertificate};
use crate::ct::SignedCertificateTimestamp;
use crate::dns::{DnsQuery, DnsResponse};
use crate::errors::{ApiError, ErrorResponse, Result as TrustChainResult};

pub mod handlers;
pub mod middleware_auth;
pub mod rate_limiter;
pub mod validators;

pub use handlers::*;
pub use middleware_auth::*;
pub use rate_limiter::*;
pub use validators::*;

/// TrustChain API server
pub struct ApiServer {
    /// Server identifier
    server_id: String,
    /// Axum application
    app: Arc<Router>,
    /// Server address
    bind_address: Ipv6Addr,
    /// Server port
    port: u16,
    /// Configuration
    config: Arc<ApiConfig>,
    /// API statistics
    stats: Arc<RwLock<ApiStats>>,
    /// Rate limiter
    rate_limiter: Arc<RateLimiter>,
}

/// API server statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiStats {
    pub requests_total: u64,
    pub requests_successful: u64,
    pub requests_failed: u64,
    pub ca_requests: u64,
    pub ct_requests: u64,
    pub dns_requests: u64,
    pub average_response_time_ms: f64,
    pub active_connections: u64,
    pub rate_limited_requests: u64,
    pub last_update: SystemTime,
}

impl Default for ApiStats {
    fn default() -> Self {
        Self {
            requests_total: 0,
            requests_successful: 0,
            requests_failed: 0,
            ca_requests: 0,
            ct_requests: 0,
            dns_requests: 0,
            average_response_time_ms: 0.0,
            active_connections: 0,
            rate_limited_requests: 0,
            last_update: SystemTime::now(),
        }
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ApiConfig>,
    pub stats: Arc<RwLock<ApiStats>>,
    pub rate_limiter: Arc<RateLimiter>,
    pub start_time: SystemTime,
    /// Certificate Authority instance
    pub ca: Arc<crate::ca::TrustChainCA>,
    /// Certificate Transparency Log
    pub ct_log: Arc<crate::ct::CertificateTransparencyLog>,
    /// Certificate store
    pub certificate_store: Arc<crate::ca::CertificateStore>,
}

impl ApiServer {
    /// Create new API server
    pub async fn new(config: ApiConfig) -> TrustChainResult<Self> {
        info!("Initializing TrustChain API server: {}", config.server_id);

        // Initialize rate limiter
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit_per_minute).await?);

        // Initialize CA components
        let ca_config = crate::ca::CAConfig::default();
        let ca = Arc::new(crate::ca::TrustChainCA::new(ca_config).await?);
        let ct_log = Arc::new(crate::ct::CertificateTransparencyLog::new().await?);
        let certificate_store = Arc::new(crate::ca::CertificateStore::new().await?);

        // Create shared state
        let state = AppState {
            config: Arc::new(config.clone()),
            stats: Arc::new(RwLock::new(ApiStats::default())),
            rate_limiter: Arc::clone(&rate_limiter),
            start_time: SystemTime::now(),
            ca: ca.clone(),
            ct_log: ct_log.clone(),
            certificate_store: certificate_store.clone(),
        };

        // Build application with routes and middleware
        let app = Self::create_app(state.clone()).await?;

        let server = Self {
            server_id: config.server_id.clone(),
            app: Arc::new(app),
            bind_address: config.bind_address,
            port: config.port,
            config: Arc::new(config),
            stats: state.stats,
            rate_limiter,
        };

        info!("TrustChain API server initialized successfully");
        Ok(server)
    }

    /// Start API server
    pub async fn start(&self) -> TrustChainResult<()> {
        info!("Starting TrustChain API server on [{}]:{}", self.bind_address, self.port);

        let socket_addr = SocketAddrV6::new(self.bind_address, self.port, 0, 0);
        
        // Start server
        let listener = tokio::net::TcpListener::bind(socket_addr).await
            .map_err(|e| ApiError::ServerStartup {
                reason: format!("Failed to bind to address: {}", e),
            })?;

        info!("TrustChain API server listening on [{}]:{}", self.bind_address, self.port);

        axum::serve(listener, (*self.app).clone()).await
            .map_err(|e| ApiError::ServerStartup {
                reason: format!("Server error: {}", e),
            })?;

        Ok(())
    }

    /// Get API server statistics
    pub async fn get_stats(&self) -> ApiStats {
        self.stats.read().await.clone()
    }

    /// Shutdown API server
    pub async fn shutdown(&self) -> TrustChainResult<()> {
        info!("Shutting down TrustChain API server");
        // Axum doesn't have explicit shutdown, it shuts down when the future is dropped
        info!("TrustChain API server shut down successfully");
        Ok(())
    }

    // Internal helper methods

    async fn create_app(state: AppState) -> TrustChainResult<Router> {
        // Create CORS layer
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(AllowMethods::any())
            .allow_headers(Any);

        // Create trace layer
        let trace = TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::default())
            .on_request(DefaultOnRequest::default())
            .on_response(DefaultOnResponse::default());

        // Build router with routes
        let app = Router::new()
            // Health and status endpoints
            .route("/health", get(health_check))
            .route("/status", get(get_status))
            .route("/stats", get(get_stats))
            
            // Certificate Authority endpoints
            .route("/ca/certificate", post(issue_certificate))
            .route("/ca/certificate/:serial", get(get_certificate))
            .route("/ca/certificate/:serial/revoke", post(revoke_certificate))
            .route("/ca/root", get(get_ca_root))
            
            // Certificate Transparency endpoints
            .route("/ct/log", post(log_certificate_ct))
            .route("/ct/sct", post(get_sct))
            .route("/ct/proof/:fingerprint", get(get_inclusion_proof))
            .route("/ct/consistency", get(get_consistency_proof))
            .route("/ct/entries", get(get_ct_entries))
            .route("/ct/stats", get(get_ct_stats))
            
            // DNS endpoints
            .route("/dns/resolve", post(resolve_dns_query))
            .route("/dns/cache/clear", post(clear_dns_cache))
            .route("/dns/stats", get(get_dns_stats))
            
            // Integration endpoints (for STOQ/HyperMesh)
            .route("/integration/certificate/validate", post(validate_certificate_integration))
            .route("/integration/dns/bulk_resolve", post(bulk_resolve_dns))
            .route("/integration/consensus/validate", post(validate_consensus_proof))
            
            // Admin endpoints
            .route("/admin/config", get(get_config))
            .route("/admin/config", put(update_config))
            .route("/admin/maintenance", post(run_maintenance))
            .route("/admin/logs", get(get_logs))
            
            .with_state(state);
            // TODO: Re-add CORS and tracing layers when API compatibility is resolved
            // .layer(trace)
            // .layer(cors);

        Ok(app)
    }
}

/// API request/response types

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: SystemTime,
    pub version: String,
    pub services: ServiceHealth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub ca: bool,
    pub ct: bool,
    pub dns: bool,
    pub consensus: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub server_id: String,
    pub uptime_seconds: u64,
    pub stats: ApiStats,
    pub configuration: StatusConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusConfig {
    pub bind_address: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateIssueRequest {
    pub common_name: String,
    pub san_entries: Vec<String>,
    pub node_id: String,
    pub ipv6_addresses: Vec<Ipv6Addr>,
    pub consensus_proof: ConsensusProof,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateResponse {
    pub certificate: IssuedCertificate,
    pub sct: Option<SignedCertificateTimestamp>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CTLogRequest {
    pub certificate_der: String, // Base64 encoded
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CTProofRequest {
    pub fingerprint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsistencyProofQuery {
    pub old_size: u64,
    pub new_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CTEntriesQuery {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DnsResolveRequest {
    pub name: String,
    pub record_type: String, // "A", "AAAA", "CNAME", etc.
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkDnsResolveRequest {
    pub queries: Vec<DnsResolveRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkDnsResolveResponse {
    pub responses: Vec<DnsResponse>,
    pub failed_queries: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateValidationRequest {
    pub certificate_der: String, // Base64 encoded
    pub domain: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateValidationResponse {
    pub is_valid: bool,
    pub reason: Option<String>,
    pub ct_verified: bool,
    pub ca_verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusValidationRequest {
    pub consensus_proof: ConsensusProof,
    pub operation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusValidationResponse {
    pub is_valid: bool,
    pub validation_details: ConsensusValidationDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsensusValidationDetails {
    pub stake_valid: bool,
    pub time_valid: bool,
    pub space_valid: bool,
    pub work_valid: bool,
    pub overall_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaintenanceRequest {
    pub operations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaintenanceResponse {
    pub completed_operations: Vec<String>,
    pub failed_operations: Vec<String>,
    pub duration_seconds: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogsQuery {
    pub level: Option<String>,
    pub limit: Option<u32>,
    pub since: Option<SystemTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogsResponse {
    pub logs: Vec<LogEntry>,
    pub total_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub level: String,
    pub message: String,
    pub module: String,
}

// Middleware functions

/// Rate limiting middleware
async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract client IP (simplified - in production would be more sophisticated)
    let client_ip = "127.0.0.1"; // TODO: Extract real client IP
    
    if !state.rate_limiter.check_rate_limit(client_ip).await {
        // Update rate limit stats
        {
            let mut stats = state.stats.write().await;
            stats.rate_limited_requests += 1;
        }
        
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    Ok(next.run(req).await)
}

/// Request logging middleware
async fn request_logging_middleware(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = std::time::Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    
    let response = next.run(req).await;
    
    let response_time = start_time.elapsed().as_millis() as f64;
    let status_code = response.status();
    
    // Update statistics
    {
        let mut stats = state.stats.write().await;
        stats.requests_total += 1;
        
        if status_code.is_success() {
            stats.requests_successful += 1;
        } else {
            stats.requests_failed += 1;
        }
        
        // Update average response time
        if stats.average_response_time_ms == 0.0 {
            stats.average_response_time_ms = response_time;
        } else {
            stats.average_response_time_ms = 0.9 * stats.average_response_time_ms + 0.1 * response_time;
        }
        
        // Update endpoint-specific stats
        if path.starts_with("/ca/") {
            stats.ca_requests += 1;
        } else if path.starts_with("/ct/") {
            stats.ct_requests += 1;
        } else if path.starts_with("/dns/") {
            stats.dns_requests += 1;
        }
        
        stats.last_update = SystemTime::now();
    }
    
    debug!("{} {} - {} - {:.2}ms", method, path, status_code, response_time);
    
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;

    fn create_test_config() -> ApiConfig {
        ApiConfig {
            server_id: "test-api".to_string(),
            bind_address: Ipv6Addr::LOCALHOST,
            port: 0, // Random port
            enable_tls: false,
            rate_limit_per_minute: 60,
            max_body_size: 1024 * 1024,
            cors_origins: vec!["*".to_string()],
            consensus_requirements: crate::consensus::ConsensusRequirements::localhost_testing(),
        }
    }

    #[tokio::test]
    async fn test_api_server_creation() {
        let config = create_test_config();
        let server = ApiServer::new(config).await.unwrap();
        
        assert_eq!(server.server_id, "test-api");
        assert_eq!(server.bind_address, Ipv6Addr::LOCALHOST);
    }

    #[tokio::test]
    async fn test_api_stats_initialization() {
        let config = create_test_config();
        let server = ApiServer::new(config).await.unwrap();
        
        let stats = server.get_stats().await;
        assert_eq!(stats.requests_total, 0);
        assert_eq!(stats.requests_successful, 0);
        assert_eq!(stats.requests_failed, 0);
    }

    #[tokio::test]
    async fn test_app_state_creation() {
        let config = create_test_config();
        let rate_limiter = Arc::new(RateLimiter::new(60).await.unwrap());
        
        let state = AppState {
            config: Arc::new(config),
            stats: Arc::new(RwLock::new(ApiStats::default())),
            rate_limiter,
            start_time: SystemTime::now(),
        };
        
        assert_eq!(state.config.server_id, "test-api");
    }

    #[tokio::test]
    async fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: SystemTime::now(),
            version: "1.0.0".to_string(),
            services: ServiceHealth {
                ca: true,
                ct: true,
                dns: true,
                consensus: true,
            },
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("services"));
    }

    #[tokio::test]
    async fn test_certificate_request_deserialization() {
        let json = r#"{
            "common_name": "test.example.com",
            "san_entries": ["test.example.com", "alt.example.com"],
            "node_id": "node123",
            "ipv6_addresses": ["::1"],
            "consensus_proof": {
                "stake_proof": {
                    "stake_amount": 1000,
                    "validator_id": "test",
                    "stake_signature": [],
                    "stake_timestamp": 1234567890
                },
                "time_proof": {
                    "network_time": 1234567890,
                    "local_time": 1234567890,
                    "network_time_offset": 0,
                    "time_signature": []
                },
                "space_proof": {
                    "total_storage": 1000000,
                    "available_storage": 500000,
                    "storage_proof": [],
                    "storage_signature": []
                },
                "work_proof": {
                    "computational_power": 1000,
                    "proof_of_work": [],
                    "work_signature": []
                }
            }
        }"#;
        
        let request: Result<CertificateIssueRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        
        let request = request.unwrap();
        assert_eq!(request.common_name, "test.example.com");
        assert_eq!(request.san_entries.len(), 2);
    }
}