//! Content-Addressed Storage Integration Tests
//!
//! Comprehensive test suite for Sprint 2.5 implementation.

use blockmatrix::assets::storage::{
    ContentAddressedStorage, DeduplicationResult, StorageStats,
    ContentAddress, RetrievalInstructions, compute_hash, bucket_id_from_hash,
};
use blockmatrix::assets::pipeline::{Shard, ShardMetadata as PipelineShardMetadata};
use blockmatrix::integration::phase1_foundation::{MatrixFoundation, MatrixFoundationConfig};
use blockmatrix::matrix::MatrixCoordinate;
use std::sync::Arc;
use std::collections::HashSet;
use tokio;

/// Create test shard with specific data
fn create_test_shard(data: Vec<u8>) -> Shard {
    Shard {
        data,
        metadata: PipelineShardMetadata {
            index: 0,
            is_parity: false,
            size: 0,
            original_size: 0,
            hash: String::new(),
        },
    }
}

/// Create test foundation
async fn create_test_foundation() -> Arc<MatrixFoundation> {
    Arc::new(MatrixFoundation::new(MatrixFoundationConfig::default()).await.unwrap())
}

// ============= Unit Tests =============

#[test]
fn test_hash_bucket_creation() {
    // Test that we can create all 256 hash buckets
    for i in 0u8..=255 {
        let mut hash = [0u8; 32];
        hash[0] = i;
        let bucket_id = bucket_id_from_hash(&hash);
        assert_eq!(bucket_id.len(), 2);
        assert_eq!(bucket_id, format!("{:02x}", i));
    }
}

#[test]
fn test_bucket_id_from_hash() {
    // Test all 256 possible bucket IDs
    let test_cases = vec![
        ([0x00; 32], "00"),
        ([0x0f; 32], "0f"),
        ([0x10; 32], "10"),
        ([0x99; 32], "99"),
        ([0xab; 32], "ab"),
        ([0xff; 32], "ff"),
    ];

    for (hash, expected) in test_cases {
        assert_eq!(bucket_id_from_hash(&hash), expected);
    }

    // Test that only first byte matters
    let hash1 = [0xab; 32];
    let mut hash2 = [0x00; 32];
    hash2[0] = 0xab;
    assert_eq!(bucket_id_from_hash(&hash1), bucket_id_from_hash(&hash2));
}

#[tokio::test]
async fn test_o1_lookup_performance() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    // Store many shards to build up the HashMap
    let mut times = Vec::new();

    for i in 0..1000 {
        let shard = create_test_shard(vec![i as u8; 100]);
        let start = std::time::Instant::now();
        let _result = storage.store_shard(shard).await.unwrap();
        times.push(start.elapsed().as_micros());
    }

    // Calculate coefficient of variation
    let mean = times.iter().sum::<u128>() as f64 / times.len() as f64;
    let variance = times.iter()
        .map(|&t| {
            let diff = t as f64 - mean;
            diff * diff
        })
        .sum::<f64>() / times.len() as f64;
    let std_dev = variance.sqrt();
    let cv = std_dev / mean;

    // O(1) operations should have low coefficient of variation (<0.5)
    assert!(cv < 0.5, "Coefficient of variation {} too high for O(1)", cv);
}

#[tokio::test]
async fn test_deduplication_identical_shards() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    let data = vec![1, 2, 3, 4, 5];
    let shard1 = create_test_shard(data.clone());
    let shard2 = create_test_shard(data.clone());

    // First shard should not be deduplicated
    let result1 = storage.store_shard(shard1).await.unwrap();
    assert!(!result1.deduplicated);
    assert_eq!(result1.reference_count, 1);

    // Second identical shard should be deduplicated
    let result2 = storage.store_shard(shard2).await.unwrap();
    assert!(result2.deduplicated);
    assert_eq!(result2.reference_count, 2);
    assert_eq!(result2.space_saved, data.len());
    assert_eq!(result1.positions, result2.positions); // Same positions
}

#[tokio::test]
async fn test_deduplication_different_shards() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    let shard1 = create_test_shard(vec![1, 2, 3]);
    let shard2 = create_test_shard(vec![4, 5, 6]);

    // Both should be unique
    let result1 = storage.store_shard(shard1).await.unwrap();
    let result2 = storage.store_shard(shard2).await.unwrap();

    assert!(!result1.deduplicated);
    assert!(!result2.deduplicated);
    assert_ne!(result1.shard_hash, result2.shard_hash);
}

// ============= Integration Tests =============

#[tokio::test]
async fn test_integration_with_phase1_matrix() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation.clone()).await.unwrap();

    // Store shard and verify matrix positions
    let shard = create_test_shard(vec![1; 1024]);
    let result = storage.store_shard(shard).await.unwrap();

    assert!(!result.positions.is_empty());
    assert_eq!(result.positions.len(), 14); // Reed-Solomon 10+4

    // Verify positions are valid matrix coordinates
    for pos in &result.positions {
        assert!(pos.x >= 0);
        assert!(pos.y >= 0);
        assert!(pos.z >= 0);
    }
}

