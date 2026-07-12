// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! # HyperMesh Substrate (`base`) — link/carrier floor
//!
//! The Substrate is the link/carrier floor *beneath* STOQ's dataplane. It owns
//! the physical/link reality — interface enumeration, carrier monitoring, and
//! self-healing on link flap — that the transport layer currently assumes
//! already exists.
//!
//! It closes the seam exposed by an interface bounce: STOQ presumes a live
//! carrier and a working interface (`stoq/src/transport/manager/constructors.rs`
//! `detect_outbound_interface()` is a hardcoded guess). The Substrate manages
//! that link floor instead of borrowing it from the incumbent network.
//!
//! Addressing IS realized here, but not invented here. HyperMesh addresses
//! assets by content (`lib::AssetAddress`: BLAKE3 + matrix coordinate); nodes are
//! traceable through the assets they hold and identity is the signed StateProof.
//! The Substrate owns the ONE derivation ([`address`]) that turns a `NodeId` into
//! its matrix cell and its `fd48:4d00::/32` address and then realizes that address
//! on the wire (lease-free, no DHCP). It composes existing `lib` primitives; it
//! does not mint a new addressing scheme.
//!
//! ## Scope (phased)
//! - **Sovereign addressing** ([`address`]). Derive a node's matrix cell and
//!   routable `fd48:4d00::/32` address from its identity — the single canonical
//!   construction `blockmatrix`'s `MatrixCoordinate::derive_cell` delegates to.
//! - **Link/carrier/interface management** ([`link`]). Enumerate interfaces,
//!   monitor carrier, self-heal on link flap (R16).
//! - **Physical/radio** (zero ISP). Roadmap stub ([`adapters::radio_mesh`]); not
//!   built.
//!
//! ## Layering
//! `base` depends ONLY on `hypermesh-lib`. STOQ does not depend on `base`; the
//! link/carrier floor sits under STOQ's dataplane.
//!
//! ## Status
//! This crate is a **scaffold**: trait contracts and types are stable; backend
//! method bodies are `todo!()`/`Unsupported` until the link/carrier reconciler
//! lands. See `papers/SUBSTRATE.md` (canonical) and `core/base/SPEC.md` (contract).

pub mod adapters;
pub mod address;
pub mod error;
pub mod link;
pub mod reachability;
pub mod substrate;

pub use adapters::SubstrateAdapterRegistry;
pub use address::{derive_address, derive_cell};
pub use error::{SubstrateError, SubstrateResult};
pub use link::{InterfaceAddress, InterfaceId, LinkEvent, LinkState};
pub use reachability::{PathKind, Reachability};
pub use substrate::{DefaultSubstrate, Substrate, SubstrateAdapter, SubstrateCapabilities};
