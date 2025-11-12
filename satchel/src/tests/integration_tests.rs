//! Integration tests for Hardware Asset Adapters
//! 
//! Tests all adapters with the new ConsensusProof system and ensures
//! proper integration with the four-proof validation (PoSp+PoSt+PoWk+PoTm).

use std::time::{Duration, SystemTime};
use std::collections::HashMap;

use hypermesh_assets::core::{
    AssetType, AssetAllocationRequest, PrivacyLevel, ResourceRequirements,
    CpuRequirements, GpuRequirements, MemoryRequirements, StorageRequirements, StorageType,
};
use hypermesh_assets::core::consensus::proof::{
    ConsensusProof, ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime,
    NetworkPosition, AccessPermissions, AccessLevel,
};
use hypermesh_assets::adapters::{
    AdapterRegistry, CpuAssetAdapter, GpuAssetAdapter, MemoryAssetAdapter, StorageAssetAdapter,
};

/// Create a valid ConsensusProof for testing
fn create_test_consensus_proof(resource_type: &str, stake_amount: u64, difficulty: u32, committed_space: u64) -> ConsensusProof {
    ConsensusProof::new(
        ProofOfSpace::new(
            format!("/test/{}", resource_type),
            NetworkPosition {
                address: "2001:db8::1".to_string(),
                zone: "us-west-1".to_string(),
                distance_metric: 100,
            },
            committed_space,
        ),
        ProofOfStake::new(
            "test-node".to_string(),
            stake_amount,
            AccessPermissions {
                read_level: AccessLevel::Public,
                write_level: AccessLevel::Network,
                admin_level: AccessLevel::None,
                allocation_rights: vec![resource_type.to_string()],
            },
        ),
        ProofOfWork::new(
            format!("{}-challenge", resource_type).as_bytes(),
            difficulty,
            resource_type.to_string(),
        ).unwrap(),
        ProofOfTime::new(1000, None, 1),
    )
}

/// Create CPU allocation request
fn create_cpu_allocation_request() -> AssetAllocationRequest {
    AssetAllocationRequest {
        asset_type: AssetType::Cpu,
        requested_resources: ResourceRequirements {
            cpu: Some(CpuRequirements {
                cores: 2,
                min_frequency_mhz: Some(2400),
                architecture: Some("x86_64".to_string()),
                required_features: vec!["AVX2".to_string()],
            }),
            ..Default::default()
        },
        privacy_level: PrivacyLevel::Private,
        consensus_proof: create_test_consensus_proof("cpu", 50, 16, 2), // CPU requirements
        certificate_fingerprint: "test-cert".to_string(),
        duration_limit: None,
        tags: HashMap::new(),
    }
}

/// Create GPU allocation request
fn create_gpu_allocation_request() -> AssetAllocationRequest {
    AssetAllocationRequest {
        asset_type: AssetType::Gpu,
        requested_resources: ResourceRequirements {
            gpu: Some(GpuRequirements {
                units: 1,
                min_memory_mb: Some(8192), // 8GB
                compute_capability: Some("8.0".to_string()),
                required_features: vec!["Nova".to_string()],
            }),
            ..Default::default()
        },
        privacy_level: PrivacyLevel::Private,
        consensus_proof: create_test_consensus_proof("gpu", 200, 20, 8 * 1024 * 1024 * 1024), // GPU requirements
        certificate_fingerprint: "test-cert".to_string(),
        duration_limit: None,
        tags: HashMap::new(),
    }
}

/// Create Memory allocation request
fn create_memory_allocation_request() -> AssetAllocationRequest {
    AssetAllocationRequest {
        asset_type: AssetType::Memory,
        requested_resources: ResourceRequirements {
            memory: Some(MemoryRequirements {
                size_bytes: 1024 * 1024 * 1024, // 1GB
                memory_type: Some("DDR4".to_string()),
                ecc_required: false,
                numa_node: None,
            }),
            ..Default::default()
        },
        privacy_level: PrivacyLevel::Private,
        consensus_proof: create_test_consensus_proof("memory", 100, 12, 1024 * 1024 * 1024), // Memory requirements
        certificate_fingerprint: "test-cert".to_string(),
        duration_limit: None,
        tags: HashMap::new(),
    }
}

