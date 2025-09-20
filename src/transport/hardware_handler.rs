//! Hardware Detection HTTP Route Handler
//!
//! Provides HTTP API endpoints for hardware detection and resource monitoring

use std::sync::Arc;
use anyhow::Result;
use serde_json::json;
use tracing::{debug, warn};
use std::collections::HashMap;

use crate::hardware::{HardwareDetectionService, HardwareApiResponse};
use super::http_gateway::{RouteHandler, HttpRequest, HttpResponse};

/// Hardware detection route handler
pub struct HardwareRouteHandler {
    hardware_service: Arc<HardwareDetectionService>,
}

impl HardwareRouteHandler {
    /// Create new hardware route handler
    pub fn new(hardware_service: Arc<HardwareDetectionService>) -> Self {
        Self { hardware_service }
    }
}

impl RouteHandler for HardwareRouteHandler {
    fn handle(&self, request: &HttpRequest) -> Result<HttpResponse> {
        debug!("Hardware API request: {} {}", request.method, request.path);

        // Parse the specific endpoint from the path
        let response = match request.path.as_str() {
            "/api/v1/system/hardware" => {
                // Get hardware capabilities - block on async operation
                let capabilities = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.hardware_service.get_hardware_capabilities().await
                    })
                });

                match capabilities {
                    Ok(capabilities) => {
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());
                        headers.insert("Cache-Control".to_string(), "max-age=5".to_string());

                        HttpResponse {
                            status: 200,
                            headers,
                            body: serde_json::to_vec(&HardwareApiResponse::success(capabilities))
                                .unwrap_or_else(|e| {
                                    warn!("Failed to serialize hardware capabilities: {}", e);
                                    serde_json::to_vec(&json!({
                                        "error": "Serialization failed"
                                    })).unwrap()
                                }),
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get hardware capabilities: {}", e);
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());

                        HttpResponse {
                            status: 500,
                            headers,
                            body: serde_json::to_vec(&HardwareApiResponse::<()>::error(
                                format!("Failed to detect hardware: {}", e)
                            )).unwrap(),
                        }
                    }
                }
            }

            "/api/v1/system/network" => {
                // Get network capabilities specifically - block on async operation
                let capabilities = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.hardware_service.get_hardware_capabilities().await
                    })
                });

                match capabilities {
                    Ok(capabilities) => {
                        let network_data = json!({
                            "interfaces": capabilities.network,
                            "detected_at": capabilities.detected_at,
                        });

                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());
                        headers.insert("Cache-Control".to_string(), "max-age=5".to_string());

                        HttpResponse {
                            status: 200,
                            headers,
                            body: serde_json::to_vec(&HardwareApiResponse::success(network_data))
                                .unwrap_or_else(|e| {
                                    warn!("Failed to serialize network capabilities: {}", e);
                                    serde_json::to_vec(&json!({
                                        "error": "Serialization failed"
                                    })).unwrap()
                                }),
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get network capabilities: {}", e);
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());

                        HttpResponse {
                            status: 500,
                            headers,
                            body: serde_json::to_vec(&HardwareApiResponse::<()>::error(
                                format!("Failed to detect network: {}", e)
                            )).unwrap(),
                        }
                    }
                }
            }

            "/api/v1/system/allocation" => {
                // Get current resource allocation - block on async operation
                let allocation = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.hardware_service.get_resource_allocation().await
                    })
                });

                match allocation {
                    Ok(allocation) => {
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());
                        headers.insert("Cache-Control".to_string(), "max-age=2".to_string());

                        HttpResponse {
                            status: 200,
                            headers,
                            body: serde_json::to_vec(&HardwareApiResponse::success(allocation))
                                .unwrap_or_else(|e| {
                                    warn!("Failed to serialize resource allocation: {}", e);
                                    serde_json::to_vec(&json!({
                                        "error": "Serialization failed"
                                    })).unwrap()
                                }),
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get resource allocation: {}", e);
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());

                        HttpResponse {
                            status: 500,
                            headers,
                            body: serde_json::to_vec(&HardwareApiResponse::<()>::error(
                                format!("Failed to get allocation: {}", e)
                            )).unwrap(),
                        }
                    }
                }
            }

            "/api/v1/system/capabilities" => {
                // Get full HyperMesh capabilities - block on async operation
                let capabilities = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.hardware_service.get_hypermesh_capabilities().await
                    })
                });

                match capabilities {
                    Ok(capabilities) => {
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());
                        headers.insert("Cache-Control".to_string(), "max-age=30".to_string());

                        HttpResponse {
                            status: 200,
                            headers,
                            body: serde_json::to_vec(&HardwareApiResponse::success(capabilities))
                                .unwrap_or_else(|e| {
                                    warn!("Failed to serialize HyperMesh capabilities: {}", e);
                                    serde_json::to_vec(&json!({
                                        "error": "Serialization failed"
                                    })).unwrap()
                                }),
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get HyperMesh capabilities: {}", e);
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());

                        HttpResponse {
                            status: 500,
                            headers,
                            body: serde_json::to_vec(&HardwareApiResponse::<()>::error(
                                format!("Failed to get capabilities: {}", e)
                            )).unwrap(),
                        }
                    }
                }
            }

            "/api/v1/system/refresh" => {
                // Force refresh hardware detection - block on async operation
                if request.method != "POST" {
                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());

                    return Ok(HttpResponse {
                        status: 405,
                        headers,
                        body: json!({
                            "error": "Method not allowed. Use POST to refresh."
                        }).to_string().into_bytes(),
                    });
                }

                let result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.hardware_service.refresh_hardware_detection().await
                    })
                });

                match result {
                    Ok(capabilities) => {
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());

                        HttpResponse {
                            status: 200,
                            headers,
                            body: serde_json::to_vec(&HardwareApiResponse::success(json!({
                                "message": "Hardware detection refreshed",
                                "capabilities": capabilities
                            })))
                            .unwrap_or_else(|e| {
                                warn!("Failed to serialize refresh response: {}", e);
                                serde_json::to_vec(&json!({
                                    "error": "Serialization failed"
                                })).unwrap()
                            }),
                        }
                    }
                    Err(e) => {
                        warn!("Failed to refresh hardware detection: {}", e);
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());

                        HttpResponse {
                            status: 500,
                            headers,
                            body: serde_json::to_vec(&HardwareApiResponse::<()>::error(
                                format!("Failed to refresh: {}", e)
                            )).unwrap(),
                        }
                    }
                }
            }

            _ => {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());

                HttpResponse {
                    status: 404,
                    headers,
                    body: json!({
                        "error": "Endpoint not found",
                        "path": request.path
                    }).to_string().into_bytes(),
                }
            }
        };

        Ok(response)
    }
}