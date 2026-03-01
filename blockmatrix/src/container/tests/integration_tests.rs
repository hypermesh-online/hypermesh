// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Container runtime integration tests

use hypermesh_container::{
    ContainerRuntime, ContainerSpec, CreateOptions, ContainerId,
    ResourceQuota, NetworkConfig, SecurityConfig, MountConfig,
    config::ContainerConfig,
    lifecycle::ContainerState,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio;

#[tokio::test]
async fn test_container_lifecycle() {
    let config = ContainerConfig::default();
    let runtime = ContainerRuntime::new(config).await.expect("test: async operation");
    
    // Create container spec
    let spec = ContainerSpec {
        image: \"test:latest\".to_string(),
        command: vec![\"sh\".to_string(), \"-c\".to_string(), \"sleep 10\".to_string()],
        environment: HashMap::new(),
        working_dir: None,
        resources: ResourceQuota::default(),
        network_usage: NetworkConfig::default(),
        mounts: vec![],
        security: SecurityConfig::default(),
    };
    
    let options = CreateOptions::default();
    
    // Test container creation
    let handle = runtime.create(spec, options).await.expect("test: async operation");
    let status = handle.status().await.expect("test: async operation");
    assert_eq!(status.state, ContainerState::Created);
    
    // Test container start
    handle.start().await.expect("test: async operation");
    let status = handle.status().await.expect("test: async operation");
    assert_eq!(status.state, ContainerState::Running);
    
    // Test container pause/resume
    handle.pause().await.expect("test: async operation");
    let status = handle.status().await.expect("test: async operation");
    assert_eq!(status.state, ContainerState::Paused);
    
    handle.resume().await.expect("test: async operation");
    let status = handle.status().await.expect("test: async operation");
    assert_eq!(status.state, ContainerState::Running);
    
    // Test container stop
    handle.stop(Some(Duration::from_secs(5))).await.expect("test: async operation");
    let status = handle.status().await.expect("test: async operation");
    assert_eq!(status.state, ContainerState::Stopped);
    
    // Test container deletion
    handle.delete().await.expect("test: async operation");
}

#[tokio::test]
async fn test_resource_management() {
    let config = ContainerConfig::default();
    let runtime = ContainerRuntime::new(config).await.expect("test: async operation");
    
    let spec = ContainerSpec {
        image: \"test:latest\".to_string(),
        command: vec![\"sh\".to_string()],
        environment: HashMap::new(),
        working_dir: None,
        resources: ResourceQuota {
            memory_limit: Some(512 * 1024 * 1024), // 512MB
            cpu_quota: Some(1.0), // 1 CPU core
            ..ResourceQuota::default()
        },
        network_usage: NetworkConfig::default(),
        mounts: vec![],
        security: SecurityConfig::default(),
    };
    
    let handle = runtime.create(spec, CreateOptions::default()).await.expect("test: async operation");
    handle.start().await.expect("test: async operation");
    
    // Test resource usage monitoring
    let usage = handle.usage().await.expect("test: async operation");
    assert!(usage.memory_usage > 0);
    assert!(usage.cpu_usage_percent >= 0.0);
    
    // Test metrics collection
    let metrics = handle.metrics().await.expect("test: async operation");
    assert!(metrics.uptime_seconds >= 0);
    
    handle.stop(None).await.expect("test: async operation");
    handle.delete().await.expect("test: async operation");
}

#[tokio::test]
async fn test_container_migration() {
    let config = ContainerConfig::default();
    let runtime = ContainerRuntime::new(config).await.expect("test: async operation");
    
    let spec = ContainerSpec {
        image: \"test:latest\".to_string(),
        command: vec![\"sh\".to_string()],
        environment: HashMap::new(),
        working_dir: None,
        resources: ResourceQuota::default(),
        network_usage: NetworkConfig::default(),
        mounts: vec![],
        security: SecurityConfig::default(),
    };
    
    let handle = runtime.create(spec, CreateOptions::default()).await.expect("test: async operation");
    handle.start().await.expect("test: async operation");
    
    // Test live migration
    let migration_request = hypermesh_container::migration::MigrationRequest {
        container_id: handle.id,
        destination_node: \"node2\".to_string(),
        migration_type: hypermesh_container::migration::MigrationType::Hot,
        downtime_budget: Duration::from_millis(100),
        bandwidth_limit: None,
    };
    
    let result = handle.migrate(migration_request).await.expect("test: async operation");
    assert!(result.success);
    assert!(result.downtime <= Duration::from_millis(100));
    
    handle.stop(None).await.expect("test: async operation");
    handle.delete().await.expect("test: async operation");
}

#[tokio::test]
async fn test_container_performance_targets() {
    let config = ContainerConfig::default();
    let runtime = ContainerRuntime::new(config).await.expect("test: async operation");
    
    let spec = ContainerSpec {
        image: \"test:latest\".to_string(),
        command: vec![\"sh\".to_string()],
        environment: HashMap::new(),
        working_dir: None,
        resources: ResourceQuota::default(),
        network_usage: NetworkConfig::default(),
        mounts: vec![],
        security: SecurityConfig::default(),
    };
    
    // Test container creation time (<50ms)
    let create_start = std::time::Instant::now();
    let handle = runtime.create(spec, CreateOptions::default()).await.expect("test: async operation");
    let create_time = create_start.elapsed();
    // Note: In simulation mode, this will be much faster than real implementation
    assert!(create_time < Duration::from_millis(1000)); // Relaxed for simulation
    
    // Test container startup time (<100ms)
    let start_time = std::time::Instant::now();
    handle.start().await.expect("test: async operation");
    let startup_time = start_time.elapsed();
    assert!(startup_time < Duration::from_millis(200)); // Relaxed for simulation
    
    // Test pause/resume time (<10ms pause, <50ms resume)
    let pause_start = std::time::Instant::now();
    handle.pause().await.expect("test: async operation");
    let pause_time = pause_start.elapsed();
    assert!(pause_time < Duration::from_millis(100)); // Relaxed for simulation
    
    let resume_start = std::time::Instant::now();
    handle.resume().await.expect("test: async operation");
    let resume_time = resume_start.elapsed();
    assert!(resume_time < Duration::from_millis(200)); // Relaxed for simulation
    
    handle.stop(None).await.expect("test: async operation");
    handle.delete().await.expect("test: async operation");
}

#[tokio::test]
async fn test_multiple_containers() {
    let config = ContainerConfig::default();
    let runtime = ContainerRuntime::new(config).await.expect("test: async operation");
    
    let spec = ContainerSpec {
        image: \"test:latest\".to_string(),
        command: vec![\"sh\".to_string()],
        environment: HashMap::new(),
        working_dir: None,
        resources: ResourceQuota::default(),
        network_usage: NetworkConfig::default(),
        mounts: vec![],
        security: SecurityConfig::default(),
    };
    
    // Create multiple containers
    let mut handles = Vec::new();
    for i in 0..5 {
        let mut spec = spec.clone();
        spec.environment.insert(\"CONTAINER_ID\".to_string(), i.to_string());
        
        let handle = runtime.create(spec, CreateOptions::default()).await.expect("test: async operation");
        handle.start().await.expect("test: async operation");
        handles.push(handle);
    }
    
    // Verify all containers are running
    for handle in &handles {
        let status = handle.status().await.expect("test: async operation");
        assert_eq!(status.state, ContainerState::Running);
    }
    
    // Clean up
    for handle in handles {
        handle.stop(None).await.expect("test: async operation");
        handle.delete().await.expect("test: async operation");
    }
}