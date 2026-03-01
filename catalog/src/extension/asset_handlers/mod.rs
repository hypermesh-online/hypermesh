// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset Handlers for Catalog Extension
//!
//! This module implements specific asset handlers for each asset type
//! that Catalog manages within the HyperMesh ecosystem.

mod dataset;
mod library;
mod template;
mod virtual_machine;

pub use dataset::DatasetHandler;
pub use library::LibraryHandler;
pub use template::TemplateHandler;
pub use virtual_machine::VirtualMachineHandler;

pub(crate) use async_trait::async_trait;
pub(crate) use blockmatrix::extensions::{
    AssetCreationSpec, AssetExtensionHandler, ExtensionAssetRecord, AssetOperation, AssetQuery,
    AssetUpdate, DeploymentResult, ExecutionResult, ExtensionError, ExtensionResult,
    OperationResult, ResourceUsageReport,
};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;

pub(crate) use blockmatrix::assets::core::{
    ApplicationDomain, AssetCategory, AssetData, AssetRegistration, AssetType, NetworkScope,
};
pub(crate) use blockmatrix::consensus::proof_of_state_integration::ConsensusProof;
// BLAKE3 used via blake3::hash() for domain hashes
