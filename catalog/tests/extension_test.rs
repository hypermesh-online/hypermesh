// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for Catalog Extension
//!
//! These tests verify that the CatalogExtension properly implements
//! the HyperMesh extension interfaces and can be loaded as a plugin.

use std::path::PathBuf;

use catalog::extension::{
    AssetLibraryExtension, CatalogExtension, CatalogExtensionConfig, ExtensionCapability,
    ExtensionCategory, HyperMeshExtension,
};

use blockmatrix::assets::core::AssetType;
use blockmatrix::extensions::{
    ExtensionHealth, ExtensionRequest, ExtensionState, PackageFilter, SearchOptions,
};

// ---------------------------------------------------------------------------
// Helper: test-friendly config with small cache to avoid OOM
// ---------------------------------------------------------------------------
fn test_config() -> CatalogExtensionConfig {
    CatalogExtensionConfig::new().with_cache_size(1024) // 1024 entries, not 1GB
}

// ===========================================================================
// CatalogExtension creation and metadata
// ===========================================================================

#[test]
fn test_extension_default_config() {
    let config = CatalogExtensionConfig::default();
    assert_eq!(config.cache_size, 1024 * 1024 * 1024); // 1GB
    assert!(config.enable_p2p);
    assert!(config.state_validation);
    assert_eq!(config.hypermesh_address, "catalog.hypermesh.online");
}

#[test]
fn test_extension_metadata_identity() {
    let extension = CatalogExtension::new(test_config());

    let metadata = extension.metadata();
    assert_eq!(metadata.id, "catalog");
    assert_eq!(metadata.name, "HyperMesh Catalog");
    assert_eq!(metadata.category, ExtensionCategory::AssetLibrary);
}

#[test]
fn test_extension_metadata_version() {
    let extension = CatalogExtension::new(test_config());

    let metadata = extension.metadata();
    assert_eq!(metadata.version.to_string(), "0.1.0");
}

#[test]
fn test_extension_provided_assets() {
    let extension = CatalogExtension::new(test_config());

    let metadata = extension.metadata();
    // provided_assets has: VirtualMachine, Library, Library(overwritten by Dataset), Container
    assert!(metadata
        .provided_assets
        .contains(&AssetType::Blockchain));
    assert!(metadata.provided_assets.contains(&AssetType::Dns));
    assert!(metadata.provided_assets.contains(&AssetType::Container));
}

#[test]
fn test_extension_required_capabilities() {
    let extension = CatalogExtension::new(test_config());

    let metadata = extension.metadata();
    let caps = &metadata.required_capabilities;
    assert_eq!(caps.len(), 7);
    assert!(caps.contains(&ExtensionCapability::AssetManagement));
    assert!(caps.contains(&ExtensionCapability::NetworkAccess));
    assert!(caps.contains(&ExtensionCapability::StateProofAccess));
    assert!(caps.contains(&ExtensionCapability::TransportAccess));
    assert!(caps.contains(&ExtensionCapability::TrustChainAccess));
    assert!(caps.contains(&ExtensionCapability::VMExecution));
    assert!(caps.contains(&ExtensionCapability::FileSystemAccess));
}

// ===========================================================================
// Extension lifecycle
// ===========================================================================

#[tokio::test]
async fn test_extension_register_assets_returns_handlers() {
    let extension = CatalogExtension::new(test_config());

    let handlers = extension.register_assets().await.unwrap();
    // HashMap deduplicates by key, so Library is overwritten by DatasetHandler
    // Expected keys: VirtualMachine, Library (DatasetHandler), Container (TemplateHandler)
    assert_eq!(
        handlers.len(),
        3,
        "Should have 3 distinct asset type handlers"
    );
    assert!(handlers.contains_key(&AssetType::Blockchain));
    assert!(handlers.contains_key(&AssetType::Dns));
    assert!(handlers.contains_key(&AssetType::Container));
}

#[tokio::test]
async fn test_extension_status_initial() {
    let extension = CatalogExtension::new(test_config());

    let status = extension.status().await;
    assert!(
        matches!(status.state, ExtensionState::Running),
        "Initial state should be Running"
    );
    assert_eq!(status.total_requests, 0);
    assert_eq!(status.error_count, 0);
    assert_eq!(status.active_operations, 0);
    assert!(status.uptime.as_secs() < 5);
}

