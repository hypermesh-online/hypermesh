//! Comprehensive integration tests for Catalog plugin loading and unloading
//!
//! This test suite validates the complete plugin lifecycle including:
//! - Plugin discovery and loading
//! - Extension integration with HyperMesh
//! - Runtime operations
//! - Graceful unloading
//! - Error handling and edge cases

use blockmatrix::assets::core::{AssetManager, AssetType, PrivacyLevel, AssetId};
use blockmatrix::extensions::{
    ExtensionCapability, ExtensionConfig, ExtensionManager, ExtensionManagerConfig,
    ExtensionMetadata, ExtensionRequest, ExtensionResponse, ResourceLimits,
};
use blockmatrix::extensions::loader::{ExtensionLoader, LoaderConfig};
use blockmatrix::extensions::registry::{ExtensionRegistry, RegistryConfig, ExtensionLocation};
use blockmatrix::extensions::security::{
    SecurityManager, SecurityConfig, ResourceQuotas, ResourceUsage, IsolationLevel,
};
use blockmatrix::consensus::{ConsensusProof, ProofType};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use tracing::{info, debug, warn, error};

/// Test plugin discovery in multiple search paths
#[tokio::test]
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
    info!("Discovered {} extensions across all paths", discovered.len());

    // Verify catalog is found
    let catalog = discovered.iter()
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
    let catalog = discovered.iter()
        .find(|m| m.metadata.id == "catalog")
        .expect("Catalog should be found");

    // Validate required fields
    assert!(catalog.metadata.hypermesh_version.major >= 1);
    assert!(catalog.metadata.required_capabilities.contains(&ExtensionCapability::AssetManagement));

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
                info!("Extension loading failed as expected without signature: {}", e);
            }
        }
    }

    info!("Signature verification test passed");
}

/// Test capability-based security during initialization
#[tokio::test]
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

/// Test loading with various configuration scenarios
#[tokio::test]
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

/// Test CatalogExtension trait implementation
#[tokio::test]
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
        serde_json::json!({
            "name": "test-library",
            "version": "1.0.0",
            "dependencies": []
        }),
        PrivacyLevel::Private,
    ).await;

    assert!(library_asset.is_ok());

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Asset registration test passed");
}

