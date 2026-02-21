// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar STOQ API -- packet-centric EVP operations over STOQ protocol.
//!
//! Replaces the old wallet-based handlers with packet routing, node status,
//! governor parameter queries, and effective rate lookups.

use async_trait::async_trait;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tracing::{info, debug, instrument};

use stoq::api::{ApiHandler, ApiRequest, ApiResponse, ApiError};
use stoq::StoqApiServer;
use stoq::transport::{StoqTransport, TransportConfig};

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

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Route a value packet to the next hop.
pub struct RoutePacketHandler;

#[async_trait]
impl ApiHandler for RoutePacketHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling route_packet: {}", request.id);

        let req: RoutePacketRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(
                format!("Invalid route_packet request: {}", e),
            ))?;

        // Stub: accept the packet and return Minted state
        let response = RoutePacketResponse {
            packet_id: req.packet_id,
            success: true,
            state: "minted".to_string(),
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
pub struct GetNodeStatusHandler;

#[async_trait]
impl ApiHandler for GetNodeStatusHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling node_status: {}", request.id);

        let req: GetNodeStatusRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(
                format!("Invalid node_status request: {}", e),
            ))?;

        // Stub: return zeroed status for the requested node
        let response = GetNodeStatusResponse {
            node_id: req.node_id,
            active_packets: 0,
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
pub struct GetGovernorParamsHandler;

#[async_trait]
impl ApiHandler for GetGovernorParamsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling governor_params: {}", request.id);

        // Stub: return default governance state
        let response = GetGovernorParamsResponse {
            pressure: "golden_era".to_string(),
            health_score: rust_decimal::Decimal::new(50, 0),
            recommended_fee_adjustment: rust_decimal::Decimal::ZERO,
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
pub struct GetEffectiveRateHandler;

#[async_trait]
impl ApiHandler for GetEffectiveRateHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling effective_rate: {}", request.id);

        // Stub: return neutral effective rate
        let response = GetEffectiveRateResponse {
            network_fees_component: rust_decimal::Decimal::ZERO,
            speculation_pressure: rust_decimal::Decimal::ZERO,
            liquidity_shadow: rust_decimal::Decimal::ZERO,
            effective_rate: rust_decimal::Decimal::ZERO,
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

/// Health check handler (unchanged from original).
pub struct CaesarHealthHandler;

#[async_trait]
impl ApiHandler for CaesarHealthHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        #[derive(Serialize)]
        struct HealthStatus {
            status: String,
            service: String,
            version: String,
        }

        let health = HealthStatus {
            status: "healthy".to_string(),
            service: "caesar".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
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
// Server
// ---------------------------------------------------------------------------

/// Caesar STOQ API Server
#[allow(dead_code)]
pub struct CaesarStoqApi {
    server: Arc<StoqApiServer>,
    config: CaesarStoqConfig,
}

impl CaesarStoqApi {
    /// Create new Caesar API server over STOQ
    #[instrument(skip(config))]
    pub async fn new(config: CaesarStoqConfig) -> Result<Self> {
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

        server.register_handler(Arc::new(RoutePacketHandler));
        server.register_handler(Arc::new(GetNodeStatusHandler));
        server.register_handler(Arc::new(GetGovernorParamsHandler));
        server.register_handler(Arc::new(GetEffectiveRateHandler));
        server.register_handler(Arc::new(CaesarHealthHandler));

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
}
