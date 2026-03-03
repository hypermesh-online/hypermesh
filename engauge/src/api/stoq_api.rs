// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Engauge STOQ API — analytics and metrics operations over STOQ protocol.
//!
//! Self-contained API server that speaks STOQ-compatible JSON over QUIC,
//! avoiding the stoq<->engauge cyclic dependency. Uses quinn directly.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::organic_detection::{TrafficClassifier, TrafficPattern};

// ---------------------------------------------------------------------------
// STOQ-compatible API types (local copies to avoid stoq<->engauge cycle)
// ---------------------------------------------------------------------------

/// API request (compatible with stoq::api::ApiRequest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub id: String,
    pub service: String,
    pub method: String,
    pub payload: Bytes,
    pub metadata: HashMap<String, String>,
}

/// API response (compatible with stoq::api::ApiResponse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub request_id: String,
    pub success: bool,
    pub payload: Bytes,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// API error types (compatible with stoq::api::ApiError)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiError {
    NotFound(String),
    InvalidRequest(String),
    HandlerError(String),
    SerializationError(String),
    TransportError(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(s) => write!(f, "not found: {s}"),
            Self::InvalidRequest(s) => write!(f, "invalid request: {s}"),
            Self::HandlerError(s) => write!(f, "handler error: {s}"),
            Self::SerializationError(s) => write!(f, "serialization error: {s}"),
            Self::TransportError(s) => write!(f, "transport error: {s}"),
        }
    }
}

impl std::error::Error for ApiError {}

