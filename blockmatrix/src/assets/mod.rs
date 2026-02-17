// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

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
pub mod pipeline;
pub mod storage;

// Re-export main types for easy access
pub use core::{
    AssetManager, AssetId, AssetType, AssetAllocation,
    ConsensusProof, SpaceProof, StakeProof, WorkProof, TimeProof,
    WorkloadType, WorkState, PrivacyLevel, AssetStatistics, AssetAdapter, AssetError,
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

#[cfg(feature = "multi-node")]
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

pub use pipeline::{
    AssetPipeline, PipelineConfig, ProcessedAsset, PipelineStats,
    Compressor, Encryptor, Sharder, MatrixDistributor,
};

pub use storage::{
    ContentAddressedStorage, StorageStats, Hash,
    HashBucket, BucketId, BucketMapper,
    DeduplicationEngine, DeduplicationResult, DeduplicationStats,
    ContentAddress, RetrievalInstructions,
    ReplicationStrategy, ReplicationConfig, PopularityMetrics,
};