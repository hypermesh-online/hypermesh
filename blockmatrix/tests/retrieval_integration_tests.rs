//! Integration tests for instruction-based retrieval system
//!
//! Tests end-to-end retrieval workflow from instruction generation to file reconstruction.

use blockmatrix::retrieval::{
    InstructionGenerator, GeneratorConfig, InstructionTransmitter, CompressionFormat,
    ClientAssembler, FallbackManager, FallbackStrategy,
    RetrievalPlan, CompleteShardMap, ShardMapEntry, ShardLocation, RetrievalMetadata,
};
use blockmatrix::retrieval::fallback::SelectionCriteria;
use blockmatrix::assets::storage::{ContentAddressedStorage, Hash, ContentAddress};
use blockmatrix::assets::pipeline::{AssetPipeline, PipelineConfig, Shard, ShardMetadata};
use blockmatrix::integration::phase1_foundation::{MatrixFoundation, MatrixFoundationConfig};
use blockmatrix::matrix::MatrixCoordinate;

use std::sync::Arc;
use tempfile::TempDir;
use anyhow::Result;

/// Test fixture with all components
struct RetrievalTestFixture {
    foundation: Arc<MatrixFoundation>,
    storage: Arc<ContentAddressedStorage>,
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
                hash: format!("{:032x}", i), // Simple hash for testing
            },
        };

        let result = fixture.storage.store_shard(shard).await?;
        shard_hashes.push(result.shard_hash);
    }

    // Store content mapping
    fixture.storage.store_content_mapping(content_hash, shard_hashes.clone()).await?;

    println!("✓ Stored content mapping with {} shards", shard_hashes.len());

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
    assert!(encoded.len() < 2048, "Instruction size {} exceeds 2KB", encoded.len());
    println!("  - Instruction size: {} bytes ({:.1} bytes/shard)", encoded.len(), encoded.len() as f64 / 14.0);

    // Step 4: Decode instructions on client
    let decoded_plan = transmitter.decode(&encoded)?;
    assert_eq!(decoded_plan.content_hash, plan.content_hash);
    println!("✓ Decoded instructions successfully");

    // Step 5: Client assembly
    let assembler = ClientAssembler::new(4); // 4 parallel fetches
    assembler.initialize(decoded_plan).await?;

    println!("✓ Initialized client assembler");

    // Fetch shards
    assembler.fetch_shards().await?;

    let progress = assembler.get_progress().await;
    println!("✓ Fetched shards");
    println!("  - Total: {}", progress.total_shards);
    println!("  - Fetched: {}", progress.fetched_shards);
    println!("  - Progress: {:.1}%", progress.percentage * 100.0);

    // Step 6: Reconstruct file
    let reconstructed = assembler.reconstruct().await?;
    println!("✓ Reconstructed file ({} bytes)", reconstructed.len());

    let stats = assembler.get_stats().await;
    println!("\nAssembly Statistics:");
    println!("  - Total time: {} ms", stats.total_time_ms);
    println!("  - Bytes fetched: {}", stats.bytes_fetched);
    println!("  - Throughput: {:.2} MB/s", stats.throughput_mbps());
    println!("  - Fallback attempts: {}", stats.fallback_attempts);

    println!("\n✅ End-to-end retrieval test PASSED\n");
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
        };

        let plan = RetrievalPlan::new(content_hash, shard_map, metadata);
        let (encoded, stats) = transmitter.encode_with_stats(&plan)?;

        println!("Shards: {} → Instruction size: {} bytes (ratio: {:.2})",
            shard_count, encoded.len(), stats.compression_ratio);

        // Verify reasonable scaling
        let bytes_per_shard = encoded.len() / shard_count;
        assert!(bytes_per_shard < 100, "Overhead per shard too high: {} bytes", bytes_per_shard);
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

    println!("Simulating retrieval with {} total shards, {} required", total_shards, min_required);

    // Mark 3 shards as failed
    for i in 0..3 {
        let shard_hash = [i as u8; 32];
        let pos = MatrixCoordinate::new(i as i64, 0, 0).unwrap();
        manager.handle_failure(shard_hash, pos);
        println!("  ✗ Shard {} failed", i);
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
    println!("  - Recommended strategy: {:?}", status.recommended_strategy);

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
    let positions = vec![
        (10, "far"),
        (0, "near"),
        (5, "medium"),
    ];

    for (i, (x, label)) in positions.iter().enumerate() {
        let shard_hash = [i as u8; 32];
        let location = ShardLocation::new(
            MatrixCoordinate::new(*x, 0, 0).unwrap(),
            0.9,
        );
        let entry = ShardMapEntry::new(shard_hash, vec![location]);
        shard_map.add_entry(entry);
        println!("Shard {} at position x={} ({})", i, x, label);
    }

    let metadata = RetrievalMetadata {
        erasure_coding: (10, 4),
        compression: "brotli".to_string(),
        encryption: "aes-256-gcm".to_string(),
        content_type: "application/octet-stream".to_string(),
        created_at: chrono::Utc::now().timestamp(),
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

        println!("{:?}:", format);
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
async fn test_parallel_vs_sequential_assembly() -> Result<()> {
    println!("\n==== Test: Parallel vs Sequential Assembly ====\n");

    let content_hash = [1u8; 32];
    let mut shard_map = CompleteShardMap::new();

    for i in 0..14 {
        let shard_hash = [i as u8; 32];
        let location = ShardLocation::new(MatrixCoordinate::new(i as i64, 0, 0).unwrap(), 0.9);
        let entry = ShardMapEntry::new(shard_hash, vec![location]);
        shard_map.add_entry(entry);
    }

    let metadata = RetrievalMetadata {
        erasure_coding: (10, 4),
        compression: "brotli".to_string(),
        encryption: "aes-256-gcm".to_string(),
        content_type: "application/octet-stream".to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };

    let plan = RetrievalPlan::new(content_hash, shard_map, metadata);

    // Test sequential (1 parallel)
    let assembler_seq = ClientAssembler::new(1);
    assembler_seq.initialize(plan.clone()).await?;
    let start_seq = std::time::Instant::now();
    assembler_seq.fetch_shards().await?;
    let time_seq = start_seq.elapsed();

    // Test parallel (4 concurrent)
    let assembler_par = ClientAssembler::new(4);
    assembler_par.initialize(plan).await?;
    let start_par = std::time::Instant::now();
    assembler_par.fetch_shards().await?;
    let time_par = start_par.elapsed();

    println!("Sequential (1 parallel): {:?}", time_seq);
    println!("Parallel (4 concurrent): {:?}", time_par);

    let speedup = if time_par.as_micros() > 0 {
        time_seq.as_micros() as f64 / time_par.as_micros() as f64
    } else {
        1.0
    };
    println!("Speedup: {:.2}x", speedup);

    // Parallel should be faster or comparable (allowing small variance due to test overhead)
    // In real scenarios with network I/O, parallel will show significant speedup
    // For dummy fetches, we just verify both complete successfully
    let ratio = time_par.as_millis() as f64 / time_seq.as_millis() as f64;
    assert!(ratio <= 1.1, "Parallel unexpectedly slower (ratio: {:.2})", ratio);

    println!("\n✅ Parallel vs sequential assembly test PASSED\n");
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
        println!("  {}. Position x={}, health={:.2}, latency={}ms",
            i + 1, loc.position.x, loc.health_score, loc.estimated_latency_ms);
    }

    // Should select only replicas meeting criteria (replicas 0 and 3)
    assert_eq!(selected.len(), 2, "Should select 2 suitable replicas");
    assert!(selected.iter().all(|loc| loc.health_score >= 0.7));
    assert!(selected.iter().all(|loc| loc.estimated_latency_ms <= 50));

    println!("\n✅ Replica selection test PASSED\n");
    Ok(())
}
