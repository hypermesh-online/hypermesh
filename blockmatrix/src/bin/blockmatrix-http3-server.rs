// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::Result;
use http::{Response, StatusCode};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::atomic::Ordering;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use blockmatrix::http3::handlers::{
    self, AllocationInfo, AllocationsResponse, AssetAllocationResponse, AssetListResponse,
    ByzantineDetection, ConnectionInfo, ConnectionsResponse, DetectionsResponse,
    HealthResponse, HyperMeshSystemStatus, MatrixPositionDto, NodeHealth,
    NodesHealthResponse, PerformanceMetrics, ResourceStatus, ServerState,
    StatusResponse, StoqHealthResponse,
};
use blockmatrix::http3::{Router, Http3StoqServer};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("BlockMatrix HTTP/3 Server starting...");

    let state = ServerState::new();

    let router = build_router(&state);

    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8446);
    let server = Http3StoqServer::new(addr, router);

    info!("BlockMatrix HTTP/3 server (STOQ transport) starting on https://[::1]:8446");
    server.run().await?;

    Ok(())
}

fn build_router(state: &ServerState) -> Router {
    let start_time = state.start_time;
    let bytes_sent = state.bytes_sent.clone();
    let bytes_received = state.bytes_received.clone();
    let request_count = state.request_count.clone();
    let connections = state.connections.clone();

    Router::new()
        .options("/*", |_req| async move {
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/plain")
                .body(Vec::new())
                .unwrap_or_else(|_| Response::new(Vec::new()))
        })
        .get("/api/v1/blockmatrix/health", move |_req| {
            let uptime = start_time.elapsed().as_secs();
            async move {
                handlers::json_success_response(HealthResponse {
                    status: "healthy".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    uptime_seconds: uptime,
                    endpoints_available: 5,
                    matrix_nodes: 42,
                })
            }
        })
        .get("/api/v1/blockmatrix/status", |_req| async move {
            handlers::json_success_response(StatusResponse {
                node_id: uuid::Uuid::new_v4().to_string(),
                matrix_position: MatrixPositionDto { x: 10, y: 20, z: 1 },
                blockchain_height: 54321,
                peers_connected: 12,
                assets_managed: 256,
                storage_gb: 1024.5,
                cpu_cores: 16,
                gpu_available: true,
            })
        })
        .get("/api/v1/blockmatrix/assets", |_req| async move {
            let assets = handlers::sample_asset_list(true);
            handlers::json_success_response(AssetListResponse {
                total: assets.len(),
                page: 1,
                per_page: 20,
                assets,
            })
        })
        .post("/api/v1/blockmatrix/assets/allocate", |_req| async move {
            handlers::json_success_response(AssetAllocationResponse {
                asset_id: uuid::Uuid::new_v4().to_string(),
                resource_type: "CPU".to_string(),
                amount_allocated: 4,
                privacy_tier: "Federated".to_string(),
                expires_at: chrono::Utc::now().timestamp() + 3600,
                proxy_address: format!("2001:db8::asset:{}", uuid::Uuid::new_v4().simple()),
                consensus_proofs: handlers::consensus_proof_strings(),
            })
        })
        .get("/api/v1/blockmatrix/assets/{asset_id}", |req| async move {
            let path = req.uri().path();
            let asset_id = path.split('/').last().unwrap_or("unknown");
            handlers::json_success_response(handlers::AssetInfo {
                asset_id: asset_id.to_string(),
                owner: "0xmnop...3456".to_string(),
                resource_type: "Memory".to_string(),
                status: "active".to_string(),
                created_at: chrono::Utc::now().timestamp() - 600,
                privacy_tier: "Anonymous".to_string(),
                proxy_address: format!("2001:db8::mem:{}", asset_id),
                consensus_proofs: handlers::consensus_proof_strings(),
                matrix_shards: vec![
                    handlers::MatrixShard {
                        shard_id: format!("{}_shard_1", asset_id),
                        position: MatrixPositionDto { x: 25, y: 30, z: 0 },
                        size_bytes: 4194304,
                        redundancy_level: 3,
                    },
                    handlers::MatrixShard {
                        shard_id: format!("{}_shard_2", asset_id),
                        position: MatrixPositionDto { x: 35, y: 40, z: 0 },
                        size_bytes: 4194304,
                        redundancy_level: 3,
                    },
                    handlers::MatrixShard {
                        shard_id: format!("{}_shard_3", asset_id),
                        position: MatrixPositionDto { x: 45, y: 50, z: 0 },
                        size_bytes: 4194304,
                        redundancy_level: 3,
                    },
                ],
            })
        })
        .get("/api/v1/hypermesh/system/status", move |_req| {
            let uptime = start_time.elapsed().as_secs();
            async move {
                handlers::json_success_response(HyperMeshSystemStatus {
                    node_id: uuid::Uuid::new_v4().to_string(),
                    status: "operational".to_string(),
                    uptime_seconds: uptime,
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    matrix_position: MatrixPositionDto { x: 10, y: 20, z: 1 },
                    resources: ResourceStatus {
                        cpu_usage_percent: 25.5,
                        memory_used_gb: 8.5,
                        memory_total_gb: 32.0,
                        storage_used_gb: 450.0,
                        storage_total_gb: 1000.0,
                    },
                })
            }
        })
        .get("/api/v1/hypermesh/assets", |_req| async move {
            let assets = handlers::sample_asset_list(false);
            handlers::json_success_response(AssetListResponse {
                total: assets.len(),
                page: 1,
                per_page: 20,
                assets,
            })
        })
        .get("/api/v1/hypermesh/allocations", move |_req| {
            let _uptime = start_time.elapsed().as_secs();
            async move {
                let allocations = vec![
                    AllocationInfo {
                        allocation_id: uuid::Uuid::new_v4().to_string(),
                        asset_id: "asset_001".to_string(),
                        resource_type: "CPU".to_string(),
                        amount: 4,
                        status: "active".to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        expires_at: (chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339(),
                    },
                    AllocationInfo {
                        allocation_id: uuid::Uuid::new_v4().to_string(),
                        asset_id: "asset_002".to_string(),
                        resource_type: "GPU".to_string(),
                        amount: 1,
                        status: "active".to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        expires_at: (chrono::Utc::now() + chrono::Duration::hours(4)).to_rfc3339(),
                    },
                    AllocationInfo {
                        allocation_id: uuid::Uuid::new_v4().to_string(),
                        asset_id: "asset_003".to_string(),
                        resource_type: "Memory".to_string(),
                        amount: 16,
                        status: "pending".to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        expires_at: (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
                    },
                ];
                let active_count = allocations.iter().filter(|a| a.status == "active").count();
                handlers::json_success_response(AllocationsResponse {
                    total: allocations.len(),
                    active: active_count,
                    allocations,
                })
            }
        })
        .get("/api/v1/stoq/system/health", {
            let bytes_sent = bytes_sent.clone();
            let bytes_received = bytes_received.clone();
            let connections = connections.clone();
            move |_req| {
                let bytes_sent = bytes_sent.clone();
                let bytes_received = bytes_received.clone();
                let connections = connections.clone();
                async move {
                    handlers::json_success_response(StoqHealthResponse {
                        transport_status: "operational".to_string(),
                        quic_version: "h3-29".to_string(),
                        active_connections: connections.len(),
                        bytes_sent: bytes_sent.load(Ordering::Relaxed),
                        bytes_received: bytes_received.load(Ordering::Relaxed),
                        packet_loss_percent: 0.02,
                    })
                }
            }
        })
        .get("/api/v1/stoq/connections", {
            let connections = connections.clone();
            move |_req| {
                let _connections = connections.clone();
                async move {
                    let mock_connections = vec![
                        ConnectionInfo {
                            connection_id: uuid::Uuid::new_v4().to_string(),
                            remote_addr: "2001:db8::1".to_string(),
                            status: "established".to_string(),
                            uptime_seconds: 3600,
                            bytes_sent: 1_048_576,
                            bytes_received: 2_097_152,
                        },
                        ConnectionInfo {
                            connection_id: uuid::Uuid::new_v4().to_string(),
                            remote_addr: "2001:db8::2".to_string(),
                            status: "established".to_string(),
                            uptime_seconds: 1800,
                            bytes_sent: 524_288,
                            bytes_received: 1_048_576,
                        },
                        ConnectionInfo {
                            connection_id: uuid::Uuid::new_v4().to_string(),
                            remote_addr: "2001:db8::3".to_string(),
                            status: "handshaking".to_string(),
                            uptime_seconds: 5,
                            bytes_sent: 1024,
                            bytes_received: 2048,
                        },
                    ];
                    handlers::json_success_response(ConnectionsResponse {
                        total: mock_connections.len(),
                        connections: mock_connections,
                    })
                }
            }
        })
        .get("/api/v1/hypermesh/nodes/health", |_req| async move {
            let nodes = vec![
                NodeHealth {
                    node_id: uuid::Uuid::new_v4().to_string(),
                    status: "healthy".to_string(),
                    last_seen: chrono::Utc::now().to_rfc3339(),
                    latency_ms: 12.5,
                    matrix_position: MatrixPositionDto { x: 10, y: 20, z: 1 },
                },
                NodeHealth {
                    node_id: uuid::Uuid::new_v4().to_string(),
                    status: "healthy".to_string(),
                    last_seen: chrono::Utc::now().to_rfc3339(),
                    latency_ms: 18.3,
                    matrix_position: MatrixPositionDto { x: 15, y: 25, z: 1 },
                },
                NodeHealth {
                    node_id: uuid::Uuid::new_v4().to_string(),
                    status: "degraded".to_string(),
                    last_seen: (chrono::Utc::now() - chrono::Duration::minutes(2)).to_rfc3339(),
                    latency_ms: 145.7,
                    matrix_position: MatrixPositionDto { x: 30, y: 40, z: 2 },
                },
                NodeHealth {
                    node_id: uuid::Uuid::new_v4().to_string(),
                    status: "healthy".to_string(),
                    last_seen: chrono::Utc::now().to_rfc3339(),
                    latency_ms: 8.9,
                    matrix_position: MatrixPositionDto { x: 5, y: 15, z: 0 },
                },
            ];
            let healthy_count = nodes.iter().filter(|n| n.status == "healthy").count();
            handlers::json_success_response(NodesHealthResponse {
                total_nodes: nodes.len(),
                healthy_nodes: healthy_count,
                nodes,
            })
        })
        .get("/api/v1/stoq/metrics/performance", {
            let request_count = request_count.clone();
            move |_req| {
                let request_count = request_count.clone();
                async move {
                    request_count.fetch_add(1, Ordering::Relaxed);
                    handlers::json_success_response(PerformanceMetrics {
                        latency_p50_ms: 12.5,
                        latency_p95_ms: 35.2,
                        latency_p99_ms: 98.7,
                        throughput_mbps: 850.5,
                        requests_per_second: request_count.load(Ordering::Relaxed) as f32 / 60.0,
                        success_rate_percent: 99.8,
                    })
                }
            }
        })
        .get("/api/v1/hypermesh/byzantine/detections", |_req| async move {
            let detections = vec![
                ByzantineDetection {
                    detection_id: uuid::Uuid::new_v4().to_string(),
                    node_id: uuid::Uuid::new_v4().to_string(),
                    detection_type: "double_sign".to_string(),
                    severity: "high".to_string(),
                    detected_at: (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339(),
                    evidence: "Multiple conflicting signatures on blocks 12345 and 12346".to_string(),
                },
                ByzantineDetection {
                    detection_id: uuid::Uuid::new_v4().to_string(),
                    node_id: uuid::Uuid::new_v4().to_string(),
                    detection_type: "invalid_proof".to_string(),
                    severity: "medium".to_string(),
                    detected_at: (chrono::Utc::now() - chrono::Duration::hours(8)).to_rfc3339(),
                    evidence: "Invalid PoSpace proof submitted for asset allocation".to_string(),
                },
                ByzantineDetection {
                    detection_id: uuid::Uuid::new_v4().to_string(),
                    node_id: uuid::Uuid::new_v4().to_string(),
                    detection_type: "network_partition_attack".to_string(),
                    severity: "critical".to_string(),
                    detected_at: (chrono::Utc::now() - chrono::Duration::hours(18)).to_rfc3339(),
                    evidence: "Attempted to isolate nodes in matrix region (5,10,0)-(15,20,1)".to_string(),
                },
            ];
            let last_24h = detections.len(); // Mock: all within 24h
            handlers::json_success_response(DetectionsResponse {
                total: detections.len(),
                last_24h,
                detections,
            })
        })
}
