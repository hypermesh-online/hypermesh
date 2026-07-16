// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Extension Header Definitions
//!
//! Defines HyperMesh-specific extension headers that are carried as a
//! PLAINTEXT PREFIX in STOQ packets, immediately after the UDP header and
//! ahead of the QUIC-encrypted body (papers/HYPERMESH.md §5.1-5.7). STOQ emits
//! these on the send path (`apply_extensions`); the kernel XDP program reads
//! them to classify HyperMesh protocol metadata at wire speed.
//!
//! Two families of types live here:
//!
//! 1. The documented userspace ExtensionValidator representations
//!    (`ProofOfStateHeader`, `AssetHashHeader`, `MatrixRoutingHeader`,
//!    `PrivacyTierHeader`) — the rich four-proof / matrix-routing structures
//!    used by the userspace structural pre-validators (`validation.rs`, §5.4).
//!
//! 2. The ON-THE-WIRE kernel headers (`WireExtHeader`, `WirePosHeader`) whose
//!    byte layout matches the C `struct hmesh_header` / `struct hmesh_pos_header`
//!    in `programs/hypermesh_xdp.c` BYTE-FOR-BYTE. These are what STOQ prepends
//!    and what XDP parses. The byte-identity between `WirePosHeader::to_bytes`
//!    and the C struct offsets is the load-bearing contract (see the
//!    `wire_pos_header_matches_c_offsets` test).

use hypermesh_lib::{AccessScope, PrivacyMode};
use serde::{Deserialize, Serialize};

/// Extension type constants (HyperMesh namespace: 0x1000-0x1FFF).
///
/// Userspace validator namespace. Distinct from the compact on-wire
/// `WIRE_HDR_*` type bytes the kernel reads.
pub const EXT_PROOF_OF_STATE: u16 = 0x1000;
pub const EXT_ASSET_HASH: u16 = 0x1001;
pub const EXT_MATRIX_ROUTING: u16 = 0x1002;
pub const EXT_PRIVACY_TIER: u16 = 0x1003;

// -----------------------------------------------------------------------
// On-the-wire kernel extension headers (match C hmesh_header / hmesh_pos_header)
// -----------------------------------------------------------------------

/// HyperMesh magic bytes ('HM', 0x484D) carried in network byte order.
///
/// The kernel checks `bpf_ntohs(hdr.magic) == 0x484D`, so on the wire the two
/// magic bytes are `[0x48, 0x4D]` (big-endian / network order).
pub const WIRE_HDR_MAGIC: u16 = 0x484D;

/// On-wire extension type bytes (match the `HMESH_HDR_*` C defines).
pub const WIRE_HDR_POS: u8 = 0x01;
pub const WIRE_HDR_ASSET: u8 = 0x02;
pub const WIRE_HDR_MATRIX: u8 = 0x03;
pub const WIRE_HDR_PRIVACY: u8 = 0x04;

/// Common 4-byte extension header preceding every wire extension payload.
///
/// Matches the C `struct hmesh_header` exactly (no padding):
/// ```c
/// struct hmesh_header {
///     __u16 magic;   /* network byte order: 0x48 0x4D */
///     __u8  type;    /* HMESH_HDR_* */
///     __u8  length;  /* payload length following this header */
/// };
/// ```
/// Wire layout (4 bytes):
///   `[0..2]` magic  (u16, network/big-endian)
///   `[2]`    type   (`WIRE_HDR_*`)
///   `[3]`    length (payload length after this header)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireExtHeader {
    /// Extension type (`WIRE_HDR_*`).
    pub ext_type: u8,
    /// Length of the payload that follows this 4-byte header.
    pub length: u8,
}

impl WireExtHeader {
    /// Serialized size of the common header.
    pub const SIZE: usize = 4;

    /// Create a new common header for the given extension type and payload len.
    pub fn new(ext_type: u8, length: u8) -> Self {
        Self { ext_type, length }
    }

    /// Serialize to 4 bytes: `[magic BE][type][length]`.
    pub fn to_bytes(&self) -> [u8; 4] {
        let mut b = [0u8; 4];
        // Magic in network byte order so XDP's bpf_ntohs sees 0x484D.
        b[0..2].copy_from_slice(&WIRE_HDR_MAGIC.to_be_bytes());
        b[2] = self.ext_type;
        b[3] = self.length;
        b
    }

    /// Deserialize from bytes. Returns `None` if too short or magic mismatch.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let magic = u16::from_be_bytes([bytes[0], bytes[1]]);
        if magic != WIRE_HDR_MAGIC {
            return None;
        }
        Some(Self {
            ext_type: bytes[2],
            length: bytes[3],
        })
    }
}

