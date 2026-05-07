// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Asset IPC handlers: info, list, register.

use std::sync::Arc;

use crate::assets::core::asset_id::{
    ApplicationDomain, AssetCategory, AssetData, BaseSystemType, NetworkScope,
};
use crate::assets::core::AssetRegistration;
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INTERNAL_ERROR, INVALID_PARAMS};
use crate::ipc::state::DaemonState;
use trustchain::proof_of_state::StateProof;

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

    // asset.register — register a new asset on the blockchain
    {
        let s = state.clone();
        handler.register(
            "asset.register",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_asset_register(params, &s).await })
            }),
        );
    }
}

/// Parse a system type name (case-insensitive) into a `BaseSystemType`.
fn parse_system_type(name: &str) -> Result<BaseSystemType, RpcError> {
    match name.to_lowercase().as_str() {
        "cpu" => Ok(BaseSystemType::Cpu),
        "gpu" => Ok(BaseSystemType::Gpu),
        "memory" => Ok(BaseSystemType::Memory),
        "storage" => Ok(BaseSystemType::Storage),
        "network" => Ok(BaseSystemType::Network),
        "container" => Ok(BaseSystemType::Container),
        "economic" => Ok(BaseSystemType::Economic),
        "blockchain" => Ok(BaseSystemType::Blockchain),
        "dns" => Ok(BaseSystemType::Dns),
        "transmission" => Ok(BaseSystemType::Transmission),
        "dashboard" => Ok(BaseSystemType::Dashboard),
        "identity" => Ok(BaseSystemType::Identity),
        "keyrotation" | "key_rotation" => Ok(BaseSystemType::KeyRotation),
        "invitation" => Ok(BaseSystemType::Invitation),
        "message" => Ok(BaseSystemType::Message),
        other => Err(RpcError {
            code: INVALID_PARAMS,
            message: format!(
                "unknown system type '{other}', use 'application' category for custom types"
            ),
            data: None,
        }),
    }
}

/// Handle the `asset.register` IPC call.
///
/// Required params: `category` ("system"|"application"), `content` (hex-encoded bytes).
/// Optional params: `type_name`, `type_hash` (hex), `metadata` (JSON object).
async fn handle_asset_register(
    params: serde_json::Value,
    state: &DaemonState,
) -> Result<serde_json::Value, RpcError> {
    let category = params
        .get("category")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "category required (system|application)".into(),
            data: None,
        })?;

    let content = params
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError {
            code: INVALID_PARAMS,
            message: "content required (hex-encoded asset data)".into(),
            data: None,
        })?;

    let type_name = params
        .get("type_name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let type_hash = params
        .get("type_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let metadata_val = params
        .get("metadata")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let content_bytes = hex::decode(content).map_err(|e| RpcError {
        code: INVALID_PARAMS,
        message: format!("invalid hex content: {e}"),
        data: None,
    })?;

    let asset_data = AssetData {
        config: type_name.as_bytes().to_vec(),
        definition: content_bytes.clone(),
        metadata: serde_json::to_vec(&metadata_val).unwrap_or_default(),
    };

    let asset_category = match category {
        "system" => {
            let base_type = parse_system_type(type_name)?;
            AssetCategory::BaseSystem(base_type)
        }
        "application" => {
            let domain_hash = if type_hash.is_empty() {
                *blake3::hash(type_name.as_bytes()).as_bytes()
            } else {
                let bytes = hex::decode(type_hash).map_err(|e| RpcError {
                    code: INVALID_PARAMS,
                    message: format!("invalid type_hash hex: {e}"),
                    data: None,
                })?;
                let mut h = [0u8; 32];
                let len = bytes.len().min(32);
                h[..len].copy_from_slice(&bytes[..len]);
                h
            };
            AssetCategory::Application(ApplicationDomain {
                domain_name: type_name.to_string(),
                domain_hash,
            })
        }
        other => {
            return Err(RpcError {
                code: INVALID_PARAMS,
                message: format!(
                    "unknown category '{other}', expected 'system' or 'application'"
                ),
                data: None,
            });
        }
    };

    let registration = AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        asset_category,
    );

    let content_hash_hex = hex::encode(registration.content_hash);

    // Use a minimal testing-grade state proof for alpha.
    // Production will require real PoS from the caller.
    let state_proof = StateProof::new_for_testing();

    match state
        .blockchain
        .register_asset_record(registration, &state_proof)
        .await
    {
        Ok(block) => Ok(serde_json::json!({
            "asset_id": content_hash_hex,
            "block_index": block.index,
            "status": "registered",
        })),
        Err(e) => Err(RpcError {
            code: INTERNAL_ERROR,
            message: format!("blockchain registration failed: {e}"),
            data: None,
        }),
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
            shard_transport: None,
            coordinate: coord,
            node_id: "asset-test".into(),
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

    #[tokio::test]
    async fn test_asset_register_application_type() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "asset.register",
            serde_json::json!({
                "category": "application",
                "type_name": "Message",
                "content": hex::encode(b"hello world"),
                "metadata": {"version": "1.0"},
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "registered");
        assert!(result["asset_id"].is_string());
        assert_eq!(result["block_index"], 1);
    }

    #[tokio::test]
    async fn test_asset_register_system_type() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "asset.register",
            serde_json::json!({
                "category": "system",
                "type_name": "Dns",
                "content": hex::encode(b"dns-record-data"),
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "registered");
        assert!(result["block_index"].is_number());
    }

    #[tokio::test]
    async fn test_asset_register_missing_category() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "asset.register",
            serde_json::json!({"content": "aabb"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error present");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("category"));
    }

    #[tokio::test]
    async fn test_asset_register_missing_content() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "asset.register",
            serde_json::json!({"category": "application"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error present");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("content"));
    }

    #[tokio::test]
    async fn test_asset_register_invalid_category() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "asset.register",
            serde_json::json!({
                "category": "bogus",
                "content": "aabb",
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error present");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("bogus"));
    }

    #[test]
    fn test_parse_system_type() {
        assert!(matches!(parse_system_type("dns"), Ok(BaseSystemType::Dns)));
        assert!(matches!(parse_system_type("Cpu"), Ok(BaseSystemType::Cpu)));
        assert!(matches!(parse_system_type("GPU"), Ok(BaseSystemType::Gpu)));
        assert!(matches!(
            parse_system_type("Dashboard"),
            Ok(BaseSystemType::Dashboard)
        ));
        assert!(matches!(
            parse_system_type("identity"),
            Ok(BaseSystemType::Identity)
        ));
        assert!(matches!(
            parse_system_type("key_rotation"),
            Ok(BaseSystemType::KeyRotation)
        ));
        assert!(matches!(
            parse_system_type("invitation"),
            Ok(BaseSystemType::Invitation)
        ));
        assert!(matches!(
            parse_system_type("transmission"),
            Ok(BaseSystemType::Transmission)
        ));
        assert!(parse_system_type("nonexistent").is_err());
    }
}
