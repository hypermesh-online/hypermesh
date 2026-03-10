// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Network IPC handlers: peers, connect.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
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
}
