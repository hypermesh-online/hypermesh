// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Extension registry for HyperMesh
//!
//! Manages the registration, discovery, and lifecycle of loaded extensions.

pub mod types;

pub use types::{
    RegistryEntry, ExtensionEntryState, ExtensionLocation, HealthStatus,
    HealthState, ExtensionMetrics, RegistryConfig, DependencyGraph,
    RegistryListener, SearchCriteria,
};

use semver::{Version, VersionReq};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use super::{
    AssetExtensionHandler, ExtensionCategory, ExtensionError, ExtensionMetadata,
    ExtensionResult, ExtensionStatus, HyperMeshExtension,
};

use crate::assets::core::AssetType;

/// Extension registry
pub struct ExtensionRegistry {
    /// Registry entries
    entries: Arc<RwLock<HashMap<String, RegistryEntry>>>,

    /// Extension instances
    extensions: Arc<RwLock<HashMap<String, Arc<dyn HyperMeshExtension>>>>,

    /// Asset handlers by type
    handlers: Arc<RwLock<HashMap<AssetType, Arc<dyn AssetExtensionHandler>>>>,

    /// Dependency graph
    dependencies: Arc<RwLock<DependencyGraph>>,

    /// Category index
    categories: Arc<RwLock<HashMap<ExtensionCategory, HashSet<String>>>>,

    /// Registry configuration
    config: RegistryConfig,

    /// Event listeners
    listeners: Arc<RwLock<Vec<Arc<dyn RegistryListener>>>>,
}

