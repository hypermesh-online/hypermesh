use anyhow::Result;
use http::{Method, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use blockmatrix::http3::{ApiResponse, Router, Http3Server};

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_seconds: u64,
    endpoints_available: usize,
    matrix_nodes: usize,
}

#[derive(Serialize)]
struct StatusResponse {
    node_id: String,
    matrix_position: MatrixPosition,
    blockchain_height: u64,
    peers_connected: usize,
    assets_managed: u64,
    storage_gb: f64,
    cpu_cores: usize,
    gpu_available: bool,
}

#[derive(Serialize, Deserialize)]
struct MatrixPosition {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Serialize)]
struct AssetInfo {
    asset_id: String,
    owner: String,
    resource_type: String,
    status: String,
    created_at: i64,
    privacy_tier: String,
    proxy_address: String,
    consensus_proofs: Vec<String>,
    matrix_shards: Vec<MatrixShard>,
}

#[derive(Serialize)]
struct MatrixShard {
    shard_id: String,
    position: MatrixPosition,
    size_bytes: u64,
    redundancy_level: u32,
}

#[derive(Serialize)]
struct AssetListResponse {
    assets: Vec<AssetInfo>,
    total: usize,
    page: usize,
    per_page: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("BlockMatrix Simple HTTP/3 Server starting...");

    // Track start time for uptime calculation
    let start_time = std::time::Instant::now();

