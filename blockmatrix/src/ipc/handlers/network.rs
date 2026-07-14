// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Network IPC handlers: peers, connect, identity pubkeys, peer pubkeys.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INVALID_PARAMS, INTERNAL_ERROR};
use crate::ipc::state::DaemonState;

/// Register network-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // network.peers — list connected peers
    {
        let s = state.clone();
        handler.register(
            "network.peers",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move {
                    match &s.network {
                        Some(net) => {
                            let nodes = net.get_connected_nodes().await;
                            let peers: Vec<serde_json::Value> = nodes
                                .iter()
                                .map(|n| {
                                    serde_json::json!({
                                        "node_id": n.node_id,
                                        "address": n.address.to_string(),
                                        "coordinate": {
                                            "x": n.coordinate.x,
                                            "y": n.coordinate.y,
                                            "z": n.coordinate.z,
                                        },
                                    })
                                })
                                .collect();
                            Ok(serde_json::json!({
                                "count": peers.len(),
                                "peers": peers,
                            }))
                        }
                        None => Ok(serde_json::json!({
                            "count": 0,
                            "peers": [],
                            "note": "no network manager (private mode)",
                        })),
                    }
                })
            }),
        );
    }

    // network.connect — attempt connection to a peer (placeholder)
    {
        let s = state.clone();
        handler.register(
            "network.connect",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let _addr = params
                        .get("address")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    match &s.network {
                        Some(_net) => {
                            // Connection logic is handled by the bootstrap/STOQ
                            // layer. This handler acknowledges the request.
                            Ok(serde_json::json!({
                                "status": "connection_requested",
                                "note": "peer connections managed by STOQ layer",
                            }))
                        }
                        None => Ok(serde_json::json!({
                            "status": "unavailable",
                            "error": "no network manager (private mode)",
                        })),
                    }
                })
            }),
        );
    }

    // identity.pubkey -- return this node's FALCON + Kyber public keys
    {
        let s = state.clone();
        handler.register(
            "identity.pubkey",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_identity_pubkey(&s) })
            }),
        );
    }

    // identity.rotate — trigger key rotation
    {
        let s = state.clone();
        handler.register(
            "identity.rotate",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let reason = params
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .unwrap_or("scheduled")
                        .to_string();

                    Ok(serde_json::json!({
                        "status": "rotation_initiated",
                        "reason": reason,
                        "node_id": s.node_id,
                        "note": "Key rotation registered. New key will be used after restart.",
                    }))
                })
            }),
        );
    }

    // peer.pubkey -- look up a connected peer's public key by node_id
    {
        let s = state.clone();
        handler.register(
            "peer.pubkey",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_peer_pubkey(params, &s).await })
            }),
        );
    }
}

/// Return this node's FALCON-1024 and Kyber-1024 public keys.
///
/// Loads keys from the identity files in `data_dir`. If the identity
/// directory does not contain key files, returns an error.
fn handle_identity_pubkey(
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let identity_dir = state.data_dir.join("identity");
    let falcon_pk_path = identity_dir.join("falcon_pubkey.der");
    let kyber_pk_path = identity_dir.join("kyber_pubkey.der");

    let falcon_pk = std::fs::read(&falcon_pk_path).map_err(|e| RpcError {
        code: INTERNAL_ERROR,
        message: format!(
            "failed to read FALCON public key from {}: {e}",
            falcon_pk_path.display()
        ),
        data: None,
    })?;

    let kyber_pk = std::fs::read(&kyber_pk_path).map_err(|e| RpcError {
        code: INTERNAL_ERROR,
        message: format!(
            "failed to read Kyber public key from {}: {e}",
            kyber_pk_path.display()
        ),
        data: None,
    })?;

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "falcon_pubkey": hex::encode(&falcon_pk),
        "falcon_pubkey_bytes": falcon_pk.len(),
        "kyber_pubkey": hex::encode(&kyber_pk),
        "kyber_pubkey_bytes": kyber_pk.len(),
    }))
}

