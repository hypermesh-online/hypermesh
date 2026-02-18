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

/// Whether participation is bounded (known group) or unbounded (open to all)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessScope {
    /// Known, finite group of participants
    Bounded,
    /// Open to any participant
    Unbounded,
}

impl fmt::Display for AccessScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessScope::Bounded => write!(f, "Bounded"),
            AccessScope::Unbounded => write!(f, "Unbounded"),
        }
    }
}

/// Two-axis privacy model: scope (bounded/unbounded) x tracked (yes/no)
///
/// Replaces the old 4-tier model (Anonymous/P2P/Federated/Public) with a
/// clearer 2-axis grid. Three named presets cover the common cases:
/// - `ANONYMOUS`: Unbounded + untracked (no identity, open participation)
/// - `PRIVATE`: Bounded + tracked (known group, identity required)
/// - `PUBLIC`: Unbounded + tracked (open participation, full transparency)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrivacyMode {
    pub scope: AccessScope,
    pub tracked: bool,
}

impl PrivacyMode {
    /// Unbounded scope, no tracking — maximum privacy
    pub const ANONYMOUS: Self = Self { scope: AccessScope::Unbounded, tracked: false };
    /// Bounded scope, tracked — known group with identity
    pub const PRIVATE: Self = Self { scope: AccessScope::Bounded, tracked: true };
    /// Unbounded scope, tracked — open participation, full transparency
    pub const PUBLIC: Self = Self { scope: AccessScope::Unbounded, tracked: true };

    /// CAESAR reward multiplier for this privacy mode
    pub fn caesar_multiplier(&self) -> f64 {
        match (self.scope, self.tracked) {
            (AccessScope::Unbounded, false) => 0.0,   // Anonymous: no rewards
            (AccessScope::Bounded, _) => 0.5,          // Private: medium rewards
            (AccessScope::Unbounded, true) => 1.0,     // Public: maximum rewards
        }
    }

    /// Whether this mode requires identity verification
    pub fn requires_identity(&self) -> bool {
        self.tracked
    }

    /// Whether this mode allows activity logging
    pub fn allows_logging(&self) -> bool {
        self.tracked
    }

    /// eBPF kernel representation (u8 for BPF maps)
    pub fn to_ebpf_u8(&self) -> u8 {
        match (self.scope, self.tracked) {
            (AccessScope::Unbounded, false) => 0, // Anonymous
            (AccessScope::Bounded, false) => 1,   // Bounded+untracked (rare)
            (AccessScope::Bounded, true) => 2,    // Private
            (AccessScope::Unbounded, true) => 3,  // Public
        }
    }

    /// Graduated timeout for connections with this privacy mode
    pub fn connection_timeout_secs(&self) -> u64 {
        match (self.scope, self.tracked) {
            (AccessScope::Unbounded, false) => 30,  // Anonymous: short
            (AccessScope::Bounded, _) => 90,         // Private: medium
            (AccessScope::Unbounded, true) => 300,   // Public: long
        }
    }
}

impl fmt::Display for PrivacyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.scope, self.tracked) {
            (AccessScope::Unbounded, false) => write!(f, "Anonymous"),
            (AccessScope::Bounded, true) => write!(f, "Private"),
            (AccessScope::Unbounded, true) => write!(f, "Public"),
            (AccessScope::Bounded, false) => write!(f, "Bounded(untracked)"),
        }
    }
}

impl Serialize for PrivacyMode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Serialize as the Display string for named presets, struct for custom
        match (self.scope, self.tracked) {
            (AccessScope::Unbounded, false) => serializer.serialize_str("Anonymous"),
            (AccessScope::Bounded, true) => serializer.serialize_str("Private"),
            (AccessScope::Unbounded, true) => serializer.serialize_str("Public"),
            _ => {
                use serde::ser::SerializeStruct;
                let mut s = serializer.serialize_struct("PrivacyMode", 2)?;
                s.serialize_field("scope", &self.scope)?;
                s.serialize_field("tracked", &self.tracked)?;
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for PrivacyMode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de;

        struct PrivacyModeVisitor;

        impl<'de> de::Visitor<'de> for PrivacyModeVisitor {
            type Value = PrivacyMode;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a privacy mode string or struct")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<PrivacyMode, E> {
                match v {
                    // Current canonical names
                    "Anonymous" => Ok(PrivacyMode::ANONYMOUS),
                    "Private" => Ok(PrivacyMode::PRIVATE),
                    "Public" => Ok(PrivacyMode::PUBLIC),
                    // Legacy variants from old enums
                    "P2P" | "PrivateP2P" => Ok(PrivacyMode::PRIVATE),
                    "Federated" | "PrivateNetwork" => Ok(PrivacyMode::PRIVATE),
                    "PublicNetwork" | "FullPublic" => Ok(PrivacyMode::PUBLIC),
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["Anonymous", "Private", "Public"],
                    )),
                }
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<PrivacyMode, A::Error> {
                let mut scope: Option<AccessScope> = None;
                let mut tracked: Option<bool> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "scope" => scope = Some(map.next_value()?),
                        "tracked" => tracked = Some(map.next_value()?),
                        _ => { let _ = map.next_value::<de::IgnoredAny>()?; }
                    }
                }
                Ok(PrivacyMode {
                    scope: scope.ok_or_else(|| de::Error::missing_field("scope"))?,
                    tracked: tracked.ok_or_else(|| de::Error::missing_field("tracked"))?,
                })
            }
        }

        deserializer.deserialize_any(PrivacyModeVisitor)
    }
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
    /// WHERE - storage location and physical/network location (PoSpace)
    Space,
    /// WHO - ownership, access rights, economic stake (PoStake)
    Stake,
    /// WHAT/HOW - computational resources and processing (PoWork)
    Work,
    /// WHEN - temporal ordering and timestamp validation (PoTime)
    Time,
}

impl fmt::Display for ProofType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProofType::Space => write!(f, "ProofOfSpace"),
            ProofType::Stake => write!(f, "ProofOfStake"),
            ProofType::Work => write!(f, "ProofOfWork"),
            ProofType::Time => write!(f, "ProofOfTime"),
        }
    }
}

impl fmt::Display for BlockchainScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockchainScope::Device => write!(f, "Device"),
            BlockchainScope::User => write!(f, "User"),
            BlockchainScope::Group => write!(f, "Group"),
            BlockchainScope::Organization => write!(f, "Organization"),
            BlockchainScope::Federation => write!(f, "Federation"),
            BlockchainScope::Public => write!(f, "Public"),
        }
    }
}

impl fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineStage::Compress => write!(f, "Compress"),
            PipelineStage::Encrypt => write!(f, "Encrypt"),
            PipelineStage::Shard => write!(f, "Shard"),
            PipelineStage::Distribute => write!(f, "Distribute"),
        }
    }
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