#[tokio::test]
async fn test_integration_with_sprint_2_4_pipeline() {
    // This test would integrate with the actual pipeline
    // For now, we simulate the pipeline output

    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    // Simulate pipeline output: 14 shards (10 data + 4 parity)
    let mut all_positions = Vec::new();

    for i in 0..14 {
        let shard_data = vec![i as u8; 1024];
        let shard = create_test_shard(shard_data);
        let result = storage.store_shard(shard).await.unwrap();
        all_positions.extend(result.positions);
    }

    // Should have distributed shards across matrix
    let unique_positions: HashSet<_> = all_positions.iter().collect();
    assert!(unique_positions.len() > 10); // Should use multiple positions
}

#[tokio::test]
async fn test_integration_with_sprint_2_3_multi_network() {
    // Test deduplication across multiple isolated networks
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    // Same content from different "networks" (simulated)
    let content = vec![42u8; 2048];

    // Network A uploads
    let shard_a = create_test_shard(content.clone());
    let result_a = storage.store_shard(shard_a).await.unwrap();
    assert!(!result_a.deduplicated);

    // Network B uploads same content - should be deduplicated
    let shard_b = create_test_shard(content.clone());
    let result_b = storage.store_shard(shard_b).await.unwrap();
    assert!(result_b.deduplicated);

    // Network C uploads same content - should also be deduplicated
    let shard_c = create_test_shard(content);
    let result_c = storage.store_shard(shard_c).await.unwrap();
    assert!(result_c.deduplicated);
    assert_eq!(result_c.reference_count, 3);
}

// ============= Performance Tests =============

#[tokio::test]
async fn test_90_percent_deduplication_rate() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    // Upload 1000 files with high similarity
    let base_content = vec![1u8; 1024];

    for i in 0..1000 {
        let mut content = base_content.clone();
        // Only 10% unique content
        if i % 10 == 0 {
            content[0] = i as u8;
        }

        let shard = create_test_shard(content);
        storage.store_shard(shard).await.unwrap();
    }

    let stats = storage.get_stats().await;

    // Should achieve at least 90% deduplication
    assert!(stats.deduplication_rate >= 0.89,
            "Deduplication rate {} below 90%", stats.deduplication_rate);

    // Verify space savings
    assert!(stats.storage_saved > stats.storage_used * 8,
            "Space saved {} not significant vs used {}",
            stats.storage_saved, stats.storage_used);
}

#[tokio::test]
async fn test_o1_bucket_lookups() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    // Test lookup performance remains constant regardless of bucket size
    let mut lookup_times = Vec::new();

    // Fill buckets with varying amounts of data
    for bucket_num in 0..10 {
        let bucket_size = (bucket_num + 1) * 100;

        // Fill this bucket
        for i in 0..bucket_size {
            let mut data = vec![bucket_num as u8; 100];
            data.extend(&(i as u32).to_le_bytes());
            let shard = create_test_shard(data);
            storage.store_shard(shard).await.unwrap();
        }

        // Measure lookup time
        let test_data = vec![bucket_num as u8; 50];
        let shard = create_test_shard(test_data);
        let start = std::time::Instant::now();
        storage.store_shard(shard).await.unwrap();
        lookup_times.push(start.elapsed().as_micros());
    }

    // Verify lookup times don't increase with bucket size
    // Calculate linear regression slope
    let n = lookup_times.len() as f64;
    let x_mean = (n - 1.0) / 2.0;
    let y_mean = lookup_times.iter().sum::<u128>() as f64 / n;

    let numerator: f64 = lookup_times.iter().enumerate()
        .map(|(i, &y)| (i as f64 - x_mean) * (y as f64 - y_mean))
        .sum();
    let denominator: f64 = (0..lookup_times.len())
        .map(|i| (i as f64 - x_mean).powi(2))
        .sum();

    let slope = numerator / denominator;

    // Slope should be near zero for O(1) operations
    assert!(slope.abs() < 1.0, "Lookup time increases with bucket size: slope={}", slope);
}

#[tokio::test]
async fn test_matrix_aware_placement() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    // Store shard and check placement
    let shard = create_test_shard(vec![1; 512]);
    let result = storage.store_shard(shard).await.unwrap();

    // Calculate average distance between positions
    let positions = &result.positions;
    let mut total_distance = 0.0;
    let mut count = 0;

    for i in 0..positions.len() {
        for j in i+1..positions.len() {
            let dx = (positions[i].x - positions[j].x) as f64;
            let dy = (positions[i].y - positions[j].y) as f64;
            let dz = (positions[i].z - positions[j].z) as f64;
            let distance = (dx*dx + dy*dy + dz*dz).sqrt();
            total_distance += distance;
            count += 1;
        }
    }

    let avg_distance = total_distance / count as f64;

    // Positions should be reasonably spread out (not all in same location)
    assert!(avg_distance > 1.0, "Shards too clustered: avg distance {}", avg_distance);
    // But not too far apart (within reasonable matrix region)
    assert!(avg_distance < 50.0, "Shards too spread out: avg distance {}", avg_distance);
}

