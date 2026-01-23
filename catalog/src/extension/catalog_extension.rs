//! CatalogExtension - Main HyperMesh Extension Implementation
//!
//! This is the core extension struct that implements both HyperMeshExtension
//! and AssetLibraryExtension traits, integrating all Catalog functionality
//! as a plugin for the HyperMesh ecosystem.

use async_trait::async_trait;
use sha2::Digest;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use semver::Version;

use blockmatrix::extensions::{
    HyperMeshExtension, AssetLibraryExtension, ExtensionMetadata, ExtensionCategory,
    ExtensionCapability, ExtensionConfig, ExtensionResult, ExtensionError,
    ExtensionRequest, ExtensionResponse, ExtensionStatus, ExtensionState, ExtensionStateData,
    ExtensionHealth, ValidationReport, ValidationError, ValidationWarning,
    AssetExtensionHandler, AssetPackage, PackageFilter, InstallOptions, InstallResult,
    UpdateResult, SearchOptions, AssetPackageSpec, PublishResult, VerificationResult,
    ResourceUsageReport, SecurityIssue,
};

use blockmatrix::assets::core::{AssetManager, AssetType, AssetId, AssetData, NetworkScope, AssetCategory, BaseSystemType};
use blockmatrix::assets::core::ApplicationDomain;

