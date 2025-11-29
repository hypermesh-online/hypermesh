//! Integration tests for HyperMesh extension system
//!
//! These tests verify that extensions can be loaded, managed, and integrated
//! properly with the HyperMesh core system.

use blockmatrix::{
    HyperMeshSystem, HyperMeshConfig,
    extensions::{
        HyperMeshExtension, ExtensionMetadata, ExtensionCapability,
        ExtensionCategory, ExtensionConfig, ExtensionRequest, ExtensionResponse,
        ExtensionStatus, ExtensionResult, ValidationReport,
        AssetExtensionHandler, AssetType, AssetCreationSpec, AssetUpdate,
        AssetQuery, AssetMetadata, AssetOperation, OperationResult,
        manager::UnifiedExtensionManager,
    },
    assets::core::{AssetManager, AssetId, PrivacyLevel, ConsensusProof},
};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use semver::Version;

/// Mock Catalog extension for testing
struct MockCatalogExtension {
    initialized: bool,
    assets: Arc<RwLock<HashMap<AssetId, AssetMetadata>>>,
}

impl MockCatalogExtension {
    fn new() -> Self {
        Self {
            initialized: false,
            assets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl HyperMeshExtension for MockCatalogExtension {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            id: "mock-catalog".to_string(),
            name: "Mock Catalog Extension".to_string(),
            version: Version::parse("1.0.0").unwrap(),
            description: "Test catalog extension for integration testing".to_string(),
            author: "HyperMesh Test Team".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            category: ExtensionCategory::AssetLibrary,
            hypermesh_version: Version::parse("1.0.0").unwrap(),
            dependencies: vec![],
            required_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::NetworkAccess,
            ]),
            provided_assets: vec![
                AssetType::VirtualMachine,
                AssetType::Container,
                AssetType::Library,
            ],
            certificate_fingerprint: None,
            config_schema: None,
        }
    }

    async fn initialize(&mut self, _config: ExtensionConfig) -> ExtensionResult<()> {
        self.initialized = true;
        Ok(())
    }

    async fn register_assets(&self) -> ExtensionResult<HashMap<AssetType, Box<dyn AssetExtensionHandler>>> {
        let mut handlers = HashMap::new();

        // Register handler for library assets
        handlers.insert(
            AssetType::Library,
            Box::new(MockAssetHandler::new(self.assets.clone())) as Box<dyn AssetExtensionHandler>
        );

        Ok(handlers)
    }

    async fn extend_manager(&self, _asset_manager: Arc<AssetManager>) -> ExtensionResult<()> {
        // Mock extending the asset manager
        Ok(())
    }

    async fn handle_request(&self, request: ExtensionRequest) -> ExtensionResult<ExtensionResponse> {
        Ok(ExtensionResponse {
            request_id: request.id,
            success: true,
            data: Some(serde_json::json!({
                "method": request.method,
                "result": "success"
            })),
            error: None,
        })
    }

    async fn status(&self) -> ExtensionStatus {
        ExtensionStatus {
            state: hypermesh::extensions::ExtensionState::Running,
            health: hypermesh::extensions::ExtensionHealth::Healthy,
            resource_usage: hypermesh::extensions::ResourceUsageReport {
                cpu_usage: 10.0,
                memory_usage: 1024 * 1024,
                network_bytes: 1024,
                storage_bytes: 1024 * 1024,
            },
            active_operations: 0,
            total_requests: 100,
            error_count: 0,
            uptime: std::time::Duration::from_secs(3600),
        }
    }

    async fn validate(&self) -> ExtensionResult<ValidationReport> {
        Ok(ValidationReport {
            valid: true,
            certificate_valid: None,
            dependencies_satisfied: true,
            resource_compliance: true,
            security_compliance: true,
            errors: vec![],
            warnings: vec![],
        })
    }

    async fn export_state(&self) -> ExtensionResult<hypermesh::extensions::ExtensionState> {
        Ok(hypermesh::extensions::ExtensionState {
            version: 1,
            metadata: self.metadata(),
            state_data: vec![],
            checksum: "mock-checksum".to_string(),
            exported_at: std::time::SystemTime::now(),
        })
    }

    async fn import_state(&mut self, _state: hypermesh::extensions::ExtensionState) -> ExtensionResult<()> {
        Ok(())
    }

    async fn shutdown(&mut self) -> ExtensionResult<()> {
        self.initialized = false;
        Ok(())
    }
}

/// Mock asset handler for testing
struct MockAssetHandler {
    assets: Arc<RwLock<HashMap<AssetId, AssetMetadata>>>,
}

impl MockAssetHandler {
    fn new(assets: Arc<RwLock<HashMap<AssetId, AssetMetadata>>>) -> Self {
        Self { assets }
    }
}

