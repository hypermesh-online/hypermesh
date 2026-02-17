// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

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
    fn path(&self) -> &str {
        "/api/health"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let response = json!({
            "status": "healthy",
            "service": "hypermesh-assets",
            "timestamp": chrono::Utc::now(),
            "version": "0.1.0"
        });

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(format!("Failed to serialize: {}", e)))?;

        Ok(ApiResponse {
            request_id: req.id.clone(),
            success: true,
            payload: bytes::Bytes::from(payload),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }
}

/// List assets handler
struct ListAssetsHandler;

#[async_trait::async_trait]
impl ApiHandler for ListAssetsHandler {
    fn path(&self) -> &str {
        "/api/assets"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let response = json!([
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
        ]);

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(format!("Failed to serialize: {}", e)))?;

        Ok(ApiResponse {
            request_id: req.id.clone(),
            success: true,
            payload: bytes::Bytes::from(payload),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }
}

/// List nodes handler
struct ListNodesHandler;

#[async_trait::async_trait]
impl ApiHandler for ListNodesHandler {
    fn path(&self) -> &str {
        "/api/nodes"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let response = json!([
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
        ]);

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(format!("Failed to serialize: {}", e)))?;

        Ok(ApiResponse {
            request_id: req.id.clone(),
            success: true,
            payload: bytes::Bytes::from(payload),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }
}

/// HyperMesh status handler
struct HyperMeshStatusHandler;

#[async_trait::async_trait]
impl ApiHandler for HyperMeshStatusHandler {
    fn path(&self) -> &str {
        "/api/status"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let response = json!({
            "network_health": "operational",
            "total_nodes": 15,
            "active_nodes": 14,
            "total_assets": 847,
            "active_assets": 823,
            "consensus_status": "synced",
            "last_block": 1_234_567
        });

        let payload = serde_json::to_vec(&response)
            .map_err(|e| ApiError::SerializationError(format!("Failed to serialize: {}", e)))?;

        Ok(ApiResponse {
            request_id: req.id.clone(),
            success: true,
            payload: bytes::Bytes::from(payload),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install rustls crypto provider
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok(); // Ignore error if already installed

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