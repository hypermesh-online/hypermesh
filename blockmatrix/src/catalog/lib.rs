// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Catalog Library
//!
//! Asset package management for HyperMesh, handling package definitions,
//! versioning, distribution, and deployment orchestration.

pub mod integration;

pub use integration::{
    CatalogHyperMeshBridge, CatalogDeploymentSpec, CatalogDeploymentResult,
    CatalogAssetType, DeploymentStrategy, BridgeConfiguration,
};
