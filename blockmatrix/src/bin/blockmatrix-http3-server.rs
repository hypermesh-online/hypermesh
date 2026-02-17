// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::Result;
use http::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use dashmap::DashMap;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use blockmatrix::http3::{ApiResponse, Router, Http3StoqServer};

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
    matrix_position: MatrixPositionDto,
    blockchain_height: u64,
    peers_connected: usize,
    assets_managed: u64,
    storage_gb: f64,
    cpu_cores: usize,
    gpu_available: bool,
}

/// Local DTO for HTTP/3 API responses; canonical MatrixPosition in hypermesh_lib.
#[derive(Serialize, Deserialize)]
struct MatrixPositionDto {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Deserialize)]
#[allow(dead_code)] // Deserialized from request payload
struct AssetAllocationRequest {
    resource_type: String,
    amount: u64,
    privacy_tier: String,
    duration_seconds: u64,
}

#[derive(Serialize)]
struct AssetAllocationResponse {
    asset_id: String,
    resource_type: String,
    amount_allocated: u64,
    privacy_tier: String,
    expires_at: i64,
    proxy_address: String,
    consensus_proofs: Vec<String>,
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
    position: MatrixPositionDto,
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

// New structures for Week 1 endpoints

#[derive(Serialize)]
struct HyperMeshSystemStatus {
    node_id: String,
    status: String,
    uptime_seconds: u64,
    version: String,
    matrix_position: MatrixPositionDto,
    resources: ResourceStatus,
}

#[derive(Serialize)]
struct ResourceStatus {
    cpu_usage_percent: f32,
    memory_used_gb: f32,
    memory_total_gb: f32,
    storage_used_gb: f32,
    storage_total_gb: f32,
}

#[derive(Serialize)]
struct AllocationInfo {
    allocation_id: String,
    asset_id: String,
    resource_type: String,
    amount: u32,
    status: String,
    created_at: String,
    expires_at: String,
}

#[derive(Serialize)]
struct AllocationsResponse {
    allocations: Vec<AllocationInfo>,
    total: usize,
    active: usize,
}

#[derive(Serialize)]
struct StoqHealthResponse {
    transport_status: String,
    quic_version: String,
    active_connections: usize,
    bytes_sent: u64,
    bytes_received: u64,
    packet_loss_percent: f32,
}

#[derive(Serialize)]
struct ConnectionInfo {
    connection_id: String,
    remote_addr: String,
    status: String,
    uptime_seconds: u64,
    bytes_sent: u64,
    bytes_received: u64,
}

#[derive(Serialize)]
struct ConnectionsResponse {
    connections: Vec<ConnectionInfo>,
    total: usize,
}

#[derive(Serialize)]
struct NodeHealth {
    node_id: String,
    status: String,
    last_seen: String,
    latency_ms: f32,
    matrix_position: MatrixPositionDto,
}

#[derive(Serialize)]
struct NodesHealthResponse {
    nodes: Vec<NodeHealth>,
    total_nodes: usize,
    healthy_nodes: usize,
}

#[derive(Serialize)]
struct PerformanceMetrics {
    latency_p50_ms: f32,
    latency_p95_ms: f32,
    latency_p99_ms: f32,
    throughput_mbps: f32,
    requests_per_second: f32,
    success_rate_percent: f32,
}

#[derive(Serialize)]
struct ByzantineDetection {
    detection_id: String,
    node_id: String,
    detection_type: String,
    severity: String,
    detected_at: String,
    evidence: String,
}

#[derive(Serialize)]
struct DetectionsResponse {
    detections: Vec<ByzantineDetection>,
    total: usize,
    last_24h: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize rustls crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("BlockMatrix HTTP/3 Server starting...");

    // Track start time for uptime calculation
    let start_time = std::time::Instant::now();

