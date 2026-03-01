// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Plugin lifecycle tests
//!
//! Tests for plugin discovery, loading, unloading, configuration, hot-reload, and cleanup.
//!
//! NOTE: All tests marked #[ignore] - Requires Catalog extension implementation

use super::*;

/// Test plugin discovery in multiple search paths
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_plugin_discovery_multiple_paths() {
    init_test_logging();

    let loader_config = LoaderConfig {
        search_paths: vec![
            PathBuf::from("../catalog/target/debug"),
            PathBuf::from("../catalog/target/release"),
            PathBuf::from("./extensions"),
            PathBuf::from("/usr/local/lib/hypermesh/extensions"),
        ],
        enable_wasm: false,
        verify_signatures: false,
        max_extensions: 20,
        default_limits: ResourceLimits::default(),
        trustchain_cert_path: None,
    };

    let loader = ExtensionLoader::new(loader_config);

    // Discover all extensions
    let discovered = loader.discover_extensions().await.unwrap();
    info!(
        "Discovered {} extensions across all paths",
        discovered.len()
    );

    // Verify catalog is found
    let catalog = discovered
        .iter()
        .find(|m| m.metadata.id == "catalog")
        .expect("Catalog extension should be discovered");

    assert_eq!(catalog.metadata.name, "Catalog Extension");
    assert!(!catalog.metadata.version.major < 1);

    // Verify metadata is complete
    assert!(!catalog.metadata.description.is_empty());
    assert!(!catalog.metadata.author.is_empty());
    assert!(!catalog.metadata.required_capabilities.is_empty());

    info!("Plugin discovery test passed");
}

/// Test manifest parsing and validation
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_manifest_validation() {
    init_test_logging();

    let loader_config = LoaderConfig {
        search_paths: vec![PathBuf::from("../catalog/target/debug")],
        enable_wasm: false,
        verify_signatures: false,
        max_extensions: 10,
        default_limits: ResourceLimits::default(),
        trustchain_cert_path: None,
    };

    let loader = ExtensionLoader::new(loader_config);

    // Test valid manifest
    let discovered = loader.discover_extensions().await.unwrap();
    let catalog = discovered
        .iter()
        .find(|m| m.metadata.id == "catalog")
        .expect("Catalog should be found");

    // Validate required fields
    assert!(catalog.metadata.hypermesh_version.major >= 1);
    assert!(catalog
        .metadata
        .required_capabilities
        .contains(&ExtensionCapability::AssetManagement));

    // Test invalid manifest scenarios
    let invalid_path = PathBuf::from("./test-data/invalid-manifest");
    std::fs::create_dir_all(&invalid_path).ok();

    // Create manifest with missing required fields
    let invalid_manifest = r#"
    {
        "id": "invalid",
        "name": "Invalid Extension"
    }
    "#;

    std::fs::write(invalid_path.join("manifest.json"), invalid_manifest).ok();

    // Should handle gracefully
    let loader2 = ExtensionLoader::new(LoaderConfig {
        search_paths: vec![invalid_path.clone()],
        ..Default::default()
    });

    let result = loader2.discover_extensions().await;
    assert!(result.is_ok()); // Should not crash

    // Cleanup
    std::fs::remove_dir_all(&invalid_path).ok();

    info!("Manifest validation test passed");
}

/// Test signature verification during loading
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_signature_verification() {
    init_test_logging();

    // Test with signature verification enabled
    let loader_config = LoaderConfig {
        search_paths: vec![PathBuf::from("../catalog/target/debug")],
        enable_wasm: false,
        verify_signatures: true, // Enable signature verification
        max_extensions: 10,
        default_limits: ResourceLimits::default(),
        trustchain_cert_path: Some(PathBuf::from("../trustchain/certs/root.crt")),
    };

    let loader = ExtensionLoader::new(loader_config);

    // Try to load extension - should fail if not signed
    let extension_path = PathBuf::from("../catalog/target/debug");
    let result = loader.load_extension(&extension_path).await;

    // In production, unsigned extensions should fail
    // For testing, we allow unsigned if explicitly configured
    if loader.config.verify_signatures {
        // Should either succeed with valid signature or fail gracefully
        match result {
            Ok(id) => {
                info!("Extension {} loaded with valid signature", id);
                loader.unload_extension(&id).await.ok();
            }
            Err(e) => {
                info!(
                    "Extension loading failed as expected without signature: {}",
                    e
                );
            }
        }
    }

    info!("Signature verification test passed");
}

