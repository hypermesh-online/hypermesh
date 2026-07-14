// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for scaling features: genesis adoption, shard dedup,
//! consumer-becomes-provider, and swarm analytics popularity detection.

use std::sync::Arc;
use std::time::{Duration, Instant};

use blockmatrix::blockchain::block::{Block, BlockAssetEntry, BlockHeader, StoragePointer};
use blockmatrix::blockchain::chain::NodeBlockchain;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::consumer_provider::ConsumerProviderManager;
use blockmatrix::network::shard_dedup::{DedupPolicy, ShardStoreResult};
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::swarm_provider::ShardLocationIndex;

use blockmatrix::assets::core::AssetRegistration;
use hypermesh_lib::ContentHash;
use trustchain::proof_of_state::StateProof;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal `BlockAssetEntry` for testing (matches chain.rs unit test pattern).
///
/// The proof is bound to the content hash so the entry satisfies the
/// signed-to-content invariant (P1) enforced on the block-receive path.
fn test_entry(coord: MatrixCoordinate) -> BlockAssetEntry {
    let reg = AssetRegistration::genesis(coord);
    let content_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
    BlockAssetEntry::new_bound(
        content_hash,
        &StateProof::new_for_testing(),
        StoragePointer::Genesis,
        reg,
    )
}

/// Create a ContentHash from a single seed byte.
fn content_hash(seed: u8) -> ContentHash {
    ContentHash([seed; 32])
}

// ===========================================================================
// Test 1: Genesis Adoption Flow
// ===========================================================================

#[tokio::test]
async fn test_genesis_adoption_and_header_sync() {
    // Step 1: Create Node A's blockchain (coord 1,1,1)
    let coord_a = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let chain_a = NodeBlockchain::new(coord_a);

    // Step 2: Create Node B's blockchain (coord 2,2,2)
    let coord_b = MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate");
    let chain_b = NodeBlockchain::new(coord_b);

    // Verify both chains start independently with their own genesis
    let genesis_a = chain_a.get_block(0).await.expect("test: Node A genesis");
    let genesis_b = chain_b.get_block(0).await.expect("test: Node B genesis");
    assert_ne!(
        genesis_a.hash, genesis_b.hash,
        "independent nodes should have different genesis blocks"
    );

    // Step 3: Node B adopts Node A's genesis
    chain_b
        .adopt_genesis(genesis_a.clone())
        .await
        .expect("test: adopt genesis");

    // Step 4: Verify Node B's chain now has Node A's genesis as block 0
    let b_head = chain_b.get_head().await.expect("test: head after adoption");
    assert_eq!(
        b_head.hash, genesis_a.hash,
        "Node B should have Node A's genesis"
    );
    assert_eq!(chain_b.get_height().await, 0);

    // Step 5: Create a few blocks on Node A's chain.
    // Use insert_received_block which validates hash integrity but not state proofs,
    // matching the real cross-node sync path where blocks arrive from peers.
    let mut prev_hash = genesis_a.hash.clone();
    for i in 1..=3u64 {
        let entry = test_entry(coord_a);
        let block = Block::new(i, vec![entry], prev_hash.clone());
        prev_hash = block.hash.clone();
        chain_a
            .insert_received_block(block)
            .await
            .expect("test: insert received block");
    }
    assert_eq!(chain_a.get_height().await, 3);

    // Step 6: Extract headers from Node A's blocks
    let chain_a_blocks = chain_a.get_chain().await;
    let headers: Vec<BlockHeader> = chain_a_blocks
        .iter()
        .filter(|b| b.index > 0) // skip genesis (already adopted)
        .map(|b| b.header())
        .collect();
    assert_eq!(headers.len(), 3);

    // Step 7: Node B inserts received headers
    let inserted = chain_b
        .insert_received_headers(headers)
        .await
        .expect("test: insert headers");
    assert_eq!(inserted, 3);

    // Step 8: Verify get_known_height reflects header height
    assert_eq!(
        chain_b.get_known_height().await,
        3,
        "known height should reflect the highest header"
    );

    // Step 9: Verify get_header returns the headers
    for idx in 1..=3u64 {
        let header = chain_b
            .get_header(idx)
            .await
            .expect("test: header should exist");
        assert_eq!(header.index, idx);
        // Header hash should match the original block
        let original = chain_a
            .get_block(idx)
            .await
            .expect("test: original block");
        assert_eq!(header.hash, original.hash);
    }

    // Step 10: Verify has_full_block returns false for header-only indices
    for idx in 1..=3u64 {
        assert!(
            !chain_b.has_full_block(idx).await,
            "block {} should be header-only on Node B",
            idx
        );
    }
    // Genesis (block 0) should be a full block
    assert!(
        chain_b.has_full_block(0).await,
        "genesis should be a full block"
    );
}

