// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! P4 behavior proof: the live store path places shards on REAL PoS-eligible
//! peer coordinates (not the deleted golden-ratio synthetic geometry), and a
//! stored asset still reconstructs byte-for-byte (store → fetch round trip).
//!
//! This exercises the actual store-path components:
//!   1. `AssetPipeline::process_asset_with_privacy` — the store side (shards).
//!   2. `network::placement::place_shards` — the single placement authority
//!      (`distribute_shards_pos_aware`: PoS eligibility → octant placement over
//!      the live peer set at proximity coordinates).
//!   3. `AssetPipeline::reconstruct_asset` — the fetch side (round trip).

use std::net::SocketAddr;

use blockmatrix::assets::pipeline::{Asset, AssetPipeline, PipelineInputMetadata};
use blockmatrix::matrix::MatrixCoordinate;
use blockmatrix::network::placement::place_shards;
use blockmatrix::network::NetworkNode;
use hypermesh_lib::PrivacyMode;

fn peer(node_id: &str, x: i64, y: i64, z: i64) -> NetworkNode {
    NetworkNode {
        coordinate: MatrixCoordinate::new(x, y, z).expect("valid coord"),
        address: "[::1]:9292".parse::<SocketAddr>().expect("valid addr"),
        node_id: node_id.to_string(),
        privacy_mode: PrivacyMode::PUBLIC,
        connection: None,
    }
}

#[tokio::test]
async fn live_store_path_places_on_real_peers_and_round_trips() {
    // Enough data to force real Reed-Solomon sharding into many shards.
    let original: Vec<u8> = b"HYPERMESH-P4-PLACEMENT-ROUNDTRIP-".repeat(4096);

    let asset = Asset {
        id: hex::encode(blake3::hash(&original).as_bytes()),
        data: original.clone(),
        metadata: PipelineInputMetadata {
            name: "p4.bin".to_string(),
            content_type: "application/octet-stream".to_string(),
            size: original.len(),
            created_at: 0,
            custom: std::collections::HashMap::new(),
        },
    };
    let asset_id = asset.id.clone();

    // STORE side: default pipeline (Private/encrypted) produces content-addressed
    // shards. The pipeline no longer computes placement (P4).
    let pipeline = AssetPipeline::default().expect("pipeline");
    let processed = pipeline
        .process_asset_with_privacy(asset, PrivacyMode::PRIVATE)
        .await
        .expect("process");
    assert!(
        processed.distributed.placements.is_empty(),
        "the pipeline must NOT fabricate placements — placement is the store path's job",
    );

    // WHERE: place over real peers at KNOWN, distinct coordinates. The deleted
    // synthetic engine would have returned golden-ratio sphere points at radius
    // 5..50; the real authority must return exactly these peer coordinates.
    let peers = vec![
        peer("peer-a", 12, 0, 0),
        peer("peer-b", -12, 0, 0),
        peer("peer-c", 0, 12, 0),
        peer("peer-d", 0, -12, 0),
    ];
    let real_coords: std::collections::HashSet<(i64, i64, i64)> = peers
        .iter()
        .map(|p| (p.coordinate.x, p.coordinate.y, p.coordinate.z))
        .collect();

    let placements =
        place_shards(&peers, &processed.shards, &asset_id, PrivacyMode::PRIVATE).await;

    assert!(
        !placements.is_empty(),
        "with eligible peers present, placement must occur",
    );
    for pl in &placements {
        let pos = (pl.position.x, pl.position.y, pl.position.z);
        assert!(
            real_coords.contains(&pos),
            "placement {pos:?} must be a REAL peer coordinate, never synthetic geometry",
        );
        assert!(
            ["peer-a", "peer-b", "peer-c", "peer-d"].contains(&pl.node_id.as_str()),
            "placement must name a real eligible peer, got {}",
            pl.node_id,
        );
    }

    // FETCH side: reconstruct through the reverse pipeline — content-addressing
    // and the store→fetch round trip must be byte-identical.
    let reconstructed = pipeline.reconstruct_asset(&processed).await.expect("reconstruct");
    assert_eq!(
        reconstructed, original,
        "store → fetch round trip must reproduce the original bytes exactly",
    );
}

/// Cold start (no connected peers): placement yields nothing and the asset is
/// kept local — no synthetic geometry is ever produced as a substitute.
#[tokio::test]
async fn cold_start_keeps_local_no_synthetic() {
    let original = b"cold-start-payload".repeat(1000);
    let asset = Asset {
        id: hex::encode(blake3::hash(&original).as_bytes()),
        data: original.clone(),
        metadata: PipelineInputMetadata {
            name: "cold.bin".to_string(),
            content_type: "application/octet-stream".to_string(),
            size: original.len(),
            created_at: 0,
            custom: std::collections::HashMap::new(),
        },
    };
    let asset_id = asset.id.clone();

    let pipeline = AssetPipeline::default().expect("pipeline");
    let processed = pipeline
        .process_asset_with_privacy(asset, PrivacyMode::PUBLIC)
        .await
        .expect("process");

    let placements = place_shards(&[], &processed.shards, &asset_id, PrivacyMode::PUBLIC).await;
    assert!(
        placements.is_empty(),
        "no peers → no remote placement (kept local), never fabricated positions",
    );
}
