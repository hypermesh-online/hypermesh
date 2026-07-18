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
use crate::blockchain::block::BlockAssetEntry;
use crate::blockchain::node_chain::NodeBlockchain;
use crate::transport::error::TransportError;

/// Frame an enveloped SHARD_FETCH response (A6.6): a self-describing envelope
/// carrying the shard bytes plus (optionally) the ONE asset registration that
/// authorizes the shard, so the fetcher can re-anchor it on its own chain.
///
/// Wire format: `shard_len(4, u32 BE) + shard_data + registration_json(rest)`.
/// The trailing registration bytes are empty when no on-chain registration
/// covers this shard. An OLD server sends bare shard bytes (no length prefix);
/// the client disambiguates by content-addressing (BLAKE3 of the framed shard
/// must equal the requested id), so no version flag is needed on the wire.
fn frame_shard_response(shard_data: &[u8], registration_json: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + shard_data.len() + registration_json.len());
    out.extend_from_slice(&(shard_data.len() as u32).to_be_bytes());
    out.extend_from_slice(shard_data);
    out.extend_from_slice(registration_json);
    out
}

/// Build the enveloped serve response for a stored shard: look up the shard's
/// on-chain registration (when a blockchain handle is present) and attach it.
///
/// The registration is the content-bound `BlockAssetEntry` returned by
/// [`NodeBlockchain::registration_for_shard`]. When absent (no handle, or no
/// on-chain asset lists this shard), the shard is still served — the trailing
/// registration segment is simply empty (the fetcher caches it but does not
/// become an authoritative mirror, matching the pre-A6.6 behavior).
async fn build_enveloped_response(
    shard_id: &ContentHash,
    shard_data: &[u8],
    blockchain: Option<&NodeBlockchain>,
) -> Vec<u8> {
    let registration_json = match blockchain {
        Some(chain) => match chain.registration_for_shard(&shard_id.0).await {
            Some(entry) => serde_json::to_vec(&entry).unwrap_or_default(),
            None => Vec::new(),
        },
        None => Vec::new(),
    };
    frame_shard_response(shard_data, &registration_json)
}

/// Parse an enveloped (or legacy bare) SHARD_FETCH response.
///
/// Disambiguation is content-addressed and needs no wire version flag: try the
/// enveloped shape first (`len(4) + shard + registration_json`), accept it ONLY
/// when the framed shard BLAKE3-hashes to the requested `shard_id`; otherwise
/// treat the WHOLE response as bare shard bytes (old server). Either way the
/// caller re-verifies the returned shard against `shard_id` — this parse never
/// weakens that gate, it only recovers the optional trailing registration.
///
/// Returns `(shard_bytes, Some(entry))` when an enveloped registration parsed,
/// `(shard_bytes, None)` for enveloped-without-registration or the bare
/// fallback. An empty `response` (shard not found) yields `(vec![], None)`.
pub fn parse_shard_response(
    response: &[u8],
    shard_id: &ContentHash,
) -> (Vec<u8>, Option<BlockAssetEntry>) {
    if response.is_empty() {
        return (Vec::new(), None);
    }

    // Attempt the enveloped shape: 4-byte BE length prefix, then that many
    // shard bytes, then the (possibly empty) registration JSON.
    if response.len() >= 4 {
        let len = u32::from_be_bytes([response[0], response[1], response[2], response[3]]) as usize;
        if 4 + len <= response.len() {
            let shard = &response[4..4 + len];
            // Content-address gate: only trust the enveloped framing when the
            // extracted shard actually hashes to the id we asked for.
            if blake3::hash(shard).as_bytes() == &shard_id.0 {
                let reg_bytes = &response[4 + len..];
                let registration = if reg_bytes.is_empty() {
                    None
                } else {
                    serde_json::from_slice::<BlockAssetEntry>(reg_bytes).ok()
                };
                return (shard.to_vec(), registration);
            }
        }
    }

    // Legacy bare response: the whole payload is the shard (old server). The
    // caller's BLAKE3 gate validates it; we carry no registration.
    (response.to_vec(), None)
}

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

