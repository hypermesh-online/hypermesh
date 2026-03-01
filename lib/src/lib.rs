// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh shared types and traits
//!
//! Canonical definitions for types shared across all HyperMesh crates.
//! Every crate should import shared types from here, not define their own.

pub mod asset;
pub mod crypto;
pub mod economic;
pub mod error;
pub mod http;
pub mod protocol;
pub mod types;

// Re-export commonly used types at crate root
pub use asset::*;
pub use economic::*;
pub use error::HypermeshError;
pub use protocol::*;
pub use types::*;
