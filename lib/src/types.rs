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

/// Unique network identifier (128-bit, compatible with UUID bytes)
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NetworkId(pub [u8; 16]);

impl fmt::Display for NetworkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// Content hash (256-bit BLAKE3 digest) used across blockchain operations
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        ContentHash(bytes)
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create zeroed hash (for defaults/tests)
    pub fn zeroed() -> Self {
        ContentHash([0u8; 32])
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0[..8] {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "...")
    }
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

/// Blockchain operating mode — independent from network privacy (PrivacyMode)
///
/// A node always runs a Device chain. It can optionally sync with a Network.
/// PrivacyMode (Anonymous/Private/Public) controls WHO participates.
/// BlockchainScope controls WHETHER chains synchronize.
///
/// Sub-federation (groups within orgs within federations) is handled by
/// nested Private networks, not separate scope variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockchainScope {
    /// Single device, local-only blockchain — always running from boot
    Device,
    /// Synchronized across participating nodes — reflector/swarm mode
    Network,
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
            BlockchainScope::Network => write!(f, "Network"),
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

/// HyperMesh ULA prefix: fd48:4d00 (fd + 'H'=0x48, 'M'=0x4d + reserved)
pub const HYPERMESH_PREFIX: [u8; 4] = [0xfd, 0x48, 0x4d, 0x00];

/// Errors that can occur when constructing or parsing an `AssetAddress`.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AddressError {
    #[error("Coordinate {axis} overflow: {value} exceeds i16 range")]
    CoordinateOverflow { axis: &'static str, value: i64 },
    #[error("Shard index {0} exceeds maximum 15")]
    InvalidShardIndex(u8),
    #[error("Not a HyperMesh address (wrong prefix)")]
    NotHyperMesh,
}

/// Routable IPv6 address for any HyperMesh asset.
///
/// Layout (128 bits):
/// ```text
/// fd48:4d00:XXXX:YYYY:ZZZZ:AAAA:AAAA:AASS
/// ├──────┤ ├──┤ ├──────────────┤ ├──────────────────┤
/// prefix  net   matrix coords    asset fingerprint
/// (16b)  (16b)  (48b: 3×i16)    (48b: 44b hash + 4b shard)
/// ```
///
/// - Bytes 0-3: `fd48:4d00` ULA prefix
/// - Bytes 4-9: Matrix x,y,z as i16 big-endian
/// - Bytes 10-14: First 5 bytes of BLAKE3 content hash (40 bits)
/// - Byte 15: High nibble = hash byte 5 high nibble (4 bits), Low nibble = shard index (0-15)
/// - Shard 0 = whole asset, 1-14 = Reed-Solomon shards
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetAddress([u8; 16]);

impl AssetAddress {
    /// Create address for whole asset (shard=0) at given matrix position.
    /// x, y, z are i64 but must fit in i16 range [-32768, 32767].
    pub fn new(
        x: i64,
        y: i64,
        z: i64,
        content_hash: &ContentHash,
    ) -> Result<Self, AddressError> {
        Self::with_shard(x, y, z, content_hash, 0)
    }

    /// Create address with specific shard index (0-15).
    pub fn with_shard(
        x: i64,
        y: i64,
        z: i64,
        content_hash: &ContentHash,
        shard: u8,
    ) -> Result<Self, AddressError> {
        let xi = Self::check_coord(x, "x")?;
        let yi = Self::check_coord(y, "y")?;
        let zi = Self::check_coord(z, "z")?;

        if shard > 15 {
            return Err(AddressError::InvalidShardIndex(shard));
        }

        let mut buf = [0u8; 16];
        // Prefix
        buf[0..4].copy_from_slice(&HYPERMESH_PREFIX);
        // Coordinates as i16 big-endian
        buf[4..6].copy_from_slice(&xi.to_be_bytes());
        buf[6..8].copy_from_slice(&yi.to_be_bytes());
        buf[8..10].copy_from_slice(&zi.to_be_bytes());
        // Asset fingerprint: first 5 bytes of content hash
        let hash_bytes = content_hash.as_bytes();
        buf[10..15].copy_from_slice(&hash_bytes[..5]);
        // Byte 15: high nibble from hash byte 5, low nibble is shard index
        buf[15] = (hash_bytes[5] & 0xF0) | (shard & 0x0F);

        Ok(AssetAddress(buf))
    }

    fn check_coord(val: i64, axis: &'static str) -> Result<i16, AddressError> {
        i16::try_from(val).map_err(|_| AddressError::CoordinateOverflow {
            axis,
            value: val,
        })
    }

    /// Convert to std::net::Ipv6Addr
    pub fn to_ipv6(&self) -> std::net::Ipv6Addr {
        std::net::Ipv6Addr::from(self.0)
    }

    /// Parse from Ipv6Addr, validating HyperMesh prefix
    pub fn from_ipv6(addr: std::net::Ipv6Addr) -> Result<Self, AddressError> {
        let octets = addr.octets();
        if octets[0..4] != HYPERMESH_PREFIX {
            return Err(AddressError::NotHyperMesh);
        }
        Ok(AssetAddress(octets))
    }

    /// Extract matrix coordinates as (x, y, z) i64 tuple
    pub fn matrix_coords(&self) -> (i64, i64, i64) {
        let x = i16::from_be_bytes([self.0[4], self.0[5]]) as i64;
        let y = i16::from_be_bytes([self.0[6], self.0[7]]) as i64;
        let z = i16::from_be_bytes([self.0[8], self.0[9]]) as i64;
        (x, y, z)
    }

    /// Extract asset fingerprint (bytes 10-15, all 6 bytes)
    pub fn asset_fingerprint(&self) -> [u8; 6] {
        let mut fp = [0u8; 6];
        fp.copy_from_slice(&self.0[10..16]);
        fp
    }

    /// Get shard index (low nibble of byte 15)
    pub fn shard_index(&self) -> u8 {
        self.0[15] & 0x0F
    }

    /// Get parent address (shard=0) by clearing shard nibble
    pub fn parent(&self) -> Self {
        let mut buf = self.0;
        buf[15] &= 0xF0;
        AssetAddress(buf)
    }

    /// Derive sub-address for a specific shard
    pub fn shard(&self, index: u8) -> Result<Self, AddressError> {
        if index > 15 {
            return Err(AddressError::InvalidShardIndex(index));
        }
        let mut buf = self.0;
        buf[15] = (buf[15] & 0xF0) | (index & 0x0F);
        Ok(AssetAddress(buf))
    }

    /// Check if this is a HyperMesh address
    pub fn is_hypermesh(&self) -> bool {
        self.0[0..4] == HYPERMESH_PREFIX
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Create from raw bytes (no validation)
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        AssetAddress(bytes)
    }
}

impl fmt::Display for AssetAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_ipv6())
    }
}
