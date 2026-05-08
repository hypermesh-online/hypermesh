// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase G.2 / I.1 — production [`TransferTransport`] backed by STOQ.
//!
//! Wraps a [`NetworkManager`] and a [`TransferCoordinator`] reference
//! so that:
//!
//! * `broadcast_lock` / `broadcast_release` / `broadcast_rollback`
//!   open a fresh STOQ stream to every connected peer and write the
//!   tag-prefixed payload (one stream per peer, best-effort delivery).
//!
//! * `send_register_request` opens a stream to a specific peer (looked
//!   up by `node_id` — `PeerCertFingerprint` is the node's hex
//!   identifier in this layer), writes
//!   `TAG_TRANSFER_REGISTER_REQ` + payload, then awaits the matching
//!   ack via the coordinator's `register_ack_waiter` oneshot.
//!
//! The wire-side handler in
//! [`crate::network::message_handlers::transfer_handlers`] receives
//! the ack on a different stream and resolves the oneshot via
//! `coordinator.deliver_register_ack`.
//!
//! # Why a `Weak<TransferCoordinator>`?
//!
//! `TransferCoordinator` owns an `Arc<dyn TransferTransport>`. If the
//! transport held a strong `Arc<TransferCoordinator>` we'd build a
//! reference cycle (coordinator → transport → coordinator). Holding
//! a `Weak` instead lets the coordinator drop cleanly during daemon
//! shutdown. Every wire call upgrades the weak to a strong; failure
//! to upgrade means the coordinator has shut down and the call
//! becomes a no-op.
//!
//! # Alpha-default inert
//!
//! Production daemons opt in to this transport by setting
//! `DaemonState::transfer_coordinator`. Without that wiring, the
//! `gateway.initiate_transfer` IPC handler returns
//! `CoordinatorNotConfigured` and no wire traffic is generated.

use std::sync::{Arc, Weak};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::gateway::transfer_coordinator::{TransferCoordinator, TransferTransport};
use crate::gateway::transfer_protocol::{
    PeerCertFingerprint, TransferLockMessage, TransferRegisterAck,
    TransferRegisterRequest, TransferRelease, TransferRollback,
};
use crate::gateway::GatewayError;
use crate::network::NetworkManager;

// Wire tag constants (mirrored from `network::message_handlers::protocol`,
// which is a private module). These MUST stay in lockstep with the
// canonical definitions there — adding a new tag without updating this
// list will leave the production transport silently inert for that tag.
const TAG_TRANSFER_LOCK: u8 = 0x40;
const TAG_TRANSFER_REGISTER_REQ: u8 = 0x41;
const TAG_TRANSFER_RELEASE: u8 = 0x43;
const TAG_TRANSFER_ROLLBACK: u8 = 0x44;

/// Production STOQ-backed transport.
///
/// Construct via [`Self::new`]; pass `Arc::new(...)` of the result to
/// [`TransferCoordinator::with_validator`] or
/// [`TransferCoordinator::new`].
pub struct StoqTransferTransport {
    /// Source of currently-connected peers (used for broadcasts and
    /// per-peer point-to-point sends).
    network: Arc<NetworkManager>,
    /// Backref to the coordinator owning this transport. Held as
    /// [`Weak`] to avoid an Arc cycle.
    coordinator: Weak<TransferCoordinator>,
}

impl StoqTransferTransport {
    /// Construct a transport wrapping the given network manager and
    /// owning coordinator. Pattern:
    ///
    /// ```ignore
    /// let coord = Arc::new_cyclic(|weak| {
    ///     let transport = Arc::new(StoqTransferTransport::new(network.clone(), weak.clone()));
    ///     TransferCoordinator::with_validator(blockchain, transport, federation, chain_id, validator)
    /// });
    /// ```
    pub fn new(network: Arc<NetworkManager>, coordinator: Weak<TransferCoordinator>) -> Self {
        Self { network, coordinator }
    }

    /// Helper: produce the tag-prefixed payload bytes used on the wire.
    fn tagged(tag: u8, body: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(body.len() + 1);
        buf.push(tag);
        buf.extend_from_slice(body);
        buf
    }

    /// Iterate every currently-active peer connection and write
    /// `payload` over a fresh unidirectional stream. Errors are
    /// logged but do not abort the broadcast — best-effort delivery
    /// is the goal (peers also gossip the lock/release entries from
    /// their local chains).
    async fn fanout(&self, payload: Vec<u8>, label: &'static str) -> Result<(), GatewayError> {
        let nodes = self.network.get_connected_nodes().await;
        if nodes.is_empty() {
            debug!("StoqTransferTransport::{label}: no connected peers");
            return Ok(());
        }
        let total = nodes.len();
        let mut sent = 0usize;
        for node in nodes {
            let conn = match node.connection {
                Some(c) if c.is_active() => c,
                _ => continue,
            };
            match conn.open_stream().await {
                Ok(mut stream) => {
                    if let Err(e) = stream.send(&payload).await {
                        warn!(
                            "StoqTransferTransport::{label}: failed to write to {}: {}",
                            &node.node_id[..8.min(node.node_id.len())],
                            e
                        );
                        continue;
                    }
                    sent += 1;
                }
                Err(e) => {
                    warn!(
                        "StoqTransferTransport::{label}: open_stream to {} failed: {}",
                        &node.node_id[..8.min(node.node_id.len())],
                        e
                    );
                }
            }
        }
        debug!(
            "StoqTransferTransport::{label}: broadcast to {sent}/{total} peers ({} bytes)",
            payload.len()
        );
        Ok(())
    }
}

