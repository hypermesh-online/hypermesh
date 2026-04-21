// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Gateway IPC handlers: transfer, status, list.
//!
//! Exposes cross-scope asset transfer operations over JSON-RPC 2.0.

use std::sync::Arc;

use hypermesh_lib::BlockchainScope;

use crate::gateway::GatewayManager;
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INTERNAL_ERROR, INVALID_PARAMS};
use crate::ipc::state::DaemonState;

/// Parse a scope string ("device" or "network") into a `BlockchainScope`.
fn parse_scope(s: &str) -> Result<BlockchainScope, RpcError> {
    match s.to_lowercase().as_str() {
        "device" => Ok(BlockchainScope::Device),
        "network" => Ok(BlockchainScope::Network),
        other => Err(RpcError {
            code: INVALID_PARAMS,
            message: format!(
                "invalid scope '{}': expected 'device' or 'network'",
                other
            ),
            data: None,
        }),
    }
}

/// Register gateway-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // GatewayManager is now wired to the node's blockchain so that
    // transfer lock/confirm/release entries are written as blocks.

    // gateway.transfer -- initiate a cross-scope asset transfer
    {
        let blockchain = state.blockchain.clone();
        handler.register(
            "gateway.transfer",
            Arc::new(move |params| {
                let bc = blockchain.clone();
                Box::pin(async move {
                    let asset_id_str = params
                        .get("asset_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing or invalid 'asset_id' parameter".into(),
                            data: None,
                        })?;

                    let source_scope = params
                        .get("source_scope")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing or invalid 'source_scope' parameter".into(),
                            data: None,
                        })?;

                    let target_scope = params
                        .get("target_scope")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing or invalid 'target_scope' parameter".into(),
                            data: None,
                        })?;

                    let from = parse_scope(source_scope)?;
                    let to = parse_scope(target_scope)?;
                    let asset_id = hypermesh_lib::AssetId::from(asset_id_str);

                    let gw = GatewayManager::with_blockchain(bc);
                    match gw.transfer_asset(asset_id, from, to).await {
                        Ok(transfer_id) => Ok(serde_json::json!({
                            "transfer_id": transfer_id,
                            "status": "pending",
                        })),
                        Err(e) => Err(RpcError {
                            code: INTERNAL_ERROR,
                            message: format!("transfer failed: {e}"),
                            data: None,
                        }),
                    }
                })
            }),
        );
    }

    // gateway.status -- get the status of a transfer by ID
    {
        handler.register(
            "gateway.status",
            Arc::new(move |params| {
                Box::pin(async move {
                    let transfer_id = params
                        .get("transfer_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing or invalid 'transfer_id' parameter".into(),
                            data: None,
                        })?;

                    // With a per-request GatewayManager we cannot look up
                    // transfers initiated by a previous request. Return a
                    // structured "not found" response rather than an RPC
                    // error so the caller can distinguish missing from broken.
                    //
                    // When GatewayManager is promoted to DaemonState this
                    // will return real transfer data.
                    Ok(serde_json::json!({
                        "transfer_id": transfer_id,
                        "status": "not_found",
                        "message": "transfer not found (gateway state is ephemeral in alpha)",
                    }))
                })
            }),
        );
    }

    // gateway.list -- list all transfers
    {
        handler.register(
            "gateway.list",
            Arc::new(move |_params| {
                Box::pin(async move {
                    // Ephemeral gateway -- no persistent transfers to list.
                    // Returns empty array. Will be populated once
                    // GatewayManager lives in DaemonState.
                    Ok(serde_json::json!({
                        "transfers": [],
                        "count": 0,
                        "message": "gateway state is ephemeral in alpha",
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
    use crate::network::shard_store::ShardStore;
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
            PersistenceManager::new(config, "gw-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);

        Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            shard_store: Arc::new(ShardStore::new()),
            shard_transport: None,
            coordinate: coord,
            node_id: "gw-test".into(),
            data_dir: PathBuf::from("/tmp"),
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: DnsResolver::default(),
            dns_popularity_tracker: None,
            #[cfg(feature = "caesar")]
            caesar: None,
            #[cfg(feature = "intelligence")]
            engauge_bridge: None,
        })
    }

    #[test]
    fn test_parse_scope_device() {
        let scope = parse_scope("device").expect("test: parse device");
        assert_eq!(scope, BlockchainScope::Device);
    }

    #[test]
    fn test_parse_scope_network() {
        let scope = parse_scope("network").expect("test: parse network");
        assert_eq!(scope, BlockchainScope::Network);
    }

    #[test]
    fn test_parse_scope_case_insensitive() {
        let scope = parse_scope("Device").expect("test: parse Device");
        assert_eq!(scope, BlockchainScope::Device);
        let scope = parse_scope("NETWORK").expect("test: parse NETWORK");
        assert_eq!(scope, BlockchainScope::Network);
    }

    #[test]
    fn test_parse_scope_invalid() {
        let err = parse_scope("invalid").unwrap_err();
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("invalid scope"));
    }

    #[test]
    fn test_parse_transfer_params() {
        let params = serde_json::json!({
            "asset_id": "test-asset-123",
            "source_scope": "device",
            "target_scope": "network"
        });
        assert_eq!(
            params["asset_id"].as_str().expect("test: asset_id"),
            "test-asset-123"
        );
        assert_eq!(
            params["source_scope"].as_str().expect("test: source_scope"),
            "device"
        );
        assert_eq!(
            params["target_scope"].as_str().expect("test: target_scope"),
            "network"
        );
    }

    #[test]
    fn test_parse_status_params() {
        let params = serde_json::json!({
            "transfer_id": "gw-tx-1"
        });
        assert_eq!(
            params["transfer_id"].as_str().expect("test: transfer_id"),
            "gw-tx-1"
        );
    }

    #[tokio::test]
    async fn test_gateway_transfer_handler() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "gateway.transfer",
            serde_json::json!({
                "asset_id": "cpu-001",
                "source_scope": "device",
                "target_scope": "network"
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "expected success, got: {:?}", resp.error);
        let result = resp.result.expect("test: result present");
        assert!(result["transfer_id"].as_str().expect("test: transfer_id").starts_with("gw-tx-"));
        assert_eq!(result["status"], "pending");
    }

    #[tokio::test]
    async fn test_gateway_transfer_same_scope_error() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "gateway.transfer",
            serde_json::json!({
                "asset_id": "asset-x",
                "source_scope": "device",
                "target_scope": "device"
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some(), "same-scope should fail");
        let err = resp.error.expect("test: error");
        assert!(err.message.contains("transfer failed"));
    }

    #[tokio::test]
    async fn test_gateway_transfer_missing_params() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        // Missing asset_id
        let req = RpcRequest::new(
            "gateway.transfer",
            serde_json::json!({"source_scope": "device", "target_scope": "network"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        assert!(
            resp.error
                .expect("test: error")
                .message
                .contains("asset_id")
        );
    }

    #[tokio::test]
    async fn test_gateway_transfer_invalid_scope() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "gateway.transfer",
            serde_json::json!({
                "asset_id": "asset-1",
                "source_scope": "bogus",
                "target_scope": "network"
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        assert!(
            resp.error
                .expect("test: error")
                .message
                .contains("invalid scope")
        );
    }

    #[tokio::test]
    async fn test_gateway_status_handler() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "gateway.status",
            serde_json::json!({"transfer_id": "gw-tx-1"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["transfer_id"], "gw-tx-1");
    }

    #[tokio::test]
    async fn test_gateway_status_missing_param() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("gateway.status", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
    }

    #[tokio::test]
    async fn test_gateway_list_handler() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("gateway.list", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["count"], 0);
        assert!(result["transfers"].is_array());
    }
}
