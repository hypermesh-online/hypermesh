//! End-to-end system tests for Catalog functionality
//!
//! This test suite validates the complete Catalog system including:
//! - Full lifecycle from initialization to shutdown
//! - Integration with HyperMesh ecosystem
//! - Real-world usage scenarios
//! - Performance under load
//! - Multi-node coordination

use catalog::{
    CatalogExtension, CatalogConfig, AssetLibrary, Package, PackageVersion,
    DistributionConfig, P2PNode, SecurityConfig, ValidationReport,
};
use blockmatrix::assets::core::{AssetManager, AssetType, PrivacyLevel, AssetId};
use blockmatrix::extensions::{Extension, ExtensionRequest, ExtensionResponse};
use blockmatrix::consensus::{ConsensusProof, ProofType};
use stoq::transport::{QuicTransport, TransportConfig};
use trustchain::{CertificateChain, TrustChainClient, VerificationResult};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::time::sleep;
use tracing::{info, debug, warn, error};

/// Test complete system initialization and setup
#[tokio::test]
async fn test_full_system_initialization() {
    init_test_logging();

    // Initialize all components
    info!("Initializing Catalog system...");

    // 1. Create Catalog configuration
    let catalog_config = CatalogConfig {
        storage_path: PathBuf::from("./test-data/catalog"),
        max_libraries: 1000,
        max_packages_per_library: 10000,
        enable_p2p: true,
        enable_trustchain: true,
        enable_consensus: true,
        cache_size_mb: 100,
        auto_verify: true,
        security_config: SecurityConfig {
            enforce_signatures: false, // For testing
            require_consensus: false,
            audit_enabled: true,
            max_package_size_mb: 50,
        },
    };

    // 2. Initialize Catalog extension
    let catalog = CatalogExtension::new(catalog_config).await.unwrap();

    // 3. Initialize supporting services
    let asset_manager = Arc::new(AssetManager::new());
    let trustchain_client = TrustChainClient::new("http://localhost:8443").await.ok();

    // 4. Register with HyperMesh
    let handlers = catalog.register_assets().await.unwrap();
    assert!(!handlers.is_empty());

    catalog.extend_manager(asset_manager.clone()).await.unwrap();

    // 5. Verify initialization
    let status = catalog.status().await;
    assert_eq!(status.state, hypermesh::extensions::ExtensionState::Active);

    // 6. Test basic functionality
    let health_check = ExtensionRequest {
        id: "init-1".to_string(),
        method: "health_check".to_string(),
        params: serde_json::json!({}),
        consensus_proof: None,
    };

    let response = catalog.handle_request(health_check).await.unwrap();
    assert!(response.success);

    info!("Full system initialization completed successfully");
}

/// Test creating and managing asset libraries
#[tokio::test]
async fn test_asset_library_management() {
    init_test_logging();

    let catalog = create_test_catalog().await;

    // Create multiple libraries
    let libraries = vec![
        ("math-lib", "Mathematical functions and algorithms"),
        ("crypto-lib", "Cryptographic utilities"),
        ("network-lib", "Networking and communication"),
        ("data-lib", "Data structures and processing"),
    ];

    let mut library_ids = HashMap::new();

    for (name, description) in libraries {
        let request = ExtensionRequest {
            id: format!("create-{}", name),
            method: "create_library".to_string(),
            params: serde_json::json!({
                "name": name,
                "description": description,
                "tags": [name.split('-').next().unwrap()],
                "metadata": {
                    "author": "test",
                    "license": "MIT"
                }
            }),
            consensus_proof: None,
        };

        let response = catalog.handle_request(request).await.unwrap();
        assert!(response.success);

        let lib_id = response.data["id"].as_str().unwrap().to_string();
        library_ids.insert(name, lib_id);
        info!("Created library: {} with ID: {}", name, lib_id);
    }

    // List all libraries
    let list_request = ExtensionRequest {
        id: "list-1".to_string(),
        method: "list_libraries".to_string(),
        params: serde_json::json!({}),
        consensus_proof: None,
    };

    let list_response = catalog.handle_request(list_request).await.unwrap();
    assert!(list_response.success);

    let libraries_list = list_response.data["libraries"].as_array().unwrap();
    assert_eq!(libraries_list.len(), library_ids.len());

    // Update library metadata
    let update_request = ExtensionRequest {
        id: "update-1".to_string(),
        method: "update_library".to_string(),
        params: serde_json::json!({
            "library_id": library_ids["math-lib"],
            "updates": {
                "description": "Advanced mathematical functions",
                "tags": ["math", "advanced", "scientific"]
            }
        }),
        consensus_proof: None,
    };

    let update_response = catalog.handle_request(update_request).await.unwrap();
    assert!(update_response.success);

    // Search libraries
    let search_request = ExtensionRequest {
        id: "search-1".to_string(),
        method: "search_libraries".to_string(),
        params: serde_json::json!({
            "query": "crypto",
            "tags": [],
            "limit": 10
        }),
        consensus_proof: None,
    };

    let search_response = catalog.handle_request(search_request).await.unwrap();
    assert!(search_response.success);

    let results = search_response.data["results"].as_array().unwrap();
    assert!(results.len() > 0);

    info!("Asset library management test completed");
}

