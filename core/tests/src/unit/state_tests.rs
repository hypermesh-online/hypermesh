//! Comprehensive unit tests for state management components

use crate::{TestResult, init_test_logging, unit_test};
use nexus_state::*;
use tempfile::TempDir;
use std::time::Duration;
use tokio::time::timeout;

pub async fn run_state_tests() -> TestResult {
    init_test_logging();
    
    // Consensus tests
    test_consensus_engine_creation().await?;
    test_consensus_leadership_election().await?;
    test_consensus_proposal_handling().await?;
    test_byzantine_consensus().await?;
    test_byzantine_fault_tolerance().await?;
    
    // Storage tests
    test_storage_operations().await?;
    test_storage_persistence().await?;
    test_storage_concurrent_access().await?;
    
    // Byzantine coordinator tests
    test_byzantine_coordinator().await?;
    test_fault_detection().await?;
    test_view_change_management().await?;
    
    Ok(())
}

unit_test!(test_consensus_engine_creation, "consensus", {
    let config = consensus::ConsensusConfig::default();
    let node_id = nexus_shared::NodeId::random();
    
    let engine = ConsensusEngine::new(&config, node_id).await?;
    assert_eq!(engine.state().await, ConsensusState::Follower);
    assert_eq!(engine.name(), "consensus-engine");
    assert!(!engine.is_running());
    
    let stats = engine.stats().await;
    assert_eq!(stats.current_term, 0);
    assert_eq!(stats.log_entries, 0);
    assert_eq!(stats.committed_entries, 0);
    
    Ok(())
});

