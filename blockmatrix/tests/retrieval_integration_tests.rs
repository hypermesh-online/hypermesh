// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for instruction-based retrieval system
//!
//! Tests end-to-end retrieval workflow from instruction generation to file reconstruction.

use blockmatrix::assets::pipeline::{AssetPipeline, PipelineConfig, Shard, ShardMetadata};
use blockmatrix::assets::storage::{ContentAddress, ContentAddressedStorage, Hash};
use blockmatrix::integration::phase1_foundation::{MatrixFoundation, MatrixFoundationConfig};
use blockmatrix::matrix::MatrixCoordinate;
use blockmatrix::retrieval::fallback::SelectionCriteria;
use blockmatrix::retrieval::{
    ClientAssembler, CompleteShardMap, CompressionFormat, FallbackManager, FallbackStrategy,
    GeneratorConfig, InstructionGenerator, InstructionTransmitter, RetrievalMetadata,
    RetrievalPlan, ShardLocation, ShardMapEntry,
};

use anyhow::Result;
use std::sync::Arc;
use tempfile::TempDir;

/// Test fixture with all components
struct RetrievalTestFixture {
    foundation: Arc<MatrixFoundation>,
    storage: Arc<ContentAddressedStorage>,
    #[allow(dead_code)]
    pipeline: Arc<AssetPipeline>,
    _temp_dir: TempDir,
}

impl RetrievalTestFixture {
    async fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let config = MatrixFoundationConfig {
            storage_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let foundation = Arc::new(MatrixFoundation::new(config).await?);
        let storage = Arc::new(ContentAddressedStorage::new(foundation.clone()).await?);

        let pipeline_config = PipelineConfig::default();
        let pipeline = Arc::new(AssetPipeline::new(pipeline_config)?);

        Ok(Self {
            foundation,
            storage,
            pipeline,
            _temp_dir: temp_dir,
        })
    }
}

#[allow(dead_code)]
fn create_test_content_address() -> ContentAddress {
    let content_hash = [1u8; 32];
    let mut shard_hashes = Vec::new();
    let mut shard_map = Vec::new();

    // Create 14 shards (Reed-Solomon 10+4)
    for i in 0..14 {
        let shard_hash = [i as u8; 32];
        shard_hashes.push(shard_hash);

        let positions = vec![
            MatrixCoordinate::new(i as i64, 0, 0).unwrap(),
            MatrixCoordinate::new(i as i64, 1, 0).unwrap(),
            MatrixCoordinate::new(i as i64, 2, 0).unwrap(),
        ];

        shard_map.push((shard_hash, positions));
    }

    ContentAddress::new(content_hash, shard_hashes, shard_map)
}

#[tokio::test]
#[ignore = "Requires ShardTransport fixture (legacy stub removed in P0.2)"]
async fn test_end_to_end_retrieval() -> Result<()> {
    println!("\n==== Integration Test: End-to-End Retrieval ====\n");

    let fixture = RetrievalTestFixture::new().await?;

    // Step 1: Create test content
    let content_hash = [42u8; 32];
    let mut shard_hashes: Vec<Hash> = Vec::new();

    // Store each shard to assign positions in the matrix
    for i in 0..14 {
        let shard_data = vec![i as u8; 1024]; // 1KB dummy data per shard
        let shard = Shard {
            data: shard_data.clone(),
            metadata: ShardMetadata {
                index: i,
                is_parity: i >= 10, // Last 4 are parity shards for Reed-Solomon 10+4
                size: 1024,
                original_size: 1024,
                hash: format!("{i:032x}"), // Simple hash for testing
            },
        };

        let result = fixture.storage.store_shard(shard).await?;
        shard_hashes.push(result.shard_hash);
    }

    // Store content mapping
    fixture
        .storage
        .store_content_mapping(content_hash, shard_hashes.clone())
        .await?;

    println!(
        "✓ Stored content mapping with {} shards",
        shard_hashes.len()
    );

    // Step 2: Generate retrieval instructions
    let gen_config = GeneratorConfig::default();
    let generator = InstructionGenerator::new(
        gen_config,
        fixture.foundation.clone(),
        fixture.storage.clone(),
    );

    let plan = generator.generate(content_hash).await?;
    println!("✓ Generated retrieval plan");
    println!("  - Shards: {}", plan.shard_map.entries.len());
    println!("  - Min required: {}", plan.min_shards_required);
    println!("  - Estimated size: {} bytes", plan.estimate_size());

    // Step 3: Transmit instructions
    let transmitter = InstructionTransmitter::new(CompressionFormat::Brotli);
    let (encoded, stats) = transmitter.encode_with_stats(&plan)?;

    println!("✓ Encoded instructions");
    println!("  - Original: {} bytes", stats.original_size);
    println!("  - Compressed: {} bytes", stats.compressed_size);
    println!("  - Ratio: {:.2}", stats.compression_ratio);
    println!("  - Saved: {:.2}%", stats.percentage_saved());

    // Verify instruction size is reasonable (under 2KB for 14 shards)
    // Note: With full position data, instructions can be slightly over 1KB but remain compact
    assert!(
        encoded.len() < 2048,
        "Instruction size {} exceeds 2KB",
        encoded.len()
    );
    println!(
        "  - Instruction size: {} bytes ({:.1} bytes/shard)",
        encoded.len(),
        encoded.len() as f64 / 14.0
    );

    // Step 4: Decode instructions on client
    let decoded_plan = transmitter.decode(&encoded)?;
    assert_eq!(decoded_plan.content_hash, plan.content_hash);
    println!("✓ Decoded instructions successfully");

    // Step 5: Client assembly
    let assembler = ClientAssembler::new(4); // 4 parallel fetches
    assembler.initialize(decoded_plan).await?;

    println!("✓ Initialized client assembler");

    // Fetch + reconstruct requires a ShardTransport fixture
    // (legacy stub removed in P0.2; test remains structured for future re-enable)
    let progress = assembler.get_progress().await;
    println!("  - Total: {}", progress.total_shards);

    println!("\n✅ End-to-end retrieval test PASSED (ignored — no transport)\n");
    Ok(())
}

