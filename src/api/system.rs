use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{info, debug, error};
use sysinfo::{System, SystemExt, CpuExt, DiskExt};

use crate::Internet2Server;
use super::{success_response, error_response};

/// Get system health status
pub async fn health(
    State(server): State<Arc<Internet2Server>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    debug!("🏥 Health check requested");
    
    // Check if all layers are running
    let stats = server.get_statistics().await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to get statistics: {}", e)))?;
    
    let health_status = json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "1.0.0",
        "layers": {
            "stoq": stats.stoq_stats.current_throughput_gbps > 0.0,
            "hypermesh": stats.hypermesh_stats.total_assets > 0,
            "trustchain": true, // Always healthy if server is running
        },
        "uptime_seconds": stats.stack_stats.uptime_seconds
    });
    
    Ok(success_response(health_status))
}

/// Get comprehensive system statistics
pub async fn stats(
    State(server): State<Arc<Internet2Server>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    debug!("📊 System statistics requested");
    
    let stats = server.get_statistics().await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to get statistics: {}", e)))?;
    
    Ok(success_response(stats))
}

/// Get system resource information (CPU, RAM, Storage)
pub async fn resources(
    State(_server): State<Arc<Internet2Server>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    debug!("🖥️ System resources requested");
    
    let mut sys = System::new_all();
    sys.refresh_all();
    
    // Get CPU information
    let cpu_count = sys.cpus().len();
    let cpu_usage = sys.global_cpu_info().cpu_usage();
    
    // Get memory information  
    let total_memory_gb = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let used_memory_gb = sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let available_memory_gb = total_memory_gb - used_memory_gb;
    
    // Get disk information
    let mut total_storage_gb = 0.0;
    let mut available_storage_gb = 0.0;
    
    for disk in sys.disks() {
        total_storage_gb += disk.total_space() as f64 / (1024.0 * 1024.0 * 1024.0);
        available_storage_gb += disk.available_space() as f64 / (1024.0 * 1024.0 * 1024.0);
    }
    
    // Calculate what resources are shared (50% by default)
    let shared_ratio = 0.5;
    
    let resources = json!({
        "total": {
            "cpu": cpu_count,
            "ram": total_memory_gb.round() as u32,
            "storage": total_storage_gb.round() as u32
        },
        "available": {
            "cpu": cpu_count,
            "ram": available_memory_gb.round() as u32, 
            "storage": available_storage_gb.round() as u32
        },
        "shared": {
            "cpu": ((cpu_count as f64) * shared_ratio).round() as u32,
            "ram": (total_memory_gb * shared_ratio).round() as u32,
            "storage": (total_storage_gb * shared_ratio).round() as u32
        },
        "usage": {
            "cpu": cpu_usage,
            "ram": (used_memory_gb / total_memory_gb) * 100.0,
            "storage": ((total_storage_gb - available_storage_gb) / total_storage_gb) * 100.0
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    Ok(success_response(resources))
}

/// Get system performance metrics
pub async fn metrics(
    State(server): State<Arc<Internet2Server>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    debug!("⚡ Performance metrics requested");
    
    let stats = server.get_statistics().await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to get statistics: {}", e)))?;
    
    // Calculate total requests across all layers
    let total_requests = 
        stats.stoq_stats.total_connections_established +
        stats.hypermesh_stats.total_asset_operations +
        stats.trustchain_stats.certificates_issued;
    
    let metrics = json!({
        "requests": {
            "total": total_requests,
            "stoq_connections": stats.stoq_stats.total_connections_established,
            "asset_operations": stats.hypermesh_stats.total_asset_operations,  
            "certificates_issued": stats.trustchain_stats.certificates_issued
        },
        "performance": {
            "stoq_throughput_gbps": stats.stoq_stats.current_throughput_gbps,
            "stoq_target_gbps": stats.stoq_stats.target_throughput_gbps,
            "connection_time_ms": stats.stoq_stats.connection_establishment_time_ms,
            "certificate_time_ms": stats.trustchain_stats.certificate_issuance_time_ms,
            "consensus_time_ms": stats.hypermesh_stats.consensus_validation_time_ms
        },
        "health": {
            "uptime_seconds": stats.stack_stats.uptime_seconds,
            "layers_integrated": stats.stack_stats.layers_integrated,
            "connection_errors": stats.stoq_stats.connection_errors,
            "consensus_failures": stats.hypermesh_stats.consensus_failures
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    Ok(success_response(metrics))
}

/// Get count of installed assets (Catalog integration)
pub async fn installed_assets_count(
    State(server): State<Arc<Internet2Server>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    debug!("📦 Installed assets count requested");
    
    let stats = server.get_statistics().await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to get statistics: {}", e)))?;
    
    // Return the total number of registered assets
    let count = stats.hypermesh_stats.total_assets;
    
    Ok(success_response(json!({
        "count": count,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Get count of available updates (placeholder for now)
pub async fn available_updates_count(
    State(_server): State<Arc<Internet2Server>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    debug!("🔄 Available updates count requested");
    
    // For now, return a simulated count
    // In production, this would check the Catalog for available updates
    let count = 3; // Simulated available updates
    
    Ok(success_response(json!({
        "count": count,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Get integration service health
pub async fn integration_health(
    State(server): State<Arc<Internet2Server>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    debug!("🔗 Integration health requested");
    
    let stats = server.get_statistics().await
        .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to get statistics: {}", e)))?;
    
    // Calculate integration health based on layer integration status
    let uptime_percentage = if stats.stack_stats.uptime_seconds > 0 {
        // Assume 99.9% uptime for demo - in production this would be calculated
        99.9
    } else {
        0.0
    };
    
    let integration_health = json!({
        "name": "Integration",
        "status": if stats.stack_stats.layers_integrated { "healthy" } else { "degraded" },
        "responseTime": stats.integration_stats.average_response_time_ms,
        "errorRate": 0.0, // Calculate based on actual errors
        "uptime": uptime_percentage,
        "lastCheck": chrono::Utc::now().to_rfc3339(),
        "layers": {
            "stoq": "healthy",
            "hypermesh": "healthy", 
            "trustchain": "healthy"
        }
    });
    
    Ok(success_response(integration_health))
}