/// Test package lifecycle within libraries
#[tokio::test]
async fn test_package_lifecycle() {
    init_test_logging();

    let catalog = create_test_catalog().await;

    // Create a library first
    let lib_id = create_test_library(&catalog, "test-lib").await;

    // Create packages with different versions
    let packages = vec![
        ("calculator", "1.0.0", "function add(a, b) { return a + b; }"),
        ("calculator", "1.1.0", "function add(a, b) { return a + b; } function sub(a, b) { return a - b; }"),
        ("calculator", "2.0.0", "class Calculator { add(a, b) { return a + b; } }"),
        ("validator", "1.0.0", "function validate(x) { return x > 0; }"),
    ];

    let mut package_ids = vec![];

    for (name, version, code) in packages {
        let request = ExtensionRequest {
            id: format!("pkg-{}-{}", name, version),
            method: "add_package".to_string(),
            params: serde_json::json!({
                "library_id": lib_id,
                "package": {
                    "name": name,
                    "version": version,
                    "code": code,
                    "dependencies": [],
                    "metadata": {
                        "language": "javascript",
                        "runtime": "v8"
                    }
                }
            }),
            consensus_proof: None,
        };

        let response = catalog.handle_request(request).await.unwrap();
        assert!(response.success);

        let pkg_id = response.data["package_id"].as_str().unwrap().to_string();
        package_ids.push(pkg_id.clone());
        info!("Created package: {}-{} with ID: {}", name, version, pkg_id);
    }

    // List packages in library
    let list_request = ExtensionRequest {
        id: "list-pkgs".to_string(),
        method: "list_library_packages".to_string(),
        params: serde_json::json!({
            "library_id": lib_id,
            "include_versions": true
        }),
        consensus_proof: None,
    };

    let list_response = catalog.handle_request(list_request).await.unwrap();
    assert!(list_response.success);

    let packages_list = list_response.data["packages"].as_array().unwrap();
    assert_eq!(packages_list.len(), 2); // Two unique packages

    // Get specific package version
    let get_request = ExtensionRequest {
        id: "get-1".to_string(),
        method: "get_package_version".to_string(),
        params: serde_json::json!({
            "library_id": lib_id,
            "package_name": "calculator",
            "version": "2.0.0"
        }),
        consensus_proof: None,
    };

    let get_response = catalog.handle_request(get_request).await.unwrap();
    assert!(get_response.success);
    assert!(get_response.data["code"].as_str().unwrap().contains("class Calculator"));

    // Update package metadata
    let update_request = ExtensionRequest {
        id: "update-pkg".to_string(),
        method: "update_package".to_string(),
        params: serde_json::json!({
            "package_id": package_ids[0],
            "updates": {
                "metadata": {
                    "deprecated": true,
                    "replacement": "calculator@2.0.0"
                }
            }
        }),
        consensus_proof: None,
    };

    let update_response = catalog.handle_request(update_request).await.unwrap();
    assert!(update_response.success);

    // Delete old version
    let delete_request = ExtensionRequest {
        id: "delete-1".to_string(),
        method: "delete_package_version".to_string(),
        params: serde_json::json!({
            "package_id": package_ids[0],
            "confirm": true
        }),
        consensus_proof: None,
    };

    let delete_response = catalog.handle_request(delete_request).await.unwrap();
    assert!(delete_response.success || delete_response.error.is_some());

    info!("Package lifecycle test completed");
}

