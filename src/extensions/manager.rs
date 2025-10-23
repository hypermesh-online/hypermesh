//! Unified Extension Management for HyperMesh
//!
//! This module provides the centralized extension manager that integrates
//! extensions as first-class citizens in the HyperMesh ecosystem.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::{
    ExtensionCapability, ExtensionCategory, ExtensionConfig, ExtensionError,
    ExtensionMetadata, ExtensionRequest, ExtensionResponse, ExtensionResult,
    ExtensionStatus, HyperMeshExtension, ResourceLimits, ValidationReport,
    AssetExtensionHandler, AssetLibraryExtension,
};

use crate::assets::core::{AssetManager, AssetType, PrivacyLevel};

/// Unified extension management system for HyperMesh
pub struct UnifiedExtensionManager {
    /// Loaded extensions
    extensions: Arc<RwLock<HashMap<String, Arc<Box<dyn HyperMeshExtension>>>>>,

    /// Extension metadata registry
    registry: Arc<RwLock<HashMap<String, ExtensionMetadata>>>,

    /// Asset handlers from extensions
    asset_handlers: Arc<RwLock<HashMap<AssetType, Arc<Box<dyn AssetExtensionHandler>>>>>,

    /// Extension load order
    load_order: Arc<RwLock<Vec<String>>>,

    /// Extension state tracking
    extension_states: Arc<RwLock<HashMap<String, ExtensionStateInfo>>>,

    /// Asset manager reference
    asset_manager: Arc<AssetManager>,

    /// Manager configuration
    config: ExtensionManagerConfig,

    /// Metrics collector
    metrics: Arc<RwLock<ExtensionMetrics>>,
}

/// Extension manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManagerConfig {
    /// Extension directory paths
    pub extension_dirs: Vec<PathBuf>,

    /// Auto-load extensions on startup
    pub auto_load: bool,

    /// Verify extension signatures
    pub verify_signatures: bool,

    /// Maximum extensions to load
    pub max_extensions: usize,

    /// Enable extension hot-reload
    pub hot_reload: bool,

    /// Extension timeout for operations
    pub operation_timeout: std::time::Duration,

    /// Global resource limits
    pub global_limits: ResourceLimits,

    /// Allowed capabilities for extensions
    pub allowed_capabilities: HashSet<ExtensionCapability>,

    /// Extension marketplace URL
    pub marketplace_url: Option<String>,

    /// Enable extension sandboxing
    pub enable_sandboxing: bool,

    /// Extension cache directory
    pub cache_dir: PathBuf,
}

impl Default for ExtensionManagerConfig {
    fn default() -> Self {
        Self {
            extension_dirs: vec![
                PathBuf::from("./extensions"),
                PathBuf::from("/usr/local/hypermesh/extensions"),
                PathBuf::from("~/.hypermesh/extensions"),
            ],
            auto_load: true,
            verify_signatures: true,
            max_extensions: 100,
            hot_reload: false,
            operation_timeout: std::time::Duration::from_secs(30),
            global_limits: ResourceLimits::default(),
            allowed_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::VMExecution,
                ExtensionCapability::ContainerManagement,
                ExtensionCapability::NetworkAccess,
                ExtensionCapability::ConsensusAccess,
                ExtensionCapability::TransportAccess,
                ExtensionCapability::MonitoringAccess,
            ]),
            marketplace_url: Some("https://marketplace.hypermesh.online".to_string()),
            enable_sandboxing: true,
            cache_dir: PathBuf::from("~/.hypermesh/extension-cache"),
        }
    }
}

/// Extension state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStateInfo {
    /// Extension ID
    pub id: String,

    /// Current state
    pub state: ExtensionState,

    /// Health status
    pub health: ExtensionHealth,

    /// Load timestamp
    pub loaded_at: std::time::SystemTime,

    /// Last activity timestamp
    pub last_activity: std::time::SystemTime,

    /// Request count
    pub request_count: u64,

    /// Error count
    pub error_count: u64,

    /// Resource usage
    pub resource_usage: ResourceUsage,
}

/// Extension state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionState {
    /// Extension is loading
    Loading,

    /// Extension is active and running
    Active,

    /// Extension is paused
    Paused,

    /// Extension is unloading
    Unloading,

    /// Extension has errored
    Error(String),
}

/// Extension health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionHealth {
    /// Extension is healthy
    Healthy,

    /// Extension is degraded
    Degraded(String),

    /// Extension is unhealthy
    Unhealthy(String),
}