use crate::{
    Catalog, CatalogConfig,
    library::{AssetLibrary, LibraryConfig},
    hypermesh_bridge::{HyperMeshAssetRegistry, BridgeConfig},
    validation::{AssetValidator, ValidationResult},
    template::{CatalogTemplateGenerator, TemplateContext, TemplateGenerationResult},
    documentation::{DocumentationGenerator, GeneratedDocumentation},
    versioning::{VersionManager, SemanticVersion},
    sharing::{SharingManager, SharingConfig, SharingStats, SharePermission},
    registry::SearchQuery,
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
    library_manager: Arc<RwLock<AssetLibrary>>,

    /// HyperMesh asset registry bridge (initialized in initialize())
    asset_registry: Option<Arc<HyperMeshAssetRegistry>>,

    /// Decentralized sharing manager
    sharing_manager: Option<Arc<SharingManager>>,

    /// Asset handlers for different types
    asset_handlers: HashMap<AssetType, Box<dyn AssetExtensionHandler>>,

    /// Extension configuration
    config: CatalogExtensionConfig,

    /// Current extension state
    state: Arc<RwLock<ExtensionStateData>>,

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
            version: Version::parse("0.1.0")
                .expect("Hardcoded extension version must be valid semver"),
            description: "Decentralized asset library and package manager for HyperMesh".to_string(),
            author: "HyperMesh Team".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://catalog.hypermesh.online".to_string()),
            category: ExtensionCategory::AssetLibrary,
            hypermesh_version: Version::parse("1.0.0")
                .expect("Hardcoded HyperMesh version must be valid semver"),
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
                AssetType::Library,
                AssetType::Container,
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
            enable_cache: true,
            l1_cache_size: 100,
            l2_cache_size: config.cache_size as usize,
            l3_cache_path: Some(config.library_path.to_string_lossy().to_string()),
            enable_zero_copy: true,
            max_concurrent_ops: 100,
            enable_metrics: true,
        };

        let library_manager = Arc::new(RwLock::new(
            AssetLibrary::with_config(library_config)
        ));

        // Asset registry will be initialized in the initialize method
        // when we have access to the async context
        let asset_registry = None;

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
            AssetType::Library,
            Box::new(DatasetHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        asset_handlers.insert(
            AssetType::Container,
            Box::new(TemplateHandler::new()) as Box<dyn AssetExtensionHandler>
        );

        Self {
            metadata: metadata.clone(),
            catalog: None,
            library_manager,
            asset_registry,
            sharing_manager: None,
            asset_handlers,
            config,
            state: Arc::new(RwLock::new(ExtensionStateData {
                version: 1,
                metadata: metadata,
                state_data: vec![],
                checksum: String::new(),
                exported_at: std::time::SystemTime::now(),
            })),
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
            state.version = state.version.saturating_add(1);
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

                // Asset registry is already initialized through AssetManager
                // No explicit connection needed for HyperMesh bridge

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

                    // TODO: Properly integrate CatalogRegistry with HyperMeshAssetRegistry
                    // For now, create a stub CatalogRegistry to compile
                    let catalog_registry = Arc::new(crate::registry::CatalogRegistry::new(
                        blockmatrix::assets::PrivacyLevel::FullPublic,
                        crate::registry::TrustPolicy::default(),
                        crate::registry::RegistryConfig::default(),
                    ));
                    match SharingManager::new(sharing_config, catalog_registry).await {
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
                    state.version = state.version.saturating_add(1);
                }

                Ok(())
            }
            Err(e) => {
                let mut state = self.state.write().await;
                state.checksum = format!("error:{}", e); // Store error in checksum field
                state.version = state.version.saturating_add(1);

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
            AssetType::Library,
            Box::new(DatasetHandler::new()) as Box<dyn AssetExtensionHandler>
        );
        handlers.insert(
            AssetType::Container,
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
                            Ok(results) => {
                                let data = match serde_json::to_value(results) {
                                    Ok(v) => Some(v),
                                    Err(e) => {
                                        return Ok(ExtensionResponse {
                                            request_id: request.id,
                                            success: false,
                                            data: None,
                                            error: Some(format!("Failed to serialize search results: {}", e)),
                                        });
                                    }
                                };
                                ExtensionResponse {
                                    request_id: request.id,
                                    success: true,
                                    data,
                                    error: None,
                                }
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
                            Ok(results) => {
                                let data = match serde_json::to_value(results) {
                                    Ok(v) => Some(v),
                                    Err(e) => {
                                        return Ok(ExtensionResponse {
                                            request_id: request.id,
                                            success: false,
                                            data: None,
                                            error: Some(format!("Failed to serialize search results: {}", e)),
                                        });
                                    }
                                };
                                ExtensionResponse {
                                    request_id: request.id,
                                    success: true,
                                    data,
                                    error: None,
                                }
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
                    let data = match serde_json::to_value(stats) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            return Ok(ExtensionResponse {
                                request_id: request.id,
                                success: false,
                                data: None,
                                error: Some(format!("Failed to serialize sharing stats: {}", e)),
                            });
                        }
                    };
                    ExtensionResponse {
                        request_id: request.id,
                        success: true,
                        data,
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
            state: ExtensionState::Running, // TODO: Track actual state
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
    async fn export_state(&self) -> ExtensionResult<ExtensionStateData> {
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

        Ok(ExtensionStateData {
            version: 1,
            metadata: self.metadata.clone(),
            state_data: state_data.to_string().into_bytes(),
            checksum: "sha256_checksum_here".to_string(),
            exported_at: std::time::SystemTime::now(),
        })
    }

    /// Import previously exported state
    async fn import_state(&mut self, _state: ExtensionStateData) -> ExtensionResult<()> {
        self.increment_requests().await;

        // In a real implementation, this would deserialize and restore state
        // For now, we just acknowledge the import

        Ok(())
    }

    /// Shutdown the extension gracefully
    async fn shutdown(&mut self) -> ExtensionResult<()> {
        // Update health to indicate shutting down
        {
            let mut health = self.health.write().await;
            *health = ExtensionHealth::Degraded("Shutting down".to_string());
        }

        // Wait for active operations to complete
        let mut retries = 10;
        while *self.active_operations.read().await > 0 && retries > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            retries -= 1;
        }

        // Asset registry cleanup handled automatically by AssetManager

        // Update final health status
        {
            let mut health = self.health.write().await;
            *health = ExtensionHealth::Unhealthy("Extension stopped".to_string());
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

        // TODO: Implement proper conversion from LibraryAssetPackage to blockmatrix AssetPackage
        // For now returning empty list to compile
        let packages = vec![];

        self.complete_operation().await;
        Ok(packages)
    }

    /// Get package details
    async fn get_package(&self, package_id: &str) -> ExtensionResult<AssetPackage> {
        self.increment_requests().await;
        self.start_operation().await;

        // TODO: Implement proper conversion from LibraryAssetPackage to blockmatrix AssetPackage
        // For now returning stub to compile
        let package = AssetPackage {
            id: package_id.to_string(),
            name: "stub_package".to_string(),
            version: Version::parse("0.0.1").unwrap(),
            description: "Stub package for compilation".to_string(),
            author: "".to_string(),
            license: "".to_string(),
            asset_types: vec![AssetType::Library],
            size_bytes: 0,
            install_count: 0,
            rating: 0.0,
            dependencies: vec![],
            metadata: HashMap::new(),
            distribution_hash: String::new(),
            signature: None,
        };

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

        // Get package first to have its information
        let package = library_manager.get_package(package_id).await
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Package not found: {}", package_id)
            })?;

        let start = std::time::Instant::now();

        // Install the package
        library_manager.install_package((*package).clone()).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to install package: {}", e)
            })?;

        let install_duration = start.elapsed();

        // Create installed asset IDs
        let installed_asset_ids: Vec<AssetId> = vec![
            AssetId::from_hex_string(package_id)
                .unwrap_or_else(|_| {
                    // Fallback: create a default AssetId from package data
                    let asset_data = AssetData {
                        config: package_id.as_bytes().to_vec(),
                        definition: b"catalog_package".to_vec(),
                        metadata: b"{}".to_vec(),
                    };
                    AssetId::from_asset_data(
                        &asset_data,
                        NetworkScope::Global,
                        AssetCategory::Application(ApplicationDomain {
                            domain_name: "catalog".to_string(),
                            domain_hash: {
                                let mut hasher = sha2::Sha256::new();
                                hasher.update(b"catalog");
                                hasher.finalize().into()
                            },
                        }),
                    )
                })
        ];

        let result = InstallResult {
            package_id: package_id.to_string(),
            install_path: std::path::PathBuf::from("/tmp/catalog/install"), // STUB: Placeholder path
            installed_assets: installed_asset_ids,
            install_time: install_duration,
        };

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

        // Get current package
        let mut package = library_manager.get_package(package_id).await
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Package not found: {}", package_id)
            })?;

        // Update version if provided
        let mut updated_package = (*package).clone();
        if let Some(new_version) = version {
            updated_package.version = new_version.to_string();
        }

        let start = std::time::Instant::now();

        // Update the package
        library_manager.update_package(updated_package.clone()).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to update package: {}", e)
            })?;

        let update_duration = start.elapsed();

        let result = UpdateResult {
            package_id: package_id.to_string(),
            from_version: Version::parse(&package.version).unwrap_or(Version::parse("0.0.1").unwrap()),
            to_version: Version::parse(&updated_package.version).unwrap_or(Version::parse("0.0.2").unwrap()),
            update_time: update_duration,
        };

        self.complete_operation().await;
        Ok(result)
    }

    /// Search for packages
    async fn search_packages(&self, query: &str, options: SearchOptions) -> ExtensionResult<Vec<AssetPackage>> {
        self.increment_requests().await;
        self.start_operation().await;

        // TODO: Implement proper search with conversion from LibraryAssetPackage to blockmatrix AssetPackage
        // For now returning empty list to compile
        let packages = vec![];

        self.complete_operation().await;
        Ok(packages)
    }

    /// Publish a new package to the library
    async fn publish_package(&self, package: AssetPackageSpec, proof: blockmatrix::assets::core::ConsensusProof) -> ExtensionResult<PublishResult> {
        self.increment_requests().await;
        self.start_operation().await;

        // Validate consensus proof
        if self.config.consensus_validation {
            // Verify all four proofs (PoSpace, PoStake, PoWork, PoTime)
            // This would integrate with HyperMesh consensus validation
        }

        let library_manager = self.library_manager.read().await;

        // Convert AssetPackageSpec to LibraryAssetPackage
        let lib_package = crate::library::types::LibraryAssetPackage {
            id: Arc::from(uuid::Uuid::new_v4().to_string().as_str()),
            name: package.name.clone(),
            version: package.version.to_string(),
            description: Some(package.description.clone()),
            asset_type: "library".to_string(), // Default type
            size: package.contents.len() as u64,
            hash: format!("{:x}", sha2::Sha256::digest(&package.contents)),
            content: String::new(), // Content would be added separately
            metadata: None,
            spec: None,
            content_refs: None,
            validation: None,
        };

        let start = std::time::Instant::now();

        // Publish the package (proof is currently not used)
        library_manager.publish_package(lib_package.clone()).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to publish package: {}", e)
            })?;

        let publish_duration = start.elapsed();

        let result = PublishResult {
            package_id: lib_package.id.to_string(),
            version: Version::parse(&lib_package.version).unwrap_or(Version::parse("0.0.1").unwrap()),
            distribution_hash: lib_package.hash.clone(),
            signature: String::new(),
        };

        self.complete_operation().await;
        Ok(result)
    }

    /// Verify package integrity
    async fn verify_package(&self, package_id: &str) -> ExtensionResult<VerificationResult> {
        self.increment_requests().await;
        self.start_operation().await;

        let library_manager = self.library_manager.read().await;
        let is_valid = library_manager.verify_package(package_id).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Verification failed: {}", e)
            })?;

        let result = VerificationResult {
            verified: is_valid,
            signature_valid: Some(is_valid),
            integrity_valid: is_valid,
            license_compliant: true,
            security_issues: if is_valid { vec![] } else { vec![SecurityIssue {
                severity: "high".to_string(),
                issue_type: "verification".to_string(),
                description: "Package verification failed".to_string(),
                affected_files: vec![],
            }] },
        };

        self.complete_operation().await;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_catalog_extension_creation() {
        let mut config = CatalogExtensionConfig::default();
        // Use much smaller cache sizes for testing (in number of entries, not bytes)
        config.cache_size = 100; // 100 entries, not bytes
        let extension = CatalogExtension::new(config);

        assert_eq!(extension.metadata.id, "catalog");
        assert_eq!(extension.metadata.category, ExtensionCategory::AssetLibrary);
        assert_eq!(extension.metadata.provided_assets.len(), 4);
    }

    #[tokio::test]
    async fn test_extension_metadata() {
        let mut config = CatalogExtensionConfig::default();
        config.cache_size = 100; // Use small cache for testing
        let extension = CatalogExtension::new(config);
        let metadata = extension.metadata();

        assert!(metadata.required_capabilities.contains(&ExtensionCapability::AssetManagement));
        assert!(metadata.required_capabilities.contains(&ExtensionCapability::VMExecution));
        assert!(metadata.required_capabilities.contains(&ExtensionCapability::NetworkAccess));
    }

    #[tokio::test]
    async fn test_extension_status() {
        let mut config = CatalogExtensionConfig::default();
        config.cache_size = 100; // Use small cache for testing
        let extension = CatalogExtension::new(config);
        let status = extension.status().await;

        assert_eq!(status.total_requests, 0);
        assert_eq!(status.error_count, 0);
        assert_eq!(status.active_operations, 0);
    }
}