#[tokio::test]
async fn test_instruction_size_scaling() -> Result<()> {
    println!("\n==== Test: Instruction Size Scaling ====\n");

    // Test that instruction size remains small even for large shard counts
    let transmitter = InstructionTransmitter::new(CompressionFormat::Brotli);

    for shard_count in [14, 28, 56, 112] {
        let content_hash = [1u8; 32];
        let mut shard_map = CompleteShardMap::new();

        for i in 0..shard_count {
            let shard_hash = [(i % 256) as u8; 32];
            let locations = vec![
                ShardLocation::new(MatrixCoordinate::new(i as i64, 0, 0).unwrap(), 0.9),
                ShardLocation::new(MatrixCoordinate::new(i as i64, 1, 0).unwrap(), 0.85),
            ];
            let entry = ShardMapEntry::new(shard_hash, locations);
            shard_map.add_entry(entry);
        }

        let metadata = RetrievalMetadata {
            erasure_coding: (shard_count * 10 / 14, shard_count * 4 / 14),
            compression: "brotli".to_string(),
            encryption: "aes-256-gcm".to_string(),
            content_type: "application/octet-stream".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            encrypted_blob_size: 0,
        };

        let plan = RetrievalPlan::new(content_hash, shard_map, metadata);
        let (encoded, stats) = transmitter.encode_with_stats(&plan)?;

        println!(
            "Shards: {} → Instruction size: {} bytes (ratio: {:.2})",
            shard_count,
            encoded.len(),
            stats.compression_ratio
        );

        // Verify reasonable scaling
        let bytes_per_shard = encoded.len() / shard_count;
        assert!(
            bytes_per_shard < 100,
            "Overhead per shard too high: {bytes_per_shard} bytes"
        );
    }

    println!("\n✅ Instruction size scaling test PASSED\n");
    Ok(())
}