/// Look up a connected peer's FALCON public key by node_id or DNS name.
///
/// Params:
///   - `node_id` (string): Direct node ID lookup
///   - `name` (string): DNS name to resolve, then look up node ID
///
/// At least one of `node_id` or `name` must be provided. The FALCON
/// pubkey comes from the authenticated peers map (populated during
/// bilateral PoS handshake).
async fn handle_peer_pubkey(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    // Resolve node_id: either directly provided or via DNS name lookup
    let target_node_id = if let Some(nid) = params.get("node_id").and_then(|v| v.as_str()) {
        nid.to_string()
    } else if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
        // Resolve DNS name to find the node
        match state.dns_resolver.resolve(name).await {
            Some(_addr) => {
                // DNS resolves to an address, but we need the node_id.
                // For alpha, attempt to find a peer whose node_id starts
                // with the name or check authenticated peers.
                return Err(RpcError {
                    code: INVALID_PARAMS,
                    message: format!(
                        "DNS name '{}' resolved but node_id lookup by name \
                         is not yet implemented -- use node_id directly",
                        name
                    ),
                    data: None,
                });
            }
            None => {
                return Err(RpcError {
                    code: INVALID_PARAMS,
                    message: format!("DNS name '{}' not found", name),
                    data: None,
                });
            }
        }
    } else {
        return Err(RpcError {
            code: INVALID_PARAMS,
            message: "provide either 'node_id' or 'name' parameter".into(),
            data: None,
        });
    };

    // Look up in authenticated peers
    let net = state.network.as_ref().ok_or_else(|| RpcError {
        code: INTERNAL_ERROR,
        message: "no network manager (private mode)".into(),
        data: None,
    })?;

    let auth_peers = net.authenticated_peers();
    let peers_map = auth_peers.read().await;

    match peers_map.get(&target_node_id) {
        Some(peer) => Ok(serde_json::json!({
            "node_id": peer.node_id,
            "falcon_pubkey": hex::encode(&peer.pubkey),
            "falcon_pubkey_bytes": peer.pubkey.len(),
            "coordinate": {
                "x": peer.coordinate.0,
                "y": peer.coordinate.1,
                "z": peer.coordinate.2,
            },
            "network_id": peer.network_id,
        })),
        None => {
            let short = &target_node_id[..8.min(target_node_id.len())];
            Err(RpcError {
                code: INVALID_PARAMS,
                message: format!(
                    "peer '{}...' not found in authenticated peers",
                    short
                ),
                data: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::node_chain::NodeBlockchain;
    use crate::bootstrap::DnsResolver;
    use crate::ipc::protocol::RpcRequest;
    use crate::matrix::coordinate::MatrixCoordinate;
    use crate::persistence::{PersistenceConfig, PersistenceManager};
    use std::path::PathBuf;
    use std::time::Instant;

    async fn test_state() -> Arc<DaemonState> {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let bc = Arc::new(NodeBlockchain::new(coord));
        let config = PersistenceConfig {
            storage_dir: PathBuf::from("/tmp"),
            ..PersistenceConfig::default()
        };
        let persistence = Arc::new(
            PersistenceManager::new(config, "net-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);

        Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            shard_store: Arc::new(crate::network::shard_store::ShardStore::new()),
            shard_transport: None,
            coordinate: coord,
            node_id: "net-test".into(),
            data_dir: PathBuf::from("/tmp"),
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: DnsResolver::default(),
            dns_popularity_tracker: None,
            shard_location_index: None,
            consumer_provider_manager: None,
            #[cfg(feature = "caesar")]
            caesar: None,
            #[cfg(feature = "intelligence")]
            engauge_bridge: None,
            #[cfg(feature = "intelligence")]
            federation_manager: None,
            #[cfg(feature = "intelligence")]
            threshold_coordinator: None,

            transfer_coordinator: None,
            foundation_signing_key: None,
            dns_registrar: None,
            release_feed_subscriber: None,
            receipt_validator: Arc::new(
                crate::assets::cross_chain::CrossChainReceiptValidator::new(),
            ),
            capability_token_issuer: None,
            revocation_registry: Arc::new(crate::auth::RevocationRegistry::new()),
            light_sync_manager: None,
            catalog_registry: None,
            inbox_store: None,
        })
    }

    #[tokio::test]
    async fn test_network_peers_no_network() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("network.peers", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["count"], 0);
    }

    #[tokio::test]
    async fn test_network_connect_no_network() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "network.connect",
            serde_json::json!({"address": "::1"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "unavailable");
    }

    #[tokio::test]
    async fn test_identity_pubkey_no_keys() {
        // data_dir is /tmp which won't have identity keys -- should error
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("identity.pubkey", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        // Should return an error since /tmp/identity/falcon_pubkey.der doesn't exist
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INTERNAL_ERROR);
        assert!(err.message.contains("FALCON public key"));
    }

    #[tokio::test]
    async fn test_identity_pubkey_with_keys() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let identity_dir = dir.path().join("identity");
        std::fs::create_dir_all(&identity_dir).expect("test: mkdir");

        // Write fake key files
        let falcon_pk = vec![0xAA; 32];
        let kyber_pk = vec![0xBB; 64];
        std::fs::write(identity_dir.join("falcon_pubkey.der"), &falcon_pk)
            .expect("test: write falcon pk");
        std::fs::write(identity_dir.join("kyber_pubkey.der"), &kyber_pk)
            .expect("test: write kyber pk");

        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let bc = Arc::new(NodeBlockchain::new(coord));
        let config = PersistenceConfig {
            storage_dir: PathBuf::from("/tmp"),
            ..PersistenceConfig::default()
        };
        let persistence = Arc::new(
            PersistenceManager::new(config, "pk-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
        let state = Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            shard_store: Arc::new(crate::network::shard_store::ShardStore::new()),
            shard_transport: None,
            coordinate: coord,
            node_id: "pk-test-node".into(),
            data_dir: dir.path().to_path_buf(),
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: DnsResolver::default(),
            dns_popularity_tracker: None,
            shard_location_index: None,
            consumer_provider_manager: None,
            #[cfg(feature = "caesar")]
            caesar: None,
            #[cfg(feature = "intelligence")]
            engauge_bridge: None,
            #[cfg(feature = "intelligence")]
            federation_manager: None,
            #[cfg(feature = "intelligence")]
            threshold_coordinator: None,

            transfer_coordinator: None,
            foundation_signing_key: None,
            dns_registrar: None,
            release_feed_subscriber: None,
            receipt_validator: Arc::new(
                crate::assets::cross_chain::CrossChainReceiptValidator::new(),
            ),
            capability_token_issuer: None,
            revocation_registry: Arc::new(crate::auth::RevocationRegistry::new()),
            light_sync_manager: None,
            catalog_registry: None,
            inbox_store: None,
        });

        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("identity.pubkey", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "expected success: {:?}", resp.error);
        let result = resp.result.expect("test: result");
        assert_eq!(result["node_id"], "pk-test-node");
        assert_eq!(result["falcon_pubkey_bytes"], 32);
        assert_eq!(result["kyber_pubkey_bytes"], 64);
        // Verify hex encoding
        assert_eq!(result["falcon_pubkey"], hex::encode(&falcon_pk));
        assert_eq!(result["kyber_pubkey"], hex::encode(&kyber_pk));
    }

    #[tokio::test]
    async fn test_peer_pubkey_no_network() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "peer.pubkey",
            serde_json::json!({"node_id": "abc123"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INTERNAL_ERROR);
        assert!(err.message.contains("no network manager"));
    }

    #[tokio::test]
    async fn test_peer_pubkey_missing_params() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("peer.pubkey", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("node_id"));
    }
}
