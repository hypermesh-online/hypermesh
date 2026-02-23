// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Public SDK prelude — single import for all commonly used types.
//!
//! ```ignore
//! use hypermesh_lib::prelude::*;
//! ```

// Core identifiers
pub use crate::types::{NodeId, AssetId, NetworkId, ContentHash};

// Matrix topology
pub use crate::types::{MatrixPosition, PipelineStage};

// Privacy & scope
pub use crate::types::{PrivacyMode, AccessScope, BlockchainScope};

// Asset system
pub use crate::asset::{
    AssetKind, SystemAssetKind, UserAssetKind,
    BaseState, AssetAdapter, AssetStatusTrait, AssetMetadata,
    AdapterCapabilities, AdapterError, ValidationOutcome,
};

// Proof of State
pub use crate::proof::{
    ProofOfState, SpaceProof, StakeProof, WorkProof, TimeProof,
    WorkCategory, ProofValidationResult, Validatable,
};
pub use crate::types::ProofType;

// Economic (Caesar EVP)
pub use crate::economic::{
    PacketId, GoldGrams, MarketTier, PacketState, DemurrageRate,
};

// Binary codec
pub use crate::codec::{encode, decode};

// Error
pub use crate::error::HypermeshError;

// Crypto
pub use crate::crypto::{CryptoAlgorithm, KeyPairId};