// ===========================================================================
// Test 2: Shard Dedup with Refcount
// ===========================================================================

#[tokio::test]
async fn test_shard_dedup_with_refcount() {
    let tmp = tempfile::TempDir::new().expect("test: create temp dir");
    let store = ShardStore::new_with_dir(tmp.path().to_path_buf());

    let hash = content_hash(0xAB);
    let data = vec![0xDE, 0xAD, 0xBE, 0xEF];

    // Step 2: Store shard with Full dedup -> returns Stored
    let result = store
        .store_with_dedup(hash, data.clone(), DedupPolicy::Full)
        .await;
    assert_eq!(result, ShardStoreResult::Stored);

    // Step 3: Store same shard again -> returns Deduplicated
    let result = store
        .store_with_dedup(hash, data.clone(), DedupPolicy::Full)
        .await;
    assert_eq!(
        result,
        ShardStoreResult::Deduplicated { ref_count: 2 },
        "second store should report deduplication"
    );

    // Step 4: Verify ref_count == 2
    assert_eq!(
        store.ref_count(&hash).await,
        Some(2),
        "refcount should be 2 after two stores"
    );

    // Step 5: release -> refcount == 1
    let rc = store.release(&hash).await.expect("test: release");
    assert_eq!(rc, 1);
    assert_eq!(store.ref_count(&hash).await, Some(1));
    assert!(store.has(&hash).await, "shard should still exist at rc=1");

    // Step 6: release -> refcount == 0, shard removed
    let rc = store.release(&hash).await.expect("test: release");
    assert_eq!(rc, 0);

    // Step 7: Verify get returns None
    assert_eq!(
        store.get(&hash).await,
        None,
        "shard should be gone after refcount reaches 0"
    );
    assert!(
        !store.has(&hash).await,
        "has() should return false for removed shard"
    );
}

// ===========================================================================
// Test 3: Consumer-Becomes-Provider
// ===========================================================================

#[tokio::test]
async fn test_consumer_becomes_provider() {
    let shard_store = Arc::new(ShardStore::new());
    let shard_location_index = Arc::new(ShardLocationIndex::new());
    let manager = ConsumerProviderManager::new(
        Arc::clone(&shard_store),
        Arc::clone(&shard_location_index),
        "test-node-1".to_string(),
    );

    // Step 3: Create 3 test shards with different hashes
    let hash_a = content_hash(0x01);
    let hash_b = content_hash(0x02);
    let hash_c = content_hash(0x03);
    let shards = vec![
        (hash_a, vec![1, 2, 3]),
        (hash_b, vec![4, 5, 6]),
        (hash_c, vec![7, 8, 9]),
    ];

    // Step 4: Process fetched shards
    let result = manager.process_fetched_shards(shards.clone()).await;

    // Step 5: Verify shards_stored == 3, shards_deduped == 0
    assert_eq!(result.shards_stored, 3);
    assert_eq!(result.shards_deduped, 0);

    // Step 6: Verify announcement_payload is Some
    assert!(
        result.announcement_payload.is_some(),
        "should have announcement payload"
    );

    // Step 7: Verify ShardLocationIndex has "test-node-1" as provider for all 3 hashes
    for hash in [hash_a, hash_b, hash_c] {
        let providers = shard_location_index.get_providers(&hash).await;
        assert!(
            providers.contains(&"test-node-1".to_string()),
            "test-node-1 should be a provider for hash {}",
            hex::encode(hash.0)
        );
    }

    // Step 8: Process same shards again
    let result2 = manager.process_fetched_shards(shards).await;

    // Step 9: Verify shards_stored == 0, shards_deduped == 3 (all deduplicated)
    assert_eq!(
        result2.shards_stored, 0,
        "second fetch should store 0 new shards"
    );
    assert_eq!(
        result2.shards_deduped, 3,
        "second fetch should dedup all 3 shards"
    );

    // Step 10: Verify ShardStore refcounts are all 2
    for hash in [hash_a, hash_b, hash_c] {
        assert_eq!(
            shard_store.ref_count(&hash).await,
            Some(2),
            "refcount should be 2 after two fetches"
        );
    }
}

