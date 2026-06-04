// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! The default Substrate implementation for Phase A.
//!
//! [`DefaultSubstrate`] answers the two Substrate.a questions that Phase A
//! implements for real:
//!
//! - [`Substrate::local_address`] — derives the node's sovereign
//!   `fd48:4d00::/32` address from its identity ([`crate::address::derive_address`]),
//!   pure and verifiable (R15).
//! - [`Substrate::reachability`] — reports how the node is reachable. In Phase A
//!   Part 1 this is [`Reachability::unknown`] until a reflector reports an
//!   observed source address; the reflector echo (Part 2) feeds this method.
//!
//! The link-layer methods ([`Substrate::active_interface`],
//! [`Substrate::watch_links`]) are Phase B and return
//! [`SubstrateError::Unsupported`] — callers therefore pass `ebpf_interface: None`
//! into STOQ and the existing interface fallback continues to operate unchanged.
//!
//! ## Layering
//! This type lives in `base` and depends only on `hypermesh-lib`. The node binary
//! (`blockmatrix`) constructs it, calls `local_address`/`reachability`, and injects
//! the results into STOQ's `TransportConfig`. STOQ never depends on `base`.

use async_trait::async_trait;
use futures::stream::BoxStream;
use hypermesh_lib::NodeId;
use std::net::Ipv6Addr;
use std::sync::Arc;

use crate::address::{derive_address, SUBNET_DEVICE_SCOPE};
use crate::adapters::SubstrateAdapterRegistry;
use crate::error::{SubstrateError, SubstrateResult};
use crate::link::{InterfaceId, LinkEvent};
use crate::reachability::Reachability;
use crate::substrate::Substrate;

/// The Phase A Substrate backend.
///
/// Holds the subnet slot this node addresses under (Device scope in Phase A) and,
/// optionally, the backend adapter registry that Phase B link/carrier work will
/// consume. Reachability is sourced from an observed-address signal a reflector
/// reports (see [`DefaultSubstrate::set_reachability`]); until one is reported it
/// is [`Reachability::unknown`].
pub struct DefaultSubstrate {
    /// Subnet slot (bits 32..64) for address derivation. `SUBNET_DEVICE_SCOPE`
    /// (0) until the node joins a Network-scope blockchain.
    subnet: u32,
    /// Backend adapter registry. Unused in Phase A Part 1 (no link/carrier work
    /// yet); retained so Phase B can select an adapter without changing callers.
    _registry: Arc<SubstrateAdapterRegistry>,
    /// Last reachability snapshot reported by a reflector echo. `unknown()` until
    /// a reflector observes this node's source address.
    reachability: std::sync::RwLock<Reachability>,
}

impl DefaultSubstrate {
    /// Construct a `DefaultSubstrate` addressing under [`SUBNET_DEVICE_SCOPE`].
    ///
    /// This is the Phase A constructor: an unjoined node derives its address in
    /// the Device-scope subnet (0). Reachability starts [`Reachability::unknown`].
    pub fn new() -> Self {
        Self::with_subnet(SUBNET_DEVICE_SCOPE)
    }

    /// Construct a `DefaultSubstrate` addressing under an explicit `subnet` slot.
    ///
    /// Network-membership subnet assignment is a documented Phase A follow-on; this
    /// entry point exists so that path needs no signature change.
    pub fn with_subnet(subnet: u32) -> Self {
        Self {
            subnet,
            _registry: Arc::new(SubstrateAdapterRegistry::with_defaults()),
            reachability: std::sync::RwLock::new(Reachability::unknown()),
        }
    }

    /// The subnet slot this Substrate derives addresses under.
    pub fn subnet(&self) -> u32 {
        self.subnet
    }

    /// Record a reachability snapshot observed by a reflector echo (Part 2).
    ///
    /// The reflector reports the source address it saw for this node; the consumer
    /// converts that into a [`Reachability`] (Direct when the observed address
    /// matches the bound/derived address, Traversed when it differs) and feeds it
    /// here. [`Substrate::reachability`] then returns the latest snapshot.
    pub fn set_reachability(&self, reachability: Reachability) {
        if let Ok(mut guard) = self.reachability.write() {
            *guard = reachability;
        }
    }
}

impl Default for DefaultSubstrate {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Substrate for DefaultSubstrate {
    async fn local_address(&self, node_id: &NodeId) -> SubstrateResult<Ipv6Addr> {
        derive_address(node_id, self.subnet)
    }

    async fn reachability(&self) -> SubstrateResult<Reachability> {
        let snapshot = self
            .reachability
            .read()
            .map(|g| *g)
            .map_err(|e| SubstrateError::Reachability(format!("reachability lock poisoned: {e}")))?;
        Ok(snapshot)
    }

    async fn active_interface(&self) -> SubstrateResult<InterfaceId> {
        // Phase B: interface selection replaces detect_outbound_interface().
        Err(SubstrateError::Unsupported(
            "active_interface is Phase B (link/carrier management)".to_string(),
        ))
    }

    async fn watch_links(&self) -> SubstrateResult<BoxStream<'static, LinkEvent>> {
        // Phase B: carrier monitoring / self-healing.
        Err(SubstrateError::Unsupported(
            "watch_links is Phase B (link-state monitoring)".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::{derive_address, SUBNET_DEVICE_SCOPE};
    use crate::reachability::PathKind;
    use hypermesh_lib::NodeId;

    fn id(name: &[u8]) -> NodeId {
        NodeId::from_public_key(name)
    }

    /// `local_address` matches `derive_address` under the Device-scope subnet.
    #[tokio::test]
    async fn local_address_matches_derive_address() {
        let substrate = DefaultSubstrate::new();
        let node = id(b"default-substrate-key");
        let got = substrate
            .local_address(&node)
            .await
            .expect("local_address should derive");
        let want = derive_address(&node, SUBNET_DEVICE_SCOPE).expect("derive_address");
        assert_eq!(got, want);
    }

    /// A non-zero subnet is threaded into derivation.
    #[tokio::test]
    async fn local_address_honors_subnet() {
        let substrate = DefaultSubstrate::with_subnet(42);
        let node = id(b"subnet-key");
        let got = substrate.local_address(&node).await.expect("derive");
        assert_eq!(got, derive_address(&node, 42).expect("derive"));
        assert_eq!(&got.octets()[4..8], &42u32.to_be_bytes());
    }

    /// Reachability starts unknown and reflects whatever is reported.
    #[tokio::test]
    async fn reachability_starts_unknown_then_tracks_reports() {
        let substrate = DefaultSubstrate::new();
        assert_eq!(
            substrate.reachability().await.expect("reach"),
            Reachability::unknown()
        );

        let observed = Reachability {
            public_v6: None,
            path: PathKind::Direct,
        };
        substrate.set_reachability(observed);
        assert_eq!(substrate.reachability().await.expect("reach"), observed);
    }

    /// Phase B methods are explicitly unsupported in Phase A.
    #[tokio::test]
    async fn phase_b_methods_unsupported() {
        let substrate = DefaultSubstrate::new();
        assert!(matches!(
            substrate.active_interface().await,
            Err(SubstrateError::Unsupported(_))
        ));
        assert!(matches!(
            substrate.watch_links().await,
            Err(SubstrateError::Unsupported(_))
        ));
    }
}
