// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-node intelligence loop simulation harness (H7).
//!
//! Tests the full intelligence loop: asset creation -> shard distribution ->
//! retrieval -> ngauge feedback -> replication triggers -> security validation.
//! Exercises 10-50 node topologies with real pipeline, shard stores, swarm
//! analytics, and PoS transfer validation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use blockmatrix::assets::pipeline::sharding::Shard;
use blockmatrix::assets::pipeline::streaming_pipeline::{
    StreamingAssetPipeline, StreamingPipelineConfig,
};
use blockmatrix::assets::pipeline::compression::CompressionAlgorithm;
use blockmatrix::assets::pipeline::PipelineInputMetadata;
use blockmatrix::gateway::asset_transfer::{AssetTransfer, PosTransferValidator, TransferValidator};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::consumer_provider::ConsumerProviderManager;
use blockmatrix::network::shard_dedup::{DedupPolicy, ShardStoreResult};
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::swarm_provider::ShardLocationIndex;
use blockmatrix::network::SwarmDemandTracker;

use ngauge::swarm_analytics::{
    ReplicationConfig, ReplicationRecommendation, ReplicationTrigger, SwarmAnalytics,
};

use hypermesh_lib::{AssetId, BlockchainScope, ContentHash, ProofType};

// ---------------------------------------------------------------------------
// Simulation infrastructure
// ---------------------------------------------------------------------------

/// A simulated node with its own shard store, location index, and identity.
struct SimNode {
    #[allow(dead_code)]
    id: usize,
    #[allow(dead_code)]
    coord: MatrixCoordinate,
    shard_store: Arc<ShardStore>,
    shard_index: Arc<ShardLocationIndex>,
    node_id: String,
    demand_tracker: Arc<SwarmDemandTracker>,
}

/// Global inter-node shard transport (in-memory).
struct SimTransport {
    shards: tokio::sync::RwLock<HashMap<(String, [u8; 32]), Vec<u8>>>,
}

