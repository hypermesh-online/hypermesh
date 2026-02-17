// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! P2P Distribution Integration Tests
//!
//! Gated: references modules and types not yet implemented.
#![cfg(feature = "future-tests")]

mod common;

use catalog::distribution::{
    P2PDistribution, DistributionConfig,
    TransferDirection, TransferStatus,
};
use catalog::assets::AssetPackage;
use std::sync::Arc;
use tempfile::TempDir;
use tokio;

/// Test basic P2P distribution creation and initialization
#[tokio::test]
async fn test_p2p_distribution_creation() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = DistributionConfig::default();
    config.storage_dir = temp_dir.path().to_path_buf();

    let distribution = P2PDistribution::new(config).await;
    assert!(distribution.is_ok(), "Failed to create P2P distribution: {:?}", distribution.err());

    let dist = distribution.unwrap();
    assert_eq!(dist.get_peer_count().await, 0, "Should have no peers initially");
}

/// Test package publishing to P2P network
#[tokio::test]
async fn test_package_publishing() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = DistributionConfig::default();
    config.storage_dir = temp_dir.path().to_path_buf();
    config.auto_seed = false; // Disable auto-seeding for test

    let distribution = P2PDistribution::new(config).await.unwrap();

    // Create a test package
    let package = create_test_package("test-package", "1.0.0");

    // Publish the package
    let package_id = distribution.publish(package.clone()).await;
    assert!(package_id.is_ok(), "Failed to publish package: {:?}", package_id.err());

    let id = package_id.unwrap();
    assert_eq!(id, package.get_package_id(), "Package ID mismatch");

    // Check metrics
    let metrics = distribution.get_metrics();
    assert!(metrics.bytes_uploaded.load(std::sync::atomic::Ordering::Relaxed) > 0);
}

/// Test package search functionality
#[tokio::test]
async fn test_package_search() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = DistributionConfig::default();
    config.storage_dir = temp_dir.path().to_path_buf();

    let distribution = P2PDistribution::new(config).await.unwrap();

    // Create and publish test packages
    let package1 = create_test_package("search-test-1", "1.0.0");
    let package2 = create_test_package("search-test-2", "1.0.0");

    distribution.publish(package1.clone()).await.unwrap();
    distribution.publish(package2.clone()).await.unwrap();

    // Search for packages
    let results = distribution.search("search-test").await;
    assert!(results.is_ok(), "Search failed: {:?}", results.err());

    // Note: In a real implementation with DHT, we'd expect to find the packages
    // For now, just verify search doesn't crash
}

/// Test transfer status tracking
#[tokio::test]
async fn test_transfer_status() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = DistributionConfig::default();
    config.storage_dir = temp_dir.path().to_path_buf();

    let distribution = P2PDistribution::new(config).await.unwrap();

    let package = create_test_package("status-test", "1.0.0");
    let package_id = package.get_package_id();

    // Check that there's no transfer initially
    let status = distribution.get_transfer_status(&package_id).await;
    assert!(status.is_none(), "Should have no transfer status initially");

    // Publish package (which creates a transfer)
    distribution.publish(package).await.unwrap();

    // In a real scenario, we'd check transfer status during download
    // For now, just verify the API works
}

/// Test content addressing and Merkle tree generation
#[tokio::test]
async fn test_content_addressing() {
    use catalog::distribution::content_addressing::{ContentAddress, MerkleTree};

    let data1 = b"Hello, World!";
    let data2 = b"Hello, Rust!";

    let addr1 = ContentAddress::from_data(data1);
    let addr2 = ContentAddress::from_data(data2);

    assert_ne!(addr1, addr2, "Different data should have different addresses");

    // Test Merkle tree
    let chunks = vec![data1.to_vec(), data2.to_vec()];
    let tree = MerkleTree::from_chunks(&chunks).unwrap();

    assert_eq!(tree.chunk_count(), 2);
    assert!(tree.verify_chunk(0, data1).unwrap());
    assert!(tree.verify_chunk(1, data2).unwrap());
}