/// On-wire PoS proof summary, matching the C `struct hmesh_pos_header`
/// BYTE-FOR-BYTE (natural alignment, 40 bytes — the `u32` forces a 4-byte
/// boundary so there are 3 pad bytes after `algorithm`):
/// ```c
/// struct hmesh_pos_header {
///     __u8  algorithm;   /* 0x01=FALCON, 0x02=Ed25519, 0x03=ECDSA */
///     __u32 difficulty;  /* required leading zero bits (kernel-carried) */
///     __u8  hash[32];    /* work hash (first 32 bytes of proof) */
/// };
/// ```
/// Wire layout (40 bytes, little-endian native — memcpy'd in-kernel):
///   `[0]`     algorithm  (u8)
///   `[1..4]`  padding    (zero)
///   `[4..8]`  difficulty (u32 LE)
///   `[8..40]` hash       (32 bytes)
///
/// The kernel reads `algorithm` @0 and `hash` @8; `difficulty` is carried but
/// not consulted in-kernel. This is the header STOQ prepends on the send path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WirePosHeader {
    /// Signing algorithm indicator (0x01 FALCON / 0x02 Ed25519 / 0x03 ECDSA).
    pub algorithm: u8,
    /// Required PoW difficulty (leading zero bits). Carried, not kernel-checked.
    pub difficulty: u32,
    /// Work hash — the PoS token's 32-byte BLAKE3 digest.
    pub hash: [u8; 32],
}

impl WirePosHeader {
    /// Serialized size matching the C struct's natural alignment.
    pub const SIZE: usize = 40; // 1 + 3 pad + 4 + 32

    /// Construct from a PoS token's 32-byte work hash (FALCON-1024 default).
    pub fn from_pos_hash(hash: [u8; 32], difficulty: u32) -> Self {
        Self {
            algorithm: WIRE_HDR_POS_ALG_FALCON,
            difficulty,
            hash,
        }
    }

    /// Serialize to the exact 40-byte C layout (algorithm@0, difficulty@4,
    /// hash@8; pad bytes are zero).
    pub fn to_bytes(&self) -> [u8; 40] {
        let mut b = [0u8; 40];
        b[0] = self.algorithm;
        // b[1..4] padding stays zero.
        b[4..8].copy_from_slice(&self.difficulty.to_le_bytes());
        b[8..40].copy_from_slice(&self.hash);
        b
    }

    /// Deserialize from the 40-byte C layout. Returns `None` if too short.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes[8..40]);
        Some(Self {
            algorithm: bytes[0],
            difficulty: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            hash,
        })
    }
}

/// FALCON-1024 algorithm indicator for the on-wire PoS header.
pub const WIRE_HDR_POS_ALG_FALCON: u8 = 0x01;

/// Serialize a complete on-wire PoS extension (common header + PoS payload).
///
/// Produces `[magic BE][type=0x01][len=40][WirePosHeader 40 bytes]` = 44 bytes.
/// This is the exact byte sequence STOQ prepends ahead of the encrypted QUIC
/// body, and the exact sequence XDP parses (`hmesh_header` then
/// `hmesh_pos_header`).
pub fn encode_pos_extension(pos: &WirePosHeader) -> Vec<u8> {
    let mut out = Vec::with_capacity(WireExtHeader::SIZE + WirePosHeader::SIZE);
    let common = WireExtHeader::new(WIRE_HDR_POS, WirePosHeader::SIZE as u8);
    out.extend_from_slice(&common.to_bytes());
    out.extend_from_slice(&pos.to_bytes());
    out
}

// -----------------------------------------------------------------------
// Userspace ExtensionValidator representations (§5.4 deep structural checks)
// -----------------------------------------------------------------------

/// Proof of State extension header (userspace four-proof representation).
///
/// Contains the four proofs required by HyperMesh Proof of State:
/// - WHO: Proof of Stake (identity validation)
/// - WHAT: Proof of Work (computational commitment)
/// - WHEN: Proof of Time (temporal ordering)
/// - WHERE: Proof of Space (storage/location commitment)
///
/// This is the rich userspace structure exercised by `ProofOfStateValidator`
/// (see `validation.rs`). It is distinct from the compact on-wire
/// [`WirePosHeader`] the kernel parses.
#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProofOfStateHeader {
    /// Proof of Stake (WHO) - Identity validation
    pub who: [u8; 32],
    /// Proof of Work (WHAT) - Computational commitment
    pub what: [u8; 32],
    /// Proof of Time (WHEN) - Timestamp in Unix epoch microseconds
    pub when: u64,
    /// Proof of Space (WHERE) - Matrix position as IPv6 address
    pub where_: [u8; 16],
}

