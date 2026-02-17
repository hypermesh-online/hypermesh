// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Test utilities for creating AssetId instances in tests

use crate::assets::core::{AssetId, AssetType, AssetData, NetworkScope, AssetCategory, BaseSystemType, ApplicationDomain};

/// Create a test AssetId from an AssetType
pub fn test_asset_id(asset_type: AssetType) -> AssetId {
    let data = AssetData {
        config: vec![1, 2, 3],
        definition: vec![4, 5, 6],
        metadata: vec![7, 8, 9],
    };
    let category = match asset_type {
        AssetType::Cpu => AssetCategory::BaseSystem(BaseSystemType::Cpu),
        AssetType::Gpu => AssetCategory::BaseSystem(BaseSystemType::Gpu),
        AssetType::Memory => AssetCategory::BaseSystem(BaseSystemType::Memory),
        AssetType::Storage => AssetCategory::BaseSystem(BaseSystemType::Storage),
        AssetType::Network => AssetCategory::BaseSystem(BaseSystemType::Network),
        AssetType::Container => AssetCategory::BaseSystem(BaseSystemType::Container),
        AssetType::Economic => AssetCategory::BaseSystem(BaseSystemType::Economic),
        AssetType::Library => AssetCategory::Application(ApplicationDomain {
            domain_name: "test".to_string(),
            domain_hash: [0u8; 32],
        }),
        AssetType::VirtualMachine => AssetCategory::BaseSystem(BaseSystemType::Container),
    };
    AssetId::from_asset_data(&data, NetworkScope::Global, category)
}

/// Create a vector of test AssetIds for use in Block::new() tests.
/// The count parameter determines how many AssetIds to generate.
pub fn test_asset_ids(count: usize) -> Vec<AssetId> {
    (0..count.max(1)).map(|i| {
        test_asset_id_with_content(AssetType::Storage, vec![i as u8])
    }).collect()
}

/// Create a test AssetId with custom content
pub fn test_asset_id_with_content(asset_type: AssetType, content: Vec<u8>) -> AssetId {
    let data = AssetData {
        config: content.clone(),
        definition: content.clone(),
        metadata: content,
    };
    let category = match asset_type {
        AssetType::Cpu => AssetCategory::BaseSystem(BaseSystemType::Cpu),
        AssetType::Gpu => AssetCategory::BaseSystem(BaseSystemType::Gpu),
        AssetType::Memory => AssetCategory::BaseSystem(BaseSystemType::Memory),
        AssetType::Storage => AssetCategory::BaseSystem(BaseSystemType::Storage),
        AssetType::Network => AssetCategory::BaseSystem(BaseSystemType::Network),
        AssetType::Container => AssetCategory::BaseSystem(BaseSystemType::Container),
        AssetType::Economic => AssetCategory::BaseSystem(BaseSystemType::Economic),
        AssetType::Library => AssetCategory::Application(ApplicationDomain {
            domain_name: "test".to_string(),
            domain_hash: [0u8; 32],
        }),
        AssetType::VirtualMachine => AssetCategory::BaseSystem(BaseSystemType::Container),
    };
    AssetId::from_asset_data(&data, NetworkScope::Global, category)
}
