// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Peer message dispatch and block/shard/sync handlers.
//!
//! Split into submodules to keep each file under 500 lines:
//! - `protocol`: wire-protocol tag bytes
//! - `peer_connection`: incoming connection routing, handshake, message loop, dispatch
//! - `block_handlers`: block announcement, propagation, DNS extraction
//! - `sync_and_reflection`: sync messages, reflector registration, shard/transfer/DNS/invite/etc.
//! - `distributed_ca`: distributed CA key share + threshold signing
//! - `message_utils`: sync reply, DNS peer resolution, metrics, gossip

mod block_handlers;
mod distributed_ca;
mod dns_protocol;
mod message_utils;
mod peer_connection;
mod protocol;
mod sync_and_reflection;
mod transfer_handlers;

// Wire-protocol tags — re-exported for other crate modules that drive sends.
pub(crate) use protocol::{
    TAG_BLOCK_ANNOUNCE, TAG_BLOCK_FETCH_REQUEST, TAG_CA_KEY_SHARE, TAG_CA_SIGN_REQUEST,
    TAG_CA_SIGN_RESPONSE, TAG_DIRECT_MESSAGE, TAG_DNS_QUERY, TAG_DNS_RESOLVE,
    TAG_DNS_RESOLVE_RESPONSE, TAG_DNS_RESPONSE, TAG_GOSSIP, TAG_KEY_ROTATION,
    TAG_SHARD_ANNOUNCE, TAG_SHARD_FETCH, TAG_SHARD_SEND, TAG_SHARE_INVITE, TAG_SYNC_MESSAGE,
    TAG_TRANSFER, TAG_TRANSFER_LOCK, TAG_TRANSFER_REGISTER_ACK, TAG_TRANSFER_REGISTER_REQ,
    TAG_TRANSFER_RELEASE, TAG_TRANSFER_ROLLBACK,
};

// Distributed DNS wire types — used by the resolver and the H.1 query handler.
pub use dns_protocol::{select_canonical, DistributedDnsQuery, DistributedDnsResponse};

// Public entry points used by the network module.
pub(crate) use peer_connection::{
    dispatch_message, handle_incoming_connection, run_peer_message_loop,
};

// DNS network fallback used by the DNS resolver.
pub use message_utils::{distributed_dns_resolve, resolve_from_network};

// Share-invite parse/store core (F5 wire delivery) — the pure parse-and-store
// path shared between the receiver dispatch and the `share.send` loopback test.
// Only surfaced to the crate for the loopback framing test; the production
// receiver reaches it directly within `sync_and_reflection`.
#[cfg(test)]
pub(crate) use sync_and_reflection::parse_and_store_share_invite;