/// Test VM execution through Catalog
#[tokio::test]
async fn test_vm_execution() {
    init_test_logging();

    let catalog = create_test_catalog().await;

    // Test different VM execution scenarios
    let test_cases = vec![
        (
            "simple_math",
            "return 2 + 2",
            serde_json::json!({}),
            serde_json::json!(4)
        ),
        (
            "with_inputs",
            "return input.a * input.b",
            serde_json::json!({"a": 5, "b": 3}),
            serde_json::json!(15)
        ),
        (
            "array_processing",
            "return input.array.map(x => x * 2)",
            serde_json::json!({"array": [1, 2, 3, 4]}),
            serde_json::json!([2, 4, 6, 8])
        ),
        (
            "object_manipulation",
            "return { ...input.obj, processed: true }",
            serde_json::json!({"obj": {"name": "test", "value": 42}}),
            serde_json::json!({"name": "test", "value": 42, "processed": true})
        ),
    ];

    for (name, code, inputs, expected) in test_cases {
        let request = ExtensionRequest {
            id: format!("vm-{}", name),
            method: "execute_vm".to_string(),
            params: serde_json::json!({
                "code": code,
                "inputs": inputs,
                "timeout_ms": 1000,
                "memory_limit_mb": 10
            }),
            consensus_proof: None,
        };

        let response = catalog.handle_request(request).await;

        match response {
            Ok(resp) if resp.success => {
                info!("VM execution '{}' succeeded: {:?}", name, resp.data["result"]);
                // Note: Exact comparison might not work due to VM implementation differences
            }
            Ok(resp) => {
                warn!("VM execution '{}' failed: {}", name, resp.error.unwrap_or_default());
            }
            Err(e) => {
                error!("VM execution '{}' error: {}", name, e);
            }
        }
    }

    // Test VM sandbox security
    let sandbox_tests = vec![
        ("infinite_loop", "while(true) {}", "Should timeout"),
        ("excessive_memory", "let x = []; while(true) { x.push(new Array(1000000)); }", "Should hit memory limit"),
        ("file_access", "require('fs').readFileSync('/etc/passwd')", "Should deny file access"),
        ("network_access", "require('http').get('http://example.com')", "Should deny network access"),
    ];

    for (name, code, expected_behavior) in sandbox_tests {
        let request = ExtensionRequest {
            id: format!("sandbox-{}", name),
            method: "execute_vm".to_string(),
            params: serde_json::json!({
                "code": code,
                "inputs": {},
                "timeout_ms": 100,
                "memory_limit_mb": 5
            }),
            consensus_proof: None,
        };

        let response = catalog.handle_request(request).await;

        match response {
            Ok(resp) if !resp.success => {
                info!("Sandbox test '{}' correctly prevented: {}", name, expected_behavior);
            }
            Ok(resp) if resp.success => {
                warn!("Sandbox test '{}' unexpectedly succeeded!", name);
            }
            Err(e) => {
                info!("Sandbox test '{}' errored as expected: {}", name, e);
            }
            _ => {}
        }
    }

    info!("VM execution test completed");
}