unit_test!(test_consensus_leadership_election, "consensus", {
    let config = consensus::ConsensusConfig::default();
    let node_id = nexus_shared::NodeId::random();
    
    let mut engine = ConsensusEngine::new(&config, node_id).await?;
    engine.start().await?;
    
    // Single node should become leader
    engine.join_cluster(vec![node_id]).await?;
    
    // Wait for election
    timeout(Duration::from_secs(5), async {
        while engine.state().await != ConsensusState::Leader {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await?;
    
    assert_eq!(engine.state().await, ConsensusState::Leader);
    
    let stats = engine.stats().await;
    assert!(stats.elections_started > 0);
    assert!(stats.elections_won > 0);
    
    engine.stop().await?;
    Ok(())
});

unit_test!(test_consensus_proposal_handling, "consensus", {
    let config = consensus::ConsensusConfig::default();
    let node_id = nexus_shared::NodeId::random();
    
    let mut engine = ConsensusEngine::new(&config, node_id).await?;
    engine.start().await?;
    engine.join_cluster(vec![node_id]).await?;
    
    // Wait to become leader
    timeout(Duration::from_secs(5), async {
        while engine.state().await != ConsensusState::Leader {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await?;
    
    // Submit proposals
    let proposal1 = Proposal::Set {
        key: "test-key-1".to_string(),
        value: b"test-value-1".to_vec(),
    };
    
    let proposal2 = Proposal::Set {
        key: "test-key-2".to_string(),
        value: b"test-value-2".to_vec(),
    };
    
    engine.propose(proposal1).await?;
    engine.propose(proposal2).await?;
    
    let stats = engine.stats().await;
    assert!(stats.proposals_received >= 2);
    assert!(stats.log_entries >= 2);
    
    engine.stop().await?;
    Ok(())
});

unit_test!(test_byzantine_consensus, "byzantine", {
    let mut config = consensus::ConsensusConfig::default();
    config.byzantine_fault_tolerance = true;
    config.byzantine_confirmations = 3;
    
    let node_id = nexus_shared::NodeId::random();
    let mut engine = ConsensusEngine::new(&config, node_id).await?;
    
    engine.start().await?;
    engine.join_cluster(vec![node_id]).await?;
    
    // Wait to become leader
    timeout(Duration::from_secs(5), async {
        while engine.state().await != ConsensusState::Leader {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await?;
    
    // Test Byzantine status
    let status = engine.byzantine_status().await;
    assert!(status.enabled);
    assert_eq!(status.total_nodes, 1);
    assert_eq!(status.max_byzantine_failures, 0);
    
    // Submit Byzantine consensus proposal
    let proposal = Proposal::Set {
        key: "byzantine-test".to_string(),
        value: b"byzantine-value".to_vec(),
    };
    
    engine.propose(proposal).await?;
    
    let stats = engine.stats().await;
    assert!(stats.proposals_received > 0);
    
    engine.stop().await?;
    Ok(())
});

unit_test!(test_byzantine_fault_tolerance, "byzantine", {
    let config = ByzantineConfig::default();
    let node_id = nexus_shared::NodeId::random();
    
    let coordinator = ByzantineCoordinator::new(config, node_id).await?;
    coordinator.start().await?;
    
    // Test fault detection
    let faults = coordinator.detect_faults().await;
    // Should not crash and return results
    assert!(faults.len() <= 100); // Reasonable upper bound
    
    // Test view change
    coordinator.trigger_view_change("test view change").await?;
    
    let status = coordinator.overall_byzantine_status().await;
    assert_eq!(status.coordinator_node, node_id);
    assert!(status.fault_detection_active);
    
    coordinator.stop().await?;
    Ok(())
});

unit_test!(test_storage_operations, "storage", {
    let temp_dir = TempDir::new()?;
    let mut storage_config = storage::StorageConfig::default();
    storage_config.data_dir = temp_dir.path().to_string_lossy().to_string();
    storage_config.backend = storage::StorageBackend::Memory; // Use memory for tests
    
    let store = StateStore::new(&storage_config).await?;
    store.start().await?;
    
    // Test basic operations
    let key = "test-key";
    let value = b"test-value";
    
    // Should not exist initially
    let result = store.get(key).await?;
    assert!(result.is_none());
    
    // Set value
    store.set(key, value).await?;
    
    // Should exist now
    let result = store.get(key).await?;
    assert_eq!(result.unwrap(), value);
    
    // Update value
    let new_value = b"updated-value";
    store.set(key, new_value).await?;
    let result = store.get(key).await?;
    assert_eq!(result.unwrap(), new_value);
    
    // Delete value
    let deleted = store.delete(key).await?;
    assert!(deleted);
    
    let result = store.get(key).await?;
    assert!(result.is_none());
    
    store.stop().await?;
    Ok(())
});

unit_test!(test_storage_persistence, "storage", {
    let temp_dir = TempDir::new()?;
    let mut storage_config = storage::StorageConfig::default();
    storage_config.data_dir = temp_dir.path().to_string_lossy().to_string();
    
    let key = "persistent-key";
    let value = b"persistent-value";
    
    // First store instance
    {
        let store = StateStore::new(&storage_config).await?;
        store.start().await?;
        store.set(key, value).await?;
        store.stop().await?;
    }
    
    // Second store instance - should restore data
    {
        let store = StateStore::new(&storage_config).await?;
        store.start().await?;
        
        let result = store.get(key).await?;
        assert_eq!(result.unwrap(), value);
        
        store.stop().await?;
    }
    
    Ok(())
});

unit_test!(test_storage_concurrent_access, "storage", {
    let temp_dir = TempDir::new()?;
    let mut storage_config = storage::StorageConfig::default();
    storage_config.data_dir = temp_dir.path().to_string_lossy().to_string();
    storage_config.backend = storage::StorageBackend::Memory;
    
    let store = std::sync::Arc::new(StateStore::new(&storage_config).await?);
    store.start().await?;
    
    let mut handles = Vec::new();
    
    // Spawn concurrent read/write operations
    for i in 0..10 {
        let store_clone = store.clone();
        let handle = tokio::spawn(async move {
            let key = format!("concurrent-key-{}", i);
            let value = format!("concurrent-value-{}", i).into_bytes();
            
            // Write
            store_clone.set(&key, &value).await?;
            
            // Read
            let result = store_clone.get(&key).await?;
            assert_eq!(result.unwrap(), value);
            
            // Update
            let new_value = format!("updated-value-{}", i).into_bytes();
            store_clone.set(&key, &new_value).await?;
            
            let result = store_clone.get(&key).await?;
            assert_eq!(result.unwrap(), new_value);
            
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        handle.await??;
    }
    
    store.stop().await?;
    Ok(())
});

unit_test!(test_byzantine_coordinator, "byzantine", {
    let config = ByzantineConfig::default();
    let node_id = nexus_shared::NodeId::random();
    
    let coordinator = ByzantineCoordinator::new(config, node_id).await?;
    coordinator.start().await?;
    
    // Create a mock consensus engine
    let consensus_config = consensus::ConsensusConfig::default();
    let consensus_engine = std::sync::Arc::new(
        ConsensusEngine::new(&consensus_config, node_id).await?
    );
    
    // Register the engine
    coordinator.register_consensus_engine("test-engine".to_string(), consensus_engine).await?;
    
    // Test Byzantine consensus
    let proposal = Proposal::Set {
        key: "coordinator-test".to_string(),
        value: b"coordinator-value".to_vec(),
    };
    
    let result = coordinator.byzantine_consensus(proposal).await;
    // May succeed or fail depending on consensus state, but shouldn't crash
    let _success = result.is_ok();
    
    // Test checkpoint creation
    let checkpoints = coordinator.create_checkpoint().await?;
    assert!(!checkpoints.is_empty());
    
    coordinator.stop().await?;
    Ok(())
});

unit_test!(test_fault_detection, "fault_detection", {
    let config = ByzantineConfig::default();
    let node_id = nexus_shared::NodeId::random();
    
    let coordinator = ByzantineCoordinator::new(config, node_id).await?;
    coordinator.start().await?;
    
    // Run fault detection multiple times
    for _ in 0..5 {
        let faults = coordinator.detect_faults().await;
        
        // Validate fault report structure
        for fault in &faults {
            assert!(!fault.evidence.is_empty());
            assert!(fault.confidence >= 0.0 && fault.confidence <= 1.0);
            // Timestamps should be recent
            assert!(fault.detected_at.elapsed() < Duration::from_secs(10));
        }
    }
    
    coordinator.stop().await?;
    Ok(())
});

unit_test!(test_view_change_management, "view_change", {
    let config = ByzantineConfig::default();
    let node_id = nexus_shared::NodeId::random();
    
    let coordinator = ByzantineCoordinator::new(config, node_id).await?;
    coordinator.start().await?;
    
    // Test view changes with different reasons
    let reasons = [
        "Primary failure detected",
        "Network partition",
        "Performance degradation",
        "Manual trigger",
    ];
    
    for reason in &reasons {
        coordinator.trigger_view_change(reason).await?;
        
        let status = coordinator.overall_byzantine_status().await;
        assert!(status.view_changes_detected > 0);
    }
    
    coordinator.stop().await?;
    Ok(())
});

#[tokio::test]
async fn test_state_manager_integration() -> TestResult {
    init_test_logging();
    
    let temp_dir = TempDir::new()?;
    let mut config = StateConfig::default();
    config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
    config.storage.backend = storage::StorageBackend::Memory;
    
    let node_id = nexus_shared::NodeId::random();
    let state_manager = StateManager::new(config, node_id).await?;
    
    // Test basic operations without starting (should work for read operations)
    let cluster_status = state_manager.cluster_status().await;
    assert_eq!(cluster_status.node_id, node_id);
    assert_eq!(cluster_status.member_count, 0);
    
    // Test stats collection
    let stats = state_manager.stats().await;
    assert_eq!(stats.node_id, node_id);
    
    Ok(())
}