//! HyperMesh Shared Library
//! Common types, traits, and utilities used across all components
//!
//! This library contains:
//! - Asset system (universal asset types and IDs)
//! - Proof of State consensus system (formerly "consensus")
//! - Common utilities and error types

pub mod assets;
pub mod proof_of_state;  // The full consensus/Proof of State system
pub mod common;

// Re-export commonly used types
pub use assets::{AssetId, AssetType, AssetMetadata};

// Re-export consensus module with both names for compatibility
pub use proof_of_state as consensus;
pub use proof_of_state::{ConsensusProof, ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime};
