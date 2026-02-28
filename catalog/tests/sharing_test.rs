// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for decentralized asset library sharing
//!
//! Tests the filled P2P sharing implementations from Sprint 23.

mod common;

use std::sync::Arc;
use std::time::Duration;

use catalog::sharing::{
    SharingManager, SharingConfig, SharingStats,
    SyncManager, SyncStrategy, MirrorManager,
    DiscoveryService,
    SharingProtocol, SharePermission,
    NetworkTopology,
    PeerInfo,
};
use catalog::registry::{CatalogRegistry, RegistryConfig, TrustPolicy};
use catalog::assets::AssetMetadata;
use blockmatrix::assets::core::{
    AssetRegistration, AssetData, NetworkScope, AssetCategory, BaseSystemType,
};
use hypermesh_lib::PrivacyMode;

// ---------------------------------------------------------------------------
// Helper: create a CatalogRegistry for sharing components that need one
// ---------------------------------------------------------------------------
fn create_test_registry() -> Arc<CatalogRegistry> {
    Arc::new(CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    ))
}

/// Helper: create a deterministic AssetRegistration from a string key
fn test_asset_registration(key: &str) -> AssetRegistration {
    let asset_data = AssetData {
        config: key.as_bytes().to_vec(),
        definition: b"test_asset".to_vec(),
        metadata: b"{}".to_vec(),
    };
    AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Storage),
    )
}

/// Helper: create a minimal AssetMetadata for discovery tests
fn test_asset_metadata(name: &str, version: &str) -> AssetMetadata {
    AssetMetadata {
        name: name.to_string(),
        version: version.to_string(),
        tags: vec!["test".to_string()],
        description: Some(format!("Test asset {}", name)),
        author: Some("test-author".to_string()),
        license: Some("MIT".to_string()),
        homepage: None,
        repository: None,
        download_count: 0,
        featured: false,
        keywords: vec![name.to_string()],
        created: None,
        updated: None,
    }
}

// ===========================================================================
// SharingManager construction
// ===========================================================================

#[tokio::test]
async fn test_sharing_manager_creation_with_registry() {
    let config = SharingConfig {
        node_id: "test-node-1".to_string(),
        max_mirror_storage: 1024 * 1024 * 1024,
        max_bandwidth: 10 * 1024 * 1024,
        replication_factor: 3,
        default_permission: SharePermission::Public,
        ..Default::default()
    };
    let registry = create_test_registry();
    let manager = SharingManager::new(config, registry).await;
    assert!(manager.is_ok(), "SharingManager::new should succeed");
}

#[tokio::test]
async fn test_sharing_manager_initial_stats_are_zero() {
    let config = SharingConfig::default();
    let registry = create_test_registry();
    let manager = SharingManager::new(config, registry).await.unwrap();

    let stats: SharingStats = manager.get_stats().await;
    assert_eq!(stats.packages_shared, 0);
    assert_eq!(stats.packages_mirrored, 0);
    assert_eq!(stats.active_peers, 0);
    assert_eq!(stats.sync_operations, 0);
}

// ===========================================================================
// DiscoveryService
// ===========================================================================

#[tokio::test]
async fn test_discovery_service_register_and_search() {
    let discovery = DiscoveryService::new(Duration::from_secs(3600)).await.unwrap();

    let asset_id = test_asset_registration("pkg-abc-123");
    let metadata = test_asset_metadata("test-package", "1.0.0");
    discovery.register_package(&asset_id, &metadata, SharePermission::Public).await.unwrap();

    let results = discovery.search_local("test-package").await.unwrap();
    assert!(!results.is_empty(), "search_local should find the registered package");
}

#[tokio::test]
async fn test_discovery_service_has_package() {
    let discovery = DiscoveryService::new(Duration::from_secs(60)).await.unwrap();

    let asset_id = test_asset_registration("pkg-has-check");
    let has_before = discovery.has_package(&asset_id).await.unwrap();
    assert!(!has_before, "Package should not exist yet");

    let metadata = test_asset_metadata("has-check", "1.0.0");
    discovery.register_package(&asset_id, &metadata, SharePermission::Public).await.unwrap();

    let has_after = discovery.has_package(&asset_id).await.unwrap();
    assert!(has_after, "Package should exist after registration");
}

#[tokio::test]
async fn test_discovery_service_popular_packages() {
    let discovery = DiscoveryService::new(Duration::from_secs(3600)).await.unwrap();

    for i in 0..5 {
        let id = test_asset_registration(&format!("pop-pkg-{}", i));
        let metadata = test_asset_metadata(&format!("popular-{}", i), "1.0.0");
        discovery.register_package(&id, &metadata, SharePermission::Public).await.unwrap();
    }

    let popular = discovery.get_popular_packages(0.0).await.unwrap();
    assert!(popular.len() >= 5, "All packages should pass threshold 0.0");
}