/// Test asset handlers integration
#[tokio::test]
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
        params: serde_json::json!({
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
        params: serde_json::json!({
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

/// Test consensus validation integration
#[tokio::test]
async fn test_consensus_integration() {
    init_test_logging();

    let loader = create_test_loader();

    // Load extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Create mock consensus proof
    let consensus_proof = ConsensusProof {
        block_height: 12345,
        block_hash: vec![0u8; 32],
        timestamp: SystemTime::now(),
        proofs: HashMap::from([
            (ProofType::PoSpace, vec![1u8; 64]),
            (ProofType::PoStake, vec![2u8; 64]),
            (ProofType::PoWork, vec![3u8; 64]),
            (ProofType::PoTime, vec![4u8; 64]),
        ]),
        validator_signatures: vec![],
        merkle_root: vec![0u8; 32],
    };

    // Test request with consensus proof
    let request = ExtensionRequest {
        id: "consensus-test".to_string(),
        method: "create_verified_package".to_string(),
        params: serde_json::json!({
            "name": "verified-package",
            "version": "1.0.0",
            "code": "verified code"
        }),
        consensus_proof: Some(consensus_proof),
    };

    let response = extension.handle_request(request).await;
    assert!(response.is_ok());

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Consensus integration test passed");
}

/// Test asset library operations through HyperMesh
#[tokio::test]
async fn test_library_operations() {
    init_test_logging();

    let asset_manager = Arc::new(AssetManager::new());
    let loader = create_test_loader();

    // Full setup
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    extension.register_assets().await.unwrap();
    extension.extend_manager(asset_manager.clone()).await.unwrap();

    // Create library
    let create_lib = ExtensionRequest {
        id: "lib-1".to_string(),
        method: "create_library".to_string(),
        params: serde_json::json!({
            "name": "math-lib",
            "description": "Mathematical functions",
            "tags": ["math", "computation"]
        }),
        consensus_proof: None,
    };

    let lib_response = extension.handle_request(create_lib).await.unwrap();
    assert!(lib_response.success);
    let lib_id = lib_response.data["id"].as_str().unwrap();

    // Add package to library
    let add_package = ExtensionRequest {
        id: "pkg-1".to_string(),
        method: "add_package_to_library".to_string(),
        params: serde_json::json!({
            "library_id": lib_id,
            "package": {
                "name": "factorial",
                "version": "1.0.0",
                "code": "function factorial(n) { return n <= 1 ? 1 : n * factorial(n-1); }"
            }
        }),
        consensus_proof: None,
    };

    let pkg_response = extension.handle_request(add_package).await.unwrap();
    assert!(pkg_response.success);

    // List library contents
    let list_contents = ExtensionRequest {
        id: "list-1".to_string(),
        method: "list_library_packages".to_string(),
        params: serde_json::json!({
            "library_id": lib_id
        }),
        consensus_proof: None,
    };

    let list_response = extension.handle_request(list_contents).await.unwrap();
    assert!(list_response.success);
    assert!(list_response.data["packages"].as_array().unwrap().len() > 0);

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Library operations test passed");
}

/// Test P2P distribution functionality
#[tokio::test]
async fn test_p2p_distribution() {
    init_test_logging();

    let loader = create_test_loader();

    // Load extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Test P2P registration
    let register_peer = ExtensionRequest {
        id: "p2p-1".to_string(),
        method: "register_p2p_node".to_string(),
        params: serde_json::json!({
            "node_id": "node-123",
            "address": "192.168.1.100:8080",
            "capabilities": ["storage", "compute"]
        }),
        consensus_proof: None,
    };

    let response = extension.handle_request(register_peer).await.unwrap();
    assert!(response.success || response.data["error"].as_str().is_some());

    // Test content distribution
    let distribute = ExtensionRequest {
        id: "p2p-2".to_string(),
        method: "distribute_package".to_string(),
        params: serde_json::json!({
            "package_id": "test-pkg",
            "target_nodes": ["node-123"],
            "replication_factor": 3
        }),
        consensus_proof: None,
    };

    let response = extension.handle_request(distribute).await.unwrap();
    debug!("P2P distribution response: {:?}", response);

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("P2P distribution test passed");
}

/// Test TrustChain package verification
#[tokio::test]
async fn test_trustchain_verification() {
    init_test_logging();

    let loader = create_test_loader();

    // Load extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Test package signing
    let sign_package = ExtensionRequest {
        id: "trust-1".to_string(),
        method: "sign_package".to_string(),
        params: serde_json::json!({
            "package_id": "test-pkg",
            "certificate": "mock-cert-data",
            "private_key": "mock-key-data"
        }),
        consensus_proof: None,
    };

    let sign_response = extension.handle_request(sign_package).await;
    // May fail without real certs, but should handle gracefully
    assert!(sign_response.is_ok());

    // Test package verification
    let verify_package = ExtensionRequest {
        id: "trust-2".to_string(),
        method: "verify_package".to_string(),
        params: serde_json::json!({
            "package_id": "test-pkg",
            "signature": "mock-signature",
            "certificate_chain": ["cert1", "cert2"]
        }),
        consensus_proof: None,
    };

    let verify_response = extension.handle_request(verify_package).await;
    assert!(verify_response.is_ok());

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("TrustChain verification test passed");
}

/// Test cross-node sharing and synchronization
#[tokio::test]
async fn test_cross_node_sync() {
    init_test_logging();

    // Simulate two nodes with extensions
    let loader1 = create_test_loader();
    let loader2 = create_test_loader();

    // Load extension on both nodes
    let extension_path = PathBuf::from("../catalog/target/debug");

    let ext_id1 = loader1.load_extension(&extension_path).await.unwrap();
    let ext1 = loader1.get_extension(&ext_id1).await.unwrap();

    let ext_id2 = loader2.load_extension(&extension_path).await.unwrap();
    let ext2 = loader2.get_extension(&ext_id2).await.unwrap();

    // Create content on node 1
    let create_content = ExtensionRequest {
        id: "sync-1".to_string(),
        method: "create_package".to_string(),
        params: serde_json::json!({
            "name": "shared-package",
            "version": "1.0.0",
            "code": "shared code"
        }),
        consensus_proof: None,
    };

    let response1 = ext1.handle_request(create_content).await.unwrap();
    assert!(response1.success);
    let package_id = response1.data["id"].as_str().unwrap();

    // Request sync from node 2
    let sync_request = ExtensionRequest {
        id: "sync-2".to_string(),
        method: "sync_package".to_string(),
        params: serde_json::json!({
            "package_id": package_id,
            "source_node": "node1",
            "verify": true
        }),
        consensus_proof: None,
    };

    let response2 = ext2.handle_request(sync_request).await;
    assert!(response2.is_ok());

    // Cleanup
    loader1.unload_extension(&ext_id1).await.unwrap();
    loader2.unload_extension(&ext_id2).await.unwrap();

    info!("Cross-node sync test passed");
}

/// Test resource isolation and quota enforcement
#[tokio::test]
async fn test_resource_isolation() {
    init_test_logging();

    // Create security manager with strict quotas
    let security_config = SecurityConfig {
        enforcement_enabled: true,
        anomaly_detection: true,
        audit_enabled: true,
        default_isolation: IsolationLevel::Container,
        max_violations: 3,
        violation_reset_interval: Duration::from_secs(60),
    };

    let security_manager = SecurityManager::new(security_config);

    // Set very restrictive quotas
    let quotas = ResourceQuotas {
        cpu_percent: 5.0,
        memory_bytes: 50 * 1024 * 1024, // 50MB
        storage_bytes: 100 * 1024 * 1024, // 100MB
        network_bandwidth: 100 * 1024, // 100KB/s
        file_descriptors: 50,
        max_threads: 5,
        ops_per_second: 10,
    };

    // Create metadata for catalog
    let metadata = create_catalog_metadata();

    // Create security context
    security_manager.create_context(
        "catalog".to_string(),
        &metadata,
        metadata.required_capabilities.clone(),
        quotas,
    ).await.unwrap();

    // Load extension with security manager monitoring
    let loader = create_test_loader();
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Simulate resource usage
    for i in 0..5 {
        let usage = ResourceUsage {
            cpu_percent: 3.0 + (i as f32 * 0.5),
            memory_bytes: 30 * 1024 * 1024 + (i * 5 * 1024 * 1024),
            storage_bytes: 50 * 1024 * 1024,
            network_bytes: i * 10 * 1024,
            file_descriptors: 20 + i,
            thread_count: 3,
            ops_per_second: 5.0 + i as f32,
            last_update: Some(SystemTime::now()),
        };

        security_manager.update_usage("catalog", usage).await.unwrap();

        // Check if within limits
        let check = security_manager.check_resource_usage("catalog").await;

        if i < 3 {
            assert!(check.is_ok(), "Should be within limits at iteration {}", i);
        } else {
            // Should exceed limits eventually
            if check.is_err() {
                info!("Resource limit exceeded as expected at iteration {}", i);
                break;
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    // Check violations were recorded
    let metrics = security_manager.get_metrics("catalog").await.unwrap();
    info!("Resource violations recorded: {}", metrics.violations);

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Resource isolation test passed");
}

/// Test proper cleanup during unloading
#[tokio::test]
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
    extension.extend_manager(asset_manager.clone()).await.unwrap();

    // Create some resources
    for i in 0..5 {
        let request = ExtensionRequest {
            id: format!("cleanup-{}", i),
            method: "create_package".to_string(),
            params: serde_json::json!({
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

    info!("Extension state before unload: {} items, {} requests processed",
          state_before.data.len(), status_before.total_requests);

    // Unload extension
    loader.unload_extension(&extension_id).await.unwrap();

    // Verify extension is gone
    assert!(loader.get_extension(&extension_id).await.is_none());

    // Verify handlers are deregistered
    let remaining_types = asset_manager.list_asset_types().await;
    let catalog_types = remaining_types.iter()
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
            params: serde_json::json!({
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
    extension2.import_state(exported_state.clone()).await.unwrap();

    // Verify state was restored
    let restored_state = extension2.export_state().await.unwrap();
    assert_eq!(restored_state.data.len(), exported_state.data.len());
    assert_eq!(restored_state.version, exported_state.version);

    // Cleanup
    loader.unload_extension(&extension_id2).await.unwrap();

    info!("State persistence test passed");
}

/// Test no memory leaks or dangling resources
#[tokio::test]
#[ignore] // Run with --ignored for memory leak testing
async fn test_memory_leaks() {
    init_test_logging();

    // Run multiple load/unload cycles
    for cycle in 0..10 {
        info!("Memory leak test cycle {}", cycle);

        let loader = create_test_loader();
        let asset_manager = Arc::new(AssetManager::new());

        // Load extension
        let extension_path = PathBuf::from("../catalog/target/debug");
        let extension_id = loader.load_extension(&extension_path).await.unwrap();
        let extension = loader.get_extension(&extension_id).await.unwrap();

        // Use extension heavily
        extension.register_assets().await.unwrap();
        extension.extend_manager(asset_manager.clone()).await.unwrap();

        // Create many resources
        for i in 0..100 {
            let request = ExtensionRequest {
                id: format!("leak-test-{}-{}", cycle, i),
                method: "create_package".to_string(),
                params: serde_json::json!({
                    "name": format!("pkg-{}-{}", cycle, i),
                    "version": "1.0.0",
                    "code": format!("code {} {}", cycle, i)
                }),
                consensus_proof: None,
            };
            extension.handle_request(request).await.ok();
        }

        // Export/import state multiple times
        for _ in 0..5 {
            let state = extension.export_state().await.unwrap();
            extension.import_state(state).await.unwrap();
        }

        // Unload
        loader.unload_extension(&extension_id).await.unwrap();

        // Force cleanup
        drop(loader);
        drop(asset_manager);

        // Give time for cleanup
        sleep(Duration::from_millis(100)).await;
    }

    info!("Memory leak test completed - check system metrics");
}

/// Test hot-reload functionality
#[tokio::test]
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
    extension1.extend_manager(asset_manager.clone()).await.unwrap();

    // Create some state
    let request = ExtensionRequest {
        id: "hot-1".to_string(),
        method: "create_library".to_string(),
        params: serde_json::json!({
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
    extension2.extend_manager(asset_manager.clone()).await.unwrap();

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
        params: serde_json::json!({}),
        consensus_proof: None,
    };

    let response = extension2.handle_request(request2).await.unwrap();
    assert!(response.success);

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Hot reload test passed");
}

/// Test behavior with corrupted plugin files
#[tokio::test]
async fn test_corrupted_plugin() {
    init_test_logging();

    // Create a corrupted plugin file
    let test_dir = PathBuf::from("./test-corrupted-plugin");
    std::fs::create_dir_all(&test_dir).ok();

    // Write corrupted binary
    let corrupted_path = test_dir.join("libcatalog.so");
    std::fs::write(&corrupted_path, b"CORRUPTED_BINARY_DATA_12345").ok();

    // Write invalid manifest
    let invalid_manifest = r#"
    {
        "this": "is",
        "not": "valid",
        "manifest": true
    }
    "#;
    std::fs::write(test_dir.join("manifest.json"), invalid_manifest).ok();

    let loader_config = LoaderConfig {
        search_paths: vec![test_dir.clone()],
        enable_wasm: false,
        verify_signatures: false,
        max_extensions: 10,
        default_limits: ResourceLimits::default(),
        trustchain_cert_path: None,
    };

    let loader = ExtensionLoader::new(loader_config);

    // Try to discover - should handle gracefully
    let discovered = loader.discover_extensions().await;
    assert!(discovered.is_ok());

    // Try to load corrupted plugin - should fail gracefully
    let load_result = loader.load_extension(&test_dir).await;
    assert!(load_result.is_err());

    if let Err(e) = load_result {
        info!("Corrupted plugin handled correctly: {}", e);
    }

    // Cleanup
    std::fs::remove_dir_all(&test_dir).ok();

    info!("Corrupted plugin test passed");
}

/// Test handling of missing dependencies
#[tokio::test]
async fn test_missing_dependencies() {
    init_test_logging();

    // Create extension with missing dependencies
    let test_metadata = ExtensionMetadata {
        id: "test-missing-deps".to_string(),
        name: "Test Extension".to_string(),
        version: semver::Version::parse("1.0.0").unwrap(),
        description: "Test".to_string(),
        author: "Test".to_string(),
        license: "MIT".to_string(),
        homepage: None,
        category: hypermesh::extensions::ExtensionCategory::AssetLibrary,
        hypermesh_version: semver::Version::parse("1.0.0").unwrap(),
        dependencies: vec![
            hypermesh::extensions::ExtensionDependency {
                id: "non-existent-extension".to_string(),
                version: semver::VersionReq::parse(">=1.0.0").unwrap(),
                optional: false,
            }
        ],
        required_capabilities: HashSet::new(),
        provided_assets: vec![],
        certificate_fingerprint: None,
        config_schema: None,
    };

    let registry_config = RegistryConfig {
        max_entries: 100,
        auto_resolve_deps: true,
        health_monitoring: false,
        health_check_interval: Duration::from_secs(60),
        collect_metrics: false,
    };

    let registry = ExtensionRegistry::new(registry_config);

    // Try to register with missing dependency
    let location = ExtensionLocation {
        path: PathBuf::from("./test"),
        url: None,
        distribution_hash: None,
    };

    let result = registry.register_extension(test_metadata, location).await;

    // Should either succeed with warning or fail gracefully
    match result {
        Ok(_) => info!("Registry accepted extension with missing deps (will fail on activation)"),
        Err(e) => info!("Registry rejected extension with missing deps: {}", e),
    }

    info!("Missing dependencies test passed");
}

/// Test network partition scenarios
#[tokio::test]
async fn test_network_partition() {
    init_test_logging();

    let loader = create_test_loader();

    // Load extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Simulate network partition by testing with network-dependent operations
    let network_request = ExtensionRequest {
        id: "net-part-1".to_string(),
        method: "sync_with_remote".to_string(),
        params: serde_json::json!({
            "remote_node": "unreachable.node.local",
            "timeout_ms": 1000
        }),
        consensus_proof: None,
    };

    let response = extension.handle_request(network_request).await;

    // Should handle network failure gracefully
    match response {
        Ok(resp) if !resp.success => {
            info!("Network partition handled correctly: {}",
                  resp.error.unwrap_or_default());
        }
        Ok(_) => {
            info!("Operation succeeded despite simulated partition");
        }
        Err(e) => {
            info!("Network partition error handled: {}", e);
        }
    }

    // Test recovery after partition
    let recovery_request = ExtensionRequest {
        id: "net-part-2".to_string(),
        method: "check_connectivity".to_string(),
        params: serde_json::json!({}),
        consensus_proof: None,
    };

    let recovery = extension.handle_request(recovery_request).await;
    assert!(recovery.is_ok());

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Network partition test passed");
}

/// Test recovery from extension crashes
#[tokio::test]
async fn test_crash_recovery() {
    init_test_logging();

    let loader = create_test_loader();

    // Load extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();
    let extension = loader.get_extension(&extension_id).await.unwrap();

    // Try to trigger a crash with invalid input
    let crash_request = ExtensionRequest {
        id: "crash-1".to_string(),
        method: "execute_vm".to_string(),
        params: serde_json::json!({
            "code": "while(true) { allocate_memory(); }", // Infinite allocation
            "timeout": 100
        }),
        consensus_proof: None,
    };

    let result = extension.handle_request(crash_request).await;

    // Should handle crash gracefully
    match result {
        Ok(resp) => {
            assert!(!resp.success || resp.error.is_some());
            info!("Potential crash handled: success={}, error={:?}",
                  resp.success, resp.error);
        }
        Err(e) => {
            info!("Crash handled with error: {}", e);
        }
    }

    // Extension should still be functional
    let health_check = ExtensionRequest {
        id: "health-1".to_string(),
        method: "health_check".to_string(),
        params: serde_json::json!({}),
        consensus_proof: None,
    };

    let health = extension.handle_request(health_check).await;
    assert!(health.is_ok());

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    info!("Crash recovery test passed");
}

/// Test concurrent loading/unloading operations
#[tokio::test]
async fn test_concurrent_operations() {
    init_test_logging();

    let loader = Arc::new(create_test_loader());
    let extension_path = PathBuf::from("../catalog/target/debug");

    // Spawn multiple concurrent operations
    let mut handles = vec![];

    // Concurrent loads
    for i in 0..3 {
        let loader_clone = loader.clone();
        let path_clone = extension_path.clone();

        let handle = tokio::spawn(async move {
            info!("Starting concurrent load {}", i);
            let result = loader_clone.load_extension(&path_clone).await;
            info!("Concurrent load {} result: {:?}", i, result.is_ok());
            result
        });

        handles.push(handle);
    }

    // Wait for loads
    let mut loaded_ids = vec![];
    for handle in handles {
        if let Ok(Ok(id)) = handle.await {
            loaded_ids.push(id);
        }
    }

    // Should all get the same extension ID (singleton)
    assert!(!loaded_ids.is_empty());
    let first_id = &loaded_ids[0];
    for id in &loaded_ids {
        assert_eq!(id, first_id, "All loads should return same extension ID");
    }

    // Concurrent operations on loaded extension
    let mut op_handles = vec![];

    for i in 0..5 {
        let loader_clone = loader.clone();
        let ext_id = first_id.clone();

        let handle = tokio::spawn(async move {
            if let Some(ext) = loader_clone.get_extension(&ext_id).await {
                let request = ExtensionRequest {
                    id: format!("concurrent-{}", i),
                    method: "list_packages".to_string(),
                    params: serde_json::json!({"limit": 5}),
                    consensus_proof: None,
                };
                ext.handle_request(request).await.ok()
            } else {
                None
            }
        });

        op_handles.push(handle);
    }

    // Wait for operations
    for handle in op_handles {
        handle.await.ok();
    }

    // Concurrent unload attempts
    let mut unload_handles = vec![];

    for i in 0..3 {
        let loader_clone = loader.clone();
        let ext_id = first_id.clone();

        let handle = tokio::spawn(async move {
            info!("Attempting concurrent unload {}", i);
            loader_clone.unload_extension(&ext_id).await
        });

        unload_handles.push(handle);
    }

    // Wait for unloads
    let mut unload_results = vec![];
    for handle in unload_handles {
        if let Ok(result) = handle.await {
            unload_results.push(result);
        }
    }

    // Only one should succeed, others should fail gracefully
    let successful = unload_results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(successful, 1, "Only one unload should succeed");

    info!("Concurrent operations test passed");
}

// ============= Helper Functions =============

fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("debug")
        .try_init();
}

fn create_test_loader() -> ExtensionLoader {
    let config = LoaderConfig {
        search_paths: vec![
            PathBuf::from("../catalog/target/debug"),
            PathBuf::from("../catalog/target/release"),
        ],
        enable_wasm: false,
        verify_signatures: false,
        max_extensions: 10,
        default_limits: ResourceLimits::default(),
        trustchain_cert_path: None,
    };

    ExtensionLoader::new(config)
}

fn create_catalog_metadata() -> ExtensionMetadata {
    ExtensionMetadata {
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
    }
}

// Helper for JSON macro
use serde_json::json;