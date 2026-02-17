// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog Extension Module - HyperMesh Plugin Implementation
//!
//! This module implements the CatalogExtension that allows Catalog to function
//! as a dynamic extension/plugin within the HyperMesh ecosystem, providing
//! specialized asset library and package management functionality.

pub mod catalog_extension;
pub mod asset_handlers;
pub mod config;

pub use catalog_extension::CatalogExtension;
pub use asset_handlers::{
    VirtualMachineHandler,
    LibraryHandler,
    DatasetHandler,
    TemplateHandler,
};
pub use config::{CatalogExtensionConfig, ExtensionSettings};

// Re-export key types for convenience
pub use blockmatrix::extensions::{
    HyperMeshExtension,
    AssetLibraryExtension,
    ExtensionMetadata,
    ExtensionCategory,
    ExtensionCapability,
    ExtensionResult,
    ExtensionError,
};