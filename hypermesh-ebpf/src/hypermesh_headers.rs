// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Extension Header Definitions
//!
//! Defines HyperMesh-specific extension headers that are carried in STOQ packets.
//! STOQ treats these as opaque byte blobs; HyperMesh interprets their semantics.

use hypermesh_lib::{AccessScope, PrivacyMode};
use serde::{Deserialize, Serialize};

/// Extension type constants (HyperMesh namespace: 0x1000-0x1FFF)
pub const EXT_PROOF_OF_STATE: u16 = 0x1000;
pub const EXT_ASSET_HASH: u16 = 0x1001;
pub const EXT_MATRIX_ROUTING: u16 = 0x1002;
pub const EXT_PRIVACY_TIER: u16 = 0x1003;

/// Proof of State extension header
///
/// Contains the four proofs required by HyperMesh Proof of State:
/// - WHO: Proof of Stake (identity validation)
/// - WHAT: Proof of Work (computational commitment)
/// - WHEN: Proof of Time (temporal ordering)
/// - WHERE: Proof of Space (storage/location commitment)
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
            .expect("system time should be after UNIX epoch")
            .as_micros() as u64;

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