#[async_trait]
impl AssetExtensionHandler for MockAssetHandler {
    fn asset_type(&self) -> AssetType {
        AssetType::Library
    }

    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetId> {
        let id = AssetId::new();
        let metadata = AssetMetadata {
            id: id.clone(),
            asset_type: AssetType::Library,
            name: spec.name,
            description: spec.description,
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            size_bytes: 0,
            metadata: spec.metadata,
            privacy_level: spec.privacy_level,
            allocation: spec.allocation,
            consensus_status: hypermesh::extensions::ConsensusStatus {
                validated: false,
                last_validated: None,
                proofs: None,
                errors: vec![],
            },
            tags: spec.tags,
        };

        self.assets.write().await.insert(id.clone(), metadata);
        Ok(id)
    }

    async fn update_asset(&self, id: &AssetId, update: AssetUpdate) -> ExtensionResult<()> {
        let mut assets = self.assets.write().await;
        if let Some(asset) = assets.get_mut(id) {
            if let Some(name) = update.name {
                asset.name = name;
            }
            if let Some(description) = update.description {
                asset.description = description;
            }
            asset.updated_at = std::time::SystemTime::now();
            Ok(())
        } else {
            Err(hypermesh::extensions::ExtensionError::RuntimeError {
                message: "Asset not found".to_string(),
            })
        }
    }

    async fn delete_asset(&self, id: &AssetId) -> ExtensionResult<()> {
        self.assets.write().await.remove(id);
        Ok(())
    }

    async fn query_assets(&self, query: AssetQuery) -> ExtensionResult<Vec<AssetId>> {
        let assets = self.assets.read().await;
        let mut results = vec![];

        for (id, metadata) in assets.iter() {
            let mut matches = true;

            if let Some(ref asset_type) = query.asset_type {
                if metadata.asset_type != *asset_type {
                    matches = false;
                }
            }

            if let Some(ref pattern) = query.name_pattern {
                if !metadata.name.contains(pattern) {
                    matches = false;
                }
            }

            if matches {
                results.push(id.clone());
            }
        }

        Ok(results)
    }

    async fn get_metadata(&self, id: &AssetId) -> ExtensionResult<AssetMetadata> {
        self.assets
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| hypermesh::extensions::ExtensionError::RuntimeError {
                message: "Asset not found".to_string(),
            })
    }

    async fn validate_asset(&self, _id: &AssetId, _proof: ConsensusProof) -> ExtensionResult<bool> {
        Ok(true)
    }

    async fn handle_operation(&self, _id: &AssetId, _operation: AssetOperation) -> ExtensionResult<OperationResult> {
        Ok(OperationResult::Custom(serde_json::json!({
            "status": "success"
        })))
    }
}

#[tokio::test]
async fn test_extension_loading() {
    // Create HyperMesh system
    let config = HyperMeshConfig::default();
    let system = HyperMeshSystem::new(config).await.unwrap();

    // Create mock catalog extension
    let extension = Box::new(MockCatalogExtension::new());

    // Load extension
    let manager = system.extension_manager();
    manager.load_extension(extension).await.unwrap();

    // Verify extension is loaded
    let extensions = manager.list_extensions().await;
    assert_eq!(extensions.len(), 1);
    assert_eq!(extensions[0].metadata.id, "mock-catalog");
}

#[tokio::test]
async fn test_extension_asset_handler() {
    // Create HyperMesh system
    let config = HyperMeshConfig::default();
    let system = HyperMeshSystem::new(config).await.unwrap();

    // Load mock catalog extension
    let extension = Box::new(MockCatalogExtension::new());
    let manager = system.extension_manager();
    manager.load_extension(extension).await.unwrap();

    // Get asset handler
    let handler = manager.get_asset_handler(&AssetType::Library).await;
    assert!(handler.is_some());

    // Create an asset through the handler
    let handler = handler.unwrap();
    let spec = AssetCreationSpec {
        name: "test-library".to_string(),
        description: Some("Test library asset".to_string()),
        metadata: HashMap::new(),
        privacy_level: PrivacyLevel::Private,
        allocation: None,
        consensus_requirements: hypermesh::extensions::ConsensusRequirements::default(),
        parent_id: None,
        tags: vec!["test".to_string()],
    };

    let asset_id = handler.create_asset(spec).await.unwrap();

    // Query the created asset
    let query = AssetQuery {
        asset_type: Some(AssetType::Library),
        name_pattern: Some("test".to_string()),
        tags: None,
        privacy_level: None,
        parent_id: None,
        limit: Some(10),
        offset: None,
    };

    let results = handler.query_assets(query).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], asset_id);
}

#[tokio::test]
async fn test_extension_request_handling() {
    // Create HyperMesh system
    let config = HyperMeshConfig::default();
    let system = HyperMeshSystem::new(config).await.unwrap();

    // Load mock catalog extension
    let extension = Box::new(MockCatalogExtension::new());
    let manager = system.extension_manager();
    manager.load_extension(extension).await.unwrap();

    // Send request to extension
    let request = ExtensionRequest {
        id: uuid::Uuid::new_v4().to_string(),
        method: "test_method".to_string(),
        params: serde_json::json!({
            "param1": "value1"
        }),
        consensus_proof: None,
    };

    let response = manager.handle_request("mock-catalog", request.clone()).await.unwrap();
    assert!(response.success);
    assert_eq!(response.request_id, request.id);
}

