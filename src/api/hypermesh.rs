//! HyperMesh Asset API endpoints for asset and consensus management

use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use axum::{
    response::Json,
    extract::{State, Path, Query},
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use tracing::{debug, error};

use crate::Internet2Server;
use super::ApiResponse;

/// HyperMesh API handlers
#[derive(Clone)]
pub struct HyperMeshApiHandlers {
    server: Arc<Internet2Server>,
}

/// Asset listing response
#[derive(Debug, Serialize)]
pub struct AssetListResponse {
    pub assets: Vec<AssetSummary>,
    pub total_count: u32,
    pub by_type: HashMap<String, u32>,
    pub by_status: HashMap<String, u32>,
}

/// Asset summary for list responses
#[derive(Debug, Serialize)]
pub struct AssetSummary {
    pub id: String,
    pub name: String,
    pub asset_type: String,
    pub status: String,
    pub privacy_level: String,
    pub total_capacity: f64,
    pub available_capacity: f64,
    pub allocation_percent: f64,
    pub proxy_address: Option<String>,
}

/// Asset creation request
#[derive(Debug, Deserialize)]
pub struct CreateAssetRequest {
    pub name: String,
    pub description: String,
    pub asset_type: String, // Simplified to string for now
    pub privacy_level: String,
    pub specifications: HashMap<String, serde_json::Value>,
    pub total_capacity: f64,
    pub unit: String,
}

/// Asset allocation request
#[derive(Debug, Deserialize)]
pub struct AllocateAssetRequest {
    pub amount: f64,
    pub duration_seconds: Option<u64>,
    pub privacy_requirements: Option<Vec<String>>,
    pub consensus_required: Option<bool>,
}

/// VM execution request
#[derive(Debug, Deserialize)]
pub struct VmExecutionRequest {
    pub vm_asset_id: String,
    pub operation: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timeout_seconds: Option<u64>,
}

/// Consensus statistics response
#[derive(Debug, Serialize)]
pub struct ConsensusStatsResponse {
    pub total_operations: u64,
    pub success_rate: f64,
    pub average_time_ms: f64,
    pub four_proof_validations: u64,
    pub proof_breakdown: ProofBreakdown,
}

/// Four-proof breakdown
#[derive(Debug, Serialize)]
pub struct ProofBreakdown {
    pub proof_of_space: u64,
    pub proof_of_stake: u64,
    pub proof_of_work: u64,
    pub proof_of_time: u64,
}

/// Query parameters for asset listing
#[derive(Debug, Deserialize)]
pub struct AssetQuery {
    pub asset_type: Option<String>,
    pub status: Option<String>,
    pub privacy_level: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl HyperMeshApiHandlers {
    pub fn new(server: Arc<Internet2Server>) -> Self {
        Self { server }
    }
    
    /// GET /api/v1/hypermesh/assets - List all assets
    pub async fn list_assets(
        Query(query): Query<AssetQuery>,
        State(server): State<Arc<Internet2Server>>
    ) -> Result<Json<ApiResponse<AssetListResponse>>, StatusCode> {
        debug!("📋 Listing HyperMesh assets with filters: {:?}", query);
        
        match server.get_statistics().await {
            Ok(stats) => {
                // TODO: Get actual asset list from HyperMesh layer
                // For now, return mock data based on statistics
                
                let assets = vec![
                    AssetSummary {
                        id: "asset-cpu-1".to_string(),
                        name: "System CPU Cores".to_string(),
                        asset_type: "cpu".to_string(),
                        status: "available".to_string(),
                        privacy_level: "private".to_string(),
                        total_capacity: 8.0,
                        available_capacity: 6.0,
                        allocation_percent: 25.0,
                        proxy_address: Some("nat://cpu-1.local".to_string()),
                    },
                    AssetSummary {
                        id: "asset-memory-1".to_string(),
                        name: "System Memory".to_string(),
                        asset_type: "memory".to_string(),
                        status: "available".to_string(),
                        privacy_level: "private".to_string(),
                        total_capacity: 32768.0,
                        available_capacity: 24576.0,
                        allocation_percent: 25.0,
                        proxy_address: Some("nat://memory-1.local".to_string()),
                    },
                    AssetSummary {
                        id: "asset-gpu-1".to_string(),
                        name: "NVIDIA GPU".to_string(),
                        asset_type: "gpu".to_string(),
                        status: "allocated".to_string(),
                        privacy_level: "p2p".to_string(),
                        total_capacity: 1.0,
                        available_capacity: 0.0,
                        allocation_percent: 100.0,
                        proxy_address: Some("nat://gpu-1.local".to_string()),
                    },
                ];
                
                let mut by_type = HashMap::new();
                let mut by_status = HashMap::new();
                
                for asset in &assets {
                    *by_type.entry(asset.asset_type.clone()).or_insert(0) += 1;
                    *by_status.entry(asset.status.clone()).or_insert(0) += 1;
                }
                
                let response = AssetListResponse {
                    total_count: assets.len() as u32,
                    assets,
                    by_type,
                    by_status,
                };
                
                Ok(Json(ApiResponse::success(response)))
            }
            Err(e) => {
                error!("Failed to list assets: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
    
    /// POST /api/v1/hypermesh/assets - Create new asset
    pub async fn create_asset(
        State(_server): State<Arc<Internet2Server>>,
        Json(request): Json<CreateAssetRequest>
    ) -> Result<Json<ApiResponse<AssetSummary>>, StatusCode> {
        debug!("➕ Creating new asset: {}", request.name);
        
        // TODO: Implement actual asset creation through HyperMesh layer
        // For now, return mock created asset
        
        let created_asset = AssetSummary {
            id: format!("asset-{}-{}", request.asset_type.to_lowercase(), uuid::Uuid::new_v4().to_string()[..8].to_string()),
            name: request.name,
            asset_type: request.asset_type.to_lowercase(),
            status: "available".to_string(),
            privacy_level: request.privacy_level.to_lowercase(),
            total_capacity: request.total_capacity,
            available_capacity: request.total_capacity,
            allocation_percent: 0.0,
            proxy_address: Some(format!("nat://{}.local", uuid::Uuid::new_v4().to_string()[..8].to_string())),
        };
        
        Ok(Json(ApiResponse::success(created_asset)))
    }
    
    /// GET /api/v1/hypermesh/assets/:id - Get specific asset
    pub async fn get_asset(
        Path(asset_id): Path<String>,
        State(_server): State<Arc<Internet2Server>>
    ) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
        debug!("🔍 Getting asset: {}", asset_id);
        
        // TODO: Get actual asset from HyperMesh layer
        // For now, return mock asset details
        
        let asset_details = serde_json::json!({
            "id": asset_id,
            "name": "System CPU Cores",
            "description": "Multi-core CPU resource for computation",
            "asset_type": "cpu",
            "status": "available",
            "privacy_level": "private",
            "location": {
                "node_id": "node-internet2-server",
                "address": "::",
                "region": "local"
            },
            "specifications": {
                "cores": 8,
                "threads": 16,
                "architecture": "x86_64",
                "clock_speed_ghz": 3.2
            },
            "allocation": {
                "total_capacity": 8.0,
                "allocated_capacity": 2.0,
                "available_capacity": 6.0,
                "unit": "cores",
                "granularity": 0.5
            },
            "proxy_address": format!("nat://{}.local", asset_id),
            "consensus_proofs": [],
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        });
        
        Ok(Json(ApiResponse::success(asset_details)))
    }
    
    /// PUT /api/v1/hypermesh/assets/:id - Update asset
    pub async fn update_asset(
        Path(asset_id): Path<String>,
        State(_server): State<Arc<Internet2Server>>,
        Json(updates): Json<serde_json::Value>
    ) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
        debug!("✏️ Updating asset: {} with {:?}", asset_id, updates);
        
        // TODO: Implement actual asset update through HyperMesh layer
        
        Ok(Json(ApiResponse::success(serde_json::json!({
            "id": asset_id,
            "updated": true,
            "changes": updates
        }))))
    }
    
    /// DELETE /api/v1/hypermesh/assets/:id - Delete asset
    pub async fn delete_asset(
        Path(asset_id): Path<String>,
        State(_server): State<Arc<Internet2Server>>
    ) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
        debug!("🗑️ Deleting asset: {}", asset_id);
        
        // TODO: Implement actual asset deletion through HyperMesh layer
        
        Ok(Json(ApiResponse::success(serde_json::json!({
            "id": asset_id,
            "deleted": true
        }))))
    }
    
    /// POST /api/v1/hypermesh/assets/:id/allocate - Allocate asset
    pub async fn allocate_asset(
        Path(asset_id): Path<String>,
        State(_server): State<Arc<Internet2Server>>,
        Json(request): Json<AllocateAssetRequest>
    ) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
        debug!("⚡ Allocating asset: {} amount: {}", asset_id, request.amount);
        
        // TODO: Implement actual allocation through HyperMesh layer
        
        let allocation = serde_json::json!({
            "allocation_id": format!("alloc-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            "asset_id": asset_id,
            "amount": request.amount,
            "duration_seconds": request.duration_seconds,
            "status": "active",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "expires_at": request.duration_seconds.map(|d| 
                (chrono::Utc::now() + chrono::Duration::seconds(d as i64)).to_rfc3339()
            )
        });
        
        Ok(Json(ApiResponse::success(allocation)))
    }
    
    /// GET /api/v1/hypermesh/assets/:id/allocation - Get asset allocation
    pub async fn get_allocation(
        Path(asset_id): Path<String>,
        State(_server): State<Arc<Internet2Server>>
    ) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
        debug!("📊 Getting allocation for asset: {}", asset_id);
        
        // TODO: Get actual allocation from HyperMesh layer
        
        let allocation_info = serde_json::json!({
            "asset_id": asset_id,
            "total_capacity": 8.0,
            "allocated_capacity": 2.0,
            "available_capacity": 6.0,
            "unit": "cores",
            "active_allocations": [
                {
                    "allocation_id": "alloc-12345678",
                    "amount": 2.0,
                    "status": "active",
                    "created_at": "2024-01-01T00:00:00Z"
                }
            ]
        });
        
        Ok(Json(ApiResponse::success(allocation_info)))
    }
    
    /// GET /api/v1/hypermesh/allocations - List all allocations
    pub async fn list_allocations(
        State(server): State<Arc<Internet2Server>>
    ) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
        debug!("📋 Listing all allocations");
        
        match server.get_statistics().await {
            Ok(stats) => {
                let allocations = serde_json::json!({
                    "total_allocations": stats.hypermesh_stats.active_allocations,
                    "allocations": [
                        {
                            "allocation_id": "alloc-12345678",
                            "asset_id": "asset-cpu-1",
                            "asset_type": "cpu",
                            "amount": 2.0,
                            "unit": "cores",
                            "status": "active",
                            "created_at": "2024-01-01T00:00:00Z"
                        }
                    ]
                });
                
                Ok(Json(ApiResponse::success(allocations)))
            }
            Err(e) => {
                error!("Failed to list allocations: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
    
    /// GET /api/v1/hypermesh/consensus - Get consensus statistics
    pub async fn get_consensus_stats(
        State(server): State<Arc<Internet2Server>>
    ) -> Result<Json<ApiResponse<ConsensusStatsResponse>>, StatusCode> {
        debug!("📊 Getting consensus statistics");
        
        match server.get_statistics().await {
            Ok(stats) => {
                let consensus_stats = ConsensusStatsResponse {
                    total_operations: stats.hypermesh_stats.consensus_operations,
                    success_rate: stats.hypermesh_stats.consensus_success_rate,
                    average_time_ms: stats.hypermesh_stats.consensus_time_ms,
                    four_proof_validations: stats.hypermesh_stats.consensus_operations,
                    proof_breakdown: ProofBreakdown {
                        proof_of_space: stats.hypermesh_stats.consensus_operations / 4,
                        proof_of_stake: stats.hypermesh_stats.consensus_operations / 4,
                        proof_of_work: stats.hypermesh_stats.consensus_operations / 4,
                        proof_of_time: stats.hypermesh_stats.consensus_operations / 4,
                    },
                };
                
                Ok(Json(ApiResponse::success(consensus_stats)))
            }
            Err(e) => {
                error!("Failed to get consensus statistics: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
    
    /// GET /api/v1/hypermesh/proxy - List proxy connections
    pub async fn list_proxy_connections(
        State(server): State<Arc<Internet2Server>>
    ) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
        debug!("🌐 Listing proxy connections");
        
        match server.get_statistics().await {
            Ok(stats) => {
                let proxy_info = serde_json::json!({
                    "active_connections": stats.hypermesh_stats.proxy_connections,
                    "total_throughput_mbps": stats.hypermesh_stats.proxy_throughput_mbps,
                    "connections": [
                        {
                            "proxy_id": "proxy-12345678",
                            "asset_id": "asset-cpu-1",
                            "proxy_address": "nat://cpu-1.local",
                            "remote_address": "2001:db8::1",
                            "status": "active",
                            "throughput_mbps": 150.0
                        }
                    ]
                });
                
                Ok(Json(ApiResponse::success(proxy_info)))
            }
            Err(e) => {
                error!("Failed to list proxy connections: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
    
    /// POST /api/v1/hypermesh/vm/execute - Execute VM
    pub async fn execute_vm(
        State(_server): State<Arc<Internet2Server>>,
        Json(request): Json<VmExecutionRequest>
    ) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
        debug!("🖥️ Executing VM: {} operation: {}", request.vm_asset_id, request.operation);
        
        // TODO: Implement actual VM execution through HyperMesh layer
        
        let execution = serde_json::json!({
            "execution_id": format!("exec-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            "vm_asset_id": request.vm_asset_id,
            "operation": request.operation,
            "parameters": request.parameters,
            "status": "running",
            "started_at": chrono::Utc::now().to_rfc3339(),
            "estimated_completion": chrono::Utc::now().to_rfc3339()
        });
        
        Ok(Json(ApiResponse::success(execution)))
    }
}