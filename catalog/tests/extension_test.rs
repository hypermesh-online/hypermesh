//! Integration tests for Catalog Extension
//!
//! These tests verify that the CatalogExtension properly implements
//! the HyperMesh extension interfaces and can be loaded as a plugin.

use catalog::extension::{
    CatalogExtension, CatalogExtensionConfig,
    VirtualMachineHandler, LibraryHandler, DatasetHandler, TemplateHandler,
};

use blockmatrix::extensions::{
    HyperMeshExtension, AssetLibraryExtension, ExtensionConfig,
    ExtensionCapability, ExtensionRequest, ExtensionState,
    AssetCreationSpec, AssetQuery, PackageFilter, InstallOptions,
    SearchOptions, ResourceLimits,
};

use blockmatrix::assets::core::{AssetType, PrivacyLevel};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio;

/// Helper function to create test extension config
fn create_test_extension_config() -> ExtensionConfig {
    ExtensionConfig {
        settings: serde_json::json!({
            "library_path": "/tmp/test-catalog-library",
            "cache_size": 1024 * 1024 * 100, // 100MB for testing
            "enable_p2p": false,
            "consensus_validation": false,
            "debug_mode": true,
        }),
        resource_limits: ResourceLimits::default(),
        granted_capabilities: HashSet::from([
            ExtensionCapability::AssetManagement,
            ExtensionCapability::NetworkAccess,
            ExtensionCapability::VMExecution,
        ]),
        privacy_level: PrivacyLevel::Private,
        debug_mode: true,
    }
}

/// Helper function to create test catalog config
fn create_test_catalog_config() -> CatalogExtensionConfig {
    let mut config = CatalogExtensionConfig::default();
    config.library_path = PathBuf::from("/tmp/test-catalog-library");
    config.enable_p2p = false;
    config.consensus_validation = false;
    config.debug_mode = true;
    config
}

#[tokio::test]
async fn test_extension_creation() {
    let config = create_test_catalog_config();
    let extension = CatalogExtension::new(config);

    // Verify metadata
    let metadata = extension.metadata();
    assert_eq!(metadata.id, "catalog");
    assert_eq!(metadata.name, "HyperMesh Catalog");
    assert_eq!(metadata.category, hypermesh::extensions::ExtensionCategory::AssetLibrary);

    // Verify provided assets
    assert!(metadata.provided_assets.contains(&AssetType::VirtualMachine));
    assert!(metadata.provided_assets.contains(&AssetType::Library));
    assert!(metadata.provided_assets.contains(&AssetType::Dataset));
    assert!(metadata.provided_assets.contains(&AssetType::Template));

    // Verify required capabilities
    assert!(metadata.required_capabilities.contains(&ExtensionCapability::AssetManagement));
    assert!(metadata.required_capabilities.contains(&ExtensionCapability::NetworkAccess));
    assert!(metadata.required_capabilities.contains(&ExtensionCapability::VMExecution));
}

#[tokio::test]
async fn test_extension_initialization() {
    let config = create_test_catalog_config();
    let mut extension = CatalogExtension::new(config);

    // Create test directory
    std::fs::create_dir_all("/tmp/test-catalog-library").ok();

    let ext_config = create_test_extension_config();
    let result = extension.initialize(ext_config).await;

    // Initialization might fail due to missing HyperMesh connection
    // but the extension should handle it gracefully
    if result.is_err() {
        let err = result.unwrap_err();
        assert!(err.to_string().contains("HyperMesh"));
    }
}

#[tokio::test]
async fn test_register_assets() {
    let config = create_test_catalog_config();
    let extension = CatalogExtension::new(config);

    let handlers = extension.register_assets().await.unwrap();

    // Verify all asset handlers are registered
    assert!(handlers.contains_key(&AssetType::VirtualMachine));
    assert!(handlers.contains_key(&AssetType::Library));
    assert!(handlers.contains_key(&AssetType::Dataset));
    assert!(handlers.contains_key(&AssetType::Template));
}

#[tokio::test]
async fn test_extension_status() {
    let config = create_test_catalog_config();
    let extension = CatalogExtension::new(config);

    let status = extension.status().await;

    // Verify initial status
    assert_eq!(status.total_requests, 0);
    assert_eq!(status.error_count, 0);
    assert_eq!(status.active_operations, 0);
    assert!(status.uptime.as_secs() < 10);
}

