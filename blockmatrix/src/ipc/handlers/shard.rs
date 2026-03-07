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

    // 2. Try network fetch from connected peers
    if let Some(ref network) = state.network {
        let peers = network.get_connected_nodes().await;
        for peer in peers.iter().take(6) {
            if let Some(ref conn) = peer.connection {
                match fetch_shard_from_peer(conn, &content_hash).await {
                    Ok(data) if !data.is_empty() => {
                        // BLAKE3 verify before returning
                        let computed = blake3::hash(&data);
                        if computed.as_bytes() == &shard_id_bytes {
                            // Cache locally for future requests
                            state.shard_store.store(content_hash, data.clone()).await;
                            return Ok(serde_json::json!({
                                "source": "network",
                                "peer": &peer.node_id[..8.min(peer.node_id.len())],
                                "data": hex::encode(&data),
                            }));
                        }
                        tracing::warn!(
                            "Shard from peer {} BLAKE3 mismatch",
                            &peer.node_id[..8.min(peer.node_id.len())]
                        );
                    }
                    _ => continue,
                }
            }
        }
    }

    Err(rpc_err(
        -32001,
        format!("shard {shard_id_hex} not found locally or on network"),
    ))
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
}
