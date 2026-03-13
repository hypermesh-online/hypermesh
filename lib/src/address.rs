// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! IPv6-based routable asset addresses for the HyperMesh network.
//!
//! Every asset in HyperMesh has a unique IPv6 address derived from its
//! matrix position and content hash, enabling direct network routing.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::types::ContentHash;

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
/// +------+ +--+ +--------------+ +------------------+
/// prefix  net   matrix coords    asset fingerprint
/// (16b)  (16b)  (48b: 3*i16)    (48b: 44b hash + 4b shard)
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
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
}
