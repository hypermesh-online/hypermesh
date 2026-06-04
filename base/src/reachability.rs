// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Substrate.a — reachability and path types.
//!
//! Describes how a node is reachable from peers: its externally visible address
//! (if any) and the kind of path that reaches it. This is the producer for
//! STOQ's `public_ipv6` field (`stoq/src/transport/config.rs`), which today must
//! be set manually with no discovery mechanism.
//!
//! NOTE: types are defined here; discovery (which may extend the existing STOQ
//! reflector pool) is a **scaffold** this pass.

use std::net::Ipv6Addr;

/// How a node is reachable from the mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    /// Directly reachable on a globally routable address — no traversal needed.
    Direct,
    /// Behind NAT/firewall; reachable via a discovered external address
    /// (hole-punched or reflector-assisted). Phase 1+ concern.
    Traversed,
    /// Reachability not yet determined.
    Unknown,
}

/// A node's reachability snapshot.
///
/// `public_v6` is the address to advertise to peers (feeds STOQ `public_ipv6`);
/// when `None`, the node advertises its derived [`crate::address`] directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reachability {
    /// Externally visible IPv6 address, if discovered.
    pub public_v6: Option<Ipv6Addr>,
    /// The kind of path that reaches this node.
    pub path: PathKind,
}

impl Reachability {
    /// An undetermined reachability (no external address known yet).
    pub fn unknown() -> Self {
        Self {
            public_v6: None,
            path: PathKind::Unknown,
        }
    }
}
