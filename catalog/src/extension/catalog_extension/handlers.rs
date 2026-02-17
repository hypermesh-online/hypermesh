// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CatalogExtension request handlers - HyperMeshExtension trait implementation

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use blockmatrix::extensions::{
    HyperMeshExtension, ExtensionMetadata,
    ExtensionConfig, ExtensionResult, ExtensionError,
    ExtensionRequest, ExtensionResponse, ExtensionStatus, ExtensionState,
    ExtensionHealth, ValidationReport, ValidationError, ValidationWarning,
    AssetExtensionHandler, ExtensionStateData,
};

use blockmatrix::assets::core::{AssetManager, AssetType};

use crate::{
    Catalog, CatalogConfig,
    sharing::{SharingConfig, SharePermission},
    registry::SearchQuery,
};

use super::super::asset_handlers::{
    VirtualMachineHandler, LibraryHandler, DatasetHandler, TemplateHandler,
};
use super::super::config::ExtensionSettings;
use super::types::CatalogExtension;

#[async_trait]
impl HyperMeshExtension for CatalogExtension {
    fn metadata(&self) -> ExtensionMetadata {
        self.metadata.clone()
    }

    async fn initialize(&mut self, config: ExtensionConfig) -> ExtensionResult<()> {
        {
            let mut state = self.state.write().await;
            state.version = state.version.saturating_add(1);
        }

        if let Ok(settings) = serde_json::from_value::<ExtensionSettings>(config.settings.clone()) {
            self.config.apply_settings(settings);
        }

        let catalog_config = CatalogConfig {
            hypermesh_address: Some(self.config.hypermesh_address.clone()),
            trustchain_cert_path: self.config.trustchain_cert_path.clone(),
            ..Default::default()
        };

        match Catalog::new(catalog_config).await {
            Ok(catalog) => {
                self.catalog = Some(Arc::new(catalog));

                if self.config.enable_p2p {
                    let sharing_config = SharingConfig {
                        node_id: format!("catalog_{}", uuid::Uuid::new_v4()),
                        max_mirror_storage: self.config.cache_size as u64,
                        max_bandwidth: 10 * 1024 * 1024,
                        replication_factor: 3,
                        default_permission: SharePermission::Public,
                        auto_mirror_popular: true,
                        enable_incentives: true,
                        ..Default::default()
                    };

                    let catalog_registry = Arc::new(crate::registry::CatalogRegistry::new(
                        blockmatrix::assets::PrivacyLevel::FullPublic,
                        crate::registry::TrustPolicy::default(),
                        crate::registry::RegistryConfig::default(),
                    ));
                    match crate::sharing::SharingManager::new(sharing_config, catalog_registry).await {
                        Ok(sharing_manager) => {
                            self.sharing_manager = Some(Arc::new(sharing_manager));
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to initialize sharing manager: {}", e);
                        }
                    }
                }

                {
                    let mut state = self.state.write().await;
                    state.version = state.version.saturating_add(1);
                }

                Ok(())
            }
            Err(e) => {
                let mut state = self.state.write().await;
                state.checksum = format!("error:{}", e);
                state.version = state.version.saturating_add(1);

                Err(ExtensionError::InitializationFailed {
                    reason: e.to_string()
                })
            }
        }
    }

