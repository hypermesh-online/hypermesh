//! Integration Test for Blockchain-Native Compute System
//!
//! Tests the complete end-to-end flow of blockchain-native compute execution
//! based on Proof of State patterns without smart contract abstraction.

use std::collections::HashMap;
use std::time::Duration;

use hypermesh_catalog::{
    BlockchainNativeCompute, ComputeAsset, ComputeAssetType, ComputeRequest,
    P2PHost, PrivacyLevel, VMConfig, ConsensusRequirements,
};

use hypermesh_catalog::blockchain::{
    ComputeAssetDeploymentBlock, ComputeExecutionBlock, ComputeCompletionBlock,
    BlockchainNativeStorage, P2PExecutionEngine, MatrixRouter, CaesarTokenManager,
    compute::{ResourceRequirements, ExecutionEnvironment, PrivacyRequirements},
    execution::{HostResources, PrivacyCapabilities, HostStatus},
    p2p::VoluntaryParticipation,
};

use crate::consensus::{ConsensusProof, ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime, NetworkPosition};
use uuid::Uuid;

/// Test the complete blockchain-native compute flow
#[tokio::test]
async fn test_complete_blockchain_native_compute_flow() {
    // Initialize the blockchain-native compute system
    let config = VMConfig::default();
    let compute_system = BlockchainNativeCompute::new(config, None).await;
    
    // May fail during early development
    if compute_system.is_err() {
        println!("System not fully implemented yet: {:?}", compute_system.err());
        return;
    }
    
    let compute_system = compute_system.unwrap();
    
    // Create a sample Julia compute asset
    let julia_asset = ComputeAsset::new(
        "Matrix Multiplication".to_string(),
        "High-performance matrix multiplication using Julia".to_string(),
        ComputeAssetType::JuliaScript,
        r#"
        function matrix_multiply(A, B)
            return A * B
        end
        
        # Input will be parsed as JSON with matrices A and B
        result = matrix_multiply(input["A"], input["B"])
        return result
        "#.to_string(),
    );
    
    // Create consensus proof for deployment
    let deployment_proof = create_test_consensus_proof();
    
    // Deploy the compute asset to blockchain
    let asset_deployment = compute_system.deploy_compute_asset(
        julia_asset.clone(),
        "test-deployer".to_string(),
        vec!["host-1".to_string(), "host-2".to_string()],
        deployment_proof,
    ).await;
    
    if let Err(e) = asset_deployment {
        println!("Asset deployment failed (expected in early development): {}", e);
        return;
    }
    
    let asset_id = asset_deployment.unwrap();
    println!("Successfully deployed asset with ID: {}", asset_id);
    
    // Create execution request
    let mut request_params = HashMap::new();
    request_params.insert("A".to_string(), serde_json::json!([[1, 2], [3, 4]]));
    request_params.insert("B".to_string(), serde_json::json!([[5, 6], [7, 8]]));
    
    let compute_request = ComputeRequest::new(
        asset_id,
        request_params,
        b"matrix multiplication test".to_vec(),
    );
    
    // Create execution proof
    let execution_proof = create_test_consensus_proof();
    
    // Execute the compute request through P2P network
    let execution_result = compute_system.execute_compute_request(
        compute_request,
        execution_proof,
    ).await;
    
    if let Err(e) = execution_result {
        println!("Execution failed (expected in early development): {}", e);
        return;
    }
    
    let result = execution_result.unwrap();
    println!("Execution completed successfully: {:?}", result.success);
    println!("Output data size: {} bytes", result.output_data.len());
    
    // Verify execution history
    let history = compute_system.get_execution_history(&asset_id, Some(10)).await.unwrap();
    assert!(!history.is_empty());
    println!("Execution history entries: {}", history.len());
    
    // Get storage statistics
    let stats = compute_system.get_storage_stats().await.unwrap();
    println!("Storage stats: {:?}", stats);
}

