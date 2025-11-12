// HyperMesh Shared Library
// Common types, traits, and utilities used across all components

pub mod assets;
pub mod consensus;
pub mod common;

pub use assets::{AssetId, AssetType, AssetMetadata};
pub use consensus::{ConsensusProof, ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime};