#[tokio::test]
async fn test_fallback_handling() -> Result<()> {
    println!("\n==== Test: Fallback Handling ====\n");

    let mut manager = FallbackManager::with_defaults();

    // Simulate Reed-Solomon 10+4 retrieval with failures
    let total_shards = 14;
    let min_required = 10;

    println!("Simulating retrieval with {total_shards} total shards, {min_required} required");

    // Mark 3 shards as failed
    for i in 0..3 {
        let shard_hash = [i as u8; 32];
        let pos = MatrixCoordinate::new(i as i64, 0, 0).unwrap();
        manager.handle_failure(shard_hash, pos);
        println!("  ✗ Shard {i} failed");
    }

    // Mark remaining as successful
    for i in 3..14 {
        let shard_hash = [i as u8; 32];
        let pos = MatrixCoordinate::new(i as i64, 0, 0).unwrap();
        manager.handle_success(shard_hash, pos);
    }

    let status = manager.get_status();
    println!("\nFallback Status:");
    println!("  - Missing: {}", status.missing_shards);
    println!("  - Retrieved: {}", status.retrieved_shards);
    println!("  - Failure rate: {:.1}%", status.failure_rate * 100.0);
    println!(
        "  - Recommended strategy: {:?}",
        status.recommended_strategy
    );

    // Should still succeed with 11 available shards (need 10)
    assert!(manager.can_succeed(min_required, total_shards));
    println!("\n✓ Can still succeed with {} failures", 3);

    // Test with too many failures
    for i in 3..7 {
        let shard_hash = [i as u8; 32];
        let pos = MatrixCoordinate::new(i as i64, 0, 0).unwrap();
        manager.handle_failure(shard_hash, pos);
    }

    // Now only 7 available (need 10) - should fail
    assert!(!manager.can_succeed(min_required, total_shards));
    println!("✓ Correctly detects insufficient shards with 7 failures");

    println!("\n✅ Fallback handling test PASSED\n");
    Ok(())
}

#[tokio::test]
async fn test_client_position_optimization() -> Result<()> {
    println!("\n==== Test: Client Position Optimization ====\n");

    let content_hash = [1u8; 32];
    let mut shard_map = CompleteShardMap::new();

    // Create shards at different distances from origin
    let positions = [(10, "far"), (0, "near"), (5, "medium")];

    for (i, (x, label)) in positions.iter().enumerate() {
        let shard_hash = [i as u8; 32];
        let location = ShardLocation::new(MatrixCoordinate::new(*x, 0, 0).unwrap(), 0.9);
        let entry = ShardMapEntry::new(shard_hash, vec![location]);
        shard_map.add_entry(entry);
        println!("Shard {i} at position x={x} ({label})");
    }

    let metadata = RetrievalMetadata {
        erasure_coding: (10, 4),
        compression: "brotli".to_string(),
        encryption: "aes-256-gcm".to_string(),
        content_type: "application/octet-stream".to_string(),
        created_at: chrono::Utc::now().timestamp(),
        encrypted_blob_size: 0,
    };

    let mut plan = RetrievalPlan::new(content_hash, shard_map, metadata);

    // Optimize for client at origin
    let client_pos = MatrixCoordinate::new(0, 0, 0).unwrap();
    plan.optimize_for_position(&client_pos);

    println!("\nOptimized retrieval order:");
    for (order, shard_idx) in plan.retrieval_order.iter().enumerate() {
        let entry = plan.shard_map.get_entry(*shard_idx).unwrap();
        let pos = &entry.locations[0].position;
        println!("  {}. Shard {} at x={}", order + 1, shard_idx, pos.x);
    }

    // First shard should be nearest (index 1, x=0)
    assert_eq!(plan.retrieval_order[0], 1, "Nearest shard should be first");

    println!("\n✅ Client position optimization test PASSED\n");
    Ok(())
}

#[tokio::test]
async fn test_compression_format_comparison() -> Result<()> {
    println!("\n==== Test: Compression Format Comparison ====\n");

    let content_hash = [1u8; 32];
    let mut shard_map = CompleteShardMap::new();

    // Create typical Reed-Solomon configuration
    for i in 0..14 {
        let shard_hash = [i as u8; 32];
        let locations = vec![
            ShardLocation::new(MatrixCoordinate::new(i as i64, 0, 0).unwrap(), 0.9),
            ShardLocation::new(MatrixCoordinate::new(i as i64, 1, 0).unwrap(), 0.85),
        ];
        let entry = ShardMapEntry::new(shard_hash, locations);
        shard_map.add_entry(entry);
    }

    let metadata = RetrievalMetadata {
        erasure_coding: (10, 4),
        compression: "brotli".to_string(),
        encryption: "aes-256-gcm".to_string(),
        content_type: "application/octet-stream".to_string(),
        created_at: chrono::Utc::now().timestamp(),
        encrypted_blob_size: 0,
    };

    let plan = RetrievalPlan::new(content_hash, shard_map, metadata);

    let formats = vec![
        CompressionFormat::None,
        CompressionFormat::Brotli,
        CompressionFormat::Zstd,
        CompressionFormat::MessagePack,
    ];

    println!("Format comparison for 14-shard retrieval plan:\n");

    for format in formats {
        let transmitter = InstructionTransmitter::new(format);
        let (encoded, stats) = transmitter.encode_with_stats(&plan)?;

        println!("{format:?}:");
        println!("  Size: {} bytes", encoded.len());
        println!("  Ratio: {:.3}", stats.compression_ratio);
        println!("  Encode time: {} μs", stats.encode_time_us);
        println!();

        // Verify decode works
        let decoded = transmitter.decode(&encoded)?;
        assert_eq!(decoded.content_hash, plan.content_hash);
    }

    println!("✅ Compression format comparison test PASSED\n");
    Ok(())
}

