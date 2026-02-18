// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

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
    ConsensusProofVM, VMConfig, ConsensusRequirements, AssetRegistration,
    PrivacyLevel, ResourceSharingConfig, AssetAllocation,
};

pub use integration::{
    CatalogHyperMeshBridge, CatalogDeploymentSpec, CatalogDeploymentResult,
    CatalogAssetType, DeploymentStrategy, BridgeConfiguration,
};