// ============= Real-World Scenarios =============

#[tokio::test]
async fn test_viral_content_replication() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    // Simulate viral video being accessed by 10,000 users
    let viral_content = vec![99u8; 10 * 1024]; // 10KB video chunk
    let shard = create_test_shard(viral_content.clone());

    // First upload
    let result = storage.store_shard(shard).await.unwrap();
    assert!(!result.deduplicated);
    let initial_positions = result.positions.len();

    // Simulate 10,000 users downloading (all deduplicated)
    let mut dedup_count = 0;
    for _user in 0..100 { // Reduced for test speed
        let shard = create_test_shard(viral_content.clone());
        let result = storage.store_shard(shard).await.unwrap();
        if result.deduplicated {
            dedup_count += 1;
        }

        // Simulate popularity-based replication
        if dedup_count % 20 == 0 {
            storage.update_replication(result.shard_hash, dedup_count).await.unwrap();
        }
    }

    assert_eq!(dedup_count, 100); // All should be deduplicated

    let stats = storage.get_stats().await;
    assert!(stats.deduplication_rate > 0.98); // 99% deduplication rate
    assert_eq!(stats.unique_shards, 1); // Only one unique shard
}

#[tokio::test]
async fn test_software_update_scenario() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    // Simulate OS update files with high redundancy
    let update_files = vec![
        vec![1u8; 5 * 1024],  // System file 1
        vec![2u8; 5 * 1024],  // System file 2
        vec![1u8; 5 * 1024],  // Duplicate of file 1
        vec![3u8; 5 * 1024],  // System file 3
        vec![2u8; 5 * 1024],  // Duplicate of file 2
        vec![1u8; 5 * 1024],  // Another duplicate of file 1
    ];

    let mut unique_count = 0;
    let mut dedup_count = 0;

    for file in update_files {
        let shard = create_test_shard(file);
        let result = storage.store_shard(shard).await.unwrap();
        if result.deduplicated {
            dedup_count += 1;
        } else {
            unique_count += 1;
        }
    }

    assert_eq!(unique_count, 3); // Only 3 unique files
    assert_eq!(dedup_count, 3);  // 3 were deduplicated

    let stats = storage.get_stats().await;
    assert_eq!(stats.deduplication_rate, 0.5); // 50% deduplication
}

#[tokio::test]
async fn test_cat_video_deduplication() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    // 1000 users upload the same cat video
    let cat_video = vec![0xCA; 2048]; // CAT in hex :)

    for user_id in 0..100 { // Reduced for test speed
        let mut video = cat_video.clone();
        // Some users might have slightly different metadata (simulated)
        if user_id % 50 == 0 {
            video.push(user_id as u8); // Different file but mostly same
        }

        let shard = create_test_shard(video);
        storage.store_shard(shard).await.unwrap();
    }

    let stats = storage.get_stats().await;

    // Should have very high deduplication rate
    assert!(stats.deduplication_rate > 0.95,
            "Cat video deduplication rate {} too low", stats.deduplication_rate);

    // Should have saved significant space
    let space_efficiency = stats.storage_saved as f64 / (stats.storage_saved + stats.storage_used) as f64;
    assert!(space_efficiency > 0.9,
            "Space efficiency {} too low", space_efficiency);

    println!("Cat Video Deduplication Results:");
    println!("  Unique shards: {}", stats.unique_shards);
    println!("  Total references: {}", stats.total_references);
    println!("  Deduplication rate: {:.2}%", stats.deduplication_rate * 100.0);
    println!("  Space saved: {} bytes", stats.storage_saved);
    println!("  Space efficiency: {:.2}%", space_efficiency * 100.0);
}

// ============= Content Address Tests =============

#[tokio::test]
async fn test_content_addressing() {
    let foundation = create_test_foundation().await;
    let storage = ContentAddressedStorage::new(foundation).await.unwrap();

    // Create a multi-shard file
    let shard_hashes = vec![
        compute_hash(&[1u8; 32]),
        compute_hash(&[2u8; 32]),
        compute_hash(&[3u8; 32]),
    ];

    let file_hash = compute_hash(b"complete file");

    // Store shards
    for hash_bytes in &shard_hashes {
        let shard = create_test_shard(hash_bytes.to_vec());
        storage.store_shard(shard).await.unwrap();
    }

    // Note: In production, the content mapping would be stored automatically
    // For this test, we'll rely on retrieve() handling the missing mapping gracefully

    // Get content address
    let content_address = storage.get_content_address(file_hash, shard_hashes.clone()).await.unwrap();

    assert_eq!(content_address.content_hash, file_hash);
    assert_eq!(content_address.shard_hashes.len(), 3);
    assert!(content_address.validate().is_ok());

    // Note: Testing retrieval would require storing the content mapping
    // which requires access to private fields. This would be tested
    // in integration tests with full pipeline
}

// Run all tests with: cargo test --test content_addressed_storage_tests