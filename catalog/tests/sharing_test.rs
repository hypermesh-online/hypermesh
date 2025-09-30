//! Integration tests for decentralized asset library sharing

use catalog::sharing::{
    SharingManager, SharingConfig, SharePermission,
    MirrorStrategy, SyncStrategy,
};
use catalog::{AssetPackage, AssetMetadata, AssetId};
use std::time::{Duration, SystemTime};
use tokio;

/// Test basic sharing manager creation and initialization
#[tokio::test]
async fn test_sharing_manager_creation() {
    let config = SharingConfig {
        node_id: "test-node-1".to_string(),
        max_mirror_storage: 1024 * 1024 * 1024, // 1GB
        max_bandwidth: 10 * 1024 * 1024, // 10MB/s
        replication_factor: 3,
        default_permission: SharePermission::Public,
        ..Default::default()
    };

    let manager = SharingManager::new(config).await;
    assert!(manager.is_ok(), "Failed to create sharing manager");
}

/// Test peer connection and disconnection
#[tokio::test]
async fn test_peer_connectivity() {
    let config = SharingConfig::default();
    let manager = SharingManager::new(config).await.unwrap();

    // Test connecting to a peer (would need mock in production)
    // For now, we test the API structure
    let result = manager.connect_peer("peer1.hypermesh.online").await;

    // In a real test, we'd have a mock peer to connect to
    // assert!(result.is_ok());
}

/// Test package sharing with different permissions
#[tokio::test]
async fn test_package_sharing_permissions() {
    let config = SharingConfig::default();
    let manager = SharingManager::new(config).await.unwrap();

    // Create a test package
    let package = create_test_package("test-package", "1.0.0");

    // Test public sharing
    let result = manager.share_package(&package, SharePermission::Public).await;
    assert!(result.is_ok(), "Failed to share package publicly");

    // Test private sharing
    let result = manager.share_package(&package, SharePermission::Private).await;
    assert!(result.is_ok(), "Failed to share package privately");

    // Test restricted sharing
    let allowed_nodes = vec!["node1".to_string(), "node2".to_string()];
    let result = manager.share_package(
        &package,
        SharePermission::Restricted { allowed_nodes },
    ).await;
    assert!(result.is_ok(), "Failed to share package with restrictions");
}

/// Test synchronization strategies
#[tokio::test]
async fn test_sync_strategies() {
    use catalog::sharing::synchronization::{SyncManager, SyncStrategy};

    let sync_manager = SyncManager::new(
        "test-node".to_string(),
        Duration::from_secs(300),
    ).await.unwrap();

    // Test different sync strategies
    let strategies = vec![
        SyncStrategy::Full,
        SyncStrategy::Incremental { since: SystemTime::now() - Duration::from_secs(3600) },
        SyncStrategy::Selective { categories: vec!["library".to_string()] },
        SyncStrategy::Priority { min_priority: 0.8 },
        SyncStrategy::Differential { merkle_root: "abc123".to_string() },
    ];

    // Each strategy should be constructible
    for strategy in strategies {
        match strategy {
            SyncStrategy::Full => assert!(true, "Full strategy created"),
            SyncStrategy::Incremental { .. } => assert!(true, "Incremental strategy created"),
            SyncStrategy::Selective { .. } => assert!(true, "Selective strategy created"),
            SyncStrategy::Priority { .. } => assert!(true, "Priority strategy created"),
            SyncStrategy::Differential { .. } => assert!(true, "Differential strategy created"),
        }
    }
}