#[tokio::test]
async fn test_discovery_service_fuzzy_search() {
    let discovery = DiscoveryService::new(Duration::from_secs(3600)).await.unwrap();

    let id = test_asset_registration("calc-001");
    let metadata = test_asset_metadata("calculator", "1.0.0");
    discovery.register_package(&id, &metadata, SharePermission::Public).await.unwrap();

    let results = discovery.fuzzy_search("calculatr", 2).await.unwrap();
    assert!(!results.is_empty(), "Fuzzy search should find 'calculator' with distance 2");
}

#[tokio::test]
async fn test_discovery_service_full_text_search() {
    let discovery = DiscoveryService::new(Duration::from_secs(3600)).await.unwrap();

    let id = test_asset_registration("ml-toolkit");
    let mut metadata = test_asset_metadata("machine-learning-toolkit", "1.0.0");
    metadata.description = Some("A comprehensive ML toolkit for machine learning".to_string());
    discovery.register_package(&id, &metadata, SharePermission::Public).await.unwrap();

    let results = discovery.full_text_search("machine learning").await.unwrap();
    assert!(!results.is_empty(), "Full text search should find the ML toolkit");
}

// ===========================================================================
// SharingProtocol
// ===========================================================================

#[tokio::test]
async fn test_sharing_protocol_connect_returns_peer_info() {
    let protocol = SharingProtocol::new(
        10 * 1024 * 1024,
        1024 * 1024,
    ).await.unwrap();

    let peer = protocol.connect("192.168.1.100:9000").await.unwrap();
    assert!(!peer.node_id.is_empty(), "Peer should have a node_id");
    assert_eq!(peer.address, "192.168.1.100:9000");
}

#[tokio::test]
async fn test_sharing_protocol_connect_deterministic_peer_id() {
    let protocol = SharingProtocol::new(10 * 1024 * 1024, 1024 * 1024).await.unwrap();

    let peer1 = protocol.connect("192.168.1.100:9000").await.unwrap();
    let peer2 = protocol.connect("192.168.1.100:9000").await.unwrap();
    assert_eq!(peer1.node_id, peer2.node_id, "Same address should yield same peer ID");
}

#[tokio::test]
async fn test_sharing_protocol_set_permission() {
    let protocol = SharingProtocol::new(10 * 1024 * 1024, 1024 * 1024).await.unwrap();

    let asset_id = test_asset_registration("asset-perm-test");
    protocol.set_permission(&asset_id, SharePermission::Public).await.unwrap();

    let private_id = test_asset_registration("asset-private");
    protocol.set_permission(&private_id, SharePermission::Private).await.unwrap();
    // No panic = success
}

#[tokio::test]
async fn test_sharing_protocol_bandwidth_negotiation() {
    let protocol = SharingProtocol::new(
        10 * 1024 * 1024,
        1024 * 1024,
    ).await.unwrap();

    let peer = protocol.connect("peer-bw:9001").await.unwrap();
    let allocated = protocol.negotiate_bandwidth(&peer.node_id, 2 * 1024 * 1024).await.unwrap();
    assert!(allocated <= 1024 * 1024, "Allocated bandwidth should be capped by fair_use_limit");
    assert!(allocated > 0, "Allocated bandwidth should be positive");
}

// ===========================================================================
// NetworkTopology
// ===========================================================================

#[tokio::test]
async fn test_topology_add_and_remove_peer() {
    let mut topology = NetworkTopology::new("local-node".to_string());

    topology.add_peer("peer-a", "10.0.0.1:9000").await.unwrap();
    topology.add_peer("peer-b", "10.0.0.2:9000").await.unwrap();

    // find_route may fail without links, but should not panic
    let route = topology.find_route("local-node", "peer-a").await;
    let _ = route;

    topology.remove_peer("peer-a").await.unwrap();
}

#[tokio::test]
async fn test_topology_measure_link_creates_metrics() {
    let mut topology = NetworkTopology::new("link-0".to_string());
    topology.add_peer("link-0", "10.0.3.0:9000").await.unwrap();
    topology.add_peer("link-1", "10.0.3.1:9000").await.unwrap();

    let link = topology.measure_link("link-0", "link-1").await.unwrap();
    assert!(link.latency > 0 || link.bandwidth > 0, "Link should have metrics");
}

#[tokio::test]
async fn test_topology_find_route_no_stored_links() {
    let mut topology = NetworkTopology::new("node-0".to_string());

    topology.add_peer("node-0", "10.0.0.0:9000").await.unwrap();
    topology.add_peer("node-1", "10.0.0.1:9000").await.unwrap();
    topology.add_peer("node-2", "10.0.0.2:9000").await.unwrap();

    // measure_link computes metrics but does NOT store links in the topology,
    // so find_route should fail (no stored edges for Dijkstra)
    topology.measure_link("node-0", "node-1").await.unwrap();
    topology.measure_link("node-1", "node-2").await.unwrap();

    let route = topology.find_route("node-0", "node-2").await;
    // Without stored links, routing has no edges to traverse
    assert!(route.is_err(), "find_route should fail without stored links");
}

