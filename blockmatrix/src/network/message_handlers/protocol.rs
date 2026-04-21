// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Wire-protocol tag bytes used by peer messages.

// ── Wire-protocol tag bytes ──────────────────────────────────────────

/// Shard send (store a shard on this node).
pub(crate) const TAG_SHARD_SEND: u8 = 0x01;
/// Shard fetch (retrieve a shard from this node).
pub(crate) const TAG_SHARD_FETCH: u8 = 0x02;
/// Block announcement.
pub(crate) const TAG_BLOCK_ANNOUNCE: u8 = 0x03;
/// Sync / reflector message.
pub(crate) const TAG_SYNC_MESSAGE: u8 = 0x10;
/// Block fetch request (pull specific blocks by hash).
pub(crate) const TAG_BLOCK_FETCH_REQUEST: u8 = 0x11;
/// Shard availability announcement (consumer-becomes-provider, R12).
pub(crate) const TAG_SHARD_ANNOUNCE: u8 = 0x04;
/// Share invite (P2P file sharing).
pub(crate) const TAG_SHARE_INVITE: u8 = 0x05;
/// Direct message (P2P encrypted messaging).
pub(crate) const TAG_DIRECT_MESSAGE: u8 = 0x06;
/// Cross-network asset transfer request/response.
pub(crate) const TAG_TRANSFER: u8 = 0x07;
/// Gossip protocol message.
pub(crate) const TAG_GOSSIP: u8 = 0x20;
/// CA key share distribution (distributed CA).
pub(crate) const TAG_CA_KEY_SHARE: u8 = 0x30;
/// Threshold signing request (distributed CA).
pub(crate) const TAG_CA_SIGN_REQUEST: u8 = 0x31;
/// Threshold signing response (distributed CA).
pub(crate) const TAG_CA_SIGN_RESPONSE: u8 = 0x32;
/// Key rotation announcement (informational, not auth-gated).
pub(crate) const TAG_KEY_ROTATION: u8 = 0x08;
/// DNS resolution request (network fallback when local DNS misses).
pub(crate) const TAG_DNS_RESOLVE: u8 = 0x09;
/// DNS resolution response (reply with address or empty).
pub(crate) const TAG_DNS_RESOLVE_RESPONSE: u8 = 0x0A;
