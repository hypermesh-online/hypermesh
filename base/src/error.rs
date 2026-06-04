// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Substrate error type and result alias.

use thiserror::Error;

/// Errors produced by the Substrate layer.
///
/// The Substrate sits beneath the kernel layer (the paper's "Layer 0"); these
/// errors describe failures in self-assigned addressing, link/carrier management,
/// and reachability discovery — the inputs the transport layer (STOQ) currently
/// assumes already exist.
#[derive(Debug, Error)]
pub enum SubstrateError {
    /// No backend adapter advertised the capability required for this operation.
    /// The registry degrades across tiers (netlink → sysfs → fallback); this fires
    /// when even the fallback cannot satisfy the request.
    #[error("no substrate adapter supports the required capability: {0}")]
    Unsupported(String),

    /// Address derivation from a `NodeId` failed (e.g. malformed identity).
    #[error("address derivation failed: {0}")]
    AddressDerivation(String),

    /// The requested network interface was not found or could not be enumerated.
    #[error("interface not found: {0}")]
    InterfaceNotFound(String),

    /// Link/carrier operation failed (bring-up, address assignment, monitoring).
    #[error("link operation failed: {0}")]
    Link(String),

    /// Reachability / path discovery failed.
    #[error("reachability discovery failed: {0}")]
    Reachability(String),

    /// Underlying OS / netlink error surfaced by a backend adapter.
    #[error("backend error: {0}")]
    Backend(String),
}

/// Convenience result alias for Substrate operations.
pub type SubstrateResult<T> = Result<T, SubstrateError>;
