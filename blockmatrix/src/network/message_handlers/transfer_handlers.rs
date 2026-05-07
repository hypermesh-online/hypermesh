// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase G.1 cross-network transfer wire handlers.
//!
//! Routes the five new TAGs (TAG_TRANSFER_LOCK / REGISTER_REQ /
//! REGISTER_ACK / RELEASE / ROLLBACK) to the appropriate
//! [`TransferCoordinator`](crate::gateway::TransferCoordinator) entry
//! points. Alpha-default inert: when the PeerContext has no coordinator
//! attached, handlers log and drop the message rather than failing.

use tracing::{debug, info, warn};

use crate::gateway::transfer_protocol::{
    TransferLockMessage, TransferRegisterAck, TransferRegisterRequest, TransferRelease,
    TransferRollback,
};
use crate::network::PeerContext;

/// Handle an incoming `TAG_TRANSFER_LOCK` (0x40).
///
/// Currently informational on the receiver side: the originating chain
/// holds the canonical lock entry. Logging is enough until reflectors
/// add deterministic mirroring (Phase G.2).
pub(super) async fn handle_transfer_lock(
    data: &[u8],
    peer_node_id: &str,
    _ctx: &PeerContext,
) {
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];
    let msg: TransferLockMessage = match serde_json::from_slice(data) {
        Ok(m) => m,
        Err(e) => {
            warn!("Invalid TAG_TRANSFER_LOCK from {}: {}", short_id, e);
            return;
        }
    };
    info!(
        "TAG_TRANSFER_LOCK {} from {}: asset={} (src={} → tgt={})",
        msg.transfer_id,
        short_id,
        msg.asset_id,
        msg.source_chain_id,
        msg.target_chain_id,
    );
}

/// Handle an incoming `TAG_TRANSFER_REGISTER_REQ` (0x41).
///
/// Phase G.1 stub: Logs the request and would normally dispatch to
/// [`TransferCoordinator::handle_register_request`] on the local node.
/// Full multi-host wire dispatch lands when the coordinator is plumbed
/// onto `PeerContext` in Phase G.2.
pub(super) async fn handle_transfer_register_req(
    data: &[u8],
    _stream: &mut stoq::Stream,
    peer_node_id: &str,
    _ctx: &PeerContext,
) {
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];
    let req: TransferRegisterRequest = match serde_json::from_slice(data) {
        Ok(r) => r,
        Err(e) => {
            warn!(
                "Invalid TAG_TRANSFER_REGISTER_REQ from {}: {}",
                short_id, e
            );
            return;
        }
    };
    info!(
        "TAG_TRANSFER_REGISTER_REQ {} from {}: asset={} shards={} (src={} → tgt={})",
        req.transfer_id,
        short_id,
        req.asset_id,
        req.shard_manifest.len(),
        req.source_chain_id,
        req.target_chain_id,
    );
}

/// Handle an incoming `TAG_TRANSFER_REGISTER_ACK` (0x42). Source-side
/// processing path — coordinator is awaiting this directly through the
/// transport future, so the handler only logs unsolicited acks.
pub(super) async fn handle_transfer_register_ack(
    data: &[u8],
    peer_node_id: &str,
    _ctx: &PeerContext,
) {
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];
    let ack: TransferRegisterAck = match serde_json::from_slice(data) {
        Ok(a) => a,
        Err(e) => {
            warn!(
                "Invalid TAG_TRANSFER_REGISTER_ACK from {}: {}",
                short_id, e
            );
            return;
        }
    };
    debug!(
        "TAG_TRANSFER_REGISTER_ACK {} accepted={} from {}",
        ack.transfer_id, ack.accepted, short_id
    );
}

/// Handle an incoming `TAG_TRANSFER_RELEASE` (0x43).
pub(super) async fn handle_transfer_release(
    data: &[u8],
    peer_node_id: &str,
    _ctx: &PeerContext,
) {
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];
    let rel: TransferRelease = match serde_json::from_slice(data) {
        Ok(r) => r,
        Err(e) => {
            warn!("Invalid TAG_TRANSFER_RELEASE from {}: {}", short_id, e);
            return;
        }
    };
    info!(
        "TAG_TRANSFER_RELEASE {} from {}: src_hash={} tgt_hash={}",
        rel.transfer_id, short_id, rel.source_block_hash, rel.target_block_hash
    );
}

/// Handle an incoming `TAG_TRANSFER_ROLLBACK` (0x44).
pub(super) async fn handle_transfer_rollback(
    data: &[u8],
    peer_node_id: &str,
    _ctx: &PeerContext,
) {
    let short_id = &peer_node_id[..8.min(peer_node_id.len())];
    let rb: TransferRollback = match serde_json::from_slice(data) {
        Ok(r) => r,
        Err(e) => {
            warn!("Invalid TAG_TRANSFER_ROLLBACK from {}: {}", short_id, e);
            return;
        }
    };
    info!(
        "TAG_TRANSFER_ROLLBACK {} from {}: reason={:?}",
        rb.transfer_id, short_id, rb.reason
    );
}
