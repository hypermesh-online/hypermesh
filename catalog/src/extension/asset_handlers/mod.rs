// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset Handlers for Catalog Extension
//!
//! This module implements specific asset handlers for each asset type
//! that Catalog manages within the HyperMesh ecosystem.

mod virtual_machine;
mod library;
mod dataset;
mod template;

pub use virtual_machine::VirtualMachineHandler;
pub use library::LibraryHandler;
pub use dataset::DatasetHandler;
pub use template::TemplateHandler;

pub(crate) use async_trait::async_trait;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use blockmatrix::extensions::{
    AssetExtensionHandler, ExtensionResult, ExtensionError,
    AssetCreationSpec, AssetUpdate, AssetQuery, AssetMetadata,
    AssetOperation, OperationResult, DeploymentResult, ExecutionResult,
    ResourceUsageReport,
};

pub(crate) use blockmatrix::assets::core::{AssetRegistration, AssetType, AssetData, NetworkScope, AssetCategory, ApplicationDomain};
pub(crate) use blockmatrix::consensus::proof_of_state_integration::ConsensusProof;
// BLAKE3 used via blake3::hash() for domain hashes
