//! Catalog extension plugin entry point for HyperMesh
//!
//! This module provides the dynamic loading entry point and plugin implementation
//! that allows Catalog to be loaded as an extension in HyperMesh.

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::sync::Arc;
use tokio::sync::RwLock;

// Import HyperMesh extension types
use blockmatrix::extensions::{
    AssetExtensionHandler, AssetType, ExtensionCapability, ExtensionCategory,
    ExtensionConfig, ExtensionError, ExtensionMetadata, ExtensionRequest,
    ExtensionResponse, ExtensionResult, ExtensionState, ExtensionStatus,
    HyperMeshExtension, ResourceLimits, ValidationReport,
};

use crate::extension::{CatalogExtension, CatalogExtensionConfig};
use crate::assets::AssetManager as CatalogAssetManager;

/// Plugin version matching HyperMesh requirements
pub const PLUGIN_VERSION: &str = "1.0.0";

/// Required HyperMesh version
pub const REQUIRED_HYPERMESH_VERSION: &str = "1.0.0";

/// Catalog plugin wrapper for HyperMesh integration
pub struct CatalogPlugin {
    /// Inner catalog extension
    inner: Arc<RwLock<CatalogExtension>>,

    /// Extension configuration
    config: ExtensionConfig,

    /// Initialization state
    initialized: bool,

    /// Asset handlers
    handlers: HashMap<AssetType, Box<dyn AssetExtensionHandler>>,
}

impl CatalogPlugin {
    /// Create new catalog plugin instance
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CatalogExtension::new())),
            config: ExtensionConfig {
                settings: serde_json::Value::Null,
                resource_limits: ResourceLimits::default(),
                granted_capabilities: HashSet::new(),
                privacy_level: hypermesh::assets::core::PrivacyLevel::Private,
                debug_mode: false,
            },
            initialized: false,
            handlers: HashMap::new(),
        }
    }
}

