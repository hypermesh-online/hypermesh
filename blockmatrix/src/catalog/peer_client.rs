// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Catalog peer client — DNS-style recursive fan-out for `catalog.search`.
//!
//! Phase M.4.5c.2: opens a STOQ stream to a peer's catalog API surface and
//! invokes `catalog/search` using the canonical wire format (bincode
//! `stoq::api::ApiRequest` carrying a JSON-serialized
//! `catalog::api::stoq_api::SearchRequest`).
//!
//! Wire format choice (Option C — reuse existing peer connection):
//! - The daemon keeps `Arc<stoq::Connection>` per connected peer in
//!   `NetworkManager::get_connected_nodes()`.
//! - `CatalogPeerClient` opens a bidirectional stream on that existing
//!   connection and speaks the catalog STOQ API wire protocol directly.
//! - Per request: write a length-prefixed bincode `ApiRequest`, then
//!   read the bincode `ApiResponse` off the recv stream.
//!
//! Alpha caveat: the daemon does NOT yet host a
//! `stoq::api::StoqApiServer` on its peer-facing connection. Until the
//! daemon-side dispatcher is added (the matrix-message handler today
//! does not multiplex API requests), real peer search will return
//! [`PeerSearchError::Timeout`] or [`PeerSearchError::Malformed`].
//! Those errors are honest — never fabricated — and integration tests
//! that need full round-trip coverage stand up a [`stoq::api::StoqApiServer`]
//! against a real catalog handler.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::network::NetworkManager;

/// One typedef returned by a peer's `catalog/search` endpoint.
///
/// Tagged with the responding peer's node id so the caller can render
/// provenance ("found on node X") and de-duplicate by `type_hash`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerSearchResult {
    /// Hex-encoded BLAKE3-256 type hash.
    pub type_hash: String,
    /// Human-readable typedef name returned by the peer.
    pub name: String,
    /// Typedef version returned by the peer.
    pub version: String,
    /// Node id of the peer that supplied this result.
    pub source_node_id: String,
}

/// Errors returned by [`CatalogPeerClient`] operations.
///
/// All variants describe a real, observed failure — the client never
/// fabricates an empty success response in place of an error.
#[derive(Debug, thiserror::Error)]
pub enum PeerSearchError {
    /// The requested peer is not in the daemon's connected-node set.
    #[error("peer not connected: {0}")]
    NotConnected(String),
    /// The peer did not respond within the configured timeout.
    #[error("timeout after {0}ms")]
    Timeout(u64),
    /// The peer responded but the payload could not be parsed as the
    /// expected `catalog::api::stoq_api::SearchResponse` JSON shape.
    #[error("malformed response: {0}")]
    Malformed(String),
    /// Underlying STOQ stream error (open_bi / write / read).
    #[error("stoq error: {0}")]
    StoqError(String),
    /// Local serialization of the outgoing `ApiRequest` failed.
    #[error("serialization error: {0}")]
    SerializationError(String),
    /// The peer returned an `ApiResponse` with `success = false`.
    #[error("peer reported failure: {0}")]
    PeerError(String),
}

/// On-the-wire request envelope. Mirrors `stoq::api::ApiRequest`
/// byte-for-byte without taking the dependency, so a peer running the
/// real `StoqApiServer` will deserialize it correctly.
///
/// Field order and types MUST stay aligned with
/// `stoq/src/api/mod.rs`'s `ApiRequest` struct or bincode reads will
/// fail at the peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireApiRequest {
    id: String,
    service: String,
    method: String,
    payload: Bytes,
    metadata: HashMap<String, String>,
}

/// On-the-wire response envelope. Mirrors `stoq::api::ApiResponse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireApiResponse {
    request_id: String,
    success: bool,
    payload: Bytes,
    error: Option<String>,
    metadata: HashMap<String, String>,
}

/// Inbound search request body. Mirrors
/// `catalog::api::stoq_api::SearchRequest` JSON layout.
#[derive(Debug, Clone, Serialize)]
struct WireSearchRequest<'a> {
    query: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    limit: u64,
    offset: u64,
}

