//! HyperMesh Asset Core Module
//!
//! Core functionality for the universal asset system.
//!
//! # Module Organization
//!
//! - `types` - Core type definitions (Asset, AssetType, etc.)
//! - `management` - Asset registration and management
//! - `layer` - Main asset layer implementation

pub mod layer;
pub mod management;
pub mod types;

// Re-export main types
pub use layer::HyperMeshAssetLayer;
pub use management::{AssetManager, AssetSystemStats};
pub use types::{
    Asset, AssetId, AssetLocation, AssetStatistics, AssetStatus,
    AssetType, AllocationId, PrivacyLevel, ResourceAllocation,
};