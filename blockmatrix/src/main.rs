use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{info, warn};

use stoq::{ApiHandler, ApiRequest, ApiResponse, ApiError, StoqApiServer};
use stoq::transport::{StoqTransport, TransportConfig};

/// Health check handler
struct HealthCheckHandler;

#[async_trait::async_trait]
impl ApiHandler for HealthCheckHandler {
    async fn handle(&self, _req: ApiRequest) -> Result<ApiResponse, ApiError> {
        Ok(ApiResponse::Json(json!({
            "status": "healthy",
            "service": "hypermesh-assets",
            "timestamp": chrono::Utc::now(),
            "version": "0.1.0"
        })))
    }
}

/// List assets handler
struct ListAssetsHandler;

#[async_trait::async_trait]
impl ApiHandler for ListAssetsHandler {
    async fn handle(&self, _req: ApiRequest) -> Result<ApiResponse, ApiError> {
        Ok(ApiResponse::Json(json!([
            {
                "id": "asset_001",
                "type": "CPU",
                "status": "active",
                "performance": "98.5%",
                "location": "node_alpha"
            },
            {
                "id": "asset_002",
                "type": "GPU",
                "status": "active",
                "performance": "92.3%",
                "location": "node_beta"
            },
            {
                "id": "asset_003",
                "type": "Memory",
                "status": "shared",
                "performance": "89.7%",
                "location": "node_gamma"
            }
        ])))
    }
}

/// List nodes handler
struct ListNodesHandler;

#[async_trait::async_trait]
impl ApiHandler for ListNodesHandler {
    async fn handle(&self, _req: ApiRequest) -> Result<ApiResponse, ApiError> {
        Ok(ApiResponse::Json(json!([
            {
                "id": "node_alpha",
                "status": "healthy",
                "cpu_usage": 45.2,
                "memory_usage": 62.8,
                "network_latency": 12
            },
            {
                "id": "node_beta",
                "status": "healthy",
                "cpu_usage": 38.9,
                "memory_usage": 71.3,
                "network_latency": 8
            },
            {
                "id": "node_gamma",
                "status": "degraded",
                "cpu_usage": 78.5,
                "memory_usage": 85.1,
                "network_latency": 25
            }
        ])))
    }
}

/// HyperMesh status handler
struct HyperMeshStatusHandler;

#[async_trait::async_trait]
impl ApiHandler for HyperMeshStatusHandler {
    async fn handle(&self, _req: ApiRequest) -> Result<ApiResponse, ApiError> {
        Ok(ApiResponse::Json(json!({
            "network_health": "operational",
            "total_nodes": 15,
            "active_nodes": 14,
            "total_assets": 847,
            "active_assets": 823,
            "consensus_status": "synced",
            "last_block": 1_234_567
        })))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Create STOQ transport configuration
    let transport_config = TransportConfig {
        bind_address: std::net::Ipv6Addr::UNSPECIFIED,  // [::] = listen on all interfaces
        port: 8446,
        ..Default::default()
    };

    // Create transport
    let transport = Arc::new(StoqTransport::new(transport_config).await?);

    // Create STOQ API server
    let server = StoqApiServer::new(transport);

    // Register handlers for API endpoints
    server.register_handler(Arc::new(HealthCheckHandler));
    server.register_handler(Arc::new(ListAssetsHandler));
    server.register_handler(Arc::new(ListNodesHandler));
    server.register_handler(Arc::new(HyperMeshStatusHandler));

    info!("🔗 HyperMesh Assets listening on [::]:8446 (STOQ protocol)");

    // Start the STOQ server
    server.listen().await?;

    Ok(())
}