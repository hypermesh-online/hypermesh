// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Proof module re-exports for nested imports
//!
//! This module supports code that imports from crate::proof_of_state::proof::*

// Re-export all proof types from parent module
pub use super::{
    // BlockMatrix-specific types
    AccessLevel,
    AccessPermissions,
    ProofOfState,
    StateProofError,
    // Core state proof
    StateProof,
    LogIndex,
    NetworkPosition,
    Proof,
    SpaceProof,
    // Individual proof types
    StakeProof,
    TimeProof,
    WorkProof,
};
