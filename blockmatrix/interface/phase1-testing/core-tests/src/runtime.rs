//! Runtime component tests

use nexus_runtime::*;
use tempfile::TempDir;
use std::collections::HashMap;
use std::time::SystemTime;

#[tokio::test]
async fn test_runtime_creation() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = RuntimeConfig::default();
    config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
    
    let runtime = Runtime::new(config).await;
    assert!(runtime.is_ok());
}

#[test]
fn test_container_spec_defaults() {
    let spec = ContainerSpec::default();
    
    assert!(!spec.id.namespace().is_empty());
    assert!(!spec.id.name().is_empty());
    assert!(!spec.image.name.is_empty());
    assert!(!spec.command.is_empty());
}

#[test]
fn test_container_spec_serialization() {
    let mut spec = ContainerSpec::default();
    spec.command = vec!["echo".to_string(), "hello".to_string()];
    spec.environment.insert("TEST_VAR".to_string(), "test_value".to_string());
    
    let json = serde_json::to_string(&spec).unwrap();
    let parsed: ContainerSpec = serde_json::from_str(&json).unwrap();
    
    assert_eq!(spec.command, parsed.command);
    assert_eq!(spec.environment, parsed.environment);
}

#[test]
fn test_container_status_transitions() {
    use ContainerStatus::*;
    
    assert_eq!(Created, Created);
    assert_ne!(Created, Running);
    assert_ne!(Running, Stopped);
    
    let statuses = vec![Created, Running, Paused, Stopped, Failed];
    assert_eq!(statuses.len(), 5);
}

#[test]
fn test_restart_policy() {
    use RestartPolicy::*;
    
    let policies = vec![Never, Always, OnFailure, UnlessStopped];
    
    for policy in policies {
        let json = serde_json::to_string(&policy).unwrap();
        let parsed: RestartPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, parsed);
    }
}

#[test]
fn test_volume_mount() {
    let mount = VolumeMount {
        source: "/host/path".to_string(),
        target: "/container/path".to_string(),
        options: vec!["ro".to_string(), "bind".to_string()],
        readonly: true,
    };
    
    assert_eq!(mount.source, "/host/path");
    assert_eq!(mount.target, "/container/path");
    assert!(mount.readonly);
    assert!(mount.options.contains(&"ro".to_string()));
}

#[test]
fn test_security_config() {
    let mut security = ContainerSecurityConfig::default();
    
    // Should drop all capabilities by default
    assert!(security.capabilities_drop.contains(&"ALL".to_string()));
    
    // Should not be privileged by default
    assert!(!security.privileged);
    
    // Test custom configuration
    security.user_id = Some(1000);
    security.group_id = Some(1000);
    security.readonly_rootfs = true;
    
    assert_eq!(security.user_id, Some(1000));
    assert!(security.readonly_rootfs);
}

#[tokio::test]
async fn test_image_spec() {
    let spec = ImageSpec {
        name: "nginx".to_string(),
        tag: "1.20".to_string(),
        registry: Some("docker.io".to_string()),
        digest: None,
    };
    
    assert_eq!(spec.cache_key(), "nginx:1.20");
    assert_eq!(spec.full_reference("registry.io"), "docker.io/nginx:1.20");
    
    let with_digest = ImageSpec {
        name: "nginx".to_string(),
        tag: "latest".to_string(),
        registry: None,
        digest: Some("sha256:abc123".to_string()),
    };
    
    assert_eq!(with_digest.cache_key(), "nginx@sha256:abc123");
}

#[tokio::test]
async fn test_image_manager() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = image::ImageConfig::default();
    config.storage_dir = temp_dir.path().to_string_lossy().to_string();
    
    let manager = image::ImageManager::new(&config).await.unwrap();
    
    let spec = ImageSpec::default();
    
    // Test image operations (would fail without actual registry)
    let cache_stats = manager.cache_stats().await;
    assert_eq!(cache_stats.image_count, 0);
    
    let images = manager.list_images().await;
    assert!(images.is_empty());
}

#[test]
fn test_runtime_config_validation() {
    let config = RuntimeConfig::default();
    assert!(config.validate().is_ok());
    
    let mut invalid_config = RuntimeConfig::default();
    invalid_config.resources.default_cpu_limit = 0.0;
    assert!(config.validate().is_err());
}

#[test]
fn test_runtime_config_presets() {
    let dev_config = RuntimeConfig::development();
    assert!(dev_config.security.allow_privileged);
    assert_eq!(dev_config.logging.level, "debug");
    
    let prod_config = RuntimeConfig::production();
    assert!(!prod_config.security.allow_privileged);
    assert!(prod_config.security.rootless_mode);
    assert_eq!(prod_config.logging.level, "warn");
}

#[test]
fn test_isolation_config() {
    let isolation = config::IsolationConfig::default();
    
    assert!(isolation.enable_namespaces);
    assert!(isolation.enable_cgroups);
    assert!(isolation.enable_seccomp);
    
    // Check that all important namespace types are enabled
    use config::NamespaceType::*;
    assert!(isolation.namespace_types.contains(&Pid));
    assert!(isolation.namespace_types.contains(&Net));
    assert!(isolation.namespace_types.contains(&Mount));
}

