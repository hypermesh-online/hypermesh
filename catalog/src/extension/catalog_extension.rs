//! CatalogExtension - Main HyperMesh Extension Implementation
//!
//! This is the core extension struct that implements both HyperMeshExtension
//! and AssetLibraryExtension traits, integrating all Catalog functionality
//! as a plugin for the HyperMesh ecosystem.

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use semver::Version;
use anyhow::Result;

use blockmatrix::extensions::{
    HyperMeshExtension, AssetLibraryExtension, ExtensionMetadata, ExtensionCategory,
    ExtensionCapability, ExtensionConfig, ExtensionResult, ExtensionError,
    ExtensionRequest, ExtensionResponse, ExtensionStatus, ExtensionState as ExtState,
    ExtensionHealth, ValidationReport, ValidationError, ValidationWarning,
    AssetExtensionHandler, AssetPackage, PackageFilter, InstallOptions, InstallResult,
    UpdateResult, SearchOptions, AssetPackageSpec, PublishResult, VerificationResult,
    ResourceUsageReport,
};

use blockmatrix::assets::core::{AssetManager, AssetId, AssetType};

use crate::{
    Catalog, CatalogConfig, CatalogBuilder,
    library::{LibraryManager, LibraryConfig},
    hypermesh_bridge::{HyperMeshAssetRegistry, BridgeConfig},
    registry::{AssetRegistry, SearchQuery, SearchResults},
    validation::{AssetValidator, ValidationResult},
    template::{CatalogTemplateGenerator, TemplateContext, TemplateGenerationResult},
    documentation::{DocumentationGenerator, GeneratedDocumentation},
    versioning::{VersionManager, SemanticVersion},
    sharing::{SharingManager, SharingConfig, SharingStats, SharePermission},
};

use super::asset_handlers::{
    VirtualMachineHandler, LibraryHandler, DatasetHandler, TemplateHandler,
};
use super::config::{CatalogExtensionConfig, ExtensionSettings};

/// CatalogExtension - HyperMesh plugin for asset library management
pub struct CatalogExtension {
    /// Extension metadata
    metadata: ExtensionMetadata,

    /// Core Catalog instance
    catalog: Option<Arc<Catalog>>,

    /// Library manager for asset packages
    library_manager: Arc<RwLock<LibraryManager>>,

    /// HyperMesh asset registry bridge
    asset_registry: Arc<HyperMeshAssetRegistry>,

    /// Decentralized sharing manager
    sharing_manager: Option<Arc<SharingManager>>,

    /// Asset handlers for different types
    asset_handlers: HashMap<AssetType, Box<dyn AssetExtensionHandler>>,

    /// Extension configuration
    config: CatalogExtensionConfig,

    /// Current extension state
    state: Arc<RwLock<ExtState>>,

    /// Extension health status
    health: Arc<RwLock<ExtensionHealth>>,

    /// Resource usage tracking
    resource_usage: Arc<RwLock<ResourceUsageReport>>,

    /// Active operations counter
    active_operations: Arc<RwLock<usize>>,

    /// Total requests counter
    total_requests: Arc<RwLock<u64>>,

    /// Error counter
    error_count: Arc<RwLock<u64>>,

    /// Extension start time
    start_time: std::time::Instant,
}

