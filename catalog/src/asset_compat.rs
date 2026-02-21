// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Conversion functions between canonical `hypermesh_lib` asset types and
//! blockmatrix's domain-specific types.
//!
//! These are standalone functions (not trait impls) to avoid orphan-rule issues.

use hypermesh_lib::{AssetKind, SystemAssetKind, UserAssetKind, ContentHash};
use blockmatrix::assets::core::{AssetType, AssetCategory, BaseSystemType};

/// Convert canonical `AssetKind` -> blockmatrix `AssetType`.
///
/// `UserDefined` maps to `AssetType::Container` (executed as containers in blockmatrix runtime).
/// `Blockchain` and `Dns` map to `VirtualMachine` and `Library` respectively until
/// blockmatrix adds the dedicated variants.
pub fn asset_kind_to_bm_asset_type(kind: &AssetKind) -> AssetType {
    match kind {
        AssetKind::System(sys) => match sys {
            SystemAssetKind::Cpu => AssetType::Cpu,
            SystemAssetKind::Gpu => AssetType::Gpu,
            SystemAssetKind::Memory => AssetType::Memory,
            SystemAssetKind::Storage => AssetType::Storage,
            SystemAssetKind::Network => AssetType::Network,
            SystemAssetKind::Container => AssetType::Container,
            SystemAssetKind::Economic => AssetType::Economic,
            // Temporary mappings until blockmatrix adds Blockchain/Dns variants
            SystemAssetKind::Blockchain => AssetType::VirtualMachine,
            SystemAssetKind::Dns => AssetType::Library,
        },
        AssetKind::UserDefined(_) => AssetType::Container,
    }
}

/// Convert blockmatrix `AssetType` -> canonical `AssetKind`.
///
/// `VirtualMachine` maps to `Blockchain` and `Library` maps to `Dns` as temporary
/// reverse mappings until blockmatrix adds the dedicated variants.
pub fn bm_asset_type_to_asset_kind(bm_type: &AssetType) -> AssetKind {
    AssetKind::System(match bm_type {
        AssetType::Cpu => SystemAssetKind::Cpu,
        AssetType::Gpu => SystemAssetKind::Gpu,
        AssetType::Memory => SystemAssetKind::Memory,
        AssetType::Storage => SystemAssetKind::Storage,
        AssetType::Network => SystemAssetKind::Network,
        AssetType::Container => SystemAssetKind::Container,
        AssetType::Economic => SystemAssetKind::Economic,
        // Temporary reverse mappings until blockmatrix adds Blockchain/Dns variants
        AssetType::VirtualMachine => SystemAssetKind::Blockchain,
        AssetType::Library => SystemAssetKind::Dns,
    })
}

/// Convert blockmatrix `AssetCategory` -> canonical `AssetKind`.
pub fn bm_category_to_asset_kind(category: &AssetCategory) -> AssetKind {
    match category {
        AssetCategory::BaseSystem(base) => AssetKind::System(bm_base_to_system_kind(base)),
        AssetCategory::Application(app) => {
            let mut hash_bytes = [0u8; 32];
            hash_bytes.copy_from_slice(&app.domain_hash);
            AssetKind::UserDefined(UserAssetKind {
                type_name: app.domain_name.clone(),
                type_hash: ContentHash::from_bytes(hash_bytes),
            })
        }
    }
}

/// Convert blockmatrix `BaseSystemType` -> canonical `SystemAssetKind`.
///
/// `BaseSystemType::Blockchain` maps directly to `SystemAssetKind::Blockchain`.
/// `Dns` is not yet a `BaseSystemType` variant; it only exists as `SystemAssetKind`.
pub fn bm_base_to_system_kind(base: &BaseSystemType) -> SystemAssetKind {
    match base {
        BaseSystemType::Cpu => SystemAssetKind::Cpu,
        BaseSystemType::Gpu => SystemAssetKind::Gpu,
        BaseSystemType::Memory => SystemAssetKind::Memory,
        BaseSystemType::Storage => SystemAssetKind::Storage,
        BaseSystemType::Network => SystemAssetKind::Network,
        BaseSystemType::Container => SystemAssetKind::Container,
        BaseSystemType::Economic => SystemAssetKind::Economic,
        BaseSystemType::Blockchain => SystemAssetKind::Blockchain,
    }
}

