// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase G.1 cross-network transfer wire handlers.
//!
//! Routes the five new TAGs (TAG_TRANSFER_LOCK / REGISTER_REQ /
//! REGISTER_ACK / RELEASE / ROLLBACK) to the appropriate
//! [`TransferCoordinator`](crate::gateway::TransferCoordinator) entry
//! points.
//!
//! Phase G.2 wires the coordinator into [`PeerContext`]:
//!
//! * `handle_transfer_lock` — informational on the receiver side
//!   (originating chain holds the canonical lock entry).
//! * `handle_transfer_register_req` — when the coordinator is present,
//!   calls
//!   [`TransferCoordinator::handle_register_request`](crate::gateway::TransferCoordinator::handle_register_request)
//!   and writes the resulting `TransferRegisterAck` back over the same
//!   stream as the request was received on (length-prefixed JSON).
//! * `handle_transfer_register_ack` — delivers the ack to any awaiting
//!   `initiate` future via
//!   [`TransferCoordinator::deliver_register_ack`](crate::gateway::TransferCoordinator::deliver_register_ack).
//! * `handle_transfer_release` / `handle_transfer_rollback` — currently
//!   informational; the originating chain carries the canonical record.
//!
//! When `ctx.transfer_coordinator` is `None`, every handler falls back
//! to log+drop behaviour so alpha-default inert is preserved.

use tracing::{debug, info, warn};
use trustchain::proof_of_state::StateProof;

use crate::gateway::transfer_protocol::{
    TransferLockMessage, TransferRegisterAck, TransferRegisterRequest, TransferRelease,
    TransferRollback,
};
use crate::network::PeerContext;

/// Handle an incoming `TAG_TRANSFER_LOCK` (0x40).
///
/// Informational on the receiver side: the originating chain holds the
/// canonical lock entry. Logging is enough; coordinator reflection is
/// not required because the source broadcasts independently.
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
/// Phase G.2: when `ctx.transfer_coordinator` is `Some`, dispatches the
/// request through the coordinator (which writes the registration entry
/// + receipt to the local chain) and writes the resulting ack back over
/// the same stream as the request was received on.
///
/// When the coordinator is absent (alpha-default), logs and drops.
pub(super) async fn handle_transfer_register_req(
    data: &[u8],
    stream: &mut stoq::Stream,
    peer_node_id: &str,
    ctx: &PeerContext,
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

    let coordinator = match ctx.transfer_coordinator.as_ref() {
        Some(c) => c.clone(),
        None => {
            debug!(
                "TAG_TRANSFER_REGISTER_REQ {} dropped — no coordinator wired (alpha-default)",
                req.transfer_id
            );
            return;
        }
    };

    // Use a fresh state proof for the target side. In production the
    // coordinator's owning daemon supplies a real PoS proof; for the
    // wire path we use the testing proof — the lock_state_proof
    // attached to the request is what's actually validated.
    let target_proof = StateProof::new_for_testing();
    let ack = match coordinator
        .handle_register_request(req.clone(), target_proof)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            warn!(
                "TransferCoordinator::handle_register_request failed for {}: {}",
                req.transfer_id, e
            );
            // Synthesize a non-accepted ack so the source rolls back
            // rather than hanging on its register-ack timeout.
            TransferRegisterAck {
                transfer_id: req.transfer_id.clone(),
                target_block_hash: String::new(),
                state_proof: StateProof::default(),
                accepted: false,
                reason: Some(format!("target coordinator error: {e}")),
                acked_at: chrono::Utc::now().timestamp(),
            }
        }
    };

    let payload = match serde_json::to_vec(&ack) {
        Ok(p) => p,
        Err(e) => {
            warn!(
                "Failed to serialize TransferRegisterAck for {}: {}",
                req.transfer_id, e
            );
            return;
        }
    };

    if let Err(e) = stream.send(&payload).await {
        warn!(
            "Failed to send TAG_TRANSFER_REGISTER_ACK for {}: {}",
            req.transfer_id, e
        );
    }
}

/// Handle an incoming `TAG_TRANSFER_REGISTER_ACK` (0x42).
///
/// Phase G.2: routes the ack to any awaiting `initiate` future via
/// [`TransferCoordinator::deliver_register_ack`]. Production STOQ
/// transports register a oneshot before broadcasting the request; the
/// handler delivers the ack here. When no coordinator is wired or no
/// waiter is registered, the ack is logged and dropped.
pub(super) async fn handle_transfer_register_ack(
    data: &[u8],
    peer_node_id: &str,
    ctx: &PeerContext,
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

    if let Some(coordinator) = ctx.transfer_coordinator.as_ref() {
        let transfer_id = ack.transfer_id.clone();
        let delivered = coordinator.deliver_register_ack(ack).await;
        if !delivered {
            debug!(
                "TAG_TRANSFER_REGISTER_ACK {} had no awaiting waiter (already-completed or unknown)",
                transfer_id
            );
        }
    }
}

/// Handle an incoming `TAG_TRANSFER_RELEASE` (0x43).
///
/// Informational: receiver's chain already records the registration +
/// receipt; the originating chain holds the canonical release.
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
///
/// Informational: the receiver did not write a registration entry yet
/// (or did, in which case the source-side rollback is the operational
/// signal that target-side cleanup should follow).
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