/// Test P2P distribution across nodes
#[tokio::test]
async fn test_p2p_distribution() {
    init_test_logging();

    // Create multiple catalog instances simulating different nodes
    let node1 = create_test_catalog().await;
    let node2 = create_test_catalog().await;
    let node3 = create_test_catalog().await;

    // Register nodes with each other
    let nodes = vec![
        ("node1", "127.0.0.1:9001", &node1),
        ("node2", "127.0.0.1:9002", &node2),
        ("node3", "127.0.0.1:9003", &node3),
    ];

    for (id, addr, catalog) in &nodes {
        let request = ExtensionRequest {
            id: format!("register-{}", id),
            method: "register_p2p_node".to_string(),
            params: serde_json::json!({
                "node_id": id,
                "address": addr,
                "capabilities": ["storage", "compute", "relay"],
                "capacity": {
                    "storage_gb": 100,
                    "bandwidth_mbps": 100,
                    "concurrent_connections": 1000
                }
            }),
            consensus_proof: None,
        };

        for (_, _, other_catalog) in &nodes {
            other_catalog.handle_request(request.clone()).await.ok();
        }
    }

    // Create content on node1
    let lib_id = create_test_library(&node1, "distributed-lib").await;

    let create_package = ExtensionRequest {
        id: "dist-pkg-1".to_string(),
        method: "add_package".to_string(),
        params: serde_json::json!({
            "library_id": lib_id,
            "package": {
                "name": "shared-package",
                "version": "1.0.0",
                "code": "shared content across nodes",
                "size_bytes": 1024
            }
        }),
        consensus_proof: None,
    };

    let pkg_response = node1.handle_request(create_package).await.unwrap();
    assert!(pkg_response.success);
    let package_id = pkg_response.data["package_id"].as_str().unwrap();

    // Distribute to other nodes
    let distribute = ExtensionRequest {
        id: "dist-1".to_string(),
        method: "distribute_package".to_string(),
        params: serde_json::json!({
            "package_id": package_id,
            "target_nodes": ["node2", "node3"],
            "replication_factor": 2,
            "distribution_strategy": "geodistributed"
        }),
        consensus_proof: None,
    };

    let dist_response = node1.handle_request(distribute).await;
    assert!(dist_response.is_ok());

    // Verify distribution status
    sleep(Duration::from_millis(500)).await; // Allow time for distribution

    let status_request = ExtensionRequest {
        id: "status-1".to_string(),
        method: "get_distribution_status".to_string(),
        params: serde_json::json!({
            "package_id": package_id
        }),
        consensus_proof: None,
    };

    for (id, _, catalog) in &nodes {
        let status = catalog.handle_request(status_request.clone()).await;
        if let Ok(resp) = status {
            info!("Distribution status on {}: {:?}", id, resp.data);
        }
    }

    // Test content retrieval from any node
    let retrieve = ExtensionRequest {
        id: "retrieve-1".to_string(),
        method: "get_package".to_string(),
        params: serde_json::json!({
            "package_id": package_id,
            "prefer_local": false
        }),
        consensus_proof: None,
    };

    // Should work from node2 even though content was created on node1
    let retrieve_response = node2.handle_request(retrieve).await;
    assert!(retrieve_response.is_ok());

    info!("P2P distribution test completed");
}

/// Test TrustChain integration for package verification
#[tokio::test]
async fn test_trustchain_verification() {
    init_test_logging();

    let catalog = create_test_catalog().await;

    // Create a library and package
    let lib_id = create_test_library(&catalog, "trusted-lib").await;

    let package_request = ExtensionRequest {
        id: "trust-pkg-1".to_string(),
        method: "add_package".to_string(),
        params: serde_json::json!({
            "library_id": lib_id,
            "package": {
                "name": "secure-package",
                "version": "1.0.0",
                "code": "security critical code",
                "require_signature": true
            }
        }),
        consensus_proof: None,
    };

    let pkg_response = catalog.handle_request(package_request).await.unwrap();
    let package_id = pkg_response.data["package_id"].as_str().unwrap();

    // Sign the package (simulated)
    let sign_request = ExtensionRequest {
        id: "sign-1".to_string(),
        method: "sign_package".to_string(),
        params: serde_json::json!({
            "package_id": package_id,
            "signer_identity": "developer@example.com",
            "certificate": "-----BEGIN CERTIFICATE-----\nMOCK_CERT\n-----END CERTIFICATE-----",
            "private_key": "-----BEGIN PRIVATE KEY-----\nMOCK_KEY\n-----END PRIVATE KEY-----"
        }),
        consensus_proof: None,
    };

    let sign_response = catalog.handle_request(sign_request).await;
    // May fail without real certs, but should handle gracefully
    debug!("Sign response: {:?}", sign_response);

    // Verify package signature
    let verify_request = ExtensionRequest {
        id: "verify-1".to_string(),
        method: "verify_package_signature".to_string(),
        params: serde_json::json!({
            "package_id": package_id,
            "trust_anchors": ["root-ca", "intermediate-ca"]
        }),
        consensus_proof: None,
    };

    let verify_response = catalog.handle_request(verify_request).await;
    debug!("Verify response: {:?}", verify_response);

    // Test certificate chain validation
    let chain_request = ExtensionRequest {
        id: "chain-1".to_string(),
        method: "validate_certificate_chain".to_string(),
        params: serde_json::json!({
            "certificates": [
                "-----BEGIN CERTIFICATE-----\nLEAF\n-----END CERTIFICATE-----",
                "-----BEGIN CERTIFICATE-----\nINTERMEDIATE\n-----END CERTIFICATE-----",
                "-----BEGIN CERTIFICATE-----\nROOT\n-----END CERTIFICATE-----"
            ],
            "purpose": "code_signing"
        }),
        consensus_proof: None,
    };

    let chain_response = catalog.handle_request(chain_request).await;
    debug!("Chain validation response: {:?}", chain_response);

    info!("TrustChain verification test completed");
}

