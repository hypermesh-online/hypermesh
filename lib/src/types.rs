// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Core shared type definitions

use serde::{Serialize, Deserialize};
use std::fmt;

/// Unique node identifier in the Block-MATRIX topology
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self { NodeId(s) }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self { NodeId(s.to_string()) }
}

/// Unique asset identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetId(pub String);

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for AssetId {
    fn from(s: String) -> Self { AssetId(s) }
}

impl From<&str> for AssetId {
    fn from(s: &str) -> Self { AssetId(s.to_string()) }
}

/// Network privacy tiers (STOQ transport layer)
/// Controls packet tracking and identity disclosure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkPrivacyTier {
    /// No identity tracking, privacy-first
    Anonymous,
    /// Peer-to-peer, minimal tracking
    P2P,
    /// Trusted network groups, selective sharing
    Federated,
    /// Full transparency, maximum rewards
    Public,
}

/// Blockchain consensus scopes (independent from network privacy)
/// Controls who participates in consensus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockchainScope {
    /// Single device, local-only
    Device,
    /// User's devices share blockchain
    User,
    /// Small trusted groups
    Group,
    /// Companies, teams
    Organization,
    /// Multi-org collaboration
    Federation,
    /// Global public blockchain
    Public,
}

/// Proof of State proof types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProofType {
    /// WHERE - storage location and physical/network location
    PoSpace,
    /// WHO - ownership, access rights, economic stake
    PoStake,
    /// WHAT/HOW - computational resources and processing
    PoWork,
    /// WHEN - temporal ordering and timestamp validation
    PoTime,
}

/// Matrix coordinate in Block-MATRIX topology
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MatrixPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Asset pipeline stage ordering
/// CORRECT ORDER: Compress -> Encrypt -> Shard -> Distribute
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineStage {
    /// Stage 1: Brotli compression
    Compress,
    /// Stage 2: Kyber-1024 encryption (whole blob)
    Encrypt,
    /// Stage 3: Reed-Solomon erasure coding
    Shard,
    /// Stage 4: Tensor-based matrix distribution
    Distribute,
}