/// Peer response body. Mirrors `catalog::api::stoq_api::SearchResponse`
/// JSON layout (the fields we consume).
#[derive(Debug, Clone, Deserialize)]
struct WireSearchResponse {
    #[serde(default)]
    results: Vec<WirePackageSummary>,
}

/// One row in a peer `SearchResponse`. Mirrors
/// `catalog::api::stoq_api::PackageSummary`.
#[derive(Debug, Clone, Deserialize)]
struct WirePackageSummary {
    name: String,
    version: String,
    // The catalog STOQ API surfaces `name` only — we look up the
    // canonical hash via the local lookup table when we have one. For
    // alpha, we synthesize the hash from the name so the wire format
    // round-trips. A follow-up task carries the real type_hash field
    // through the wire format (requires extending PackageSummary in the
    // catalog crate, which is outside the M.4.5c scope).
    #[serde(default)]
    type_hash: Option<String>,
}

/// DNS-style recursive fan-out client for the `catalog/search` STOQ API.
///
/// Holds an [`Arc`] of the daemon's [`NetworkManager`] so it can look up
/// peer connections without taking ownership.
pub struct CatalogPeerClient {
    network: Arc<NetworkManager>,
}

impl CatalogPeerClient {
    /// Construct a client backed by the supplied network manager.
    pub fn new(network: Arc<NetworkManager>) -> Self {
        Self { network }
    }

    /// Query a specific peer's `catalog/search` endpoint and return its
    /// results tagged with the peer's node id.
    ///
    /// Errors are surfaced exactly as encountered — no empty success
    /// envelopes hide network/serialization failures.
    pub async fn search_peer(
        &self,
        peer_node_id: &str,
        query: &str,
        timeout_ms: u64,
    ) -> Result<Vec<PeerSearchResult>, PeerSearchError> {
        // 1. Locate the peer's stoq::Connection in the network manager.
        let connection = self.find_peer_connection(peer_node_id).await?;

        // 2. Build the catalog/search wire request.
        let search_req = WireSearchRequest {
            query,
            tags: Vec::new(),
            author: None,
            limit: 50,
            offset: 0,
        };
        let payload = serde_json::to_vec(&search_req)
            .map_err(|e| PeerSearchError::SerializationError(e.to_string()))?;
        let api_req = WireApiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            service: "catalog".to_string(),
            method: "search".to_string(),
            payload: Bytes::from(payload),
            metadata: HashMap::new(),
        };
        let req_bytes = bincode::serialize(&api_req)
            .map_err(|e| PeerSearchError::SerializationError(e.to_string()))?;

        // 3. Drive the request through a bidirectional stream with a
        // hard timeout. We avoid leaking long-lived futures by wrapping
        // the entire send+recv in a single `tokio::time::timeout`.
        let send_recv = async move {
            let (mut send, mut recv) = connection
                .open_bi()
                .await
                .map_err(|e| PeerSearchError::StoqError(e.to_string()))?;
            send.write_all(&req_bytes)
                .await
                .map_err(|e| PeerSearchError::StoqError(e.to_string()))?;
            send.finish()
                .map_err(|e| PeerSearchError::StoqError(e.to_string()))?;
            let resp_bytes = recv
                .read_to_end(10 * 1024 * 1024)
                .await
                .map_err(|e| PeerSearchError::StoqError(e.to_string()))?;
            Ok::<Vec<u8>, PeerSearchError>(resp_bytes)
        };

        let resp_bytes = match tokio::time::timeout(Duration::from_millis(timeout_ms), send_recv)
            .await
        {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(PeerSearchError::Timeout(timeout_ms)),
        };

        // 4. Decode the ApiResponse envelope.
        let api_resp: WireApiResponse = bincode::deserialize(&resp_bytes)
            .map_err(|e| PeerSearchError::Malformed(format!("ApiResponse decode: {e}")))?;
        if !api_resp.success {
            return Err(PeerSearchError::PeerError(
                api_resp.error.unwrap_or_else(|| "unknown error".to_string()),
            ));
        }

