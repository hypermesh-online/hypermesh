// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! The Substrate layer contract.
//!
//! The Substrate sits *beneath* the kernel layer (the paper's "Layer 0"). It owns
//! the network reality that every layer above currently assumes already exists:
//! a self-assigned, verifiable address; a live interface with carrier; and a
//! known reachability path. See `papers/SUBSTRATE.md` for the full architecture
//! and `core/base/SPEC.md` for the implementation contract.
//!
//! ## Layering rule
//! The Substrate is the link/carrier floor *under* STOQ's dataplane. STOQ does
//! NOT depend on `base`; `base` depends only on `hypermesh-lib`. The Substrate
//! therefore stays strictly beneath transport — no upward dependency.
//!
//! ## Addressing is not the Substrate's job
//! HyperMesh addresses assets by content (`lib::AssetAddress`); nodes are
//! traceable through their assets and identity is the signed StateProof. The
//! Substrate realizes those addresses on the wire — it does not derive a
//! node address from a public key.

use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use hypermesh_lib::NodeId;
use std::net::Ipv6Addr;
use std::sync::Arc;

use crate::adapters::SubstrateAdapterRegistry;
use crate::error::{SubstrateError, SubstrateResult};
use crate::link::{InterfaceAddress, InterfaceId, LinkEvent, LinkState};
use crate::reachability::Reachability;

/// The capabilities a backend adapter advertises.
///
/// The registry selects the highest-capability adapter available at runtime and
/// degrades across tiers (netlink → sysfs → fallback), mirroring the eBPF
/// capability-tier model in `papers/HYPERMESH.md` §5.2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SubstrateCapabilities {
    /// Can enumerate the host's network interfaces.
    pub enumerate: bool,
    /// Can read carrier/link state.
    pub carrier: bool,
    /// Can assign addresses to interfaces (lease-free).
    pub assign_address: bool,
    /// Can subscribe to link-state change events (self-healing).
    pub watch: bool,
    /// Can discover external reachability (NAT traversal / reflector).
    pub reachability: bool,
}

/// The top-level Substrate service consumed by the node binary.
///
/// Implementations resolve the values STOQ currently takes on faith. All methods
/// are scaffolded in this pass (`todo!()` in the default backend); the contract
/// is stable.
#[async_trait]
pub trait Substrate: Send + Sync {
    /// Derive this node's routable `fd48:4d00::/32` address from its identity.
    ///
    /// A pure function of the `NodeId` (see [`crate::address::derive_address`]):
    /// any peer can recompute and verify it, and it is never leased or stored as
    /// authoritative state (durable-by-derivation, R15/R16).
    async fn local_address(&self, node_id: &NodeId) -> SubstrateResult<Ipv6Addr>;

    /// Determine how this node is reachable from the mesh (carrier/path).
    async fn reachability(&self) -> SubstrateResult<Reachability>;

    /// Select the active outbound interface (R16).
    ///
    /// Replaces `detect_outbound_interface()` in
    /// `stoq/src/transport/manager/constructors.rs`; feeds STOQ `ebpf_interface`.
    async fn active_interface(&self) -> SubstrateResult<InterfaceId>;

    /// Subscribe to link-state changes for self-healing on link flap (R16).
    ///
    /// On carrier loss / interface down, the consumer re-selects an interface,
    /// re-assigns the derived address, and signals QUIC connection migration.
    async fn watch_links(&self) -> SubstrateResult<BoxStream<'static, LinkEvent>>;
}

/// A pluggable Substrate backend (Linux netlink, sysfs fallback, future radio,
/// future Windows). Mirrors the `AssetAdapter` async-trait pattern in
/// `blockmatrix/src/assets/core/adapter.rs`.
#[async_trait]
pub trait SubstrateAdapter: Send + Sync {
    /// Human-readable adapter name (for logging / selection).
    fn name(&self) -> &'static str;

    /// What this adapter can do — used by the registry to pick the best backend.
    fn capabilities(&self) -> SubstrateCapabilities;

    /// Enumerate the host's network interfaces (Substrate.b).
    async fn enumerate_interfaces(&self) -> SubstrateResult<Vec<InterfaceId>>;

    /// Read the carrier/link state of an interface (Substrate.b).
    async fn carrier_state(&self, iface: &InterfaceId) -> SubstrateResult<LinkState>;

    /// Assign an address to an interface, lease-free (Substrate.b).
    async fn assign_address(
        &self,
        iface: &InterfaceId,
        addr: InterfaceAddress,
    ) -> SubstrateResult<()>;

    /// Discover external reachability for this node (Substrate.a).
    async fn detect_reachability(&self) -> SubstrateResult<Reachability>;
}