// ===========================================================================
// Test 4: Swarm Analytics Popularity Detection
// ===========================================================================

#[tokio::test]
async fn test_swarm_analytics_popularity_detection() {
    use engauge::swarm_analytics::{ReplicationRecommendation, SwarmAnalytics};

    // Step 1: Create SwarmAnalytics with 60s window, threshold 5
    let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 5);
    let shard_a = [0xAA; 32];
    let shard_b = [0xBB; 32];
    let now = Instant::now();

    // Step 2: Record 3 fetches for shard A -> recommendation should be None
    for i in 0..3 {
        analytics.record_fetch_at(shard_a, now + Duration::from_millis(i * 10));
    }
    let rec = analytics.get_recommendation(&shard_a);
    assert_eq!(
        rec,
        ReplicationRecommendation::None,
        "3 fetches below threshold of 5 should yield None"
    );

    // Step 3: Record 4 more fetches for shard A (total 7) -> should be Replicate
    for i in 3..7 {
        analytics.record_fetch_at(shard_a, now + Duration::from_millis(i * 10));
    }
    let rec = analytics.get_recommendation(&shard_a);
    match rec {
        ReplicationRecommendation::Replicate { fetch_rate, .. } => {
            assert_eq!(fetch_rate, 7, "7 fetches in window");
        }
        other => unreachable!(
            "test: expected Replicate for 7 fetches (threshold 5), got {:?}",
            other
        ),
    }

    // Step 4: Record 20 fetches for shard B -> should be UrgentReplicate (>3x threshold)
    for i in 0..20 {
        analytics.record_fetch_at(shard_b, now + Duration::from_millis(i * 5));
    }
    let rec = analytics.get_recommendation(&shard_b);
    match rec {
        ReplicationRecommendation::UrgentReplicate { fetch_rate, .. } => {
            assert_eq!(fetch_rate, 20, "20 fetches in window");
        }
        other => unreachable!(
            "test: expected UrgentReplicate for 20 fetches (threshold 5), got {:?}",
            other
        ),
    }

    // Step 5: get_popular_shard_recommendations returns 2 entries, B first (higher rate)
    let recs = analytics.get_popular_shard_recommendations();
    assert_eq!(
        recs.len(),
        2,
        "both shard A and shard B should be above threshold"
    );

    // First entry should be shard B (higher fetch rate of 20)
    match &recs[0] {
        ReplicationRecommendation::UrgentReplicate {
            shard_hash,
            fetch_rate,
            ..
        } => {
            assert_eq!(*shard_hash, shard_b, "shard B should be first (highest rate)");
            assert_eq!(*fetch_rate, 20);
        }
        other => unreachable!(
            "test: expected UrgentReplicate for shard B, got {:?}",
            other
        ),
    }

    // Second entry should be shard A (lower fetch rate of 7)
    match &recs[1] {
        ReplicationRecommendation::Replicate {
            shard_hash,
            fetch_rate,
            ..
        } => {
            assert_eq!(*shard_hash, shard_a, "shard A should be second");
            assert_eq!(*fetch_rate, 7);
        }
        other => unreachable!(
            "test: expected Replicate for shard A, got {:?}",
            other
        ),
    }
}