impl ProofOfStateHeader {
    /// Size of serialized header in bytes
    pub const SIZE: usize = 88; // 32 + 32 + 8 + 16

    /// Serialize to bytes for STOQ extension header
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.extend_from_slice(&self.who);
        bytes.extend_from_slice(&self.what);
        bytes.extend_from_slice(&self.when.to_be_bytes());
        bytes.extend_from_slice(&self.where_);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let mut who = [0u8; 32];
        let mut what = [0u8; 32];
        let mut where_ = [0u8; 16];

        who.copy_from_slice(&bytes[0..32]);
        what.copy_from_slice(&bytes[32..64]);
        let when = u64::from_be_bytes(bytes[64..72].try_into().ok()?);
        where_.copy_from_slice(&bytes[72..88]);

        Some(Self {
            who,
            what,
            when,
            where_,
        })
    }

    /// Validate proof timestamps (basic sanity check)
    pub fn validate_timestamps(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        // Allow 5-minute clock skew
        let tolerance = 5 * 60 * 1_000_000; // 5 minutes in microseconds

        // Proof cannot be from the future
        if self.when > now + tolerance {
            return false;
        }

        // Proof cannot be too old (24 hours)
        let max_age = 24 * 60 * 60 * 1_000_000; // 24 hours in microseconds
        if now.saturating_sub(self.when) > max_age {
            return false;
        }

        true
    }
}

/// Asset Hash extension header
///
/// Provides content integrity validation for asset transfers
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetHashHeader {
    /// Asset unique identifier (BLAKE3 hash)
    pub asset_id: [u8; 32],
    /// Content hash (BLAKE3)
    pub hash: [u8; 32],
    /// Number of shards for this asset
    pub shard_count: u32,
    /// Current shard index (0-based)
    pub shard_index: u32,
}

impl AssetHashHeader {
    /// Size of serialized header in bytes
    pub const SIZE: usize = 72; // 32 + 32 + 4 + 4

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.extend_from_slice(&self.asset_id);
        bytes.extend_from_slice(&self.hash);
        bytes.extend_from_slice(&self.shard_count.to_be_bytes());
        bytes.extend_from_slice(&self.shard_index.to_be_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let mut asset_id = [0u8; 32];
        let mut hash = [0u8; 32];

        asset_id.copy_from_slice(&bytes[0..32]);
        hash.copy_from_slice(&bytes[32..64]);
        let shard_count = u32::from_be_bytes(bytes[64..68].try_into().ok()?);
        let shard_index = u32::from_be_bytes(bytes[68..72].try_into().ok()?);

        Some(Self {
            asset_id,
            hash,
            shard_count,
            shard_index,
        })
    }

    /// Validate shard indices
    pub fn validate_shard_indices(&self) -> bool {
        if self.shard_count == 0 {
            return false;
        }
        self.shard_index < self.shard_count
    }
}

/// Matrix coordinate for routing
#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MatrixCoordinate {
    /// X coordinate in matrix
    pub x: u16,
    /// Y coordinate in matrix
    pub y: u16,
    /// Z coordinate in matrix (layer)
    pub z: u16,
}

impl MatrixCoordinate {
    pub const SIZE: usize = 6; // 2 + 2 + 2

    pub fn to_bytes(&self) -> [u8; 6] {
        let mut bytes = [0u8; 6];
        bytes[0..2].copy_from_slice(&self.x.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.y.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.z.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            x: u16::from_be_bytes(bytes[0..2].try_into().ok()?),
            y: u16::from_be_bytes(bytes[2..4].try_into().ok()?),
            z: u16::from_be_bytes(bytes[4..6].try_into().ok()?),
        })
    }
}

/// Matrix Routing extension header
///
/// Defines routing path through HyperMesh matrix topology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixRoutingHeader {
    /// Source node position
    pub source: MatrixCoordinate,
    /// Destination node position
    pub destination: MatrixCoordinate,
    /// Routing path through matrix (up to 8 hops)
    pub path: Vec<MatrixCoordinate>,
}

impl MatrixRoutingHeader {
    /// Maximum hops in routing path
    pub const MAX_HOPS: usize = 8;

    /// Minimum size (source + destination, no path)
    pub const MIN_SIZE: usize = 12; // 6 + 6

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let path_len = self.path.len().min(Self::MAX_HOPS);
        let size = Self::MIN_SIZE + (path_len * MatrixCoordinate::SIZE);

        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(&self.source.to_bytes());
        bytes.extend_from_slice(&self.destination.to_bytes());

