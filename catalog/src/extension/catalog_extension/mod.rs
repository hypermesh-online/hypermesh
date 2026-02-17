// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CatalogExtension - Main HyperMesh Extension Implementation
//!
//! Split into submodules:
//! - `types`: CatalogExtension struct definition, constructor, internal helpers
//! - `handlers`: HyperMeshExtension trait implementation and request routing
//! - `lifecycle`: AssetLibraryExtension trait implementation (package operations)

pub mod types;
pub mod handlers;
pub mod lifecycle;

pub use types::CatalogExtension;

#[cfg(test)]
mod tests {
    use super::*;
    use blockmatrix::extensions::{ExtensionCapability, ExtensionCategory};
    use crate::extension::config::CatalogExtensionConfig;

    #[tokio::test]
    async fn test_catalog_extension_creation() {
        let mut config = CatalogExtensionConfig::default();
        config.cache_size = 100;
        let extension = CatalogExtension::new(config);

        assert_eq!(extension.metadata.id, "catalog");
        assert_eq!(extension.metadata.category, ExtensionCategory::AssetLibrary);
        assert_eq!(extension.metadata.provided_assets.len(), 4);
    }

    #[tokio::test]
    async fn test_extension_metadata() {
        let mut config = CatalogExtensionConfig::default();
        config.cache_size = 100;
        let extension = CatalogExtension::new(config);
        let metadata = extension.metadata.clone();

        assert!(metadata.required_capabilities.contains(&ExtensionCapability::AssetManagement));
        assert!(metadata.required_capabilities.contains(&ExtensionCapability::VMExecution));
        assert!(metadata.required_capabilities.contains(&ExtensionCapability::NetworkAccess));
    }

    #[tokio::test]
    async fn test_extension_status() {
        use blockmatrix::extensions::HyperMeshExtension;

        let mut config = CatalogExtensionConfig::default();
        config.cache_size = 100;
        let extension = CatalogExtension::new(config);
        let status = extension.status().await;

        assert_eq!(status.total_requests, 0);
        assert_eq!(status.error_count, 0);
        assert_eq!(status.active_operations, 0);
    }
}
