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
//! The node binary constructs a `Substrate` and *injects* the resolved
//! `bind_address` / `public_ipv6` / `ebpf_interface` into STOQ's `TransportConfig`.
//! STOQ does NOT depend on `base`; `base` depends only on `hypermesh-lib`. The
//! Substrate therefore stays strictly beneath transport — no upward dependency.

use async_trait::async_trait;
use futures::stream::BoxStream;
use hypermesh_lib::NodeId;
use std::net::Ipv6Addr;

use crate::error::SubstrateResult;
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
    /// Derive this node's sovereign `fd48:4d00::/32` address from its identity (R15).
    ///
    /// Feeds STOQ `bind_address` (`stoq/src/transport/config.rs`).
    async fn local_address(&self, node_id: &NodeId) -> SubstrateResult<Ipv6Addr>;

    /// Determine how this node is reachable from the mesh (R15 reachability half).
    ///
    /// Feeds STOQ `public_ipv6`.
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