/// Create Storage allocation request
fn create_storage_allocation_request() -> AssetAllocationRequest {
    AssetAllocationRequest {
        asset_type: AssetType::Storage,
        requested_resources: ResourceRequirements {
            storage: Some(StorageRequirements {
                size_bytes: 10 * 1024 * 1024 * 1024, // 10GB
                storage_type: StorageType::Nvme,
                min_iops: Some(100000),
                min_bandwidth_mbps: Some(1000),
                durability_replicas: 3,
            }),
            ..Default::default()
        },
        privacy_level: PrivacyLevel::Private,
        consensus_proof: create_test_consensus_proof("storage", 75, 14, 10 * 1024 * 1024 * 1024), // Storage requirements
        certificate_fingerprint: "test-cert".to_string(),
        duration_limit: None,
        tags: HashMap::new(),
    }
}

#[tokio::test]
async fn test_adapter_registry_integration() {
    let registry = AdapterRegistry::new().await;
    
    // Test that all adapters are created successfully
    assert!(registry.get_adapter(&AssetType::Cpu).is_some());
    assert!(registry.get_adapter(&AssetType::Gpu).is_some());
    assert!(registry.get_adapter(&AssetType::Memory).is_some());
    assert!(registry.get_adapter(&AssetType::Storage).is_some());
    
    let adapters = registry.get_all_adapters();
    assert_eq!(adapters.len(), 6); // All 6 asset types
}

#[tokio::test]
async fn test_cpu_adapter_consensus_validation() {
    let adapter = CpuAssetAdapter::new().await;
    let request = create_cpu_allocation_request();
    
    // Test consensus proof validation
    let valid = adapter.validate_consensus_proof(&request.consensus_proof).await.unwrap();
    assert!(valid, "CPU consensus proof validation should pass");
    
    // Test allocation
    let allocation = adapter.allocate_asset(&request).await;
    assert!(allocation.is_ok(), "CPU allocation should succeed with valid consensus proof");
    
    if let Ok(allocation) = allocation {
        // Test deallocation
        let result = adapter.deallocate_asset(&allocation.asset_id).await;
        assert!(result.is_ok(), "CPU deallocation should succeed");
    }
}

#[tokio::test]
async fn test_gpu_adapter_consensus_validation() {
    let adapter = GpuAssetAdapter::new().await;
    let request = create_gpu_allocation_request();
    
    // Test consensus proof validation
    let valid = adapter.validate_consensus_proof(&request.consensus_proof).await.unwrap();
    assert!(valid, "GPU consensus proof validation should pass");
    
    // Test allocation
    let allocation = adapter.allocate_asset(&request).await;
    assert!(allocation.is_ok(), "GPU allocation should succeed with valid consensus proof");
    
    if let Ok(allocation) = allocation {
        // Test deallocation
        let result = adapter.deallocate_asset(&allocation.asset_id).await;
        assert!(result.is_ok(), "GPU deallocation should succeed");
    }
}

