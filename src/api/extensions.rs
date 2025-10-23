//! Extension Management API endpoints for HyperMesh
//!
//! This module provides RESTful API endpoints for managing extensions
//! in the HyperMesh ecosystem.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, error, info};

use crate::extensions::{
    manager::{UnifiedExtensionManager, ExtensionInfo},
    ExtensionRequest, ExtensionResponse, ExtensionCapability,
    ExtensionCategory, ExtensionMetadata,
};

/// API state containing the extension manager
#[derive(Clone)]
pub struct ExtensionApiState {
    pub manager: Arc<UnifiedExtensionManager>,
}

/// Create extension API router
pub fn create_extension_router(state: ExtensionApiState) -> Router {
    Router::new()
        .route("/extensions", get(list_extensions))
        .route("/extensions/:id", get(get_extension))
        .route("/extensions/:id/load", post(load_extension))
        .route("/extensions/:id/unload", post(unload_extension))
        .route("/extensions/:id/reload", post(reload_extension))
        .route("/extensions/:id/pause", post(pause_extension))
        .route("/extensions/:id/resume", post(resume_extension))
        .route("/extensions/:id/request", post(handle_extension_request))
        .route("/extensions/:id/validate", post(validate_extension))
        .route("/extensions/:id/status", get(get_extension_status))
        .route("/extensions/metrics", get(get_extension_metrics))
        .route("/extensions/search", get(search_extensions))
        .route("/extensions/install", post(install_extension))
        .route("/extensions/:id/configure", put(configure_extension))
        .with_state(state)
}

/// List extensions query parameters
#[derive(Debug, Deserialize)]
pub struct ListExtensionsQuery {
    /// Filter by category
    pub category: Option<String>,
    /// Include detailed information
    pub detailed: Option<bool>,
    /// Page number for pagination
    pub page: Option<u32>,
    /// Page size for pagination
    pub page_size: Option<u32>,
}

/// List all loaded extensions
async fn list_extensions(
    State(state): State<ExtensionApiState>,
    Query(params): Query<ListExtensionsQuery>,
) -> Result<Json<ListExtensionsResponse>, StatusCode> {
    debug!("Listing extensions with params: {:?}", params);

    let extensions = state.manager.list_extensions().await;

    // Filter by category if specified
    let filtered: Vec<ExtensionInfo> = if let Some(category) = params.category {
        extensions
            .into_iter()
            .filter(|e| {
                format!("{:?}", e.metadata.category)
                    .to_lowercase()
                    .contains(&category.to_lowercase())
            })
            .collect()
    } else {
        extensions
    };

    // Apply pagination
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(filtered.len());

    let paginated = filtered[start..end].to_vec();

    Ok(Json(ListExtensionsResponse {
        extensions: paginated,
        total: filtered.len(),
        page,
        page_size,
    }))
}

/// List extensions response
#[derive(Debug, Serialize)]
pub struct ListExtensionsResponse {
    pub extensions: Vec<ExtensionInfo>,
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
}

