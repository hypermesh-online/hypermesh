// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Core shared type definitions

use serde::{Deserialize, Serialize};
use std::fmt;

/// Cryptographic node identity -- BLAKE3 hash of FALCON-1024 public key.
///
/// Derivation: hardware assessment -> BLAKE3(capabilities) -> FALCON-1024 keypair
/// -> BLAKE3(pubkey) -> NodeId
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 32]);

impl NodeId {
    /// Derive a NodeId from a FALCON-1024 public key by BLAKE3-hashing it.
    pub fn from_public_key(pubkey_bytes: &[u8]) -> Self {
        let hash = blake3::hash(pubkey_bytes);
        NodeId(*hash.as_bytes())
    }

    /// Create from raw 32-byte identity.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        NodeId(bytes)
    }

    /// Get the raw 32-byte identity.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Hex-encode the full 32-byte identity.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Create an all-zero NodeId (for defaults/tests).
    pub fn zeroed() -> Self {
        NodeId([0u8; 32])
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Show first 8 hex chars (4 bytes) followed by ellipsis
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}\u{2026}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
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
    fn from(s: String) -> Self {
        AssetId(s)
    }
}

impl From<&str> for AssetId {
    fn from(s: &str) -> Self {
        AssetId(s.to_string())
    }
}

/// Unique network identifier (128-bit, compatible with UUID bytes)
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NetworkId(pub [u8; 16]);

/// The single implicit world — the flat, global partition that is the *only*
/// world until worlds actually form (VISION.md §5.5: NGauge decides placement
/// per-world; worlds are emergent, nestable networks).
///
/// This lives next to [`NetworkId`] because it is a `NetworkId` sentinel and
/// `lib` is the universal dependency every world-keyed structure shares
/// (ngauge, blockmatrix, feature-gated or not) — one definition, zero
/// dependency/feature coupling. NGauge owns the *policy* of worlds; `lib` owns
/// the *type* and this default value.
///
/// Keying a map by `(GLOBAL_WORLD, k)` while only ever inserting this value is
/// byte-for-byte identical to the pre-world flat `k`-keyed map: one implicit
/// world = today's flat set. It is the seam P5 later uses to mount per-world
/// isolation without changing behavior here.
pub const GLOBAL_WORLD: NetworkId = NetworkId([0u8; 16]);

impl fmt::Display for NetworkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
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
            write!(f, "{byte:02x}")?;
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
    pub const ANONYMOUS: Self = Self {
        scope: AccessScope::Unbounded,
        tracked: false,
    };
    /// Bounded scope, tracked — known group with identity
    pub const PRIVATE: Self = Self {
        scope: AccessScope::Bounded,
        tracked: true,
    };
    /// Unbounded scope, tracked — open participation, full transparency
    pub const PUBLIC: Self = Self {
        scope: AccessScope::Unbounded,
        tracked: true,
    };

    /// CAESAR reward multiplier for this privacy mode
    pub fn caesar_multiplier(&self) -> f64 {
        match (self.scope, self.tracked) {
            (AccessScope::Unbounded, false) => 0.0, // Anonymous: no rewards
            (AccessScope::Bounded, _) => 0.5,       // Private: medium rewards
            (AccessScope::Unbounded, true) => 1.0,  // Public: maximum rewards
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
            (AccessScope::Unbounded, false) => 30, // Anonymous: short
            (AccessScope::Bounded, _) => 90,       // Private: medium
            (AccessScope::Unbounded, true) => 300, // Public: long
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
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
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

// ---------------------------------------------------------------------------
// ScopedIdentity types
// ---------------------------------------------------------------------------

/// What kind of workload this identity represents.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkloadType {
    /// Physical/virtual node in the mesh
    Node,
    /// Long-running service on a node
    Service,
    /// Autonomous agent operating on behalf of a node
    Agent,
}

impl fmt::Display for WorkloadType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkloadType::Node => write!(f, "Node"),
            WorkloadType::Service => write!(f, "Service"),
            WorkloadType::Agent => write!(f, "Agent"),
        }
    }
}

/// How this identity is scoped and traced.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct IdentityScope {
    /// Blockchain scope: Device (local only) or Network (distributed)
    pub blockchain_scope: BlockchainScope,
    /// Whether this identity is traceable (Anonymous = false, Private/Public = true)
    pub tracked: bool,
}

