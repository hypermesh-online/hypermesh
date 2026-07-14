// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Shard IPC handlers — local lookup and network fetch fallback.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INVALID_PARAMS};
use crate::ipc::state::DaemonState;
use hypermesh_lib::ContentHash;

/// Register shard-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // shard.fetch — look up a shard by its BLAKE3 hash, first in the local
    // ShardStore, then by requesting it from connected network peers.
    {
        let s = state.clone();
        handler.register(
            "shard.fetch",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_shard_fetch(params, &s).await })
            }),
        );
    }
}

fn rpc_err(code: i64, message: impl Into<String>) -> RpcError {
    RpcError {
        code,
        message: message.into(),
        data: None,
    }
}

async fn handle_shard_fetch(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let shard_id_hex = params["shard_id"]
        .as_str()
        .ok_or_else(|| rpc_err(INVALID_PARAMS, "missing 'shard_id' parameter"))?;

    let shard_id_bytes: [u8; 32] = hex::decode(shard_id_hex)
        .map_err(|e| rpc_err(INVALID_PARAMS, format!("invalid shard_id hex: {e}")))?
        .try_into()
        .map_err(|_| rpc_err(INVALID_PARAMS, "shard_id must be 32 bytes"))?;

    let content_hash = ContentHash(shard_id_bytes);

    // 1. Try local shard store
    if let Some(data) = state.shard_store.get(&content_hash).await {
        return Ok(serde_json::json!({
            "source": "local",
            "data": hex::encode(&data),
        }));
    }

    // 2. Try network fetch from connected peers.
    //
    // A2 two-layer resolve: consult the live-mirror layer (ShardLocationIndex,
    // populated by TAG_SHARD_ANNOUNCE) FIRST — known providers are a directed
    // hint instead of a blind broadcast. The canonical matrix-placement layer
    // is NOT available on this bare-content-hash IPC entrypoint (there is no
    // RetrievalPlan here — the ClientAssembler path carries canonical
    // coordinates); on a miss we widen via the upstream tracker fallback
    // (Part 2) rather than canonical placement.
    if let Some(ref network) = state.network {
        let peers = network.get_connected_nodes().await;

        // Live-mirror providers known to hold this shard (non-expired). Empty
        // when the index is unwired (Private mode / test fixtures) or on a miss.
        let mut known_providers: Vec<String> = match state.shard_location_index {
            Some(ref index) => index.get_providers(&content_hash).await,
            None => Vec::new(),
        };

        // First pass: order connected peers with live-mirror providers first,
        // then everyone else (bounded fallback scan).
        if let Some(result) =
            try_fetch_from_ordered_peers(state, network, &peers, &content_hash, &known_providers)
                .await?
        {
            return Ok(result);
        }

        // A2 Part 2 — upstream tracker fallback. Local store + live mirrors +
        // connected-peer scan all missed. Ask a bounded set of upstream peers
        // "who has content_hash X?" (the shard analog of DNS upstream), merge
        // their answers, and retry any newly-named provider that is connected.
        let upstream_providers =
            query_upstream_trackers(&peers, &content_hash, &known_providers).await;
        if !upstream_providers.is_empty() {
            // Merge new provider ids ahead of the (now-exhausted) old ones and
            // retry only the newly-learned, still-connected providers.
            for id in &upstream_providers {
                if !known_providers.contains(id) {
                    known_providers.push(id.clone());
                }
            }
            let retry_peers: Vec<_> = peers
                .iter()
                .filter(|p| upstream_providers.iter().any(|id| id == &p.node_id))
                .cloned()
                .collect();
            if let Some(result) = try_fetch_from_ordered_peers(
                state,
                network,
                &retry_peers,
                &content_hash,
                &upstream_providers,
            )
            .await?
            {
                return Ok(result);
            }
        }
    }

    Err(rpc_err(
        -32001,
        format!("shard {shard_id_hex} not found locally or on network"),
    ))
}

