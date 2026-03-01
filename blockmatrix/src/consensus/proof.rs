// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Proof module re-exports for backward compatibility with nested imports
//!
//! This module exists to support legacy code that imports from crate::consensus::proof::*
//! All types are re-exported from the parent consensus module.

// Re-export all proof types from parent module
pub use super::{
    // BlockMatrix-specific types
    AccessLevel,
    AccessPermissions,
    Consensus,
    ConsensusError,
    // Core consensus proof
    ConsensusProof,

    LogIndex,

    NetworkPosition,
    Proof,
    SpaceProof,
    // Individual proof types
    StakeProof,
    TimeProof,
    WorkProof,

    WorkState,
    // Additional types from TrustChain
    WorkloadType,
};