    // Create router with all endpoints
    let router = Router::new()
        // Global OPTIONS handler for CORS preflight requests
        .options("/*", |_req| async move {
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/plain")
                .body(Vec::new())
                .unwrap_or_else(|_| Response::new(Vec::new()))
        })
        // Health endpoint
        .get("/api/v1/blockmatrix/health", move |_req| {
            let uptime = start_time.elapsed().as_secs();
            async move {
                let response = HealthResponse {
                    status: "healthy".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    uptime_seconds: uptime,
                    endpoints_available: 5,
                    matrix_nodes: 42,
                };

                let body = serde_json::to_vec(&ApiResponse::success(
                    response,
                    uuid::Uuid::new_v4().to_string(),
                )).unwrap_or_default();

                Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(body)
                    .unwrap_or_else(|_| Response::new(Vec::new()))
            }
        })
        // Status endpoint (blockmatrix path)
        .get("/api/v1/blockmatrix/status", |_req| async move {
            let response = StatusResponse {
                node_id: uuid::Uuid::new_v4().to_string(),
                matrix_position: MatrixPosition { x: 10, y: 20, z: 1 },
                blockchain_height: 54321,
                peers_connected: 12,
                assets_managed: 256,
                storage_gb: 1024.5,
                cpu_cores: 16,
                gpu_available: true,
            };

            let body = serde_json::to_vec(&ApiResponse::success(
                response,
                uuid::Uuid::new_v4().to_string(),
            )).unwrap_or_default();

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(body)
                .unwrap_or_else(|_| Response::new(Vec::new()))
        })
        // Status endpoint (hypermesh path for compatibility)
        .get("/api/v1/hypermesh/system/status", |_req| async move {
            let response = StatusResponse {
                node_id: uuid::Uuid::new_v4().to_string(),
                matrix_position: MatrixPosition { x: 10, y: 20, z: 1 },
                blockchain_height: 54321,
                peers_connected: 12,
                assets_managed: 256,
                storage_gb: 1024.5,
                cpu_cores: 16,
                gpu_available: true,
            };

            let body = serde_json::to_vec(&ApiResponse::success(
                response,
                uuid::Uuid::new_v4().to_string(),
            )).unwrap_or_default();

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(body)
                .unwrap_or_else(|_| Response::new(Vec::new()))
        })
        // List assets endpoint
        .get("/api/v1/blockmatrix/assets", |_req| async move {
            let assets = vec![
                AssetInfo {
                    asset_id: "asset_001".to_string(),
                    owner: "0xabcd...1234".to_string(),
                    resource_type: "CPU".to_string(),
                    status: "active".to_string(),
                    created_at: chrono::Utc::now().timestamp() - 3600,
                    privacy_tier: "Federated".to_string(),
                    proxy_address: "2001:db8::cpu:1".to_string(),
                    consensus_proofs: vec![
                        "PoSpace".to_string(),
                        "PoStake".to_string(),
                        "PoWork".to_string(),
                        "PoTime".to_string(),
                    ],
                    matrix_shards: vec![
                        MatrixShard {
                            shard_id: "shard_001_a".to_string(),
                            position: MatrixPosition { x: 5, y: 10, z: 0 },
                            size_bytes: 1048576,
                            redundancy_level: 3,
                        },
                    ],
                },
                AssetInfo {
                    asset_id: "asset_002".to_string(),
                    owner: "0xefgh...5678".to_string(),
                    resource_type: "GPU".to_string(),
                    status: "active".to_string(),
                    created_at: chrono::Utc::now().timestamp() - 7200,
                    privacy_tier: "Public".to_string(),
                    proxy_address: "2001:db8::gpu:1".to_string(),
                    consensus_proofs: vec![
                        "PoSpace".to_string(),
                        "PoStake".to_string(),
                        "PoWork".to_string(),
                        "PoTime".to_string(),
                    ],
                    matrix_shards: vec![
                        MatrixShard {
                            shard_id: "shard_002_a".to_string(),
                            position: MatrixPosition { x: 8, y: 12, z: 1 },
                            size_bytes: 2097152,
                            redundancy_level: 5,
                        },
                    ],
                },
            ];

            let response = AssetListResponse {
                total: assets.len(),
                page: 1,
                per_page: 20,
                assets,
            };

            let body = serde_json::to_vec(&ApiResponse::success(
                response,
                uuid::Uuid::new_v4().to_string(),
            )).unwrap_or_default();

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(body)
                .unwrap_or_else(|_| Response::new(Vec::new()))
        })
        // Allocate asset endpoint
        .post("/api/v1/blockmatrix/assets/allocate", |_req| async move {
            // In production, this would parse the request body and allocate real resources
            let response = serde_json::json!({
                "asset_id": uuid::Uuid::new_v4().to_string(),
                "resource_type": "CPU",
                "amount_allocated": 4,
                "privacy_tier": "Federated",
                "expires_at": chrono::Utc::now().timestamp() + 3600,
                "proxy_address": format!("2001:db8::asset:{}", uuid::Uuid::new_v4().simple()),
                "consensus_proofs": vec![
                    "PoSpace",
                    "PoStake",
                    "PoWork",
                    "PoTime",
                ],
            });

            let body = serde_json::to_vec(&ApiResponse::success(
                response,
                uuid::Uuid::new_v4().to_string(),
            )).unwrap_or_default();

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(body)
                .unwrap_or_else(|_| Response::new(Vec::new()))
        })
        // Get specific asset endpoint
        .get("/api/v1/blockmatrix/assets/{asset_id}", |req| async move {
            // Extract asset ID from path
            let path = req.uri().path();
            let asset_id = path.split('/').last().unwrap_or("unknown");

            let response = AssetInfo {
                asset_id: asset_id.to_string(),
                owner: "0xmnop...3456".to_string(),
                resource_type: "Memory".to_string(),
                status: "active".to_string(),
                created_at: chrono::Utc::now().timestamp() - 600,
                privacy_tier: "Anonymous".to_string(),
                proxy_address: format!("2001:db8::mem:{}", asset_id),
                consensus_proofs: vec![
                    "PoSpace".to_string(),
                    "PoStake".to_string(),
                    "PoWork".to_string(),
                    "PoTime".to_string(),
                ],
                matrix_shards: vec![
                    MatrixShard {
                        shard_id: format!("{}_shard_1", asset_id),
                        position: MatrixPosition { x: 25, y: 30, z: 0 },
                        size_bytes: 4194304,
                        redundancy_level: 3,
                    },
                ],
            };

            let body = serde_json::to_vec(&ApiResponse::success(
                response,
                uuid::Uuid::new_v4().to_string(),
            )).unwrap_or_default();

            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(body)
                .unwrap_or_else(|_| Response::new(Vec::new()))
        });

    // Start server on IPv6 localhost port 8446 using standard HTTP/3
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8446);
    let server = Http3Server::new(addr, router);

    info!("BlockMatrix Simple HTTP/3 server starting on https://[::1]:8446");
    server.run().await?;

    Ok(())
}