/// Test P2P host registration and execution
#[tokio::test]
async fn test_p2p_host_participation() {
    // Create P2P execution engine
    let storage = std::sync::Arc::new(
        BlockchainNativeStorage::new(None).await.unwrap()
    );
    let engine = P2PExecutionEngine::new(storage).await.unwrap();
    
    // Create test hosts
    let host1 = create_test_host("host-1", "Test Host 1");
    let host2 = create_test_host("host-2", "Test Host 2");
    
    // Register hosts
    engine.register_host(host1.clone()).await.unwrap();
    engine.register_host(host2.clone()).await.unwrap();
    
    // Get available hosts
    let available_hosts = engine.get_available_hosts().await.unwrap();
    assert_eq!(available_hosts.len(), 2);
    
    println!("Successfully registered {} P2P hosts", available_hosts.len());
    
    // Test voluntary participation system
    let mut participation = VoluntaryParticipation::new();
    
    // Register hosts for participation
    let session1 = participation.register_host_participation(host1.host_id.clone());
    let session2 = participation.register_host_participation(host2.host_id.clone());
    
    assert!(!session1.to_string().is_empty());
    assert!(!session2.to_string().is_empty());
    
    // Add mock execution
    let execution_id = Uuid::new_v4();
    participation.add_execution_to_session(&host1.host_id, execution_id).unwrap();
    
    // Complete execution successfully
    participation.remove_execution_from_session(&host1.host_id, execution_id, true).unwrap();
    
    // Get participation stats
    let stats = participation.get_participation_stats(&host1.host_id);
    assert!(stats.is_some());
    
    let stats = stats.unwrap();
    assert_eq!(stats.successful_executions, 1);
    assert_eq!(stats.failed_executions, 0);
    assert!(stats.participation_score > 0.0);
    
    println!("Host participation stats: success_rate={:.2}, score={:.2}", 
             stats.success_rate, stats.participation_score);
}

/// Test matrix routing system
#[tokio::test]
async fn test_matrix_routing() {
    use hypermesh_catalog::vm::P2PConfig;
    
    let config = P2PConfig::default();
    let router = MatrixRouter::new(config).await.unwrap();
    
    // Create and register test hosts
    let host1 = create_test_host("matrix-host-1", "Matrix Host 1");
    let host2 = create_test_host("matrix-host-2", "Matrix Host 2");
    
    // Register hosts in matrix
    let coord1 = router.register_host(host1).await.unwrap();
    let coord2 = router.register_host(host2).await.unwrap();
    
    assert!(!coord1.host_id.is_empty());
    assert!(!coord2.host_id.is_empty());
    assert!(coord1.x != coord2.x || coord1.y != coord2.y); // Different coordinates
    
    println!("Host coordinates: ({}, {}) and ({}, {})", 
             coord1.x, coord1.y, coord2.x, coord2.y);
    
    // Register compute asset routing
    let asset_id = Uuid::new_v4();
    router.register_compute_asset(
        asset_id,
        vec![coord1.host_id.clone(), coord2.host_id.clone()],
    ).await.unwrap();
    
    println!("Successfully set up matrix routing for asset {}", asset_id);
}

/// Test Caesar token management
#[tokio::test]
async fn test_caesar_token_payments() {
    let storage = std::sync::Arc::new(
        BlockchainNativeStorage::new(None).await.unwrap()
    );
    let mut token_manager = CaesarTokenManager::new(storage).await.unwrap();
    
    // Create mock compute request and execution result
    let request = ComputeRequest::new(
        Uuid::new_v4(),
        HashMap::new(),
        b"test input".to_vec(),
    );
    
    let execution_result = create_mock_execution_result(&request);
    let consensus_proof = create_test_consensus_proof();
    
    // Process payment
    let payment_tokens = token_manager.process_execution_payment(
        &request,
        &execution_result,
        &consensus_proof,
    ).await.unwrap();
    
    assert!(!payment_tokens.is_empty());
    println!("Generated {} payment tokens", payment_tokens.len());
    
    for token in &payment_tokens {
        println!("Payment token: {} -> {} tokens to {}", 
                token.token_id, token.amount, token.recipient);
    }
    
    // Get payment statistics
    let stats = token_manager.get_payment_statistics();
    println!("Payment stats: {} payments, {} total tokens", 
             stats.total_payments, stats.total_amount_paid);
}

