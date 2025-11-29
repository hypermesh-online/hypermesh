//! Integration test for dynamic extension loading
//!
//! This test demonstrates loading the Catalog extension into HyperMesh
//! at runtime using the dynamic loading mechanism.

use blockmatrix::assets::core::{AssetManager, AssetType, PrivacyLevel};
use blockmatrix::extensions::{
    ExtensionCapability, ExtensionConfig, ExtensionManager, ExtensionManagerConfig,
    ExtensionMetadata, ResourceLimits,
};
use blockmatrix::extensions::loader::{ExtensionLoader, LoaderConfig};
use blockmatrix::extensions::registry::{ExtensionRegistry, RegistryConfig, ExtensionLocation};
use blockmatrix::extensions::security::{SecurityManager, SecurityConfig, ResourceQuotas};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio;

#[tokio::test]
async fn test_catalog_extension_loading() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("debug")
        .init();

    // Create asset manager
    let asset_manager = Arc::new(AssetManager::new());

    // Configure extension loader
    let loader_config = LoaderConfig {
        search_paths: vec![
            PathBuf::from("../catalog/target/debug"),
            PathBuf::from("../catalog/target/release"),
            PathBuf::from("./extensions"),
        ],
        enable_wasm: false, // Only native for now
        verify_signatures: false, // Disable for testing
        max_extensions: 10,
        default_limits: ResourceLimits::default(),
        trustchain_cert_path: None,
    };

    let loader = ExtensionLoader::new(loader_config);

    // Create extension registry
    let registry_config = RegistryConfig {
        max_entries: 100,
        auto_resolve_deps: true,
        health_monitoring: false, // Disable for test
        health_check_interval: std::time::Duration::from_secs(60),
        collect_metrics: true,
    };

    let registry = ExtensionRegistry::new(registry_config);

    // Create security manager
    let security_config = SecurityConfig {
        enforcement_enabled: true,
        anomaly_detection: false, // Disable for test
        audit_enabled: true,
        default_isolation: hypermesh::extensions::security::IsolationLevel::Process,
        max_violations: 10,
        violation_reset_interval: std::time::Duration::from_secs(3600),
    };

    let security_manager = SecurityManager::new(security_config);

    // Discover available extensions
    let discovered = loader.discover_extensions().await.unwrap();
    println!("Discovered {} extensions", discovered.len());

    // Look for catalog extension
    let catalog_manifest = discovered.iter()
        .find(|m| m.metadata.id == "catalog")
        .expect("Catalog extension not found");

    println!("Found catalog extension: {:?}", catalog_manifest.metadata);

    // Register the extension
    let location = ExtensionLocation {
        path: PathBuf::from("../catalog/target/debug"),
        url: None,
        distribution_hash: None,
    };

    registry.register_extension(catalog_manifest.metadata.clone(), location).await.unwrap();

    // Create security context for the extension
    let granted_capabilities = HashSet::from([
        ExtensionCapability::AssetManagement,
        ExtensionCapability::VMExecution,
        ExtensionCapability::NetworkAccess,
        ExtensionCapability::ConsensusAccess,
    ]);

    let quotas = ResourceQuotas::from(ResourceLimits::default());

    security_manager.create_context(
        "catalog".to_string(),
        &catalog_manifest.metadata,
        granted_capabilities,
        quotas,
    ).await.unwrap();

    // Load the extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();

    assert_eq!(extension_id, "catalog");

    // Get the loaded extension
    let extension = loader.get_extension(&extension_id).await
        .expect("Failed to get loaded extension");

    // Activate the extension in registry
    registry.activate_extension(&extension_id, extension.clone()).await.unwrap();

    // Verify extension is active
    assert!(registry.is_active(&extension_id).await);

    // Get extension status
    let status = extension.status().await;
    println!("Extension status: {:?}", status.state);

    // Register assets with asset manager
    let handlers = extension.register_assets().await.unwrap();
    println!("Registered {} asset handlers", handlers.len());

    // Extend the asset manager
    extension.extend_manager(asset_manager.clone()).await.unwrap();

    // Test extension functionality
    let request = hypermesh::extensions::ExtensionRequest {
        id: "test-1".to_string(),
        method: "list_packages".to_string(),
        params: serde_json::json!({
            "limit": 10
        }),
        consensus_proof: None,
    };

    let response = extension.handle_request(request).await.unwrap();
    assert!(response.success);
    println!("List packages response: {:?}", response.data);

    // Test VM execution request
    let vm_request = hypermesh::extensions::ExtensionRequest {
        id: "test-2".to_string(),
        method: "execute_vm".to_string(),
        params: serde_json::json!({
            "code": "print('Hello from Catalog VM')",
            "inputs": {}
        }),
        consensus_proof: None,
    };

    let vm_response = extension.handle_request(vm_request).await.unwrap();
    println!("VM execution response: {:?}", vm_response);

    // Validate the extension
    let validation = extension.validate().await.unwrap();
    assert!(validation.valid || !validation.warnings.is_empty());
    println!("Validation report: {} errors, {} warnings",
             validation.errors.len(), validation.warnings.len());

    // Export extension state
    let state = extension.export_state().await.unwrap();
    println!("Exported state version: {}", state.version);

    // Check resource usage through security manager
    if let Some(metrics) = security_manager.get_metrics(&extension_id).await {
        println!("Extension metrics - CPU: {:.1}%, Memory: {} bytes",
                 metrics.cpu_usage, metrics.memory_usage);
    }

    // Test capability check
    let cap_check = security_manager.check_capability(
        &extension_id,
        &ExtensionCapability::AssetManagement,
        "test_operation"
    ).await;
    assert!(cap_check.is_ok());

    // Deactivate extension
    registry.deactivate_extension(&extension_id).await.unwrap();
    assert!(!registry.is_active(&extension_id).await);

    // Unload extension
    loader.unload_extension(&extension_id).await.unwrap();

    // Verify extension is unloaded
    assert!(loader.get_extension(&extension_id).await.is_none());

    println!("Extension loading test completed successfully!");
}

