// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ-based block transport for reflector pool communication.
//!
//! Uses an outbox/inbox pattern that decouples the sync protocol from
//! real STOQ network I/O. A consumer reads from the outbox, sends over
//! a STOQ stream, receives responses, and writes to the inbox. This
//! makes the transport fully testable without network connectivity and
//! allows seamless integration with real STOQ streams later.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use tracing::{debug, info, warn};

use hypermesh_lib::{BlockchainScope, MatrixPosition, NetworkId, PrivacyMode};

use super::message::ReflectorMessage;

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

/// Aggregate transport metrics for monitoring and diagnostics.
#[derive(Debug)]
pub struct ReflectorTransportMetrics {
    /// Total messages queued for sending.
    pub messages_sent: AtomicU64,
    /// Total messages received via the inbox.
    pub messages_received: AtomicU64,
    /// Total serialized bytes queued for sending.
    pub bytes_sent: AtomicU64,
    /// Total raw bytes received via the inbox.
    pub bytes_received: AtomicU64,
    /// Number of `connect_reflector` calls.
    pub connect_attempts: AtomicU64,
    /// Number of failed connect attempts.
    pub connect_failures: AtomicU64,
}

impl ReflectorTransportMetrics {
    fn new() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            connect_attempts: AtomicU64::new(0),
            connect_failures: AtomicU64::new(0),
        }
    }
}

// ---------------------------------------------------------------------------
// ReflectorNode
// ---------------------------------------------------------------------------

/// Represents a single reflector peer that this transport is connected to.
#[derive(Debug)]
pub struct ReflectorNode {
    /// Unique identifier of the reflector.
    pub node_id: String,
    /// Remote endpoint address string (e.g. "[::1]:9292").
    pub endpoint_addr: String,
    /// Block-MATRIX position of the reflector.
    pub position: MatrixPosition,
    /// Current block height reported by the reflector.
    pub block_height: AtomicU64,
    /// Health score (0.0-1.0) reported by the reflector.
    pub health_score: RwLock<f64>,
    /// When the local node connected to this reflector.
    pub connected_at: Instant,
    /// Timestamp of the last heartbeat received from this reflector.
    pub last_heartbeat: RwLock<Instant>,
}

// ---------------------------------------------------------------------------
// StoqBlockTransport
// ---------------------------------------------------------------------------

/// STOQ-based block transport for reflector pool synchronization.
///
/// Manages connections to reflector peers and provides an outbox/inbox
/// message queue for sending and receiving [`ReflectorMessage`]s. The
/// actual STOQ stream I/O is performed by an external consumer that
/// drains the outbox and populates the inbox, keeping this struct
/// testable without real network connectivity.
pub struct StoqBlockTransport {
    /// Network this transport operates on.
    network_id: NetworkId,
    /// Privacy mode governing transport behaviour.
    privacy_mode: PrivacyMode,
    /// Blockchain scope -- always `Network` for reflector sync.
    scope: BlockchainScope,
    /// Local node identifier.
    local_node_id: String,
    /// Local node's Block-MATRIX position.
    local_position: MatrixPosition,
    /// Connected reflector peers keyed by `node_id`.
    reflector_nodes: Arc<DashMap<String, Arc<ReflectorNode>>>,
    /// Outbound message queue: `(target_node_id, serialized_bytes)`.
    outbox: Arc<RwLock<Vec<(String, Vec<u8>)>>>,
    /// Inbound message queue: `(raw_bytes, from_node_id)`.
    inbox: Arc<RwLock<Vec<(Vec<u8>, String)>>>,
    /// Transport-level metrics.
    metrics: Arc<ReflectorTransportMetrics>,
}