#[tokio::test]
async fn test_handle_request_stats() {
    let config = create_test_catalog_config();
    let extension = CatalogExtension::new(config);

    let request = ExtensionRequest {
        id: "test-req-1".to_string(),
        method: "catalog.stats".to_string(),
        params: serde_json::Value::Null,
        consensus_proof: None,
    };

    let response = extension.handle_request(request).await.unwrap();

    assert!(response.success);
    assert!(response.data.is_some());
    assert!(response.error.is_none());

    // Verify stats structure
    let data = response.data.unwrap();
    assert!(data.get("total_requests").is_some());
    assert!(data.get("active_operations").is_some());
    assert!(data.get("error_count").is_some());
    assert!(data.get("uptime_seconds").is_some());
}

#[tokio::test]
async fn test_handle_request_unknown_method() {
    let config = create_test_catalog_config();
    let extension = CatalogExtension::new(config);

    let request = ExtensionRequest {
        id: "test-req-2".to_string(),
        method: "catalog.unknown".to_string(),
        params: serde_json::Value::Null,
        consensus_proof: None,
    };

    let response = extension.handle_request(request).await.unwrap();

    assert!(!response.success);
    assert!(response.data.is_none());
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("Unknown method"));
}

#[tokio::test]
async fn test_validation_report() {
    let config = create_test_catalog_config();
    let extension = CatalogExtension::new(config);

    let report = extension.validate().await.unwrap();

    // Extension not initialized, so should have errors
    assert!(!report.valid);
    assert!(!report.errors.is_empty());

    // Find the "not initialized" error
    let has_init_error = report.errors.iter()
        .any(|e| e.code == "CATALOG_NOT_INITIALIZED");
    assert!(has_init_error);
}

