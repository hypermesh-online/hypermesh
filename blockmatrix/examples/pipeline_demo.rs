// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset Pipeline Demo
//!
//! Demonstrates the complete asset processing pipeline:
//! Compression → Encryption → Sharding → Distribution
//!
//! Gated: references CompressionAlgorithm::Zstd variant and CompressionConfig
//! fields that have been refactored.
#![cfg(feature = "future-tests")]

use blockmatrix::assets::pipeline::{
    Asset, PipelineInputMetadata, AssetPipeline, CompressionAlgorithm, CompressionConfig,
    DistributionConfig, MatrixConstraints, PipelineConfig, ShardingConfig,
};
use blockmatrix::matrix::coordinate::MatrixCoordinate;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Asset Pipeline Demo ===\n");

    // Create sample asset
    let data = b"Hello, World! This is a test asset. ".repeat(1000);
    let asset = Asset {
        id: "demo-asset-001".to_string(),
        data: data.clone(),
        metadata: PipelineInputMetadata {
            name: "demo.txt".to_string(),
            content_type: "text/plain".to_string(),
            size: data.len(),
            created_at: chrono::Utc::now().timestamp(),
            custom: std::collections::HashMap::new(),
        },
    };

    println!("📦 Original Asset:");
    println!("   ID: {}", asset.id);
    println!("   Size: {} bytes", asset.data.len());
    println!();

    // Configure pipeline
    let config = PipelineConfig {
        compression: CompressionConfig {
            algorithm: CompressionAlgorithm::Zstd,
            level: 3,
            chunk_size: 64 * 1024,
            streaming: true,
        },
        sharding: ShardingConfig {
            data_shards: 10,
            parity_shards: 4,
            target_shard_size: 1024 * 1024,
        },
        distribution: DistributionConfig {
            constraints: MatrixConstraints {
                min_distance: 5.0,
                max_distance: 50.0,
                load_balance: true,
                max_hops: 10,
            },
            network_ids: vec!["network-1".to_string(), "network-2".to_string()],
            preferred_zones: vec![],
            replication_factor: 1,
        },
        ..Default::default()
    };

    // Create pipeline
    let num_networks = config.distribution.network_ids.len();
    let mut pipeline = AssetPipeline::new(config)?;

    // Register some nodes
    pipeline
        .distributor_mut()
        .register_node("node-1".to_string(), MatrixCoordinate::new(10, 20, 30)?);
    pipeline
        .distributor_mut()
        .register_node("node-2".to_string(), MatrixCoordinate::new(50, 60, 70)?);
    pipeline
        .distributor_mut()
        .register_node("node-3".to_string(), MatrixCoordinate::new(-10, -20, 15)?);

    println!("🔧 Pipeline Configuration:");
    println!("   Compression: Zstd level 3");
    println!("   Sharding: 10 data + 4 parity shards");
    println!(
        "   Distribution: {} networks, {} nodes",
        num_networks,
        pipeline.distributor_mut().node_count()
    );
    println!();

    // Process asset
    println!("⚙️  Processing asset through pipeline...\n");
    let processed = pipeline.process_asset(asset).await?;

    // Display statistics
    println!("📊 Pipeline Statistics:");
    println!();
    println!("   Stage 1 - Compression:");
    println!(
        "      Original: {} bytes",
        processed.stats.compression.original_size
    );
    println!(
        "      Compressed: {} bytes",
        processed.stats.compression.compressed_size
    );
    println!("      Ratio: {:.2}x", processed.stats.compression.ratio);
    println!("      Time: {} ms", processed.stats.compression.duration_ms);
    println!(
        "      Throughput: {:.2} MB/s",
        processed.stats.compression.throughput_mbps
    );
    println!();

    println!("   Stage 2 - Sharding:");
    println!(
        "      Data shards: {}",
        processed.stats.sharding.data_shards
    );
    println!(
        "      Parity shards: {}",
        processed.stats.sharding.parity_shards
    );
    println!(
        "      Total size: {} bytes",
        processed.stats.sharding.total_shard_size
    );
    println!(
        "      Redundancy: {:.2}x",
        processed.stats.sharding.redundancy_factor
    );
    println!("      Time: {} ms", processed.stats.sharding.duration_ms);
    println!();

    println!("   Stage 3 - Encryption:");
    println!(
        "      Shards encrypted: {}",
        processed.stats.encryption.shards_encrypted
    );
    println!(
        "      Total size: {} bytes",
        processed.stats.encryption.encrypted_size
    );
    println!("      Time: {} ms", processed.stats.encryption.duration_ms);
    println!(
        "      Throughput: {:.2} MB/s",
        processed.stats.encryption.throughput_mbps
    );
    println!();

    println!("   Stage 4 - Distribution:");
    println!(
        "      Shards distributed: {}",
        processed.stats.distribution.shards_distributed
    );
    println!(
        "      Networks used: {}",
        processed.stats.distribution.networks_used
    );
    println!(
        "      Avg shard distance: {:.2}",
        processed.stats.distribution.avg_shard_distance
    );
    println!(
        "      Quality score: {:.1}/100",
        processed.stats.distribution.quality_score
    );
    println!(
        "      Time: {} ms",
        processed.stats.distribution.duration_ms
    );
    println!();

    println!("   Overall:");
    println!("      Total time: {} ms", processed.stats.total_duration_ms);
    println!(
        "      Total throughput: {:.2} MB/s",
        processed.stats.total_throughput_mbps
    );
    println!("      Final size: {} bytes", processed.stats.final_size);
    println!();

    // Shard placement is no longer computed by the pipeline (P4). WHERE shards
    // live is decided on the live store path over the real PoS-eligible peer
    // set (see `network::placement::place_shards` →
    // `distribution::distribute_shards_pos_aware`), which this offline demo has
    // no peers for — so placements are empty here by design.
    println!("📍 Shard Placements: computed on the store path (real peers), not in the pipeline");
    println!("   placements in this offline demo: {}", processed.distributed.placements.len());
    println!();

    // Reconstruct asset
    println!("🔄 Reconstructing asset from shards...\n");
    let reconstructed = pipeline.reconstruct_asset(&processed).await?;

    // Verify integrity
    if reconstructed == data {
        println!("✅ SUCCESS: Asset reconstructed perfectly!");
        println!("   Original size: {} bytes", data.len());
        println!("   Reconstructed size: {} bytes", reconstructed.len());
        println!(
            "   Match: {}",
            if reconstructed == data { "✓" } else { "✗" }
        );
    } else {
        println!("❌ ERROR: Reconstruction failed!");
        return Err(anyhow::anyhow!("Data integrity check failed"));
    }

    println!();
    println!("=== Demo Complete ===");

    Ok(())
}