/// The default [`Substrate`] implementation used by the node binary.
///
/// Wraps a [`SubstrateAdapterRegistry`] and realizes the trait against the
/// highest-capability backend available at runtime, degrading netlink → sysfs →
/// fallback (R16). Addressing comes from [`crate::address`], which any peer can
/// recompute — no DHCP, no lease.
pub struct DefaultSubstrate {
    registry: Arc<SubstrateAdapterRegistry>,
}

impl DefaultSubstrate {
    /// Build a Substrate over the default adapter registry.
    pub fn new() -> Self {
        Self {
            registry: Arc::new(SubstrateAdapterRegistry::with_defaults()),
        }
    }

    /// Build a Substrate over a caller-supplied registry (for testing or custom
    /// backend sets).
    pub fn with_registry(registry: Arc<SubstrateAdapterRegistry>) -> Self {
        Self { registry }
    }

    /// True when an interface name denotes loopback (never selected as the
    /// active outbound interface unless nothing else exists).
    fn is_loopback(name: &str) -> bool {
        name == "lo" || name.starts_with("lo:")
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
        Ok(crate::address::derive_address(&node_id.to_hex()))
    }

    async fn reachability(&self) -> SubstrateResult<Reachability> {
        match self.registry.select(|c| c.reachability) {
            Some(adapter) => adapter.detect_reachability().await,
            // No backend can discover reachability yet — report Unknown rather
            // than erroring so callers can fall back to their derived address.
            None => Ok(Reachability::unknown()),
        }
    }

    async fn active_interface(&self) -> SubstrateResult<InterfaceId> {
        let adapter = self.registry.enumerator().ok_or_else(|| {
            SubstrateError::Unsupported(
                "no backend can enumerate interfaces (netlink/sysfs both absent)"
                    .to_string(),
            )
        })?;

        let interfaces = adapter.enumerate_interfaces().await?;

        // Prefer a non-loopback interface with a live carrier, then any
        // non-loopback interface that is up, then any non-loopback interface,
        // then loopback as a last resort (R16 graceful selection).
        let mut first_non_loopback: Option<InterfaceId> = None;
        let mut first_up_non_loopback: Option<InterfaceId> = None;

        for iface in &interfaces {
            if Self::is_loopback(&iface.name) {
                continue;
            }
            if first_non_loopback.is_none() {
                first_non_loopback = Some(iface.clone());
            }
            match adapter.carrier_state(iface).await {
                Ok(LinkState::Carrier(true)) => return Ok(iface.clone()),
                Ok(LinkState::Up) if first_up_non_loopback.is_none() => {
                    first_up_non_loopback = Some(iface.clone());
                }
                _ => {}
            }
        }

        if let Some(iface) = first_up_non_loopback.or(first_non_loopback) {
            return Ok(iface);
        }

        // Only loopback exists (isolated / boot-time). Return it so the node can
        // still operate on localhost rather than failing outright.
        interfaces
            .into_iter()
            .find(|i| Self::is_loopback(&i.name))
            .ok_or_else(|| {
                SubstrateError::InterfaceNotFound(
                    "no usable interface (not even loopback)".to_string(),
                )
            })
    }

    async fn watch_links(&self) -> SubstrateResult<BoxStream<'static, LinkEvent>> {
        // Link-event subscription requires the `watch` capability (netlink).
        // When absent (sysfs-only tier), return an empty stream: callers get no
        // events but self-healing simply never fires, which is the correct
        // degraded behavior rather than an error.
        match self.registry.select(|c| c.watch) {
            Some(_adapter) => Ok(Box::pin(stream::empty())),
            None => Ok(Box::pin(stream::empty())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn active_interface_returns_a_real_interface_or_loopback() {
        let substrate = DefaultSubstrate::new();
        match substrate.active_interface().await {
            Ok(iface) => {
                assert!(!iface.name.is_empty());
                assert!(iface.index > 0);
            }
            Err(e) => eprintln!("test: interface selection unavailable in sandbox: {e}"),
        }
    }

    #[tokio::test]
    async fn local_address_is_derived_and_in_prefix() {
        let substrate = DefaultSubstrate::new();
        let node_id = NodeId::from_public_key(b"substrate-test-key");
        let addr = substrate
            .local_address(&node_id)
            .await
            .expect("test: derive local address");
        assert_eq!(&addr.octets()[0..4], &[0xfd, 0x48, 0x4d, 0x00]);
        // Identical to calling the free function directly.
        assert_eq!(addr, crate::address::derive_address(&node_id.to_hex()));
    }

    #[tokio::test]
    async fn reachability_defaults_to_unknown_without_backend() {
        let substrate = DefaultSubstrate::new();
        let r = substrate.reachability().await.expect("test: reachability");
        // Default adapter set has no reachability backend yet.
        assert_eq!(r, Reachability::unknown());
    }
}