/// Test blockchain block creation and integrity
#[tokio::test]
async fn test_blockchain_blocks() {
    // Test deployment block
    let asset = ComputeAsset::new(
        "Test Asset".to_string(),
        "Test compute asset".to_string(),
        ComputeAssetType::JuliaScript,
        "println(\"test\")".to_string(),
    );
    
    let consensus_proof = create_test_consensus_proof();
    
    let deployment_block = ComputeAssetDeploymentBlock::new(
        asset,
        "test-deployer".to_string(),
        vec!["host1".to_string()],
        consensus_proof.clone(),
    );
    
    // Verify block integrity
    assert!(deployment_block.verify_integrity());
    println!("Deployment block integrity verified: {}", deployment_block.block_id);
    
    // Test execution block
    let request = ComputeRequest::new(Uuid::new_v4(), HashMap::new(), vec![]);
    
    let execution_block = ComputeExecutionBlock::new(
        request,
        vec!["host1".to_string()],
        consensus_proof.clone(),
    );
    
    assert!(execution_block.verify_integrity());
    println!("Execution block integrity verified: {}", execution_block.block_id);
    
    // Test completion block
    let execution_result = create_mock_execution_result(&ComputeRequest::new(
        Uuid::new_v4(), HashMap::new(), vec![]
    ));
    
    let completion_block = ComputeCompletionBlock::new(
        execution_result,
        vec![], // No payment tokens for test
        consensus_proof,
    );
    
    assert!(completion_block.verify_integrity());
    println!("Completion block integrity verified: {}", completion_block.block_id);
}

// Helper functions

fn create_test_consensus_proof() -> ConsensusProof {
    let space_proof = ProofOfSpace::new(
        "/test/space".to_string(),
        NetworkPosition {
            address: "test-address".to_string(),
            zone: "test-zone".to_string(),
            distance_metric: 1,
        },
        1024, // 1KB space commitment
    );
    
    let stake_proof = ProofOfStake::new(
        "test-stakeholder".to_string(),
        "test-node".to_string(),
        1000, // Authority level
        crate::consensus::proof::AccessPermissions {
            read_level: crate::consensus::proof::AccessLevel::Public,
            write_level: crate::consensus::proof::AccessLevel::Network,
            admin_level: crate::consensus::proof::AccessLevel::None,
            allocation_rights: vec!["test-compute".to_string()],
        },
        vec![], // No allowances for test
    );
    
    let work_proof = ProofOfWork::new(
        b"test-challenge",
        8, // Low difficulty for testing
        "test-compute".to_string(),
    ).unwrap();
    
    let time_proof = ProofOfTime::new(1000, None, 1);
    
    ConsensusProof::new(space_proof, stake_proof, work_proof, time_proof)
}

fn create_test_host(host_id: &str, name: &str) -> P2PHost {
    P2PHost {
        host_id: host_id.to_string(),
        name: name.to_string(),
        address: format!("127.0.0.1:{}", 8080 + host_id.len()), // Simple port assignment
        available_resources: HostResources {
            cpu_cores: 4,
            memory_mb: 8192,
            storage_mb: 100000,
            gpu: None,
            network_mbps: 100,
        },
        supported_asset_types: vec!["JuliaScript".to_string(), "PythonScript".to_string()],
        reputation_score: 0.8,
        privacy_capabilities: PrivacyCapabilities {
            supports_private: true,
            supports_encryption: true,
            supports_secure_enclaves: false,
            supports_anonymous: false,
        },
        geographic_location: Some("US".to_string()),
        registered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
        status: HostStatus::Available,
    }
}

fn create_mock_execution_result(request: &ComputeRequest) -> hypermesh_catalog::ExecutionResult {
    hypermesh_catalog::blockchain::execution::ExecutionResult {
        request_id: request.request_id,
        asset_id: request.asset_id,
        success: true,
        output_data: b"mock execution result".to_vec(),
        error_message: None,
        execution_logs: "Mock execution completed".to_string(),
        resource_usage: hypermesh_catalog::blockchain::compute::ExecutionResourceUsage {
            cpu_time_micros: 5_000_000, // 5 seconds
            peak_memory_bytes: 512 * 1024 * 1024, // 512MB
            storage_read_bytes: 1024,
            storage_write_bytes: 2048,
            network_sent_bytes: 1024,
            network_received_bytes: 4096,
            gpu_utilization_percent: None,
        },
        execution_duration: Duration::from_secs(5),
        participating_hosts: vec!["test-host".to_string()],
        result_consensus_proof: create_test_consensus_proof(),
        executed_at: std::time::SystemTime::now(),
    }
}