impl ExtensionRegistry {
    /// Create new extension registry
    pub fn new(config: RegistryConfig) -> Self {
        let registry = Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            extensions: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(DependencyGraph::default())),
            categories: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            listeners: Arc::new(RwLock::new(Vec::new())),
        };

        // Start health monitoring if enabled
        if config.health_monitoring {
            let registry_clone = registry.clone_for_monitoring();
            tokio::spawn(async move {
                registry_clone.health_monitoring_loop().await;
            });
        }

        registry
    }

    /// Clone registry for monitoring task
    fn clone_for_monitoring(&self) -> ExtensionRegistry {
        Self {
            entries: self.entries.clone(),
            extensions: self.extensions.clone(),
            handlers: self.handlers.clone(),
            dependencies: self.dependencies.clone(),
            categories: self.categories.clone(),
            config: self.config.clone(),
            listeners: self.listeners.clone(),
        }
    }

    /// Register an extension
    pub async fn register_extension(
        &self,
        metadata: ExtensionMetadata,
        location: ExtensionLocation,
    ) -> ExtensionResult<()> {
        let extension_id = metadata.id.clone();

        // Check registry capacity
        {
            let entries = self.entries.read().await;
            if entries.len() >= self.config.max_entries {
                return Err(ExtensionError::ResourceLimitExceeded {
                    resource: format!("Registry capacity: {}", self.config.max_entries),
                });
            }

            if entries.contains_key(&extension_id) {
                return Err(ExtensionError::ExtensionAlreadyLoaded {
                    id: extension_id,
                });
            }
        }

        // Check dependency cycle
        if self.config.auto_resolve_deps {
            let deps: Vec<String> = metadata.dependencies.iter()
                .map(|d| d.extension_id.clone())
                .collect();

            let dep_graph = self.dependencies.read().await;
            if dep_graph.would_create_cycle(&extension_id, &deps) {
                return Err(ExtensionError::DependencyResolutionFailed {
                    extension: extension_id,
                    dependency: "Circular dependency detected".to_string(),
                });
            }
        }

        // Create registry entry
        let entry = RegistryEntry {
            metadata: metadata.clone(),
            state: ExtensionEntryState::Registered,
            registered_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            location,
            health: HealthStatus {
                state: HealthState::Unknown,
                last_check: std::time::SystemTime::now(),
                failures: 0,
                errors: Vec::new(),
            },
            metrics: ExtensionMetrics::default(),
        };

        {
            let mut entries = self.entries.write().await;
            entries.insert(extension_id.clone(), entry);
        }

        {
            let mut categories = self.categories.write().await;
            categories.entry(metadata.category.clone())
                .or_default()
                .insert(extension_id.clone());
        }

        if self.config.auto_resolve_deps {
            let deps: Vec<String> = metadata.dependencies.iter()
                .map(|d| d.extension_id.clone())
                .collect();

            let mut dep_graph = self.dependencies.write().await;
            dep_graph.add_extension(extension_id.clone(), deps);
        }

        self.notify_registered(&extension_id, &metadata).await;

        info!("Registered extension: {}", extension_id);
        Ok(())
    }

    /// Activate a registered extension
    pub async fn activate_extension(
        &self,
        extension_id: &str,
        extension: Arc<dyn HyperMeshExtension>,
    ) -> ExtensionResult<()> {
        {
            let mut entries = self.entries.write().await;
            let entry = entries.get_mut(extension_id)
                .ok_or_else(|| ExtensionError::ExtensionNotFound {
                    id: extension_id.to_string(),
                })?;

            entry.state = ExtensionEntryState::Active;
            entry.updated_at = std::time::SystemTime::now();
        }

        {
            let mut extensions = self.extensions.write().await;
            extensions.insert(extension_id.to_string(), extension.clone());
        }

        let handlers = extension.register_assets().await?;
        {
            let mut handler_map = self.handlers.write().await;
            for (asset_type, handler) in handlers {
                handler_map.insert(asset_type, Arc::from(handler));
            }
        }

        self.notify_loaded(extension_id).await;

        info!("Activated extension: {}", extension_id);
        Ok(())
    }

    /// Deactivate an extension
    pub async fn deactivate_extension(&self, extension_id: &str) -> ExtensionResult<()> {
        {
            let dep_graph = self.dependencies.read().await;
            if let Some(dependents) = dep_graph.reverse.get(extension_id) {
                if !dependents.is_empty() {
                    return Err(ExtensionError::DependencyResolutionFailed {
                        extension: extension_id.to_string(),
                        dependency: format!("Extension has active dependents: {:?}", dependents),
                    });
                }
            }
        }

        {
            let mut entries = self.entries.write().await;
            let entry = entries.get_mut(extension_id)
                .ok_or_else(|| ExtensionError::ExtensionNotFound {
                    id: extension_id.to_string(),
                })?;

            entry.state = ExtensionEntryState::Unloading;
            entry.updated_at = std::time::SystemTime::now();
        }

        {
            let mut extensions = self.extensions.write().await;
            extensions.remove(extension_id);
        }

        {
            let mut entries = self.entries.write().await;
            if let Some(entry) = entries.get_mut(extension_id) {
                entry.state = ExtensionEntryState::Registered;
            }
        }

        self.notify_unloaded(extension_id).await;

        info!("Deactivated extension: {}", extension_id);
        Ok(())
    }

    /// Unregister an extension completely
    pub async fn unregister_extension(&self, extension_id: &str) -> ExtensionResult<()> {
        if self.is_active(extension_id).await {
            self.deactivate_extension(extension_id).await?;
        }

        {
            let mut entries = self.entries.write().await;
            let entry = entries.remove(extension_id)
                .ok_or_else(|| ExtensionError::ExtensionNotFound {
                    id: extension_id.to_string(),
                })?;

            let mut categories = self.categories.write().await;
            if let Some(cat_set) = categories.get_mut(&entry.metadata.category) {
                cat_set.remove(extension_id);
            }
        }

        {
            let mut dep_graph = self.dependencies.write().await;
            dep_graph.remove_extension(extension_id);
        }

        info!("Unregistered extension: {}", extension_id);
        Ok(())
    }

    /// Get extension metadata
    pub async fn get_metadata(&self, extension_id: &str) -> Option<ExtensionMetadata> {
        let entries = self.entries.read().await;
        entries.get(extension_id).map(|e| e.metadata.clone())
    }

    /// Get extension instance
    pub async fn get_extension(&self, extension_id: &str) -> Option<Arc<dyn HyperMeshExtension>> {
        let extensions = self.extensions.read().await;
        extensions.get(extension_id).cloned()
    }

    /// Get asset handler for type
    pub async fn get_handler(&self, asset_type: &AssetType) -> Option<Arc<dyn AssetExtensionHandler>> {
        let handlers = self.handlers.read().await;
        handlers.get(asset_type).cloned()
    }

    /// List all registered extensions
    pub async fn list_extensions(&self) -> Vec<RegistryEntry> {
        let entries = self.entries.read().await;
        entries.values().cloned().collect()
    }

    /// List extensions by category
    pub async fn list_by_category(&self, category: &ExtensionCategory) -> Vec<String> {
        let categories = self.categories.read().await;
        categories.get(category)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get dependency load order
    pub async fn get_load_order(&self) -> Vec<String> {
        let dep_graph = self.dependencies.read().await;
        dep_graph.get_load_order()
    }

    /// Check if extension is active
    pub async fn is_active(&self, extension_id: &str) -> bool {
        let entries = self.entries.read().await;
        entries.get(extension_id)
            .map(|e| e.state == ExtensionEntryState::Active)
            .unwrap_or(false)
    }

    /// Update extension health
    pub async fn update_health(&self, extension_id: &str, state: HealthState) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(extension_id) {
            let old_state = entry.health.state.clone();
            entry.health.state = state.clone();
            entry.health.last_check = std::time::SystemTime::now();

            if state != HealthState::Healthy {
                entry.health.failures += 1;
            } else {
                entry.health.failures = 0;
                entry.health.errors.clear();
            }

            if old_state != state {
                let id = extension_id.to_string();
                let health_state = state.clone();
                let listeners = self.listeners.clone();

                tokio::spawn(async move {
                    let listeners = listeners.read().await;
                    for listener in listeners.iter() {
                        listener.on_health_changed(&id, &health_state).await;
                    }
                });
            }
        }
    }

    /// Add registry listener
    pub async fn add_listener(&self, listener: Arc<dyn RegistryListener>) {
        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
    }

    /// Health monitoring loop
    async fn health_monitoring_loop(&self) {
        let mut interval = tokio::time::interval(self.config.health_check_interval);

        loop {
            interval.tick().await;

            let extension_ids: Vec<String> = {
                let entries = self.entries.read().await;
                entries.iter()
                    .filter(|(_, e)| e.state == ExtensionEntryState::Active)
                    .map(|(id, _)| id.clone())
                    .collect()
            };

            for id in extension_ids {
                if let Some(extension) = self.get_extension(&id).await {
                    match extension.status().await {
                        ExtensionStatus { health: super::ExtensionHealth::Healthy, .. } => {
                            self.update_health(&id, HealthState::Healthy).await;
                        }
                        ExtensionStatus { health: super::ExtensionHealth::Degraded(msg), .. } => {
                            self.update_health(&id, HealthState::Degraded).await;
                            warn!("Extension {} degraded: {}", id, msg);
                        }
                        ExtensionStatus { health: super::ExtensionHealth::Unhealthy(msg), .. } => {
                            self.update_health(&id, HealthState::Unhealthy).await;
                            error!("Extension {} unhealthy: {}", id, msg);
                        }
                    }

                    if self.config.collect_metrics {
                        self.update_metrics(&id, &extension).await;
                    }
                }
            }
        }
    }

    /// Update extension metrics
    async fn update_metrics(&self, extension_id: &str, extension: &Arc<dyn HyperMeshExtension>) {
        let status = extension.status().await;

        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(extension_id) {
            entry.metrics.total_requests = status.total_requests;
            entry.metrics.failed_requests = status.error_count;
            entry.metrics.cpu_usage_percent = status.resource_usage.cpu_usage as f32;
            entry.metrics.memory_usage_bytes = status.resource_usage.memory_usage;
            entry.metrics.uptime = status.uptime;
        }
    }

    /// Notify listeners of registration
    async fn notify_registered(&self, id: &str, metadata: &ExtensionMetadata) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener.on_extension_registered(id, metadata).await;
        }
    }

    /// Notify listeners of loading
    async fn notify_loaded(&self, id: &str) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener.on_extension_loaded(id).await;
        }
    }

    /// Notify listeners of unloading
    async fn notify_unloaded(&self, id: &str) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener.on_extension_unloaded(id).await;
        }
    }

    /// Search extensions by criteria
    pub async fn search_extensions(&self, criteria: SearchCriteria) -> Vec<RegistryEntry> {
        let entries = self.entries.read().await;

        entries.values()
            .filter(|entry| {
                if let Some(ref cat) = criteria.category {
                    if entry.metadata.category != *cat {
                        return false;
                    }
                }

                if let Some(ref state) = criteria.state {
                    if entry.state != *state {
                        return false;
                    }
                }

                if let Some(ref health) = criteria.health {
                    if entry.health.state != *health {
                        return false;
                    }
                }

                if let Some(ref pattern) = criteria.name_pattern {
                    if !entry.metadata.name.contains(pattern) {
                        return false;
                    }
                }

                if let Some(ref author) = criteria.author {
                    if !entry.metadata.author.contains(author) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    /// Validate extension compatibility
    pub async fn validate_compatibility(
        &self,
        extension_id: &str,
        hypermesh_version: &Version,
    ) -> ExtensionResult<()> {
        let entries = self.entries.read().await;
        let entry = entries.get(extension_id)
            .ok_or_else(|| ExtensionError::ExtensionNotFound {
                id: extension_id.to_string(),
            })?;

        if !VersionReq::parse(&format!("^{}", entry.metadata.hypermesh_version))
            .unwrap()
            .matches(hypermesh_version)
        {
            return Err(ExtensionError::VersionIncompatible {
                extension: extension_id.to_string(),
                required: entry.metadata.hypermesh_version.to_string(),
                found: hypermesh_version.to_string(),
            });
        }

        for dep in &entry.metadata.dependencies {
            if !dep.optional {
                let dep_entry = entries.get(&dep.extension_id)
                    .ok_or_else(|| ExtensionError::DependencyResolutionFailed {
                        extension: extension_id.to_string(),
                        dependency: dep.extension_id.clone(),
                    })?;

                if !dep.version_requirement.matches(&dep_entry.metadata.version) {
                    return Err(ExtensionError::VersionIncompatible {
                        extension: extension_id.to_string(),
                        required: dep.version_requirement.to_string(),
                        found: dep_entry.metadata.version.to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::default();

        graph.add_extension("ext1".to_string(), vec![]);
        graph.add_extension("ext2".to_string(), vec!["ext1".to_string()]);
        graph.add_extension("ext3".to_string(), vec!["ext1".to_string(), "ext2".to_string()]);

        let order = graph.get_load_order();
        assert_eq!(order[0], "ext1");
        assert!(order.contains(&"ext2".to_string()));
        assert!(order.contains(&"ext3".to_string()));

        assert!(graph.would_create_cycle("ext1", &["ext3".to_string()]));
        assert!(!graph.would_create_cycle("ext4", &["ext1".to_string()]));
    }

    #[tokio::test]
    async fn test_registry_basic() {
        let registry = ExtensionRegistry::new(RegistryConfig::default());

        let metadata = ExtensionMetadata {
            id: "test-ext".to_string(),
            name: "Test Extension".to_string(),
            version: Version::parse("1.0.0").unwrap(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            category: ExtensionCategory::AssetLibrary,
            hypermesh_version: Version::parse("1.0.0").unwrap(),
            dependencies: vec![],
            required_capabilities: HashSet::new(),
            provided_assets: vec![],
            certificate_fingerprint: None,
            config_schema: None,
        };

        let location = ExtensionLocation {
            path: std::path::PathBuf::from("/test/path"),
            url: None,
            distribution_hash: None,
        };

        registry.register_extension(metadata.clone(), location).await.unwrap();

        let retrieved = registry.get_metadata("test-ext").await.unwrap();
        assert_eq!(retrieved.id, "test-ext");

        let list = registry.list_extensions().await;
        assert_eq!(list.len(), 1);
    }
}