#[tokio::test]
async fn test_export_import_state() {
    let config = create_test_catalog_config();
    let mut extension = CatalogExtension::new(config);

    // Export state
    let state = extension.export_state().await.unwrap();
    assert_eq!(state.version, 1);
    assert_eq!(state.metadata.id, "catalog");

    // Import state (should succeed even if it's a no-op)
    let result = extension.import_state(state).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_shutdown() {
    let config = create_test_catalog_config();
    let mut extension = CatalogExtension::new(config);

    // Shutdown should succeed
    let result = extension.shutdown().await;
    assert!(result.is_ok());

    // Status should show stopped
    let status = extension.status().await;
    match status.state {
        ExtensionState::Stopped => {},
        _ => panic!("Expected extension to be stopped"),
    }
}

// Asset handler tests

#[tokio::test]
async fn test_vm_handler_operations() {
    let handler = VirtualMachineHandler::new();

    // Create a VM asset
    let spec = AssetCreationSpec {
        name: "Test Julia VM".to_string(),
        description: Some("Test virtual machine".to_string()),
        metadata: HashMap::from([
            ("language".to_string(), serde_json::json!("julia")),
            ("version".to_string(), serde_json::json!("1.9.0")),
        ]),
        privacy_level: PrivacyLevel::Private,
        allocation: None,
        consensus_requirements: hypermesh::extensions::ConsensusRequirements::default(),
        parent_id: None,
        tags: vec!["test".to_string()],
    };

    let asset_id = handler.create_asset(spec).await.unwrap();

    // Query the asset
    let query = AssetQuery {
        asset_type: Some(AssetType::VirtualMachine),
        name_pattern: Some("julia".to_string()),
        tags: None,
        privacy_level: None,
        parent_id: None,
        limit: Some(10),
        offset: None,
    };

    let results = handler.query_assets(query).await.unwrap();
    assert!(!results.is_empty());
    assert!(results.contains(&asset_id));

    // Get metadata
    let metadata = handler.get_metadata(&asset_id).await.unwrap();
    assert_eq!(metadata.asset_type, AssetType::VirtualMachine);
    assert!(metadata.name.contains("julia"));

    // Delete the asset
    let result = handler.delete_asset(&asset_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_library_handler_operations() {
    let handler = LibraryHandler::new();

    // Create a library asset
    let spec = AssetCreationSpec {
        name: "TestPackage.jl".to_string(),
        description: Some("Test Julia package".to_string()),
        metadata: HashMap::from([
            ("version".to_string(), serde_json::json!("1.0.0")),
            ("language".to_string(), serde_json::json!("julia")),
        ]),
        privacy_level: PrivacyLevel::Public,
        allocation: None,
        consensus_requirements: hypermesh::extensions::ConsensusRequirements::default(),
        parent_id: None,
        tags: vec!["julia".to_string(), "package".to_string()],
    };

    let asset_id = handler.create_asset(spec).await.unwrap();

    // Get metadata
    let metadata = handler.get_metadata(&asset_id).await.unwrap();
    assert_eq!(metadata.asset_type, AssetType::Library);
    assert_eq!(metadata.name, "TestPackage.jl");

    // Update the asset
    let update = hypermesh::extensions::AssetUpdate {
        name: Some("UpdatedPackage.jl".to_string()),
        description: None,
        metadata: None,
        privacy_level: None,
        allocation: None,
        tags: None,
    };

    let result = handler.update_asset(&asset_id, update).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_dataset_handler_operations() {
    let handler = DatasetHandler::new();

    // Create a dataset asset
    let spec = AssetCreationSpec {
        name: "TestDataset".to_string(),
        description: Some("Test dataset for ML".to_string()),
        metadata: HashMap::from([
            ("format".to_string(), serde_json::json!("parquet")),
            ("size_bytes".to_string(), serde_json::json!(1024 * 1024)),
            ("record_count".to_string(), serde_json::json!(10000)),
        ]),
        privacy_level: PrivacyLevel::Private,
        allocation: None,
        consensus_requirements: hypermesh::extensions::ConsensusRequirements::default(),
        parent_id: None,
        tags: vec!["ml".to_string(), "dataset".to_string()],
    };

    let asset_id = handler.create_asset(spec).await.unwrap();

    // Get metadata
    let metadata = handler.get_metadata(&asset_id).await.unwrap();
    assert_eq!(metadata.asset_type, AssetType::Dataset);
    assert!(metadata.description.unwrap().contains("10000 records"));
}

#[tokio::test]
async fn test_template_handler_operations() {
    let handler = TemplateHandler::new();

    // Create a template asset
    let spec = AssetCreationSpec {
        name: "MLProjectTemplate".to_string(),
        description: Some("Template for ML projects".to_string()),
        metadata: HashMap::from([
            ("template_type".to_string(), serde_json::json!("ml_project")),
            ("language".to_string(), serde_json::json!("julia")),
        ]),
        privacy_level: PrivacyLevel::Public,
        allocation: None,
        consensus_requirements: hypermesh::extensions::ConsensusRequirements::default(),
        parent_id: None,
        tags: vec!["template".to_string(), "ml".to_string()],
    };

    let asset_id = handler.create_asset(spec).await.unwrap();

    // Validate the asset (should exist)
    let valid = handler.validate_asset(&asset_id,
        hypermesh::consensus::proof_of_state_integration::ConsensusProof {
            space_proof: None,
            stake_proof: None,
            work_proof: None,
            time_proof: None,
        }
    ).await.unwrap();

    assert!(valid);
}

// Configuration tests

#[test]
fn test_config_validation() {
    let mut config = CatalogExtensionConfig::default();
    config.library_path = PathBuf::from("/nonexistent/path");

    let result = config.validate();
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn test_config_builder_pattern() {
    let config = CatalogExtensionConfig::new()
        .with_library_path(PathBuf::from("/tmp/catalog"))
        .with_cache_size(512 * 1024 * 1024)
        .with_p2p(false)
        .with_consensus_validation(true)
        .with_hypermesh_address("test.hypermesh.online".to_string())
        .with_trustchain_cert("cert.pem".to_string());

    assert_eq!(config.library_path, PathBuf::from("/tmp/catalog"));
    assert_eq!(config.cache_size, 512 * 1024 * 1024);
    assert!(!config.enable_p2p);
    assert!(config.consensus_validation);
    assert_eq!(config.hypermesh_address, "test.hypermesh.online");
    assert_eq!(config.trustchain_cert_path, Some("cert.pem".to_string()));
}