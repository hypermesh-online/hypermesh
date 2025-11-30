//! Extension Management API handlers for HyperMesh using STOQ protocol
//!
//! This module provides STOQ API handlers for managing extensions
//! in the HyperMesh ecosystem.

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info};

use stoq::{ApiHandler, ApiRequest, ApiResponse, ApiError};

use crate::extensions::{
    manager::{UnifiedExtensionManager, ExtensionInfo},
    ExtensionRequest, ExtensionResponse, ExtensionCapability,
    ExtensionCategory, ExtensionMetadata,
};

/// Helper to create a success JSON API response
fn json_response(request_id: String, value: serde_json::Value) -> ApiResponse {
    ApiResponse {
        request_id,
        success: true,
        payload: Bytes::from(serde_json::to_vec(&value).unwrap_or_default()),
        error: None,
        metadata: HashMap::new(),
    }
}

/// Helper to create an empty success response
fn success_response(request_id: String) -> ApiResponse {
    ApiResponse {
        request_id,
        success: true,
        payload: Bytes::from(r#"{"status":"success"}"#),
        error: None,
        metadata: HashMap::new(),
    }
}

/// Create extension handlers for STOQ API
pub fn create_extension_handlers(
    manager: Arc<UnifiedExtensionManager>
) -> Vec<Arc<dyn ApiHandler>> {
    vec![
        Arc::new(ListExtensionsHandler { manager: Arc::clone(&manager) }),
        Arc::new(GetExtensionHandler { manager: Arc::clone(&manager) }),
        Arc::new(LoadExtensionHandler { manager: Arc::clone(&manager) }),
        Arc::new(UnloadExtensionHandler { manager: Arc::clone(&manager) }),
        Arc::new(ReloadExtensionHandler { manager: Arc::clone(&manager) }),
        Arc::new(PauseExtensionHandler { manager: Arc::clone(&manager) }),
        Arc::new(ResumeExtensionHandler { manager: Arc::clone(&manager) }),
        Arc::new(HandleExtensionRequestHandler { manager: Arc::clone(&manager) }),
        Arc::new(ValidateExtensionHandler { manager: Arc::clone(&manager) }),
        Arc::new(ExtensionStatusHandler { manager: Arc::clone(&manager) }),
        Arc::new(ExtensionMetricsHandler { manager: Arc::clone(&manager) }),
        Arc::new(HealthCheckHandler),
    ]
}

/// List extensions handler
struct ListExtensionsHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[async_trait::async_trait]
impl ApiHandler for ListExtensionsHandler {
    fn path(&self) -> &str {
        "/api/extensions"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Listing extensions");

        let extensions = self.manager.list_extensions().await;

        // Parse query parameters from metadata if provided
        let category = req.metadata.get("category");
        let page = req.metadata
            .get("page")
            .and_then(|p| p.parse::<u32>().ok())
            .unwrap_or(1)
            .max(1);
        let page_size = req.metadata
            .get("page_size")
            .and_then(|p| p.parse::<u32>().ok())
            .unwrap_or(20)
            .min(100);

        // Filter by category if specified
        let filtered: Vec<ExtensionInfo> = if let Some(category) = category {
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
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());
        let paginated = filtered[start..end].to_vec();

        Ok(json_response(req.id, json!({
            "extensions": paginated,
            "total": filtered.len(),
            "page": page,
            "page_size": page_size,
        })))
    }
}

/// Get specific extension handler
struct GetExtensionHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[async_trait::async_trait]
impl ApiHandler for GetExtensionHandler {
    fn path(&self) -> &str {
        "/api/extensions/:id"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let id = req.metadata.get("id")
            .ok_or_else(|| ApiError::InvalidRequest("Missing extension ID".to_string()))?;

        debug!("Getting extension info for: {}", id);

        let info = self.manager
            .get_extension_info(id)
            .await
            .ok_or_else(|| ApiError::NotFound(format!("Extension not found: {}", id)))?;

        Ok(json_response(req.id, serde_json::to_value(info).unwrap()))
    }
}

/// Load extension handler
struct LoadExtensionHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[derive(Debug, Deserialize)]
struct LoadExtensionRequest {
    source: String,
    force: Option<bool>,
    skip_verification: Option<bool>,
    config: Option<serde_json::Value>,
}

#[async_trait::async_trait]
impl ApiHandler for LoadExtensionHandler {
    fn path(&self) -> &str {
        "/api/extensions/:id/load"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let id = req.metadata.get("id")
            .ok_or_else(|| ApiError::InvalidRequest("Missing extension ID".to_string()))?;

        let load_req: LoadExtensionRequest = serde_json::from_slice(&req.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid request: {}", e)))?;

        info!("Loading extension: {} from {}", id, load_req.source);

        // TODO: Implement actual extension loading
        Ok(success_response(req.id))
    }
}

/// Unload extension handler
struct UnloadExtensionHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[async_trait::async_trait]
impl ApiHandler for UnloadExtensionHandler {
    fn path(&self) -> &str {
        "/api/extensions/:id/unload"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let id = req.metadata.get("id")
            .ok_or_else(|| ApiError::InvalidRequest("Missing extension ID".to_string()))?;

        info!("Unloading extension: {}", id);

        self.manager
            .unload_extension(id)
            .await
            .map_err(|e| {
                error!("Failed to unload extension {}: {}", id, e);
                ApiError::HandlerError(format!("Failed to unload extension: {}", e))
            })?;

        Ok(success_response(req.id))
    }
}

/// Reload extension handler
struct ReloadExtensionHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[async_trait::async_trait]
impl ApiHandler for ReloadExtensionHandler {
    fn path(&self) -> &str {
        "/api/extensions/:id/reload"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let id = req.metadata.get("id")
            .ok_or_else(|| ApiError::InvalidRequest("Missing extension ID".to_string()))?;

        info!("Reloading extension: {}", id);

        self.manager
            .reload_extension(id)
            .await
            .map_err(|e| {
                error!("Failed to reload extension {}: {}", id, e);
                ApiError::HandlerError(format!("Failed to reload extension: {}", e))
            })?;

        Ok(success_response(req.id))
    }
}

/// Pause extension handler
struct PauseExtensionHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[async_trait::async_trait]
impl ApiHandler for PauseExtensionHandler {
    fn path(&self) -> &str {
        "/api/extensions/:id/pause"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let id = req.metadata.get("id")
            .ok_or_else(|| ApiError::InvalidRequest("Missing extension ID".to_string()))?;

        info!("Pausing extension: {}", id);

        self.manager
            .pause_extension(id)
            .await
            .map_err(|e| {
                error!("Failed to pause extension {}: {}", id, e);
                ApiError::HandlerError(format!("Failed to pause extension: {}", e))
            })?;

        Ok(success_response(req.id))
    }
}

/// Resume extension handler
struct ResumeExtensionHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[async_trait::async_trait]
impl ApiHandler for ResumeExtensionHandler {
    fn path(&self) -> &str {
        "/api/extensions/:id/resume"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let id = req.metadata.get("id")
            .ok_or_else(|| ApiError::InvalidRequest("Missing extension ID".to_string()))?;

        info!("Resuming extension: {}", id);

        self.manager
            .resume_extension(id)
            .await
            .map_err(|e| {
                error!("Failed to resume extension {}: {}", id, e);
                ApiError::HandlerError(format!("Failed to resume extension: {}", e))
            })?;

        Ok(success_response(req.id))
    }
}

/// Handle extension request handler
struct HandleExtensionRequestHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[async_trait::async_trait]
impl ApiHandler for HandleExtensionRequestHandler {
    fn path(&self) -> &str {
        "/api/extensions/:id/request"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let id = req.metadata.get("id")
            .ok_or_else(|| ApiError::InvalidRequest("Missing extension ID".to_string()))?;

        let ext_request: ExtensionRequest = serde_json::from_slice(&req.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid request: {}", e)))?;

        debug!("Handling request for extension {}: {:?}", id, ext_request);

        let response = self.manager
            .handle_request(id, ext_request)
            .await
            .map_err(|e| {
                error!("Failed to handle extension request: {}", e);
                ApiError::HandlerError(format!("Failed to handle request: {}", e))
            })?;

        Ok(json_response(req.id, serde_json::to_value(response).unwrap()))
    }
}

/// Validate extension handler
struct ValidateExtensionHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[async_trait::async_trait]
impl ApiHandler for ValidateExtensionHandler {
    fn path(&self) -> &str {
        "/api/extensions/:id/validate"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let id = req.metadata.get("id")
            .ok_or_else(|| ApiError::InvalidRequest("Missing extension ID".to_string()))?;

        info!("Validating extension: {}", id);

        let reports = self.manager.validate_all_extensions().await;

        let report = reports
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("Extension not found: {}", id)))?;

        Ok(json_response(req.id, json!({
            "valid": report.valid,
            "errors": report.errors
                .into_iter()
                .map(|e| format!("{}: {}", e.code, e.message))
                .collect::<Vec<_>>(),
            "warnings": report.warnings
                .into_iter()
                .map(|w| format!("{}: {}", w.code, w.message))
                .collect::<Vec<_>>(),
        })))
    }
}

/// Get extension status handler
struct ExtensionStatusHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[async_trait::async_trait]
impl ApiHandler for ExtensionStatusHandler {
    fn path(&self) -> &str {
        "/api/extensions/:id/status"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let id = req.metadata.get("id")
            .ok_or_else(|| ApiError::InvalidRequest("Missing extension ID".to_string()))?;

        debug!("Getting status for extension: {}", id);

        let info = self.manager
            .get_extension_info(id)
            .await
            .ok_or_else(|| ApiError::NotFound(format!("Extension not found: {}", id)))?;

        Ok(json_response(req.id, json!({
            "id": info.metadata.id,
            "name": info.metadata.name,
            "version": info.metadata.version.to_string(),
            "state": format!("{:?}", info.state.state),
            "health": format!("{:?}", info.state.health),
            "request_count": info.state.request_count,
            "error_count": info.state.error_count,
            "cpu_usage": info.state.resource_usage.cpu_percent,
            "memory_usage": info.state.resource_usage.memory_bytes,
        })))
    }
}

/// Get extension metrics handler
struct ExtensionMetricsHandler {
    manager: Arc<UnifiedExtensionManager>,
}

#[async_trait::async_trait]
impl ApiHandler for ExtensionMetricsHandler {
    fn path(&self) -> &str {
        "/api/extensions/metrics"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        let metrics = self.manager.get_metrics().await;

        Ok(json_response(req.id, json!({
            "total_loaded": metrics.total_loaded,
            "total_failed": metrics.total_failed,
            "total_requests": metrics.total_requests,
            "total_errors": metrics.total_errors,
            "avg_request_duration_ms": metrics.avg_request_duration.as_millis() as u64,
            "peak_memory_mb": metrics.peak_memory / 1024 / 1024,
            "peak_cpu_percent": metrics.peak_cpu,
        })))
    }
}

/// Health check handler
struct HealthCheckHandler;

#[async_trait::async_trait]
impl ApiHandler for HealthCheckHandler {
    fn path(&self) -> &str {
        "/api/health"
    }

    async fn handle(&self, req: ApiRequest) -> Result<ApiResponse, ApiError> {
        Ok(json_response(req.id, json!({
            "status": "healthy",
            "service": "extension-api",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })))
    }
}

/// Real-time event streaming support using STOQ streaming
pub mod streaming {
    use super::*;
    use tokio::sync::mpsc;
    use futures::stream::Stream;

    /// Extension events
    #[derive(Debug, Serialize, Deserialize)]
    pub enum ExtensionEvent {
        StateChanged { extension_id: String, new_state: String },
        HealthChanged { extension_id: String, new_health: String },
        RequestCompleted { extension_id: String, request_id: String },
        Error { extension_id: String, error: String },
    }

    /// Event stream handler for real-time extension events
    pub struct ExtensionEventStreamHandler {
        manager: Arc<UnifiedExtensionManager>,
    }

    impl ExtensionEventStreamHandler {
        pub fn new(manager: Arc<UnifiedExtensionManager>) -> Self {
            Self { manager }
        }

        /// Create an event stream for a specific extension
        pub async fn create_event_stream(
            &self,
            extension_id: String
        ) -> impl Stream<Item = ExtensionEvent> {
            let (tx, rx) = mpsc::channel(100);

            // TODO: Hook into extension manager events
            // This would monitor the extension and send events through the channel

            tokio_stream::wrappers::ReceiverStream::new(rx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_event_serialization() {
        use streaming::ExtensionEvent;

        let event = ExtensionEvent::StateChanged {
            extension_id: "test-ext".to_string(),
            new_state: "loaded".to_string(),
        };

        let json = serde_json::to_value(&event).unwrap();
        assert!(json["StateChanged"].is_object());
        assert_eq!(json["StateChanged"]["extension_id"], "test-ext");
        assert_eq!(json["StateChanged"]["new_state"], "loaded");
    }
}