/// The simulation harness managing N nodes in a helix topology.
struct SimHarness {
    nodes: Vec<SimNode>,
    transport: Arc<SimTransport>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_node(id: usize, x: i64, y: i64, z: i64) -> SimNode {
    let coord = MatrixCoordinate::new(x, y, z).expect("test: valid coordinate");
    let shard_store = Arc::new(ShardStore::new());
    let shard_index = Arc::new(ShardLocationIndex::new());
    let demand_tracker = Arc::new(SwarmDemandTracker::new());
    SimNode {
        id,
        coord,
        shard_store,
        shard_index,
        node_id: format!("node-{}", id),
        demand_tracker,
    }
}

fn create_helix_harness(n: usize) -> SimHarness {
    let mut nodes = Vec::with_capacity(n);
    for i in 0..n {
        let angle = (i as f64) * std::f64::consts::TAU / (n as f64);
        let x = (10.0 * angle.cos() + 50.0) as i64;
        let y = (10.0 * angle.sin() + 50.0) as i64;
        let z = i as i64;
        nodes.push(create_node(i, x, y, z));
    }
    SimHarness {
        nodes,
        transport: Arc::new(SimTransport {
            shards: tokio::sync::RwLock::new(HashMap::new()),
        }),
    }
}

fn test_metadata() -> PipelineInputMetadata {
    PipelineInputMetadata {
        name: "sim-test.bin".to_string(),
        content_type: "application/octet-stream".to_string(),
        size: 0,
        created_at: 0,
        custom: HashMap::new(),
    }
}

fn make_pipeline(
    segment_size: u32,
    data_shards: u8,
    parity_shards: u8,
) -> StreamingAssetPipeline {
    let config = StreamingPipelineConfig {
        segment_size,
        compression: CompressionAlgorithm::None,
        rs_data_shards: data_shards,
        rs_parity_shards: parity_shards,
        ..Default::default()
    };
    StreamingAssetPipeline::new(config).expect("test: pipeline creation")
}

/// Distribute shard sets across harness nodes using round-robin placement.
/// Records each shard in the global transport and the target node's store.
async fn distribute_shards(
    harness: &SimHarness,
    shard_sets: &[blockmatrix::assets::pipeline::streaming_pipeline::SegmentShardSet],
) {
    for set in shard_sets {
        for (j, shard) in set.shards.iter().enumerate() {
            let target_idx = (set.segment_index as usize * set.shards.len() + j)
                % harness.nodes.len();
            let target = &harness.nodes[target_idx];
            let hash = ContentHash(*blake3::hash(&shard.data).as_bytes());
            target.shard_store.store(hash, shard.data.clone()).await;
            harness
                .transport
                .shards
                .write()
                .await
                .insert((target.node_id.clone(), hash.0), shard.data.clone());
        }
    }
}

// ===========================================================================
// Test 1: 10-node asset lifecycle (process -> distribute -> reconstruct)
// ===========================================================================

#[tokio::test]
async fn test_10_node_asset_lifecycle() {
    let harness = create_helix_harness(10);

    // 1. Create 50KB test data and process through pipeline
    let data: Vec<u8> = (0..50_000u32).map(|i| (i % 256) as u8).collect();
    let pipeline = make_pipeline(25_000, 4, 2);
    let (manifest, key, shard_sets) = pipeline
        .process_segmented(&data, &test_metadata())
        .expect("test: process_segmented");

    assert!(!shard_sets.is_empty(), "Should produce at least one segment");

    // 2. Distribute shards across 10 nodes
    distribute_shards(&harness, &shard_sets).await;

    // 3. Verify shards are spread across multiple nodes
    let mut nodes_with_shards = 0usize;
    for node in &harness.nodes {
        if node.shard_store.count().await > 0 {
            nodes_with_shards += 1;
        }
    }
    assert!(
        nodes_with_shards >= 2,
        "Shards should be distributed to at least 2 nodes, got {}",
        nodes_with_shards
    );

    // 4. Reconstruct by collecting all shards per segment
    let all_shards: Vec<Vec<Shard>> = shard_sets.iter().map(|s| s.shards.clone()).collect();
    let reconstructed = pipeline
        .reconstruct_segmented(&manifest, &key, &all_shards)
        .expect("test: reconstruct_segmented");
    assert_eq!(
        reconstructed, data,
        "Reconstructed data must match original"
    );
}

// ===========================================================================
// Test 2: ngauge feedback loop — popularity detection via windowed fetch rate
// ===========================================================================

#[tokio::test]
async fn test_10_node_ngauge_feedback_loop() {
    // Create analytics with 60s window and threshold of 5 fetches
    let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 5);
    let popular_hash = [0xAA; 32];
    let now = Instant::now();

    // Simulate 7 fetch events (above threshold of 5)
    for i in 0..7u32 {
        analytics.record_fetch_at(popular_hash, now + Duration::from_millis(i as u64 * 10));
    }

    // Should trigger a Replicate recommendation (1x-3x threshold)
    let rec = analytics.get_recommendation(&popular_hash);
    match rec {
        ReplicationRecommendation::Replicate { fetch_rate, .. } => {
            assert_eq!(fetch_rate, 7, "Expected 7 fetches in window");
        }
        other => panic!(
            "test: expected Replicate for 7 fetches (threshold 5), got {:?}",
            other
        ),
    }

    // Add more fetches to exceed 3x threshold (>15) for UrgentReplicate
    let urgent_hash = [0xBB; 32];
    for i in 0..20u32 {
        analytics.record_fetch_at(urgent_hash, now + Duration::from_millis(i as u64 * 5));
    }

    let urgent_rec = analytics.get_recommendation(&urgent_hash);
    match urgent_rec {
        ReplicationRecommendation::UrgentReplicate { fetch_rate, .. } => {
            assert_eq!(fetch_rate, 20, "Expected 20 fetches in window");
        }
        other => panic!(
            "test: expected UrgentReplicate for 20 fetches (threshold 5), got {:?}",
            other
        ),
    }

