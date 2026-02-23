// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HTTP/3 route handler DTOs and response builders for the BlockMatrix server.

use http::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use dashmap::DashMap;

use super::ApiResponse;

// ---------------------------------------------------------------------------
// DTO types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub endpoints_available: usize,
    pub matrix_nodes: usize,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub node_id: String,
    pub matrix_position: MatrixPositionDto,
    pub blockchain_height: u64,
    pub peers_connected: usize,
    pub assets_managed: u64,
    pub storage_gb: f64,
    pub cpu_cores: usize,
    pub gpu_available: bool,
}

/// Local DTO for HTTP/3 API responses; canonical MatrixPosition in hypermesh_lib.
#[derive(Serialize, Deserialize)]
pub struct MatrixPositionDto {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Deserialize)]
pub struct AssetAllocationRequest {
    #[serde(rename = "resource_type")]
    pub _resource_type: String,
    #[serde(rename = "amount")]
    pub _amount: u64,
    #[serde(rename = "privacy_tier")]
    pub _privacy_tier: String,
    #[serde(rename = "duration_seconds")]
    pub _duration_seconds: u64,
}

#[derive(Serialize)]
pub struct AssetAllocationResponse {
    pub asset_id: String,
    pub resource_type: String,
    pub amount_allocated: u64,
    pub privacy_tier: String,
    pub expires_at: i64,
    pub proxy_address: String,
    pub consensus_proofs: Vec<String>,
}

#[derive(Serialize)]
pub struct AssetInfo {
    pub asset_id: String,
    pub owner: String,
    pub resource_type: String,
    pub status: String,
    pub created_at: i64,
    pub privacy_tier: String,
    pub proxy_address: String,
    pub consensus_proofs: Vec<String>,
    pub matrix_shards: Vec<MatrixShard>,
}

#[derive(Serialize)]
pub struct MatrixShard {
    pub shard_id: String,
    pub position: MatrixPositionDto,
    pub size_bytes: u64,
    pub redundancy_level: u32,
}

#[derive(Serialize)]
pub struct AssetListResponse {
    pub assets: Vec<AssetInfo>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

#[derive(Serialize)]
pub struct HyperMeshSystemStatus {
    pub node_id: String,
    pub status: String,
    pub uptime_seconds: u64,
    pub version: String,
    pub matrix_position: MatrixPositionDto,
    pub resources: ResourceStatus,
}

#[derive(Serialize)]
pub struct ResourceStatus {
    pub cpu_usage_percent: f32,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
    pub storage_used_gb: f32,
    pub storage_total_gb: f32,
}

#[derive(Serialize)]
pub struct AllocationInfo {
    pub allocation_id: String,
    pub asset_id: String,
    pub resource_type: String,
    pub amount: u32,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Serialize)]
pub struct AllocationsResponse {
    pub allocations: Vec<AllocationInfo>,
    pub total: usize,
    pub active: usize,
}

#[derive(Serialize)]
pub struct StoqHealthResponse {
    pub transport_status: String,
    pub quic_version: String,
    pub active_connections: usize,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packet_loss_percent: f32,
}

#[derive(Serialize, Clone)]
pub struct ConnectionInfo {
    pub connection_id: String,
    pub remote_addr: String,
    pub status: String,
    pub uptime_seconds: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Serialize)]