/// Try to fetch a shard from an ordered set of connected peers.
///
/// Peers whose node_id appears in `known_providers` are tried first (directed
/// fetch), then the rest (bounded fallback scan). On the first BLAKE3-valid
/// response the shard is seeded (consumer-becomes-provider, gated on
/// `authorizes_shard`) and the JSON result is returned. Returns `Ok(None)` when
/// no peer in the set produced a valid shard.
async fn try_fetch_from_ordered_peers(
    state: &DaemonState,
    network: &Arc<crate::network::NetworkManager>,
    peers: &[crate::network::NetworkNode],
    content_hash: &ContentHash,
    known_providers: &[String],
) -> Result<Option<serde_json::Value>, RpcError> {
    let mut ordered: Vec<&crate::network::NetworkNode> = peers
        .iter()
        .filter(|p| known_providers.iter().any(|id| id == &p.node_id))
        .collect();
    ordered.extend(
        peers
            .iter()
            .filter(|p| !known_providers.iter().any(|id| id == &p.node_id)),
    );

    for peer in ordered.into_iter().take(6) {
        let Some(ref conn) = peer.connection else {
            continue;
        };
        match fetch_shard_from_peer(conn, content_hash).await {
            Ok(data) if !data.is_empty() => {
                // BLAKE3 content gate (mirror invariant #1): the fetched shard
                // MUST hash to its claimed content address before we return,
                // seed, or re-announce it.
                let computed = blake3::hash(&data);
                if computed.as_bytes() != &content_hash.0 {
                    tracing::warn!(
                        "Shard from peer {} BLAKE3 mismatch",
                        &peer.node_id[..8.min(peer.node_id.len())]
                    );
                    continue;
                }

                let announce_targets =
                    seed_fetched_shard(state, network, content_hash, &data).await;

                return Ok(Some(serde_json::json!({
                    "source": "network",
                    "peer": &peer.node_id[..8.min(peer.node_id.len())],
                    "data": hex::encode(&data),
                    "announce_targets": announce_targets,
                })));
            }
            _ => continue,
        }
    }

    Ok(None)
}

/// Seed a freshly-fetched, BLAKE3-verified shard: cache it, and — ONLY when the
/// shard belongs to an asset registered (content-bound) on OUR chain — register
/// as a provider and re-announce (consumer-becomes-provider, R12).
///
/// A2 P1 gate: `blockchain.authorizes_shard` is the on-chain "signed-to-content"
/// authority. Every registering entry carries a validated, content-bound
/// StateProof (`content_binding_ok`), so a positive answer means the shard is
/// part of an asset whose registration this node verified. We become an
/// advertised PROVIDER only for such shards — matching the serve-auth invariant
/// (`peer_connection::authorize_shard_fetch` gates serving on the same
/// `authorizes_shard`). The DATA is always returned to the caller; only the
/// re-announce is gated. This ADDS a gate — it does not weaken the existing
/// BLAKE3 content check.
///
/// Returns the number of peers the announcement reached (0 when the shard is
/// not on-chain-authorized, no manager is wired, or there are no peers).
async fn seed_fetched_shard(
    state: &DaemonState,
    network: &Arc<crate::network::NetworkManager>,
    content_hash: &ContentHash,
    data: &[u8],
) -> usize {
    // Always cache locally so repeat fetches hit the local store.
    state.shard_store.store(*content_hash, data.to_vec()).await;

    // P1 signed-to-content gate on the become-provider path: only advertise as
    // a provider for shards of assets registered + content-bound on our chain.
    if !reannounce_authorized(state, content_hash).await {
        tracing::debug!(
            "Fetched shard {} cached but NOT re-announced (no content-bound asset on our chain)",
            hex::encode(&content_hash.0[..8]),
        );
        return 0;
    }

    let Some(ref mgr) = state.consumer_provider_manager else {
        // No manager wired (test fixtures / Private mode). Shard is already
        // cached above; nothing to announce.
        return 0;
    };

    let result = mgr
        .process_fetched_shards(vec![(*content_hash, data.to_vec())])
        .await;
    let Some(payload) = result.announcement_payload else {
        return 0;
    };

    let conns: Vec<Arc<stoq::Connection>> = network
        .get_connected_nodes()
        .await
        .into_iter()
        .filter_map(|n| n.connection.clone())
        .collect();
    crate::network::consumer_provider::broadcast_announcement(&payload, &conns).await
}

/// P1 signed-to-content gate for the become-provider re-announce.
///
/// Returns `true` only when the shard belongs to an asset registered +
/// content-bound on THIS node's chain (`blockchain.authorizes_shard`). This is
/// the same on-chain anchor the serve path enforces
/// (`peer_connection::authorize_shard_fetch`), factored out so the gate can be
/// tested independently of the network layer.
async fn reannounce_authorized(state: &DaemonState, content_hash: &ContentHash) -> bool {
    state.blockchain.authorizes_shard(&content_hash.0).await
}

/// Query a bounded set of upstream peers "who has `content_hash`?" and return
/// the merged provider node_ids they know that we did NOT already have.
///
/// A2 Part 2: mirrors the DNS cache-first→upstream fallback. Bounded to at most
/// two upstream hops, each timeout-guarded via `locate_shard_upstream`. Every
/// connected peer is its own mini-tracker (it answers from its own
/// `ShardLocationIndex` + local store), so we simply ask the first couple of
/// connected peers. Providers we already knew are filtered out.
async fn query_upstream_trackers(
    peers: &[crate::network::NetworkNode],
    content_hash: &ContentHash,
    known_providers: &[String],
) -> Vec<String> {
    use crate::network::consumer_provider::{locate_shard_upstream, SHARD_LOCATE_TIMEOUT};

    let mut merged: Vec<String> = Vec::new();
    let mut hops = 0usize;

    for peer in peers.iter() {
        if hops >= 2 {
            break;
        }
        let Some(ref conn) = peer.connection else {
            continue;
        };
        hops += 1;

        let answers = locate_shard_upstream(conn, content_hash, SHARD_LOCATE_TIMEOUT).await;
        for id in answers {
            if !known_providers.contains(&id) && !merged.contains(&id) {
                merged.push(id);
            }
        }
    }

    merged
}

