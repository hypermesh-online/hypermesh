//! HyperMesh shared types and traits
//!
//! Canonical definitions for types shared across all HyperMesh crates.
//! Every crate should import shared types from here, not define their own.

pub mod types;
pub mod error;
pub mod crypto;

// Re-export commonly used types at crate root
pub use types::*;
pub use error::HypermeshError;