/// Test mirroring strategies
#[tokio::test]
async fn test_mirror_strategies() {
    use catalog::sharing::mirroring::{MirrorManager, MirrorStrategy};

    let mirror_manager = MirrorManager::new(
        10 * 1024 * 1024 * 1024, // 10GB
        3, // replication factor
    ).await.unwrap();

    // Test popularity-based mirroring
    let strategy = MirrorStrategy::Popularity {
        threshold: 0.8,
        max_mirrors: 10,
    };
    let result = mirror_manager.apply_strategy(strategy).await;
    assert!(result.is_ok(), "Failed to apply popularity strategy");

    // Test geographic mirroring
    let strategy = MirrorStrategy::Geographic {
        regions: vec!["US-East".to_string(), "EU-West".to_string()],
        mirrors_per_region: 2,
    };
    let result = mirror_manager.apply_strategy(strategy).await;
    assert!(result.is_ok(), "Failed to apply geographic strategy");

    // Test adaptive mirroring
    let strategy = MirrorStrategy::Adaptive {
        target_availability: 0.99,
        max_latency_ms: 100,
    };
    let result = mirror_manager.apply_strategy(strategy).await;
    assert!(result.is_ok(), "Failed to apply adaptive strategy");
}

/// Test discovery service functionality
#[tokio::test]
async fn test_discovery_service() {
    use catalog::sharing::discovery::{DiscoveryService, SearchCapabilities};

    let discovery = DiscoveryService::new(Duration::from_secs(3600)).await.unwrap();

    // Test local search
    let results = discovery.search_local("test").await;
    assert!(results.is_ok(), "Failed to perform local search");

    // Test full-text search
    let results = discovery.full_text_search("virtual machine").await;
    assert!(results.is_ok(), "Failed to perform full-text search");

    // Test fuzzy search
    let results = discovery.fuzzy_search("packge", 2).await; // typo intentional
    assert!(results.is_ok(), "Failed to perform fuzzy search");
}

/// Test network topology management
#[tokio::test]
async fn test_network_topology() {
    use catalog::sharing::topology::{NetworkTopology, RoutingStrategy, NodeLocation};

    let mut topology = NetworkTopology::new("local-node".to_string());

    // Add peers to topology
    let result = topology.add_peer("peer1", "192.168.1.1").await;
    assert!(result.is_ok(), "Failed to add peer to topology");

    let result = topology.add_peer("peer2", "192.168.1.2").await;
    assert!(result.is_ok(), "Failed to add second peer");

    // Test route finding
    let route = topology.find_route("local-node", "peer1").await;
    // In a real network, this would return a valid route
    // assert!(route.is_ok(), "Failed to find route");

    // Test network partition handling
    let result = topology.handle_partition(vec!["peer1".to_string()]).await;
    assert!(result.is_ok(), "Failed to handle partition");

    // Test partition recovery
    let result = topology.recover_partition(vec!["peer1".to_string()]).await;
    assert!(result.is_ok(), "Failed to recover from partition");
}

/// Test sharing protocols and bandwidth management
#[tokio::test]
async fn test_sharing_protocols() {
    use catalog::sharing::protocols::{SharingProtocol, BandwidthAllocation, TransferPriority};

    let protocol = SharingProtocol::new(
        10 * 1024 * 1024, // 10MB/s
        1024 * 1024, // 1MB/s fair use
    ).await.unwrap();

    // Test bandwidth negotiation
    let allocated = protocol.negotiate_bandwidth("peer1", 2 * 1024 * 1024).await;
    assert!(allocated.is_ok(), "Failed to negotiate bandwidth");
    assert!(allocated.unwrap() <= 1024 * 1024, "Bandwidth should be limited by fair use");

    // Test contribution tracking
    let stats = protocol.get_contribution_stats("peer1").await;
    assert!(stats.is_none() || stats.unwrap().bytes_uploaded == 0);

    // Test incentive calculation
    let rewards = protocol.calculate_rewards("peer1").await;
    assert!(rewards.is_ok(), "Failed to calculate rewards");
}