#[tokio::test]
async fn test_extension_handle_request_stats() {
    let extension = CatalogExtension::new(test_config());

    let request = ExtensionRequest {
        id: "stats-req-1".to_string(),
        method: "catalog.stats".to_string(),
        params: serde_json::Value::Null,
        state_proof: None,
    };

    let response = extension.handle_request(request).await.unwrap();
    assert!(response.success);
    assert!(response.data.is_some());

    let data = response.data.unwrap();
    assert!(data.get("total_requests").is_some());
    assert!(data.get("active_operations").is_some());
    assert!(data.get("error_count").is_some());
    assert!(data.get("uptime_seconds").is_some());
}

#[tokio::test]
async fn test_extension_handle_request_unknown_method() {
    let extension = CatalogExtension::new(test_config());

    let request = ExtensionRequest {
        id: "unknown-1".to_string(),
        method: "catalog.nonexistent".to_string(),
        params: serde_json::Value::Null,
        state_proof: None,
    };

    let response = extension.handle_request(request).await.unwrap();
    assert!(!response.success);
    assert!(response.error.is_some());
    let error_msg = response.error.unwrap();
    assert!(
        error_msg.contains("Unknown method"),
        "Error should mention unknown method"
    );
}

#[tokio::test]
async fn test_extension_validate_without_catalog() {
    let extension = CatalogExtension::new(test_config());

    // Catalog is not initialized (no Catalog::new called via initialize)
    let report = extension.validate().await.unwrap();
    assert!(!report.valid);
    let has_init_error = report
        .errors
        .iter()
        .any(|e| e.code == "CATALOG_NOT_INITIALIZED");
    assert!(has_init_error, "Should have CATALOG_NOT_INITIALIZED error");
}

#[tokio::test]
async fn test_extension_export_state() {
    let extension = CatalogExtension::new(test_config());

    let state = extension.export_state().await.unwrap();
    assert_eq!(state.version, 1);
    assert_eq!(state.metadata.id, "catalog");
    assert!(!state.state_data.is_empty());
}