/// Resource usage tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_percent: f32,

    /// Memory usage in bytes
    pub memory_bytes: u64,

    /// Network bandwidth in bytes/sec
    pub network_bandwidth: u64,

    /// Storage usage in bytes
    pub storage_bytes: u64,

    /// Active operation count
    pub active_operations: usize,
}

/// Extension metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionMetrics {
    /// Total extensions loaded
    pub total_loaded: usize,

    /// Total extensions failed
    pub total_failed: usize,

    /// Total requests processed
    pub total_requests: u64,

    /// Total errors
    pub total_errors: u64,

    /// Average request duration
    pub avg_request_duration: std::time::Duration,

    /// Peak memory usage
    pub peak_memory: u64,

    /// Peak CPU usage
    pub peak_cpu: f32,
}

/// Extension operation context
pub struct ExtensionContext {
    /// Extension ID
    pub extension_id: String,

    /// Request ID
    pub request_id: String,

    /// Operation timeout
    pub timeout: std::time::Duration,

    /// Granted capabilities
    pub capabilities: HashSet<ExtensionCapability>,

    /// Privacy level
    pub privacy_level: PrivacyLevel,
}

impl UnifiedExtensionManager {
    /// Create new unified extension manager
    pub fn new(asset_manager: Arc<AssetManager>, config: ExtensionManagerConfig) -> Self {
        Self {
            extensions: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(RwLock::new(HashMap::new())),
            asset_handlers: Arc::new(RwLock::new(HashMap::new())),
            load_order: Arc::new(RwLock::new(Vec::new())),
            extension_states: Arc::new(RwLock::new(HashMap::new())),
            asset_manager,
            config,
            metrics: Arc::new(RwLock::new(ExtensionMetrics::default())),
        }
    }

    /// Initialize extension manager
    pub async fn initialize(&self) -> ExtensionResult<()> {
        info!("Initializing unified extension manager");

        // Create extension directories if they don't exist
        for dir in &self.config.extension_dirs {
            if !dir.exists() {
                if let Err(e) = std::fs::create_dir_all(dir) {
                    warn!("Failed to create extension directory {:?}: {}", dir, e);
                }
            }
        }

        // Create cache directory
        if !self.config.cache_dir.exists() {
            std::fs::create_dir_all(&self.config.cache_dir)
                .map_err(|e| ExtensionError::Internal(e.into()))?;
        }

        // Auto-load extensions if enabled
        if self.config.auto_load {
            let loaded = self.auto_discover_and_load().await?;
            info!("Auto-loaded {} extensions", loaded.len());
        }

        Ok(())
    }

    /// Load an extension
    pub async fn load_extension(
        &self,
        mut extension: Box<dyn HyperMeshExtension>,
    ) -> ExtensionResult<()> {
        let metadata = extension.metadata();
        let extension_id = metadata.id.clone();

        info!("Loading extension: {}", extension_id);

        // Check if already loaded
        {
            let extensions = self.extensions.read().await;
            if extensions.contains_key(&extension_id) {
                return Err(ExtensionError::ExtensionAlreadyLoaded { id: extension_id });
            }
        }

        // Check extension limit
        {
            let extensions = self.extensions.read().await;
            if extensions.len() >= self.config.max_extensions {
                return Err(ExtensionError::ResourceLimitExceeded {
                    resource: "max_extensions".to_string(),
                });
            }
        }

        // Verify extension
        self.verify_extension(&metadata).await?;

        // Create extension configuration
        let config = self.create_extension_config(&metadata)?;

        // Update state
        {
            let mut states = self.extension_states.write().await;
            states.insert(
                extension_id.clone(),
                ExtensionStateInfo {
                    id: extension_id.clone(),
                    state: ExtensionState::Loading,
                    health: ExtensionHealth::Healthy,
                    loaded_at: std::time::SystemTime::now(),
                    last_activity: std::time::SystemTime::now(),
                    request_count: 0,
                    error_count: 0,
                    resource_usage: ResourceUsage::default(),
                },
            );
        }

        // Initialize extension
        if let Err(e) = extension.initialize(config).await {
            self.update_extension_state(&extension_id, ExtensionState::Error(e.to_string())).await;
            return Err(e);
        }

        // Register asset handlers
        let handlers = extension.register_assets().await?;
        {
            let mut asset_handlers = self.asset_handlers.write().await;
            for (asset_type, handler) in handlers {
                info!("Registered asset handler for type: {:?}", asset_type);
                asset_handlers.insert(asset_type, Arc::new(handler));
            }
        }

        // Extend asset manager
        extension.extend_manager(self.asset_manager.clone()).await?;

        // Store extension
        let extension_arc = Arc::new(extension);
        {
            let mut extensions = self.extensions.write().await;
            extensions.insert(extension_id.clone(), extension_arc);
        }

        // Update registry
        {
            let mut registry = self.registry.write().await;
            registry.insert(extension_id.clone(), metadata);
        }

        // Update load order
        {
            let mut load_order = self.load_order.write().await;
            load_order.push(extension_id.clone());
        }

        // Update state to active
        self.update_extension_state(&extension_id, ExtensionState::Active).await;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_loaded += 1;
        }

