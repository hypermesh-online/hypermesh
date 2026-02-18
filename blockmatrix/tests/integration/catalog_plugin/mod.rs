// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Common utilities and helpers for Catalog plugin integration tests
//!
//! NOTE: All tests in this module are marked #[ignore] because they require
//! the Catalog extension implementation which does not currently exist in BlockMatrix.
//! These tests are prepared for when Catalog is implemented.

use blockmatrix::assets::core::{AssetManager, AssetType, PrivacyLevel, AssetRegistration};
use blockmatrix::extensions::{
    ExtensionCapability, ExtensionConfig, ExtensionManager, ExtensionManagerConfig,
    ExtensionMetadata, ExtensionRequest, ExtensionResponse, ResourceLimits,
};
use blockmatrix::extensions::loader::{ExtensionLoader, LoaderConfig};
use blockmatrix::extensions::registry::{ExtensionRegistry, RegistryConfig, ExtensionLocation};
use blockmatrix::extensions::security::{
    SecurityManager, SecurityConfig, ResourceQuotas, ResourceUsage, IsolationLevel,
};
use blockmatrix::consensus::{ConsensusProof, ProofType};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use tracing::{info, debug, warn, error};
use serde_json::json;

// Re-export submodules
pub mod lifecycle;
pub mod integration;
pub mod operations;
pub mod reliability;

// ============= Common Test Utilities =============

/// Initialize test logging with debug level
pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("debug")
        .try_init();
}

/// Create a test loader with standard configuration
pub fn create_test_loader() -> ExtensionLoader {
    let config = LoaderConfig {
        search_paths: vec![
            PathBuf::from("../catalog/target/debug"),
            PathBuf::from("../catalog/target/release"),
        ],
        enable_wasm: false,
        verify_signatures: false,
        max_extensions: 10,
        default_limits: ResourceLimits::default(),
        trustchain_cert_path: None,
    };

    ExtensionLoader::new(config)
}

/// Create catalog extension metadata for testing
pub fn create_catalog_metadata() -> ExtensionMetadata {
    ExtensionMetadata {
        id: "catalog".to_string(),
        name: "Catalog Extension".to_string(),
        version: semver::Version::parse("1.0.0").unwrap(),
        description: "Asset library management".to_string(),
        author: "HyperMesh".to_string(),
        license: "MIT".to_string(),
        homepage: None,
        category: hypermesh::extensions::ExtensionCategory::AssetLibrary,
        hypermesh_version: semver::Version::parse("1.0.0").unwrap(),
        dependencies: vec![],
        required_capabilities: HashSet::from([
            ExtensionCapability::AssetManagement,
            ExtensionCapability::VMExecution,
            ExtensionCapability::NetworkAccess,
        ]),
        provided_assets: vec!["library".to_string(), "package".to_string()],
        certificate_fingerprint: None,
        config_schema: None,
    }
}