impl IdentityScope {
    /// Anonymous device scope: local chain, untracked.
    pub fn anonymous_device() -> Self {
        Self {
            blockchain_scope: BlockchainScope::Device,
            tracked: false,
        }
    }

    /// Private network scope: synced chain, tracked.
    pub fn private_network() -> Self {
        Self {
            blockchain_scope: BlockchainScope::Network,
            tracked: true,
        }
    }

    /// Public network scope: synced chain, tracked.
    pub fn public_network() -> Self {
        Self {
            blockchain_scope: BlockchainScope::Network,
            tracked: true,
        }
    }
}

impl fmt::Display for IdentityScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tracking = if self.tracked { "tracked" } else { "untracked" };
        write!(f, "{}:{}", self.blockchain_scope, tracking)
    }
}

/// A fully-qualified identity with scope awareness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedIdentity {
    /// The cryptographic node identity
    pub node_id: NodeId,
    /// What kind of workload this identity represents
    pub workload_type: WorkloadType,
    /// How this identity is scoped
    pub scope: IdentityScope,
    /// Optional human-readable label (not authoritative)
    pub label: Option<String>,
}

impl ScopedIdentity {
    /// Create a Node workload identity with the given scope.
    pub fn new_node(node_id: NodeId, scope: IdentityScope) -> Self {
        Self {
            node_id,
            workload_type: WorkloadType::Node,
            scope,
            label: None,
        }
    }

    /// Create a Node workload identity with a label.
    pub fn new_node_with_label(
        node_id: NodeId,
        scope: IdentityScope,
        label: impl Into<String>,
    ) -> Self {
        Self {
            node_id,
            workload_type: WorkloadType::Node,
            scope,
            label: Some(label.into()),
        }
    }
}

