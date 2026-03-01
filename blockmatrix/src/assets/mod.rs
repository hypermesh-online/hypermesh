// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Asset Management System
//!
//! This module provides the core asset management system for HyperMesh,
//! treating all resources as assets with consensus proof validation.

pub mod adapters;
pub mod blockchain;
pub mod core;
pub mod cross_chain;
pub mod matrix_blockchain;
pub mod multi_node;
pub mod pipeline;
pub mod privacy;
pub mod proxy;
pub mod storage;

// Re-export main types for easy access
pub use core::{
    AssetAdapter, AssetAllocation, AssetError, AssetManager, AssetRegistration, AssetStatistics,
    AssetType, ConsensusProof, PrivacyMode, SpaceProof, StakeProof, TimeProof, WorkProof,
    WorkState, WorkloadType,
};

pub use adapters::{CpuAssetAdapter, GpuAssetAdapter, MemoryAssetAdapter, StorageAssetAdapter};

pub use proxy::{ProxyAddress, ProxyNetworkConfig, RemoteProxyManager};

pub use privacy::{PrivacyManager, ResourceAllocation};

#[cfg(feature = "multi-node")]
pub use multi_node::{
    ConsensusDecision, ConsensusManager, MultiNodeCoordinator, NetworkTopology, NodeCapabilities,
    NodeInfo,
};

pub use blockchain::{AssetBlockchainManager, AssetRecordType, HyperMeshAssetRecord};

pub use matrix_blockchain::{
    BlockchainMatrixCoordinate, EntityBlockchain, EntityType, MatrixBlockchainManager,
};

pub use pipeline::{
    AssetPipeline, Compressor, Encryptor, MatrixDistributor, PipelineConfig, PipelineStats,
    ProcessedAsset, Sharder,
};

pub use storage::{
    BucketId, BucketMapper, ContentAddress, ContentAddressedStorage, DeduplicationEngine,
    DeduplicationResult, DeduplicationStats, Hash, HashBucket, PopularityMetrics,
    ReplicationConfig, ReplicationStrategy, RetrievalInstructions, StorageStats,
};
