// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ-backed block transport adapter.
//!
//! Implements [`BlockTransport`] using real STOQ/QUIC connections for
//! sending blocks to remote nodes.  Connections are cached per
//! coordinate and reused when still active.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use super::block::Block;
use super::propagation::BlockTransport;
use crate::matrix::coordinate::MatrixCoordinate;

/// Tag byte identifying a BLOCK_ANNOUNCE message on the wire.
const BLOCK_ANNOUNCE_TAG: u8 = 0x03;

/// Adapter that implements [`BlockTransport`] over bilateral handshake connections.
///
/// Block propagation ONLY uses connections established via bilateral PoS
/// handshake (R11). Connections are injected after handshake completes;
/// `send_block` never creates new connections. If no handshake connection
/// exists for a target coordinate, propagation to that peer is skipped —
/// the peer must handshake first.
///
/// Streams opened on handshake connections are received by
/// `run_peer_message_loop` on the remote side, which already knows our
/// identity from the handshake. No discriminator or node-id prefix is
/// needed — just the raw `[tag][body]` payload.
pub struct StoqBlockTransportAdapter {
    /// Bilateral handshake connections keyed by coordinate string ("x,y,z").
    /// Populated via [`inject_connection`] after successful PoS handshake.
    connections: Arc<RwLock<HashMap<String, Arc<stoq::Connection>>>>,
}

impl StoqBlockTransportAdapter {
    /// Create a new adapter.
    ///
    /// Starts with no connections. Call [`inject_connection`] after each
    /// successful bilateral PoS handshake to enable block propagation to
    /// that peer.
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Inject a bilateral handshake connection for a coordinate.
    ///
    /// After a successful PoS handshake, the authenticated connection is
    /// registered here so that block propagation can reuse it. The remote
    /// side's `run_peer_message_loop` is already accepting streams on this
    /// connection and knows our identity from the handshake.
    pub async fn inject_connection(
        &self,
        coord: &MatrixCoordinate,
        connection: Arc<stoq::Connection>,
    ) {
        let key = format!("{},{},{}", coord.x, coord.y, coord.z);
        self.connections.write().await.insert(key, connection);
    }

    /// Build the wire-format payload for a block announcement.
    ///
    /// Layout:
    /// - `[0]`    tag byte `0x03` (BLOCK_ANNOUNCE)
    /// - `[1..9]` block_json_len: u64 LE
    /// - `[9..9+N]` block_json bytes
    /// - `[9+N..17+N]` proof_hash_len: u64 LE (32 if present, 0 if None)
    /// - `[17+N..17+N+P]` proof_hash bytes (32 bytes if present, empty if None)
    fn build_wire_payload(block: &Block) -> Result<Vec<u8>, serde_json::Error> {
        let block_json = serde_json::to_vec(block)?;
        let block_json_len = block_json.len() as u64;

        let first_proof_hash: Option<[u8; 32]> =
            block.entries.first().map(|e| e.proof_hash);
        let (proof_hash_len, proof_hash_bytes): (u64, Vec<u8>) = match first_proof_hash {
            Some(hash) => (32u64, hash.to_vec()),
            None => (0u64, Vec::new()),
        };

        let total = 1 + 8 + block_json.len() + 8 + proof_hash_bytes.len();
        let mut buf = Vec::with_capacity(total);

        buf.push(BLOCK_ANNOUNCE_TAG);
        buf.extend_from_slice(&block_json_len.to_le_bytes());
        buf.extend_from_slice(&block_json);
        buf.extend_from_slice(&proof_hash_len.to_le_bytes());
        buf.extend_from_slice(&proof_hash_bytes);

        Ok(buf)
    }
}

#[async_trait::async_trait]
impl BlockTransport for StoqBlockTransportAdapter {
    async fn send_block(
        &self,
        block: &Block,
        target: &MatrixCoordinate,
        _origin: &MatrixCoordinate,
    ) -> bool {
        let key = format!("{},{},{}", target.x, target.y, target.z);

        // Look up the handshake connection for this coordinate.
        // If no connection exists, the peer hasn't completed bilateral PoS
        // handshake — skip propagation to this peer.
        let conn = {
            let cache = self.connections.read().await;
            match cache.get(&key) {
                Some(c) if c.is_active() => c.clone(),
                Some(_) => {
                    debug!(coord = %key, "handshake connection inactive, skipping");
                    return false;
                }
                None => {
                    debug!(coord = %key, "no handshake connection for coordinate, skipping");
                    return false;
                }
            }
        };

        // Build the wire payload: [tag][body].
        let payload = match Self::build_wire_payload(block) {
            Ok(p) => p,
            Err(e) => {
                warn!(block_index = block.index, error = %e, "block serialization failed");
                return false;
            }
        };

        // Open a Stream on the handshake connection and send the payload.
        // Uses `open_stream()` + `stream.send()` so the framing matches
        // what the remote side's `run_peer_message_loop` expects from
        // `stream.receive()`.
        match conn.open_stream().await {
            Ok(mut stream) => {
                if let Err(e) = stream.send(&payload).await {
                    warn!(coord = %key, error = %e, "stream send failed");
                    return false;
                }
                debug!(
                    coord = %key,
                    block_index = block.index,
                    payload_bytes = payload.len(),
                    "block announced via STOQ handshake connection"
                );
                true
            }
            Err(e) => {
                warn!(coord = %key, error = %e, "failed to open STOQ stream");
                self.connections.write().await.remove(&key);
                false
            }
        }
    }
}