/// Fetch a shard from a connected peer using the SHARD_FETCH wire protocol.
async fn fetch_shard_from_peer(
    conn: &Arc<stoq::Connection>,
    shard_id: &ContentHash,
) -> Result<Vec<u8>, String> {
    let mut stream = conn
        .open_stream()
        .await
        .map_err(|e| format!("open stream: {e}"))?;

    // Wire format: tag(0x02) + shard_id(32)
    let mut request = Vec::with_capacity(33);
    request.push(0x02); // SHARD_FETCH tag
    request.extend_from_slice(&shard_id.0);

    stream
        .send(&request)
        .await
        .map_err(|e| format!("send fetch request: {e}"))?;

    let response = stream
        .receive()
        .await
        .map_err(|e| format!("receive shard: {e}"))?;

    Ok(response.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::handler::RequestHandler;
    use crate::ipc::protocol::RpcRequest;

    #[tokio::test]
    async fn test_shard_fetch_missing() {
        let state = crate::ipc::handlers::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "shard.fetch",
            serde_json::json!({ "shard_id": "aa".repeat(32) }),
        );
        let resp = handler.dispatch(req).await;
        // Should return error since shard doesn't exist
        assert!(resp.error.is_some());
    }

    #[tokio::test]
    async fn test_shard_fetch_local() {
        let state = crate::ipc::handlers::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        // Store a shard
        let data = vec![0xAB; 128];
        let hash = blake3::hash(&data);
        let content_hash = ContentHash(*hash.as_bytes());
        state.shard_store.store(content_hash, data.clone()).await;

        let req = RpcRequest::new(
            "shard.fetch",
            serde_json::json!({ "shard_id": hex::encode(hash.as_bytes()) }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "should succeed for local shard");
        let result = resp.result.expect("test: result present");
        assert_eq!(result["source"], "local");
        assert_eq!(result["data"], hex::encode(&data));
    }

    // ── A2: P1 signed-to-content re-announce gate ────────────────────────

    /// Register a content-bound sharded asset on `state`'s chain so that
    /// `authorizes_shard(shard_id)` is true for the given shard. Mirrors the
    /// `chain.rs` `sharded_entry` helper, using `new_bound` so the
    /// signed-to-content binding holds by construction.
    async fn register_sharded_asset(state: &DaemonState, shard_id: [u8; 32]) {
        use crate::assets::core::AssetRegistration;
        use crate::blockchain::block::{Block, BlockAssetEntry, StoragePointer};
        use trustchain::proof_of_state::StateProof;

        let coord = state.coordinate;
        let reg = AssetRegistration::genesis(coord);
        let asset_hash = *blake3::hash(reg.to_string().as_bytes()).as_bytes();
        let state_proof = StateProof::default();
        let entry = BlockAssetEntry::new_bound(
            asset_hash,
            &state_proof,
            StoragePointer::Sharded {
                shard_hashes: vec![shard_id],
                placements: vec![coord],
            },
            reg,
        );

        // Binding must hold — this is the P1 invariant the gate depends on.
        assert!(
            entry.content_binding_ok(),
            "test fixture must be content-bound",
        );

        let head = state.blockchain.get_head().await.expect("test: head");
        let block = Block::new(1, vec![entry], head.hash.clone());
        state
            .blockchain
            .insert_block(block)
            .await
            .expect("test: insert sharded asset block");
    }

    /// The re-announce gate PASSES only when the shard belongs to an asset
    /// registered + content-bound on our chain (`authorizes_shard`).
    #[tokio::test]
    async fn test_reannounce_gate_allows_onchain_content_bound_shard() {
        let state = crate::ipc::handlers::tests::test_state().await;
        let shard_id = [0x77u8; 32];
        register_sharded_asset(&state, shard_id).await;

        assert!(
            reannounce_authorized(&state, &ContentHash(shard_id)).await,
            "an on-chain, content-bound shard must be re-announce-authorized",
        );
    }

    /// The re-announce gate REJECTS a shard whose asset is NOT registered
    /// (content-bound) on our chain — the consumer caches the shard but does
    /// NOT advertise itself as an authoritative provider. This proves the P1
    /// signed-to-content gate is enforced on the become-provider path.
    #[tokio::test]
    async fn test_reannounce_gate_rejects_unbound_shard() {
        let state = crate::ipc::handlers::tests::test_state().await;

        // A shard with no registered, content-bound asset on our chain.
        let orphan_shard = [0xEEu8; 32];
        assert!(
            !reannounce_authorized(&state, &ContentHash(orphan_shard)).await,
            "a shard not content-bound on our chain must NOT be re-announced",
        );
    }
}
