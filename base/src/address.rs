// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Substrate.a — sovereign, verifiable address derivation.
//!
//! The Substrate owns the ONE construction that turns a node identity into a
//! matrix cell and a routable `fd48:4d00::/32` address. Any peer that knows a
//! node's identity (`NodeId = BLAKE3(falcon_pubkey)`, carried as a hex string)
//! can recompute both and verify them — there is no DHCP, no lease, and no
//! authoritative on-disk copy. The address is *durable by derivation*: every
//! boot recomputes it byte-identically.
//!
//! ## Two derivations, one hash family
//!
//! - [`derive_cell`] maps a node id to a `(x, y, z)` matrix cell. This is the
//!   canonical construction; `blockmatrix`'s `MatrixCoordinate::derive_cell`
//!   delegates here so there is exactly one place the cell is computed.
//! - [`derive_address`] composes that cell with a content fingerprint of the
//!   identity into an [`AssetAddress`] and returns its `Ipv6Addr` in the
//!   HyperMesh ULA prefix.
//!
//! ## Layering
//!
//! `base` depends only on `hypermesh-lib` (for [`AssetAddress`]/[`ContentHash`]).
//! STOQ does NOT depend on `base`; the node binary injects derived values into
//! STOQ's `TransportConfig`. See `base/CLAUDE.md` §Layering.

use std::net::Ipv6Addr;

use hypermesh_lib::{AssetAddress, ContentHash};

/// Domain separator for matrix-cell derivation. Keep in lockstep with the
/// historical `blockmatrix` construction so cell derivation is byte-identical
/// across the delegation boundary.
const CELL_DOMAIN: &[u8] = b"hypermesh-matrix-cell-v1";

/// Domain separator for the identity content fingerprint used in the node's
/// derived address. Distinct from [`CELL_DOMAIN`] so the fingerprint and the
/// cell axes are drawn from independent digests.
const ADDRESS_DOMAIN: &[u8] = b"hypermesh-node-address-v1";

/// Deterministically derive a matrix cell `(x, y, z)` from a device node id.
///
/// `device_node_id` is the canonical node id (`BLAKE3(falcon_pubkey)` hex). The
/// id is hashed with a domain separator and three 16-bit windows of the digest
/// are mapped into signed `i16` axes. Producing `i16`-range coordinates
/// guarantees every asset the node hosts gets a valid [`AssetAddress`] under its
/// derived cell prefix (bytes 4-9 of the address encode `i16` big-endian).
///
/// This is the single canonical construction: `blockmatrix` delegates to it.
///
/// ## This cell is an identity fingerprint, NOT an authoritative location
///
/// The cell this returns — and the cell packed into [`AssetAddress`] — is a
/// deterministic content/identity derivation (a uniform-random point in the
/// matrix, with no locality). It answers *what an asset IS* (a durable,
/// peer-verifiable identity), **not** *where it currently lives*. Where an asset
/// actually resides and replicates is a demand-driven NGauge placement decision
/// (`ngauge::placement::PlacementLease`), elastic and re-issued as load shifts —
/// never this hash. Do not read this cell as a physical or authoritative
/// location. See VISION.md §5.5 (identity is durable; location is elastic and
/// NGauge-owned).
pub fn derive_cell(device_node_id: &str) -> (i16, i16, i16) {
    let mut hasher = blake3::Hasher::new();
    hasher.update(CELL_DOMAIN);
    hasher.update(device_node_id.as_bytes());
    let digest = hasher.finalize();
    let bytes = digest.as_bytes();

    let x = i16::from_be_bytes([bytes[0], bytes[1]]);
    let y = i16::from_be_bytes([bytes[2], bytes[3]]);
    let z = i16::from_be_bytes([bytes[4], bytes[5]]);
    (x, y, z)
}

