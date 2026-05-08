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
                            "asset_count": block.asset_count(),
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

    // chain.lookup_cross_receipt — Phase I.1.
    //
    // Query the local cross-chain receipt index for a given
    // `transfer_id` and return both source and target block hashes
    // when present. An auditor uses this to prove transfer atomicity
    // from either side without consulting the other chain.
    //
    // Params: `{ "transfer_id": "<string>" }`
    // Result on hit: `{ "found": true, "transfer_id", "source_chain_id",
    //                   "source_block_hash", "target_chain_id",
    //                   "target_block_hash", "completed_at",
    //                   "asset_id" }`
    // Result on miss: `{ "found": false, "transfer_id": "<input>" }`
    {
        let s = state.clone();
        handler.register(
            "chain.lookup_cross_receipt",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let transfer_id = params
                        .get("transfer_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing or invalid 'transfer_id' parameter".into(),
                            data: None,
                        })?;

                    match s.receipt_validator.get_by_transfer_id(transfer_id).await {
                        Some(rcpt) => Ok(serde_json::json!({
                            "found": true,
                            "transfer_id": rcpt.transfer_id,
                            "source_chain_id": rcpt.source_chain_id,
                            "source_block_hash": rcpt.source_block_hash,
                            "target_chain_id": rcpt.target_chain_id,
                            "target_block_hash": rcpt.target_block_hash,
                            "completed_at": rcpt.completed_at,
                            "asset_id": rcpt.asset_id,
                        })),
                        None => Ok(serde_json::json!({
                            "found": false,
                            "transfer_id": transfer_id,
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
            PersistenceManager::new(config, "bc-test".into())
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
            node_id: "bc-test".into(),
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
            receipt_validator: Arc::new(
                crate::assets::cross_chain::CrossChainReceiptValidator::new(),
            ),
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

    #[tokio::test]
    async fn test_lookup_cross_receipt_missing() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "chain.lookup_cross_receipt",
            serde_json::json!({ "transfer_id": "tx-nonexistent" }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["found"], false);
        assert_eq!(result["transfer_id"], "tx-nonexistent");
    }

    #[tokio::test]
    async fn test_lookup_cross_receipt_found() {
        use crate::gateway::asset_transfer::TransferReceipt;
        use hypermesh_lib::BlockchainScope;

        let state = test_state().await;
        // Inject a receipt directly into the validator (simulates
        // a coordinator having written one to the chain).
        let receipt = TransferReceipt {
            transfer_id: "tx-i1-found".to_string(),
            source_chain_id: "chain-A".to_string(),
            target_chain_id: "chain-B".to_string(),
            source_block_hash: "srcHash".to_string(),
            target_block_hash: "tgtHash".to_string(),
            completed_at: 1_700_000_000,
            asset_id: "asset-i1".to_string(),
            source_scope: BlockchainScope::Device,
            target_scope: BlockchainScope::Network,
        };
        state.receipt_validator.insert(receipt).await;

        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "chain.lookup_cross_receipt",
            serde_json::json!({ "transfer_id": "tx-i1-found" }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["found"], true);
        assert_eq!(result["source_chain_id"], "chain-A");
        assert_eq!(result["target_chain_id"], "chain-B");
        assert_eq!(result["source_block_hash"], "srcHash");
        assert_eq!(result["target_block_hash"], "tgtHash");
    }
}
