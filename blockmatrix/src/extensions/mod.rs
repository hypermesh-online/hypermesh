// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Extension Interface Architecture
//!
//! This module defines the comprehensive plugin/extension system for HyperMesh,
//! allowing external components like Catalog to integrate as dynamic extensions
//! that provide specialized functionality while maintaining consensus validation
//! and security requirements.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    HyperMesh Core System                        │
//! │                                                                  │
//! │  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐         │
//! │  │   Asset     │  │   Consensus  │  │   Transport   │         │
//! │  │   Manager   │  │   (Proof of State)   │  │    (STOQ)     │         │
//! │  └──────┬──────┘  └──────┬───────┘  └───────┬───────┘         │
//! │         │                 │                   │                  │
//! │  ┌──────┴─────────────────┴───────────────────┴──────────┐     │
//! │  │              Extension Manager Runtime                  │     │
//! │  │                                                          │     │
//! │  │  • Dynamic Loading    • Dependency Resolution           │     │
//! │  │  • Lifecycle Control  • Security Sandboxing             │     │
//! │  │  • Resource Limits    • Consensus Validation            │     │
//! │  └──────────────────────────────────────────────────────────┘   │
//! └─────────────────────────┬────────────────────────────────────┘
//!                           │ Extension Interface
//!     ┌────────────────────┴────────────────────────┐
//!     │                                              │
//! ┌────┴──────┐  ┌──────────────┐  ┌───────────────┴───────────┐
//! │  Catalog  │  │   Custom     │  │     Future Extensions     │
//! │ Extension │  │  Extensions  │  │                           │
//! │           │  │              │  │  • Analytics Engine       │
//! │ • Assets  │  │ • User Apps  │  │  • Machine Learning       │
//! │ • Library │  │ • Protocols  │  │  • Specialized Compute    │
//! │ • VM/Julia│  │ • Services   │  │  • Domain-Specific Tools  │
//! └───────────┘  └──────────────┘  └──────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! All extensions operate under strict security constraints:
//! - Capability-based security with explicit permission grants
//! - Resource quotas and runtime limits
//! - Consensus validation for critical operations
//! - TrustChain certificate verification for signed extensions
//! - Isolated execution environments with controlled API access

#![deny(unsafe_code)]

// Submodules for extension system
pub mod loader;
pub mod manager;
pub mod registry;
pub mod security;

// Internal modules
mod asset_types;
mod extension_manager;
mod package_types;
mod traits;
mod types;

// Re-export main types
pub use types::{
    ExtensionCapability, ExtensionCategory, ExtensionConfig, ExtensionDependency, ExtensionError,
    ExtensionHealth, ExtensionManagerConfig, ExtensionMetadata, ExtensionRequest,
    ExtensionResponse, ExtensionResult, ExtensionState, ExtensionStateData, ExtensionStatus,
    ResourceLimits, ResourceUsageReport, ValidationError, ValidationReport, ValidationWarning,
};

pub use asset_types::{
    AssetCreationSpec, AssetMetadata, AssetOperation, AssetQuery, AssetUpdate,
    ConsensusRequirements, ConsensusStatus, CpuRequirement, DeploymentResult, DeploymentSpec,
    ExecutionResult, ExecutionSpec, GpuRequirement, MemoryRequirement, NetworkConfig,
    OperationResult, PortMapping, ResourceRequirements, SharingResult, SharingSpec,
    StorageRequirement, TransferResult, TransferSpec, VolumeMount,
};

pub use package_types::{
    AssetManifest, AssetPackage, AssetPackageSpec, InstallOptions, InstallResult,
    PackageDependency, PackageFilter, PublishResult, SearchOptions, SecurityIssue, UpdateResult,
    VerificationResult,
};

pub use traits::{AssetExtensionHandler, AssetLibraryExtension, HyperMeshExtension};

pub use extension_manager::ExtensionManager;
pub use manager::UnifiedExtensionManager;

/// Integration flow for Catalog as a HyperMesh extension
///
/// This demonstrates how Catalog would integrate as an extension:
///
/// 1. **Discovery Phase**:
///    - HyperMesh scans extension directories
///    - Finds Catalog extension manifest
///    - Validates signature with TrustChain
///
/// 2. **Loading Phase**:
///    - ExtensionManager loads Catalog extension
///    - Verifies dependencies (STOQ, TrustChain)
///    - Grants required capabilities
///
/// 3. **Initialization Phase**:
///    - Catalog initializes with configuration
///    - Registers asset types (VM, Container, Library)
///    - Extends AssetManager with catalog-specific operations
///
/// 4. **Registration Phase**:
///    - Catalog registers asset handlers for each type
///    - Sets up P2P distribution through STOQ
///    - Configures consensus validation requirements
///
/// 5. **Operation Phase**:
///    - Catalog handles asset library requests
///    - Manages package installation/updates
///    - Validates operations with consensus proofs
///    - Distributes assets through P2P network
///
/// 6. **Integration Points**:
///    - **Consensus**: All operations require Proof of State four-proof validation
///    - **TrustChain**: Package signatures and certificate validation
///    - **STOQ**: P2P distribution of asset packages
///    - **Proxy/NAT**: Remote asset access through NAT-like addressing
pub struct CatalogExtensionIntegration;

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;
    use std::collections::HashSet;

    #[test]
    fn test_extension_metadata() {
        let metadata = ExtensionMetadata {
            id: "catalog".to_string(),
            name: "HyperMesh Catalog".to_string(),
            version: Version::parse("1.0.0").expect("test: expected success"),
            description: "Decentralized asset library for HyperMesh".to_string(),
            author: "HyperMesh Team".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://hypermesh.online/catalog".to_string()),
            category: ExtensionCategory::AssetLibrary,
            hypermesh_version: Version::parse("1.0.0").expect("test: expected success"),
            dependencies: vec![],
            required_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::NetworkAccess,
                ExtensionCapability::ConsensusAccess,
                ExtensionCapability::TransportAccess,
            ]),
            provided_assets: vec![
                crate::assets::core::AssetType::Blockchain,
                crate::assets::core::AssetType::Container,
                crate::assets::core::AssetType::Dns,
            ],
            certificate_fingerprint: Some("SHA256:1234567890abcdef".to_string()),
            config_schema: None,
        };

        assert_eq!(metadata.id, "catalog");
        assert_eq!(metadata.category, ExtensionCategory::AssetLibrary);
        assert!(metadata
            .required_capabilities
            .contains(&ExtensionCapability::AssetManagement));
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_cpu_percent, 25.0);
        assert_eq!(limits.max_memory_bytes, 1024 * 1024 * 1024);
        assert_eq!(limits.max_concurrent_operations, 100);
    }

    #[test]
    fn test_consensus_requirements_default() {
        let reqs = ConsensusRequirements::default();
        assert!(reqs.require_proof_of_space);
        assert!(reqs.require_proof_of_stake);
        assert!(reqs.require_proof_of_work);
        assert!(reqs.require_proof_of_time);
        assert_eq!(reqs.min_space_commitment, Some(1024 * 1024));
    }
}
