// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ-backed block transport adapter.
//!
//! Implements [`BlockTransport`] using real STOQ/QUIC connections for
//! sending blocks to remote nodes.  Connections are cached per
//! coordinate and reused when still active.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use super::block::Block;
use super::propagation::BlockTransport;
use crate::matrix::coordinate::MatrixCoordinate;

/// Tag byte identifying a BLOCK_ANNOUNCE message on the wire.
const BLOCK_ANNOUNCE_TAG: u8 = 0x03;

/// Adapter that implements [`BlockTransport`] using real STOQ connections.
///
/// Connects to remote nodes via STOQ/QUIC and sends serialized blocks over
/// bidirectional streams. Connections are cached per coordinate and reused
/// when still active.
pub struct StoqBlockTransportAdapter {
    /// STOQ transport for establishing connections.
    transport: Arc<stoq::StoqTransport>,
    /// Cached connections keyed by coordinate string ("x,y,z").
    connections: Arc<RwLock<HashMap<String, Arc<stoq::Connection>>>>,
    /// Coordinate -> (node_id_hex, socket_addr) mapping for address resolution.
    node_map: Arc<RwLock<HashMap<String, (String, SocketAddr)>>>,
}

impl StoqBlockTransportAdapter {
    /// Create a new adapter backed by the given STOQ transport.
    ///
    /// `node_map` maps coordinate keys ("x,y,z") to `(node_id_hex, SocketAddr)`.
    pub fn new(
        transport: Arc<stoq::StoqTransport>,
        node_map: Arc<RwLock<HashMap<String, (String, SocketAddr)>>>,
    ) -> Self {
        Self {
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
            node_map,
        }
    }

    /// Register a node's address for a coordinate.
    pub async fn register_node(
        &self,
        coord: &MatrixCoordinate,
        node_id: &str,
        addr: SocketAddr,
    ) {
        let key = format!("{},{},{}", coord.x, coord.y, coord.z);
        self.node_map
            .write()
            .await
            .insert(key, (node_id.to_string(), addr));
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

        // 1. Resolve target coordinate to a socket address.
        let addr = {
            let map = self.node_map.read().await;
            match map.get(&key) {
                Some((_node_id, addr)) => *addr,
                None => {
                    warn!(coord = %key, "no node registered for coordinate");
                    return false;
                }
            }
        };

        // STOQ is IPv6-only — reject IPv4 addresses.
        let ipv6_addr = match addr {
            SocketAddr::V6(v6) => *v6.ip(),
            SocketAddr::V4(_) => {
                warn!(addr = %addr, "IPv4 address rejected — STOQ is IPv6-only");
                return false;
            }
        };

        // 2. Try to reuse a cached connection, or establish a new one.
        let conn = {
            let cache = self.connections.read().await;
            cache.get(&key).filter(|c| c.is_active()).cloned()
        };

        let conn = match conn {
            Some(c) => c,
            None => {
                let endpoint = stoq::Endpoint::new(ipv6_addr, addr.port());
                match self.transport.connect(&endpoint).await {
                    Ok(new_conn) => {
                        self.connections
                            .write()
                            .await
                            .insert(key.clone(), new_conn.clone());
                        new_conn
                    }
                    Err(e) => {
                        warn!(coord = %key, error = %e, "STOQ connect failed");
                        return false;
                    }
                }
            }
        };

        // 3. Build the wire payload.
        let payload = match Self::build_wire_payload(block) {
            Ok(p) => p,
            Err(e) => {
                warn!(block_index = block.index, error = %e, "block serialization failed");
                return false;
            }
        };

        // 4. Open a stream, write the PEER_MESSAGE discriminator, then payload.
        match conn.open_bi().await {
            Ok((mut send, _recv)) => {
                // Write the connection-type discriminator so the acceptor
                // knows this is a peer message, not a handshake.
                if let Err(e) = send.write_all(&[crate::network::CONN_TYPE_PEER_MESSAGE]).await {
                    warn!(coord = %key, error = %e, "discriminator write failed");
                    return false;
                }
                if let Err(e) = send.write_all(&payload).await {
                    warn!(coord = %key, error = %e, "stream write failed");
                    return false;
                }
                if let Err(e) = send.finish() {
                    warn!(coord = %key, error = %e, "stream finish failed");
                    return false;
                }
                debug!(
                    coord = %key,
                    block_index = block.index,
                    payload_bytes = payload.len(),
                    "block announced via STOQ"
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