/// API handler trait (compatible with stoq::api::ApiHandler)
#[async_trait]
pub trait ApiHandler: Send + Sync {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError>;
    fn path(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

/// Shared application state for Engauge STOQ API handlers.
pub struct EngaugeAppState {
    pub service_name: String,
    pub version: String,
    pub active_nodes: Arc<std::sync::atomic::AtomicU64>,
    pub total_receipts: Arc<std::sync::atomic::AtomicU64>,
    pub total_metrics: Arc<std::sync::atomic::AtomicU64>,
}

impl EngaugeAppState {
    pub fn new() -> Self {
        Self {
            service_name: "engauge".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            active_nodes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_receipts: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_metrics: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn set_active_nodes(&self, count: u64) {
        self.active_nodes
            .store(count, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn increment_receipts(&self) {
        self.total_receipts
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn increment_metrics(&self) {
        self.total_metrics
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Default for EngaugeAppState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EngaugeStoqConfig {
    pub bind_address: String,
    pub service_name: String,
    pub enable_logging: bool,
}

impl Default for EngaugeStoqConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::1]:9296".to_string(),
            service_name: "engauge".to_string(),
            enable_logging: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub active_nodes: u64,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub total_receipts: u64,
    pub total_metrics_collected: u64,
    pub active_nodes: u64,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityResponse {
    pub bytes_served: u64,
    pub compute_delivered: u64,
    pub storage_maintained_bytes: u64,
    pub bandwidth_available_bps: u64,
    pub uptime_ratio: f64,
    pub active_nodes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficRequest {
    pub resource_diversity: f64,
    pub counterparty_diversity: f64,
    pub geographic_spread: f64,
    pub velocity: f64,
    #[serde(default = "default_duration")]
    pub duration_secs: u64,
}

fn default_duration() -> u64 {
    60
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficResponse {
    pub classification: String,
    pub confidence: f64,
    pub organic_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmResponse {
    pub tracked_shards: u64,
    pub active_nodes: u64,
    pub total_receipts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingResponse {
    pub tracked_nodes: u64,
    pub active_nodes: u64,
    pub version: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub struct EngaugeHealthHandler {
    pub state: Arc<EngaugeAppState>,
    pub start_time: std::time::Instant,
}

#[async_trait]
impl ApiHandler for EngaugeHealthHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        let response = HealthResponse {
            status: "healthy".to_string(),
            service: self.state.service_name.clone(),
            version: self.state.version.clone(),
            active_nodes: self
                .state
                .active_nodes
                .load(std::sync::atomic::Ordering::Relaxed),
            uptime_secs: self.start_time.elapsed().as_secs(),
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
        "engauge/health"
    }
}

pub struct MetricsHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for MetricsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/metrics: {}", request.id);

        let response = MetricsResponse {
            total_receipts: self
                .state
                .total_receipts
                .load(std::sync::atomic::Ordering::Relaxed),
            total_metrics_collected: self
                .state
                .total_metrics
                .load(std::sync::atomic::Ordering::Relaxed),
            active_nodes: self
                .state
                .active_nodes
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
        "engauge/metrics"
    }
}

pub struct CapacityHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for CapacityHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/capacity: {}", request.id);

        let response = CapacityResponse {
            bytes_served: 0,
            compute_delivered: 0,
            storage_maintained_bytes: 0,
            bandwidth_available_bps: 0,
            uptime_ratio: 0.0,
            active_nodes: self
                .state
                .active_nodes
                .load(std::sync::atomic::Ordering::Relaxed),
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
        "engauge/capacity"
    }
}

pub struct TrafficHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for TrafficHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/traffic: {}", request.id);

        let req: TrafficRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid traffic request: {e}")))?;

        let classifier = TrafficClassifier::with_defaults();
        let pattern = TrafficPattern {
            resource_diversity: req.resource_diversity,
            counterparty_diversity: req.counterparty_diversity,
            geographic_spread: req.geographic_spread,
            velocity: req.velocity,
            avg_value: hypermesh_lib::GoldGrams::zero(),
            duration_secs: req.duration_secs,
        };

        let classification = classifier.classify(&pattern);
        let (class_name, confidence, organic_ratio) = match &classification {
            crate::organic_detection::TrafficClassification::Organic { confidence } => {
                ("organic".to_string(), *confidence, 1.0)
            }
            crate::organic_detection::TrafficClassification::Speculative { confidence } => {
                ("speculative".to_string(), *confidence, 0.0)
            }
            crate::organic_detection::TrafficClassification::Mixed {
                confidence,
                organic_ratio,
            } => ("mixed".to_string(), *confidence, *organic_ratio),
        };

        let response = TrafficResponse {
            classification: class_name,
            confidence,
            organic_ratio,
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
        "engauge/traffic"
    }
}

pub struct SwarmHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for SwarmHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/swarm: {}", request.id);

        let response = SwarmResponse {
            tracked_shards: 0,
            active_nodes: self
                .state
                .active_nodes
                .load(std::sync::atomic::Ordering::Relaxed),
            total_receipts: self
                .state
                .total_receipts
                .load(std::sync::atomic::Ordering::Relaxed),
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
        "engauge/swarm"
    }
}

pub struct TrendingHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for TrendingHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/trending: {}", request.id);

        let response = TrendingResponse {
            tracked_nodes: 0,
            active_nodes: self
                .state
                .active_nodes
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
        "engauge/trending"
    }
}

// ---------------------------------------------------------------------------
// Server (uses quinn directly to avoid stoq cycle)
// ---------------------------------------------------------------------------

/// Engauge STOQ-compatible API Server using quinn directly.
pub struct EngaugeStoqApi {
    handlers: HashMap<String, Arc<dyn ApiHandler>>,
    config: EngaugeStoqConfig,
}

impl EngaugeStoqApi {
    pub fn new(config: EngaugeStoqConfig, app_state: Arc<EngaugeAppState>) -> Self {
        let start_time = std::time::Instant::now();
        let mut handlers = HashMap::<String, Arc<dyn ApiHandler>>::new();

        let health: Arc<dyn ApiHandler> = Arc::new(EngaugeHealthHandler {
            state: app_state.clone(),
            start_time,
        });
        handlers.insert(health.path().to_string(), health);

        let metrics: Arc<dyn ApiHandler> = Arc::new(MetricsHandler {
            state: app_state.clone(),
        });
        handlers.insert(metrics.path().to_string(), metrics);

        let capacity: Arc<dyn ApiHandler> = Arc::new(CapacityHandler {
            state: app_state.clone(),
        });
        handlers.insert(capacity.path().to_string(), capacity);

        let traffic: Arc<dyn ApiHandler> = Arc::new(TrafficHandler {
            state: app_state.clone(),
        });
        handlers.insert(traffic.path().to_string(), traffic);

        let swarm: Arc<dyn ApiHandler> = Arc::new(SwarmHandler {
            state: app_state.clone(),
        });
        handlers.insert(swarm.path().to_string(), swarm);

        let trending: Arc<dyn ApiHandler> = Arc::new(TrendingHandler {
            state: app_state,
        });
        handlers.insert(trending.path().to_string(), trending);

        info!("Engauge STOQ API handlers registered ({} endpoints)", handlers.len());

        Self { handlers, config }
    }

    /// Start the QUIC/STOQ server using quinn directly.
    #[cfg(feature = "server")]
    pub async fn serve(&self) -> Result<()> {
        use std::net::SocketAddr;

        // Parse bind address
        let sock_addr: SocketAddr = if self.config.bind_address.starts_with('[') {
            let s = self.config.bind_address.trim_start_matches('[');
            let (addr_str, port_str) = s
                .split_once("]:")
                .ok_or_else(|| anyhow!("Invalid bind address: expected [addr]:port"))?;
            let addr: std::net::Ipv6Addr = addr_str
                .parse()
                .map_err(|e| anyhow!("Invalid IPv6 address '{}': {}", addr_str, e))?;
            let port: u16 = port_str
                .parse()
                .map_err(|e| anyhow!("Invalid port '{}': {}", port_str, e))?;
            SocketAddr::V6(std::net::SocketAddrV6::new(addr, port, 0, 0))
        } else {
            self.config
                .bind_address
                .parse()
                .map_err(|e| anyhow!("Invalid bind address '{}': {}", self.config.bind_address, e))?
        };

        // Generate self-signed cert for QUIC
        let cert = rcgen::generate_simple_self_signed(vec!["engauge".to_string()])
            .map_err(|e| anyhow!("Failed to generate self-signed cert: {e}"))?;
        let cert_der = rustls::pki_types::CertificateDer::from(cert.cert.der().clone());
        let key_der = rustls::pki_types::PrivateKeyDer::try_from(cert.key_pair.serialize_der())
            .map_err(|e| anyhow!("Failed to serialize private key: {e}"))?;

        let mut server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .map_err(|e| anyhow!("TLS config error: {e}"))?;

        server_config.alpn_protocols = vec![b"stoq".to_vec(), b"h3".to_vec()];

        let quic_config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(server_config)?,
        ));

        let endpoint = quinn::Endpoint::server(quic_config, sock_addr)?;
        info!("Engauge STOQ server listening on {}", sock_addr);

        while let Some(incoming) = endpoint.accept().await {
            let handlers = self.handlers.clone();
            tokio::spawn(async move {
                match incoming.await {
                    Ok(conn) => {
                        if let Err(e) = handle_connection(conn, handlers).await {
                            warn!("Connection error: {e}");
                        }
                    }
                    Err(e) => warn!("Accept error: {e}"),
                }
            });
        }

        Ok(())
    }
}

#[cfg(feature = "server")]
async fn handle_connection(
    conn: quinn::Connection,
    handlers: HashMap<String, Arc<dyn ApiHandler>>,
) -> Result<()> {
    loop {
        let stream = conn.accept_bi().await;
        match stream {
            Ok((send, recv)) => {
                let handlers = handlers.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(send, recv, handlers).await {
                        debug!("Stream error: {e}");
                    }
                });
            }
            Err(quinn::ConnectionError::ApplicationClosed(_)) => return Ok(()),
            Err(e) => return Err(anyhow!("Connection error: {e}")),
        }
    }
}

#[cfg(feature = "server")]
async fn handle_stream(
    mut send: quinn::SendStream,
    mut recv: quinn::RecvStream,
    handlers: HashMap<String, Arc<dyn ApiHandler>>,
) -> Result<()> {
    let data = recv.read_to_end(1024 * 1024).await?; // 1MB max
    let request: ApiRequest =
        serde_json::from_slice(&data).map_err(|e| anyhow!("Invalid request: {e}"))?;

    let path = format!("{}/{}", request.service, request.method);
    let response = if let Some(handler) = handlers.get(&path) {
        match handler.handle(request).await {
            Ok(resp) => resp,
            Err(e) => ApiResponse {
                request_id: String::new(),
                success: false,
                payload: Bytes::new(),
                error: Some(e.to_string()),
                metadata: HashMap::new(),
            },
        }
    } else {
        ApiResponse {
            request_id: String::new(),
            success: false,
            payload: Bytes::new(),
            error: Some(format!("Handler not found: {path}")),
            metadata: HashMap::new(),
        }
    };

    let response_data = serde_json::to_vec(&response)?;
    send.write_all(&response_data).await?;
    send.finish()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let resp = HealthResponse {
            status: "healthy".to_string(),
            service: "engauge".to_string(),
            version: "1.0.0".to_string(),
            active_nodes: 5,
            uptime_secs: 3600,
        };
        let json = serde_json::to_string(&resp).expect("test: serialization should succeed");
        assert!(json.contains("healthy"));
        assert!(json.contains("engauge"));
    }

    #[test]
    fn test_engauge_app_state() {
        let state = EngaugeAppState::new();
        assert_eq!(state.service_name, "engauge");

        state.set_active_nodes(42);
        assert_eq!(
            state
                .active_nodes
                .load(std::sync::atomic::Ordering::Relaxed),
            42
        );

        state.increment_receipts();
        state.increment_receipts();
        assert_eq!(
            state
                .total_receipts
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );
    }

    #[test]
    fn test_config_default() {
        let config = EngaugeStoqConfig::default();
        assert_eq!(config.bind_address, "[::1]:9296");
        assert_eq!(config.service_name, "engauge");
    }

    #[tokio::test]
    async fn test_health_handler() {
        let state = Arc::new(EngaugeAppState::new());
        state.set_active_nodes(7);

        let handler = EngaugeHealthHandler {
            state,
            start_time: std::time::Instant::now(),
        };

        let api_req = ApiRequest {
            id: "test-1".to_string(),
            service: "engauge".to_string(),
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
        assert_eq!(body.active_nodes, 7);
    }

    #[tokio::test]
    async fn test_traffic_handler() {
        let state = Arc::new(EngaugeAppState::new());
        let handler = TrafficHandler { state };

        let req_body = TrafficRequest {
            resource_diversity: 0.9,
            counterparty_diversity: 0.8,
            geographic_spread: 0.7,
            velocity: 5.0,
            duration_secs: 60,
        };

        let api_req = ApiRequest {
            id: "test-traffic-1".to_string(),
            service: "engauge".to_string(),
            method: "traffic".to_string(),
            payload: Bytes::from(
                serde_json::to_vec(&req_body).expect("test: serialize request"),
            ),
            metadata: HashMap::new(),
        };

        let resp = handler
            .handle(api_req)
            .await
            .expect("test: traffic handler should succeed");
        assert!(resp.success);

        let body: TrafficResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize response");
        assert!(body.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_traffic_handler_invalid_payload() {
        let state = Arc::new(EngaugeAppState::new());
        let handler = TrafficHandler { state };

        let api_req = ApiRequest {
            id: "test-bad-1".to_string(),
            service: "engauge".to_string(),
            method: "traffic".to_string(),
            payload: Bytes::from("not valid json"),
            metadata: HashMap::new(),
        };

        let result = handler.handle(api_req).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_handler_paths() {
        let state = Arc::new(EngaugeAppState::new());

        let health = EngaugeHealthHandler {
            state: state.clone(),
            start_time: std::time::Instant::now(),
        };
        assert_eq!(health.path(), "engauge/health");

        let metrics = MetricsHandler {
            state: state.clone(),
        };
        assert_eq!(metrics.path(), "engauge/metrics");

        let capacity = CapacityHandler {
            state: state.clone(),
        };
        assert_eq!(capacity.path(), "engauge/capacity");

        let traffic = TrafficHandler {
            state: state.clone(),
        };
        assert_eq!(traffic.path(), "engauge/traffic");

        let swarm = SwarmHandler {
            state: state.clone(),
        };
        assert_eq!(swarm.path(), "engauge/swarm");

        let trending = TrendingHandler { state };
        assert_eq!(trending.path(), "engauge/trending");
    }
}
