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
//! Addressing is **not** the Substrate's job. HyperMesh addresses assets by
//! content (`lib::AssetAddress`: BLAKE3 + matrix coordinate); nodes are traceable
//! through the assets they hold, and identity is the signed StateProof — never a
//! hash-of-pubkey IPv6. The Substrate realizes content/PoS addresses on the wire;
//! it does not invent them.
//!
//! ## Scope (phased)
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
pub mod error;
pub mod link;
pub mod reachability;
pub mod substrate;

pub use adapters::SubstrateAdapterRegistry;
pub use error::{SubstrateError, SubstrateResult};
pub use link::{InterfaceAddress, InterfaceId, LinkEvent, LinkState};
pub use reachability::{PathKind, Reachability};
pub use substrate::{Substrate, SubstrateAdapter, SubstrateCapabilities};
