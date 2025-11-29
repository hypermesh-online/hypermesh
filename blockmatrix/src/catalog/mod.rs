//! HyperMesh Catalog - VM Integration
//!
//! The catalog provides the core virtual machine systems for HyperMesh,
//! integrating Julia VM execution with consensus validation.

pub mod vm;
pub mod integration;

// Re-exports for convenience
pub use vm::{
    ConsensusProofVM, VMConfig, ConsensusRequirements,
    PrivacyLevel, ResourceSharingConfig,
};

pub use integration::{
    CatalogHyperMeshBridge, CatalogDeploymentSpec, CatalogAssetType,
    DeploymentStrategy, BridgeConfiguration, CatalogDeploymentResult,
};