#[tokio::test]
async fn test_replica_selection() -> Result<()> {
    println!("\n==== Test: Replica Selection ====\n");

    let criteria = SelectionCriteria {
        min_health: 0.7,
        max_latency_ms: Some(50),
        ..Default::default()
    };

    let manager = FallbackManager::new(criteria, FallbackStrategy::Adaptive);

    let shard_hash = [1u8; 32];
    let locations = vec![
        {
            let mut loc = ShardLocation::new(MatrixCoordinate::new(0, 0, 0).unwrap(), 0.95);
            loc.estimated_latency_ms = 10;
            loc
        },
        {
            let mut loc = ShardLocation::new(MatrixCoordinate::new(1, 0, 0).unwrap(), 0.6); // Below health threshold
            loc.estimated_latency_ms = 20;
            loc
        },
        {
            let mut loc = ShardLocation::new(MatrixCoordinate::new(2, 0, 0).unwrap(), 0.9);
            loc.estimated_latency_ms = 100; // Above latency threshold
            loc
        },
        {
            let mut loc = ShardLocation::new(MatrixCoordinate::new(3, 0, 0).unwrap(), 0.85);
            loc.estimated_latency_ms = 30;
            loc
        },
    ];

    let entry = ShardMapEntry::new(shard_hash, locations);
    let selected = manager.get_alternatives(&entry, 10);

    println!("Selected {} out of 4 replicas", selected.len());
    for (i, loc) in selected.iter().enumerate() {
        println!(
            "  {}. Position x={}, health={:.2}, latency={}ms",
            i + 1,
            loc.position.x,
            loc.health_score,
            loc.estimated_latency_ms
        );
    }

    // Should select only replicas meeting criteria (replicas 0 and 3)
    assert_eq!(selected.len(), 2, "Should select 2 suitable replicas");
    assert!(selected.iter().all(|loc| loc.health_score >= 0.7));
    assert!(selected.iter().all(|loc| loc.estimated_latency_ms <= 50));

    println!("\n✅ Replica selection test PASSED\n");
    Ok(())
}

// ── Phase A2: two-layer resolve + upstream fallback + become-provider ──

use blockmatrix::network::consumer_provider::ConsumerProviderManager;
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::MockShardTransport;
use blockmatrix::network::swarm_provider::{
    build_shard_locate_response, parse_shard_locate_response, ShardLocationIndex,
};
use blockmatrix::retrieval::client_assembly::fetching::node_id_from_coordinate;
use blockmatrix::retrieval::client_assembly::seeding::ConsumerProviderSeeder;
use blockmatrix::retrieval::location_resolver::{
    coordinate_to_node_id_hex, merge_upstream, resolve_shard_locations, ProviderSource,
};
use hypermesh_lib::ContentHash;

/// A2 Part 1: the two-layer resolve returns BOTH the live-mirror providers
/// (freshest-first) AND the canonical matrix-placement providers, in that
/// order, de-duplicated.
#[tokio::test]
async fn test_a2_two_layer_resolve_merges_live_and_canonical() {
    let index = ShardLocationIndex::new();
    let content_hash = ContentHash([0x5Au8; 32]);

    // Live-mirror layer: two peers currently announcing the shard.
    index.register_provider("live-peer-alpha", &[content_hash]).await;
    index.register_provider("live-peer-beta", &[content_hash]).await;

    // Canonical-placement layer: two matrix cells where the shard is placed.
    let placements = [
        MatrixCoordinate::new(3, 1, 0).unwrap(),
        MatrixCoordinate::new(4, 1, 0).unwrap(),
    ];

    let resolved =
        resolve_shard_locations(&content_hash, Some(&index), &placements).await;

    // Both layers are present: 2 live mirrors + 2 canonical placements.
    assert_eq!(resolved.len(), 4, "resolve must carry BOTH location layers");

    // Live mirrors come first.
    assert!(matches!(resolved[0].source, ProviderSource::LiveMirror));
    assert!(matches!(resolved[1].source, ProviderSource::LiveMirror));
    // Canonical placements follow, tagged with their coordinate.
    assert!(matches!(
        resolved[2].source,
        ProviderSource::CanonicalPlacement { .. }
    ));
    assert!(matches!(
        resolved[3].source,
        ProviderSource::CanonicalPlacement { .. }
    ));

    // Canonical node_ids reconcile coordinate → owning node id.
    assert_eq!(resolved[2].node_id, coordinate_to_node_id_hex(&placements[0]));
    assert_eq!(resolved[3].node_id, coordinate_to_node_id_hex(&placements[1]));

    println!("\n✅ A2 two-layer resolve (live + canonical) PASSED\n");
}