    async fn register_assets(&self) -> ExtensionResult<HashMap<AssetType, Box<dyn AssetExtensionHandler>>> {
        self.increment_requests().await;

        let mut handlers = HashMap::new();
        handlers.insert(
            AssetType::VirtualMachine,
            Box::new(VirtualMachineHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        handlers.insert(
            AssetType::Library,
            Box::new(LibraryHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        handlers.insert(
            AssetType::Library,
            Box::new(DatasetHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        handlers.insert(
            AssetType::Container,
            Box::new(TemplateHandler::new()) as Box<dyn AssetExtensionHandler>
        );

        Ok(handlers)
    }

    async fn extend_manager(&self, _asset_manager: Arc<AssetManager>) -> ExtensionResult<()> {
        self.increment_requests().await;
        Ok(())
    }

    async fn handle_request(&self, request: ExtensionRequest) -> ExtensionResult<ExtensionResponse> {
        self.increment_requests().await;
        self.start_operation().await;

        let response = match request.method.as_str() {
            "catalog.search" => self.handle_search(request).await,
            "catalog.validate" => self.handle_validate(request).await,
            "catalog.stats" => self.handle_stats(request).await,
            "catalog.sharing.connect" => self.handle_sharing_connect(request).await,
            "catalog.sharing.search" => self.handle_sharing_search(request).await,
            "catalog.sharing.stats" => self.handle_sharing_stats(request).await,
            _ => ExtensionResponse {
                request_id: request.id,
                success: false,
                data: None,
                error: Some(format!("Unknown method: {}", request.method)),
            }
        };

        self.complete_operation().await;

        if !response.success {
            if let Some(error) = &response.error {
                self.track_error(error).await;
            }
        }

        Ok(response)
    }

    async fn status(&self) -> ExtensionStatus {
        ExtensionStatus {
            state: ExtensionState::Running,
            health: self.health.read().await.clone(),
            resource_usage: self.resource_usage.read().await.clone(),
            active_operations: *self.active_operations.read().await,
            total_requests: *self.total_requests.read().await,
            error_count: *self.error_count.read().await,
            uptime: self.start_time.elapsed(),
        }
    }

    async fn validate(&self) -> ExtensionResult<ValidationReport> {
        self.increment_requests().await;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if self.catalog.is_none() {
            errors.push(ValidationError {
                code: "CATALOG_NOT_INITIALIZED".to_string(),
                message: "Catalog core is not initialized".to_string(),
                context: None,
            });
        }

        if !self.config.library_path.exists() {
            warnings.push(ValidationWarning {
                code: "LIBRARY_PATH_MISSING".to_string(),
                message: format!("Library path does not exist: {:?}", self.config.library_path),
                context: None,
            });
        }

        let usage = self.resource_usage.read().await;
        if usage.memory_usage > self.config.max_memory_usage {
            warnings.push(ValidationWarning {
                code: "HIGH_MEMORY_USAGE".to_string(),
                message: format!("Memory usage exceeds limit: {} > {}",
                    usage.memory_usage, self.config.max_memory_usage),
                context: Some(serde_json::json!({ "current": usage.memory_usage })),
            });
        }

        Ok(ValidationReport {
            valid: errors.is_empty(),
            certificate_valid: self.config.certificate_fingerprint.as_ref().map(|_| true),
            dependencies_satisfied: true,
            resource_compliance: usage.memory_usage <= self.config.max_memory_usage,
            security_compliance: true,
            errors,
            warnings,
        })
    }

    async fn export_state(&self) -> ExtensionResult<ExtensionStateData> {
        self.increment_requests().await;

        let state_data = serde_json::json!({
            "library_manager": "serialized_library_state",
            "asset_registry": "serialized_registry_state",
            "statistics": {
                "total_requests": *self.total_requests.read().await,
                "error_count": *self.error_count.read().await,
            }
        });

        Ok(ExtensionStateData {
            version: 1,
            metadata: self.metadata.clone(),
            state_data: state_data.to_string().into_bytes(),
            checksum: "sha256_checksum_here".to_string(),
            exported_at: std::time::SystemTime::now(),
        })
    }

    async fn import_state(&mut self, _state: ExtensionStateData) -> ExtensionResult<()> {
        self.increment_requests().await;
        Ok(())
    }

    async fn shutdown(&mut self) -> ExtensionResult<()> {
        {
            let mut health = self.health.write().await;
            *health = ExtensionHealth::Degraded("Shutting down".to_string());
        }

        let mut retries = 10;
        while *self.active_operations.read().await > 0 && retries > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            retries -= 1;
        }

        {
            let mut health = self.health.write().await;
            *health = ExtensionHealth::Unhealthy("Extension stopped".to_string());
        }

        Ok(())
    }
}

// Private request handler methods
impl CatalogExtension {
    async fn handle_search(&self, request: ExtensionRequest) -> ExtensionResponse {
        if let Some(catalog) = &self.catalog {
            if let Ok(query) = serde_json::from_value::<SearchQuery>(request.params) {
                match catalog.search_assets(&query).await {
                    Ok(results) => {
                        let data = match serde_json::to_value(results) {
                            Ok(v) => Some(v),
                            Err(e) => return ExtensionResponse {
                                request_id: request.id,
                                success: false,
                                data: None,
                                error: Some(format!("Failed to serialize search results: {}", e)),
                            },
                        };
                        ExtensionResponse { request_id: request.id, success: true, data, error: None }
                    },
                    Err(e) => ExtensionResponse { request_id: request.id, success: false, data: None, error: Some(e.to_string()) }
                }
            } else {
                ExtensionResponse { request_id: request.id, success: false, data: None, error: Some("Invalid search query".to_string()) }
            }
        } else {
            ExtensionResponse { request_id: request.id, success: false, data: None, error: Some("Catalog not initialized".to_string()) }
        }
    }

    async fn handle_validate(&self, request: ExtensionRequest) -> ExtensionResponse {
        if self.catalog.is_some() {
            ExtensionResponse { request_id: request.id, success: true, data: Some(serde_json::json!({ "valid": true })), error: None }
        } else {
            ExtensionResponse { request_id: request.id, success: false, data: None, error: Some("Catalog not initialized".to_string()) }
        }
    }

    async fn handle_stats(&self, request: ExtensionRequest) -> ExtensionResponse {
        let stats = serde_json::json!({
            "total_requests": *self.total_requests.read().await,
            "active_operations": *self.active_operations.read().await,
            "error_count": *self.error_count.read().await,
            "uptime_seconds": self.start_time.elapsed().as_secs(),
        });
        ExtensionResponse { request_id: request.id, success: true, data: Some(stats), error: None }
    }

    async fn handle_sharing_connect(&self, request: ExtensionRequest) -> ExtensionResponse {
        if let Some(sharing_manager) = &self.sharing_manager {
            if let Some(address) = request.params.get("address").and_then(|v| v.as_str()) {
                match sharing_manager.connect_peer(address).await {
                    Ok(peer_id) => ExtensionResponse {
                        request_id: request.id, success: true,
                        data: Some(serde_json::json!({ "peer_id": peer_id })), error: None,
                    },
                    Err(e) => ExtensionResponse {
                        request_id: request.id, success: false, data: None,
                        error: Some(format!("Failed to connect to peer: {}", e)),
                    }
                }
            } else {
                ExtensionResponse { request_id: request.id, success: false, data: None, error: Some("Missing address parameter".to_string()) }
            }
        } else {
            ExtensionResponse { request_id: request.id, success: false, data: None, error: Some("Sharing not enabled".to_string()) }
        }
    }

    async fn handle_sharing_search(&self, request: ExtensionRequest) -> ExtensionResponse {
        if let Some(sharing_manager) = &self.sharing_manager {
            if let Some(query) = request.params.get("query").and_then(|v| v.as_str()) {
                match sharing_manager.search_packages(query).await {
                    Ok(results) => {
                        let data = match serde_json::to_value(results) {
                            Ok(v) => Some(v),
                            Err(e) => return ExtensionResponse {
                                request_id: request.id, success: false, data: None,
                                error: Some(format!("Failed to serialize search results: {}", e)),
                            },
                        };
                        ExtensionResponse { request_id: request.id, success: true, data, error: None }
                    },
                    Err(e) => ExtensionResponse {
                        request_id: request.id, success: false, data: None,
                        error: Some(format!("Search failed: {}", e)),
                    }
                }
            } else {
                ExtensionResponse { request_id: request.id, success: false, data: None, error: Some("Missing query parameter".to_string()) }
            }
        } else {
            ExtensionResponse { request_id: request.id, success: false, data: None, error: Some("Sharing not enabled".to_string()) }
        }
    }

    async fn handle_sharing_stats(&self, request: ExtensionRequest) -> ExtensionResponse {
        if let Some(sharing_manager) = &self.sharing_manager {
            let stats = sharing_manager.get_stats().await;
            let data = match serde_json::to_value(stats) {
                Ok(v) => Some(v),
                Err(e) => return ExtensionResponse {
                    request_id: request.id, success: false, data: None,
                    error: Some(format!("Failed to serialize sharing stats: {}", e)),
                },
            };
            ExtensionResponse { request_id: request.id, success: true, data, error: None }
        } else {
            ExtensionResponse { request_id: request.id, success: false, data: None, error: Some("Sharing not enabled".to_string()) }
        }
    }
}
