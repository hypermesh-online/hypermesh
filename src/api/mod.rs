use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};
use tracing::{info, debug, error};

use crate::{Internet2Server, Internet2Config};

pub mod trustchain;
pub mod hypermesh;
pub mod stoq;
pub mod system;
pub mod caesar;
pub mod search;

/// HTTP REST API server for UI management interface
pub struct HttpApiServer {
    router: Router,
    server: Arc<Internet2Server>,
}

impl HttpApiServer {
    /// Create new HTTP API server
    pub fn new(server: Arc<Internet2Server>) -> Self {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let router = Router::new()
            // System endpoints
            .route("/api/v1/system/health", get(system::health))
            .route("/api/v1/system/stats", get(system::stats))
            .route("/api/v1/system/resources", get(system::resources))
            .route("/api/v1/system/metrics", get(system::metrics))
            
            // TrustChain endpoints
            .route("/api/v1/trustchain/certificates", get(trustchain::list_certificates))
            .route("/api/v1/trustchain/certificates", post(trustchain::create_certificate))
            .route("/api/v1/trustchain/certificates/:id", get(trustchain::get_certificate))
            .route("/api/v1/trustchain/trust-score", get(trustchain::trust_score))
            .route("/api/v1/trustchain/networks/count", get(trustchain::network_count))
            
            // HyperMesh endpoints
            .route("/api/v1/hypermesh/assets", get(hypermesh::list_assets))
            .route("/api/v1/hypermesh/assets", post(hypermesh::create_asset))
            .route("/api/v1/hypermesh/assets/:id", get(hypermesh::get_asset))
            .route("/api/v1/hypermesh/assets/:id", put(hypermesh::update_asset))
            .route("/api/v1/hypermesh/assets/:id", delete(hypermesh::delete_asset))
            .route("/api/v1/hypermesh/assets/:id/allocation", put(hypermesh::update_allocation))
            .route("/api/v1/hypermesh/consensus/proofs", get(hypermesh::consensus_proofs))
            .route("/api/v1/hypermesh/proxy/addresses", get(hypermesh::proxy_addresses))
            
            // STOQ endpoints  
            .route("/api/v1/stoq/connections", get(stoq::list_connections))
            .route("/api/v1/stoq/performance", get(stoq::performance_metrics))
            .route("/api/v1/stoq/optimization", get(stoq::optimization_status))
            
            // Caesar endpoints (token/economic layer)
            .route("/api/v1/caesar/balance", get(caesar::get_balance))
            .route("/api/v1/caesar/earnings/today", get(caesar::earnings_today))
            .route("/api/v1/caesar/earnings/breakdown", get(caesar::earnings_breakdown))
            .route("/api/v1/caesar/pending", get(caesar::pending_transactions))
            .route("/api/v1/caesar/staking/amount", get(caesar::staking_amount))
            .route("/api/v1/caesar/wallet/summary", get(caesar::wallet_summary))
            .route("/api/v1/caesar/transactions", get(caesar::list_transactions))
            .route("/api/v1/caesar/reward-rates", get(caesar::reward_rates))
            .route("/api/v1/caesar/projections", get(caesar::projections))
            
            // Search endpoints
            .route("/api/v1/search", get(search::search))
            .route("/api/v1/search/suggestions", get(search::suggestions))
            
            // Catalog endpoints
            .route("/api/v1/catalog/installed/count", get(system::installed_assets_count))
            .route("/api/v1/catalog/updates/count", get(system::available_updates_count))
            
            // Integration health
            .route("/api/v1/integration/health", get(system::integration_health))
            
            // WebSocket endpoint for real-time updates
            .route("/ws", get(websocket_handler))
            
            .layer(cors)
            .with_state(server.clone());

        Self {
            router,
            server,
        }
    }

    /// Get the Axum router
    pub fn router(self) -> Router {
        self.router
    }
}

/// WebSocket handler for real-time updates
async fn websocket_handler(
    ws: axum::extract::WebSocketUpgrade,
    State(server): State<Arc<Internet2Server>>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, server))
}

/// Handle WebSocket connection for real-time updates
async fn handle_websocket(
    mut socket: axum::extract::ws::WebSocket,
    server: Arc<Internet2Server>,
) {
    info!("🔗 WebSocket connection established for real-time updates");
    
    // Send initial system status
    if let Ok(stats) = server.get_statistics().await {
        let message = json!({
            "type": "system_stats",
            "data": stats
        });
        
        if let Ok(text) = serde_json::to_string(&message) {
            if socket.send(axum::extract::ws::Message::Text(text)).await.is_err() {
                return;
            }
        }
    }
    
    // Keep connection alive and send periodic updates
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        // Send updated statistics
        if let Ok(stats) = server.get_statistics().await {
            let message = json!({
                "type": "system_stats_update", 
                "data": stats,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            
            if let Ok(text) = serde_json::to_string(&message) {
                if socket.send(axum::extract::ws::Message::Text(text)).await.is_err() {
                    info!("📡 WebSocket connection closed");
                    break;
                }
            }
        }
    }
}

/// Error response helper
pub fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({
        "error": true,
        "message": message,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Success response helper
pub fn success_response<T: serde::Serialize>(data: T) -> Json<Value> {
    Json(json!({
        "success": true,
        "data": data,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}