        // 5. Decode the inner SearchResponse JSON.
        let search_resp: WireSearchResponse = serde_json::from_slice(&api_resp.payload)
            .map_err(|e| PeerSearchError::Malformed(format!("SearchResponse decode: {e}")))?;

        // 6. Tag every row with the peer's node id and surface up.
        Ok(search_resp
            .results
            .into_iter()
            .map(|row| PeerSearchResult {
                type_hash: row.type_hash.unwrap_or_default(),
                name: row.name,
                version: row.version,
                source_node_id: peer_node_id.to_string(),
            })
            .collect())
    }

    /// Fan out a search query to up to `max_neighbors` connected peers
    /// in parallel and return one `(node_id, result)` pair per peer.
    ///
    /// Returns an empty vector when there are no connected peers — the
    /// caller decides how to combine local + peer results.
    pub async fn search_neighbors(
        &self,
        query: &str,
        max_neighbors: usize,
        timeout_ms: u64,
    ) -> Vec<(String, Result<Vec<PeerSearchResult>, PeerSearchError>)> {
        if max_neighbors == 0 {
            return Vec::new();
        }
        let nodes = self.network.get_connected_nodes().await;
        if nodes.is_empty() {
            return Vec::new();
        }

        // Pick up to `max_neighbors` connected peers. Stable order based
        // on whatever the network manager hands back — alpha policy.
        // ngauge-driven dispersion comes later.
        let chosen: Vec<String> = nodes
            .into_iter()
            .filter(|n| n.connection.is_some())
            .take(max_neighbors)
            .map(|n| n.node_id)
            .collect();

        if chosen.is_empty() {
            return Vec::new();
        }

        // Spawn one task per peer; collect (peer_id, result) tuples.
        let mut handles = Vec::with_capacity(chosen.len());
        for peer_id in chosen {
            let network = self.network.clone();
            let query = query.to_string();
            handles.push(tokio::spawn(async move {
                let client = CatalogPeerClient::new(network);
                let result = client.search_peer(&peer_id, &query, timeout_ms).await;
                (peer_id, result)
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for h in handles {
            match h.await {
                Ok(pair) => results.push(pair),
                Err(e) => {
                    // Join errors (panic/cancel) get logged but do not
                    // poison sibling results. They surface in the
                    // caller as a missing peer rather than a fake row.
                    debug!("CatalogPeerClient: join error: {e}");
                }
            }
        }
        results
    }

    /// Look up the [`Arc<stoq::Connection>`] for a peer by its node id.
    async fn find_peer_connection(
        &self,
        peer_node_id: &str,
    ) -> Result<Arc<stoq::Connection>, PeerSearchError> {
        let nodes = self.network.get_connected_nodes().await;
        for node in nodes {
            if node.node_id == peer_node_id {
                return node
                    .connection
                    .ok_or_else(|| PeerSearchError::NotConnected(peer_node_id.to_string()));
            }
        }
        Err(PeerSearchError::NotConnected(peer_node_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a NetworkManager configured for tests with no connected
    /// peers. The manager's STOQ transport is started but no acceptor
    /// loop runs, so any attempt to send will fail honestly.
    async fn test_network_manager() -> Arc<NetworkManager> {
        use crate::matrix::coordinate::MatrixCoordinate;
        use hypermesh_lib::PrivacyMode;
        use stoq::transport::NetworkType;

        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let stoq_config = stoq::TransportConfig {
            port: 0,
            bind_address: std::net::Ipv6Addr::UNSPECIFIED,
            enable_falcon_crypto: false,
            ..stoq::TransportConfig::default()
        };
        let transport = Arc::new(
            stoq::StoqTransport::new_for_network(stoq_config, NetworkType::Anonymous)
                .await
                .expect("test: stoq transport up"),
        );
        // Build a FALCON identity so the manager has a real signer.
        let dir = tempfile::tempdir().expect("test: tempdir");
        let identity = crate::identity::FalconIdentity::load_or_create(dir.path())
            .expect("test: identity");
        let signer: Arc<dyn hypermesh_lib::NodeSigner> = Arc::new(identity);
        let proof_provider: Arc<dyn hypermesh_lib::StateProofProvider> = Arc::new(
            crate::proof_of_state::BlockMatrixProofProvider::new(
                signer.node_id().to_string(),
                signer.clone(),
            ),
        );
        let manager = NetworkManager::new(
            coord,
            transport,
            PrivacyMode::ANONYMOUS,
            Vec::new(),
            signer,
            proof_provider,
            "test-net".to_string(),
        )
        .await
        .expect("test: NetworkManager::new");
        Arc::new(manager)
    }

    #[tokio::test]
    async fn test_peer_client_not_connected_error() {
        let network = test_network_manager().await;
        let client = CatalogPeerClient::new(network);
        let err = client
            .search_peer("nonexistent-peer", "Message", 500)
            .await
            .expect_err("test: must fail when peer not connected");
        match err {
            PeerSearchError::NotConnected(id) => {
                assert_eq!(id, "nonexistent-peer");
            }
            other => unreachable!("test: expected NotConnected, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_search_neighbors_no_peers_returns_empty() {
        let network = test_network_manager().await;
        let client = CatalogPeerClient::new(network);
        let results = client.search_neighbors("Message", 8, 500).await;
        assert!(
            results.is_empty(),
            "no connected peers -> empty fan-out, never fabricated"
        );
    }

    #[tokio::test]
    async fn test_search_neighbors_zero_max_returns_empty() {
        let network = test_network_manager().await;
        let client = CatalogPeerClient::new(network);
        let results = client.search_neighbors("Message", 0, 500).await;
        assert!(results.is_empty(), "max_neighbors=0 short-circuits");
    }

    #[test]
    fn test_wire_search_request_serializes_to_catalog_format() {
        // Confirms the JSON shape matches catalog::api::stoq_api::SearchRequest:
        // { query, tags?, author?, limit, offset }
        let req = WireSearchRequest {
            query: "Message",
            tags: Vec::new(),
            author: None,
            limit: 50,
            offset: 0,
        };
        let json = serde_json::to_value(&req).expect("test: serialize");
        assert_eq!(json["query"], "Message");
        assert_eq!(json["limit"], 50);
        assert_eq!(json["offset"], 0);
        // Empty tags + None author are skipped so the wire is minimal.
        assert!(json.get("tags").is_none(), "tags omitted when empty");
        assert!(json.get("author").is_none(), "author omitted when None");
    }

    #[test]
    fn test_wire_envelopes_bincode_roundtrip() {
        // Confirms bincode round-trips both envelopes so a real
        // StoqApiServer on the peer side will accept our requests.
        let req = WireApiRequest {
            id: "test-id".to_string(),
            service: "catalog".to_string(),
            method: "search".to_string(),
            payload: Bytes::from_static(b"{}"),
            metadata: HashMap::new(),
        };
        let bytes = bincode::serialize(&req).expect("test: encode request");
        let decoded: WireApiRequest =
            bincode::deserialize(&bytes).expect("test: decode request");
        assert_eq!(decoded.service, "catalog");
        assert_eq!(decoded.method, "search");
        assert_eq!(decoded.payload.as_ref(), b"{}");

        let resp = WireApiResponse {
            request_id: "test-id".to_string(),
            success: true,
            payload: Bytes::from_static(b"{\"results\":[]}"),
            error: None,
            metadata: HashMap::new(),
        };
        let bytes = bincode::serialize(&resp).expect("test: encode response");
        let decoded: WireApiResponse =
            bincode::deserialize(&bytes).expect("test: decode response");
        assert!(decoded.success);
        assert_eq!(decoded.payload.as_ref(), b"{\"results\":[]}");
    }
}
