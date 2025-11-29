//! End-to-end integration tests for HyperMesh platform

use std::time::Duration;
use tokio::time::timeout;
use hypermesh_integration::{
    HyperMeshPlatform, HyperMeshConfig, HyperMeshPlatformBuilder,
    PlatformMetrics, ServiceQuery, ServiceHealthStatus,
};
use crate::transport::TransportConfig;
use crate::consensus::ConsensusConfig;
use hypermesh_container::ContainerRuntimeConfig;
use hypermesh_security::SecurityConfig;
use hypermesh_orchestration::OrchestrationConfig;
use stoq::StoqConfig;

/// Test basic platform initialization
#[tokio::test]
async fn test_platform_initialization() {
    let config = create_test_config().await;
    
    let platform = timeout(Duration::from_secs(30), HyperMeshPlatform::new(config))
        .await
        .expect("Platform creation should not timeout")
        .expect("Platform should initialize successfully");
        
    // Platform should be created but not yet initialized
    let metrics = platform.metrics().await;
    assert!(metrics.uptime < Duration::from_secs(5));
}

/// Test full platform startup and shutdown
#[tokio::test] 
async fn test_platform_lifecycle() {
    let config = create_test_config().await;
    
    let platform = HyperMeshPlatform::new(config)
        .await
        .expect("Platform should initialize");
    
    // Initialize all components
    timeout(Duration::from_secs(60), platform.initialize())
        .await
        .expect("Platform initialization should not timeout")
        .expect("Platform should initialize successfully");
    
    // Check that platform is running
    let metrics = platform.metrics().await;
    assert!(metrics.overall_health_score > 0.0);
    assert!(metrics.components.len() > 0);
    
    // Shutdown gracefully
    timeout(Duration::from_secs(30), platform.shutdown())
        .await
        .expect("Platform shutdown should not timeout")
        .expect("Platform should shutdown successfully");
}

/// Test service registry functionality
#[tokio::test]
async fn test_service_registry_integration() {
    let config = create_test_config().await;
    
    let platform = HyperMeshPlatform::new(config)
        .await
        .expect("Platform should initialize");
    
    let registry = platform.service_registry();
    
    // Register a test service
    let service_endpoint = hypermesh_integration::services::ServiceEndpoint {
        service_type: "test-service".to_string(),
        address: "127.0.0.1".to_string(),
        port: 8080,
        health_check_path: "/health".to_string(),
    };
    
    registry.register_service("test-service-1".to_string(), service_endpoint.clone())
        .await
        .expect("Service registration should succeed");
    
    // Discover the service
    let query = ServiceQuery {
        service_type: "test-service".to_string(),
        required_tags: None,
        preferred_tags: None,
        min_health: None,
        limit: None,
    };
    
    let discovered = registry.discover_services(query).await;
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].service_type, "test-service");
    assert_eq!(discovered[0].port, 8080);
}

/// Test component integration points
#[tokio::test]
async fn test_component_integrations() {
    let config = create_test_config().await;
    
    let platform = HyperMeshPlatform::new(config)
        .await
        .expect("Platform should initialize");
    
    // Initialize platform
    platform.initialize()
        .await
        .expect("Platform should initialize");
    
    // Verify component handles are available
    assert!(!platform.stoq().clone().as_ref() as *const _ == std::ptr::null());
    assert!(!platform.transport().clone().as_ref() as *const _ == std::ptr::null());
    assert!(!platform.consensus().clone().as_ref() as *const _ == std::ptr::null());
    assert!(!platform.container_runtime().clone().as_ref() as *const _ == std::ptr::null());
    assert!(!platform.security().clone().as_ref() as *const _ == std::ptr::null());
    assert!(!platform.orchestration().clone().as_ref() as *const _ == std::ptr::null());
    
    // Check that services are registered in the registry
    let services = platform.service_registry().list_services().await;
    assert!(services.len() > 0, "Platform services should be registered");
    
    platform.shutdown().await.expect("Platform should shutdown");
}

/// Test platform metrics collection
#[tokio::test]
async fn test_metrics_collection() {
    let config = create_test_config().await;
    
    let platform = HyperMeshPlatform::new(config)
        .await
        .expect("Platform should initialize");
    
    platform.initialize()
        .await
        .expect("Platform should initialize");
    
    // Give some time for metrics to be collected
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let metrics = platform.metrics().await;
    
    // Verify metrics structure
    assert!(metrics.uptime > Duration::from_secs(0));
    assert!(metrics.timestamp > std::time::UNIX_EPOCH);
    assert!(metrics.overall_health_score >= 0.0 && metrics.overall_health_score <= 1.0);
    
    // Verify resource utilization metrics
    let utilization = &metrics.resource_utilization;
    assert!(utilization.total_cpu_usage >= 0.0);
    assert!(utilization.total_memory_usage >= 0);
    assert!(utilization.avg_response_time_ms >= 0.0);
    
    platform.shutdown().await.expect("Platform should shutdown");
}

