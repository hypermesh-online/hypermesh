// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Conversion functions between canonical `hypermesh_lib` asset types and
//! blockmatrix's domain-specific types.
//!
//! These are standalone functions (not trait impls) to avoid orphan-rule issues.

use blockmatrix::assets::core::{AssetCategory, AssetType, BaseSystemType};
use hypermesh_lib::{AssetKind, ContentHash, SystemAssetKind, UserAssetKind};

/// Convert canonical `AssetKind` -> blockmatrix `AssetType`.
///
/// `UserDefined` maps to `AssetType::Container` (executed as containers in blockmatrix runtime).
pub fn _asset_kind_to_bm_asset_type(kind: &AssetKind) -> AssetType {
    match kind {
        AssetKind::System(sys) => match sys {
            SystemAssetKind::Cpu => AssetType::Cpu,
            SystemAssetKind::Gpu => AssetType::Gpu,
            SystemAssetKind::Memory => AssetType::Memory,
            SystemAssetKind::Storage => AssetType::Storage,
            SystemAssetKind::Network => AssetType::Network,
            SystemAssetKind::Container => AssetType::Container,
            SystemAssetKind::Economic => AssetType::Economic,
            SystemAssetKind::Blockchain => AssetType::Blockchain,
            SystemAssetKind::Dns => AssetType::Dns,
            SystemAssetKind::Transmission => AssetType::Transmission,
            SystemAssetKind::Dashboard => AssetType::Dashboard,
            SystemAssetKind::Identity => AssetType::Identity,
            SystemAssetKind::KeyRotation => AssetType::KeyRotation,
        },
        AssetKind::UserDefined(_) => AssetType::Container,
    }
}

/// Convert blockmatrix `AssetType` -> canonical `AssetKind`.
pub fn _bm_asset_type_to_asset_kind(bm_type: &AssetType) -> AssetKind {
    AssetKind::System(match bm_type {
        AssetType::Cpu => SystemAssetKind::Cpu,
        AssetType::Gpu => SystemAssetKind::Gpu,
        AssetType::Memory => SystemAssetKind::Memory,
        AssetType::Storage => SystemAssetKind::Storage,
        AssetType::Network => SystemAssetKind::Network,
        AssetType::Container => SystemAssetKind::Container,
        AssetType::Economic => SystemAssetKind::Economic,
        AssetType::Blockchain => SystemAssetKind::Blockchain,
        AssetType::Dns => SystemAssetKind::Dns,
        AssetType::Transmission => SystemAssetKind::Transmission,
        AssetType::Dashboard => SystemAssetKind::Dashboard,
        AssetType::Identity => SystemAssetKind::Identity,
        AssetType::KeyRotation => SystemAssetKind::KeyRotation,
    })
}

/// Convert blockmatrix `AssetCategory` -> canonical `AssetKind`.
pub fn _bm_category_to_asset_kind(category: &AssetCategory) -> AssetKind {
    match category {
        AssetCategory::BaseSystem(base) => AssetKind::System(_bm_base_to_system_kind(base)),
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
pub fn _bm_base_to_system_kind(base: &BaseSystemType) -> SystemAssetKind {
    match base {
        BaseSystemType::Cpu => SystemAssetKind::Cpu,
        BaseSystemType::Gpu => SystemAssetKind::Gpu,
        BaseSystemType::Memory => SystemAssetKind::Memory,
        BaseSystemType::Storage => SystemAssetKind::Storage,
        BaseSystemType::Network => SystemAssetKind::Network,
        BaseSystemType::Container => SystemAssetKind::Container,
        BaseSystemType::Economic => SystemAssetKind::Economic,
        BaseSystemType::Blockchain => SystemAssetKind::Blockchain,
        BaseSystemType::Dns => SystemAssetKind::Dns,
        BaseSystemType::Transmission => SystemAssetKind::Transmission,
        BaseSystemType::Dashboard => SystemAssetKind::Dashboard,
        BaseSystemType::Identity => SystemAssetKind::Identity,
        BaseSystemType::KeyRotation => SystemAssetKind::KeyRotation,
    }
}

/// Parse a string label into a canonical `AssetKind`.
///
/// Recognises the same labels as `HyperMeshAssetRegistry::map_asset_type` plus
/// canonical system kind names.
pub fn _parse_asset_kind(s: &str) -> AssetKind {
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
        "transmission" | "relay" | "bandwidth" => {
            AssetKind::System(SystemAssetKind::Transmission)
        }
        "dashboard" => AssetKind::System(SystemAssetKind::Dashboard),
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
        // All 10 system kinds have 1:1 mapping
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
            SystemAssetKind::Transmission,
        ];

        for kind in &kinds {
            let asset_kind = AssetKind::System(*kind);
            let bm = _asset_kind_to_bm_asset_type(&asset_kind);
            let back = _bm_asset_type_to_asset_kind(&bm);
            assert_eq!(back, asset_kind, "Roundtrip failed for {kind:?}");
        }
    }

    #[test]
    fn test_parse_asset_kind() {
        assert_eq!(
            _parse_asset_kind("cpu"),
            AssetKind::System(SystemAssetKind::Cpu)
        );
        assert_eq!(
            _parse_asset_kind("compute"),
            AssetKind::System(SystemAssetKind::Cpu)
        );
        assert_eq!(
            _parse_asset_kind("GPU"),
            AssetKind::System(SystemAssetKind::Gpu)
        );
        assert_eq!(
            _parse_asset_kind("dns"),
            AssetKind::System(SystemAssetKind::Dns)
        );
        assert_eq!(
            _parse_asset_kind("transmission"),
            AssetKind::System(SystemAssetKind::Transmission)
        );
        assert_eq!(
            _parse_asset_kind("relay"),
            AssetKind::System(SystemAssetKind::Transmission)
        );

        // Unknown -> UserDefined
        match _parse_asset_kind("custom_widget") {
            AssetKind::UserDefined(u) => assert_eq!(u.type_name, "custom_widget"),
            other => unreachable!("test: expected UserDefined, got {:?}", other),
        }
    }

    #[test]
    fn test_bm_category_to_asset_kind() {
        let base = AssetCategory::BaseSystem(BaseSystemType::Blockchain);
        let kind = _bm_category_to_asset_kind(&base);
        assert_eq!(kind, AssetKind::System(SystemAssetKind::Blockchain));

        let app = AssetCategory::Application(blockmatrix::assets::core::ApplicationDomain {
            domain_name: "myapp".to_string(),
            domain_hash: [99u8; 32],
        });
        match _bm_category_to_asset_kind(&app) {
            AssetKind::UserDefined(u) => {
                assert_eq!(u.type_name, "myapp");
                assert_eq!(u.type_hash, ContentHash::from_bytes([99u8; 32]));
            }
            other => unreachable!("test: expected UserDefined, got {:?}", other),
        }
    }
}