#[tokio::test]
async fn test_topology_distance_score() {
    let mut topology = NetworkTopology::new("dist-0".to_string());
    topology.add_peer("dist-0", "10.0.2.0:9000").await.unwrap();
    topology.add_peer("dist-1", "10.0.2.1:9000").await.unwrap();
    topology.measure_link("dist-0", "dist-1").await.unwrap();

    let score = topology.get_distance_score("dist-0", "dist-1");
    assert!(score >= 0.0 && score <= 1.0, "Distance score should be in [0.0, 1.0]");
}

#[tokio::test]
async fn test_topology_self_distance_score() {
    let topology = NetworkTopology::new("self-node".to_string());
    let score = topology.get_distance_score("self-node", "self-node");
    assert!((score - 1.0).abs() < f64::EPSILON, "Distance to self should be 1.0");
}

// ===========================================================================
// SyncManager
// ===========================================================================

#[tokio::test]
async fn test_sync_manager_creation() {
    let registry = create_test_registry();
    let sync = SyncManager::new(
        "sync-node-1".to_string(),
        Duration::from_secs(300),
        registry,
    ).await;
    assert!(sync.is_ok(), "SyncManager::new should succeed");
}

#[tokio::test]
async fn test_sync_manager_selective_sync() {
    let registry = create_test_registry();
    let sync = SyncManager::new(
        "sync-node-sel".to_string(),
        Duration::from_secs(300),
        registry,
    ).await.unwrap();

    // Build a PeerInfo to sync with
    let peer = PeerInfo {
        node_id: "peer-sel-1".to_string(),
        address: "10.0.0.50:9000".to_string(),
        available_packages: std::collections::HashSet::new(),
        storage_capacity: 1024 * 1024 * 1024,
        bandwidth_capacity: 10 * 1024 * 1024,
        trust_weight: 0.9,
        last_seen: std::time::SystemTime::now(),
        location: None,
        supported_protocols: vec!["stoq".to_string()],
    };

    // selective_sync with Selective strategy for library category
    let result = sync.selective_sync(
        &peer,
        SyncStrategy::Selective { categories: vec!["library".to_string()] },
    ).await;
    assert!(result.is_ok(), "selective_sync should succeed");
}

// ===========================================================================
// MirrorManager
// ===========================================================================

#[tokio::test]
async fn test_mirror_manager_creation() {
    let registry = create_test_registry();
    let mirror = MirrorManager::new(
        10 * 1024 * 1024 * 1024,
        3,
        registry,
    ).await;
    assert!(mirror.is_ok(), "MirrorManager::new should succeed");
}

#[tokio::test]
async fn test_mirror_manager_get_storage_usage() {
    let registry = create_test_registry();
    let mirror = MirrorManager::new(
        10 * 1024 * 1024 * 1024,
        3,
        registry,
    ).await.unwrap();

    let usage = mirror.get_storage_usage().await.unwrap();
    assert_eq!(usage, 0, "Initial storage usage should be 0");
}

#[tokio::test]
async fn test_mirror_manager_health_check() {
    let registry = create_test_registry();
    let mirror = MirrorManager::new(
        10 * 1024 * 1024 * 1024,
        3,
        registry,
    ).await.unwrap();

    let result = mirror.health_check().await;
    assert!(result.is_ok(), "health_check should succeed on empty manager");
}

#[tokio::test]
async fn test_mirror_manager_update_popularity() {
    let registry = create_test_registry();
    let mirror = MirrorManager::new(
        10 * 1024 * 1024 * 1024,
        3,
        registry,
    ).await.unwrap();

    let asset_id = test_asset_registration("pkg-popular-001");
    let result = mirror.update_popularity(
        &asset_id,
        true,
        Some("user-abc".to_string()),
    ).await;
    assert!(result.is_ok(), "update_popularity should succeed");
}

// ===========================================================================
// Legacy tests (gated behind future-tests feature)
// ===========================================================================

#[cfg(feature = "future-tests")]
mod future_sharing_tests {
    use catalog::sharing::{
        SharingManager, SharingConfig, SharePermission,
    };
    use catalog::assets::AssetPackage;
    use std::time::Duration;

    mod common {
        pub use crate::common::*;
    }

    fn create_test_package(name: &str, version: &str) -> AssetPackage {
        common::create_test_package(name, version)
    }

    #[tokio::test]
    async fn test_sharing_workflow() {
        let config = SharingConfig {
            node_id: "test-node".to_string(),
            auto_mirror_popular: true,
            enable_incentives: true,
            ..Default::default()
        };
        let manager = SharingManager::new(config).await.unwrap();

        let package = create_test_package("workflow-test", "1.0.0");
        let result = manager.share_package(&package, SharePermission::Public).await;
        assert!(result.is_ok(), "Failed to share package");

        let stats = manager.get_stats().await;
        assert_eq!(stats.packages_shared, 1);
    }
}