/// Handle a shard message when the tag byte has already been read.
///
/// `data` is the full message payload INCLUDING the tag byte at `data[0]`.
/// This is the same logic as [`handle_incoming_shard_stream`] but can be
/// called from a dispatch loop that has already received the stream data.
///
/// Returns `Ok(None)` for SHARD_SEND (no reply), `Ok(Some(bytes))` for
/// SHARD_FETCH (response data to send back).
///
/// A6.6: `blockchain` is the serving node's chain. On a SHARD_FETCH hit the
/// response is an ENVELOPE (`len(4)+shard+registration_json`) so the fetcher
/// can re-anchor the shard's registration on its own chain. Pass `None` to
/// serve legacy bare shard bytes (no registration).
pub async fn handle_shard_message(
    data: &[u8],
    store: &ShardStore,
    blockchain: Option<&NodeBlockchain>,
) -> Result<Option<Vec<u8>>, TransportError> {
    if data.is_empty() {
        return Err(TransportError::Protocol("empty shard message".into()));
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

            // Verify BLAKE3 integrity before storing
            let computed_hash = blake3::hash(&shard_data);
            if computed_hash.as_bytes() != &shard_id.0 {
                tracing::warn!(
                    "Shard BLAKE3 mismatch: expected {}, got {}",
                    hex::encode(shard_id.0),
                    hex::encode(computed_hash.as_bytes())
                );
                return Err(TransportError::Network("shard hash mismatch".into()));
            }

            store.store(shard_id, shard_data).await;
            tracing::debug!("Stored shard {} from peer", hex::encode(shard_id_bytes));
            Ok(None)
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
                    tracing::debug!("Serving shard {} to peer", hex::encode(shard_id_bytes));
                    // A6.6: attach the shard's on-chain registration (when
                    // present) so the fetcher can re-anchor it on its own chain.
                    let enveloped =
                        build_enveloped_response(&shard_id, &shard_data, blockchain).await;
                    Ok(Some(enveloped))
                }
                None => {
                    // Empty response indicates shard not found
                    Ok(Some(Vec::new()))
                }
            }
        }
        _ => Err(TransportError::Protocol(format!(
            "unknown shard tag: 0x{tag:02x}"
        ))),
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

            // Verify BLAKE3 integrity before storing
            let computed_hash = blake3::hash(&shard_data);
            if computed_hash.as_bytes() != &shard_id.0 {
                tracing::warn!(
                    "Shard BLAKE3 mismatch: expected {}, got {}",
                    hex::encode(shard_id.0),
                    hex::encode(computed_hash.as_bytes())
                );
                return Err(TransportError::Network("shard hash mismatch".into()));
            }

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

    // ── A6.6: enveloped SHARD_FETCH response framing + parse ─────────────

    fn sample_registration(shard_id: [u8; 32]) -> BlockAssetEntry {
        use crate::assets::core::AssetRegistration;
        use crate::blockchain::block::StoragePointer;
        use crate::matrix::coordinate::MatrixCoordinate;
        use trustchain::proof_of_state::StateProof;

        let coord = MatrixCoordinate::new(3, 3, 3).expect("test: coord");
        let reg = AssetRegistration::genesis(coord);
        let asset_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        BlockAssetEntry::new_bound(
            asset_hash,
            &StateProof::new_for_testing(),
            StoragePointer::Sharded {
                shard_hashes: vec![shard_id],
                placements: vec![coord],
            },
            reg,
        )
    }

    /// Round-trip: an enveloped response with a registration parses back to the
    /// exact shard bytes + the exact registration entry.
    #[test]
    fn parse_recovers_enveloped_shard_and_registration() {
        let shard_data: Vec<u8> = (0..777).map(|i| (i % 256) as u8).collect();
        let shard_id = ContentHash(*blake3::hash(&shard_data).as_bytes());
        let entry = sample_registration(shard_id.0);
        let reg_json = serde_json::to_vec(&entry).expect("test: serialize entry");

        let framed = frame_shard_response(&shard_data, &reg_json);
        let (got_shard, got_reg) = parse_shard_response(&framed, &shard_id);

        assert_eq!(got_shard, shard_data, "shard bytes must round-trip");
        assert_eq!(
            got_reg.expect("test: registration recovered"),
            entry,
            "registration entry must round-trip exactly",
        );
    }

    /// An enveloped response with NO registration (empty trailing segment)
    /// parses to the shard and `None`.
    #[test]
    fn parse_recovers_enveloped_shard_without_registration() {
        let shard_data = vec![0xAB; 300];
        let shard_id = ContentHash(*blake3::hash(&shard_data).as_bytes());

        let framed = frame_shard_response(&shard_data, &[]);
        let (got_shard, got_reg) = parse_shard_response(&framed, &shard_id);

        assert_eq!(got_shard, shard_data);
        assert!(got_reg.is_none(), "no trailing bytes → no registration");
    }

    /// BACKWARDS-COMPAT: a BARE response (old server, no length prefix) is
    /// treated as raw shard bytes with no registration — the content-address
    /// gate disambiguates it from the enveloped shape.
    #[test]
    fn parse_falls_back_to_bare_response() {
        // Bare shard bytes: exactly what the legacy `handle_incoming_shard_stream`
        // serve path sends (`stream.send(&shard_data)`).
        let shard_data = vec![0xCD; 64];
        let shard_id = ContentHash(*blake3::hash(&shard_data).as_bytes());

        let (got_shard, got_reg) = parse_shard_response(&shard_data, &shard_id);

        assert_eq!(got_shard, shard_data, "bare bytes returned as the shard");
        assert!(got_reg.is_none(), "bare response carries no registration");
    }

    /// A bare response whose leading 4 bytes HAPPEN to look like a length prefix
    /// still falls back to bare, because the framed-shard content-address gate
    /// fails (the framed slice does not hash to the requested id).
    #[test]
    fn parse_bare_response_with_prefixlike_bytes_falls_back() {
        // 40 bytes; the first 4 read as a large BE length (0x01020304) that is
        // NOT <= remaining, so the envelope branch is skipped and we fall back.
        let mut shard_data = vec![0x01u8, 0x02, 0x03, 0x04];
        shard_data.extend_from_slice(&[0x55u8; 36]);
        let shard_id = ContentHash(*blake3::hash(&shard_data).as_bytes());

        let (got_shard, got_reg) = parse_shard_response(&shard_data, &shard_id);
        assert_eq!(got_shard, shard_data);
        assert!(got_reg.is_none());
    }

    /// An empty response (shard not found) yields empty bytes + no registration.
    #[test]
    fn parse_empty_response_is_not_found() {
        let shard_id = ContentHash([0x11u8; 32]);
        let (got_shard, got_reg) = parse_shard_response(&[], &shard_id);
        assert!(got_shard.is_empty());
        assert!(got_reg.is_none());
    }

    /// The enveloped serve path (`build_enveloped_response`) with a blockchain
    /// that HAS the registration produces bytes the client parses back to the
    /// registration; with `None` blockchain it produces a registration-less
    /// envelope. End-to-end frame↔parse symmetry across the serve+fetch split.
    #[tokio::test]
    async fn serve_envelope_round_trips_through_parse() {
        use crate::blockchain::node_chain::NodeBlockchain;
        use crate::matrix::coordinate::MatrixCoordinate;

        let coord = MatrixCoordinate::new(4, 4, 4).expect("test: coord");
        let chain = NodeBlockchain::new(coord);

        // Register a sharded asset on the chain so registration_for_shard hits.
        let shard_data = vec![0x7Au8; 200];
        let shard_id = ContentHash(*blake3::hash(&shard_data).as_bytes());
        let entry = sample_registration(shard_id.0);
        // new_for_testing proof passes add_block's validate().
        chain
            .add_block(vec![entry.clone()])
            .await
            .expect("test: register sharded asset");

        // Serve WITH chain → envelope carries the registration.
        let served = build_enveloped_response(&shard_id, &shard_data, Some(&chain)).await;
        let (got_shard, got_reg) = parse_shard_response(&served, &shard_id);
        assert_eq!(got_shard, shard_data);
        assert!(got_reg.is_some(), "serve-with-chain must attach a registration");

        // Serve WITHOUT chain → registration-less envelope.
        let served_none = build_enveloped_response(&shard_id, &shard_data, None).await;
        let (got_shard2, got_reg2) = parse_shard_response(&served_none, &shard_id);
        assert_eq!(got_shard2, shard_data);
        assert!(got_reg2.is_none(), "serve-without-chain carries no registration");
    }
}
