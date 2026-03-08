// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Backward-compatibility facade for node blockchain types.
//!
//! The implementation has been split into focused modules:
//! - [`super::chain`] -- core chain state and queries
//! - [`super::mutations`] -- block addition and asset registration
//! - [`super::genesis_ops`] -- MFA genesis authentication helpers

pub use super::chain::*;
