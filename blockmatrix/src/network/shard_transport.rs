// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Shard Transport Layer
//!
//! Abstracts network I/O for sending and receiving shards between matrix nodes.
//! The `ShardTransport` trait enables testing with `MockShardTransport` while
//! `StoqShardTransport` provides the real STOQ-backed implementation.

use async_trait::async_trait;
use hypermesh_lib::{ContentHash, NodeId};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::shard_store::ShardStore;
use crate::transport::error::TransportError;

/// Abstraction for shard-level network operations.
///
/// Implementations handle the actual byte transfer of shards between nodes.
/// The trait is object-safe and designed for both real STOQ connections and
/// in-memory testing mocks.
#[async_trait]
pub trait ShardTransport: Send + Sync {
    /// Send a shard to a target node.
    ///
    /// The implementation must ensure the shard data reaches the target and
    /// is stored under the given `shard_id`.
    async fn send_shard(
        &self,
        target: &NodeId,
        shard_id: &ContentHash,
        data: &[u8],
    ) -> Result<(), TransportError>;

    /// Fetch a shard from a source node.
    ///
    /// Returns the raw shard bytes. The caller is responsible for verifying
    /// the content hash matches expectations.
    async fn fetch_shard(
        &self,
        source: &NodeId,
        shard_id: &ContentHash,
    ) -> Result<Vec<u8>, TransportError>;

    /// Check if a node is reachable.
    ///
    /// A lightweight probe -- implementations should avoid heavy handshakes.
    async fn is_reachable(&self, node: &NodeId) -> bool;
}

/// STOQ-backed shard transport.
///
/// Routes shard send/fetch operations through the STOQ protocol layer.
/// Maintains a connection pool keyed by `NodeId` with auto-dial support:
/// if a peer has a registered address (via `register_node_address`) but no
/// cached connection, `send_shard` and `fetch_shard` will automatically
/// establish a STOQ connection on demand.
pub struct StoqShardTransport {
    /// STOQ transport instance for connection management
    transport: Arc<stoq::StoqTransport>,
    /// Cached connections keyed by node ID hex
    connections: Arc<RwLock<HashMap<String, Arc<stoq::Connection>>>>,
    /// Known node addresses for auto-dialing
    node_addresses: Arc<RwLock<HashMap<String, SocketAddr>>>,
}