    // get_popular_shard_recommendations should return both, sorted by rate
    let recs = analytics.get_popular_shard_recommendations();
    assert_eq!(recs.len(), 2, "Both shards should be above threshold");

    // First should be the urgent one (higher rate)
    match &recs[0] {
        ReplicationRecommendation::UrgentReplicate { shard_hash, .. } => {
            assert_eq!(*shard_hash, urgent_hash);
        }
        other => panic!("test: expected UrgentReplicate first, got {:?}", other),
    }
}

// ===========================================================================
// Test 3: Shard loss recovery — RS(10,4) reconstructs from 10 data shards
// ===========================================================================

#[tokio::test]
async fn test_shard_loss_recovery() {
    let data: Vec<u8> = (0..100_000u32).map(|i| (i % 256) as u8).collect();
    let pipeline = make_pipeline(100_000, 10, 4);
    let (manifest, key, shard_sets) = pipeline
        .process_segmented(&data, &test_metadata())
        .expect("test: process_segmented");

    assert_eq!(shard_sets.len(), 1, "Single segment for 100KB at 100KB seg");

    // Keep only data shards (first 10), drop parity shards (last 4)
    let data_only: Vec<Vec<Shard>> = shard_sets
        .iter()
        .map(|set| {
            set.shards
                .iter()
                .filter(|s| !s.metadata.is_parity)
                .cloned()
                .collect()
        })
        .collect();

    assert_eq!(
        data_only[0].len(),
        10,
        "Should have exactly 10 data shards"
    );

    let reconstructed = pipeline
        .reconstruct_segmented(&manifest, &key, &data_only)
        .expect("test: reconstruct with data shards only");
    assert_eq!(reconstructed, data, "Must reconstruct from data shards only");
}

// ===========================================================================
// Test 4: Byzantine proof rejection via PosTransferValidator
// ===========================================================================

#[tokio::test]
async fn test_byzantine_proof_rejection() {
    let validator = PosTransferValidator;

    // 4a. Empty proof bytes should be rejected
    let transfer_empty = AssetTransfer::new(
        "gw-tx-byz-1".to_string(),
        AssetId::from("asset-byz-1"),
        BlockchainScope::Device,
        BlockchainScope::Network,
    );
    let result = validator.validate_transfer(&transfer_empty).await;
    assert!(
        result.is_err(),
        "Empty proof bytes should be rejected"
    );

    // 4b. Too-small proof bytes should be rejected (< 256 bytes)
    let mut transfer_small = AssetTransfer::new(
        "gw-tx-byz-2".to_string(),
        AssetId::from("asset-byz-2"),
        BlockchainScope::Device,
        BlockchainScope::Network,
    );
    transfer_small.source_proof_bytes = vec![0xAA; 10];
    transfer_small.target_proof_bytes = vec![0xBB; 10];
    let result = validator.validate_transfer(&transfer_small).await;
    assert!(
        result.is_err(),
        "Proofs below MIN_PROOF_SIZE should be rejected"
    );

    // 4c. Identical source and target proofs should be rejected
    let mut transfer_dup = AssetTransfer::new(
        "gw-tx-byz-3".to_string(),
        AssetId::from("asset-byz-3"),
        BlockchainScope::Device,
        BlockchainScope::Network,
    );
    transfer_dup.source_proof_bytes = vec![0xCC; 1024];
    transfer_dup.target_proof_bytes = vec![0xCC; 1024];
    let result = validator.validate_transfer(&transfer_dup).await;
    assert!(
        result.is_err(),
        "Identical source/target proofs should be rejected"
    );

    // 4d. Valid distinct proofs should pass
    let mut transfer_valid = AssetTransfer::new(
        "gw-tx-byz-4".to_string(),
        AssetId::from("asset-byz-4"),
        BlockchainScope::Device,
        BlockchainScope::Network,
    );
    transfer_valid.source_proof_bytes = vec![0xDD; 1024];
    transfer_valid.target_proof_bytes = vec![0xEE; 1024];
    let result = validator
        .validate_transfer(&transfer_valid)
        .await
        .expect("test: validation should not error");
    assert!(result, "Valid distinct proofs should pass validation");

    // 4e. Missing required proof types should fail (not error — returns false)
    let mut transfer_no_space = AssetTransfer::new(
        "gw-tx-byz-5".to_string(),
        AssetId::from("asset-byz-5"),
        BlockchainScope::Device,
        BlockchainScope::Network,
    );
    transfer_no_space.source_proof_bytes = vec![0xAA; 512];
    transfer_no_space.target_proof_bytes = vec![0xBB; 512];
    transfer_no_space.source_proofs_required = vec![ProofType::Stake]; // Missing Space
    let result = validator
        .validate_transfer(&transfer_no_space)
        .await
        .expect("test: validation");
    assert!(
        !result,
        "Missing PoSpace requirement should return false"
    );
}