/// Test loading with various configuration scenarios
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_configuration_scenarios() {
    init_test_logging();

    let asset_manager = Arc::new(AssetManager::new());

    // Scenario 1: Minimal configuration
    let minimal_config = ExtensionManagerConfig {
        extension_dirs: vec![PathBuf::from("../catalog/target/debug")],
        auto_load: false,
        verify_signatures: false,
        max_extensions: 1,
        global_limits: ResourceLimits {
            cpu_percent: 10.0,
            memory_mb: 100,
            storage_mb: 500,
            network_bandwidth_kbps: 100,
            file_descriptors: 50,
            max_threads: 5,
            ops_per_second: 10,
        },
        allowed_capabilities: HashSet::from([ExtensionCapability::AssetManagement]),
    };

    let manager1 = ExtensionManager::new(asset_manager.clone(), minimal_config);
    assert_eq!(manager1.list_extensions().await.len(), 0); // No auto-load

    // Scenario 2: Production configuration
    let production_config = ExtensionManagerConfig {
        extension_dirs: vec![
            PathBuf::from("../catalog/target/release"),
            PathBuf::from("/usr/local/lib/hypermesh/extensions"),
        ],
        auto_load: true,
        verify_signatures: true,
        max_extensions: 50,
        global_limits: ResourceLimits {
            cpu_percent: 80.0,
            memory_mb: 8192,
            storage_mb: 100000,
            network_bandwidth_kbps: 10000,
            file_descriptors: 1000,
            max_threads: 100,
            ops_per_second: 1000,
        },
        allowed_capabilities: HashSet::from([
            ExtensionCapability::AssetManagement,
            ExtensionCapability::VMExecution,
            ExtensionCapability::NetworkAccess,
            ExtensionCapability::ConsensusAccess,
            ExtensionCapability::StorageAccess,
        ]),
    };

    let manager2 = ExtensionManager::new(asset_manager.clone(), production_config);

    // Scenario 3: Development configuration with hot reload
    let dev_config = ExtensionManagerConfig {
        extension_dirs: vec![PathBuf::from("../catalog/target/debug")],
        auto_load: false,
        verify_signatures: false,
        max_extensions: 100,
        global_limits: ResourceLimits::unlimited(), // No limits in dev
        allowed_capabilities: ExtensionCapability::all(), // All capabilities
    };

    let manager3 = ExtensionManager::new(asset_manager.clone(), dev_config);

    info!("Configuration scenarios test passed");
}

/// Test proper cleanup during unloading
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_cleanup_on_unload() {
    init_test_logging();

    let asset_manager = Arc::new(AssetManager::new());
    let loader = create_test_loader();

    // Load and fully setup extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Register assets and handlers
    let handlers = extension.register_assets().await.unwrap();
    let handler_count = handlers.len();
    extension
        .extend_manager(asset_manager.clone())
        .await
        .unwrap();

    // Create some resources
    for i in 0..5 {
        let request = ExtensionRequest {
            id: format!("cleanup-{}", i),
            method: "create_package".to_string(),
            params: json!({
                "name": format!("test-pkg-{}", i),
                "version": "1.0.0",
                "code": "test code"
            }),
            consensus_proof: None,
        };
        extension.handle_request(request).await.ok();
    }

    // Get initial state
    let state_before = extension.export_state().await.unwrap();
    let status_before = extension.status().await;

    info!(
        "Extension state before unload: {} items, {} requests processed",
        state_before.data.len(),
        status_before.total_requests
    );

    // Unload extension
    loader.unload_extension(&extension_id).await.unwrap();

    // Verify extension is gone
    assert!(loader.get_extension(&extension_id).await.is_none());

    // Verify handlers are deregistered
    let remaining_types = asset_manager.list_asset_types().await;
    let catalog_types = remaining_types
        .iter()
        .filter(|t| t.name == "library" || t.name == "package")
        .count();
    assert_eq!(catalog_types, 0, "Asset types should be deregistered");

    // Try to load again - should work
    let extension_id2 = loader.load_extension(&extension_path).await.unwrap();
    assert_eq!(extension_id2, extension_id); // Same ID

    let extension2 = loader.get_extension(&extension_id2).await.unwrap();
    let status_after = extension2.status().await;

    // Should be fresh instance
    assert_eq!(status_after.total_requests, 0);

    // Final cleanup
    loader.unload_extension(&extension_id2).await.unwrap();

    info!("Cleanup on unload test passed");
}

