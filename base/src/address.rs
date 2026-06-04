// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Substrate.a — sovereign self-assigned addressing.
//!
//! Derives a node's HyperMesh IPv6 address deterministically and verifiably from
//! its identity. This is the *producer* for a requirement the protocol already
//! mandates but never wired up:
//!
//! - `papers/HYPERMESH.md` **R1** requires hardware Network interfaces to be
//!   instantiated as assets "each with an IPv6 address in the HyperMesh address
//!   space (`fd48:4d00::/32`)".
//! - `papers/HYPERMESH.md` **R15** (added with the Substrate) makes the derivation
//!   normative: the address interface identifier is a function of
//!   `node_id = BLAKE3(falcon_public_key)`, with no DHCP, lease, or external
//!   address authority. Any peer holding the Falcon public key can recompute and
//!   verify the address.
//!
//! ## Address layout (the mapping R15/SUBSTRATE.md fix)
//!
//! ```text
//!   bits   0..32   fd48:4d00          fixed HyperMesh ULA prefix (R1)
//!   bits  32..64   <subnet/network>   set by network membership (0 = Device scope)
//!   bits  64..128  <IID from node_id> low 8 bytes of NodeId (BLAKE3 digest)
//! ```
//!
//! The interface identifier (low 64 bits) is taken from the last 8 bytes of the
//! 32-byte `NodeId`. Because `NodeId` is itself `BLAKE3(falcon_public_key)`
//! (`hypermesh_lib::NodeId::from_public_key`), the full address is a pure function
//! of the public key — deterministic and independently verifiable.
//!
//! Phase A (Substrate.a) implements the pure derivation/verification bodies and
//! their property tests. The mapping above is the normative contract.

use hypermesh_lib::NodeId;
use std::net::Ipv6Addr;

use crate::error::SubstrateResult;

/// The fixed HyperMesh ULA prefix (`fd48:4d00::/32`), per R1.
///
/// First 32 bits of every HyperMesh node address. The remaining 96 bits are the
/// subnet/network slot (bits 32..64) and the identity-derived interface
/// identifier (bits 64..128).
pub const HYPERMESH_PREFIX: [u8; 4] = [0xfd, 0x48, 0x4d, 0x00];

/// The "Device scope" subnet value used when a node is not (yet) a member of a
/// Network-scope blockchain. See `papers/HYPERMESH.md` on Device vs Network scope.
pub const SUBNET_DEVICE_SCOPE: u32 = 0;

/// Derive a node's sovereign `fd48:4d00::/32` address from its identity (R15).
///
/// `subnet` occupies bits 32..64 (use [`SUBNET_DEVICE_SCOPE`] for an unjoined
/// node); the interface identifier (bits 64..128) is derived from `node_id`.
///
/// The result is verifiable: any peer with the originating Falcon public key can
/// recompute `NodeId::from_public_key(pubkey)` and re-run this derivation to
/// confirm the address belongs to that identity.
pub fn derive_address(node_id: &NodeId, subnet: u32) -> SubstrateResult<Ipv6Addr> {
    let id = node_id.as_bytes(); // &[u8; 32]
    let mut octets = [0u8; 16];
    octets[0..4].copy_from_slice(&HYPERMESH_PREFIX); // fd48:4d00
    octets[4..8].copy_from_slice(&subnet.to_be_bytes()); // subnet slot
    octets[8..16].copy_from_slice(&id[24..32]); // IID from digest tail
    Ok(Ipv6Addr::from(octets))
}

/// Verify that `addr` is the address that `node_id` would derive under `subnet`.
///
/// Used by peers to confirm a claimed address matches the identity that signed
/// for it (the verifiability half of R15).
pub fn verify_address(addr: &Ipv6Addr, node_id: &NodeId, subnet: u32) -> SubstrateResult<bool> {
    Ok(*addr == derive_address(node_id, subnet)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::NodeId;

    /// Distinct test identities, derived from distinct public-key byte strings.
    fn id(name: &[u8]) -> NodeId {
        NodeId::from_public_key(name)
    }

    /// Same inputs always produce the same address, across repeated calls.
    #[test]
    fn determinism() {
        let node = id(b"determinism-key");
        let first = derive_address(&node, 7).unwrap();
        for _ in 0..1000 {
            assert_eq!(derive_address(&node, 7).unwrap(), first);
        }
    }

    /// An address derived for an identity verifies against that identity.
    #[test]
    fn round_trip_verify() {
        let cases: &[(&[u8], u32)] = &[
            (b"rt-a", SUBNET_DEVICE_SCOPE),
            (b"rt-b", 1),
            (b"rt-c", 0xDEAD_BEEF),
            (b"rt-d", u32::MAX),
        ];
        for &(name, subnet) in cases {
            let node = id(name);
            let addr = derive_address(&node, subnet).unwrap();
            assert!(
                verify_address(&addr, &node, subnet).unwrap(),
                "round-trip verify failed for {name:?}@{subnet}"
            );
        }
    }

    /// First four octets are always the fixed HyperMesh ULA prefix fd:48:4d:00.
    #[test]
    fn prefix_correctness() {
        let cases: &[(&[u8], u32)] = &[
            (b"pfx-a", SUBNET_DEVICE_SCOPE),
            (b"pfx-b", 42),
            (b"pfx-c", u32::MAX),
        ];
        for &(name, subnet) in cases {
            let addr = derive_address(&id(name), subnet).unwrap();
            let o = addr.octets();
            assert_eq!(&o[0..4], &[0xfd, 0x48, 0x4d, 0x00]);
        }
    }

    /// Octets [4..8] carry the subnet big-endian; device scope (0) yields zeros.
    #[test]
    fn subnet_placement() {
        let node = id(b"subnet-key");
        for subnet in [SUBNET_DEVICE_SCOPE, 1u32, 256, 0x0102_0304, u32::MAX] {
            let addr = derive_address(&node, subnet).unwrap();
            let o = addr.octets();
            assert_eq!(&o[4..8], &subnet.to_be_bytes());
        }
        let device = derive_address(&node, SUBNET_DEVICE_SCOPE).unwrap();
        assert_eq!(&device.octets()[4..8], &[0u8; 4]);
    }

    /// The interface identifier (octets [8..16]) is the NodeId digest tail [24..32].
    #[test]
    fn iid_source() {
        let cases: &[&[u8]] = &[b"iid-a", b"iid-b", b"iid-c"];
        for &name in cases {
            let node = id(name);
            let addr = derive_address(&node, 99).unwrap();
            assert_eq!(&addr.octets()[8..16], &node.as_bytes()[24..32]);
        }
    }

    /// Distinct identities yield distinct addresses across a sampled set.
    #[test]
    fn distinctness() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..512u32 {
            let node = id(format!("distinct-{i}").as_bytes());
            let addr = derive_address(&node, SUBNET_DEVICE_SCOPE).unwrap();
            assert!(seen.insert(addr), "address collision for index {i}: {addr}");
        }
    }

    /// An address derived for identity A does not verify against identity B.
    #[test]
    fn cross_identity_rejection() {
        let a = id(b"identity-A");
        let b = id(b"identity-B");
        let subnet = 5u32;
        let addr_a = derive_address(&a, subnet).unwrap();
        assert!(verify_address(&addr_a, &a, subnet).unwrap());
        assert!(
            !verify_address(&addr_a, &b, subnet).unwrap(),
            "A's address must not verify against B"
        );
    }
}