        info!("Successfully loaded extension: {}", extension_id);
        Ok(())
    }

    /// Unload an extension
    pub async fn unload_extension(&self, extension_id: &str) -> ExtensionResult<()> {
        info!("Unloading extension: {}", extension_id);

        // Update state
        self.update_extension_state(extension_id, ExtensionState::Unloading).await;

        // Get and remove extension
        let extension = {
            let mut extensions = self.extensions.write().await;
            extensions
                .remove(extension_id)
                .ok_or_else(|| ExtensionError::ExtensionNotFound {
                    id: extension_id.to_string(),
                })?
        };

        // Shutdown extension
        if let Err(e) = Arc::get_mut(&mut extension.clone())
            .ok_or_else(|| ExtensionError::Internal(anyhow::anyhow!("Extension still in use")))?
            .shutdown()
            .await
        {
            error!("Error shutting down extension {}: {}", extension_id, e);
        }

        // Remove from registry
        {
            let mut registry = self.registry.write().await;
            registry.remove(extension_id);
        }

        // Remove from load order
        {
            let mut load_order = self.load_order.write().await;
            load_order.retain(|id| id != extension_id);
        }

        // Remove state
        {
            let mut states = self.extension_states.write().await;
            states.remove(extension_id);
        }

        // TODO: Remove asset handlers belonging to this extension

        info!("Successfully unloaded extension: {}", extension_id);
        Ok(())
    }

    /// List all loaded extensions
    pub async fn list_extensions(&self) -> Vec<ExtensionInfo> {
        let registry = self.registry.read().await;
        let states = self.extension_states.read().await;

        registry
            .values()
            .map(|metadata| ExtensionInfo {
                metadata: metadata.clone(),
                state: states
                    .get(&metadata.id)
                    .map(|s| s.clone())
                    .unwrap_or_else(|| ExtensionStateInfo {
                        id: metadata.id.clone(),
                        state: ExtensionState::Error("Unknown state".to_string()),
                        health: ExtensionHealth::Unhealthy("Unknown state".to_string()),
                        loaded_at: std::time::SystemTime::now(),
                        last_activity: std::time::SystemTime::now(),
                        request_count: 0,
                        error_count: 0,
                        resource_usage: ResourceUsage::default(),
                    }),
            })
            .collect()
    }

    /// Get extension information
    pub async fn get_extension_info(&self, extension_id: &str) -> Option<ExtensionInfo> {
        let registry = self.registry.read().await;
        let states = self.extension_states.read().await;

        registry.get(extension_id).map(|metadata| ExtensionInfo {
            metadata: metadata.clone(),
            state: states
                .get(extension_id)
                .map(|s| s.clone())
                .unwrap_or_else(|| ExtensionStateInfo {
                    id: extension_id.to_string(),
                    state: ExtensionState::Error("Unknown state".to_string()),
                    health: ExtensionHealth::Unhealthy("Unknown state".to_string()),
                    loaded_at: std::time::SystemTime::now(),
                    last_activity: std::time::SystemTime::now(),
                    request_count: 0,
                    error_count: 0,
                    resource_usage: ResourceUsage::default(),
                }),
        })
    }

    /// Handle extension request
    pub async fn handle_request(
        &self,
        extension_id: &str,
        request: ExtensionRequest,
    ) -> ExtensionResult<ExtensionResponse> {
        let extension = {
            let extensions = self.extensions.read().await;
            extensions
                .get(extension_id)
                .ok_or_else(|| ExtensionError::ExtensionNotFound {
                    id: extension_id.to_string(),
                })?
                .clone()
        };

        // Update activity
        {
            let mut states = self.extension_states.write().await;
            if let Some(state) = states.get_mut(extension_id) {
                state.last_activity = std::time::SystemTime::now();
                state.request_count += 1;
            }
        }

        // Handle request with timeout
        let result = tokio::time::timeout(
            self.config.operation_timeout,
            extension.handle_request(request),
        )
        .await
        .map_err(|_| ExtensionError::RuntimeError {
            message: "Request timeout".to_string(),
        })?;

        // Update metrics
        match &result {
            Ok(_) => {
                let mut metrics = self.metrics.write().await;
                metrics.total_requests += 1;
            }
            Err(_) => {
                let mut metrics = self.metrics.write().await;
                metrics.total_errors += 1;

                let mut states = self.extension_states.write().await;
                if let Some(state) = states.get_mut(extension_id) {
                    state.error_count += 1;
                }
            }
        }

        result
    }

    /// Get asset handler for type
    pub async fn get_asset_handler(
        &self,
        asset_type: &AssetType,
    ) -> Option<Arc<Box<dyn AssetExtensionHandler>>> {
        let handlers = self.asset_handlers.read().await;
        handlers.get(asset_type).cloned()
    }

    /// Get extension as asset library
    pub async fn get_asset_library(
        &self,
        extension_id: &str,
    ) -> Option<Arc<Box<dyn AssetLibraryExtension>>> {
        let extensions = self.extensions.read().await;

        // This would require dynamic casting which Rust doesn't support directly
        // In practice, we'd need a different approach or store library extensions separately
        None
    }

    /// Auto-discover and load extensions
    async fn auto_discover_and_load(&self) -> ExtensionResult<Vec<String>> {
        let mut loaded = Vec::new();

        for dir in &self.config.extension_dirs {
            if !dir.exists() {
                continue;
            }

            debug!("Scanning directory for extensions: {:?}", dir);

            // Look for extension manifests
            let manifest_path = dir.join("manifest.json");
            if manifest_path.exists() {
                match self.load_extension_from_manifest(&manifest_path).await {
                    Ok(id) => {
                        loaded.push(id);
                    }
                    Err(e) => {
                        warn!("Failed to load extension from {:?}: {}", manifest_path, e);
                        let mut metrics = self.metrics.write().await;
                        metrics.total_failed += 1;
                    }
                }
            }

            // Scan subdirectories
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let manifest_path = entry.path().join("manifest.json");
                        if manifest_path.exists() {
                            match self.load_extension_from_manifest(&manifest_path).await {
                                Ok(id) => {
                                    loaded.push(id);
                                }
                                Err(e) => {
                                    warn!("Failed to load extension from {:?}: {}", manifest_path, e);
                                    let mut metrics = self.metrics.write().await;
                                    metrics.total_failed += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Load extension from manifest file
    async fn load_extension_from_manifest(&self, manifest_path: &PathBuf) -> ExtensionResult<String> {
        // Read manifest
        let manifest_data = std::fs::read_to_string(manifest_path)
            .map_err(|e| ExtensionError::Internal(e.into()))?;

        let manifest: ExtensionManifest = serde_json::from_str(&manifest_data)
            .map_err(|e| ExtensionError::Internal(e.into()))?;

        // TODO: Load actual extension binary/module based on manifest
        // This would involve dynamic loading of shared libraries or WASM modules

        Ok(manifest.id)
    }

    /// Verify extension before loading
    async fn verify_extension(&self, metadata: &ExtensionMetadata) -> ExtensionResult<()> {
        // Verify signature if required
        if self.config.verify_signatures {
            if let Some(fingerprint) = &metadata.certificate_fingerprint {
                // TODO: Implement TrustChain certificate verification
                debug!("Verifying certificate fingerprint: {}", fingerprint);
            }
        }

        // Verify dependencies
        let registry = self.registry.read().await;
        for dep in &metadata.dependencies {
            if !dep.optional {
                let installed = registry
                    .get(&dep.extension_id)
                    .ok_or_else(|| ExtensionError::DependencyResolutionFailed {
                        extension: metadata.id.clone(),
                        dependency: dep.extension_id.clone(),
                    })?;

                if !dep.version_requirement.matches(&installed.version) {
                    return Err(ExtensionError::VersionIncompatible {
                        extension: metadata.id.clone(),
                        required: dep.version_requirement.to_string(),
                        found: installed.version.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Create extension configuration
    fn create_extension_config(&self, metadata: &ExtensionMetadata) -> ExtensionResult<ExtensionConfig> {
        // Grant only allowed capabilities
        let granted_capabilities: HashSet<ExtensionCapability> = metadata
            .required_capabilities
            .intersection(&self.config.allowed_capabilities)
            .cloned()
            .collect();

        // Check if all required capabilities are granted
        for cap in &metadata.required_capabilities {
            if !granted_capabilities.contains(cap) {
                return Err(ExtensionError::CapabilityNotGranted {
                    capability: format!("{:?}", cap),
                });
            }
        }

        Ok(ExtensionConfig {
            settings: serde_json::Value::Null,
            resource_limits: self.config.global_limits.clone(),
            granted_capabilities,
            privacy_level: PrivacyLevel::Private,
            debug_mode: false,
        })
    }

    /// Update extension state
    async fn update_extension_state(&self, extension_id: &str, new_state: ExtensionState) {
        let mut states = self.extension_states.write().await;
        if let Some(state) = states.get_mut(extension_id) {
            state.state = new_state;
            state.last_activity = std::time::SystemTime::now();
        }
    }

    /// Get extension metrics
    pub async fn get_metrics(&self) -> ExtensionMetrics {
        self.metrics.read().await.clone()
    }

    /// Pause an extension
    pub async fn pause_extension(&self, extension_id: &str) -> ExtensionResult<()> {
        self.update_extension_state(extension_id, ExtensionState::Paused).await;
        Ok(())
    }

    /// Resume an extension
    pub async fn resume_extension(&self, extension_id: &str) -> ExtensionResult<()> {
        self.update_extension_state(extension_id, ExtensionState::Active).await;
        Ok(())
    }

    /// Reload an extension
    pub async fn reload_extension(&self, extension_id: &str) -> ExtensionResult<()> {
        if !self.config.hot_reload {
            return Err(ExtensionError::RuntimeError {
                message: "Hot reload is disabled".to_string(),
            });
        }

        // Get current extension metadata
        let metadata = {
            let registry = self.registry.read().await;
            registry
                .get(extension_id)
                .ok_or_else(|| ExtensionError::ExtensionNotFound {
                    id: extension_id.to_string(),
                })?
                .clone()
        };

        // Unload current extension
        self.unload_extension(extension_id).await?;

        // TODO: Reload extension from source
        // This would involve re-reading the extension binary/module and loading it

        Ok(())
    }

    /// Validate all extensions
    pub async fn validate_all_extensions(&self) -> HashMap<String, ValidationReport> {
        let mut reports = HashMap::new();
        let extensions = self.extensions.read().await;

        for (id, extension) in extensions.iter() {
            match extension.validate().await {
                Ok(report) => {
                    reports.insert(id.clone(), report);
                }
                Err(e) => {
                    error!("Failed to validate extension {}: {}", id, e);
                }
            }
        }

        reports
    }

    /// Shutdown all extensions
    pub async fn shutdown(&self) -> ExtensionResult<()> {
        info!("Shutting down extension manager");

        // Get all extension IDs
        let extension_ids: Vec<String> = {
            let load_order = self.load_order.read().await;
            load_order.clone()
        };

        // Unload extensions in reverse order
        for id in extension_ids.iter().rev() {
            if let Err(e) = self.unload_extension(id).await {
                error!("Failed to unload extension {}: {}", id, e);
            }
        }

        Ok(())
    }
}

/// Extension information combining metadata and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    /// Extension metadata
    pub metadata: ExtensionMetadata,

    /// Current state information
    pub state: ExtensionStateInfo,
}

/// Extension manifest for file-based loading
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtensionManifest {
    /// Extension ID
    pub id: String,

    /// Extension name
    pub name: String,

    /// Extension version
    pub version: String,

    /// Entry point (binary or module path)
    pub entry_point: String,

    /// Extension type (native, wasm, script)
    pub extension_type: String,

    /// Required capabilities
    pub capabilities: Vec<String>,

    /// Configuration schema
    pub config_schema: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::AssetManager;

    #[tokio::test]
    async fn test_extension_manager_creation() {
        let asset_manager = Arc::new(AssetManager::new());
        let config = ExtensionManagerConfig::default();
        let manager = UnifiedExtensionManager::new(asset_manager, config);

        assert_eq!(manager.list_extensions().await.len(), 0);
        assert_eq!(manager.get_metrics().await.total_loaded, 0);
    }

    #[tokio::test]
    async fn test_extension_state_tracking() {
        let asset_manager = Arc::new(AssetManager::new());
        let config = ExtensionManagerConfig::default();
        let manager = UnifiedExtensionManager::new(asset_manager, config);

        // Initially no extensions
        assert!(manager.get_extension_info("test").await.is_none());

        // After loading an extension, it should be tracked
        // (Would need a mock extension to test this fully)
    }
}