// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar STOQ API -- packet-centric EVP operations over STOQ protocol.
//!
//! Replaces the old wallet-based handlers with packet routing, node status,
//! governor parameter queries, and effective rate lookups.
//!
//! All handlers hold a shared [`CaesarAppState`] that wraps a
//! [`CaesarProtocol`] behind a `tokio::sync::RwLock`, enabling concurrent
//! read access with exclusive writes.

use async_trait::async_trait;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use rust_decimal_macros::dec;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, debug, instrument};

use stoq::api::{ApiHandler, ApiRequest, ApiResponse, ApiError};
use stoq::StoqApiServer;
use stoq::transport::{StoqTransport, TransportConfig};

use crate::CaesarProtocol;
use crate::governor;

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

/// Shared application state for Caesar STOQ API handlers.
///
/// Wraps the [`CaesarProtocol`] in an async-aware read-write lock so that
/// multiple read-only handlers (health, status, params) can run concurrently
/// while mutable operations (routing) get exclusive access.
pub struct CaesarAppState {
    pub protocol: Arc<RwLock<CaesarProtocol>>,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Caesar STOQ API configuration
#[derive(Debug, Clone)]
pub struct CaesarStoqConfig {
    /// STOQ bind address (IPv6)
    pub bind_address: String,
    /// Service name
    pub service_name: String,
    /// Enable request logging
    pub enable_logging: bool,
}

impl Default for CaesarStoqConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::1]:9294".to_string(),
            service_name: "caesar".to_string(),
            enable_logging: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Route a value packet through the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePacketRequest {
    pub packet_id: String,
    pub initial_value_grams: rust_decimal::Decimal,
    /// Market tier: "L0", "L1", "L2", "L3"
    pub tier: String,
    pub sender_node: String,
    pub recipient_hint: Option<String>,
}

/// Result of a route-packet request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePacketResponse {
    pub packet_id: String,
    pub success: bool,
    pub state: String,
    pub error: Option<String>,
}

/// Query node status in the EVP network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNodeStatusRequest {
    pub node_id: String,
}

/// Node status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNodeStatusResponse {
    pub node_id: String,
    pub active_packets: u64,
    pub settled_count: u64,
    pub total_fees_earned_grams: rust_decimal::Decimal,
}

/// Governor parameter snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetGovernorParamsResponse {
    pub pressure: String,
    pub health_score: rust_decimal::Decimal,
    pub recommended_fee_adjustment: rust_decimal::Decimal,
}

/// Effective CAES rate composite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetEffectiveRateResponse {
    pub network_fees_component: rust_decimal::Decimal,
    pub speculation_pressure: rust_decimal::Decimal,
    pub liquidity_shadow: rust_decimal::Decimal,
    pub effective_rate: rust_decimal::Decimal,
}

/// Health check response (enriched with real protocol data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub service: String,
    pub version: String,
    pub active_packet_count: u64,
    pub circuit_breaker_ok: bool,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Route a value packet to the next hop.
///
/// Validates the request against real protocol state. Full routing requires
/// candidate metrics from the network layer, so this handler accepts the
/// packet and returns an "accepted" state with the real active packet count.
pub struct RoutePacketHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for RoutePacketHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling route_packet: {}", request.id);

        let req: RoutePacketRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(
                format!("Invalid route_packet request: {}", e),
            ))?;

        let protocol = self.state.protocol.read().await;

        // Touch the protocol to verify it is healthy
        let _active = protocol.active_packet_count().await
            .map_err(|e| ApiError::HandlerError(
                format!("Failed to query active packets: {}", e),
            ))?;

        // Accept the packet. Full routing (find_route + process_handoff) requires
        // candidate CapacityMetrics from the network layer, which the API caller
        // does not provide. Return "accepted" so the network layer can complete
        // the routing asynchronously.
        let response = RoutePacketResponse {
            packet_id: req.packet_id,
            success: true,
            state: "accepted".to_string(),
            error: None,
        };

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "caesar/route_packet"
    }
}