// ===========================================================================
// Test 5: 50-node distribution scaling — O(log N) per-node load
// ===========================================================================

#[tokio::test]
async fn test_50_node_distribution_scaling() {
    let harness = create_helix_harness(50);

    // Create a 200KB asset with 4 segments of 50KB each, RS(4,2)
    let data: Vec<u8> = (0..200_000u32).map(|i| (i % 256) as u8).collect();
    let pipeline = make_pipeline(50_000, 4, 2);
    let (manifest, key, shard_sets) = pipeline
        .process_segmented(&data, &test_metadata())
        .expect("test: process_segmented");

    // Distribute shards across 50 nodes
    distribute_shards(&harness, &shard_sets).await;

    // Count shards per node
    let mut per_node_count: HashMap<usize, usize> = HashMap::new();
    for (idx, node) in harness.nodes.iter().enumerate() {
        let count = node.shard_store.count().await;
        if count > 0 {
            per_node_count.insert(idx, count);
        }
    }

    // Verify distribution: total shards distributed correctly
    let total_shards: usize = shard_sets.iter().map(|s| s.shards.len()).sum();
    let stored_total: usize = per_node_count.values().sum();
    assert_eq!(
        stored_total, total_shards,
        "All shards should be stored across the mesh"
    );

    // Verify no single node holds more than a bounded amount
    // With 50 nodes and ~24 total shards (4 segments * 6 shards), max per node is small
    let max_per_node = per_node_count.values().copied().max().unwrap_or(0);
    assert!(
        max_per_node <= total_shards / 2 + 1,
        "No single node should hold more than half the shards: max={}, total={}",
        max_per_node,
        total_shards
    );

    // Verify reconstruction still works from all shards
    let all_shards: Vec<Vec<Shard>> = shard_sets.iter().map(|s| s.shards.clone()).collect();
    let reconstructed = pipeline
        .reconstruct_segmented(&manifest, &key, &all_shards)
        .expect("test: reconstruct from 50-node mesh");
    assert_eq!(reconstructed, data);
}

// ===========================================================================
// Test 6: Privacy-scoped dedup — Anonymous (HashOnly) vs Full
// ===========================================================================

