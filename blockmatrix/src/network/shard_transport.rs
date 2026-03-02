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
use std::sync::Arc;
use tokio::sync::RwLock;

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
    /// A lightweight probe — implementations should avoid heavy handshakes.
    async fn is_reachable(&self, node: &NodeId) -> bool;
}

/// STOQ-backed shard transport.
///
/// Routes shard send/fetch operations through the STOQ protocol layer.
/// Maintains a connection pool keyed by `NodeId`. When a node is not yet
/// connected, `send_shard` and `fetch_shard` return a connection error —
/// the caller must ensure the node is connected via the `NetworkManager`
/// before issuing shard operations.
pub struct StoqShardTransport {
    /// STOQ transport instance for connection management
    transport: Arc<stoq::StoqTransport>,
    /// Cached connections keyed by node ID hex
    connections: Arc<RwLock<HashMap<String, Arc<stoq::Connection>>>>,
}

impl StoqShardTransport {
    /// Create a new STOQ shard transport.
    pub fn new(transport: Arc<stoq::StoqTransport>) -> Self {
        Self {
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an existing connection for a node.
    pub async fn register_connection(&self, node_id: &NodeId, connection: Arc<stoq::Connection>) {
        self.connections
            .write()
            .await
            .insert(node_id.to_hex(), connection);
    }

    /// Get a connection to a node, if one is registered.
    async fn get_connection(&self, node_id: &NodeId) -> Option<Arc<stoq::Connection>> {
        self.connections.read().await.get(&node_id.to_hex()).cloned()
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
        let connection = self.get_connection(target).await.ok_or_else(|| {
            TransportError::NoConnection(format!("no connection to node {}", target.to_hex()))
        })?;

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
        let connection = self.get_connection(source).await.ok_or_else(|| {
            TransportError::NoConnection(format!("no connection to node {}", source.to_hex()))
        })?;

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
        self.get_connection(node).await.is_some()
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