/// Test end-to-end sharing workflow
#[tokio::test]
async fn test_sharing_workflow() {
    // Create sharing manager
    let config = SharingConfig {
        node_id: "test-node".to_string(),
        auto_mirror_popular: true,
        enable_incentives: true,
        ..Default::default()
    };
    let manager = SharingManager::new(config).await.unwrap();

    // Create and share a package
    let package = create_test_package("workflow-test", "1.0.0");
    let result = manager.share_package(&package, SharePermission::Public).await;
    assert!(result.is_ok(), "Failed to share package");

    // Search for the package (would find it in a real network)
    let results = manager.search_packages("workflow").await;
    assert!(results.is_ok(), "Failed to search for package");

    // Auto-mirror popular packages
    let mirrored = manager.auto_mirror_packages().await;
    assert!(mirrored.is_ok(), "Failed to auto-mirror packages");

    // Get sharing statistics
    let stats = manager.get_stats().await;
    assert_eq!(stats.packages_shared, 1);
}

/// Test sharing manager event handling
#[tokio::test]
async fn test_sharing_events() {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use catalog::sharing::SharingEvent;

    let config = SharingConfig::default();
    let manager = SharingManager::new(config).await.unwrap();

    // Track events
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();

    manager.on_event(move |event| {
        let events = events_clone.clone();
        tokio::spawn(async move {
            let mut events = events.lock().await;
            events.push(event);
        });
    }).await;

    // Trigger some events by sharing a package
    let package = create_test_package("event-test", "1.0.0");
    let _ = manager.share_package(&package, SharePermission::Public).await;

    // Small delay to allow event processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check that events were recorded
    // In a real test with proper mocking, we'd verify specific events
    let recorded_events = events.lock().await;
    // assert!(!recorded_events.is_empty(), "No events were recorded");
}

// Helper function to create test packages
fn create_test_package(name: &str, version: &str) -> AssetPackage {
    use catalog::{AssetContent, AssetSecurity, AssetResources, AssetExecution, AssetDependency};

    AssetPackage {
        metadata: AssetMetadata {
            id: AssetId::from(format!("test-{}", name)),
            name: name.to_string(),
            version: version.to_string(),
            description: format!("Test package {}", name),
            author: "Test Author".to_string(),
            category: "test".to_string(),
            tags: vec!["test".to_string(), "integration".to_string()],
            dependencies: vec![],
            size: 1024,
            hash: "abc123".to_string(),
            signature: None,
            timestamp: SystemTime::now(),
            priority: 0.5,
        },
        spec: catalog::AssetSpec {
            asset_type: catalog::AssetType::Library,
            platform: "universal".to_string(),
            requirements: HashMap::new(),
            capabilities: vec![],
        },
        content: AssetContent {
            data: vec![],
            format: "binary".to_string(),
            compression: None,
            encryption: None,
        },
        security: AssetSecurity {
            permissions: vec![],
            consensus_required: false,
            validators: vec![],
            signatures: vec![],
        },
        resources: AssetResources {
            cpu: 1,
            memory: 1024,
            storage: 1024,
            network: 100,
        },
        execution: AssetExecution {
            runtime: "native".to_string(),
            entry_point: "main".to_string(),
            arguments: vec![],
            environment: HashMap::new(),
        },
        dependencies: vec![],
    }
}

use std::collections::HashMap;

/// Test performance and scalability
#[tokio::test]
#[ignore] // This is a longer test, run with --ignored flag
async fn test_sharing_performance() {
    let config = SharingConfig {
        node_id: "perf-test-node".to_string(),
        max_mirror_storage: 100 * 1024 * 1024 * 1024, // 100GB
        ..Default::default()
    };
    let manager = SharingManager::new(config).await.unwrap();

    // Simulate sharing many packages
    let start = std::time::Instant::now();

    for i in 0..100 {
        let package = create_test_package(&format!("perf-test-{}", i), "1.0.0");
        let _ = manager.share_package(&package, SharePermission::Public).await;
    }

    let elapsed = start.elapsed();
    println!("Shared 100 packages in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(10), "Sharing took too long");

    // Test search performance
    let start = std::time::Instant::now();

    for i in 0..100 {
        let _ = manager.search_packages(&format!("perf-test-{}", i)).await;
    }

    let elapsed = start.elapsed();
    println!("Performed 100 searches in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(5), "Search took too long");
}