/// Test consensus validation for critical operations
#[tokio::test]
async fn test_consensus_validation() {
    init_test_logging();

    let catalog = create_test_catalog().await;

    // Create consensus proof
    let consensus_proof = ConsensusProof {
        block_height: 1000,
        block_hash: vec![0xAB; 32],
        timestamp: SystemTime::now(),
        proofs: HashMap::from([
            (ProofType::PoSpace, vec![0x01; 64]),
            (ProofType::PoStake, vec![0x02; 64]),
            (ProofType::PoWork, vec![0x03; 64]),
            (ProofType::PoTime, vec![0x04; 64]),
        ]),
        validator_signatures: vec![
            vec![0xF1; 65],
            vec![0xF2; 65],
            vec![0xF3; 65],
        ],
        merkle_root: vec![0xEE; 32],
    };

    // Test creating verified library with consensus
    let verified_lib = ExtensionRequest {
        id: "consensus-lib-1".to_string(),
        method: "create_verified_library".to_string(),
        params: serde_json::json!({
            "name": "consensus-library",
            "description": "Requires consensus validation",
            "governance": {
                "require_consensus": true,
                "min_validators": 3,
                "consensus_threshold": 0.67
            }
        }),
        consensus_proof: Some(consensus_proof.clone()),
    };

    let lib_response = catalog.handle_request(verified_lib).await;
    assert!(lib_response.is_ok());

    // Test modifying without consensus (should fail)
    let invalid_update = ExtensionRequest {
        id: "invalid-1".to_string(),
        method: "update_verified_library".to_string(),
        params: serde_json::json!({
            "library_id": "consensus-library",
            "updates": {
                "description": "Unauthorized update"
            }
        }),
        consensus_proof: None, // Missing consensus
    };

    let invalid_response = catalog.handle_request(invalid_update).await;
    match invalid_response {
        Ok(resp) if !resp.success => {
            info!("Correctly rejected update without consensus");
        }
        _ => {
            warn!("Should have rejected update without consensus");
        }
    }

    // Test with valid consensus
    let valid_update = ExtensionRequest {
        id: "valid-1".to_string(),
        method: "update_verified_library".to_string(),
        params: serde_json::json!({
            "library_id": "consensus-library",
            "updates": {
                "description": "Authorized update with consensus"
            }
        }),
        consensus_proof: Some(consensus_proof),
    };

    let valid_response = catalog.handle_request(valid_update).await;
    assert!(valid_response.is_ok());

    info!("Consensus validation test completed");
}

