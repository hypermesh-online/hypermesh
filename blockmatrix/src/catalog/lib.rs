//! HyperMesh Catalog Library
//!
//! A blockchain-native compute integration system for JuliaVM and other language
//! runtimes, providing direct blockchain storage without smart contract abstraction.

pub mod blockchain;
pub mod vm;
pub mod integration;

// Re-export main types
pub use blockchain::{
    BlockchainNativeCompute, ComputeAsset, ComputeRequest, ExecutionResult,
    P2PHost, MatrixRouter, CaesarTokenManager, ComputeAssetType,
    PaymentToken, ResourcePayment,
};

pub use vm::{
    ConsensusProofVM, VMConfig, ConsensusRequirements, AssetId,
    PrivacyLevel, ResourceSharingConfig, AssetAllocation,
};

pub use integration::{
    CatalogHyperMeshBridge, CatalogDeploymentSpec, CatalogDeploymentResult,
    CatalogAssetType, DeploymentStrategy, BridgeConfiguration,
};