        for coord in self.path.iter().take(Self::MAX_HOPS) {
            bytes.extend_from_slice(&coord.to_bytes());
        }

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::MIN_SIZE {
            return None;
        }

        let source = MatrixCoordinate::from_bytes(&bytes[0..6])?;
        let destination = MatrixCoordinate::from_bytes(&bytes[6..12])?;

        let mut path = Vec::new();
        let mut offset = 12;

        while offset + MatrixCoordinate::SIZE <= bytes.len() && path.len() < Self::MAX_HOPS {
            if let Some(coord) = MatrixCoordinate::from_bytes(&bytes[offset..offset + 6]) {
                path.push(coord);
                offset += MatrixCoordinate::SIZE;
            } else {
                break;
            }
        }

        Some(Self {
            source,
            destination,
            path,
        })
    }

    /// Validate routing path (no loops, within matrix bounds)
    pub fn validate_path(&self, matrix_size: u16) -> bool {
        // Check source and destination are within bounds
        if self.source.x >= matrix_size
            || self.source.y >= matrix_size
            || self.destination.x >= matrix_size
            || self.destination.y >= matrix_size
        {
            return false;
        }

        // Check all path coordinates are within bounds
        for coord in &self.path {
            if coord.x >= matrix_size || coord.y >= matrix_size {
                return false;
            }
        }

        // Check for loops (no duplicate coordinates)
        let mut seen = std::collections::HashSet::new();
        seen.insert(self.source);

        for coord in &self.path {
            if !seen.insert(*coord) {
                return false; // Loop detected
            }
        }

        true
    }
}

/// Convert a u8 (from eBPF map or wire format) back to PrivacyMode
pub fn privacy_mode_from_u8(value: u8) -> Option<PrivacyMode> {
    match value {
        0 => Some(PrivacyMode::ANONYMOUS),
        1 => Some(PrivacyMode {
            scope: AccessScope::Bounded,
            tracked: false,
        }),
        2 => Some(PrivacyMode::PRIVATE),
        3 => Some(PrivacyMode::PUBLIC),
        _ => None,
    }
}

/// Privacy mode extension header
#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PrivacyTierHeader {
    /// Privacy mode
    pub tier: PrivacyMode,
    /// Reserved for future use
    _reserved: [u8; 7],
}

impl PrivacyTierHeader {
    pub const SIZE: usize = 8;

    pub fn new(tier: PrivacyMode) -> Self {
        Self {
            tier,
            _reserved: [0u8; 7],
        }
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0] = self.tier.to_ebpf_u8();
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            tier: privacy_mode_from_u8(bytes[0])?,
            _reserved: [0u8; 7],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- On-wire header byte-identity contract (load-bearing) ----

    #[test]
    fn wire_common_header_magic_is_network_order() {
        // XDP does bpf_ntohs(hdr.magic) == 0x484D, so the first two wire
        // bytes MUST be 0x48 0x4D (network / big-endian).
        let hdr = WireExtHeader::new(WIRE_HDR_POS, WirePosHeader::SIZE as u8);
        let b = hdr.to_bytes();
        assert_eq!(b[0], 0x48, "magic high byte 'H'");
        assert_eq!(b[1], 0x4D, "magic low byte 'M'");
        assert_eq!(b[2], WIRE_HDR_POS);
        assert_eq!(b[3], 40);
    }

    #[test]
    fn wire_pos_header_matches_c_offsets() {
        // The C struct hmesh_pos_header (natural alignment) places:
        //   algorithm @0, (pad 1..4), difficulty @4 (LE u32), hash @8..40.
        // sizeof == 40. This test pins the Rust emit to those EXACT offsets —
        // this byte-identity is what lets the kernel memcpy the header and
        // read algorithm@0 / hash@8 correctly.
        let pos = WirePosHeader {
            algorithm: 0x01,
            difficulty: 0x0A0B0C0D,
            hash: {
                let mut h = [0u8; 32];
                for (i, b) in h.iter_mut().enumerate() {
                    *b = (i as u8).wrapping_add(1);
                }
                h
            },
        };
        let bytes = pos.to_bytes();
        assert_eq!(bytes.len(), WirePosHeader::SIZE);
        assert_eq!(bytes.len(), 40);

        // algorithm @ offset 0
        assert_eq!(bytes[0], 0x01);
        // 3 pad bytes @ 1..4 must be zero (C compiler-inserted alignment pad)
        assert_eq!(&bytes[1..4], &[0u8, 0u8, 0u8]);
        // difficulty @ offset 4, little-endian (native memcpy on x86)
        assert_eq!(
            u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            0x0A0B0C0D
        );
        // hash @ offset 8..40
        assert_eq!(&bytes[8..40], &pos.hash);
    }