/// Get specific extension information
async fn get_extension(
    State(state): State<ExtensionApiState>,
    Path(id): Path<String>,
) -> Result<Json<ExtensionInfo>, StatusCode> {
    debug!("Getting extension info for: {}", id);

    state
        .manager
        .get_extension_info(&id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Load extension request
#[derive(Debug, Deserialize)]
pub struct LoadExtensionRequest {
    /// Extension path or manifest
    pub source: String,
    /// Force load even if already loaded
    pub force: Option<bool>,
    /// Skip signature verification
    pub skip_verification: Option<bool>,
    /// Extension configuration
    pub config: Option<serde_json::Value>,
}

/// Load an extension
async fn load_extension(
    State(state): State<ExtensionApiState>,
    Path(id): Path<String>,
    Json(request): Json<LoadExtensionRequest>,
) -> Result<StatusCode, StatusCode> {
    info!("Loading extension: {} from {}", id, request.source);

    // TODO: Implement actual extension loading
    // This would involve:
    // 1. Loading the extension from the source
    // 2. Creating the extension instance
    // 3. Loading it into the manager

    Ok(StatusCode::OK)
}

/// Unload an extension
async fn unload_extension(
    State(state): State<ExtensionApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    info!("Unloading extension: {}", id);

    state
        .manager
        .unload_extension(&id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|e| {
            error!("Failed to unload extension {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Reload an extension
async fn reload_extension(
    State(state): State<ExtensionApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    info!("Reloading extension: {}", id);

    state
        .manager
        .reload_extension(&id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|e| {
            error!("Failed to reload extension {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Pause an extension
async fn pause_extension(
    State(state): State<ExtensionApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    info!("Pausing extension: {}", id);

    state
        .manager
        .pause_extension(&id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|e| {
            error!("Failed to pause extension {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Resume an extension
async fn resume_extension(
    State(state): State<ExtensionApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    info!("Resuming extension: {}", id);

    state
        .manager
        .resume_extension(&id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|e| {
            error!("Failed to resume extension {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Handle extension request
async fn handle_extension_request(
    State(state): State<ExtensionApiState>,
    Path(id): Path<String>,
    Json(request): Json<ExtensionRequest>,
) -> Result<Json<ExtensionResponse>, StatusCode> {
    debug!("Handling request for extension {}: {:?}", id, request);

    state
        .manager
        .handle_request(&id, request)
        .await
        .map(Json)
        .map_err(|e| {
            error!("Failed to handle extension request: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Validate an extension
async fn validate_extension(
    State(state): State<ExtensionApiState>,
    Path(id): Path<String>,
) -> Result<Json<ValidationResponse>, StatusCode> {
    info!("Validating extension: {}", id);

    let reports = state.manager.validate_all_extensions().await;

    reports
        .get(&id)
        .cloned()
        .map(|report| {
            Json(ValidationResponse {
                valid: report.valid,
                errors: report
                    .errors
                    .into_iter()
                    .map(|e| format!("{}: {}", e.code, e.message))
                    .collect(),
                warnings: report
                    .warnings
                    .into_iter()
                    .map(|w| format!("{}: {}", w.code, w.message))
                    .collect(),
            })
        })
        .ok_or(StatusCode::NOT_FOUND)
}

/// Validation response
#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Get extension status
async fn get_extension_status(
    State(state): State<ExtensionApiState>,
    Path(id): Path<String>,
) -> Result<Json<ExtensionStatusResponse>, StatusCode> {
    debug!("Getting status for extension: {}", id);

    state
        .manager
        .get_extension_info(&id)
        .await
        .map(|info| {
            Json(ExtensionStatusResponse {
                id: info.metadata.id,
                name: info.metadata.name,
                version: info.metadata.version.to_string(),
                state: format!("{:?}", info.state.state),
                health: format!("{:?}", info.state.health),
                request_count: info.state.request_count,
                error_count: info.state.error_count,
                cpu_usage: info.state.resource_usage.cpu_percent,
                memory_usage: info.state.resource_usage.memory_bytes,
            })
        })
        .ok_or(StatusCode::NOT_FOUND)
}

/// Extension status response
#[derive(Debug, Serialize)]
pub struct ExtensionStatusResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub state: String,
    pub health: String,
    pub request_count: u64,
    pub error_count: u64,
    pub cpu_usage: f32,
    pub memory_usage: u64,
}

/// Get extension metrics
async fn get_extension_metrics(
    State(state): State<ExtensionApiState>,
) -> Json<MetricsResponse> {
    let metrics = state.manager.get_metrics().await;

    Json(MetricsResponse {
        total_loaded: metrics.total_loaded,
        total_failed: metrics.total_failed,
        total_requests: metrics.total_requests,
        total_errors: metrics.total_errors,
        avg_request_duration_ms: metrics.avg_request_duration.as_millis() as u64,
        peak_memory_mb: metrics.peak_memory / 1024 / 1024,
        peak_cpu_percent: metrics.peak_cpu,
    })
}

/// Metrics response
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub total_loaded: usize,
    pub total_failed: usize,
    pub total_requests: u64,
    pub total_errors: u64,
    pub avg_request_duration_ms: u64,
    pub peak_memory_mb: u64,
    pub peak_cpu_percent: f32,
}

/// Search extensions query parameters
#[derive(Debug, Deserialize)]
pub struct SearchExtensionsQuery {
    /// Search query string
    pub q: String,
    /// Filter by category
    pub category: Option<String>,
    /// Minimum version
    pub min_version: Option<String>,
    /// Maximum results
    pub limit: Option<usize>,
}

/// Search for extensions in marketplace
async fn search_extensions(
    State(state): State<ExtensionApiState>,
    Query(params): Query<SearchExtensionsQuery>,
) -> Result<Json<SearchExtensionsResponse>, StatusCode> {
    info!("Searching for extensions with query: {}", params.q);

    // TODO: Implement marketplace search
    // This would query the extension marketplace API

    Ok(Json(SearchExtensionsResponse {
        results: vec![],
        total: 0,
    }))
}

/// Search extensions response
#[derive(Debug, Serialize)]
pub struct SearchExtensionsResponse {
    pub results: Vec<ExtensionSearchResult>,
    pub total: usize,
}

/// Extension search result
#[derive(Debug, Serialize)]
pub struct ExtensionSearchResult {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub downloads: u64,
    pub rating: f32,
}

/// Install extension request
#[derive(Debug, Deserialize)]
pub struct InstallExtensionRequest {
    /// Package name or URL
    pub package: String,
    /// Version to install
    pub version: Option<String>,
    /// Include optional dependencies
    pub with_optional: Option<bool>,
}

/// Install an extension from marketplace
async fn install_extension(
    State(state): State<ExtensionApiState>,
    Json(request): Json<InstallExtensionRequest>,
) -> Result<Json<InstallExtensionResponse>, StatusCode> {
    info!("Installing extension: {}", request.package);

    // TODO: Implement marketplace installation
    // This would:
    // 1. Download the extension from marketplace
    // 2. Verify signatures
    // 3. Install and load the extension

    Ok(Json(InstallExtensionResponse {
        id: "placeholder".to_string(),
        version: "1.0.0".to_string(),
        installed_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// Install extension response
#[derive(Debug, Serialize)]
pub struct InstallExtensionResponse {
    pub id: String,
    pub version: String,
    pub installed_at: String,
}

/// Configure extension request
#[derive(Debug, Deserialize)]
pub struct ConfigureExtensionRequest {
    /// Configuration settings
    pub settings: serde_json::Value,
    /// Resource limits
    pub resource_limits: Option<ResourceLimitsConfig>,
    /// Granted capabilities
    pub capabilities: Option<Vec<String>>,
}

/// Resource limits configuration
#[derive(Debug, Deserialize, Serialize)]
pub struct ResourceLimitsConfig {
    pub max_cpu_percent: Option<f32>,
    pub max_memory_mb: Option<u64>,
    pub max_storage_mb: Option<u64>,
    pub max_concurrent_operations: Option<usize>,
}

/// Configure an extension
async fn configure_extension(
    State(state): State<ExtensionApiState>,
    Path(id): Path<String>,
    Json(request): Json<ConfigureExtensionRequest>,
) -> Result<StatusCode, StatusCode> {
    info!("Configuring extension: {}", id);

    // TODO: Implement extension configuration
    // This would update the extension's configuration

    Ok(StatusCode::OK)
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "extension-api",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Extension WebSocket support for real-time events
pub mod websocket {
    use axum::{
        extract::{
            ws::{Message, WebSocket, WebSocketUpgrade},
            State,
        },
        response::Response,
    };
    use futures::{sink::SinkExt, stream::StreamExt};
    use serde::{Deserialize, Serialize};
    use tracing::{debug, error};

    use super::ExtensionApiState;

    /// WebSocket message types
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "type")]
    pub enum WsMessage {
        /// Subscribe to extension events
        Subscribe { extension_id: String },
        /// Unsubscribe from extension events
        Unsubscribe { extension_id: String },
        /// Extension event
        Event {
            extension_id: String,
            event: ExtensionEvent,
        },
        /// Error message
        Error { message: String },
    }

    /// Extension events
    #[derive(Debug, Serialize, Deserialize)]
    pub enum ExtensionEvent {
        StateChanged { new_state: String },
        HealthChanged { new_health: String },
        RequestCompleted { request_id: String },
        Error { error: String },
    }

    /// WebSocket handler for extension events
    pub async fn websocket_handler(
        ws: WebSocketUpgrade,
        State(state): State<ExtensionApiState>,
    ) -> Response {
        ws.on_upgrade(move |socket| handle_socket(socket, state))
    }

    async fn handle_socket(mut socket: WebSocket, state: ExtensionApiState) {
        debug!("WebSocket connection established");

        while let Some(msg) = socket.recv().await {
            if let Ok(msg) = msg {
                match msg {
                    Message::Text(text) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            // Handle WebSocket message
                            match ws_msg {
                                WsMessage::Subscribe { extension_id } => {
                                    debug!("Subscribing to extension: {}", extension_id);
                                    // TODO: Implement subscription logic
                                }
                                WsMessage::Unsubscribe { extension_id } => {
                                    debug!("Unsubscribing from extension: {}", extension_id);
                                    // TODO: Implement unsubscription logic
                                }
                                _ => {
                                    // Send error for unexpected message types
                                    let error_msg = WsMessage::Error {
                                        message: "Unexpected message type".to_string(),
                                    };
                                    if let Ok(json) = serde_json::to_string(&error_msg) {
                                        let _ = socket.send(Message::Text(json)).await;
                                    }
                                }
                            }
                        }
                    }
                    Message::Close(_) => {
                        debug!("WebSocket connection closed");
                        break;
                    }
                    _ => {}
                }
            } else {
                error!("WebSocket error");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_extensions_query() {
        let query = ListExtensionsQuery {
            category: Some("AssetLibrary".to_string()),
            detailed: Some(true),
            page: Some(1),
            page_size: Some(20),
        };

        assert_eq!(query.category, Some("AssetLibrary".to_string()));
        assert_eq!(query.detailed, Some(true));
        assert_eq!(query.page, Some(1));
        assert_eq!(query.page_size, Some(20));
    }

    #[test]
    fn test_search_extensions_query() {
        let query = SearchExtensionsQuery {
            q: "catalog".to_string(),
            category: Some("AssetLibrary".to_string()),
            min_version: Some("1.0.0".to_string()),
            limit: Some(10),
        };

        assert_eq!(query.q, "catalog");
        assert_eq!(query.category, Some("AssetLibrary".to_string()));
        assert_eq!(query.min_version, Some("1.0.0".to_string()));
        assert_eq!(query.limit, Some(10));
    }
}