impl StoqBlockTransport {
    /// Create a new block transport for the given network.
    ///
    /// The `scope` is always set to [`BlockchainScope::Network`] because
    /// reflector pools only exist for Network-scope synchronization.
    pub fn new(
        network_id: NetworkId,
        privacy_mode: PrivacyMode,
        local_node_id: String,
        local_position: MatrixPosition,
    ) -> Self {
        info!(
            node = %local_node_id,
            network = %network_id,
            "Creating StoqBlockTransport for reflector pool"
        );
        Self {
            network_id,
            privacy_mode,
            scope: BlockchainScope::Network,
            local_node_id,
            local_position,
            reflector_nodes: Arc::new(DashMap::new()),
            outbox: Arc::new(RwLock::new(Vec::new())),
            inbox: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(ReflectorTransportMetrics::new()),
        }
    }

    // -- Connection management -----------------------------------------------

    /// Register a reflector peer.
    ///
    /// If a peer with the same `node_id` already exists it will be
    /// replaced (reconnect semantics).
    pub fn connect_reflector(
        &self,
        node_id: String,
        endpoint_addr: String,
        position: MatrixPosition,
    ) -> Result<()> {
        self.metrics
            .connect_attempts
            .fetch_add(1, Ordering::Relaxed);

        let now = Instant::now();
        let node = Arc::new(ReflectorNode {
            node_id: node_id.clone(),
            endpoint_addr,
            position,
            block_height: AtomicU64::new(0),
            health_score: RwLock::new(1.0),
            connected_at: now,
            last_heartbeat: RwLock::new(now),
        });

        self.reflector_nodes.insert(node_id.clone(), node);
        debug!(node = %node_id, "Connected reflector");
        Ok(())
    }

    /// Remove a reflector peer. Returns `true` if the peer existed.
    pub fn disconnect_reflector(&self, node_id: &str) -> bool {
        let removed = self.reflector_nodes.remove(node_id).is_some();
        if removed {
            debug!(node = %node_id, "Disconnected reflector");
        }
        removed
    }

    // -- Messaging -----------------------------------------------------------

    /// Serialize a message and push it to the outbox for a single target.
    pub fn send_message(&self, message: &ReflectorMessage, target_node_id: &str) -> Result<()> {
        if !self.reflector_nodes.contains_key(target_node_id) {
            warn!(target = %target_node_id, "send_message to unknown reflector");
        }
        let bytes = message
            .serialize_message()
            .context("serializing outbound message")?;
        let byte_len = bytes.len() as u64;

        self.outbox
            .write()
            .push((target_node_id.to_string(), bytes));
        self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.metrics
            .bytes_sent
            .fetch_add(byte_len, Ordering::Relaxed);
        Ok(())
    }

    /// Broadcast a message to **all** connected reflectors.
    ///
    /// Returns the number of reflectors the message was queued for.
    pub fn broadcast_message(&self, message: &ReflectorMessage) -> Result<usize> {
        let bytes = message
            .serialize_message()
            .context("serializing broadcast message")?;
        let byte_len = bytes.len() as u64;

        let node_ids: Vec<String> = self
            .reflector_nodes
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        let count = node_ids.len();
        let mut outbox = self.outbox.write();
        for nid in node_ids {
            outbox.push((nid, bytes.clone()));
        }

        self.metrics
            .messages_sent
            .fetch_add(count as u64, Ordering::Relaxed);
        self.metrics
            .bytes_sent
            .fetch_add(byte_len * count as u64, Ordering::Relaxed);
        Ok(count)
    }

    /// Drain the inbox and return deserialized messages with their source
    /// node identifiers.
    ///
    /// Messages that fail to deserialize are logged and dropped.
    pub fn receive_messages(&self) -> Vec<(ReflectorMessage, String)> {
        let raw: Vec<(Vec<u8>, String)> = {
            let mut inbox = self.inbox.write();
            std::mem::take(&mut *inbox)
        };

        let mut result = Vec::with_capacity(raw.len());
        for (data, from) in raw {
            match ReflectorMessage::deserialize_message(&data) {
                Ok(msg) => {
                    self.metrics
                        .messages_received
                        .fetch_add(1, Ordering::Relaxed);
                    result.push((msg, from));
                }
                Err(e) => {
                    warn!(from = %from, err = %e, "dropping malformed inbox message");
                }
            }
        }
        result
    }

