// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Catalog - Asset Package Registry
//!
//! The catalog provides asset package management for HyperMesh,
//! handling package definitions, versioning, and distribution.

pub mod integration;
pub mod peer_client;
pub mod provider;

pub use integration::{
    BridgeConfiguration, CatalogAssetType, CatalogDeploymentResult, CatalogDeploymentSpec,
    CatalogHyperMeshBridge, DeploymentStrategy,
};
pub use peer_client::{CatalogPeerClient, PeerSearchError, PeerSearchResult};
pub use provider::{
    CatalogDependencyGraph, CatalogDependencyNode, CatalogProvider, CatalogProviderError,
    CatalogTypeInfo,
};
