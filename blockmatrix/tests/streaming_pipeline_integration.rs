// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for the streaming-first asset pipeline.

use blockmatrix::assets::pipeline::{
    compression::CompressionAlgorithm,
    orchestrator::DecryptionKey,
    sharding::Shard,
    streaming_pipeline::{StreamingAssetPipeline, StreamingPipelineConfig},
    PipelineInputMetadata,
};

fn test_metadata() -> PipelineInputMetadata {
    PipelineInputMetadata {
        name: "integration-test.bin".to_string(),
        content_type: "application/octet-stream".to_string(),
        size: 0,
        created_at: 0,
        custom: std::collections::HashMap::new(),
    }
}

#[test]
fn test_full_e2e_process_and_reconstruct() {
    let data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
    let config = StreamingPipelineConfig {
        segment_size: 20_000,
        compression: CompressionAlgorithm::Zstd,
        compression_level: 3,
        rs_data_shards: 4,
        rs_parity_shards: 2,
        ..Default::default()
    };
    let pipeline = StreamingAssetPipeline::new(config).expect("test: pipeline");
    let (manifest, key, shard_sets) = pipeline
        .process_segmented(&data, &test_metadata())
        .expect("test: process");

    assert_eq!(manifest.segment_count, 5);
    assert_eq!(manifest.version, 1);
    assert!(manifest.inline_index.is_some()); // 5 <= 22

    let all_shards: Vec<Vec<Shard>> = shard_sets.into_iter().map(|s| s.shards).collect();
    let reconstructed = pipeline
        .reconstruct_segmented(&manifest, &key, &all_shards)
        .expect("test: reconstruct");

    assert_eq!(reconstructed, data);
}

#[test]
fn test_range_access() {
    let data: Vec<u8> = (0..50_000).map(|i| (i % 256) as u8).collect();
    let config = StreamingPipelineConfig {
        segment_size: 10_000,
        compression: CompressionAlgorithm::None,
        rs_data_shards: 4,
        rs_parity_shards: 2,
        ..Default::default()
    };
    let pipeline = StreamingAssetPipeline::new(config).expect("test: pipeline");
    let (manifest, key, shard_sets) = pipeline
        .process_segmented(&data, &test_metadata())
        .expect("test: process");

    // Read bytes 25000-35000 (segments 2 and 3)
    let range_shards = vec![
        (2u32, shard_sets[2].shards.clone()),
        (3u32, shard_sets[3].shards.clone()),
    ];
    let result = pipeline
        .reconstruct_range(&manifest, &key, &range_shards, 25000..35000)
        .expect("test: range");

    assert_eq!(result, &data[25000..35000]);
}

#[test]
fn test_backward_compat_old_decryption_key() {
    // Verify that the old AssetPipeline still works with DecryptionKey::Kyber
    use blockmatrix::assets::pipeline::{orchestrator::AssetPipeline, Asset};

    let data = b"Old pipeline backward compatibility test data".to_vec();
    let asset = Asset {
        id: "compat-test".to_string(),
        data,
        metadata: test_metadata(),
    };

    let rt = tokio::runtime::Runtime::new().expect("test: runtime");
    rt.block_on(async {
        let pipeline = AssetPipeline::default().expect("test: old pipeline");
        let processed = pipeline.process_asset(asset).await.expect("test: process");

        // Verify it uses the old Kyber variant (not KyberSegmented)
        match &processed.decryption_key {
            DecryptionKey::Kyber { .. } => {} // expected
            other => unreachable!(
                "test: expected Kyber, got {:?}",
                std::mem::discriminant(other)
            ),
        }

        let reconstructed = pipeline
            .reconstruct_asset(&processed)
            .await
            .expect("test: reconstruct");
        assert_eq!(
            reconstructed,
            b"Old pipeline backward compatibility test data"
        );
    });
}

#[tokio::test]
async fn test_streaming_e2e_process_and_reconstruct_to_writer() {
    let data: Vec<u8> = (0..200_000).map(|i| (i % 256) as u8).collect();
    let config = StreamingPipelineConfig {
        segment_size: 40_000,
        compression: CompressionAlgorithm::Zstd,
        compression_level: 1,
        rs_data_shards: 4,
        rs_parity_shards: 2,
        ..Default::default()
    };
    let pipeline = StreamingAssetPipeline::new(config).expect("test: pipeline");
    let meta = test_metadata();

    // Process via async reader
    let cursor = tokio::io::BufReader::new(&data[..]);
    let (manifest, key, shard_sets) = pipeline
        .process_stream(cursor, data.len() as u64, &meta)
        .await
        .expect("test: stream process");

    assert_eq!(manifest.segment_count, 5);

    // Reconstruct via async writer
    let all_shards: Vec<Vec<Shard>> = shard_sets.into_iter().map(|s| s.shards).collect();
    let mut output = Vec::new();
    pipeline
        .reconstruct_to_writer(&manifest, &key, &all_shards, &mut output)
        .await
        .expect("test: reconstruct to writer");

    assert_eq!(output, data);
}
