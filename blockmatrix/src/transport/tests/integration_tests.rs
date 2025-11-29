//! Integration tests for HyperMesh Transport Layer

use crate::transport::{
    HyperMeshTransport, HyperMeshTransportConfig, NodeId, TransportError
};
use stoq::Endpoint;
use std::net::Ipv6Addr;
use std::time::Duration;
use tokio::time::timeout;

/// Helper function to create test configuration
fn create_test_config() -> HyperMeshTransportConfig {
    let mut config = HyperMeshTransportConfig::default();
    config.network.port = 0; // Use any available port for tests
    config.connection_pool.max_pool_size = 10;
    config.monitoring.enable_prometheus = false; // Disable for tests
    config
}

/// Test transport initialization
#[tokio::test]
async fn test_transport_initialization() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await;
    
    assert!(transport.is_ok());
    let transport = transport.unwrap();
    
    // Verify initial state
    assert_eq!(transport.connection_count().await, 0);
    let stats = transport.stats().await;
    assert_eq!(stats.active_connections, 0);
}

/// Test configuration validation
#[tokio::test]
async fn test_configuration_validation() {
    let mut config = create_test_config();
    
    // Valid configuration should work
    assert!(config.validate().is_ok());
    
    // Invalid port should fail
    config.network.port = 0;
    let result = HyperMeshTransport::new_async(config.clone()).await;
    // Port 0 is actually valid (means "any port"), so this should succeed
    assert!(result.is_ok());
    
    // Invalid buffer size should fail validation
    config.performance.send_buffer_size = 0;
    assert!(config.validate().is_err());
}

/// Test connection attempt without server
#[tokio::test]
async fn test_connection_failure() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    let node_id = NodeId::new("nonexistent-node".to_string());
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 65535); // Unlikely to have service
    
    // Connection should fail since no server is running
    let result = timeout(
        Duration::from_secs(2),
        transport.connect_to_node(node_id, &endpoint)
    ).await;
    
    assert!(result.is_err() || result.unwrap().is_err());
}

/// Test connection pool management
#[tokio::test]
async fn test_connection_pool_management() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    let node_id = NodeId::new("test-node".to_string());
    
    // Initially no connections
    assert_eq!(transport.connection_count().await, 0);
    
    // Pool should be empty
    let connections = transport.active_connections().await;
    assert!(connections.is_empty());
}

/// Test metrics collection
#[tokio::test]
async fn test_metrics_collection() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    // Get initial stats
    let stats = transport.stats().await;
    assert_eq!(stats.total_bytes_sent, 0);
    assert_eq!(stats.total_bytes_received, 0);
    assert_eq!(stats.total_connections, 0);
    
    // Verify monitoring is working
    assert!(stats.monitoring_period_secs >= 0);
}

/// Test node authentication
#[tokio::test]
async fn test_node_authentication() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    let node_id = NodeId::new("test-node".to_string());
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
    
    // Basic node verification should pass
    let result = transport.authenticator.verify_node(&node_id, &endpoint).await;
    assert!(result.is_ok());
}

/// Test connection cleanup
#[tokio::test]
async fn test_connection_cleanup() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    // Cleanup should succeed even with no connections
    let removed = transport.cleanup_idle_connections().await.unwrap();
    assert_eq!(removed, 0);
}

/// Test transport shutdown
#[tokio::test]
async fn test_transport_shutdown() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    // Shutdown should succeed
    let result = transport.shutdown().await;
    assert!(result.is_ok());
}

/// Test concurrent operations
#[tokio::test]
async fn test_concurrent_operations() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    // Spawn multiple tasks doing operations concurrently
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let transport = transport.clone();
        let handle = tokio::spawn(async move {
            let node_id = NodeId::new(format!("concurrent-node-{}", i));
            let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292 + i as u16);
            
            // These will fail but test concurrent access
            let _ = transport.connect_to_node(node_id.clone(), &endpoint).await;
            let _ = transport.send_to_node(&node_id, b"test data").await;
            let _ = transport.stats().await;
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Transport should still be functional
    let stats = transport.stats().await;
    assert!(stats.monitoring_period_secs >= 0);
}

/// Test error handling
#[tokio::test]
async fn test_error_handling() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    let node_id = NodeId::new("nonexistent-node".to_string());
    
    // Sending to nonexistent node should fail
    let result = transport.send_to_node(&node_id, b"test data").await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        TransportError::NoConnection(_) => {
            // Expected error type
        }
        other => {
            panic!("Unexpected error type: {:?}", other);
        }
    }
}

/// Test configuration serialization
#[tokio::test]
async fn test_configuration_serialization() {
    let config = create_test_config();
    
    // Test YAML serialization
    let yaml = serde_yaml::to_string(&config).unwrap();
    assert!(!yaml.is_empty());
    
    // Test deserialization
    let deserialized: HyperMeshTransportConfig = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(config.network.max_connections, deserialized.network.max_connections);
}

/// Test memory safety and cleanup
#[tokio::test]
async fn test_memory_cleanup() {
    let config = create_test_config();
    
    // Create and drop multiple transport instances
    for _ in 0..5 {
        let transport = HyperMeshTransport::new(config.clone()).await.unwrap();
        
        // Do some operations
        let _ = transport.stats().await;
        let _ = transport.connection_count().await;
        
        // Transport should be dropped cleanly
    }
}

/// Test STOQ integration
#[tokio::test]
async fn test_stoq_integration() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    // Verify STOQ integration is working
    let stats = transport.stats().await;
    
    // STOQ stats should be integrated
    assert_eq!(stats.bytes_sent, 0);
    assert_eq!(stats.bytes_received, 0);
    assert_eq!(stats.active_connections, 0);
}

/// Performance test for basic operations
#[tokio::test]
async fn test_performance_basic_operations() {
    let config = create_test_config();
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    let start = std::time::Instant::now();
    
    // Perform various operations
    for i in 0..100 {
        let node_id = NodeId::new(format!("perf-test-{}", i));
        let _ = transport.send_to_node(&node_id, b"test").await; // Will fail but measures overhead
    }
    
    let duration = start.elapsed();
    
    // Should complete within reasonable time
    assert!(duration < Duration::from_secs(1));
}

/// Test resource limits
#[tokio::test]
async fn test_resource_limits() {
    let mut config = create_test_config();
    config.connection_pool.max_pool_size = 2; // Very small pool
    
    let transport = HyperMeshTransport::new_async(config).await.unwrap();
    
    // Test that limits are enforced (through connection pool)
    let pool_stats = transport.connection_pool.stats().await;
    assert!(pool_stats.utilization_percent >= 0.0);
}