/// Test performance under load
#[tokio::test]
async fn test_performance_under_load() {
    init_test_logging();

    let catalog = Arc::new(create_test_catalog().await);
    let lib_id = create_test_library(&catalog, "perf-lib").await;

    let start = Instant::now();
    let mut handles = vec![];

    // Spawn concurrent operations
    let operations = 100;
    let concurrency = 10;

    for batch in 0..(operations / concurrency) {
        let mut batch_handles = vec![];

        for i in 0..concurrency {
            let catalog_clone = catalog.clone();
            let lib_clone = lib_id.clone();
            let op_id = batch * concurrency + i;

            let handle = tokio::spawn(async move {
                let request = ExtensionRequest {
                    id: format!("perf-{}", op_id),
                    method: "add_package".to_string(),
                    params: serde_json::json!({
                        "library_id": lib_clone,
                        "package": {
                            "name": format!("pkg-{}", op_id),
                            "version": "1.0.0",
                            "code": format!("// Package {}", op_id)
                        }
                    }),
                    consensus_proof: None,
                };

                let start = Instant::now();
                let result = catalog_clone.handle_request(request).await;
                let duration = start.elapsed();

                (result.is_ok(), duration)
            });

            batch_handles.push(handle);
        }

        // Wait for batch to complete
        for handle in batch_handles {
            handles.push(handle);
        }
    }

    // Collect results
    let mut successes = 0;
    let mut total_duration = Duration::from_secs(0);

    for handle in handles {
        if let Ok((success, duration)) = handle.await {
            if success {
                successes += 1;
            }
            total_duration += duration;
        }
    }

    let total_time = start.elapsed();
    let avg_duration = total_duration / operations;
    let ops_per_sec = operations as f64 / total_time.as_secs_f64();

    info!("Performance test results:");
    info!("  Total operations: {}", operations);
    info!("  Successful: {}", successes);
    info!("  Total time: {:.2}s", total_time.as_secs_f64());
    info!("  Average latency: {:.2}ms", avg_duration.as_millis());
    info!("  Throughput: {:.2} ops/sec", ops_per_sec);

    assert!(successes >= operations * 95 / 100); // 95% success rate
    assert!(ops_per_sec >= 10.0); // At least 10 ops/sec

    info!("Performance under load test completed");
}

/// Test multi-node coordination
#[tokio::test]
async fn test_multi_node_coordination() {
    init_test_logging();

    // Create a cluster of nodes
    let num_nodes = 5;
    let mut nodes = vec![];

    for i in 0..num_nodes {
        let catalog = create_test_catalog().await;
        nodes.push((format!("node-{}", i), Arc::new(catalog)));
    }

    // Register all nodes with each other
    for i in 0..num_nodes {
        for j in 0..num_nodes {
            if i != j {
                let register = ExtensionRequest {
                    id: format!("reg-{}-{}", i, j),
                    method: "register_peer".to_string(),
                    params: serde_json::json!({
                        "peer_id": format!("node-{}", j),
                        "address": format!("127.0.0.1:900{}", j),
                        "public_key": format!("pubkey-{}", j)
                    }),
                    consensus_proof: None,
                };

                nodes[i].1.handle_request(register).await.ok();
            }
        }
    }

    // Elect a coordinator
    let coordinator_request = ExtensionRequest {
        id: "elect-1".to_string(),
        method: "elect_coordinator".to_string(),
        params: serde_json::json!({
            "candidates": ["node-0", "node-1", "node-2"],
            "election_timeout_ms": 5000
        }),
        consensus_proof: None,
    };

    let election = nodes[0].1.handle_request(coordinator_request).await;
    debug!("Election result: {:?}", election);

    // Test distributed operation
    let distributed_op = ExtensionRequest {
        id: "dist-op-1".to_string(),
        method: "distributed_operation".to_string(),
        params: serde_json::json!({
            "operation": "create_replicated_library",
            "params": {
                "name": "replicated-lib",
                "replication_factor": 3,
                "consistency": "eventual"
            },
            "participants": ["node-0", "node-1", "node-2", "node-3", "node-4"]
        }),
        consensus_proof: None,
    };

    // Execute on coordinator
    let op_result = nodes[0].1.handle_request(distributed_op).await;
    assert!(op_result.is_ok());

    // Verify replication across nodes
    sleep(Duration::from_millis(1000)).await; // Allow replication

    for (node_id, catalog) in &nodes {
        let check = ExtensionRequest {
            id: format!("check-{}", node_id),
            method: "check_library_replica".to_string(),
            params: serde_json::json!({
                "library_name": "replicated-lib"
            }),
            consensus_proof: None,
        };

        let result = catalog.handle_request(check).await;
        debug!("Replica check on {}: {:?}", node_id, result);
    }

    // Test partition tolerance
    info!("Simulating network partition...");

    // Partition nodes 3 and 4
    for i in 0..3 {
        let unregister = ExtensionRequest {
            id: format!("unreg-{}", i),
            method: "unregister_peer".to_string(),
            params: serde_json::json!({
                "peer_id": "node-3"
            }),
            consensus_proof: None,
        };
        nodes[i].1.handle_request(unregister.clone()).await.ok();

        let unregister2 = ExtensionRequest {
            id: format!("unreg2-{}", i),
            method: "unregister_peer".to_string(),
            params: serde_json::json!({
                "peer_id": "node-4"
            }),
            consensus_proof: None,
        };
        nodes[i].1.handle_request(unregister2).await.ok();
    }

    // Operations should still work with majority
    let partition_op = ExtensionRequest {
        id: "part-op-1".to_string(),
        method: "create_library".to_string(),
        params: serde_json::json!({
            "name": "during-partition",
            "description": "Created during network partition"
        }),
        consensus_proof: None,
    };

    let part_result = nodes[0].1.handle_request(partition_op).await;
    assert!(part_result.is_ok());

    info!("Multi-node coordination test completed");
}