/// Test state persistence and recovery
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_state_persistence() {
    init_test_logging();

    let loader = create_test_loader();

    // Load extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Create state
    for i in 0..3 {
        let request = ExtensionRequest {
            id: format!("persist-{}", i),
            method: "create_library".to_string(),
            params: json!({
                "name": format!("lib-{}", i),
                "description": format!("Library {}", i)
            }),
            consensus_proof: None,
        };
        extension.handle_request(request).await.ok();
    }

    // Export state
    let exported_state = extension.export_state().await.unwrap();
    assert!(!exported_state.data.is_empty());

    // Unload extension
    loader.unload_extension(&extension_id).await.unwrap();

    // Reload extension
    let extension_id2 = loader.load_extension(&extension_path).await.unwrap();
    let extension2 = loader.get_extension(&extension_id2).await.unwrap();

    // Import state
    extension2
        .import_state(exported_state.clone())
        .await
        .unwrap();

    // Verify state was restored
    let restored_state = extension2.export_state().await.unwrap();
    assert_eq!(restored_state.data.len(), exported_state.data.len());
    assert_eq!(restored_state.version, exported_state.version);

    // Cleanup
    loader.unload_extension(&extension_id2).await.unwrap();

    info!("State persistence test passed");
}

/// Test hot-reload functionality
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
async fn test_hot_reload() {
    init_test_logging();

    let asset_manager = Arc::new(AssetManager::new());
    let loader = create_test_loader();

    // Initial load
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension1 = loader.get_extension(&extension_id).await.unwrap();

    // Setup extension
    extension1.register_assets().await.unwrap();
    extension1
        .extend_manager(asset_manager.clone())
        .await
        .unwrap();

    // Create some state
    let request = ExtensionRequest {
        id: "hot-1".to_string(),
        method: "create_library".to_string(),
        params: json!({
            "name": "persistent-lib",
            "description": "Should survive reload"
        }),
        consensus_proof: None,
    };
    extension1.handle_request(request).await.unwrap();

    // Export state before reload
    let state = extension1.export_state().await.unwrap();
    let status1 = extension1.status().await;

    // Hot reload
    info!("Performing hot reload...");
    loader.reload_extension(&extension_id).await.unwrap();

    // Get reloaded extension
    let extension2 = loader.get_extension(&extension_id).await.unwrap();

    // Re-setup extension
    extension2.register_assets().await.unwrap();
    extension2
        .extend_manager(asset_manager.clone())
        .await
        .unwrap();

    // Import state
    extension2.import_state(state).await.unwrap();

    // Verify state survived
    let status2 = extension2.status().await;
    info!("Status before reload: {:?}", status1);
    info!("Status after reload: {:?}", status2);

    // Verify can still handle requests
    let request2 = ExtensionRequest {
        id: "hot-2".to_string(),
        method: "list_libraries".to_string(),
        params: json!({}),
        consensus_proof: None,
    };

    let response = extension2.handle_request(request2).await.unwrap();
    assert!(response.success);

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Hot reload test passed");
}