/// Parse a string label into a canonical `AssetKind`.
///
/// Recognises the same labels as `HyperMeshAssetRegistry::map_asset_type` plus
/// canonical system kind names.
pub fn parse_asset_kind(s: &str) -> AssetKind {
    match s.to_lowercase().as_str() {
        "cpu" | "compute" => AssetKind::System(SystemAssetKind::Cpu),
        "gpu" => AssetKind::System(SystemAssetKind::Gpu),
        "memory" | "mem" | "ram" => AssetKind::System(SystemAssetKind::Memory),
        "storage" | "disk" => AssetKind::System(SystemAssetKind::Storage),
        "network" | "net" => AssetKind::System(SystemAssetKind::Network),
        "container" | "vm" | "virtual_machine" => AssetKind::System(SystemAssetKind::Container),
        "economic" | "token" | "wallet" => AssetKind::System(SystemAssetKind::Economic),
        "blockchain" | "chain" => AssetKind::System(SystemAssetKind::Blockchain),
        "dns" => AssetKind::System(SystemAssetKind::Dns),
        other => {
            // Treat anything else as a UserDefined type with a zeroed hash
            // (callers can fill in the real hash later).
            AssetKind::UserDefined(UserAssetKind {
                type_name: other.to_string(),
                type_hash: ContentHash::zeroed(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_system_kinds() {
        // All 9 system kinds have 1:1 mapping
        let kinds = [
            SystemAssetKind::Cpu,
            SystemAssetKind::Gpu,
            SystemAssetKind::Memory,
            SystemAssetKind::Storage,
            SystemAssetKind::Network,
            SystemAssetKind::Container,
            SystemAssetKind::Economic,
            SystemAssetKind::Blockchain,
            SystemAssetKind::Dns,
        ];

        for kind in &kinds {
            let asset_kind = AssetKind::System(*kind);
            let bm = asset_kind_to_bm_asset_type(&asset_kind);
            let back = bm_asset_type_to_asset_kind(&bm);
            assert_eq!(back, asset_kind, "Roundtrip failed for {:?}", kind);
        }
    }

    #[test]
    fn test_parse_asset_kind() {
        assert_eq!(
            parse_asset_kind("cpu"),
            AssetKind::System(SystemAssetKind::Cpu)
        );
        assert_eq!(
            parse_asset_kind("compute"),
            AssetKind::System(SystemAssetKind::Cpu)
        );
        assert_eq!(
            parse_asset_kind("GPU"),
            AssetKind::System(SystemAssetKind::Gpu)
        );
        assert_eq!(
            parse_asset_kind("dns"),
            AssetKind::System(SystemAssetKind::Dns)
        );

        // Unknown -> UserDefined
        match parse_asset_kind("custom_widget") {
            AssetKind::UserDefined(u) => assert_eq!(u.type_name, "custom_widget"),
            other => unreachable!("test: expected UserDefined, got {:?}", other),
        }
    }

    #[test]
    fn test_bm_category_to_asset_kind() {
        let base = AssetCategory::BaseSystem(BaseSystemType::Blockchain);
        let kind = bm_category_to_asset_kind(&base);
        assert_eq!(kind, AssetKind::System(SystemAssetKind::Blockchain));

        let app = AssetCategory::Application(blockmatrix::assets::core::ApplicationDomain {
            domain_name: "myapp".to_string(),
            domain_hash: [99u8; 32],
        });
        match bm_category_to_asset_kind(&app) {
            AssetKind::UserDefined(u) => {
                assert_eq!(u.type_name, "myapp");
                assert_eq!(u.type_hash, ContentHash::from_bytes([99u8; 32]));
            }
            other => unreachable!("test: expected UserDefined, got {:?}", other),
        }
    }
}
