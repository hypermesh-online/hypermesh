// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Test utilities for creating AssetRegistration instances in tests

use crate::assets::core::{
    ApplicationDomain, AssetCategory, AssetData, AssetRegistration, AssetType, BaseSystemType,
    NetworkScope,
};

/// Create a test AssetRegistration from an AssetType
pub fn test_asset_id(asset_type: AssetType) -> AssetRegistration {
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
        AssetType::Blockchain => AssetCategory::BaseSystem(BaseSystemType::Blockchain),
        AssetType::Dns => AssetCategory::BaseSystem(BaseSystemType::Dns),
        AssetType::Transmission => AssetCategory::BaseSystem(BaseSystemType::Transmission),
        AssetType::Dashboard => AssetCategory::BaseSystem(BaseSystemType::Dashboard),
        AssetType::Identity => AssetCategory::BaseSystem(BaseSystemType::Identity),
        AssetType::KeyRotation => AssetCategory::BaseSystem(BaseSystemType::KeyRotation),
        AssetType::Invitation => AssetCategory::BaseSystem(BaseSystemType::Invitation),
        AssetType::Message => AssetCategory::BaseSystem(BaseSystemType::Message),
    };
    AssetRegistration::from_asset_data(&data, NetworkScope::Global, category)
}

/// Create a vector of test AssetRegistrations for use in Block::new() tests.
/// The count parameter determines how many AssetRegistrations to generate.
pub fn test_asset_ids(count: usize) -> Vec<AssetRegistration> {
    (0..count.max(1))
        .map(|i| test_asset_id_with_content(AssetType::Storage, vec![i as u8]))
        .collect()
}

/// Create a test AssetRegistration with custom content
pub fn test_asset_id_with_content(asset_type: AssetType, content: Vec<u8>) -> AssetRegistration {
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
        AssetType::Blockchain => AssetCategory::BaseSystem(BaseSystemType::Blockchain),
        AssetType::Dns => AssetCategory::BaseSystem(BaseSystemType::Dns),
        AssetType::Transmission => AssetCategory::BaseSystem(BaseSystemType::Transmission),
        AssetType::Dashboard => AssetCategory::BaseSystem(BaseSystemType::Dashboard),
        AssetType::Identity => AssetCategory::BaseSystem(BaseSystemType::Identity),
        AssetType::KeyRotation => AssetCategory::BaseSystem(BaseSystemType::KeyRotation),
        AssetType::Invitation => AssetCategory::BaseSystem(BaseSystemType::Invitation),
        AssetType::Message => AssetCategory::BaseSystem(BaseSystemType::Message),
    };
    AssetRegistration::from_asset_data(&data, NetworkScope::Global, category)
}
