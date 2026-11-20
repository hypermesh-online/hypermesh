// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Offline asset transform and local reconstruction demo.
//!
//! Compress → encrypt the whole blob → shard → reconstruct → decrypt → decompress.
//! No shards are dispatched or fetched from peers. Catalog ingestion and scoped
//! Proof of State validation are separate paths, not exercised by this demo.
#![cfg(feature = "future-tests")]

use blockmatrix::assets::pipeline::{
    Asset, AssetPipeline, CompressionAlgorithm, CompressionConfig, PipelineConfig,
    PipelineInputMetadata, PipelineStats, ProcessedAsset, ShardingConfig,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    println!("=== Offline Asset Pipeline Demo ===\n");
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
    let pipeline = AssetPipeline::new(demo_config())?;
    println!("Asset: {} ({} bytes)", asset.id, asset.data.len());
    println!("Compression: Zstd level 3");
    println!("Encryption: Kyber-1024 KEM + AES-256-GCM, whole compressed blob");
    println!("Sharding: default 10+4 policy with size-adaptive parameters");
    println!("Decryption material stays in memory for this local round trip only.\n");

    let processed = pipeline.process_asset(asset).await?;
    report_compression(&processed.stats);
    report_encryption(&processed.stats);
    report_sharding(&processed);
    report_offline_limits(&processed)?;
    let reconstructed = pipeline.reconstruct_asset(&processed).await?;
    anyhow::ensure!(reconstructed == data, "Data integrity check failed");
    println!(
        "SUCCESS: Local reconstruction is byte-equal ({} bytes)",
        reconstructed.len()
    );
    println!("No network retrieval, Catalog ingestion or scoped PoS validation was tested.");
    Ok(())
}

fn demo_config() -> PipelineConfig {
    PipelineConfig {
        compression: CompressionConfig {
            algorithm: CompressionAlgorithm::Zstd,
            level: 3,
            ..Default::default()
        },
        sharding: ShardingConfig {
            data_shards: 10,
            parity_shards: 4,
            target_shard_size: 1024 * 1024,
        },
        ..Default::default()
    }
}

fn report_compression(stats: &PipelineStats) {
    println!("Stage 1 - Compression:");
    println!("   Original: {} bytes", stats.compression.original_size);
    println!("   Compressed: {} bytes", stats.compression.compressed_size);
    println!("   Compressed/original: {:.4}", stats.compression.ratio);
    println!(
        "   Time: {} ms (integer-ms resolution)",
        stats.compression.duration_ms
    );
}

fn report_encryption(stats: &PipelineStats) {
    println!("Stage 2 - Whole-blob encryption (before sharding):");
    println!("   Input: {} bytes", stats.encryption.original_size);
    println!(
        "   Ciphertext including authentication tag: {} bytes",
        stats.encryption.encrypted_size
    );
    println!("   Kyber KEM used: {}", stats.encryption.quantum_resistant);
    println!(
        "   Time: {} ms (integer-ms resolution)",
        stats.encryption.duration_ms
    );
}

fn report_sharding(processed: &ProcessedAsset) {
    let stats = &processed.stats;
    println!("Stage 3 - Reed-Solomon sharding (actual parameters):");
    println!("   Data shards: {}", stats.sharding.data_shards);
    println!("   Parity shards: {}", stats.sharding.parity_shards);
    println!("   Shards retained locally: {}", processed.shards.len());
    println!("   Total shard bytes: {}", stats.sharding.total_shard_size);
    println!(
        "   Redundancy factor: {:.2}",
        stats.sharding.redundancy_factor
    );
    println!(
        "   Time: {} ms (integer-ms resolution)",
        stats.sharding.duration_ms
    );
    println!(
        "Overall local transform: {} ms; {} shard bytes",
        stats.total_duration_ms, stats.final_size
    );
}

fn report_offline_limits(processed: &ProcessedAsset) -> anyhow::Result<()> {
    anyhow::ensure!(
        processed.distributed.placements.is_empty()
            && processed.stats.distribution.shards_distributed == 0,
        "Offline pipeline unexpectedly reported shard distribution"
    );
    println!("Distribution: NOT performed; no registered peers, dispatch or acknowledgements.");
    println!("Placement and PoS-gated retrieval belong to the live store/fetch paths.");
    println!(
        "The pipeline proof_hash is a metadata-derived placeholder, NOT a validated state proof."
    );
    Ok(())
}
