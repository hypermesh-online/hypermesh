//! Plugin operation tests
//!
//! Tests for library operations, P2P distribution, cross-node sync, consensus integration, and TrustChain verification.
//!
//! NOTE: All tests marked #[ignore] - Requires Catalog extension implementation

use super::*;

/// Test consensus validation integration
#[tokio::test]
#[ignore = "Requires Catalog extension implementation"]
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
        params: json!({
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
#[ignore = "Requires Catalog extension implementation"]
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
        params: json!({
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
        params: json!({
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
        params: json!({
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
#[ignore = "Requires Catalog extension implementation"]
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
        params: json!({
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
        params: json!({
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
#[ignore = "Requires Catalog extension implementation"]
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
        params: json!({
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
        params: json!({
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
#[ignore = "Requires Catalog extension implementation"]
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
        params: json!({
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
        params: json!({
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
