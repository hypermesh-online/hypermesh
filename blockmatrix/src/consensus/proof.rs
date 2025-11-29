//! Proof module re-exports for backward compatibility with nested imports
//!
//! This module exists to support legacy code that imports from crate::consensus::proof::*
//! All types are re-exported from the parent consensus module.

// Re-export all proof types from parent module
pub use super::{
    // Core consensus proof
    ConsensusProof,

    // Individual proof types
    StakeProof,
    TimeProof,
    SpaceProof,
    WorkProof,

    // BlockMatrix-specific types
    AccessLevel,
    NetworkPosition,
    AccessPermissions,
    ConsensusError,
    Consensus,
    LogIndex,

    // Additional types from TrustChain
    WorkloadType,
    WorkState,
    Proof,
};