#[tokio::test]
async fn test_extension_import_state_succeeds() {
    let mut extension = CatalogExtension::new(test_config());

    let state = extension.export_state().await.unwrap();
    let result = extension.import_state(state).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_extension_shutdown() {
    let mut extension = CatalogExtension::new(test_config());

    let result = extension.shutdown().await;
    assert!(result.is_ok());

    // After shutdown, health should be degraded/unhealthy
    let status = extension.status().await;
    match status.health {
        ExtensionHealth::Unhealthy(_) => { /* expected */ }
        ExtensionHealth::Degraded(_) => { /* also acceptable during shutdown */ }
        other => panic!("Expected unhealthy/degraded after shutdown, got {other:?}"),
    }
}

#[tokio::test]
async fn test_extension_request_increments_counter() {
    let extension = CatalogExtension::new(test_config());

    let status_before = extension.status().await;
    assert_eq!(status_before.total_requests, 0);

    // Make a request
    let request = ExtensionRequest {
        id: "count-1".to_string(),
        method: "catalog.stats".to_string(),
        params: serde_json::Value::Null,
        state_proof: None,
    };
    extension.handle_request(request).await.unwrap();

    let status_after = extension.status().await;
    // handle_request calls increment_requests (1) + status also calls it (total may vary)
    assert!(
        status_after.total_requests > 0,
        "Total requests should be incremented"
    );
}

// ===========================================================================
// AssetLibraryExtension trait
// ===========================================================================

#[tokio::test]
async fn test_extension_list_packages_returns_empty() {
    let extension = CatalogExtension::new(test_config());

    let filter = PackageFilter {
        asset_type: None,
        author: None,
        license: None,
        min_rating: None,
        verified_only: false,
    };
    let packages = extension.list_packages(filter).await.unwrap();
    assert!(
        packages.is_empty(),
        "list_packages should return empty for new extension"
    );
}

#[tokio::test]
async fn test_extension_get_package_returns_stub() {
    let extension = CatalogExtension::new(test_config());

    let package = extension.get_package("test-pkg-id").await.unwrap();
    assert_eq!(package.id, "test-pkg-id");
    assert_eq!(package.name, "stub_package");
}

#[tokio::test]
async fn test_extension_search_packages_returns_empty() {
    let extension = CatalogExtension::new(test_config());

    let options = SearchOptions {
        limit: None,
        offset: None,
        sort_by: None,
        order: None,
    };
    let packages = extension
        .search_packages("anything", options)
        .await
        .unwrap();
    assert!(
        packages.is_empty(),
        "search_packages should return empty for new extension"
    );
}

// ===========================================================================
// Config builder pattern
// ===========================================================================

#[test]
fn test_config_builder_pattern() {
    let config = CatalogExtensionConfig::new()
        .with_library_path(PathBuf::from("/tmp/catalog"))
        .with_cache_size(512 * 1024 * 1024)
        .with_p2p(false)
        .with_state_validation(true)
        .with_hypermesh_address("test.hypermesh.online".to_string())
        .with_trustchain_cert("cert.pem".to_string());

    assert_eq!(config.library_path, PathBuf::from("/tmp/catalog"));
    assert_eq!(config.cache_size, 512 * 1024 * 1024);
    assert!(!config.enable_p2p);
    assert!(config.state_validation);
    assert_eq!(config.hypermesh_address, "test.hypermesh.online");
    assert_eq!(config.trustchain_cert_path, Some("cert.pem".to_string()));
}

// ===========================================================================
// Legacy tests (gated behind future-tests feature)
// ===========================================================================

#[cfg(feature = "future-tests")]
mod future_extension_tests {
    use catalog::extension::{
        CatalogExtension, CatalogExtensionConfig, DatasetHandler, LibraryHandler, TemplateHandler,
        VirtualMachineHandler,
    };

    use blockmatrix::extensions::{
        AssetCreationSpec, AssetLibraryExtension, AssetQuery, ExtensionCapability, ExtensionConfig,
        ExtensionRequest, ExtensionState, HyperMeshExtension, InstallOptions, PackageFilter,
        ResourceLimits, SearchOptions,
    };

    use blockmatrix::assets::core::AssetType;
    use hypermesh_lib::PrivacyMode;

    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;
    use tokio;

    fn create_test_extension_config() -> ExtensionConfig {
        ExtensionConfig {
            settings: serde_json::json!({
                "library_path": "/tmp/test-catalog-library",
                "cache_size": 1024 * 1024 * 100,
                "enable_p2p": false,
                "state_validation": false,
                "debug_mode": true,
            }),
            resource_limits: ResourceLimits::default(),
            granted_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::NetworkAccess,
                ExtensionCapability::VMExecution,
            ]),
            privacy_level: PrivacyMode::PRIVATE,
            debug_mode: true,
        }
    }

    fn create_test_catalog_config() -> CatalogExtensionConfig {
        let mut config = CatalogExtensionConfig::default();
        config.library_path = PathBuf::from("/tmp/test-catalog-library");
        config.enable_p2p = false;
        config.state_validation = false;
        config.debug_mode = true;
        config
    }

    #[tokio::test]
    async fn test_extension_creation() {
        let config = create_test_catalog_config();
        let extension = CatalogExtension::new(config);

        let metadata = extension.metadata();
        assert_eq!(metadata.id, "catalog");
        assert_eq!(metadata.name, "HyperMesh Catalog");
        assert_eq!(
            metadata.category,
            hypermesh::extensions::ExtensionCategory::AssetLibrary
        );

        assert!(metadata
            .provided_assets
            .contains(&AssetType::Blockchain));
        assert!(metadata.provided_assets.contains(&AssetType::Dns));
        assert!(metadata.provided_assets.contains(&AssetType::Dataset));
        assert!(metadata.provided_assets.contains(&AssetType::Template));

        assert!(metadata
            .required_capabilities
            .contains(&ExtensionCapability::AssetManagement));
        assert!(metadata
            .required_capabilities
            .contains(&ExtensionCapability::NetworkAccess));
        assert!(metadata
            .required_capabilities
            .contains(&ExtensionCapability::VMExecution));
    }

    #[tokio::test]
    async fn test_vm_handler_operations() {
        let handler = VirtualMachineHandler::new();

        let spec = AssetCreationSpec {
            name: "Test Lua VM".to_string(),
            description: Some("Test virtual machine".to_string()),
            metadata: HashMap::from([
                ("language".to_string(), serde_json::json!("lua")),
                ("version".to_string(), serde_json::json!("5.4.0")),
            ]),
            privacy_level: PrivacyMode::PRIVATE,
            allocation: None,
            state_requirements: hypermesh::extensions::StateRequirements::default(),
            parent_id: None,
            tags: vec!["test".to_string()],
        };

        let asset_id = handler.create_asset(spec).await.unwrap();

        let query = AssetQuery {
            asset_type: Some(AssetType::Blockchain),
            name_pattern: Some("lua".to_string()),
            tags: None,
            privacy_level: None,
            parent_id: None,
            limit: Some(10),
            offset: None,
        };

        let results = handler.query_assets(query).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.contains(&asset_id));
    }
}
