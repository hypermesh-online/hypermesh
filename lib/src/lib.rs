//! HyperMesh Shared Library
//! Common types, traits, and utilities used across all components
//!
//! This library contains:
//! - Asset system (universal asset types and IDs)
//! - Common utilities and error types
//!
//! Note: Consensus/Proof of State system is in blockmatrix crate

pub mod assets;
pub mod common;

// Re-export commonly used types
pub use assets::{AssetId, AssetType, AssetMetadata};