/// A2 Part 2: the upstream tracker fallback fires when the local layers miss,
/// merging upstream-returned providers (that connected peers did NOT know)
/// into the resolve. Exercises the shard-locate wire codec end to end plus the
/// `merge_upstream` reconciliation.
#[tokio::test]
async fn test_a2_upstream_fallback_merges_new_providers() {
    let index = ShardLocationIndex::new();
    let content_hash = ContentHash([0x6Bu8; 32]);

    // Local layers MISS: no live mirrors, no canonical placements.
    let mut resolved = resolve_shard_locations(&content_hash, Some(&index), &[]).await;
    assert!(
        resolved.is_empty(),
        "both local layers must miss to trigger upstream fallback",
    );

    // Simulate an upstream tracker's shard-locate RESPONSE naming a provider
    // that we did not know locally, via the real wire codec.
    let upstream_wire =
        build_shard_locate_response(&["upstream-only-provider".to_string()]);
    let upstream_ids = parse_shard_locate_response(&upstream_wire);
    assert_eq!(upstream_ids, vec!["upstream-only-provider".to_string()]);

    // Fold the upstream answer into the (empty) resolve — the fallback fires.
    merge_upstream(&mut resolved, &upstream_ids);

    assert_eq!(resolved.len(), 1, "upstream fallback must add the new provider");
    assert_eq!(resolved[0].node_id, "upstream-only-provider");
    assert!(matches!(resolved[0].source, ProviderSource::UpstreamTracker));

    println!("\n✅ A2 upstream tracker fallback PASSED\n");
}

/// A2 Part 3: a fetched shard triggers the consumer-becomes-provider
/// re-announce (become-provider) on the unified client-assembly path — the
/// path that PREVIOUSLY skipped this. The `ClientAssembler` is wired with the
/// SAME `ConsumerProviderManager`-backed seeder the live IPC path uses.
#[tokio::test]
async fn test_a2_fetch_triggers_become_provider_reannounce() {
    use blockmatrix::assets::pipeline::{Asset, AssetPipeline, PipelineInputMetadata};
    use std::sync::Arc;

    let original = b"A2 become-provider on the unified fetch path ".repeat(64);
    let asset = Asset {
        id: "a2-seed".to_string(),
        data: original.clone(),
        metadata: PipelineInputMetadata {
            name: "a2.bin".to_string(),
            content_type: "application/octet-stream".to_string(),
            size: original.len(),
            created_at: 1234567890,
            custom: std::collections::HashMap::new(),
        },
    };

    let pipeline = AssetPipeline::default().expect("test: pipeline");
    let processed = pipeline.process_asset(asset).await.expect("test: process");

    // Build a retrieval plan + pre-populate a mock transport at canonical cells.
    let mut shard_map = CompleteShardMap::new();
    let transport = MockShardTransport::new();
    let mut shard_hashes: Vec<ContentHash> = Vec::new();

    for (i, shard) in processed.shards.iter().enumerate() {
        let shard_hash = *blake3::hash(&shard.data).as_bytes();
        shard_hashes.push(ContentHash(shard_hash));
        let pos = MatrixCoordinate::new(i as i64, 0, 0).unwrap();
        shard_map.add_entry(ShardMapEntry::new(shard_hash, vec![ShardLocation::new(pos, 1.0)]));
        transport
            .insert_shard(
                &node_id_from_coordinate(&pos),
                &ContentHash(shard_hash),
                shard.data.clone(),
            )
            .await;
    }

    let data_shards = processed.shards.iter().filter(|s| !s.metadata.is_parity).count();
    let parity_shards = processed.shards.iter().filter(|s| s.metadata.is_parity).count();
    let metadata = RetrievalMetadata {
        erasure_coding: (data_shards, parity_shards),
        compression: "brotli".to_string(),
        encryption: "kyber-1024".to_string(),
        content_type: "application/octet-stream".to_string(),
        created_at: 1234567890,
        encrypted_blob_size: processed.stats.encryption.encrypted_size,
    };
    let mut plan = RetrievalPlan::new([0xA2u8; 32], shard_map, metadata);
    plan.original_size = original.len();

    // Shared swarm state: the SAME index + store the seeder registers into.
    let store = Arc::new(ShardStore::new());
    let index = Arc::new(ShardLocationIndex::new());
    let manager = Arc::new(ConsumerProviderManager::new(
        store.clone(),
        index.clone(),
        "a2-local-node".to_string(),
    ));
    // No peer connections in the test — seed still registers us as a provider.
    let seeder = Arc::new(ConsumerProviderSeeder::new(manager, Vec::new()));

    let assembler = ClientAssembler::new(4)
        .with_live_index(index.clone())
        .with_seeder(seeder);
    assembler.initialize(plan).await.expect("test: init");

    let reconstructed = assembler
        .retrieve_asset(&transport, &processed.decryption_key)
        .await
        .expect("test: retrieve should succeed");
    assert_eq!(reconstructed, original, "unified fetch must reconstruct");

    // BECOME-PROVIDER: every fetched shard registered the local node as a
    // provider in the shared index (this is what the old path skipped).
    for ch in &shard_hashes {
        let providers = index.get_providers(ch).await;
        assert!(
            providers.contains(&"a2-local-node".to_string()),
            "fetched shard must trigger become-provider re-announce",
        );
        // And the shard was seeded into the shared store (we are now a host).
        assert!(store.has(ch).await, "fetched shard must be seeded locally");
    }

    println!("\n✅ A2 fetch triggers become-provider re-announce PASSED\n");
}