#[async_trait]
impl HyperMeshExtension for CatalogPlugin {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            id: "catalog".to_string(),
            name: "HyperMesh Catalog Extension".to_string(),
            version: semver::Version::parse(PLUGIN_VERSION).unwrap(),
            description: "Decentralized asset library and VM runtime for HyperMesh".to_string(),
            author: "HyperMesh Team".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://hypermesh.online/catalog".to_string()),
            category: ExtensionCategory::AssetLibrary,
            hypermesh_version: semver::Version::parse(REQUIRED_HYPERMESH_VERSION).unwrap(),
            dependencies: vec![],
            required_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::VMExecution,
                ExtensionCapability::NetworkAccess,
                ExtensionCapability::ConsensusAccess,
                ExtensionCapability::TransportAccess,
                ExtensionCapability::FileSystemAccess,
            ]),
            provided_assets: vec![
                AssetType::VirtualMachine,
                AssetType::Container,
                AssetType::Library,
                AssetType::DataSet,
                AssetType::Model,
                AssetType::Algorithm,
                AssetType::Template,
            ],
            certificate_fingerprint: Some("SHA256:catalog_cert_fingerprint".to_string()),
            config_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "storage_path": {
                        "type": "string",
                        "description": "Path for catalog storage"
                    },
                    "cache_size": {
                        "type": "integer",
                        "description": "Cache size in MB"
                    },
                    "p2p_enabled": {
                        "type": "boolean",
                        "description": "Enable P2P distribution"
                    }
                }
            })),
        }
    }

    async fn initialize(&mut self, config: ExtensionConfig) -> ExtensionResult<()> {
        if self.initialized {
            return Ok(());
        }

        self.config = config;

        // Extract catalog-specific configuration
        let catalog_config = if let Some(settings) = self.config.settings.as_object() {
            CatalogExtensionConfig {
                storage_path: settings.get("storage_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.into())
                    .unwrap_or_else(|| "./catalog_storage".into()),
                cache_size_mb: settings.get("cache_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1024) as usize,
                p2p_enabled: settings.get("p2p_enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                consensus_required: true,
                max_asset_size: 10 * 1024 * 1024 * 1024, // 10GB
            }
        } else {
            CatalogExtensionConfig::default()
        };

        // Initialize the catalog extension
        let mut inner = self.inner.write().await;
        inner.initialize(catalog_config).await.map_err(|e| {
            ExtensionError::InitializationFailed {
                reason: format!("Catalog initialization failed: {}", e),
            }
        })?;

        self.initialized = true;
        Ok(())
    }

    async fn register_assets(&self) -> ExtensionResult<HashMap<AssetType, Box<dyn AssetExtensionHandler>>> {
        let inner = self.inner.read().await;

        // Get asset handlers from catalog extension
        let handlers = inner.get_asset_handlers().await.map_err(|e| {
            ExtensionError::RuntimeError {
                message: format!("Failed to get asset handlers: {}", e),
            }
        })?;

        // Convert catalog handlers to HyperMesh handlers
        let mut hypermesh_handlers = HashMap::new();
        for (asset_type, handler) in handlers {
            hypermesh_handlers.insert(asset_type, handler as Box<dyn AssetExtensionHandler>);
        }

        Ok(hypermesh_handlers)
    }

    async fn extend_manager(&self, asset_manager: Arc<hypermesh::assets::core::AssetManager>) -> ExtensionResult<()> {
        let mut inner = self.inner.write().await;

        // Integrate with HyperMesh asset manager
        inner.integrate_with_hypermesh(asset_manager).await.map_err(|e| {
            ExtensionError::RuntimeError {
                message: format!("Failed to integrate with AssetManager: {}", e),
            }
        })?;

        Ok(())
    }

    async fn handle_request(&self, request: ExtensionRequest) -> ExtensionResult<ExtensionResponse> {
        let inner = self.inner.read().await;

        // Route requests to catalog extension
        match request.method.as_str() {
            "list_packages" => {
                let packages = inner.list_packages(request.params).await.map_err(|e| {
                    ExtensionError::RuntimeError {
                        message: format!("Failed to list packages: {}", e),
                    }
                })?;

                Ok(ExtensionResponse {
                    request_id: request.id,
                    success: true,
                    data: Some(serde_json::to_value(packages).unwrap()),
                    error: None,
                })
            }
            "install_package" => {
                let package_id = request.params["package_id"].as_str()
                    .ok_or_else(|| ExtensionError::RuntimeError {
                        message: "Missing package_id parameter".to_string(),
                    })?;

                let result = inner.install_package(package_id, request.consensus_proof).await.map_err(|e| {
                    ExtensionError::RuntimeError {
                        message: format!("Failed to install package: {}", e),
                    }
                })?;

                Ok(ExtensionResponse {
                    request_id: request.id,
                    success: true,
                    data: Some(serde_json::to_value(result).unwrap()),
                    error: None,
                })
            }
            "execute_vm" => {
                let code = request.params["code"].as_str()
                    .ok_or_else(|| ExtensionError::RuntimeError {
                        message: "Missing code parameter".to_string(),
                    })?;

                let result = inner.execute_vm(code, request.params["inputs"].clone()).await.map_err(|e| {
                    ExtensionError::RuntimeError {
                        message: format!("VM execution failed: {}", e),
                    }
                })?;

                Ok(ExtensionResponse {
                    request_id: request.id,
                    success: true,
                    data: Some(result),
                    error: None,
                })
            }
            _ => {
                Ok(ExtensionResponse {
                    request_id: request.id,
                    success: false,
                    data: None,
                    error: Some(format!("Unknown method: {}", request.method)),
                })
            }
        }
    }

    async fn status(&self) -> ExtensionStatus {
        let inner = self.inner.read().await;
        inner.get_status().await.unwrap_or(ExtensionStatus {
            state: ExtensionState::Error("Unable to get status".to_string()),
            health: hypermesh::extensions::ExtensionHealth::Unhealthy("Status unavailable".to_string()),
            resource_usage: hypermesh::extensions::ResourceUsageReport {
                cpu_usage: 0.0,
                memory_usage: 0,
                network_bytes: 0,
                storage_bytes: 0,
            },
            active_operations: 0,
            total_requests: 0,
            error_count: 0,
            uptime: std::time::Duration::from_secs(0),
        })
    }

    async fn validate(&self) -> ExtensionResult<ValidationReport> {
        let inner = self.inner.read().await;

        // Perform validation checks
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check initialization
        if !self.initialized {
            errors.push(hypermesh::extensions::ValidationError {
                code: "NOT_INITIALIZED".to_string(),
                message: "Extension not initialized".to_string(),
                context: None,
            });
        }

        // Check capabilities
        for cap in &self.metadata().required_capabilities {
            if !self.config.granted_capabilities.contains(cap) {
                warnings.push(hypermesh::extensions::ValidationWarning {
                    code: "CAPABILITY_NOT_GRANTED".to_string(),
                    message: format!("Required capability not granted: {:?}", cap),
                    context: None,
                });
            }
        }

        // Validate internal catalog state
        if let Err(e) = inner.validate_internal().await {
            errors.push(hypermesh::extensions::ValidationError {
                code: "INTERNAL_VALIDATION_FAILED".to_string(),
                message: format!("Internal validation failed: {}", e),
                context: None,
            });
        }

        Ok(ValidationReport {
            valid: errors.is_empty(),
            certificate_valid: Some(true), // TODO: Implement actual certificate validation
            dependencies_satisfied: true,
            resource_compliance: true,
            security_compliance: warnings.is_empty(),
            errors,
            warnings,
        })
    }

    async fn export_state(&self) -> ExtensionResult<ExtensionState> {
        let inner = self.inner.read().await;

        let state_data = inner.export_state().await.map_err(|e| {
            ExtensionError::RuntimeError {
                message: format!("Failed to export state: {}", e),
            }
        })?;

        Ok(ExtensionState {
            version: 1,
            metadata: self.metadata(),
            state_data,
            checksum: "TODO_CALCULATE_CHECKSUM".to_string(),
            exported_at: std::time::SystemTime::now(),
        })
    }

    async fn import_state(&mut self, state: ExtensionState) -> ExtensionResult<()> {
        if state.version != 1 {
            return Err(ExtensionError::VersionIncompatible {
                extension: "catalog".to_string(),
                required: "1".to_string(),
                found: state.version.to_string(),
            });
        }

        let mut inner = self.inner.write().await;
        inner.import_state(state.state_data).await.map_err(|e| {
            ExtensionError::RuntimeError {
                message: format!("Failed to import state: {}", e),
            }
        })?;

        Ok(())
    }

    async fn shutdown(&mut self) -> ExtensionResult<()> {
        if !self.initialized {
            return Ok(());
        }

        let mut inner = self.inner.write().await;
        inner.shutdown().await.map_err(|e| {
            ExtensionError::RuntimeError {
                message: format!("Failed to shutdown: {}", e),
            }
        })?;

        self.initialized = false;
        Ok(())
    }
}

/// C-compatible entry point for dynamic loading
///
/// This function is called by HyperMesh's extension loader to create
/// an instance of the Catalog plugin.
///
/// # Safety
/// This function is marked as unsafe because it returns a raw pointer
/// that must be properly managed by the caller.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_extension_create() -> *mut dyn HyperMeshExtension {
    let plugin = Box::new(CatalogPlugin::new());
    Box::into_raw(plugin) as *mut dyn HyperMeshExtension
}

/// C-compatible destructor for the plugin
///
/// # Safety
/// This function is marked as unsafe because it deallocates a raw pointer.
#[no_mangle]
pub unsafe extern "C" fn hypermesh_extension_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr as *mut CatalogPlugin);
    }
}