#[tokio::test]
async fn test_extension_unloading() {
    // Create HyperMesh system
    let config = HyperMeshConfig::default();
    let system = HyperMeshSystem::new(config).await.unwrap();

    // Load mock catalog extension
    let extension = Box::new(MockCatalogExtension::new());
    let manager = system.extension_manager();
    manager.load_extension(extension).await.unwrap();

    // Verify extension is loaded
    assert_eq!(manager.list_extensions().await.len(), 1);

    // Unload extension
    manager.unload_extension("mock-catalog").await.unwrap();

    // Verify extension is unloaded
    assert_eq!(manager.list_extensions().await.len(), 0);
}

#[tokio::test]
async fn test_extension_validation() {
    // Create HyperMesh system
    let config = HyperMeshConfig::default();
    let system = HyperMeshSystem::new(config).await.unwrap();

    // Load mock catalog extension
    let extension = Box::new(MockCatalogExtension::new());
    let manager = system.extension_manager();
    manager.load_extension(extension).await.unwrap();

    // Validate extension
    let reports = manager.validate_all_extensions().await;
    assert_eq!(reports.len(), 1);

    let report = reports.get("mock-catalog").unwrap();
    assert!(report.valid);
    assert!(report.dependencies_satisfied);
    assert!(report.resource_compliance);
    assert!(report.security_compliance);
}

#[tokio::test]
async fn test_extension_metrics() {
    // Create HyperMesh system
    let config = HyperMeshConfig::default();
    let system = HyperMeshSystem::new(config).await.unwrap();

    // Load mock catalog extension
    let extension = Box::new(MockCatalogExtension::new());
    let manager = system.extension_manager();
    manager.load_extension(extension).await.unwrap();

    // Get metrics
    let metrics = manager.get_metrics().await;
    assert_eq!(metrics.total_loaded, 1);
    assert_eq!(metrics.total_failed, 0);
}

#[tokio::test]
async fn test_catalog_extension_integration() {
    // This test simulates the full integration of a Catalog extension
    // providing asset library functionality to HyperMesh

    // Create HyperMesh system
    let config = HyperMeshConfig::default();
    let system = HyperMeshSystem::new(config).await.unwrap();

    // Create and load catalog extension
    let catalog = Box::new(MockCatalogExtension::new());
    let manager = system.extension_manager();
    manager.load_extension(catalog).await.unwrap();

    // Simulate package installation through catalog
    let request = ExtensionRequest {
        id: uuid::Uuid::new_v4().to_string(),
        method: "install_package".to_string(),
        params: serde_json::json!({
            "package_id": "julia-scientific",
            "version": "1.0.0"
        }),
        consensus_proof: None,
    };

    let response = manager.handle_request("mock-catalog", request).await.unwrap();
    assert!(response.success);

    // Verify the extension is providing asset management
    let handler = manager.get_asset_handler(&AssetType::Library).await;
    assert!(handler.is_some());

    // Create a library asset
    let spec = AssetCreationSpec {
        name: "julia-scientific-package".to_string(),
        description: Some("Julia scientific computing package".to_string()),
        metadata: HashMap::from([
            ("language".to_string(), serde_json::json!("julia")),
            ("version".to_string(), serde_json::json!("1.0.0")),
        ]),
        privacy_level: PrivacyLevel::PublicNetwork,
        allocation: None,
        consensus_requirements: hypermesh::extensions::ConsensusRequirements::default(),
        parent_id: None,
        tags: vec!["julia".to_string(), "scientific".to_string()],
    };

    let handler = handler.unwrap();
    let asset_id = handler.create_asset(spec).await.unwrap();

    // Get asset metadata
    let metadata = handler.get_metadata(&asset_id).await.unwrap();
    assert_eq!(metadata.name, "julia-scientific-package");
    assert_eq!(metadata.asset_type, AssetType::Library);

    // Validate with consensus
    let consensus_proof = ConsensusProof::default();
    let valid = handler.validate_asset(&asset_id, consensus_proof).await.unwrap();
    assert!(valid);
}

#[tokio::test]
async fn test_extension_pause_resume() {
    // Create HyperMesh system
    let config = HyperMeshConfig::default();
    let system = HyperMeshSystem::new(config).await.unwrap();

    // Load mock catalog extension
    let extension = Box::new(MockCatalogExtension::new());
    let manager = system.extension_manager();
    manager.load_extension(extension).await.unwrap();

    // Pause extension
    manager.pause_extension("mock-catalog").await.unwrap();

    // Check state
    let info = manager.get_extension_info("mock-catalog").await.unwrap();
    assert!(matches!(
        info.state.state,
        hypermesh::extensions::manager::ExtensionState::Paused
    ));

    // Resume extension
    manager.resume_extension("mock-catalog").await.unwrap();

    // Check state
    let info = manager.get_extension_info("mock-catalog").await.unwrap();
    assert!(matches!(
        info.state.state,
        hypermesh::extensions::manager::ExtensionState::Active
    ));
}