impl fmt::Display for ScopedIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}@{}]", self.workload_type, self.node_id, self.scope)?;
        if let Some(ref label) = self.label {
            write!(f, " \"{}\"", label)?;
        }
        Ok(())
    }
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
    pub fn new(x: i64, y: i64, z: i64, content_hash: &ContentHash) -> Result<Self, AddressError> {
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
        i16::try_from(val).map_err(|_| AddressError::CoordinateOverflow { axis, value: val })
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

// ===========================================================================
// Identity and Proof Traits (API boundaries between crates)
// ===========================================================================

/// Trait for signing data with a node's FALCON-1024 identity key.
///
/// Implemented by TrustChain's FalconIdentity, consumed by STOQ's bilateral
/// handshake protocol. This trait lives in lib so STOQ can accept any signer
/// without depending on TrustChain (which would create a circular dependency).
pub trait NodeSigner: Send + Sync {
    /// The node's unique identifier (BLAKE3 hex of FALCON-1024 public key).
    fn node_id(&self) -> &str;

    /// Raw FALCON-1024 public key bytes for identity verification.
    fn public_key_bytes(&self) -> &[u8];

    /// Sign arbitrary data with the node's FALCON-1024 secret key.
    fn sign(&self, data: &[u8]) -> anyhow::Result<Vec<u8>>;

    /// Verify a FALCON-1024 signature against a public key.
    fn verify_signature(pubkey: &[u8], data: &[u8], signature: &[u8]) -> anyhow::Result<bool>
    where
        Self: Sized;

    /// Return serialized key rotation entries, empty if no rotations.
    ///
    /// Nodes that have rotated keys include their rotation chain so peers
    /// can verify identity continuity from genesis pubkey to current pubkey.
    /// Default returns empty -- nodes that haven't rotated don't send anything.
    fn rotation_chain(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Trait for Kyber-1024 KEM encryption operations on a node's identity.
///
/// Each node has a persistent Kyber-1024 keypair alongside its FALCON-1024
/// signing keypair. Together they form the node's dual-key identity:
/// - FALCON pubkey proves provenance (WHO created/sent something)
/// - Kyber pubkey enables access control (encrypt FOR this node, issue tokens to authorize decryption)
///
/// Implemented by TrustChain's `NodeIdentityKeys`, consumed by the asset pipeline
/// and handshake protocol.
pub trait NodeEncryptor: Send + Sync {
    /// Raw Kyber-1024 public key bytes for KEM encapsulation.
    fn encryption_public_key(&self) -> &[u8];

    /// Decapsulate a KEM ciphertext to recover the shared secret.
    ///
    /// Used when someone has encrypted an asset for this node — the node
    /// decapsulates to get the AES key, then can issue tokens (re-encrypted
    /// shared secrets) to authorize specific peers to decrypt.
    fn decapsulate(&self, kem_ciphertext: &[u8]) -> anyhow::Result<Vec<u8>>;
}

/// Trait for providing and validating Proof of State data during handshakes.
///
/// Implemented by BlockMatrix (which has blockchain state), consumed by STOQ
/// (which performs the bilateral handshake at protocol layer). STOQ sees proofs
/// as opaque bytes — it never imports BlockMatrix or TrustChain types directly.
#[async_trait::async_trait]
pub trait StateProofProvider: Send + Sync {
    /// Generate serialized state proof bytes for this node.
    ///
    /// The proof contains all four sub-proofs (PoSpace, PoStake, PoWork, PoTime)
    /// serialized into a single byte vector.
    async fn generate_proof(&self) -> anyhow::Result<Vec<u8>>;

    /// Validate serialized state proof bytes received from a peer.
    ///
    /// Returns true if all four sub-proofs pass validation.
    async fn validate_proof(&self, proof_bytes: &[u8]) -> anyhow::Result<bool>;
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- NodeId ---

    #[test]
    fn node_id_from_public_key() {
        let pubkey = b"test-falcon-1024-public-key-data";
        let id1 = NodeId::from_public_key(pubkey);
        let id2 = NodeId::from_public_key(pubkey);
        assert_eq!(id1, id2, "from_public_key must be deterministic");

        // Different key must produce different id
        let id3 = NodeId::from_public_key(b"different-key");
        assert_ne!(id1, id3);
    }

    #[test]
    fn node_id_from_bytes_roundtrip() {
        let bytes = [0xAB; 32];
        let id = NodeId::from_bytes(bytes);
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn node_id_display() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0xDE;
        bytes[1] = 0xAD;
        bytes[2] = 0xBE;
        bytes[3] = 0xEF;
        let id = NodeId::from_bytes(bytes);
        let display = format!("{}", id);
        assert_eq!(display, "deadbeef\u{2026}");
    }

    #[test]
    fn node_id_to_hex() {
        let id = NodeId::from_bytes([0xFF; 32]);
        let hex = id.to_hex();
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c == 'f'));
    }

    #[test]
    fn node_id_zeroed() {
        let id = NodeId::zeroed();
        assert_eq!(id.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn node_id_serde_roundtrip() {
        let id = NodeId::from_public_key(b"test-key");
        let json = serde_json::to_string(&id).expect("test: serialize");
        let back: NodeId = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(id, back);
    }

    #[test]
    fn node_id_copy_semantics() {
        let id = NodeId::from_bytes([1u8; 32]);
        let copied = id; // Copy
        assert_eq!(id, copied); // Original still usable
    }

    // --- ScopedIdentity ---

    #[test]
    fn scoped_identity_anonymous_device() {
        let scope = IdentityScope::anonymous_device();
        assert_eq!(scope.blockchain_scope, BlockchainScope::Device);
        assert!(!scope.tracked);
    }

    #[test]
    fn scoped_identity_private_network() {
        let scope = IdentityScope::private_network();
        assert_eq!(scope.blockchain_scope, BlockchainScope::Network);
        assert!(scope.tracked);
    }

    #[test]
    fn scoped_identity_public_network() {
        let scope = IdentityScope::public_network();
        assert_eq!(scope.blockchain_scope, BlockchainScope::Network);
        assert!(scope.tracked);
    }

    #[test]
    fn scoped_identity_new_node() {
        let node_id = NodeId::from_public_key(b"test-key");
        let scope = IdentityScope::anonymous_device();
        let identity = ScopedIdentity::new_node(node_id, scope);
        assert_eq!(identity.node_id, node_id);
        assert_eq!(identity.workload_type, WorkloadType::Node);
        assert_eq!(identity.scope, scope);
        assert!(identity.label.is_none());
    }

    #[test]
    fn scoped_identity_serde_roundtrip() {
        let node_id = NodeId::from_public_key(b"serde-test-key");
        let identity = ScopedIdentity::new_node_with_label(
            node_id,
            IdentityScope::private_network(),
            "my-node",
        );
        let json = serde_json::to_string(&identity).expect("test: serialize");
        let back: ScopedIdentity = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(identity.node_id, back.node_id);
        assert_eq!(identity.workload_type, back.workload_type);
        assert_eq!(identity.scope, back.scope);
        assert_eq!(identity.label, back.label);
    }

    #[test]
    fn workload_type_display() {
        assert_eq!(WorkloadType::Node.to_string(), "Node");
        assert_eq!(WorkloadType::Service.to_string(), "Service");
        assert_eq!(WorkloadType::Agent.to_string(), "Agent");
    }

    #[test]
    fn identity_scope_display() {
        let scope = IdentityScope::anonymous_device();
        assert_eq!(scope.to_string(), "Device:untracked");

        let scope = IdentityScope::private_network();
        assert_eq!(scope.to_string(), "Network:tracked");
    }

    #[test]
    fn scoped_identity_display() {
        let node_id = NodeId::from_bytes([0xAB; 32]);
        let identity = ScopedIdentity::new_node(node_id, IdentityScope::anonymous_device());
        let display = format!("{}", identity);
        assert!(display.contains("Node"), "got: {display}");
        assert!(display.contains("abababab"), "got: {display}");
    }

    // --- ContentHash ---

    #[test]
    fn content_hash_roundtrip() {
        let bytes = [42u8; 32];
        let hash = ContentHash::from_bytes(bytes);
        assert_eq!(hash.as_bytes(), &bytes);
    }

    #[test]
    fn content_hash_zeroed() {
        let hash = ContentHash::zeroed();
        assert_eq!(hash.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn privacy_mode_presets() {
        assert_eq!(PrivacyMode::ANONYMOUS.scope, AccessScope::Unbounded);
        assert!(!PrivacyMode::ANONYMOUS.tracked);

        assert_eq!(PrivacyMode::PRIVATE.scope, AccessScope::Bounded);
        assert!(PrivacyMode::PRIVATE.tracked);

        assert_eq!(PrivacyMode::PUBLIC.scope, AccessScope::Unbounded);
        assert!(PrivacyMode::PUBLIC.tracked);
    }

    #[test]
    fn privacy_mode_caesar_multiplier() {
        assert!((PrivacyMode::ANONYMOUS.caesar_multiplier() - 0.0).abs() < f64::EPSILON);
        assert!((PrivacyMode::PRIVATE.caesar_multiplier() - 0.5).abs() < f64::EPSILON);
        assert!((PrivacyMode::PUBLIC.caesar_multiplier() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn privacy_mode_serde_roundtrip() {
        let modes = [PrivacyMode::ANONYMOUS, PrivacyMode::PRIVATE, PrivacyMode::PUBLIC];
        for mode in &modes {
            let json = serde_json::to_string(mode).expect("test: serialize");
            let back: PrivacyMode = serde_json::from_str(&json).expect("test: deserialize");
            assert_eq!(*mode, back);
        }
    }

    #[test]
    fn asset_address_roundtrip() {
        let hash = ContentHash::from_bytes([0xAB; 32]);
        let addr = AssetAddress::new(10, -20, 30, &hash).expect("test: valid address");
        assert_eq!(addr.matrix_coords(), (10, -20, 30));
        assert_eq!(addr.shard_index(), 0);
        assert!(addr.is_hypermesh());
    }

    #[test]
    fn asset_address_shard_derivation() {
        let hash = ContentHash::from_bytes([0xCD; 32]);
        let parent = AssetAddress::new(0, 0, 0, &hash).expect("test: valid address");
        let shard3 = parent.shard(3).expect("test: valid shard");
        assert_eq!(shard3.shard_index(), 3);
        assert_eq!(shard3.parent(), parent);
    }

    #[test]
    fn asset_address_ipv6_roundtrip() {
        let hash = ContentHash::from_bytes([0x11; 32]);
        let addr = AssetAddress::new(1, 2, 3, &hash).expect("test: valid address");
        let ipv6 = addr.to_ipv6();
        let back = AssetAddress::from_ipv6(ipv6).expect("test: valid ipv6");
        assert_eq!(addr, back);
    }

    #[test]
    fn blockchain_scope_display() {
        assert_eq!(BlockchainScope::Device.to_string(), "Device");
        assert_eq!(BlockchainScope::Network.to_string(), "Network");
    }

    #[test]
    fn pipeline_stage_display() {
        assert_eq!(PipelineStage::Compress.to_string(), "Compress");
        assert_eq!(PipelineStage::Encrypt.to_string(), "Encrypt");
        assert_eq!(PipelineStage::Shard.to_string(), "Shard");
        assert_eq!(PipelineStage::Distribute.to_string(), "Distribute");
    }
}
