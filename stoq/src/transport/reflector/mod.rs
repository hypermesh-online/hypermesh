// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Reflector Pool Transport for Network-scope blockchain synchronization.
//!
//! Provides the STOQ-side transport layer for reflector-based chain sync,
//! including heartbeat/health tracking, replication threshold detection, a sync protocol
//! state machine, message serialization, and a bridge to blockmatrix's
//! `MatrixMessage` system.
//!
//! # Architecture
//!
//! - [`ReflectorMessage`] -- wire-format messages exchanged between reflector
//!   pool nodes, serialized with length-prefixed bincode.
//! - [`StoqBlockTransport`] -- transport layer built on an outbox/inbox
//!   pattern that decouples the protocol from real STOQ network I/O,
//!   enabling testability and future stream integration.
//! - [`SyncProtocol`] -- state machine for heartbeats, replication threshold,
//!   sync requests, block announcements, and stale-peer pruning.
//! - [`ReflectorBridge`] -- bidirectional converter between STOQ
//!   `ReflectorMessage` and blockmatrix `BridgedMatrixMessage`.

pub mod block_transport;
pub mod bridge;
pub mod message;
pub mod sync_protocol;

pub use block_transport::StoqBlockTransport;
pub use bridge::{BridgedMatrixMessage, ReflectorBridge};
pub use message::ReflectorMessage;
pub use sync_protocol::{SyncProtocol, SyncProtocolConfig};

/// Check whether a reflector should be considered stale based on how
/// long ago it was last seen.
///
/// Returns `false` (unhealthy) when `last_seen_secs >= timeout_secs`,
/// meaning the reflector has not been heard from within the timeout.
#[inline]
pub fn check_reflector_health(last_seen_secs: u64, timeout_secs: u64) -> bool {
    last_seen_secs < timeout_secs
}
