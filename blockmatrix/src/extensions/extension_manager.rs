//! Extension manager implementation

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::assets::core::{AssetManager, AssetType};
use super::types::*;
use super::traits::{HyperMeshExtension, AssetExtensionHandler};

/// Extension manager for loading and managing extensions
pub struct ExtensionManager {
    extensions: Arc<RwLock<HashMap<String, Box<dyn HyperMeshExtension>>>>,
    registry: Arc<RwLock<HashMap<String, ExtensionMetadata>>>,
    asset_handlers: Arc<RwLock<HashMap<AssetType, Box<dyn AssetExtensionHandler>>>>,
    dependencies: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    load_order: Arc<RwLock<Vec<String>>>,
    asset_manager: Arc<AssetManager>,
    config: ExtensionManagerConfig,
}

impl ExtensionManager {
    /// Create new extension manager
    pub fn new(asset_manager: Arc<AssetManager>, config: ExtensionManagerConfig) -> Self {
        Self {
            extensions: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(RwLock::new(HashMap::new())),
            asset_handlers: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            load_order: Arc::new(RwLock::new(Vec::new())),
            asset_manager,
            config,
        }
    }

    /// Load an extension
    pub async fn load_extension(&self, mut extension: Box<dyn HyperMeshExtension>) -> ExtensionResult<()> {
        let metadata = extension.metadata();
        let extension_id = metadata.id.clone();

        // Check if already loaded
        {
            let extensions = self.extensions.read().await;
            if extensions.contains_key(&extension_id) {
                return Err(ExtensionError::ExtensionAlreadyLoaded { id: extension_id });
            }
        }

        // Verify dependencies
        self.verify_dependencies(&metadata).await?;

        // Verify signature if required
        if self.config.verify_signatures {
            if let Some(fingerprint) = &metadata.certificate_fingerprint {
                self.verify_certificate(fingerprint).await?;
            }
        }

        // Create extension configuration
        let config = ExtensionConfig {
            settings: serde_json::Value::Null,
            resource_limits: self.config.global_limits.clone(),
            granted_capabilities: metadata.required_capabilities.intersection(&self.config.allowed_capabilities).cloned().collect(),
            privacy_level: crate::assets::core::PrivacyLevel::Private,
            debug_mode: false,
        };

        // Initialize extension
        extension.initialize(config).await?;

        // Register asset handlers
        let handlers = extension.register_assets().await?;
        {
            let mut asset_handlers = self.asset_handlers.write().await;
            for (asset_type, handler) in handlers {
                asset_handlers.insert(asset_type, handler);
            }
        }

        // Extend asset manager
        extension.extend_manager(self.asset_manager.clone()).await?;

        // Store extension
        {
            let mut extensions = self.extensions.write().await;
            extensions.insert(extension_id.clone(), extension);
        }

        // Update registry
        {
            let mut registry = self.registry.write().await;
            registry.insert(extension_id.clone(), metadata);
        }

        // Update load order
        {
            let mut load_order = self.load_order.write().await;
            load_order.push(extension_id);
        }

        Ok(())
    }

    /// Unload an extension
    pub async fn unload_extension(&self, extension_id: &str) -> ExtensionResult<()> {
        let mut extension = {
            let mut extensions = self.extensions.write().await;
            extensions.remove(extension_id)
                .ok_or_else(|| ExtensionError::ExtensionNotFound { id: extension_id.to_string() })?
        };

        extension.shutdown().await?;

        {
            let mut registry = self.registry.write().await;
            registry.remove(extension_id);
        }

        {
            let mut load_order = self.load_order.write().await;
            load_order.retain(|id| id != extension_id);
        }

        Ok(())
    }

    /// List loaded extensions
    pub async fn list_extensions(&self) -> Vec<ExtensionMetadata> {
        let registry = self.registry.read().await;
        registry.values().cloned().collect()
    }

    /// Verify extension dependencies
    async fn verify_dependencies(&self, metadata: &ExtensionMetadata) -> ExtensionResult<()> {
        let registry = self.registry.read().await;

        for dep in &metadata.dependencies {
            if !dep.optional {
                let installed = registry.get(&dep.extension_id)
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

    /// Verify extension certificate
    async fn verify_certificate(&self, _fingerprint: &str) -> ExtensionResult<()> {
        // TODO: Implement TrustChain certificate verification
        Ok(())
    }

    /// Auto-discover and load extensions
    pub async fn auto_load_extensions(&self) -> ExtensionResult<Vec<String>> {
        if !self.config.auto_load {
            return Ok(Vec::new());
        }

        let loaded = Vec::new();

        for dir in &self.config.extension_dirs {
            if !dir.exists() {
                continue;
            }
            // TODO: Implement extension discovery from filesystem
        }

        Ok(loaded)
    }
}
