// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Reliability and error handling tests
//!
//! Tests for resource isolation, error recovery, crash handling, network partitions,
//! corrupted plugins, missing dependencies, memory leaks, and concurrent operations.
//!
//! NOTE: All tests marked #[ignore] - Requires Catalog extension implementation

use super::*;

/// Test resource isolation and quota enforcement
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
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
        memory_bytes: 50 * 1024 * 1024,   // 50MB
        storage_bytes: 100 * 1024 * 1024, // 100MB
        network_bandwidth: 100 * 1024,    // 100KB/s
        file_descriptors: 50,
        max_threads: 5,
        ops_per_second: 10,
    };

    // Create metadata for catalog
    let metadata = create_catalog_metadata();

    // Create security context
    security_manager
        .create_context(
            "catalog".to_string(),
            &metadata,
            metadata.required_capabilities.clone(),
            quotas,
        )
        .await
        .unwrap();

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

        security_manager
            .update_usage("catalog", usage)
            .await
            .unwrap();

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

/// Test no memory leaks or dangling resources
#[tokio::test]
#[ignore = "Requires Catalog extension implementation - run with --ignored for memory leak testing"]
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
        extension
            .extend_manager(asset_manager.clone())
            .await
            .unwrap();

        // Create many resources
        for i in 0..100 {
            let request = ExtensionRequest {
                id: format!("leak-test-{}-{}", cycle, i),
                method: "create_package".to_string(),
                params: json!({
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

/// Test behavior with corrupted plugin files
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
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
#[ignore = "Requires Catalog extension implementation"]
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
        dependencies: vec![hypermesh::extensions::ExtensionDependency {
            id: "non-existent-extension".to_string(),
            version: semver::VersionReq::parse(">=1.0.0").unwrap(),
            optional: false,
        }],
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
#[ignore = "Requires Catalog extension implementation"]
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
        params: json!({
            "remote_node": "unreachable.node.local",
            "timeout_ms": 1000
        }),
        consensus_proof: None,
    };

    let response = extension.handle_request(network_request).await;

    // Should handle network failure gracefully
    match response {
        Ok(resp) if !resp.success => {
            info!(
                "Network partition handled correctly: {}",
                resp.error.unwrap_or_default()
            );
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
        params: json!({}),
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
#[ignore = "Requires Catalog extension implementation"]
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
        params: json!({
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
            info!(
                "Potential crash handled: success={}, error={:?}",
                resp.success, resp.error
            );
        }
        Err(e) => {
            info!("Crash handled with error: {}", e);
        }
    }

    // Extension should still be functional
    let health_check = ExtensionRequest {
        id: "health-1".to_string(),
        method: "health_check".to_string(),
        params: json!({}),
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
#[ignore = "Requires Catalog extension implementation"]
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
                    params: json!({"limit": 5}),
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