    /// Inject raw bytes into the inbox as if they arrived from a remote
    /// peer. Used for testing and by the real STOQ stream consumer.
    pub fn inject_message(&self, data: Vec<u8>, from_node_id: String) {
        let byte_len = data.len() as u64;
        self.inbox.write().push((data, from_node_id));
        self.metrics
            .bytes_received
            .fetch_add(byte_len, Ordering::Relaxed);
    }

    /// Drain and return the raw outbox contents.
    ///
    /// A STOQ stream consumer calls this to obtain messages that need
    /// to be sent over the wire.
    pub fn drain_outbox(&self) -> Vec<(String, Vec<u8>)> {
        let mut outbox = self.outbox.write();
        std::mem::take(&mut *outbox)
    }

    // -- Reflector queries ---------------------------------------------------

    /// List identifiers of all connected reflectors.
    pub fn connected_reflectors(&self) -> Vec<String> {
        self.reflector_nodes
            .iter()
            .map(|e| e.key().clone())
            .collect()
    }

    /// Number of connected reflectors.
    pub fn reflector_count(&self) -> usize {
        self.reflector_nodes.len()
    }

    /// Get a reference-counted handle to a specific reflector.
    pub fn get_reflector(&self, node_id: &str) -> Option<Arc<ReflectorNode>> {
        self.reflector_nodes.get(node_id).map(|e| e.value().clone())
    }

    /// Update a reflector's health score and block height.
    pub fn update_reflector_health(&self, node_id: &str, health: f64, block_height: u64) {
        if let Some(entry) = self.reflector_nodes.get(node_id) {
            let node = entry.value();
            *node.health_score.write() = health.clamp(0.0, 1.0);
            node.block_height.store(block_height, Ordering::Relaxed);
            *node.last_heartbeat.write() = Instant::now();
            debug!(
                node = %node_id,
                health = health,
                height = block_height,
                "Updated reflector health"
            );
        }
    }

    /// Return the node_id of the reflector with the highest health score.
    ///
    /// Ties are broken arbitrarily (first encountered wins).
    pub fn best_reflector(&self) -> Option<String> {
        let mut best_id: Option<String> = None;
        let mut best_score: f64 = -1.0;

        for entry in self.reflector_nodes.iter() {
            let score = *entry.value().health_score.read();
            if score > best_score {
                best_score = score;
                best_id = Some(entry.key().clone());
            }
        }
        best_id
    }

    // -- Accessors -----------------------------------------------------------

    /// Transport-level metrics reference.
    pub fn metrics(&self) -> &ReflectorTransportMetrics {
        &self.metrics
    }

    /// Network this transport operates on.
    pub fn network_id(&self) -> &NetworkId {
        &self.network_id
    }

    /// Privacy mode governing this transport.
    pub fn privacy_mode(&self) -> &PrivacyMode {
        &self.privacy_mode
    }

    /// Blockchain scope (always `Network`).
    pub fn scope(&self) -> BlockchainScope {
        self.scope
    }

    /// Local node identifier.
    pub fn local_node_id(&self) -> &str {
        &self.local_node_id
    }