pub struct ConnectionsResponse {
    pub connections: Vec<ConnectionInfo>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct NodeHealth {
    pub node_id: String,
    pub status: String,
    pub last_seen: String,
    pub latency_ms: f32,
    pub matrix_position: MatrixPositionDto,
}

#[derive(Serialize)]
pub struct NodesHealthResponse {
    pub nodes: Vec<NodeHealth>,
    pub total_nodes: usize,
    pub healthy_nodes: usize,
}

#[derive(Serialize)]
pub struct PerformanceMetrics {
    pub latency_p50_ms: f32,
    pub latency_p95_ms: f32,
    pub latency_p99_ms: f32,
    pub throughput_mbps: f32,
    pub requests_per_second: f32,
    pub success_rate_percent: f32,
}

#[derive(Serialize)]
pub struct ByzantineDetection {
    pub detection_id: String,
    pub node_id: String,
    pub detection_type: String,
    pub severity: String,
    pub detected_at: String,
    pub evidence: String,
}

#[derive(Serialize)]
pub struct DetectionsResponse {
    pub detections: Vec<ByzantineDetection>,
    pub total: usize,
    pub last_24h: usize,
}

// ---------------------------------------------------------------------------
// Shared server state
// ---------------------------------------------------------------------------

/// Shared state for the HTTP/3 server metrics and connections.
pub struct ServerState {
    pub start_time: std::time::Instant,
    pub bytes_sent: Arc<AtomicU64>,
    pub bytes_received: Arc<AtomicU64>,
    pub request_count: Arc<AtomicU64>,
    pub connections: Arc<DashMap<String, ConnectionInfo>>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            request_count: Arc::new(AtomicU64::new(0)),
            connections: Arc::new(DashMap::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// JSON response helper
// ---------------------------------------------------------------------------

/// Build a JSON success response from any serialisable payload.
pub fn json_success_response<T: Serialize>(payload: T) -> Response<Vec<u8>> {
    let body = serde_json::to_vec(&ApiResponse::success(
        payload,
        uuid::Uuid::new_v4().to_string(),
    ))
    .unwrap_or_default();

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(body)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

// ---------------------------------------------------------------------------
// Mock data builders (used by multiple routes)
// ---------------------------------------------------------------------------

/// Standard four consensus proof strings.
pub fn consensus_proof_strings() -> Vec<String> {
    vec![
        "PoSpace".to_string(),
        "PoStake".to_string(),
        "PoWork".to_string(),
        "PoTime".to_string(),
    ]
}

/// Build the default sample asset list used by both blockmatrix and hypermesh asset endpoints.
pub fn sample_asset_list(full: bool) -> Vec<AssetInfo> {
    let mut assets = vec![
        AssetInfo {
            asset_id: "asset_001".to_string(),
            owner: "0xabcd...1234".to_string(),
            resource_type: "CPU".to_string(),
            status: "active".to_string(),
            created_at: chrono::Utc::now().timestamp() - 3600,
            privacy_tier: "Federated".to_string(),
            proxy_address: "2001:db8::cpu:1".to_string(),
            consensus_proofs: consensus_proof_strings(),
            matrix_shards: vec![MatrixShard {
                shard_id: "shard_001_a".to_string(),
                position: MatrixPositionDto { x: 5, y: 10, z: 0 },
                size_bytes: 1048576,
                redundancy_level: 3,
            }],
        },
        AssetInfo {
            asset_id: "asset_002".to_string(),
            owner: "0xefgh...5678".to_string(),
            resource_type: "GPU".to_string(),
            status: "active".to_string(),
            created_at: chrono::Utc::now().timestamp() - 7200,
            privacy_tier: "Public".to_string(),
            proxy_address: "2001:db8::gpu:1".to_string(),
            consensus_proofs: consensus_proof_strings(),
            matrix_shards: vec![MatrixShard {
                shard_id: "shard_002_a".to_string(),
                position: MatrixPositionDto { x: 8, y: 12, z: 1 },
                size_bytes: 2097152,
                redundancy_level: 5,
            }],
        },
    ];

    if full {
        // Extra shard on first asset + third asset only in "full" mode
        assets[0].matrix_shards.push(MatrixShard {
            shard_id: "shard_001_b".to_string(),
            position: MatrixPositionDto { x: 15, y: 20, z: 0 },
            size_bytes: 1048576,
            redundancy_level: 3,
        });

        assets.push(AssetInfo {
            asset_id: "asset_003".to_string(),
            owner: "0xijkl...9012".to_string(),
            resource_type: "Storage".to_string(),
            status: "active".to_string(),
            created_at: chrono::Utc::now().timestamp() - 1800,
            privacy_tier: "Private".to_string(),
            proxy_address: "2001:db8::storage:1".to_string(),
            consensus_proofs: consensus_proof_strings(),
            matrix_shards: vec![MatrixShard {
                shard_id: "shard_003_a".to_string(),
                position: MatrixPositionDto { x: 3, y: 7, z: 2 },
                size_bytes: 10485760,
                redundancy_level: 7,
            }],
        });
    }

    assets
}
