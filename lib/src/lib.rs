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
pub mod encoding;
pub mod error;
pub mod http;
pub mod proof;
pub mod protocol;
pub mod runtime;
pub mod sdk;
pub mod types;
pub mod validation;

/// Test utilities available to other crates via `features = ["test-utils"]`.
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

// Re-export commonly used types at crate root
pub use asset::*;
pub use economic::*;
pub use encoding::{decode, encode, encode_bounded, EncodingError};
pub use error::HypermeshError;
pub use proof::{ProofOfState, ProofValidationResult, Validatable};
pub use protocol::*;
pub use types::*;