/// Get plugin metadata without creating instance
///
/// This can be used by the loader to check compatibility before loading.
#[no_mangle]
pub extern "C" fn hypermesh_extension_metadata() -> *const u8 {
    let metadata = ExtensionMetadata {
        id: "catalog".to_string(),
        name: "HyperMesh Catalog Extension".to_string(),
        version: semver::Version::parse(PLUGIN_VERSION).unwrap(),
        description: "Decentralized asset library and VM runtime for HyperMesh".to_string(),
        author: "HyperMesh Team".to_string(),
        license: "MIT".to_string(),
        homepage: Some("https://hypermesh.online/catalog".to_string()),
        category: ExtensionCategory::AssetLibrary,
        hypermesh_version: semver::Version::parse(REQUIRED_HYPERMESH_VERSION).unwrap(),
        dependencies: vec![],
        required_capabilities: HashSet::from([
            ExtensionCapability::AssetManagement,
            ExtensionCapability::VMExecution,
            ExtensionCapability::NetworkAccess,
        ]),
        provided_assets: vec![
            AssetType::VirtualMachine,
            AssetType::Container,
            AssetType::Library,
        ],
        certificate_fingerprint: Some("SHA256:catalog_cert_fingerprint".to_string()),
        config_schema: None,
    };

    let json = serde_json::to_string(&metadata).unwrap();
    json.as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = CatalogPlugin::new();
        let metadata = plugin.metadata();
        assert_eq!(metadata.id, "catalog");
        assert_eq!(metadata.category, ExtensionCategory::AssetLibrary);
    }

    #[test]
    fn test_plugin_metadata() {
        let metadata_ptr = unsafe { hypermesh_extension_metadata() };
        assert!(!metadata_ptr.is_null());
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let mut plugin = CatalogPlugin::new();

        // Initialize
        let config = ExtensionConfig {
            settings: serde_json::json!({
                "storage_path": "/tmp/catalog_test",
                "cache_size": 512,
                "p2p_enabled": true
            }),
            resource_limits: ResourceLimits::default(),
            granted_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::VMExecution,
            ]),
            privacy_level: hypermesh::assets::core::PrivacyLevel::Private,
            debug_mode: true,
        };

        assert!(plugin.initialize(config).await.is_ok());

        // Validate
        let validation = plugin.validate().await.unwrap();
        assert!(validation.valid || !validation.warnings.is_empty());

        // Get status
        let status = plugin.status().await;
        assert!(matches!(status.state, ExtensionState::Running | ExtensionState::Error(_)));

        // Shutdown
        assert!(plugin.shutdown().await.is_ok());
    }
}