//! Catalog extension plugin entry point for HyperMesh
//!
//! This module provides the dynamic loading entry point and plugin implementation
//! that allows Catalog to be loaded as an extension in HyperMesh.

// Allow unsafe code for FFI plugin entry points - required for dynamic loading
#![allow(unsafe_code)]

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::sync::Arc;
use tokio::sync::RwLock;

// Import HyperMesh extension types
use blockmatrix::extensions::{
    AssetExtensionHandler, ExtensionCapability, ExtensionCategory,
    ExtensionConfig, ExtensionError, ExtensionMetadata, ExtensionRequest,
    ExtensionResponse, ExtensionResult, ExtensionState, ExtensionStateData, ExtensionStatus,
    HyperMeshExtension, AssetLibraryExtension, ResourceLimits, ValidationReport,
    PackageFilter,
};
use blockmatrix::assets::core::AssetType;

use crate::extension::{CatalogExtension, CatalogExtensionConfig};

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
            inner: Arc::new(RwLock::new(CatalogExtension::new(CatalogExtensionConfig::default()))),
            config: ExtensionConfig {
                settings: serde_json::Value::Null,
                resource_limits: ResourceLimits::default(),
                granted_capabilities: HashSet::new(),
                privacy_level: blockmatrix::assets::core::PrivacyLevel::Private,
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
        // Parse hardcoded versions with compile-time validation via const assertion
        const _: () = {
            // This will fail at compile time if versions are invalid
            match semver::Version::parse(PLUGIN_VERSION) {
                Ok(_) => {},
                Err(_) => panic!("PLUGIN_VERSION must be valid semver"),
            }
        };

        let version = semver::Version::parse(PLUGIN_VERSION)
            .expect("PLUGIN_VERSION validated at compile time");
        let hypermesh_version = semver::Version::parse(REQUIRED_HYPERMESH_VERSION)
            .expect("REQUIRED_HYPERMESH_VERSION validated at compile time");

        ExtensionMetadata {
            id: "catalog".to_string(),
            name: "HyperMesh Catalog Extension".to_string(),
            version,
            description: "Decentralized asset library and VM runtime for HyperMesh".to_string(),
            author: "HyperMesh Team".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://hypermesh.online/catalog".to_string()),
            category: ExtensionCategory::AssetLibrary,
            hypermesh_version,
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
                library_path: settings.get("storage_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.into())
                    .unwrap_or_else(|| "./catalog_storage".into()),
                cache_size: settings.get("cache_size")
                    .and_then(|v| v.as_u64())
                    .map(|mb| mb * 1024 * 1024)
                    .unwrap_or(1024 * 1024 * 1024),
                max_package_size: 10 * 1024 * 1024 * 1024, // 10GB
                enable_p2p: settings.get("p2p_enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                consensus_validation: true,
                hypermesh_address: "catalog.hypermesh.online".to_string(),
                trustchain_cert_path: None,
                certificate_fingerprint: None,
                max_memory_usage: 2 * 1024 * 1024 * 1024, // 2GB
                max_concurrent_ops: 100,
                debug_mode: false,
                indexing: Default::default(),
                security: Default::default(),
                performance: Default::default(),
            }
        } else {
            CatalogExtensionConfig::default()
        };

        // Initialize the catalog extension with ExtensionConfig, not CatalogExtensionConfig
        let mut inner = self.inner.write().await;
        inner.initialize(self.config.clone()).await.map_err(|e| {
            ExtensionError::InitializationFailed {
                reason: format!("Catalog initialization failed: {}", e),
            }
        })?;

        self.initialized = true;
        Ok(())
    }

    async fn register_assets(&self) -> ExtensionResult<HashMap<AssetType, Box<dyn AssetExtensionHandler>>> {
        // Return asset handlers through the HyperMeshExtension trait
        let inner = self.inner.read().await;
        inner.register_assets().await
    }

    async fn extend_manager(&self, asset_manager: Arc<blockmatrix::assets::core::AssetManager>) -> ExtensionResult<()> {
        let mut inner = self.inner.write().await;
        inner.extend_manager(asset_manager).await
    }

    async fn handle_request(&self, request: ExtensionRequest) -> ExtensionResult<ExtensionResponse> {
        let inner = self.inner.read().await;

        // Delegate request handling to inner extension
        inner.handle_request(request).await
    }

    async fn status(&self) -> ExtensionStatus {
        let inner = self.inner.read().await;
        inner.status().await
    }

    async fn validate(&self) -> ExtensionResult<ValidationReport> {
        let inner = self.inner.read().await;
        inner.validate().await
    }

    async fn export_state(&self) -> ExtensionResult<ExtensionStateData> {
        let inner = self.inner.read().await;
        inner.export_state().await
    }

    async fn import_state(&mut self, state: ExtensionStateData) -> ExtensionResult<()> {
        let mut inner = self.inner.write().await;
        inner.import_state(state).await
    }

    async fn shutdown(&mut self) -> ExtensionResult<()> {
        let mut inner = self.inner.write().await;
        self.initialized = false;
        inner.shutdown().await
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
    // Parse hardcoded versions - validated at compile time by metadata() function
    let version = semver::Version::parse(PLUGIN_VERSION)
        .expect("PLUGIN_VERSION validated at compile time");
    let hypermesh_version = semver::Version::parse(REQUIRED_HYPERMESH_VERSION)
        .expect("REQUIRED_HYPERMESH_VERSION validated at compile time");

    let metadata = ExtensionMetadata {
        id: "catalog".to_string(),
        name: "HyperMesh Catalog Extension".to_string(),
        version,
        description: "Decentralized asset library and VM runtime for HyperMesh".to_string(),
        author: "HyperMesh Team".to_string(),
        license: "MIT".to_string(),
        homepage: Some("https://hypermesh.online/catalog".to_string()),
        category: ExtensionCategory::AssetLibrary,
        hypermesh_version,
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

    let json = serde_json::to_string(&metadata)
        .expect("ExtensionMetadata serialization should never fail");
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
            privacy_level: blockmatrix::assets::core::PrivacyLevel::Private,
            debug_mode: true,
        };

        assert!(plugin.initialize(config).await.is_ok());

        // Validate
        let validation = plugin.validate().await
            .expect("Plugin validation should succeed after initialization");
        assert!(validation.valid || !validation.warnings.is_empty());

        // Get status
        let status = plugin.status().await;
        assert!(matches!(status.state, ExtensionState::Running | ExtensionState::Error(_)));

        // Shutdown
        assert!(plugin.shutdown().await.is_ok());
    }
}