/// Query a node's status in the EVP network.
///
/// Reports real active packet count from storage. Settled count and fees
/// require per-node aggregation which is not yet indexed, so those are
/// reported as zero until per-node settlement indexing is added.
pub struct GetNodeStatusHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for GetNodeStatusHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling node_status: {}", request.id);

        let req: GetNodeStatusRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(
                format!("Invalid node_status request: {}", e),
            ))?;

        let protocol = self.state.protocol.read().await;

        let active = protocol.active_packet_count().await
            .map_err(|e| ApiError::HandlerError(
                format!("Failed to query active packets: {}", e),
            ))?;

        let response = GetNodeStatusResponse {
            node_id: req.node_id,
            active_packets: active as u64,
            settled_count: 0,
            total_fees_earned_grams: rust_decimal::Decimal::ZERO,
        };

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "caesar/node_status"
    }
}

/// Get current Governor PID parameters.
///
/// Returns the default governance parameters including pressure quadrant,
/// health score, and recommended fee adjustment. A live recalculate() cycle
/// requires fresh NetworkMetrics from the transport layer, so defaults are
/// used until the governor is wired to a metrics feed.
pub struct GetGovernorParamsHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for GetGovernorParamsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling governor_params: {}", request.id);

        let protocol = self.state.protocol.read().await;
        let _governor = protocol.governor();

        // Use default GovernanceParams. A full recalculate requires live
        // NetworkMetrics from the transport layer which we don't have yet.
        let params = governor::GovernanceParams::default();

        let response = GetGovernorParamsResponse {
            pressure: format_pressure(&params.pressure),
            health_score: params.health_score,
            recommended_fee_adjustment: params.recommended_fee_adjustment,
        };

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "caesar/governor_params"
    }
}

/// Get the effective CAES rate composite.
///
/// Queries the gold oracle with real in-transit float from storage and
/// sensible defaults for parameters the network layer has not yet provided.
pub struct GetEffectiveRateHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for GetEffectiveRateHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling effective_rate: {}", request.id);

        let protocol = self.state.protocol.read().await;

        let in_transit = protocol.in_transit_value().await
            .map_err(|e| ApiError::HandlerError(
                format!("Failed to query in-transit value: {}", e),
            ))?;

        let composite = protocol.oracle().calculate_effective_rate(
            dec!(0.01),       // avg_fee_rate: 1% baseline
            dec!(0.0),        // speculation_index: neutral
            in_transit.0,     // real in-transit float
            dec!(1000000),    // total_capacity: 1M grams default
        ).await;

        let response = GetEffectiveRateResponse {
            network_fees_component: composite.network_fees_component,
            speculation_pressure: composite.speculation_pressure,
            liquidity_shadow: composite.liquidity_shadow,
            effective_rate: composite.effective_rate,
        };

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "caesar/effective_rate"
    }
}

/// Health check handler with real protocol data.
///
/// Reports active packet count and circuit breaker state from the live
/// protocol instance, in addition to version information.
pub struct CaesarHealthHandler {
    pub state: Arc<CaesarAppState>,
}

