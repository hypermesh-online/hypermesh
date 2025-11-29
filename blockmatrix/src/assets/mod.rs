//! HyperMesh Asset Management System
//!
//! This module provides the core asset management system for HyperMesh,
//! treating all resources as assets with consensus proof validation.

pub mod core;
pub mod adapters;
pub mod proxy;
pub mod privacy;
pub mod multi_node;
pub mod blockchain;
pub mod matrix_blockchain;
pub mod cross_chain;

// Re-export main types for easy access
pub use core::{
    AssetManager, AssetId, AssetType, AssetAllocation,
    ConsensusProof, PrivacyLevel, AssetStatistics, AssetAdapter, AssetError,
};

pub use adapters::{
    CpuAssetAdapter, GpuAssetAdapter, MemoryAssetAdapter, StorageAssetAdapter,
};

pub use proxy::{
    ProxyAddress, RemoteProxyManager, ProxyNetworkConfig,
};

pub use privacy::{
    PrivacyManager, ResourceAllocation,
};

pub use multi_node::{
    MultiNodeCoordinator, NodeInfo, NodeCapabilities,
    ConsensusManager, ConsensusDecision, NetworkTopology,
};

pub use blockchain::{
    HyperMeshAssetRecord, AssetRecordType, AssetPrivacyLevel,
    AssetBlockchainManager,
};

pub use matrix_blockchain::{
    MatrixCoordinate, EntityBlockchain, EntityType,
    MatrixBlockchainManager,
};