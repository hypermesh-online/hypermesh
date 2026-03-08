// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Asset IPC handlers: info, list.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INVALID_PARAMS};
use crate::ipc::state::DaemonState;

/// Register asset-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // asset.list — list registered assets from the blockchain
    {
        let s = state.clone();
        handler.register(
            "asset.list",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move {
                    let height = s.blockchain.get_height().await;
                    let mut assets: Vec<serde_json::Value> = Vec::new();

                    for idx in 0..=height {
                        if let Some(block) = s.blockchain.get_block(idx).await {
                            for reg in block.get_assets() {
                                let hash_hex = hex::encode(reg.content_hash);
                                assets.push(serde_json::json!({
                                    "block_index": idx,
                                    "content_hash": hash_hex,
                                    "category": format!("{:?}", reg.category),
                                    "scope": format!("{:?}", reg.network_scope),
                                }));
                            }
                        }
                    }

                    Ok(serde_json::json!({
                        "count": assets.len(),
                        "assets": assets,
                    }))
                })
            }),
        );
    }

    // asset.info — get info about a specific asset by content hash prefix
    {
        let s = state.clone();
        handler.register(
            "asset.info",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let asset_id = params
                        .get("asset_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing 'asset_id' parameter".into(),
                            data: None,
                        })?
                        .to_string();

                    let height = s.blockchain.get_height().await;
                    let mut matches: Vec<serde_json::Value> = Vec::new();

                    for idx in 0..=height {
                        if let Some(block) = s.blockchain.get_block(idx).await {
                            for reg in block.get_assets() {
                                let hash_hex = hex::encode(reg.content_hash);
                                if hash_hex.contains(&asset_id) {
                                    matches.push(serde_json::json!({
                                        "block_index": idx,
                                        "block_hash": &block.hash,
                                        "content_hash": hash_hex,
                                        "category": format!("{:?}", reg.category),
                                        "scope": format!("{:?}", reg.network_scope),
                                    }));
                                }
                            }
                        }
                    }

                    if matches.is_empty() {
                        Ok(serde_json::json!({
                            "asset_id": asset_id,
                            "found": false,
                        }))
                    } else {
                        Ok(serde_json::json!({
                            "asset_id": asset_id,
                            "found": true,
                            "entries": matches,
                        }))
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
            PersistenceManager::new(config, "asset-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);

        Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            shard_store: Arc::new(crate::network::shard_store::ShardStore::new()),
            coordinate: coord,
            node_id: "asset-test".into(),
            data_dir: PathBuf::from("/tmp"),
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: DnsResolver::default(),
        })
    }

    #[tokio::test]
    async fn test_asset_list() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("asset.list", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert!(result["count"].is_number());
    }

    #[tokio::test]
    async fn test_asset_info_not_found() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "asset.info",
            serde_json::json!({"asset_id": "nonexistent-id"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["found"], false);
    }

    #[tokio::test]
    async fn test_asset_info_missing_param() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("asset.info", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error present");
        assert_eq!(err.code, INVALID_PARAMS);
    }
}