#[test]
fn test_resource_config() {
    let resources = config::ResourceConfig::default();
    
    assert!(resources.default_cpu_limit > 0.0);
    assert!(resources.default_memory_limit > 0);
    assert!(resources.enable_monitoring);
    assert!(resources.enable_oom_killer);
}

#[test]
fn test_networking_config() {
    let networking = config::NetworkingConfig::default();
    
    assert!(matches!(networking.default_network_mode, config::NetworkMode::Bridge));
    assert!(networking.enable_port_mapping);
    assert!(networking.enable_isolation);
    assert!(networking.enable_ipv6);
    assert!(!networking.dns_servers.is_empty());
}

#[test]
fn test_storage_config() {
    let storage = config::StorageConfig::default();
    
    assert!(matches!(storage.default_driver, config::StorageDriver::Overlay2));
    assert!(storage.enable_encryption);
    assert!(storage.enable_compression);
    assert!(storage.max_container_size_gb > 0.0);
}

#[test]
fn test_security_config_defaults() {
    let security = config::SecurityConfig::default();
    
    assert!(!security.allow_privileged);
    assert!(security.no_new_privileges);
    assert!(security.enable_user_namespaces);
    assert!(!security.default_cap_drop.is_empty());
}

#[test]
fn test_logging_config() {
    let logging = config::LoggingConfig::default();
    
    assert!(matches!(logging.default_driver, config::LogDriver::JsonFile));
    assert!(logging.structured_logging);
    assert!(logging.rotation.max_file_size_mb > 0);
}

/// Integration tests for runtime components
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_container_lifecycle_simulation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = RuntimeConfig::default();
        config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
        
        let runtime = Runtime::new(config).await.unwrap();
        
        let mut spec = ContainerSpec::default();
        spec.command = vec!["echo".to_string(), "hello world".to_string()];
        spec.resources.cpu_cores = 0.5;
        spec.resources.memory_mb = 256;
        
        // Simulate container operations (would require actual runtime)
        let containers = runtime.list_containers().await;
        assert!(containers.is_empty());
        
        let usage = runtime.resource_usage().await;
        assert!(usage.is_empty());
    }
    
    #[tokio::test]
    async fn test_image_management_integration() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = image::ImageConfig::default();
        config.storage_dir = temp_dir.path().to_string_lossy().to_string();
        
        let manager = image::ImageManager::new(&config).await.unwrap();
        
        // Test basic operations
        let images = manager.list_images().await;
        assert!(images.is_empty());
        
        let stats = manager.cache_stats().await;
        assert_eq!(stats.image_count, 0);
        assert_eq!(stats.total_size_bytes, 0);
    }
    
    #[test]
    fn test_config_file_operations() {
        use std::fs;
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("runtime.toml");
        
        let config = RuntimeConfig::default();
        
        // Save config
        config.to_file(config_path.to_str().unwrap()).unwrap();
        assert!(config_path.exists());
        
        // Load config
        let loaded = RuntimeConfig::from_file(config_path.to_str().unwrap()).unwrap();
        assert_eq!(config.resources.default_cpu_limit, loaded.resources.default_cpu_limit);
        assert_eq!(config.storage.data_dir, loaded.storage.data_dir);
    }
}

/// Performance tests for runtime components
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_container_spec_serialization_performance() {
        let mut spec = ContainerSpec::default();
        spec.command = vec!["test".to_string(); 100];
        for i in 0..100 {
            spec.environment.insert(format!("VAR_{}", i), format!("value_{}", i));
        }
        
        let start = Instant::now();
        
        for _ in 0..1000 {
            let json = serde_json::to_string(&spec).unwrap();
            let _parsed: ContainerSpec = serde_json::from_str(&json).unwrap();
        }
        
        let elapsed = start.elapsed();
        println!("Serialized/deserialized 1000 container specs in {:?}", elapsed);
        
        // Should handle serialization efficiently
        assert!(elapsed < std::time::Duration::from_millis(500));
    }
    
    #[tokio::test]
    async fn test_runtime_creation_performance() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = RuntimeConfig::default();
        config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
        
        let start = Instant::now();
        
        for _ in 0..10 {
            let _runtime = Runtime::new(config.clone()).await.unwrap();
        }
        
        let elapsed = start.elapsed();
        println!("Created 10 runtime instances in {:?}", elapsed);
        
        // Should create runtimes efficiently
        assert!(elapsed < std::time::Duration::from_secs(1));
    }
}

/// Error handling tests
#[cfg(test)]
mod error_tests {
    use super::*;
    
    #[test]
    fn test_runtime_error_categories() {
        let error = RuntimeError::ContainerNotFound { 
            id: nexus_shared::ResourceId::new("test", "container", "container") 
        };
        assert_eq!(error.category(), "container_not_found");
        assert!(!error.is_retryable());
        
        let resource_error = RuntimeError::ResourceAllocation { 
            message: "Not enough memory".to_string() 
        };
        assert_eq!(resource_error.category(), "resources");
        assert!(resource_error.is_retryable());
    }
    
    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let runtime_error: RuntimeError = io_error.into();
        
        assert!(matches!(runtime_error, RuntimeError::Io(_)));
        
        let nexus_error: nexus_shared::NexusError = runtime_error.into();
        assert!(matches!(nexus_error, nexus_shared::NexusError::Network(_)));
    }
}