/// Derive the routable `fd48:4d00::/32` address for a node from its identity.
///
/// Composition:
/// 1. Matrix cell from [`derive_cell`] (`i16` axes, always in range).
/// 2. A 32-byte content fingerprint = `BLAKE3(ADDRESS_DOMAIN || node_id)`,
///    wrapped as a [`ContentHash`] (the asset model uses its first 6 bytes).
/// 3. [`AssetAddress::new`] at `(x, y, z)`, shard 0 (whole node).
///
/// Because the axes are always `i16`-representable, `AssetAddress::new` cannot
/// reject them; the fallback below is defensive and never allocates on the wire.
///
/// The result is a pure function of `node_id` — recomputable and verifiable by
/// any peer, never leased and never stored as authoritative state (R15/R16 and
/// the AssetAddress model in `lib::AssetAddress`).
pub fn derive_address(node_id: &str) -> Ipv6Addr {
    let (x, y, z) = derive_cell(node_id);

    let mut hasher = blake3::Hasher::new();
    hasher.update(ADDRESS_DOMAIN);
    hasher.update(node_id.as_bytes());
    let fingerprint = ContentHash::from_bytes(*hasher.finalize().as_bytes());

    match AssetAddress::new(x as i64, y as i64, z as i64, &fingerprint) {
        Ok(addr) => addr.to_ipv6(),
        // Unreachable in practice: i16 axes are always inside AssetAddress
        // bounds. Fall back to the ULA-prefixed fingerprint-only form rather
        // than surfacing an error the caller cannot act on.
        Err(_) => fallback_address(&fingerprint),
    }
}

/// Defensive fallback: an `fd48:4d00::/32` address at the origin cell carrying
/// only the identity fingerprint. Used only if [`AssetAddress::new`] rejects the
/// derived axes, which cannot happen for `i16` inputs.
fn fallback_address(fingerprint: &ContentHash) -> Ipv6Addr {
    // Origin cell (0,0,0) is always valid; new() then cannot fail. On the
    // truly-impossible failure, return the bare ULA prefix `fd48:4d00::`.
    match AssetAddress::new(0, 0, 0, fingerprint) {
        Ok(addr) => addr.to_ipv6(),
        Err(_) => Ipv6Addr::new(0xfd48, 0x4d00, 0, 0, 0, 0, 0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The historical `blockmatrix` construction, inlined verbatim, so the test
    /// proves `derive_cell` is byte-identical to the code it replaces.
    fn legacy_blockmatrix_derive_cell(device_node_id: &str) -> (i16, i16, i16) {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"hypermesh-matrix-cell-v1");
        hasher.update(device_node_id.as_bytes());
        let digest = hasher.finalize();
        let bytes = digest.as_bytes();
        let x = i16::from_be_bytes([bytes[0], bytes[1]]);
        let y = i16::from_be_bytes([bytes[2], bytes[3]]);
        let z = i16::from_be_bytes([bytes[4], bytes[5]]);
        (x, y, z)
    }

    #[test]
    fn derive_cell_matches_legacy_blockmatrix_construction() {
        for id in [
            "9f4fc6ed4ba7",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "deadbeefcafef00dba5eba11badf00d0",
            "",
        ] {
            assert_eq!(
                derive_cell(id),
                legacy_blockmatrix_derive_cell(id),
                "cell derivation diverged from legacy blockmatrix for id {id:?}"
            );
        }
    }

    #[test]
    fn derive_cell_is_deterministic() {
        let a = derive_cell("some-node-id");
        let b = derive_cell("some-node-id");
        assert_eq!(a, b);
    }

    #[test]
    fn derive_cell_distinguishes_ids() {
        assert_ne!(derive_cell("node-a"), derive_cell("node-b"));
    }

    #[test]
    fn derive_address_is_in_hypermesh_prefix() {
        let addr = derive_address("9f4fc6ed4ba7");
        let octets = addr.octets();
        // fd48:4d00 ULA prefix (see lib::HYPERMESH_PREFIX).
        assert_eq!(&octets[0..4], &[0xfd, 0x48, 0x4d, 0x00]);
    }

    #[test]
    fn derive_address_is_deterministic() {
        assert_eq!(derive_address("node-x"), derive_address("node-x"));
        assert_ne!(derive_address("node-x"), derive_address("node-y"));
    }

    #[test]
    fn derive_address_encodes_the_derived_cell() {
        let node_id = "abc123def456";
        let (x, y, z) = derive_cell(node_id);
        let addr = derive_address(node_id);
        let parsed = AssetAddress::from_ipv6(addr).expect("test: valid hypermesh address");
        assert_eq!(parsed.matrix_coords(), (x as i64, y as i64, z as i64));
    }
}