    /// Local node's Block-MATRIX position.
    pub fn local_position(&self) -> &MatrixPosition {
        &self.local_position
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_network() -> NetworkId {
        NetworkId([0x01; 16])
    }

    fn test_position(x: f64) -> MatrixPosition {
        MatrixPosition { x, y: 0.0, z: 0.0 }
    }

    fn make_transport() -> StoqBlockTransport {
        StoqBlockTransport::new(
            test_network(),
            PrivacyMode::PUBLIC,
            "local-node".to_string(),
            test_position(0.0),
        )
    }

    #[test]
    fn test_connect_disconnect_reflectors() {
        let t = make_transport();

        t.connect_reflector("r1".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect r1");
        t.connect_reflector("r2".into(), "[::1]:9002".into(), test_position(2.0))
            .expect("test: connect r2");
        t.connect_reflector("r3".into(), "[::1]:9003".into(), test_position(3.0))
            .expect("test: connect r3");

        assert_eq!(t.reflector_count(), 3);

        assert!(t.disconnect_reflector("r2"));
        assert_eq!(t.reflector_count(), 2);

        // r2 is gone, r1 and r3 remain
        assert!(t.get_reflector("r1").is_some());
        assert!(t.get_reflector("r2").is_none());
        assert!(t.get_reflector("r3").is_some());

        // metrics
        assert_eq!(t.metrics().connect_attempts.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_send_receive_messages() {
        let t = make_transport();
        t.connect_reflector("peer".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect");

        let msg = ReflectorMessage::Heartbeat {
            node_id: "peer".to_string(),
            network_id: test_network(),
            block_height: 10,
            health_score: 0.9,
            position: test_position(1.0),
            timestamp: 100,
        };

        t.send_message(&msg, "peer").expect("test: send");

        // Simulate the STOQ consumer: drain outbox, deliver to inbox
        let outbox = t.drain_outbox();
        assert_eq!(outbox.len(), 1);
        let (target, data) = &outbox[0];
        assert_eq!(target, "peer");

        t.inject_message(data.clone(), "peer".to_string());

        let received = t.receive_messages();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].0, msg);
        assert_eq!(received[0].1, "peer");
    }

    #[test]
    fn test_broadcast_message() {
        let t = make_transport();
        t.connect_reflector("r1".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect r1");
        t.connect_reflector("r2".into(), "[::1]:9002".into(), test_position(2.0))
            .expect("test: connect r2");
        t.connect_reflector("r3".into(), "[::1]:9003".into(), test_position(3.0))
            .expect("test: connect r3");

        let msg = ReflectorMessage::BlockAnnounce {
            network_id: test_network(),
            block_height: 50,
            block_hash: [0xAA; 32],
            announcing_node: "local-node".to_string(),
        };

        let count = t.broadcast_message(&msg).expect("test: broadcast");
        assert_eq!(count, 3);

        let outbox = t.drain_outbox();
        assert_eq!(outbox.len(), 3);
    }

    #[test]
    fn test_best_reflector_selection() {
        let t = make_transport();
        t.connect_reflector("r1".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect r1");
        t.connect_reflector("r2".into(), "[::1]:9002".into(), test_position(2.0))
            .expect("test: connect r2");
        t.connect_reflector("r3".into(), "[::1]:9003".into(), test_position(3.0))
            .expect("test: connect r3");

        // Set different health scores
        t.update_reflector_health("r1", 0.5, 10);
        t.update_reflector_health("r2", 0.9, 20);
        t.update_reflector_health("r3", 0.7, 15);

        assert_eq!(t.best_reflector(), Some("r2".to_string()));

        // Verify block height was also updated
        let r2 = t.get_reflector("r2").expect("test: r2 should exist");
        assert_eq!(r2.block_height.load(Ordering::Relaxed), 20);
    }

    #[test]
    fn test_scope_always_network() {
        let t = make_transport();
        assert_eq!(t.scope(), BlockchainScope::Network);
    }

    #[test]
    fn test_metrics_accumulate() {
        let t = make_transport();
        t.connect_reflector("r1".into(), "[::1]:9001".into(), test_position(1.0))
            .expect("test: connect");

        let msg = ReflectorMessage::QuorumConfirm {
            network_id: test_network(),
            block_height: 5,
            confirming_node: "r1".to_string(),
        };

        t.send_message(&msg, "r1").expect("test: send 1");
        t.send_message(&msg, "r1").expect("test: send 2");

        assert_eq!(t.metrics().messages_sent.load(Ordering::Relaxed), 2);
        assert!(t.metrics().bytes_sent.load(Ordering::Relaxed) > 0);
    }
}