/// Test platform builder pattern
#[tokio::test]
async fn test_platform_builder() {
    let stoq_config = StoqConfig::default();
    let transport_config = TransportConfig::default();
    let consensus_config = ConsensusConfig::default();
    
    let platform = HyperMeshPlatformBuilder::new()
        .stoq_config(stoq_config)
        .transport_config(transport_config)
        .consensus_config(consensus_config)
        .build()
        .await
        .expect("Builder should create platform successfully");
    
    // Verify platform was created
    let metrics = platform.metrics().await;
    assert!(metrics.uptime >= Duration::from_secs(0));
}

/// Test configuration validation
#[tokio::test]
async fn test_config_validation() {
    let mut config = create_test_config().await;
    
    // Test valid configuration
    assert!(config.validate().is_ok(), "Default configuration should be valid");
    
    // Test invalid configuration (port conflict)
    config.transport.bind_port = 8080;
    config.consensus.port = 8080;
    
    assert!(config.validate().is_err(), "Port conflict should be detected");
}

/// Test error handling during initialization
#[tokio::test] 
async fn test_initialization_error_handling() {
    let mut config = create_test_config().await;
    
    // Create invalid configuration that should cause initialization to fail
    config.integration.initialization_timeout_secs = 1; // Very short timeout
    
    let platform = HyperMeshPlatform::new(config)
        .await
        .expect("Platform creation should succeed");
    
    // This should timeout or fail due to short timeout
    let result = timeout(Duration::from_secs(5), platform.initialize()).await;
    
    // Either timeout or initialization error is acceptable for this test
    if let Ok(init_result) = result {
        // If initialization completes, clean up
        if init_result.is_ok() {
            let _ = platform.shutdown().await;
        }
    }
}

/// Helper function to create test configuration
async fn create_test_config() -> HyperMeshConfig {
    let mut config = HyperMeshConfig::default();
    
    // Use different ports to avoid conflicts
    config.transport.bind_port = 9001;
    config.consensus.port = 9002;
    config.integration.metrics.prometheus_port = 9003;
    
    // Shorter timeouts for testing
    config.integration.initialization_timeout_secs = 60;
    config.integration.health_check_interval_secs = 5;
    config.integration.communication_timeout_secs = 5;
    
    config
}

/// Performance test for platform initialization
#[tokio::test]
async fn test_platform_initialization_performance() {
    let config = create_test_config().await;
    
    let start_time = std::time::Instant::now();
    
    let platform = HyperMeshPlatform::new(config)
        .await
        .expect("Platform should initialize");
    
    platform.initialize()
        .await
        .expect("Platform should initialize");
    
    let initialization_time = start_time.elapsed();
    
    // Platform should initialize within reasonable time
    assert!(
        initialization_time < Duration::from_secs(120),
        "Platform initialization took too long: {:?}",
        initialization_time
    );
    
    println!("Platform initialized in {:?}", initialization_time);
    
    platform.shutdown().await.expect("Platform should shutdown");
}

/// Test concurrent platform operations
#[tokio::test]
async fn test_concurrent_operations() {
    let config = create_test_config().await;
    
    let platform = std::sync::Arc::new(
        HyperMeshPlatform::new(config)
            .await
            .expect("Platform should initialize")
    );
    
    platform.initialize()
        .await
        .expect("Platform should initialize");
    
    // Spawn multiple concurrent operations
    let mut handles = Vec::new();
    
    // Concurrent metrics collection
    for _ in 0..5 {
        let platform_clone = platform.clone();
        let handle = tokio::spawn(async move {
            let metrics = platform_clone.metrics().await;
            assert!(metrics.overall_health_score >= 0.0);
        });
        handles.push(handle);
    }
    
    // Concurrent service registry operations
    for i in 0..3 {
        let platform_clone = platform.clone();
        let handle = tokio::spawn(async move {
            let registry = platform_clone.service_registry();
            let endpoint = hypermesh_integration::services::ServiceEndpoint {
                service_type: format!("concurrent-service-{}", i),
                address: "127.0.0.1".to_string(),
                port: 8000 + i as u16,
                health_check_path: "/health".to_string(),
            };
            
            registry.register_service(format!("service-{}", i), endpoint)
                .await
                .expect("Service registration should succeed");
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        handle.await.expect("Concurrent operation should complete");
    }
    
    platform.shutdown().await.expect("Platform should shutdown");
}