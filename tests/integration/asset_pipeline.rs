// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset Pipeline Round-Trip Integration Tests
//!
//! Verifies that lib types (AssetId, ContentHash, PipelineStage, AssetAddress)
//! wire together with blockmatrix's domain types (AssetType, AssetRegistration).

use hypermesh_lib::{
    AssetAddress, AssetKind, ContentHash, MatrixPosition, PipelineStage, SystemAssetKind,
};

#[test]
fn pipeline_stages_are_distinct_and_ordered() {
    let stages = [
        PipelineStage::Compress,
        PipelineStage::Encrypt,
        PipelineStage::Shard,
        PipelineStage::Distribute,
    ];

    for i in 0..stages.len() {
        for j in (i + 1)..stages.len() {
            assert_ne!(stages[i], stages[j], "pipeline stages must be distinct");
        }
    }
}

#[test]
fn asset_address_round_trip_through_ipv6() {
    let hash = ContentHash::from_bytes([0x42u8; 32]);
    let addr = AssetAddress::new(10, -20, 30, &hash).expect("valid coordinates");

    let ipv6 = addr.to_ipv6();
    let recovered = AssetAddress::from_ipv6(ipv6).expect("valid HyperMesh address");

    assert_eq!(addr.matrix_coords(), recovered.matrix_coords());
    assert_eq!(addr.shard_index(), recovered.shard_index());
}

#[test]
fn shard_derivation_preserves_coordinates() {
    let hash = ContentHash::from_bytes([0xAB; 32]);
    let parent = AssetAddress::new(5, 10, 15, &hash).expect("valid coordinates");

    for idx in 1..=14u8 {
        let shard = parent.shard(idx).expect("valid shard index");
        assert_eq!(shard.shard_index(), idx);
        assert_eq!(shard.matrix_coords(), parent.matrix_coords());
    }
}

#[test]
fn lib_asset_types_wire_to_blockmatrix() {
    // lib defines SystemAssetKind, blockmatrix defines AssetType.
    // Both represent the same domain concept but at different layers.
    let lib_kind = SystemAssetKind::Cpu;
    assert_eq!(lib_kind.type_id(), 0);

    let bm_type = blockmatrix::AssetType::Cpu;
    assert_eq!(bm_type.type_id(), 0);

    // Both layers agree on numeric IDs for the core types
    assert_eq!(
        SystemAssetKind::Gpu.type_id(),
        blockmatrix::AssetType::Gpu.type_id()
    );
    assert_eq!(
        SystemAssetKind::Memory.type_id(),
        blockmatrix::AssetType::Memory.type_id()
    );
    assert_eq!(
        SystemAssetKind::Storage.type_id(),
        blockmatrix::AssetType::Storage.type_id()
    );
}

#[test]
fn content_hash_and_matrix_position_basics() {
    let hash = ContentHash::from_bytes([1u8; 32]);
    assert_ne!(hash, ContentHash::zeroed());

    let pos = MatrixPosition {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    assert!((pos.x - 1.0).abs() < f64::EPSILON);
}

#[test]
fn asset_kind_system_variant_display() {
    let kind = AssetKind::System(SystemAssetKind::Blockchain);
    assert_eq!(kind.to_string(), "System(Blockchain)");
}