#[async_trait]
impl TransferTransport for StoqTransferTransport {
    async fn broadcast_lock(&self, msg: TransferLockMessage) -> Result<(), GatewayError> {
        let body = serde_json::to_vec(&msg).map_err(|e| GatewayError::TransportFailure {
            transfer_id: msg.transfer_id.clone(),
            detail: format!("serialize TransferLockMessage: {e}"),
        })?;
        self.fanout(Self::tagged(TAG_TRANSFER_LOCK, &body), "broadcast_lock")
            .await
    }

    async fn send_register_request(
        &self,
        peer: &PeerCertFingerprint,
        req: TransferRegisterRequest,
        deadline: Duration,
    ) -> Result<TransferRegisterAck, GatewayError> {
        let coord = match self.coordinator.upgrade() {
            Some(c) => c,
            None => {
                return Err(GatewayError::TransportFailure {
                    transfer_id: req.transfer_id.clone(),
                    detail: "coordinator dropped before send_register_request".into(),
                });
            }
        };

        // 1. Register a oneshot waiter BEFORE writing to the wire so we
        //    cannot race with an ack arriving on another stream.
        let (tx, rx) = oneshot::channel();
        coord
            .register_ack_waiter(req.transfer_id.clone(), tx)
            .await;

        // 2. Locate the target peer's connection by node_id.
        let nodes = self.network.get_connected_nodes().await;
        let target_node = nodes
            .iter()
            .find(|n| &n.node_id == peer)
            .cloned()
            .ok_or_else(|| GatewayError::TransportFailure {
                transfer_id: req.transfer_id.clone(),
                detail: format!("peer {peer} not connected"),
            })?;
        let conn = match target_node.connection {
            Some(c) if c.is_active() => c,
            _ => {
                return Err(GatewayError::TransportFailure {
                    transfer_id: req.transfer_id.clone(),
                    detail: format!("peer {peer} has no active STOQ connection"),
                });
            }
        };

        // 3. Serialize and write.
        let body = serde_json::to_vec(&req).map_err(|e| GatewayError::TransportFailure {
            transfer_id: req.transfer_id.clone(),
            detail: format!("serialize TransferRegisterRequest: {e}"),
        })?;
        let payload = Self::tagged(TAG_TRANSFER_REGISTER_REQ, &body);

        let mut stream = conn.open_stream().await.map_err(|e| {
            GatewayError::TransportFailure {
                transfer_id: req.transfer_id.clone(),
                detail: format!("open_stream to {peer}: {e}"),
            }
        })?;
        stream
            .send(&payload)
            .await
            .map_err(|e| GatewayError::TransportFailure {
                transfer_id: req.transfer_id.clone(),
                detail: format!("send TAG_TRANSFER_REGISTER_REQ to {peer}: {e}"),
            })?;

        // 4. Await the ack via the coordinator-resolved oneshot, with
        //    deadline. Coordinator additionally guards with its own
        //    timeout so a misbehaving peer cannot stall us.
        match tokio::time::timeout(deadline, rx).await {
            Ok(Ok(ack)) => Ok(ack),
            Ok(Err(_canceled)) => Err(GatewayError::TransportFailure {
                transfer_id: req.transfer_id,
                detail: "ack waiter cancelled (coordinator dropped)".into(),
            }),
            Err(_elapsed) => Err(GatewayError::TransportFailure {
                transfer_id: req.transfer_id,
                detail: format!("register-ack timeout after {}ms", deadline.as_millis()),
            }),
        }
    }

    async fn broadcast_release(&self, msg: TransferRelease) -> Result<(), GatewayError> {
        let body = serde_json::to_vec(&msg).map_err(|e| GatewayError::TransportFailure {
            transfer_id: msg.transfer_id.clone(),
            detail: format!("serialize TransferRelease: {e}"),
        })?;
        self.fanout(
            Self::tagged(TAG_TRANSFER_RELEASE, &body),
            "broadcast_release",
        )
        .await
    }

    async fn broadcast_rollback(&self, msg: TransferRollback) -> Result<(), GatewayError> {
        let body = serde_json::to_vec(&msg).map_err(|e| GatewayError::TransportFailure {
            transfer_id: msg.transfer_id.clone(),
            detail: format!("serialize TransferRollback: {e}"),
        })?;
        self.fanout(
            Self::tagged(TAG_TRANSFER_ROLLBACK, &body),
            "broadcast_rollback",
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tagged_prepends_tag_byte() {
        let out = StoqTransferTransport::tagged(0x42, b"hello");
        assert_eq!(out[0], 0x42);
        assert_eq!(&out[1..], b"hello");
    }
}