#[tokio::test]
async fn test_memory_adapter_nat_addressing() {
    let adapter = MemoryAssetAdapter::new().await;
    let request = create_memory_allocation_request();
    
    // Test consensus proof validation
    let valid = adapter.validate_consensus_proof(&request.consensus_proof).await.unwrap();
    assert!(valid, "Memory consensus proof validation should pass");
    
    // Test allocation with NAT-like addressing
    let allocation = adapter.allocate_asset(&request).await;
    assert!(allocation.is_ok(), "Memory allocation should succeed with valid consensus proof");
    
    if let Ok(allocation) = allocation {
        // Test proxy address assignment (NAT-like system)
        let proxy_addr = adapter.assign_proxy_address(&allocation.asset_id).await;
        assert!(proxy_addr.is_ok(), "Memory proxy address assignment should succeed");
        
        if let Ok(proxy_addr) = proxy_addr {
            // Test proxy address resolution
            let resolved_id = adapter.resolve_proxy_address(&proxy_addr).await;
            assert!(resolved_id.is_ok(), "Memory proxy address resolution should succeed");
            assert_eq!(resolved_id.unwrap(), allocation.asset_id);
        }
        
        // Test deallocation
        let result = adapter.deallocate_asset(&allocation.asset_id).await;
        assert!(result.is_ok(), "Memory deallocation should succeed");
    }
}

#[tokio::test]
async fn test_storage_adapter_pos_validation() {
    let adapter = StorageAssetAdapter::new().await;
    let request = create_storage_allocation_request();
    
    // Test consensus proof validation (critical PoSpace validation for storage)
    let valid = adapter.validate_consensus_proof(&request.consensus_proof).await.unwrap();
    assert!(valid, "Storage consensus proof validation should pass with proper PoSpace");
    
    // Test allocation with sharding
    let allocation = adapter.allocate_asset(&request).await;
    assert!(allocation.is_ok(), "Storage allocation should succeed with valid PoSpace proof");
    
    if let Ok(allocation) = allocation {
        // Verify allocation metadata includes sharding information
        assert!(allocation.status.metadata.contains_key("shard_count"));
        assert!(allocation.status.metadata.contains_key("redundancy_level"));
        assert!(allocation.status.metadata.contains_key("encryption_enabled"));
        
        // Test deallocation
        let result = adapter.deallocate_asset(&allocation.asset_id).await;
        assert!(result.is_ok(), "Storage deallocation should succeed");
    }
}

#[tokio::test]
async fn test_consensus_proof_validation_failures() {
    let adapter = CpuAssetAdapter::new().await;
    
    // Test with invalid PoSpace (0 committed space)
    let invalid_space_proof = ConsensusProof::new(
        ProofOfSpace::new(
            "/test/cpu".to_string(),
            NetworkPosition {
                address: "2001:db8::1".to_string(),
                zone: "us-west-1".to_string(),
                distance_metric: 100,
            },
            0, // Invalid: 0 committed space
        ),
        ProofOfStake::new(
            "test-node".to_string(),
            100,
            AccessPermissions {
                read_level: AccessLevel::Public,
                write_level: AccessLevel::Network,
                admin_level: AccessLevel::None,
                allocation_rights: vec!["cpu".to_string()],
            },
        ),
        ProofOfWork::new(b"cpu-challenge", 16, "cpu".to_string()).unwrap(),
        ProofOfTime::new(1000, None, 1),
    );
    
    let valid = adapter.validate_consensus_proof(&invalid_space_proof).await.unwrap();
    assert!(!valid, "Consensus proof with 0 committed space should fail");
    
    // Test with insufficient stake
    let invalid_stake_proof = ConsensusProof::new(
        ProofOfSpace::new(
            "/test/cpu".to_string(),
            NetworkPosition {
                address: "2001:db8::1".to_string(),
                zone: "us-west-1".to_string(),
                distance_metric: 100,
            },
            2,
        ),
        ProofOfStake::new(
            "test-node".to_string(),
            10, // Invalid: insufficient stake for CPU (needs 50+)
            AccessPermissions {
                read_level: AccessLevel::Public,
                write_level: AccessLevel::Network,
                admin_level: AccessLevel::None,
                allocation_rights: vec!["cpu".to_string()],
            },
        ),
        ProofOfWork::new(b"cpu-challenge", 16, "cpu".to_string()).unwrap(),
        ProofOfTime::new(1000, None, 1),
    );
    
    let valid = adapter.validate_consensus_proof(&invalid_stake_proof).await.unwrap();
    assert!(!valid, "Consensus proof with insufficient stake should fail");
    
    // Test with insufficient work difficulty
    let invalid_work_proof = ConsensusProof::new(
        ProofOfSpace::new(
            "/test/cpu".to_string(),
            NetworkPosition {
                address: "2001:db8::1".to_string(),
                zone: "us-west-1".to_string(),
                distance_metric: 100,
            },
            2,
        ),
        ProofOfStake::new(
            "test-node".to_string(),
            100,
            AccessPermissions {
                read_level: AccessLevel::Public,
                write_level: AccessLevel::Network,
                admin_level: AccessLevel::None,
                allocation_rights: vec!["cpu".to_string()],
            },
        ),
        ProofOfWork::new(b"cpu-challenge", 8, "cpu".to_string()).unwrap(), // Invalid: insufficient difficulty (needs 16+)
        ProofOfTime::new(1000, None, 1),
    );
    
    let valid = adapter.validate_consensus_proof(&invalid_work_proof).await.unwrap();
    assert!(!valid, "Consensus proof with insufficient work difficulty should fail");
}

