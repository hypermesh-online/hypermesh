// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Substrate.b — link, carrier, and interface types.
//!
//! These types describe the network interfaces a node owns and their live state.
//! They replace the hardcoded interface guess in
//! `stoq/src/transport/manager/constructors.rs` (`detect_outbound_interface()`,
//! which probes `[eth0, ens3, ens4, enp0s3, wlan0, wlp2s0]` and falls back to
//! `lo`) with enumerated, carrier-aware selection (R16).
//!
//! NOTE: types are defined here; the netlink/sysfs machinery that produces them
//! lives in the adapters and is a **scaffold** this pass.

use std::net::Ipv6Addr;

/// A network interface identified by kernel index and name.
///
/// `index` is the value returned by `libc::if_nametoindex` (already used in
/// `hypermesh-ebpf/src/af_xdp/helpers.rs`); `name` is the kernel name (e.g.
/// `eno1`, `wlan0`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceId {
    /// Kernel interface index (`if_nametoindex`).
    pub index: u32,
    /// Kernel interface name.
    pub name: String,
}

/// Administrative + carrier state of an interface.
///
/// `Carrier(true)` means the link is up AND a carrier is present (the cable is
/// live / the radio is associated) — the distinction that matters for the
/// `eno1`-bounce problem: an interface can be administratively `Up` while having
/// no carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    /// Interface is administratively up.
    Up,
    /// Interface is administratively down.
    Down,
    /// Carrier presence on an up interface (`true` = carrier detected).
    Carrier(bool),
}

/// An observed change in link state, emitted by `Substrate::watch_links`.
///
/// Consumed by the self-healing path: on carrier loss / interface down, the node
/// re-selects an active interface, re-assigns its derived address, and signals
/// the transport layer for QUIC connection migration (R16).
#[derive(Debug, Clone)]
pub struct LinkEvent {
    /// The interface whose state changed.
    pub interface: InterfaceId,
    /// The new state.
    pub state: LinkState,
}

/// An IPv6 address assigned to an interface, with its prefix length.
///
/// For HyperMesh-derived addresses this carries a `fd48:4d00::/32` address
/// (see [`crate::address`]); the Substrate assigns it lease-free (no DHCP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterfaceAddress {
    /// The assigned address.
    pub addr: Ipv6Addr,
    /// Prefix length in bits.
    pub prefix_len: u8,
}