/// A2 P1 invariant preserved: a forged shard (data does NOT hash to its claimed
/// content address) is REJECTED at the content gate on the unified fetch path,
/// so it is never stored, never reconstructed, and — critically — never
/// re-announced (a node cannot become a provider for a shard it never validly
/// held).
#[tokio::test]
async fn test_a2_forged_shard_rejected_never_reannounced() {
    use std::sync::Arc;

    let content_hash = ContentHash([0x9Fu8; 32]);
    // Claimed hash for a shard whose bytes will NOT match.
    let honest_data = vec![1u8, 2, 3, 4, 5];
    let claimed_hash = *blake3::hash(&honest_data).as_bytes();

    let mut shard_map = CompleteShardMap::new();
    let pos = MatrixCoordinate::new(0, 0, 0).unwrap();
    shard_map.add_entry(ShardMapEntry::new(claimed_hash, vec![ShardLocation::new(pos, 1.0)]));

    // Transport serves FORGED bytes under the claimed hash's node/id.
    let transport = MockShardTransport::new();
    transport
        .insert_shard(
            &node_id_from_coordinate(&pos),
            &ContentHash(claimed_hash),
            vec![0xDE, 0xAD, 0xBE, 0xEF], // forged: hashes to something else
        )
        .await;

    let metadata = RetrievalMetadata {
        erasure_coding: (1, 0),
        compression: "none".to_string(),
        encryption: "none".to_string(),
        content_type: "application/octet-stream".to_string(),
        created_at: 0,
        encrypted_blob_size: 0,
    };
    let plan = RetrievalPlan::new(content_hash.0, shard_map, metadata);

    let store = Arc::new(ShardStore::new());
    let index = Arc::new(ShardLocationIndex::new());
    let manager = Arc::new(ConsumerProviderManager::new(
        store.clone(),
        index.clone(),
        "a2-forge-node".to_string(),
    ));
    let seeder = Arc::new(ConsumerProviderSeeder::new(manager, Vec::new()));

    let assembler = ClientAssembler::new(1)
        .with_live_index(index.clone())
        .with_seeder(seeder);
    assembler.initialize(plan).await.expect("test: init");

    // Fetch must FAIL: the only location serves a forged shard.
    let result = assembler.fetch_shards_via_transport(&transport).await;
    assert!(result.is_err(), "forged shard must fail the fetch (content gate)");

    // And it must NOT have been seeded / re-announced.
    let providers = index.get_providers(&ContentHash(claimed_hash)).await;
    assert!(
        providers.is_empty(),
        "a forged shard must never trigger become-provider",
    );
    assert!(
        !store.has(&ContentHash(claimed_hash)).await,
        "a forged shard must never be seeded into the store",
    );

    println!("\n✅ A2 forged-shard rejection (P1 gate preserved) PASSED\n");
}
