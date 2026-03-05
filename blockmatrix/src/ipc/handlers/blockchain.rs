// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Blockchain IPC handlers: height, block, validate.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INVALID_PARAMS};
use crate::ipc::state::DaemonState;

/// Register blockchain-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // blockchain.height — current chain height
    {
        let s = state.clone();
        handler.register(
            "blockchain.height",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move {
                    let height = s.blockchain.get_height().await;
                    Ok(serde_json::json!({"height": height}))
                })
            }),
        );
    }

    // blockchain.block — fetch block by index
    {
        let s = state.clone();
        handler.register(
            "blockchain.block",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let index = params
                        .get("index")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing or invalid 'index' parameter".into(),
                            data: None,
                        })?;

                    match s.blockchain.get_block(index).await {
                        Some(block) => Ok(serde_json::json!({
                            "index": block.index,
                            "hash": block.hash,
                            "previous_hash": block.previous_hash,
                            "timestamp": block.timestamp,
                            "asset_count": block.assets.len(),
                        })),
                        None => Ok(serde_json::json!({
                            "index": index,
                            "error": "block not found",
                        })),
                    }
                })
            }),
        );
    }

    // blockchain.validate — validate chain integrity
    {
        let s = state.clone();
        handler.register(
            "blockchain.validate",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move {
                    let valid = s.blockchain.validate_chain().await;
                    let height = s.blockchain.get_height().await;
                    Ok(serde_json::json!({
                        "valid": valid,
                        "height": height,
                    }))
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
            PersistenceManager::new(config, "bc-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);

        Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            coordinate: coord,
            node_id: "bc-test".into(),
            data_dir: PathBuf::from("/tmp"),
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: DnsResolver::default(),
        })
    }

    #[tokio::test]
    async fn test_blockchain_height() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("blockchain.height", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        // Genesis block exists, height >= 0
        assert!(result["height"].is_number());
    }

    #[tokio::test]
    async fn test_blockchain_block_genesis() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "blockchain.block",
            serde_json::json!({"index": 0}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["index"], 0);
        assert!(result["hash"].is_string());
    }

    #[tokio::test]
    async fn test_blockchain_validate() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("blockchain.validate", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["valid"], true);
    }
}
