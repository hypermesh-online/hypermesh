// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! # HyperMesh Substrate (`base`)
//!
//! The Substrate is the network layer *beneath* the kernel. The paper defines
//! "Layer 0" as the OS kernel (eBPF/XDP/AF_XDP); the Substrate sits below even
//! that — it owns the physical/link reality, the self-assigned address, and the
//! reachability path that every layer above currently assumes already exists.
//!
//! It closes the seam exposed by an interface bounce: STOQ presumes a routable
//! IPv6 on a live carrier (`stoq/src/transport/config.rs` `bind_address`,
//! `public_ipv6`), and the interface is a hardcoded guess
//! (`stoq/src/transport/manager/constructors.rs` `detect_outbound_interface()`).
//! The Substrate produces those values sovereignly instead of borrowing them from
//! the incumbent network.
//!
//! ## Scope (phased)
//! - **Substrate.a** — sovereign addressing & reachability ([`address`],
//!   [`reachability`]). Derives a verifiable `fd48:4d00::/32` address from the
//!   node's identity (R15). *Phase 1.*
//! - **Substrate.b** — link/carrier/interface management ([`link`]). Enumerates
//!   interfaces, monitors carrier, assigns addresses lease-free, self-heals on
//!   link flap (R16). *Phase 2.*
//! - **Substrate.c** — physical/radio (zero ISP). Roadmap stub
//!   ([`adapters::radio_mesh`]); not built.
//!
//! ## Layering
//! `base` depends ONLY on `hypermesh-lib`. STOQ does not depend on `base`; the
//! node binary injects Substrate-derived values into STOQ's `TransportConfig`.
//!
//! ## Status
//! This crate is a **scaffold**: trait contracts and types are stable; backend
//! method bodies are `todo!()`/`Unsupported` until Phase 1/2. See
//! `papers/SUBSTRATE.md` (canonical) and `core/base/SPEC.md` (contract).

pub mod adapters;
pub mod address;
pub mod error;
pub mod link;
pub mod reachability;
pub mod substrate;

pub use adapters::SubstrateAdapterRegistry;
pub use address::{derive_address, verify_address, HYPERMESH_PREFIX, SUBNET_DEVICE_SCOPE};
pub use error::{SubstrateError, SubstrateResult};
pub use link::{InterfaceAddress, InterfaceId, LinkEvent, LinkState};
pub use reachability::{PathKind, Reachability};
pub use substrate::{Substrate, SubstrateAdapter, SubstrateCapabilities};