    // Shared state for metrics
    let bytes_sent = Arc::new(AtomicU64::new(0));
    let bytes_received = Arc::new(AtomicU64::new(0));
    let request_count = Arc::new(AtomicU64::new(0));
    let connections: Arc<DashMap<String, ConnectionInfo>> = Arc::new(DashMap::new());

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
                    matrix_nodes: 42, // In production, query actual matrix topology
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
        // Status endpoint
        .get("/api/v1/blockmatrix/status", |_req| async move {
            let response = StatusResponse {
                node_id: uuid::Uuid::new_v4().to_string(),
                matrix_position: MatrixPositionDto { x: 10, y: 20, z: 1 },
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
                            position: MatrixPositionDto { x: 5, y: 10, z: 0 },
                            size_bytes: 1048576,
                            redundancy_level: 3,
                        },
                        MatrixShard {
                            shard_id: "shard_001_b".to_string(),
                            position: MatrixPositionDto { x: 15, y: 20, z: 0 },
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
                            position: MatrixPositionDto { x: 8, y: 12, z: 1 },
                            size_bytes: 2097152,
                            redundancy_level: 5,
                        },
                    ],
                },
                AssetInfo {
                    asset_id: "asset_003".to_string(),
                    owner: "0xijkl...9012".to_string(),
                    resource_type: "Storage".to_string(),
                    status: "active".to_string(),
                    created_at: chrono::Utc::now().timestamp() - 1800,
                    privacy_tier: "Private".to_string(),
                    proxy_address: "2001:db8::storage:1".to_string(),
                    consensus_proofs: vec![
                        "PoSpace".to_string(),
                        "PoStake".to_string(),
                        "PoWork".to_string(),
                        "PoTime".to_string(),
                    ],
                    matrix_shards: vec![
                        MatrixShard {
                            shard_id: "shard_003_a".to_string(),
                            position: MatrixPositionDto { x: 3, y: 7, z: 2 },
                            size_bytes: 10485760,
                            redundancy_level: 7,
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
            let response = AssetAllocationResponse {
                asset_id: uuid::Uuid::new_v4().to_string(),
                resource_type: "CPU".to_string(),
                amount_allocated: 4,
                privacy_tier: "Federated".to_string(),
                expires_at: chrono::Utc::now().timestamp() + 3600,
                proxy_address: format!("2001:db8::asset:{}", uuid::Uuid::new_v4().simple()),
                consensus_proofs: vec![
                    "PoSpace".to_string(),
                    "PoStake".to_string(),
                    "PoWork".to_string(),
                    "PoTime".to_string(),
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
                        position: MatrixPositionDto { x: 25, y: 30, z: 0 },
                        size_bytes: 4194304,
                        redundancy_level: 3,
                    },
                    MatrixShard {
                        shard_id: format!("{}_shard_2", asset_id),
                        position: MatrixPositionDto { x: 35, y: 40, z: 0 },
                        size_bytes: 4194304,
                        redundancy_level: 3,
                    },
                    MatrixShard {
                        shard_id: format!("{}_shard_3", asset_id),
                        position: MatrixPositionDto { x: 45, y: 50, z: 0 },
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
        })

        // NEW WEEK 1 ENDPOINTS

        // 2. HyperMesh System Status endpoint with real system data
        .get("/api/v1/hypermesh/system/status", move |_req| {
            let uptime = start_time.elapsed().as_secs();

            async move {
                // Mock system info since sysinfo 0.30 API is different
                // In production with newer sysinfo version, would use real system data
                let cpu_usage = 25.5; // Mock 25.5% CPU usage
                let memory_used_gb = 8.5; // Mock 8.5 GB used
                let memory_total_gb = 32.0; // Mock 32 GB total
                let storage_used_gb = 450.0; // Mock 450 GB used
                let storage_total_gb = 1000.0; // Mock 1 TB total

                let response = HyperMeshSystemStatus {
                    node_id: uuid::Uuid::new_v4().to_string(),
                    status: "operational".to_string(),
                    uptime_seconds: uptime,
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    matrix_position: MatrixPositionDto { x: 10, y: 20, z: 1 },
                    resources: ResourceStatus {
                        cpu_usage_percent: cpu_usage,
                        memory_used_gb,
                        memory_total_gb,
                        storage_used_gb,
                        storage_total_gb,
                    },
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

        // 3. HyperMesh Assets endpoint (alias for blockmatrix/assets)
        .get("/api/v1/hypermesh/assets", |_req| async move {
            // Reuse the same asset list as blockmatrix/assets
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
                            position: MatrixPositionDto { x: 5, y: 10, z: 0 },
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
                            position: MatrixPositionDto { x: 8, y: 12, z: 1 },
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

        // 4. HyperMesh Allocations endpoint
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

                let response = AllocationsResponse {
                    total: allocations.len(),
                    active: active_count,
                    allocations,
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

        // 5. STOQ System Health endpoint
        .get("/api/v1/stoq/system/health", {
            let bytes_sent = bytes_sent.clone();
            let bytes_received = bytes_received.clone();
            let connections = connections.clone();

            move |_req| {
                let bytes_sent = bytes_sent.clone();
                let bytes_received = bytes_received.clone();
                let connections = connections.clone();

                async move {
                    let response = StoqHealthResponse {
                        transport_status: "operational".to_string(),
                        quic_version: "h3-29".to_string(),
                        active_connections: connections.len(),
                        bytes_sent: bytes_sent.load(Ordering::Relaxed),
                        bytes_received: bytes_received.load(Ordering::Relaxed),
                        packet_loss_percent: 0.02, // Mock low packet loss
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
            }
        })

        // 6. STOQ Connections endpoint
        .get("/api/v1/stoq/connections", {
            let connections = connections.clone();

            move |_req| {
                let _connections = connections.clone();

                async move {
                    // Mock some connections
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

                    let response = ConnectionsResponse {
                        total: mock_connections.len(),
                        connections: mock_connections,
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
            }
        })

        // 7. HyperMesh Nodes Health endpoint
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

            let response = NodesHealthResponse {
                total_nodes: nodes.len(),
                healthy_nodes: healthy_count,
                nodes,
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

        // 8. STOQ Performance Metrics endpoint
        .get("/api/v1/stoq/metrics/performance", {
            let request_count = request_count.clone();

            move |_req| {
                let request_count = request_count.clone();

                async move {
                    request_count.fetch_add(1, Ordering::Relaxed);

                    let response = PerformanceMetrics {
                        latency_p50_ms: 12.5,
                        latency_p95_ms: 35.2,
                        latency_p99_ms: 98.7,
                        throughput_mbps: 850.5,
                        requests_per_second: request_count.load(Ordering::Relaxed) as f32 / 60.0, // Rough RPS
                        success_rate_percent: 99.8,
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
            }
        })

        // 9. Byzantine Detections endpoint
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

            let last_24h = detections.iter().filter(|_d| {
                // In production, parse timestamp and check if within 24h
                true // Mock all as within 24h for demo
            }).count();

            let response = DetectionsResponse {
                total: detections.len(),
                last_24h,
                detections,
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

    // Start server on IPv6 localhost port 8446 using STOQ transport
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8446);
    let server = Http3StoqServer::new(addr, router);

    info!("BlockMatrix HTTP/3 server (STOQ transport) starting on https://[::1]:8446");
    server.run().await?;

    Ok(())
}