#[tokio::test]
async fn test_adapter_health_checks() {
    let registry = AdapterRegistry::new().await;
    let adapters = registry.get_all_adapters();
    
    for (asset_type, adapter) in adapters {
        let health = adapter.health_check().await;
        assert!(health.is_ok(), "Health check should succeed for {:?}", asset_type);
        
        if let Ok(health) = health {
            assert!(!health.performance_metrics.is_empty(), "Health check should include performance metrics for {:?}", asset_type);
        }
    }
}

#[tokio::test]
async fn test_adapter_capabilities() {
    let registry = AdapterRegistry::new().await;
    let adapters = registry.get_all_adapters();
    
    for (asset_type, adapter) in adapters {
        let capabilities = adapter.get_capabilities();
        assert_eq!(capabilities.asset_type, asset_type);
        assert!(capabilities.supports_proxy_addressing, "All adapters should support proxy addressing");
        assert!(capabilities.supports_resource_monitoring, "All adapters should support resource monitoring");
        assert!(!capabilities.features.is_empty(), "All adapters should have feature list");
        
        // Check privacy level support
        assert!(capabilities.supported_privacy_levels.contains(&PrivacyLevel::Private));
        assert!(capabilities.supported_privacy_levels.contains(&PrivacyLevel::FullPublic));
    }
}

#[tokio::test]
async fn test_privacy_level_configuration() {
    let adapter = MemoryAssetAdapter::new().await;
    let request = create_memory_allocation_request();
    
    let allocation = adapter.allocate_asset(&request).await.unwrap();
    
    // Test privacy level changes
    for privacy_level in vec![PrivacyLevel::Private, PrivacyLevel::P2P, PrivacyLevel::FullPublic] {
        let result = adapter.configure_privacy_level(&allocation.asset_id, privacy_level.clone()).await;
        assert!(result.is_ok(), "Privacy level configuration should succeed for {:?}", privacy_level);
    }
    
    adapter.deallocate_asset(&allocation.asset_id).await.unwrap();
}

#[tokio::test]
async fn test_resource_usage_monitoring() {
    let adapter = CpuAssetAdapter::new().await;
    let request = create_cpu_allocation_request();
    
    let allocation = adapter.allocate_asset(&request).await.unwrap();
    
    // Test resource usage monitoring
    let usage = adapter.get_resource_usage(&allocation.asset_id).await;
    assert!(usage.is_ok(), "Resource usage monitoring should work");
    
    if let Ok(usage) = usage {
        assert!(usage.cpu_usage.is_some(), "CPU usage should be available for CPU assets");
        assert!(usage.measurement_timestamp <= SystemTime::now());
    }
    
    adapter.deallocate_asset(&allocation.asset_id).await.unwrap();
}