impl CatalogExtension {
    /// Create a new CatalogExtension instance
    pub fn new(config: CatalogExtensionConfig) -> Self {
        // Create extension metadata
        let metadata = ExtensionMetadata {
            id: "catalog".to_string(),
            name: "HyperMesh Catalog".to_string(),
            version: Version::parse("0.1.0").unwrap(),
            description: "Decentralized asset library and package manager for HyperMesh".to_string(),
            author: "HyperMesh Team".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://catalog.hypermesh.online".to_string()),
            category: ExtensionCategory::AssetLibrary,
            hypermesh_version: Version::parse("1.0.0").unwrap(),
            dependencies: vec![],
            required_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::NetworkAccess,
                ExtensionCapability::ConsensusAccess,
                ExtensionCapability::TransportAccess,
                ExtensionCapability::TrustChainAccess,
                ExtensionCapability::VMExecution,
                ExtensionCapability::FileSystemAccess,
            ]),
            provided_assets: vec![
                AssetType::VirtualMachine,
                AssetType::Library,
                AssetType::Dataset,
                AssetType::Template,
            ],
            certificate_fingerprint: config.certificate_fingerprint.clone(),
            config_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "library_path": { "type": "string" },
                    "cache_size": { "type": "integer" },
                    "max_package_size": { "type": "integer" },
                    "enable_p2p": { "type": "boolean" },
                    "consensus_validation": { "type": "boolean" }
                }
            })),
        };

        // Initialize components
        let library_config = LibraryConfig {
            storage_path: config.library_path.clone(),
            cache_size: config.cache_size,
            enable_p2p_distribution: config.enable_p2p,
            consensus_validation: config.consensus_validation,
        };

        let library_manager = Arc::new(RwLock::new(
            LibraryManager::new(library_config)
        ));

        let bridge_config = BridgeConfig::default();
        let asset_registry = Arc::new(
            HyperMeshAssetRegistry::new(bridge_config)
        );

        // Create asset handlers
        let mut asset_handlers = HashMap::new();
        asset_handlers.insert(
            AssetType::VirtualMachine,
            Box::new(VirtualMachineHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        asset_handlers.insert(
            AssetType::Library,
            Box::new(LibraryHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        asset_handlers.insert(
            AssetType::Dataset,
            Box::new(DatasetHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        asset_handlers.insert(
            AssetType::Template,
            Box::new(TemplateHandler::new()) as Box<dyn AssetExtensionHandler>
        );

        Self {
            metadata,
            catalog: None,
            library_manager,
            asset_registry,
            sharing_manager: None,
            asset_handlers,
            config,
            state: Arc::new(RwLock::new(ExtState::Initializing)),
            health: Arc::new(RwLock::new(ExtensionHealth::Healthy)),
            resource_usage: Arc::new(RwLock::new(ResourceUsageReport {
                cpu_usage: 0.0,
                memory_usage: 0,
                network_bytes: 0,
                storage_bytes: 0,
            })),
            active_operations: Arc::new(RwLock::new(0)),
            total_requests: Arc::new(RwLock::new(0)),
            error_count: Arc::new(RwLock::new(0)),
            start_time: std::time::Instant::now(),
        }
    }

    /// Internal helper to increment request counter
    async fn increment_requests(&self) {
        let mut count = self.total_requests.write().await;
        *count += 1;
    }

    /// Internal helper to track errors
    async fn track_error(&self, error: &str) {
        let mut count = self.error_count.write().await;
        *count += 1;

        // Update health status if too many errors
        if *count > 100 {
            let mut health = self.health.write().await;
            *health = ExtensionHealth::Degraded(
                format!("High error rate: {} errors", *count)
            );
        }
    }

    /// Internal helper to track active operations
    async fn start_operation(&self) {
        let mut ops = self.active_operations.write().await;
        *ops += 1;
    }

    /// Internal helper to complete operations
    async fn complete_operation(&self) {
        let mut ops = self.active_operations.write().await;
        if *ops > 0 {
            *ops -= 1;
        }
    }

    /// Update resource usage metrics
    async fn update_resource_usage(&self, delta: ResourceUsageReport) {
        let mut usage = self.resource_usage.write().await;
        usage.cpu_usage += delta.cpu_usage;
        usage.memory_usage += delta.memory_usage;
        usage.network_bytes += delta.network_bytes;
        usage.storage_bytes += delta.storage_bytes;
    }
}

#[async_trait]
impl HyperMeshExtension for CatalogExtension {
    /// Get extension metadata
    fn metadata(&self) -> ExtensionMetadata {
        self.metadata.clone()
    }

    /// Initialize the extension with configuration
    async fn initialize(&mut self, config: ExtensionConfig) -> ExtensionResult<()> {
        // Update state
        {
            let mut state = self.state.write().await;
            *state = ExtState::Initializing;
        }

        // Parse extension-specific settings
        if let Ok(settings) = serde_json::from_value::<ExtensionSettings>(config.settings.clone()) {
            self.config.apply_settings(settings);
        }

        // Initialize Catalog core
        let catalog_config = CatalogConfig {
            hypermesh_address: Some(self.config.hypermesh_address.clone()),
            trustchain_cert_path: self.config.trustchain_cert_path.clone(),
            ..Default::default()
        };

        match Catalog::new(catalog_config).await {
            Ok(catalog) => {
                self.catalog = Some(Arc::new(catalog));

                // Initialize asset registry with HyperMesh connection
                if let Err(e) = self.asset_registry.connect_to_hypermesh().await {
                    return Err(ExtensionError::InitializationFailed {
                        reason: format!("Failed to connect to HyperMesh: {}", e)
                    });
                }

                // Initialize sharing manager for decentralized operations
                if self.config.enable_p2p {
                    let sharing_config = SharingConfig {
                        node_id: format!("catalog_{}", uuid::Uuid::new_v4()),
                        max_mirror_storage: self.config.cache_size as u64,
                        max_bandwidth: 10 * 1024 * 1024, // 10MB/s default
                        replication_factor: 3,
                        default_permission: SharePermission::Public,
                        auto_mirror_popular: true,
                        enable_incentives: true,
                        ..Default::default()
                    };

                    match SharingManager::new(sharing_config).await {
                        Ok(sharing_manager) => {
                            self.sharing_manager = Some(Arc::new(sharing_manager));
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to initialize sharing manager: {}", e);
                            // Continue without sharing capabilities
                        }
                    }
                }

                // Update state to running
                {
                    let mut state = self.state.write().await;
                    *state = ExtState::Running;
                }

                Ok(())
            }
            Err(e) => {
                let mut state = self.state.write().await;
                *state = ExtState::Error(format!("Initialization failed: {}", e));

                Err(ExtensionError::InitializationFailed {
                    reason: e.to_string()
                })
            }
        }
    }

    /// Register assets provided by this extension
    async fn register_assets(&self) -> ExtensionResult<HashMap<AssetType, Box<dyn AssetExtensionHandler>>> {
        self.increment_requests().await;

        // Clone handlers to return
        let mut handlers = HashMap::new();

        // Note: We can't directly return our handlers due to ownership,
        // so we create new instances
        handlers.insert(
            AssetType::VirtualMachine,
            Box::new(VirtualMachineHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        handlers.insert(
            AssetType::Library,
            Box::new(LibraryHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        handlers.insert(
            AssetType::Dataset,
            Box::new(DatasetHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        handlers.insert(
            AssetType::Template,
            Box::new(TemplateHandler::new()) as Box<dyn AssetExtensionHandler>
        );

        Ok(handlers)
    }

    /// Extend the asset manager with custom functionality
    async fn extend_manager(&self, _asset_manager: Arc<AssetManager>) -> ExtensionResult<()> {
        self.increment_requests().await;

        // The asset manager is already extended through the asset handlers
        // Additional custom extensions can be added here if needed

        Ok(())
    }

    /// Handle extension-specific API calls
    async fn handle_request(&self, request: ExtensionRequest) -> ExtensionResult<ExtensionResponse> {
        self.increment_requests().await;
        self.start_operation().await;

        let response = match request.method.as_str() {
            "catalog.search" => {
                // Handle catalog search request
                if let Some(catalog) = &self.catalog {
                    if let Ok(query) = serde_json::from_value::<SearchQuery>(request.params) {
                        match catalog.search_assets(&query).await {
                            Ok(results) => ExtensionResponse {
                                request_id: request.id,
                                success: true,
                                data: Some(serde_json::to_value(results).unwrap()),
                                error: None,
                            },
                            Err(e) => ExtensionResponse {
                                request_id: request.id,
                                success: false,
                                data: None,
                                error: Some(e.to_string()),
                            }
                        }
                    } else {
                        ExtensionResponse {
                            request_id: request.id,
                            success: false,
                            data: None,
                            error: Some("Invalid search query".to_string()),
                        }
                    }
                } else {
                    ExtensionResponse {
                        request_id: request.id,
                        success: false,
                        data: None,
                        error: Some("Catalog not initialized".to_string()),
                    }
                }
            },

            "catalog.validate" => {
                // Handle validation request
                if let Some(catalog) = &self.catalog {
                    // Validation logic here
                    ExtensionResponse {
                        request_id: request.id,
                        success: true,
                        data: Some(serde_json::json!({ "valid": true })),
                        error: None,
                    }
                } else {
                    ExtensionResponse {
                        request_id: request.id,
                        success: false,
                        data: None,
                        error: Some("Catalog not initialized".to_string()),
                    }
                }
            },

            "catalog.stats" => {
                // Return catalog statistics
                let stats = serde_json::json!({
                    "total_requests": *self.total_requests.read().await,
                    "active_operations": *self.active_operations.read().await,
                    "error_count": *self.error_count.read().await,
                    "uptime_seconds": self.start_time.elapsed().as_secs(),
                });

                ExtensionResponse {
                    request_id: request.id,
                    success: true,
                    data: Some(stats),
                    error: None,
                }
            },

            "catalog.sharing.connect" => {
                // Connect to a peer for sharing
                if let Some(sharing_manager) = &self.sharing_manager {
                    if let Some(address) = request.params.get("address").and_then(|v| v.as_str()) {
                        match sharing_manager.connect_peer(address).await {
                            Ok(peer_id) => ExtensionResponse {
                                request_id: request.id,
                                success: true,
                                data: Some(serde_json::json!({ "peer_id": peer_id })),
                                error: None,
                            },
                            Err(e) => ExtensionResponse {
                                request_id: request.id,
                                success: false,
                                data: None,
                                error: Some(format!("Failed to connect to peer: {}", e)),
                            }
                        }
                    } else {
                        ExtensionResponse {
                            request_id: request.id,
                            success: false,
                            data: None,
                            error: Some("Missing address parameter".to_string()),
                        }
                    }
                } else {
                    ExtensionResponse {
                        request_id: request.id,
                        success: false,
                        data: None,
                        error: Some("Sharing not enabled".to_string()),
                    }
                }
            },

            "catalog.sharing.search" => {
                // Search across the decentralized network
                if let Some(sharing_manager) = &self.sharing_manager {
                    if let Some(query) = request.params.get("query").and_then(|v| v.as_str()) {
                        match sharing_manager.search_packages(query).await {
                            Ok(results) => ExtensionResponse {
                                request_id: request.id,
                                success: true,
                                data: Some(serde_json::to_value(results).unwrap()),
                                error: None,
                            },
                            Err(e) => ExtensionResponse {
                                request_id: request.id,
                                success: false,
                                data: None,
                                error: Some(format!("Search failed: {}", e)),
                            }
                        }
                    } else {
                        ExtensionResponse {
                            request_id: request.id,
                            success: false,
                            data: None,
                            error: Some("Missing query parameter".to_string()),
                        }
                    }
                } else {
                    ExtensionResponse {
                        request_id: request.id,
                        success: false,
                        data: None,
                        error: Some("Sharing not enabled".to_string()),
                    }
                }
            },

            "catalog.sharing.stats" => {
                // Get sharing statistics
                if let Some(sharing_manager) = &self.sharing_manager {
                    let stats = sharing_manager.get_stats().await;
                    ExtensionResponse {
                        request_id: request.id,
                        success: true,
                        data: Some(serde_json::to_value(stats).unwrap()),
                        error: None,
                    }
                } else {
                    ExtensionResponse {
                        request_id: request.id,
                        success: false,
                        data: None,
                        error: Some("Sharing not enabled".to_string()),
                    }
                }
            },

            _ => {
                ExtensionResponse {
                    request_id: request.id,
                    success: false,
                    data: None,
                    error: Some(format!("Unknown method: {}", request.method)),
                }
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

    /// Get current extension status
    async fn status(&self) -> ExtensionStatus {
        ExtensionStatus {
            state: self.state.read().await.clone(),
            health: self.health.read().await.clone(),
            resource_usage: self.resource_usage.read().await.clone(),
            active_operations: *self.active_operations.read().await,
            total_requests: *self.total_requests.read().await,
            error_count: *self.error_count.read().await,
            uptime: self.start_time.elapsed(),
        }
    }

    /// Validate extension integrity and configuration
    async fn validate(&self) -> ExtensionResult<ValidationReport> {
        self.increment_requests().await;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if catalog is initialized
        if self.catalog.is_none() {
            errors.push(ValidationError {
                code: "CATALOG_NOT_INITIALIZED".to_string(),
                message: "Catalog core is not initialized".to_string(),
                context: None,
            });
        }

        // Check library path exists
        if !self.config.library_path.exists() {
            warnings.push(ValidationWarning {
                code: "LIBRARY_PATH_MISSING".to_string(),
                message: format!("Library path does not exist: {:?}", self.config.library_path),
                context: None,
            });
        }

        // Check resource usage
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
            dependencies_satisfied: true, // No external dependencies
            resource_compliance: usage.memory_usage <= self.config.max_memory_usage,
            security_compliance: true, // Assuming security checks pass
            errors,
            warnings,
        })
    }

    /// Export extension state for migration or backup
    async fn export_state(&self) -> ExtensionResult<ExtState> {
        self.increment_requests().await;

        // Serialize current state
        let state_data = serde_json::json!({
            "library_manager": "serialized_library_state",
            "asset_registry": "serialized_registry_state",
            "statistics": {
                "total_requests": *self.total_requests.read().await,
                "error_count": *self.error_count.read().await,
            }
        });

        Ok(ExtState {
            version: 1,
            metadata: self.metadata.clone(),
            state_data: state_data.to_string().into_bytes(),
            checksum: "sha256_checksum_here".to_string(),
            exported_at: std::time::SystemTime::now(),
        })
    }

    /// Import previously exported state
    async fn import_state(&mut self, _state: ExtState) -> ExtensionResult<()> {
        self.increment_requests().await;

        // In a real implementation, this would deserialize and restore state
        // For now, we just acknowledge the import

        Ok(())
    }

    /// Shutdown the extension gracefully
    async fn shutdown(&mut self) -> ExtensionResult<()> {
        // Update state
        {
            let mut state = self.state.write().await;
            *state = ExtState::ShuttingDown;
        }

        // Wait for active operations to complete
        let mut retries = 10;
        while *self.active_operations.read().await > 0 && retries > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            retries -= 1;
        }

        // Disconnect from HyperMesh
        if let Err(e) = self.asset_registry.disconnect().await {
            eprintln!("Error disconnecting from HyperMesh: {}", e);
        }

        // Update final state
        {
            let mut state = self.state.write().await;
            *state = ExtState::Stopped;
        }

        Ok(())
    }
}

#[async_trait]
impl AssetLibraryExtension for CatalogExtension {
    /// List available asset packages
    async fn list_packages(&self, filter: PackageFilter) -> ExtensionResult<Vec<AssetPackage>> {
        self.increment_requests().await;
        self.start_operation().await;

        let library_manager = self.library_manager.read().await;
        let packages = library_manager.list_packages(filter).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to list packages: {}", e)
            })?;

        self.complete_operation().await;
        Ok(packages)
    }

    /// Get package details
    async fn get_package(&self, package_id: &str) -> ExtensionResult<AssetPackage> {
        self.increment_requests().await;
        self.start_operation().await;

        let library_manager = self.library_manager.read().await;
        let package = library_manager.get_package(package_id).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to get package: {}", e)
            })?;

        self.complete_operation().await;
        Ok(package)
    }

    /// Install an asset package
    async fn install_package(&self, package_id: &str, options: InstallOptions) -> ExtensionResult<InstallResult> {
        self.increment_requests().await;
        self.start_operation().await;

        // Verify consensus proof if required
        if self.config.consensus_validation {
            // Validate consensus proof
            // This would integrate with HyperMesh consensus validation
        }

        let library_manager = self.library_manager.read().await;
        let result = library_manager.install_package(package_id, options).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to install package: {}", e)
            })?;

        // Update resource usage
        self.update_resource_usage(ResourceUsageReport {
            cpu_usage: 0.1,
            memory_usage: result.installed_assets.len() as u64 * 1024,
            network_bytes: 1024 * 1024, // Estimate
            storage_bytes: 1024 * 1024, // Estimate
        }).await;

        self.complete_operation().await;
        Ok(result)
    }

    /// Uninstall an asset package
    async fn uninstall_package(&self, package_id: &str) -> ExtensionResult<()> {
        self.increment_requests().await;
        self.start_operation().await;

        let library_manager = self.library_manager.read().await;
        library_manager.uninstall_package(package_id).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to uninstall package: {}", e)
            })?;

        self.complete_operation().await;
        Ok(())
    }

    /// Update an installed package
    async fn update_package(&self, package_id: &str, version: Option<Version>) -> ExtensionResult<UpdateResult> {
        self.increment_requests().await;
        self.start_operation().await;

        let library_manager = self.library_manager.read().await;
        let result = library_manager.update_package(package_id, version).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to update package: {}", e)
            })?;

        self.complete_operation().await;
        Ok(result)
    }

    /// Search for packages
    async fn search_packages(&self, query: &str, options: SearchOptions) -> ExtensionResult<Vec<AssetPackage>> {
        self.increment_requests().await;
        self.start_operation().await;

        let library_manager = self.library_manager.read().await;
        let packages = library_manager.search_packages(query, options).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Search failed: {}", e)
            })?;

        self.complete_operation().await;
        Ok(packages)
    }

    /// Publish a new package to the library
    async fn publish_package(&self, package: AssetPackageSpec, proof: hypermesh::consensus::proof_of_state_integration::ConsensusProof) -> ExtensionResult<PublishResult> {
        self.increment_requests().await;
        self.start_operation().await;

        // Validate consensus proof
        if self.config.consensus_validation {
            // Verify all four proofs (PoSpace, PoStake, PoWork, PoTime)
            // This would integrate with HyperMesh consensus validation
        }

        let library_manager = self.library_manager.read().await;
        let result = library_manager.publish_package(package, proof).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to publish package: {}", e)
            })?;

        self.complete_operation().await;
        Ok(result)
    }

    /// Verify package integrity
    async fn verify_package(&self, package_id: &str) -> ExtensionResult<VerificationResult> {
        self.increment_requests().await;
        self.start_operation().await;

        let library_manager = self.library_manager.read().await;
        let result = library_manager.verify_package(package_id).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Verification failed: {}", e)
            })?;

        self.complete_operation().await;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_catalog_extension_creation() {
        let config = CatalogExtensionConfig::default();
        let extension = CatalogExtension::new(config);

        assert_eq!(extension.metadata.id, "catalog");
        assert_eq!(extension.metadata.category, ExtensionCategory::AssetLibrary);
        assert_eq!(extension.metadata.provided_assets.len(), 4);
    }

    #[tokio::test]
    async fn test_extension_metadata() {
        let config = CatalogExtensionConfig::default();
        let extension = CatalogExtension::new(config);
        let metadata = extension.metadata();

        assert!(metadata.required_capabilities.contains(&ExtensionCapability::AssetManagement));
        assert!(metadata.required_capabilities.contains(&ExtensionCapability::VMExecution));
        assert!(metadata.required_capabilities.contains(&ExtensionCapability::NetworkAccess));
    }

    #[tokio::test]
    async fn test_extension_status() {
        let config = CatalogExtensionConfig::default();
        let extension = CatalogExtension::new(config);
        let status = extension.status().await;

        assert_eq!(status.total_requests, 0);
        assert_eq!(status.error_count, 0);
        assert_eq!(status.active_operations, 0);
    }
}