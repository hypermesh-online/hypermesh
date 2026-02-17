// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Matrix-aware distribution and placement logic
//!
//! Handles shard placement calculations based on Block-MATRIX topology

use crate::assets::core::ProxyAddress;
use crate::assets::core::AssetId;

/// Generate proxy address for storage access
pub async fn generate_proxy_address(asset_id: &AssetId) -> ProxyAddress {
    let mut node_id = [0u8; 8];
    node_id.copy_from_slice(&asset_id.content_hash[..8]);
    ProxyAddress::new(
        [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
        node_id,
        8080
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::{AssetData, NetworkScope, AssetCategory, BaseSystemType};

    #[tokio::test]
    async fn test_generate_proxy_address() {
        let data = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };
        let asset_id = AssetId::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Storage),
        );
        let proxy_addr = generate_proxy_address(&asset_id).await;

        // Verify proxy address structure
        assert_eq!(proxy_addr.asset_port, 8080);
    }
}