    #[test]
    fn wire_pos_header_round_trip() {
        // Rust to_bytes -> from_bytes is identity; and a fresh from_bytes over
        // the exact C offsets recovers the same fields (the vice-versa
        // direction of the byte-identity contract).
        let pos = WirePosHeader::from_pos_hash([0x7Au8; 32], 8);
        let bytes = pos.to_bytes();
        let decoded = WirePosHeader::from_bytes(&bytes).expect("test: from_bytes");
        assert_eq!(decoded, pos);
        assert_eq!(decoded.algorithm, WIRE_HDR_POS_ALG_FALCON);
        assert_eq!(decoded.difficulty, 8);
        assert_eq!(decoded.hash, [0x7Au8; 32]);
    }

    #[test]
    fn encode_pos_extension_is_44_bytes_common_plus_payload() {
        // Full on-wire PoS extension = 4-byte common header + 40-byte payload.
        // A C reader parses hmesh_header (4) then hmesh_pos_header (40) from
        // exactly these bytes.
        let pos = WirePosHeader::from_pos_hash([0x11u8; 32], 8);
        let ext = encode_pos_extension(&pos);
        assert_eq!(ext.len(), 44);

        // Common header parses back with the right magic + type + length.
        let common = WireExtHeader::from_bytes(&ext[0..4]).expect("test: common");
        assert_eq!(common.ext_type, WIRE_HDR_POS);
        assert_eq!(common.length, 40);

        // Payload at offset 4 parses back to the same PoS header.
        let parsed = WirePosHeader::from_bytes(&ext[4..]).expect("test: payload");
        assert_eq!(parsed, pos);
    }

    #[test]
    fn wire_common_header_rejects_bad_magic() {
        let mut bytes = WireExtHeader::new(WIRE_HDR_POS, 40).to_bytes();
        bytes[0] = 0x00; // corrupt magic
        assert!(WireExtHeader::from_bytes(&bytes).is_none());
    }

    // ---- Userspace representation self-tests (documented structs) ----

    #[test]
    fn test_proof_of_state_serialization() {
        let proof = ProofOfStateHeader {
            who: [1u8; 32],
            what: [2u8; 32],
            when: 1234567890,
            where_: [3u8; 16],
        };

        let bytes = proof.to_bytes();
        assert_eq!(bytes.len(), ProofOfStateHeader::SIZE);

        let decoded = ProofOfStateHeader::from_bytes(&bytes).expect("test: expected success");
        assert_eq!(decoded.who, proof.who);
        assert_eq!(decoded.what, proof.what);
        assert_eq!(decoded.when, proof.when);
        assert_eq!(decoded.where_, proof.where_);
    }

    #[test]
    fn test_asset_hash_validation() {
        let asset = AssetHashHeader {
            asset_id: [1u8; 32],
            hash: [2u8; 32],
            shard_count: 10,
            shard_index: 5,
        };

        assert!(asset.validate_shard_indices());

        let invalid = AssetHashHeader {
            asset_id: [1u8; 32],
            hash: [2u8; 32],
            shard_count: 10,
            shard_index: 10, // Invalid: >= shard_count
        };

        assert!(!invalid.validate_shard_indices());
    }

    #[test]
    fn test_matrix_routing_path_validation() {
        let routing = MatrixRoutingHeader {
            source: MatrixCoordinate { x: 0, y: 0, z: 0 },
            destination: MatrixCoordinate { x: 5, y: 5, z: 0 },
            path: vec![
                MatrixCoordinate { x: 1, y: 1, z: 0 },
                MatrixCoordinate { x: 2, y: 2, z: 0 },
            ],
        };

        assert!(routing.validate_path(10)); // 10x10 matrix

        let with_loop = MatrixRoutingHeader {
            source: MatrixCoordinate { x: 0, y: 0, z: 0 },
            destination: MatrixCoordinate { x: 5, y: 5, z: 0 },
            path: vec![
                MatrixCoordinate { x: 1, y: 1, z: 0 },
                MatrixCoordinate { x: 0, y: 0, z: 0 }, // Loop back to source
            ],
        };

        assert!(!with_loop.validate_path(10)); // Loop detected
    }

    #[test]
    fn test_privacy_mode_conversion() {
        assert_eq!(privacy_mode_from_u8(0), Some(PrivacyMode::ANONYMOUS));
        assert_eq!(privacy_mode_from_u8(3), Some(PrivacyMode::PUBLIC));
        assert_eq!(privacy_mode_from_u8(99), None);

        assert_eq!(PrivacyMode::PRIVATE.to_ebpf_u8(), 2);
    }
}