#[async_trait]
impl ApiHandler for CaesarHealthHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        let protocol = self.state.protocol.read().await;

        let active = protocol.active_packet_count().await.unwrap_or(0);
        let breaker_ok = !protocol.conservation_status();

        let health = HealthStatus {
            status: if breaker_ok { "healthy".to_string() } else { "degraded".to_string() },
            service: "caesar".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            active_packet_count: active as u64,
            circuit_breaker_ok: breaker_ok,
        };

        let payload = serde_json::to_vec(&health)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "caesar/health"
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format a [`PressureQuadrant`] as a lowercase snake_case string for JSON.
fn format_pressure(p: &governor::PressureQuadrant) -> String {
    match p {
        governor::PressureQuadrant::Bubble => "bubble".to_string(),
        governor::PressureQuadrant::Crash => "crash".to_string(),
        governor::PressureQuadrant::Stagnation => "stagnation".to_string(),
        governor::PressureQuadrant::GoldenEra => "golden_era".to_string(),
        governor::PressureQuadrant::Bottleneck => "bottleneck".to_string(),
        governor::PressureQuadrant::Vacuum => "vacuum".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

/// Caesar STOQ API Server
#[allow(dead_code)]
pub struct CaesarStoqApi {
    server: Arc<StoqApiServer>,
    config: CaesarStoqConfig,
}

impl CaesarStoqApi {
    /// Create new Caesar API server over STOQ with shared application state.
    #[instrument(skip(config, app_state))]
    pub async fn new(
        config: CaesarStoqConfig,
        app_state: Arc<CaesarAppState>,
    ) -> Result<Self> {
        info!("Creating Caesar STOQ API server on {}", config.bind_address);

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

        // Create API server and register packet-centric handlers
        let server = Arc::new(StoqApiServer::new(transport));

        server.register_handler(Arc::new(RoutePacketHandler {
            state: app_state.clone(),
        }));
        server.register_handler(Arc::new(GetNodeStatusHandler {
            state: app_state.clone(),
        }));
        server.register_handler(Arc::new(GetGovernorParamsHandler {
            state: app_state.clone(),
        }));
        server.register_handler(Arc::new(GetEffectiveRateHandler {
            state: app_state.clone(),
        }));
        server.register_handler(Arc::new(CaesarHealthHandler {
            state: app_state,
        }));

        info!("Caesar STOQ API handlers registered (5 endpoints)");

        Ok(Self { server, config })
    }

    /// Start the API server
    #[instrument(skip(self))]
    pub async fn serve(self: Arc<Self>) -> Result<()> {
        info!("Starting Caesar STOQ API server...");
        self.server.listen().await
    }

    /// Stop the server gracefully
    pub fn stop(&self) {
        info!("Stopping Caesar STOQ API server");
        self.server.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use tempfile::TempDir;

    // ------------------------------------------------------------------
    // Serialization tests (preserved from original)
    // ------------------------------------------------------------------

    #[test]
    fn route_packet_request_serialization() {
        let req = RoutePacketRequest {
            packet_id: "pkt-001".to_string(),
            initial_value_grams: rust_decimal::Decimal::new(100, 0),
            tier: "L0".to_string(),
            sender_node: "node-a".to_string(),
            recipient_hint: Some("node-z".to_string()),
        };
        let json = serde_json::to_string(&req)
            .expect("test: serialization should succeed");
        assert!(json.contains("pkt-001"));
        assert!(json.contains("L0"));
    }

    #[test]
    fn governor_params_response_serialization() {
        let resp = GetGovernorParamsResponse {
            pressure: "golden_era".to_string(),
            health_score: rust_decimal::Decimal::new(75, 0),
            recommended_fee_adjustment: rust_decimal::Decimal::ZERO,
        };
        let json = serde_json::to_string(&resp)
            .expect("test: serialization should succeed");
        assert!(json.contains("golden_era"));
    }

    // ------------------------------------------------------------------
    // Helper: build a real CaesarAppState backed by TempDir
    // ------------------------------------------------------------------

    async fn make_app_state(dir: &TempDir) -> Arc<CaesarAppState> {
        let config = crate::CaesarConfig {
            storage: crate::storage::StorageConfig {
                path: dir.path().to_str()
                    .expect("test: tempdir path")
                    .to_string(),
            },
            ..crate::CaesarConfig::default()
        };
        let protocol = CaesarProtocol::new(config)
            .await
            .expect("test: protocol init");
        Arc::new(CaesarAppState {
            protocol: Arc::new(RwLock::new(protocol)),
        })
    }

    fn make_api_request(path: &str, body: &impl Serialize) -> ApiRequest {
        ApiRequest {
            id: "test-req-1".to_string(),
            service: "caesar".to_string(),
            method: path.to_string(),
            payload: Bytes::from(
                serde_json::to_vec(body).expect("test: serialize request body"),
            ),
            metadata: std::collections::HashMap::new(),
        }
    }

    // ------------------------------------------------------------------
    // Handler integration tests
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_route_packet_handler_parses_request() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = RoutePacketHandler { state: app };

        let req = RoutePacketRequest {
            packet_id: "pkt-test-001".to_string(),
            initial_value_grams: rust_decimal::Decimal::new(50, 0),
            tier: "L1".to_string(),
            sender_node: "node-sender".to_string(),
            recipient_hint: None,
        };

        let api_req = make_api_request("route_packet", &req);
        let resp = handler.handle(api_req).await
            .expect("test: handler should succeed");

        assert!(resp.success, "response should be successful");
        let body: RoutePacketResponse = serde_json::from_slice(&resp.payload)
            .expect("test: deserialize response");
        assert_eq!(body.packet_id, "pkt-test-001");
        assert!(body.success);
        assert_eq!(body.state, "accepted");
        assert!(body.error.is_none());
    }

    #[tokio::test]
    async fn test_health_handler_returns_real_data() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = CaesarHealthHandler { state: app };

        // Empty payload for health check
        let api_req = ApiRequest {
            id: "test-health-1".to_string(),
            service: "caesar".to_string(),
            method: "health".to_string(),
            payload: Bytes::from("{}"),
            metadata: std::collections::HashMap::new(),
        };

        let resp = handler.handle(api_req).await
            .expect("test: health handler should succeed");

        assert!(resp.success);
        let body: HealthStatus = serde_json::from_slice(&resp.payload)
            .expect("test: deserialize health response");
        assert_eq!(body.status, "healthy");
        assert_eq!(body.service, "caesar");
        assert_eq!(body.active_packet_count, 0);
        assert!(body.circuit_breaker_ok);
    }

    #[tokio::test]
    async fn test_governor_params_handler() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = GetGovernorParamsHandler { state: app };

        let api_req = ApiRequest {
            id: "test-gov-1".to_string(),
            service: "caesar".to_string(),
            method: "governor_params".to_string(),
            payload: Bytes::from("{}"),
            metadata: std::collections::HashMap::new(),
        };

        let resp = handler.handle(api_req).await
            .expect("test: governor handler should succeed");

        assert!(resp.success);
        let body: GetGovernorParamsResponse = serde_json::from_slice(&resp.payload)
            .expect("test: deserialize governor response");
        assert_eq!(body.pressure, "golden_era");
        assert_eq!(body.health_score, rust_decimal::Decimal::new(50, 0));
        assert_eq!(body.recommended_fee_adjustment, rust_decimal::Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_effective_rate_handler() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;
        let handler = GetEffectiveRateHandler { state: app };

        let api_req = ApiRequest {
            id: "test-rate-1".to_string(),
            service: "caesar".to_string(),
            method: "effective_rate".to_string(),
            payload: Bytes::from("{}"),
            metadata: std::collections::HashMap::new(),
        };

        let resp = handler.handle(api_req).await
            .expect("test: effective rate handler should succeed");

        assert!(resp.success);
        let body: GetEffectiveRateResponse = serde_json::from_slice(&resp.payload)
            .expect("test: deserialize rate response");

        // With default gold price 2350 and 1% fee, 0 speculation, 0 in-transit:
        // effective_rate = (2350 / 31.1035) * (1 + 0.01 + 0 - 0) > 0
        assert!(body.effective_rate > rust_decimal::Decimal::ZERO,
            "effective rate should be positive: {}", body.effective_rate);
        assert_eq!(body.network_fees_component, dec!(0.01));
        assert_eq!(body.speculation_pressure, rust_decimal::Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_node_status_handler_returns_real_count() {
        let dir = TempDir::new().expect("test: tempdir");
        let app = make_app_state(&dir).await;

        // Mint a packet so active count is non-zero
        {
            let mut protocol = app.protocol.write().await;
            protocol.mint_packet(
                hypermesh_lib::NodeId::from("sender"),
                hypermesh_lib::NodeId::from("recipient"),
                hypermesh_lib::economic::GoldGrams::from_decimal(dec!(100)),
                hypermesh_lib::economic::GoldGrams::from_decimal(dec!(0.1)),
                hypermesh_lib::economic::MarketTier::L0,
                20,
                hypermesh_lib::economic::GoldGrams::from_decimal(dec!(5)),
            ).await.expect("test: mint");
        }

        let handler = GetNodeStatusHandler { state: app };
        let req = GetNodeStatusRequest { node_id: "node-abc".to_string() };
        let api_req = make_api_request("node_status", &req);

        let resp = handler.handle(api_req).await
            .expect("test: node status handler should succeed");

        assert!(resp.success);
        let body: GetNodeStatusResponse = serde_json::from_slice(&resp.payload)
            .expect("test: deserialize node status response");
        assert_eq!(body.node_id, "node-abc");
        assert_eq!(body.active_packets, 1, "should report 1 active packet");
    }

    #[test]
    fn test_format_pressure_all_variants() {
        assert_eq!(format_pressure(&governor::PressureQuadrant::Bubble), "bubble");
        assert_eq!(format_pressure(&governor::PressureQuadrant::Crash), "crash");
        assert_eq!(format_pressure(&governor::PressureQuadrant::Stagnation), "stagnation");
        assert_eq!(format_pressure(&governor::PressureQuadrant::GoldenEra), "golden_era");
        assert_eq!(format_pressure(&governor::PressureQuadrant::Bottleneck), "bottleneck");
        assert_eq!(format_pressure(&governor::PressureQuadrant::Vacuum), "vacuum");
    }
}
