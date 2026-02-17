// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Tests for extension security system.

use super::*;
use super::super::{
    ExtensionCapability, ExtensionMetadata, ExtensionCategory, ResourceLimits,
};
use std::collections::HashSet;

#[test]
fn test_resource_quotas_from_limits() {
    let limits = ResourceLimits {
        max_cpu_percent: 50.0,
        max_memory_bytes: 1024 * 1024 * 1024,
        max_storage_bytes: 10 * 1024 * 1024 * 1024,
        max_network_bandwidth: 100 * 1024 * 1024,
        ..Default::default()
    };

    let quotas = ResourceQuotas::from(limits.clone());
    assert_eq!(quotas.cpu_percent, limits.max_cpu_percent);
    assert_eq!(quotas.memory_bytes, limits.max_memory_bytes);
    assert_eq!(quotas.storage_bytes, limits.max_storage_bytes);
}

#[tokio::test]
async fn test_resource_monitor() {
    let quotas = ResourceQuotas {
        cpu_percent: 50.0,
        memory_bytes: 1024 * 1024,
        storage_bytes: 10 * 1024 * 1024,
        network_bandwidth: 1024 * 1024,
        file_descriptors: 100,
        max_threads: 10,
        ops_per_second: 100,
    };

    let monitor = ResourceMonitor::new("test".to_string(), quotas);

    // Update with usage within limits
    let usage = ResourceUsage {
        cpu_percent: 25.0,
        memory_bytes: 512 * 1024,
        storage_bytes: 1024 * 1024,
        network_bytes: 0,
        file_descriptors: 10,
        thread_count: 5,
        ops_per_second: 50.0,
        last_update: Some(std::time::SystemTime::now()),
    };

    monitor.update_usage(usage).await.expect("test");
    assert!(monitor.check_quotas().await.is_ok());

    // Update with usage exceeding limits
    let excessive_usage = ResourceUsage {
        cpu_percent: 75.0,
        memory_bytes: 2 * 1024 * 1024,
        ..Default::default()
    };

    monitor.update_usage(excessive_usage).await.expect("test");
    assert!(monitor.check_quotas().await.is_err());
}

#[tokio::test]
async fn test_security_manager() {
    let config = SecurityConfig::default();
    let manager = SecurityManager::new(config);

    let metadata = ExtensionMetadata {
        id: "test".to_string(),
        name: "Test".to_string(),
        version: semver::Version::parse("1.0.0").expect("test"),
        description: "Test".to_string(),
        author: "Test".to_string(),
        license: "MIT".to_string(),
        homepage: None,
        category: ExtensionCategory::AssetLibrary,
        hypermesh_version: semver::Version::parse("1.0.0").expect("test"),
        dependencies: vec![],
        required_capabilities: HashSet::from([
            ExtensionCapability::AssetManagement,
            ExtensionCapability::NetworkAccess,
        ]),
        provided_assets: vec![],
        certificate_fingerprint: None,
        config_schema: None,
    };

    let quotas = ResourceQuotas {
        cpu_percent: 50.0,
        memory_bytes: 1024 * 1024,
        storage_bytes: 10 * 1024 * 1024,
        network_bandwidth: 1024 * 1024,
        file_descriptors: 100,
        max_threads: 10,
        ops_per_second: 100,
    };

    // Create context with limited capabilities
    let context = manager.create_context(
        "test".to_string(),
        &metadata,
        HashSet::from([ExtensionCapability::AssetManagement]),
        quotas,
    ).await.expect("test");

    assert_eq!(context.extension_id, "test");
    assert!(context.capabilities.contains(&ExtensionCapability::AssetManagement));
    assert!(!context.capabilities.contains(&ExtensionCapability::NetworkAccess));

    // Check granted capability
    assert!(manager.check_capability(
        "test",
        &ExtensionCapability::AssetManagement,
        "test_operation"
    ).await.is_ok());

    // Check non-granted capability
    assert!(manager.check_capability(
        "test",
        &ExtensionCapability::NetworkAccess,
        "test_operation"
    ).await.is_err());
}