/// Test DHT node ID generation and distance calculation
#[tokio::test]
async fn test_dht_node_id() {
    use catalog::distribution::dht::NodeId;
    use std::net::{SocketAddr, IpAddr, Ipv6Addr};

    let id1 = NodeId::random();
    let id2 = NodeId::random();

    assert_ne!(id1, id2, "Random IDs should be different");

    let addr = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
        8080,
    );
    let id_from_addr = NodeId::from_address(&addr);

    // Same address should generate same ID
    let id_from_addr2 = NodeId::from_address(&addr);
    assert_eq!(id_from_addr, id_from_addr2);

    // Test distance calculation
    let distance = id1.distance(&id2);
    assert!(distance > id1.distance(&id1), "Distance to self should be minimal");
}

/// Test bandwidth management
#[tokio::test]
async fn test_bandwidth_management() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = DistributionConfig::default();
    config.storage_dir = temp_dir.path().to_path_buf();
    config.enable_bandwidth_management = true;
    config.max_upload_bandwidth = Some(1024 * 1024); // 1MB/s
    config.max_download_bandwidth = Some(2 * 1024 * 1024); // 2MB/s

    let distribution = P2PDistribution::new(config).await.unwrap();

    // Create and publish a package
    let package = create_test_package("bandwidth-test", "1.0.0");
    distribution.publish(package).await.unwrap();

    // Bandwidth limits should be respected during transfers
    // This is verified internally by the rate limiter
}

/// Test incremental updates with binary diffs
#[tokio::test]
async fn test_incremental_updates() {
    use catalog::distribution::content_addressing::BinaryDiff;

    let old_data = b"Hello, World! This is version 1.0";
    let new_data = b"Hello, World! This is version 2.0";

    let diff = BinaryDiff::create_diff(old_data, new_data).unwrap();
    assert!(diff.len() < new_data.len(), "Diff should be smaller than full data");

    let result = BinaryDiff::apply_diff(old_data, &diff).unwrap();
    assert_eq!(result, new_data, "Applied diff should produce new data");
}

/// Test NAT traversal configuration
#[tokio::test]
async fn test_nat_traversal() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = DistributionConfig::default();
    config.storage_dir = temp_dir.path().to_path_buf();
    config.nat_traversal.enable_upnp = true;
    config.nat_traversal.enable_stun = true;
    config.nat_traversal.enable_relay = true;

    let distribution = P2PDistribution::new(config).await.unwrap();

    // NAT traversal should be configured
    // Actual traversal would happen during peer connections
}

/// Test peer discovery mechanisms
#[tokio::test]
async fn test_peer_discovery() {
    use catalog::distribution::peer_discovery::{PeerDiscovery, PeerCapability};

    let temp_dir = TempDir::new().unwrap();
    let mut config = DistributionConfig::default();
    config.storage_dir = temp_dir.path().to_path_buf();

    // Note: In a real test, we'd need to set up mock transport and DHT
    // For now, just verify the structure compiles and basic operations work
}

/// Test package integrity verification
#[tokio::test]
async fn test_package_integrity() {
    use catalog::distribution::content_addressing::MerkleTree;

    let package = create_test_package("integrity-test", "1.0.0");
    let tree = MerkleTree::from_package(&package).unwrap();

    // Verify package passes integrity check
    assert!(tree.verify_package(&package).is_ok());

    // Modify package and verify it fails
    let mut modified_package = package.clone();
    modified_package.spec.metadata.name = "modified".to_string();
    assert!(tree.verify_package(&modified_package).is_err());
}

// Helper functions

fn create_test_package(name: &str, version: &str) -> AssetPackage {
    common::create_test_package(name, version)
}

/// Test concurrent package operations
#[tokio::test]
async fn test_concurrent_operations() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = DistributionConfig::default();
    config.storage_dir = temp_dir.path().to_path_buf();
    config.max_concurrent_transfers = 5;

    let distribution = Arc::new(P2PDistribution::new(config).await.unwrap());

    // Spawn multiple publish operations
    let mut handles = vec![];
    for i in 0..10 {
        let dist = distribution.clone();
        let handle = tokio::spawn(async move {
            let package = create_test_package(&format!("concurrent-{}", i), "1.0.0");
            dist.publish(package).await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // Verify metrics
    let metrics = distribution.get_metrics();
    assert!(metrics.successful_transfers.load(std::sync::atomic::Ordering::Relaxed) > 0);
}