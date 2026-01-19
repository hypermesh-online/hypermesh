//! Test utilities for creating AssetId instances in tests

#[cfg(test)]
pub mod asset_test_helpers {
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
}
