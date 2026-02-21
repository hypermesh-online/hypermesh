// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! NAT-like Address Translation System for HyperMesh
//!
//! CRITICAL COMPONENT: Implements the core NAT-like memory addressing system
//! that enables remote memory access via IPv6-like global addresses.

pub mod types;
pub mod routing;
pub mod translation;

// Re-export all public types
pub use types::*;

// Re-export the translator
pub use translation::NATTranslator;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::AssetType;
    use crate::test_utils::test_asset_id;

    #[test]
    fn test_global_address_creation() {
        let asset_id = test_asset_id(AssetType::Memory);
        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
            &asset_id,
            8080,
            GlobalAddressType::Memory,
        );

        assert_eq!(global_addr.service_port, 8080);
        assert!(matches!(global_addr.address_type, GlobalAddressType::Memory));
    }

    #[test]
    fn test_global_address_string_conversion() {
        let asset_id = test_asset_id(AssetType::Memory);
        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
            &asset_id,
            8080,
            GlobalAddressType::Memory,
        );

        let addr_str = global_addr.to_string();
        assert!(addr_str.starts_with("hypermesh://"));
        assert!(addr_str.contains("8080"));
    }

    #[tokio::test]
    async fn test_nat_translator_creation() {
        let translator = NATTranslator::new().await.expect("test");
        let stats = translator.get_stats().await.expect("test");
        assert_eq!(stats.total_translations, 0);
        assert_eq!(stats.active_translations, 0);
    }

    #[tokio::test]
    async fn test_translation_creation() {
        let translator = NATTranslator::new().await.expect("test");
        let asset_id = test_asset_id(AssetType::Memory);

        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
            &asset_id,
            8080,
            GlobalAddressType::Memory,
        );

        let permissions = MemoryPermissions {
            read: true,
            write: true,
            execute: false,
            share: false,
            cache: true,
            prefetch: true,
        };

        let mapping = translator.create_translation(
            global_addr.clone(),
            1024 * 1024, // 1MB
            permissions,
        ).await.expect("test");

        assert_eq!(mapping.region_size, 1024 * 1024);
        assert!(matches!(mapping.translation_state, TranslationState::Active));

        // Test address translation
        let local_addr = translator.translate_to_local(&global_addr).await.expect("test");
        assert_eq!(local_addr, mapping.local_address);

        // Test reverse translation
        let reverse_global = translator.translate_to_global(local_addr).await.expect("test");
        assert_eq!(reverse_global.hash(), global_addr.hash());
    }

    #[tokio::test]
    async fn test_real_memory_mapping() {
        let translator = NATTranslator::new().await.expect("test");
        let asset_id = test_asset_id(AssetType::Memory);

        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
            &asset_id,
            8081,
            GlobalAddressType::Memory,
        );

        let permissions = MemoryPermissions {
            read: true,
            write: true,
            execute: false,
            share: false,
            cache: true,
            prefetch: false,
        };

        let mapping = translator.create_translation(
            global_addr.clone(),
            4096, // 4KB page
            permissions,
        ).await.expect("test");

        let local_ptr = mapping.local_address as *mut u8;
        #[allow(unsafe_code)]
        unsafe {
            *local_ptr = 42;
            assert_eq!(*local_ptr, 42);

            for i in 0..256 {
                *local_ptr.add(i) = i as u8;
            }

            for i in 0..256 {
                assert_eq!(*local_ptr.add(i), i as u8);
            }
        }

        translator.remove_translation(&global_addr).await.expect("test");

        let stats = translator.get_stats().await.expect("test");
        assert_eq!(stats.active_translations, 0);
    }

    #[tokio::test]
    async fn test_translation_with_privacy() {
        let translator = NATTranslator::new().await.expect("test");
        let asset_id = test_asset_id(AssetType::Memory);

        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22],
            &asset_id,
            8082,
            GlobalAddressType::Memory,
        );

        let permissions = MemoryPermissions {
            read: true,
            write: false,
            execute: false,
            share: true,
            cache: true,
            prefetch: false,
        };

        let privacy_config = PrivacyConfig {
            level: PrivacyMode::PRIVATE,
            allowed_networks: vec![],
            allowed_peers: vec![],
            max_concurrent_access: 5,
            require_consensus: false,
        };

        let mapping = translator.create_translation_with_privacy(
            global_addr.clone(),
            8192, // 8KB
            permissions,
            privacy_config.clone(),
        ).await.expect("test");

        assert!(mapping.privacy_config.is_some());
        let attached_privacy = mapping.privacy_config.expect("test");
        assert_eq!(attached_privacy.level, PrivacyMode::PRIVATE);
        assert_eq!(attached_privacy.max_concurrent_access, 5);

        translator.remove_translation(&global_addr).await.expect("test");
    }

    #[tokio::test]
    async fn test_invalid_privacy_config() {
        let translator = NATTranslator::new().await.expect("test");
        let asset_id = test_asset_id(AssetType::Memory);

        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11],
            &asset_id,
            8083,
            GlobalAddressType::Memory,
        );

        let permissions = MemoryPermissions {
            read: true,
            write: true,
            execute: false,
            share: false,
            cache: false,
            prefetch: false,
        };

        // PRIVATE level should not have allowed networks or peers
        let invalid_privacy = PrivacyConfig {
            level: PrivacyMode::PRIVATE,
            allowed_networks: vec!["some-net".to_string()],
            allowed_peers: vec![],
            max_concurrent_access: 1,
            require_consensus: false,
        };

        let result = translator.create_translation_with_privacy(
            global_addr,
            4096,
            permissions,
            invalid_privacy,
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Private level should not have allowed networks or peers"));
    }
}
