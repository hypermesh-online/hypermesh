// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Extension integration tests
//!
//! Tests for asset registration, handlers, extension traits, API endpoints, and capability security.
//!
//! NOTE: All tests marked #[ignore] - Requires Catalog extension implementation

use super::*;

/// Test capability-based security during initialization
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_capability_based_security() {
    init_test_logging();

    let asset_manager = Arc::new(AssetManager::new());

    // Create security manager with strict enforcement
    let security_config = SecurityConfig {
        enforcement_enabled: true,
        anomaly_detection: true,
        audit_enabled: true,
        default_isolation: IsolationLevel::Container,
        max_violations: 3,
        violation_reset_interval: Duration::from_secs(300),
    };

    let security_manager = SecurityManager::new(security_config);

    // Create extension metadata
    let metadata = ExtensionMetadata {
        id: "catalog".to_string(),
        name: "Catalog Extension".to_string(),
        version: semver::Version::parse("1.0.0").unwrap(),
        description: "Asset library management".to_string(),
        author: "HyperMesh".to_string(),
        license: "MIT".to_string(),
        homepage: None,
        category: hypermesh::extensions::ExtensionCategory::AssetLibrary,
        hypermesh_version: semver::Version::parse("1.0.0").unwrap(),
        dependencies: vec![],
        required_capabilities: HashSet::from([
            ExtensionCapability::AssetManagement,
            ExtensionCapability::VMExecution,
            ExtensionCapability::NetworkAccess,
        ]),
        provided_assets: vec!["library".to_string(), "package".to_string()],
        certificate_fingerprint: None,
        config_schema: None,
    };

    // Test granting only partial capabilities
    let limited_capabilities = HashSet::from([
        ExtensionCapability::AssetManagement, // Grant only this
    ]);

    let quotas = ResourceQuotas::default();

    // Create context with limited capabilities
    security_manager.create_context(
        "catalog".to_string(),
        &metadata,
        limited_capabilities.clone(),
        quotas,
    ).await.unwrap();

    // Test allowed capability
    assert!(security_manager.check_capability(
        "catalog",
        &ExtensionCapability::AssetManagement,
        "register_asset"
    ).await.is_ok());

    // Test denied capability
    assert!(security_manager.check_capability(
        "catalog",
        &ExtensionCapability::VMExecution,
        "execute_code"
    ).await.is_err());

    // Test with all required capabilities
    security_manager.create_context(
        "catalog-full".to_string(),
        &metadata,
        metadata.required_capabilities.clone(),
        quotas,
    ).await.unwrap();

    // All should be allowed
    for capability in &metadata.required_capabilities {
        assert!(security_manager.check_capability(
            "catalog-full",
            capability,
            "test_operation"
        ).await.is_ok());
    }

    info!("Capability-based security test passed");
}

/// Test CatalogExtension trait implementation
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_extension_trait_implementation() {
    init_test_logging();

    let loader = create_test_loader();

    // Load catalog extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();

    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Test all trait methods

    // 1. Test status
    let status = extension.status().await;
    assert_eq!(status.total_requests, 0);
    assert!(status.uptime.as_secs() < 60);

    // 2. Test metadata
    let metadata = extension.metadata().await;
    assert_eq!(metadata.id, "catalog");
    assert!(!metadata.provided_assets.is_empty());

    // 3. Test validation
    let validation = extension.validate().await.unwrap();
    assert!(validation.valid || !validation.warnings.is_empty());

    // 4. Test export/import state
    let state = extension.export_state().await.unwrap();
    assert_eq!(state.version, 1);

    // Import should work
    extension.import_state(state).await.unwrap();

    // 5. Test lifecycle methods
    extension.pause().await.unwrap();
    extension.resume().await.unwrap();

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Extension trait implementation test passed");
}

/// Test asset type registration with AssetManager
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_asset_registration() {
    init_test_logging();

    let asset_manager = Arc::new(AssetManager::new());
    let loader = create_test_loader();

    // Load extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Register assets
    let handlers = extension.register_assets().await.unwrap();
    assert!(!handlers.is_empty());

    // Extend asset manager
    extension.extend_manager(asset_manager.clone()).await.unwrap();

    // Verify asset types are registered
    let asset_types = asset_manager.list_asset_types().await;
    assert!(asset_types.iter().any(|t| t.name == "library"));
    assert!(asset_types.iter().any(|t| t.name == "package"));

    // Test creating assets of new types
    let library_asset = asset_manager.create_asset(
        AssetType::Custom("library".to_string()),
        json!({
            "name": "test-library",
            "version": "1.0.0",
            "dependencies": []
        }),
        PrivacyLevel::PRIVATE,
    ).await;

    assert!(library_asset.is_ok());

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Asset registration test passed");
}

/// Test asset handlers integration
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_asset_handlers() {
    init_test_logging();

    let asset_manager = Arc::new(AssetManager::new());
    let loader = create_test_loader();

    // Load and setup extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    extension.register_assets().await.unwrap();
    extension.extend_manager(asset_manager.clone()).await.unwrap();

    // Test library asset handler
    let library_request = ExtensionRequest {
        id: "test-lib-1".to_string(),
        method: "create_library".to_string(),
        params: json!({
            "name": "test-library",
            "description": "Test library for validation",
            "packages": []
        }),
        consensus_proof: None,
    };

    let response = extension.handle_request(library_request).await.unwrap();
    assert!(response.success);

    // Test package asset handler
    let package_request = ExtensionRequest {
        id: "test-pkg-1".to_string(),
        method: "create_package".to_string(),
        params: json!({
            "name": "test-package",
            "version": "1.0.0",
            "library": "test-library",
            "code": "function test() { return 42; }"
        }),
        consensus_proof: None,
    };

    let response = extension.handle_request(package_request).await.unwrap();
    assert!(response.success);

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Asset handlers test passed");
}

/// Test extension API endpoints accessibility
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_api_endpoints() {
    init_test_logging();

    let loader = create_test_loader();

    // Load extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Test all documented API endpoints
    let endpoints = vec![
        ("list_packages", json!({"limit": 10})),
        ("get_package", json!({"id": "test-pkg"})),
        ("search_packages", json!({"query": "test"})),
        ("list_libraries", json!({})),
        ("get_library_info", json!({"name": "test-lib"})),
        ("execute_vm", json!({"code": "1+1", "inputs": {}})),
        ("validate_package", json!({"package_id": "test"})),
    ];

    for (method, params) in endpoints {
        let request = ExtensionRequest {
            id: format!("test-{}", method),
            method: method.to_string(),
            params,
            consensus_proof: None,
        };

        let response = extension.handle_request(request).await;
        // Some might fail due to missing data, but should not crash
        assert!(response.is_ok());

        if let Ok(resp) = response {
            debug!("API endpoint {} responded: {}", method, resp.success);
        }
    }

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("API endpoints test passed");
}