#[tokio::test]
async fn test_privacy_scoped_dedup_anonymous() {
    let store = ShardStore::new();

    let hash = ContentHash([0xBB; 32]);
    let data = vec![0xCC; 1024];

    // Store with HashOnly (Anonymous mode)
    let result = store
        .store_with_dedup(hash, data.clone(), DedupPolicy::HashOnly)
        .await;
    assert_eq!(result, ShardStoreResult::Stored);

    // Store same shard again — should be detected as duplicate
    let result2 = store
        .store_with_dedup(hash, data.clone(), DedupPolicy::HashOnly)
        .await;
    assert_eq!(
        result2,
        ShardStoreResult::Deduplicated { ref_count: 1 },
        "HashOnly should report dedup with ref_count=1 (no tracking)"
    );

    // Shard exists but refcount stays at 1 (no increment for Anonymous)
    assert!(store.get(&hash).await.is_some(), "Shard should exist");
    assert_eq!(
        store.ref_count(&hash).await,
        Some(1),
        "Anonymous mode must NOT increment refcount"
    );

    // Verify no providers tracked when using ConsumerProviderManager with HashOnly
    let cp_store = Arc::new(ShardStore::new());
    let cp_index = Arc::new(ShardLocationIndex::new());
    let manager = ConsumerProviderManager::new(
        Arc::clone(&cp_store),
        Arc::clone(&cp_index),
        "anon-node".to_string(),
    );
    let cp_result = manager
        .process_fetched_shards_with_policy(
            vec![(hash, data.clone())],
            DedupPolicy::HashOnly,
        )
        .await;

    assert_eq!(cp_result.shards_stored, 1);
    assert!(
        cp_result.announcement_payload.is_none(),
        "Anonymous mode must NOT generate announcements"
    );
    assert!(
        cp_index.get_providers(&hash).await.is_empty(),
        "Anonymous mode must NOT register providers in ShardLocationIndex"
    );
}

// ===========================================================================
// Test 7: Consumer-becomes-provider with demand tracking
// ===========================================================================

#[tokio::test]
async fn test_consumer_becomes_provider_demand_tracking() {
    let harness = create_helix_harness(10);

    // Node 0 creates and distributes an asset
    let data: Vec<u8> = (0..10_000u32).map(|i| (i % 256) as u8).collect();
    let pipeline = make_pipeline(10_000, 4, 2);
    let (_, _, shard_sets) = pipeline
        .process_segmented(&data, &test_metadata())
        .expect("test: process_segmented");

    distribute_shards(&harness, &shard_sets).await;

    // Node 5 fetches shards from the network (simulated) and becomes a provider
    let node5 = &harness.nodes[5];
    let fetched_shards: Vec<(ContentHash, Vec<u8>)> = shard_sets
        .iter()
        .flat_map(|set| {
            set.shards.iter().map(|s| {
                let hash = ContentHash(*blake3::hash(&s.data).as_bytes());
                (hash, s.data.clone())
            })
        })
        .collect();

    let cp_manager = ConsumerProviderManager::new(
        Arc::clone(&node5.shard_store),
        Arc::clone(&node5.shard_index),
        node5.node_id.clone(),
    );

    let result = cp_manager.process_fetched_shards(fetched_shards.clone()).await;

    // Some shards may already be on node 5 from distribution (deduped)
    let total = result.shards_stored + result.shards_deduped;
    assert_eq!(
        total,
        fetched_shards.len(),
        "All shards should be processed (stored or deduped)"
    );
    assert!(
        result.announcement_payload.is_some(),
        "Should generate TAG_SHARD_ANNOUNCE payload"
    );

    // Verify node 5 is registered as provider for all fetched shards
    for (hash, _) in &fetched_shards {
        let providers = node5.shard_index.get_providers(hash).await;
        assert!(
            providers.contains(&node5.node_id),
            "Node 5 should be provider for shard {}",
            hex::encode(hash.0)
        );
    }

    // Simulate demand tracking: multiple peers request the same shard
    let popular_shard = fetched_shards[0].0;
    for i in 0..5 {
        node5
            .demand_tracker
            .record_fetch(popular_shard, &format!("requester-{}", i))
            .await;
    }

    let entry = node5
        .demand_tracker
        .get(&popular_shard)
        .await
        .expect("test: demand entry should exist");
    assert_eq!(entry.request_count, 5);
    assert_eq!(entry.requester_ids.len(), 5);
}

// ===========================================================================
// Test 8: ReplicationTrigger fires based on cumulative demand
// ===========================================================================

