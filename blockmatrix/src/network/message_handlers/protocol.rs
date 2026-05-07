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
/// CRL fetch request (federated revocation lookup, Phase F.2).
pub(crate) const TAG_CRL_REQUEST: u8 = 0x33;
/// CRL fetch response (federated revocation lookup, Phase F.2).
pub(crate) const TAG_CRL_RESPONSE: u8 = 0x34;
/// Key rotation announcement (informational, not auth-gated).
pub(crate) const TAG_KEY_ROTATION: u8 = 0x08;
/// DNS resolution request (network fallback when local DNS misses).
pub(crate) const TAG_DNS_RESOLVE: u8 = 0x09;
/// DNS resolution response (reply with address or empty).
pub(crate) const TAG_DNS_RESOLVE_RESPONSE: u8 = 0x0A;

// ── Phase G.1: cross-network transfer choreography ───────────────────
//
// The plan reserved 0x10-0x14, but 0x10 (TAG_SYNC_MESSAGE) and 0x11
// (TAG_BLOCK_FETCH_REQUEST) are already in use. Phase G.1 instead uses
// the next free contiguous range, 0x40-0x44, for the five transfer
// wire tags. Slots 0x40-0x44 are documented in
// `gateway::transfer_protocol` and routed in `peer_connection.rs`.

/// Cross-network transfer: source-side broadcast that an asset has been
/// locked and is being prepared for migration to the target chain.
pub(crate) const TAG_TRANSFER_LOCK: u8 = 0x40;
/// Cross-network transfer: source-side request to register the asset on
/// the target chain (carries shard manifest + lock proof).
pub(crate) const TAG_TRANSFER_REGISTER_REQ: u8 = 0x41;
/// Cross-network transfer: target-side acknowledgement that the asset
/// has been registered on the target chain (carries target block hash).
pub(crate) const TAG_TRANSFER_REGISTER_ACK: u8 = 0x42;
/// Cross-network transfer: source-side broadcast that the lock has been
/// released and the transfer is complete (carries target block hash).
pub(crate) const TAG_TRANSFER_RELEASE: u8 = 0x43;
/// Cross-network transfer: rollback notification (timeout or rejection)
/// — both sides should restore pre-transfer state.
pub(crate) const TAG_TRANSFER_ROLLBACK: u8 = 0x44;

// ── Phase H.1: distributed DNS query/response ─────────────────────────
//
// The legacy `TAG_DNS_RESOLVE` (0x09) / `TAG_DNS_RESOLVE_RESPONSE`
// (0x0A) carry only `name → IPv6 string`, which is enough for the
// flat bootstrap resolver but NOT enough for cross-node conflict
// resolution. Phase H.1 adds richer query/response types in the
// 0x50/0x51 slot — slots 0x45-0x4F are reserved for any further
// transfer-protocol extensions, keeping H.1 cleanly separated.
//
// Wire payloads are JSON-serialized [`DistributedDnsQuery`] and
// [`DistributedDnsResponse`] from `dns_protocol.rs`.

/// DNS query (rich): correlation ID + domain name. Carries enough
/// metadata in the matching response for canonical-winner selection
/// across multiple peers.
pub(crate) const TAG_DNS_QUERY: u8 = 0x50;
/// DNS response (rich): records + chain metadata + foundation-grant
/// flag for cross-node conflict resolution.
pub(crate) const TAG_DNS_RESPONSE: u8 = 0x51;