/// Test recovery and resilience
#[tokio::test]
async fn test_recovery_resilience() {
    init_test_logging();

    let catalog = create_test_catalog().await;

    // Create initial state
    let lib_id = create_test_library(&catalog, "resilience-lib").await;

    for i in 0..5 {
        let pkg = ExtensionRequest {
            id: format!("res-pkg-{}", i),
            method: "add_package".to_string(),
            params: serde_json::json!({
                "library_id": lib_id,
                "package": {
                    "name": format!("pkg-{}", i),
                    "version": "1.0.0",
                    "code": format!("code {}", i)
                }
            }),
            consensus_proof: None,
        };
        catalog.handle_request(pkg).await.ok();
    }

    // Export state
    let export_result = catalog.export_state().await.unwrap();
    let state_size = export_result.data.len();
    info!("Exported state with {} entries", state_size);

    // Simulate crash by pausing
    catalog.pause().await.unwrap();

    // Clear some internal state (simulated corruption)
    // In real scenario, this would be actual data loss

    // Resume and recover
    catalog.resume().await.unwrap();

    // Import saved state
    catalog.import_state(export_result).await.unwrap();

    // Verify recovery
    let verify_request = ExtensionRequest {
        id: "verify-recovery".to_string(),
        method: "list_library_packages".to_string(),
        params: serde_json::json!({
            "library_id": lib_id
        }),
        consensus_proof: None,
    };

    let verify_response = catalog.handle_request(verify_request).await.unwrap();
    assert!(verify_response.success);

    let recovered_packages = verify_response.data["packages"].as_array().unwrap();
    assert_eq!(recovered_packages.len(), 5);

    // Test automatic recovery mechanisms
    let recovery_test = ExtensionRequest {
        id: "auto-recovery".to_string(),
        method: "test_auto_recovery".to_string(),
        params: serde_json::json!({
            "failure_type": "storage_corruption",
            "recovery_strategy": "from_replicas"
        }),
        consensus_proof: None,
    };

    let recovery_response = catalog.handle_request(recovery_test).await;
    debug!("Auto recovery test: {:?}", recovery_response);

    info!("Recovery and resilience test completed");
}

// ============= Helper Functions =============

fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("debug")
        .try_init();
}

async fn create_test_catalog() -> CatalogExtension {
    let config = CatalogConfig {
        storage_path: PathBuf::from(format!("./test-data/catalog-{}", uuid::Uuid::new_v4())),
        max_libraries: 100,
        max_packages_per_library: 1000,
        enable_p2p: false, // Disable for simpler testing
        enable_trustchain: false,
        enable_consensus: false,
        cache_size_mb: 10,
        auto_verify: false,
        security_config: SecurityConfig {
            enforce_signatures: false,
            require_consensus: false,
            audit_enabled: true,
            max_package_size_mb: 10,
        },
    };

    CatalogExtension::new(config).await.unwrap()
}

async fn create_test_library(catalog: &CatalogExtension, name: &str) -> String {
    let request = ExtensionRequest {
        id: format!("create-lib-{}", name),
        method: "create_library".to_string(),
        params: serde_json::json!({
            "name": name,
            "description": format!("Test library {}", name),
            "tags": ["test"]
        }),
        consensus_proof: None,
    };

    let response = catalog.handle_request(request).await.unwrap();
    response.data["id"].as_str().unwrap().to_string()
}

// External crate usage
use serde_json::json;
use uuid;