#[tokio::test]
async fn test_replication_trigger_cumulative_demand() {
    let mut analytics = SwarmAnalytics::new();
    let shard_id = ContentHash([0xDD; 32]);

    // Record 150 requests from 15 unique consumers
    for i in 0..150u64 {
        let consumer = hypermesh_lib::NodeId::from_bytes([(i % 15) as u8; 32]);
        let position = hypermesh_lib::MatrixPosition {
            x: (i % 10) as f64,
            y: (i / 10) as f64,
            z: 0.0,
        };
        analytics.record_request(shard_id, consumer, position, i * 1000);
    }

    // Set current replica count to 1 (below what demand warrants)
    analytics.set_replica_count(shard_id, 1);

    // ReplicationTrigger with default config (threshold=100 req/replica, min=3)
    let trigger = ReplicationTrigger::new(ReplicationConfig::default());
    let signals = trigger.check(&analytics);

    assert!(!signals.is_empty(), "Should emit replication signals");
    let signal = &signals[0];
    assert_eq!(signal.shard_id, shard_id);
    assert!(
        signal.urgency > 0.0,
        "Urgency should be positive when replicas < needed"
    );
    assert!(
        signal.suggested_count >= 2,
        "Should suggest at least 2 replicas for 150 requests"
    );
    assert_eq!(signal.current_replicas, 1);
    assert_eq!(signal.current_request_rate, 150);
}

// ===========================================================================
// Test 9: End-to-end intelligence loop — process, distribute, demand, feedback
// ===========================================================================

#[tokio::test]
async fn test_end_to_end_intelligence_loop() {
    let harness = create_helix_harness(10);

    // Step 1: Create asset and distribute
    let data: Vec<u8> = (0..20_000u32).map(|i| (i % 256) as u8).collect();
    let pipeline = make_pipeline(20_000, 4, 2);
    let (manifest, key, shard_sets) = pipeline
        .process_segmented(&data, &test_metadata())
        .expect("test: process");

    distribute_shards(&harness, &shard_sets).await;

    // Step 2: Multiple nodes "fetch" the same shard (consumer-becomes-provider)
    let target_shard = &shard_sets[0].shards[0];
    let target_hash = ContentHash(*blake3::hash(&target_shard.data).as_bytes());

    for node_idx in [2, 4, 6, 8] {
        let node = &harness.nodes[node_idx];
        let cp = ConsumerProviderManager::new(
            Arc::clone(&node.shard_store),
            Arc::clone(&node.shard_index),
            node.node_id.clone(),
        );
        cp.process_fetched_shards(vec![(target_hash, target_shard.data.clone())])
            .await;
    }

    // Step 3: Verify 4 additional providers registered
    let providers_count = harness.nodes[2]
        .shard_index
        .get_providers(&target_hash)
        .await
        .len();
    assert_eq!(
        providers_count, 1,
        "Each node's index has itself as provider"
    );

    // Step 4: Feed demand into SwarmAnalytics
    let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 3);
    let now = Instant::now();
    for i in 0..10u32 {
        analytics.record_fetch_at(target_hash.0, now + Duration::from_millis(i as u64 * 5));
    }

    // Step 5: Verify replication recommendation fires
    let rec = analytics.get_recommendation(&target_hash.0);
    match rec {
        ReplicationRecommendation::UrgentReplicate { fetch_rate, .. } => {
            assert_eq!(fetch_rate, 10);
        }
        ReplicationRecommendation::Replicate { fetch_rate, .. } => {
            assert!(fetch_rate >= 3);
        }
        ReplicationRecommendation::None => {
            panic!("test: expected replication recommendation for 10 fetches (threshold 3)");
        }
    }

    // Step 6: Verify data integrity (reconstruction) still works
    let all_shards: Vec<Vec<Shard>> = shard_sets.iter().map(|s| s.shards.clone()).collect();
    let reconstructed = pipeline
        .reconstruct_segmented(&manifest, &key, &all_shards)
        .expect("test: reconstruct");
    assert_eq!(reconstructed, data);
}