impl StoqShardTransport {
    /// Create a new STOQ shard transport.
    pub fn new(transport: Arc<stoq::StoqTransport>) -> Self {
        Self {
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
            node_addresses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an existing connection for a node.
    pub async fn register_connection(&self, node_id: &NodeId, connection: Arc<stoq::Connection>) {
        self.connections
            .write()
            .await
            .insert(node_id.to_hex(), connection);
    }

    /// Register a node's network address for auto-dialing.
    pub async fn register_node_address(&self, node_id: &NodeId, addr: SocketAddr) {
        self.node_addresses
            .write()
            .await
            .insert(node_id.to_hex(), addr);
    }

    /// Get an existing connection or auto-dial the node if we know its address.
    async fn get_or_connect(
        &self,
        node_id: &NodeId,
    ) -> Result<Arc<stoq::Connection>, TransportError> {
        let hex = node_id.to_hex();

        // Try cached connection first
        {
            let conns = self.connections.read().await;
            if let Some(conn) = conns.get(&hex) {
                if conn.is_active() {
                    return Ok(conn.clone());
                }
            }
        }

        // Try auto-dial if we know the address
        let addr = {
            let addrs = self.node_addresses.read().await;
            addrs.get(&hex).copied()
        };

        let addr = addr.ok_or_else(|| {
            TransportError::NoConnection(format!("no address registered for node {hex}"))
        })?;

        // Connect via STOQ (IPv6 only)
        let endpoint = stoq::Endpoint::new(
            match addr {
                SocketAddr::V6(v6) => *v6.ip(),
                _ => return Err(TransportError::Network("only IPv6 supported".into())),
            },
            addr.port(),
        );

        let connection = self
            .transport
            .connect(&endpoint)
            .await
            .map_err(|e| TransportError::Network(format!("auto-dial to {addr} failed: {e}")))?;

        // Cache the connection
        self.connections
            .write()
            .await
            .insert(hex, connection.clone());

        Ok(connection)
    }
}

#[async_trait]
impl ShardTransport for StoqShardTransport {
    async fn send_shard(
        &self,
        target: &NodeId,
        shard_id: &ContentHash,
        data: &[u8],
    ) -> Result<(), TransportError> {
        let connection = self.get_or_connect(target).await?;

        // Open a unidirectional stream and send: [32-byte shard_id][shard data]
        let mut stream = connection
            .open_stream()
            .await
            .map_err(|e| TransportError::Network(format!("failed to open stream: {e}")))?;

        // Build message: tag(1) + shard_id(32) + data_len(8) + data
        let mut message = Vec::with_capacity(1 + 32 + 8 + data.len());
        message.push(0x01); // SHARD_SEND tag
        message.extend_from_slice(&shard_id.0);
        message.extend_from_slice(&(data.len() as u64).to_le_bytes());
        message.extend_from_slice(data);

        stream
            .send(&message)
            .await
            .map_err(|e| TransportError::Network(format!("failed to send shard: {e}")))?;

        Ok(())
    }

    async fn fetch_shard(
        &self,
        source: &NodeId,
        shard_id: &ContentHash,
    ) -> Result<Vec<u8>, TransportError> {
        let connection = self.get_or_connect(source).await?;

        // Open stream and request shard
        let mut stream = connection
            .open_stream()
            .await
            .map_err(|e| TransportError::Network(format!("failed to open stream: {e}")))?;

        // Send request: tag(1) + shard_id(32)
        let mut request = Vec::with_capacity(33);
        request.push(0x02); // SHARD_FETCH tag
        request.extend_from_slice(&shard_id.0);

        stream
            .send(&request)
            .await
            .map_err(|e| TransportError::Network(format!("failed to send fetch request: {e}")))?;

        // Receive response
        let response = stream
            .receive()
            .await
            .map_err(|e| TransportError::Network(format!("failed to receive shard: {e}")))?;

        Ok(response.to_vec())
    }

    async fn is_reachable(&self, node: &NodeId) -> bool {
        let hex = node.to_hex();
        // Check connection cache first
        if self.connections.read().await.contains_key(&hex) {
            return true;
        }
        // Check if we know the address (we could potentially connect)
        self.node_addresses.read().await.contains_key(&hex)
    }
}

/// In-memory mock shard transport for testing.
///
/// Stores shards in a `HashMap` keyed by `(node_id_hex, shard_id_hex)`.
/// All operations succeed unless a node is in the `unreachable` set.
pub struct MockShardTransport {
    /// Stored shards: (node_hex, shard_hex) -> data
    shards: Arc<RwLock<HashMap<(String, String), Vec<u8>>>>,
    /// Set of unreachable node IDs (hex)
    unreachable: Arc<RwLock<Vec<String>>>,
}

impl MockShardTransport {
    /// Create a new empty mock transport.
    pub fn new() -> Self {
        Self {
            shards: Arc::new(RwLock::new(HashMap::new())),
            unreachable: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Mark a node as unreachable.
    pub async fn set_unreachable(&self, node_id: &NodeId) {
        self.unreachable.write().await.push(node_id.to_hex());
    }

    /// Pre-populate a shard (simulating a remote node having data).
    pub async fn insert_shard(&self, node_id: &NodeId, shard_id: &ContentHash, data: Vec<u8>) {
        let key = (node_id.to_hex(), hex::encode(shard_id.0));
        self.shards.write().await.insert(key, data);
    }

    /// Get the number of stored shards.
    pub async fn shard_count(&self) -> usize {
        self.shards.read().await.len()
    }
}

impl Default for MockShardTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ShardTransport for MockShardTransport {
    async fn send_shard(
        &self,
        target: &NodeId,
        shard_id: &ContentHash,
        data: &[u8],
    ) -> Result<(), TransportError> {
        let unreachable = self.unreachable.read().await;
        if unreachable.contains(&target.to_hex()) {
            return Err(TransportError::NoConnection(format!(
                "node {} is unreachable",
                target.to_hex()
            )));
        }

        let key = (target.to_hex(), hex::encode(shard_id.0));
        self.shards.write().await.insert(key, data.to_vec());
        Ok(())
    }

    async fn fetch_shard(
        &self,
        source: &NodeId,
        shard_id: &ContentHash,
    ) -> Result<Vec<u8>, TransportError> {
        let unreachable = self.unreachable.read().await;
        if unreachable.contains(&source.to_hex()) {
            return Err(TransportError::NoConnection(format!(
                "node {} is unreachable",
                source.to_hex()
            )));
        }

        let key = (source.to_hex(), hex::encode(shard_id.0));
        let shards = self.shards.read().await;
        shards.get(&key).cloned().ok_or_else(|| {
            TransportError::Network(format!(
                "shard {} not found on node {}",
                hex::encode(shard_id.0),
                source.to_hex()
            ))
        })
    }

    async fn is_reachable(&self, node: &NodeId) -> bool {
        let unreachable = self.unreachable.read().await;
        !unreachable.contains(&node.to_hex())
    }
}

/// Handle an incoming shard stream from a peer.
///
/// Reads the tag byte, then dispatches:
/// - 0x01 (SHARD_SEND): peer is pushing a shard to us -- read shard_id(32) + data_len(8) + data, store in ShardStore
/// - 0x02 (SHARD_FETCH): peer is requesting a shard from us -- read shard_id(32), look up in ShardStore, send data back
pub async fn handle_incoming_shard_stream(
    stream: &mut stoq::Stream,
    store: &ShardStore,
) -> Result<(), TransportError> {
    // Read the full message from the stream
    let data = stream
        .receive()
        .await
        .map_err(|e| TransportError::Network(format!("failed to read stream: {e}")))?;

    if data.is_empty() {
        return Err(TransportError::Protocol("empty shard stream".into()));
    }

    let tag = data[0];

    match tag {
        0x01 => {
            // SHARD_SEND: tag(1) + shard_id(32) + data_len(8) + data
            if data.len() < 41 {
                return Err(TransportError::Protocol("SHARD_SEND too short".into()));
            }
            let mut shard_id_bytes = [0u8; 32];
            shard_id_bytes.copy_from_slice(&data[1..33]);
            let shard_id = ContentHash(shard_id_bytes);

            let data_len =
                u64::from_le_bytes(data[33..41].try_into().map_err(|_| {
                    TransportError::Protocol("SHARD_SEND invalid data_len".into())
                })?) as usize;
            if data.len() < 41 + data_len {
                return Err(TransportError::Protocol("SHARD_SEND data truncated".into()));
            }
            let shard_data = data[41..41 + data_len].to_vec();

            store.store(shard_id, shard_data).await;
            tracing::debug!("Stored shard {} from peer", hex::encode(shard_id_bytes));
            Ok(())
        }
        0x02 => {
            // SHARD_FETCH: tag(1) + shard_id(32)
            if data.len() < 33 {
                return Err(TransportError::Protocol("SHARD_FETCH too short".into()));
            }
            let mut shard_id_bytes = [0u8; 32];
            shard_id_bytes.copy_from_slice(&data[1..33]);
            let shard_id = ContentHash(shard_id_bytes);

            match store.get(&shard_id).await {
                Some(shard_data) => {
                    stream.send(&shard_data).await.map_err(|e| {
                        TransportError::Network(format!("failed to send shard: {e}"))
                    })?;
                    tracing::debug!("Served shard {} to peer", hex::encode(shard_id_bytes));
                    Ok(())
                }
                None => {
                    // Send empty response to indicate shard not found
                    stream.send(&[]).await.map_err(|e| {
                        TransportError::Network(format!("failed to send not-found: {e}"))
                    })?;
                    Ok(())
                }
            }
        }
        _ => Err(TransportError::Protocol(format!(
            "unknown shard tag: 0x{tag:02x}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node_id(seed: u8) -> NodeId {
        NodeId::from_bytes([seed; 32])
    }

    fn test_content_hash(seed: u8) -> ContentHash {
        ContentHash([seed; 32])
    }

    #[tokio::test]
    async fn test_mock_send_and_fetch() {
        let transport = MockShardTransport::new();
        let node = test_node_id(1);
        let shard_id = test_content_hash(42);
        let data = vec![0xAB; 1024];

        // Send shard
        transport
            .send_shard(&node, &shard_id, &data)
            .await
            .expect("test: send should succeed");

        assert_eq!(transport.shard_count().await, 1);

        // Fetch shard
        let fetched = transport
            .fetch_shard(&node, &shard_id)
            .await
            .expect("test: fetch should succeed");

        assert_eq!(fetched, data);
    }

    #[tokio::test]
    async fn test_mock_unreachable_node() {
        let transport = MockShardTransport::new();
        let node = test_node_id(2);
        let shard_id = test_content_hash(43);

        transport.set_unreachable(&node).await;

        // Send should fail
        let result = transport.send_shard(&node, &shard_id, &[1, 2, 3]).await;
        assert!(result.is_err());

        // Fetch should fail
        let result = transport.fetch_shard(&node, &shard_id).await;
        assert!(result.is_err());

        // Reachability check
        assert!(!transport.is_reachable(&node).await);
    }

    #[tokio::test]
    async fn test_mock_fetch_nonexistent_shard() {
        let transport = MockShardTransport::new();
        let node = test_node_id(3);
        let shard_id = test_content_hash(44);

        // Node is reachable but shard doesn't exist
        assert!(transport.is_reachable(&node).await);

        let result = transport.fetch_shard(&node, &shard_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_pre_populate() {
        let transport = MockShardTransport::new();
        let node = test_node_id(4);
        let shard_id = test_content_hash(45);
        let data = vec![0xFF; 512];

        transport.insert_shard(&node, &shard_id, data.clone()).await;

        let fetched = transport
            .fetch_shard(&node, &shard_id)
            .await
            .expect("test: fetch pre-populated shard");

        assert_eq!(fetched, data);
    }
}