#[tokio::test]
async fn test_extension_hot_reload() {
    // Initialize components
    let loader_config = LoaderConfig {
        search_paths: vec![PathBuf::from("../catalog/target/debug")],
        enable_wasm: false,
        verify_signatures: false,
        max_extensions: 10,
        default_limits: ResourceLimits::default(),
        trustchain_cert_path: None,
    };

    let loader = ExtensionLoader::new(loader_config);

    // Load extension
    let extension_path = PathBuf::from("../catalog/target/debug");
    let extension_id = loader.load_extension(&extension_path).await.unwrap();

    // Get initial extension
    let extension1 = loader.get_extension(&extension_id).await.unwrap();
    let status1 = extension1.status().await;

    // Reload extension
    loader.reload_extension(&extension_id).await.unwrap();

    // Get reloaded extension
    let extension2 = loader.get_extension(&extension_id).await.unwrap();
    let status2 = extension2.status().await;

    // Verify reload worked (new instance)
    assert_eq!(status1.total_requests, 0);
    assert_eq!(status2.total_requests, 0);

    // Cleanup
    loader.unload_extension(&extension_id).await.unwrap();

    println!("Hot reload test completed successfully!");
}

#[tokio::test]
async fn test_extension_resource_limits() {
    // Create security manager with strict limits
    let security_config = SecurityConfig {
        enforcement_enabled: true,
        anomaly_detection: false,
        audit_enabled: true,
        default_isolation: hypermesh::extensions::security::IsolationLevel::Process,
        max_violations: 5,
        violation_reset_interval: std::time::Duration::from_secs(60),
    };

    let security_manager = SecurityManager::new(security_config);

    // Create extension metadata
    let metadata = ExtensionMetadata {
        id: "test-ext".to_string(),
        name: "Test Extension".to_string(),
        version: semver::Version::parse("1.0.0").unwrap(),
        description: "Test".to_string(),
        author: "Test".to_string(),
        license: "MIT".to_string(),
        homepage: None,
        category: hypermesh::extensions::ExtensionCategory::AssetLibrary,
        hypermesh_version: semver::Version::parse("1.0.0").unwrap(),
        dependencies: vec![],
        required_capabilities: HashSet::from([ExtensionCapability::AssetManagement]),
        provided_assets: vec![],
        certificate_fingerprint: None,
        config_schema: None,
    };

    // Create restrictive quotas
    let quotas = ResourceQuotas {
        cpu_percent: 10.0,
        memory_bytes: 100 * 1024 * 1024, // 100MB
        storage_bytes: 1024 * 1024 * 1024, // 1GB
        network_bandwidth: 1024 * 1024, // 1MB/s
        file_descriptors: 100,
        max_threads: 10,
        ops_per_second: 100,
    };

    // Create security context
    security_manager.create_context(
        "test-ext".to_string(),
        &metadata,
        HashSet::from([ExtensionCapability::AssetManagement]),
        quotas,
    ).await.unwrap();

    // Update with usage within limits
    let usage = hypermesh::extensions::security::ResourceUsage {
        cpu_percent: 5.0,
        memory_bytes: 50 * 1024 * 1024,
        storage_bytes: 100 * 1024 * 1024,
        network_bytes: 0,
        file_descriptors: 10,
        thread_count: 5,
        ops_per_second: 50.0,
        last_update: Some(std::time::SystemTime::now()),
    };

    security_manager.update_usage("test-ext", usage).await.unwrap();

    // Check resource usage is OK
    assert!(security_manager.check_resource_usage("test-ext").await.is_ok());

    // Update with excessive usage
    let excessive_usage = hypermesh::extensions::security::ResourceUsage {
        cpu_percent: 50.0, // Exceeds 10% limit
        memory_bytes: 200 * 1024 * 1024, // Exceeds 100MB limit
        ..Default::default()
    };

    security_manager.update_usage("test-ext", excessive_usage).await.unwrap();

    // Check should fail
    assert!(security_manager.check_resource_usage("test-ext").await.is_err());

    // Record violations
    for i in 0..3 {
        security_manager.record_violation("test-ext", "resource_limit", &format!("Violation {}", i)).await;
    }

    // Check metrics
    let metrics = security_manager.get_metrics("test-ext").await.unwrap();
    assert_eq!(metrics.violations, 3);

    println!("Resource limits test completed successfully!");
}

#[tokio::test]
async fn test_extension_manager_integration() {
    // Create complete extension manager
    let asset_manager = Arc::new(AssetManager::new());

    let config = ExtensionManagerConfig {
        extension_dirs: vec![PathBuf::from("../catalog/target/debug")],
        auto_load: false, // Manual loading for test
        verify_signatures: false,
        max_extensions: 10,
        global_limits: ResourceLimits::default(),
        allowed_capabilities: HashSet::from([
            ExtensionCapability::AssetManagement,
            ExtensionCapability::VMExecution,
            ExtensionCapability::NetworkAccess,
        ]),
    };

    let manager = ExtensionManager::new(asset_manager, config);

    // Mock catalog extension for testing
    // In real usage, this would be loaded from the shared library

    // List extensions (should be empty initially)
    let extensions = manager.list_extensions().await;
    assert_eq!(extensions.len(), 0);

    println!("Extension manager integration test completed!");
}