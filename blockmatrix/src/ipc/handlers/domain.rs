// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Domain IPC handlers: register, list, join.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::INVALID_PARAMS;
use crate::ipc::state::DaemonState;

/// Register domain-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // domain.register — register a domain on the local blockchain
    {
        let s = state.clone();
        handler.register(
            "domain.register",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let name = params
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| crate::ipc::protocol::RpcError {
                            code: INVALID_PARAMS,
                            message: "missing 'name' parameter".into(),
                            data: None,
                        })?;

                    let privacy_str = params
                        .get("privacy")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Private");

                    let privacy_mode = match privacy_str {
                        "Public" => crate::bootstrap::PrivacyMode::PUBLIC,
                        "Anonymous" => crate::bootstrap::PrivacyMode::ANONYMOUS,
                        _ => crate::bootstrap::PrivacyMode::PRIVATE,
                    };

                    // Load existing domains and check for duplicate
                    let domains_path = s
                        .data_dir
                        .join(&s.node_id)
                        .join("domain_registrations.json");
                    let mut domains =
                        crate::dns::domain::load_domains(&domains_path).unwrap_or_default();

                    if domains.iter().any(|d| d.domain_name == name) {
                        return Err(crate::ipc::protocol::RpcError {
                            code: INVALID_PARAMS,
                            message: format!("domain '{}' already registered", name),
                            data: None,
                        });
                    }

                    let reg = crate::dns::domain::DomainRegistration::new(
                        name,
                        privacy_mode,
                        s.node_id.clone(),
                    );

                    // Register as blockchain asset (DNS-as-asset, R10)
                    let dns_data_str = format!("DOMAIN:REGISTER:{name}");
                    let asset_data = crate::assets::core::AssetData {
                        config: dns_data_str.as_bytes().to_vec(),
                        definition: format!("domain-registration:{name}").into_bytes(),
                        metadata: format!(
                            "network_id={},privacy={privacy_str}",
                            reg.network_id
                        )
                        .into_bytes(),
                    };
                    let registration =
                        crate::assets::core::AssetRegistration::from_asset_data(
                            &asset_data,
                            crate::assets::core::NetworkScope::Global,
                            crate::assets::core::AssetCategory::BaseSystem(
                                crate::assets::core::BaseSystemType::Dns,
                            ),
                        );
                    let state_proof =
                        trustchain::proof_of_state::StateProof::generate_from_network(&s.node_id)
                            .await
                            .map_err(|e| crate::ipc::protocol::RpcError {
                                code: -32603,
                                message: format!("PoS proof generation failed: {e}"),
                                data: None,
                            })?;

                    let block = s
                        .blockchain
                        .register_asset_record(registration, &state_proof)
                        .await
                        .map_err(|e| crate::ipc::protocol::RpcError {
                            code: -32603,
                            message: format!("blockchain write failed: {e}"),
                            data: None,
                        })?;

                    // Persist to disk
                    domains.push(reg.clone());
                    if let Some(parent) = domains_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) =
                        crate::dns::domain::save_domains(&domains, &domains_path)
                    {
                        tracing::warn!("Failed to persist domain registration: {e}");
                    }

                    Ok(serde_json::json!({
                        "domain": name,
                        "network_id": reg.network_id,
                        "privacy": privacy_str,
                        "owner": s.node_id,
                        "block": block.index,
                        "status": "registered",
                    }))
                })
            }),
        );
    }

    // domain.list — list domain registrations known to this node
    {
        let s = state.clone();
        handler.register(
            "domain.list",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move {
                    // Load from disk
                    let path = s
                        .data_dir
                        .join(&s.node_id)
                        .join("domain_registrations.json");
                    let domains = if path.exists() {
                        crate::dns::domain::load_domains(&path).unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    let entries: Vec<serde_json::Value> = domains
                        .iter()
                        .map(|d| {
                            serde_json::json!({
                                "domain": d.domain_name,
                                "network_id": d.network_id,
                                "owner": d.owner_node_id,
                                "privacy": format!("{:?}", d.privacy_mode),
                            })
                        })
                        .collect();

                    Ok(serde_json::json!({
                        "count": entries.len(),
                        "domains": entries,
                    }))
                })
            }),
        );
    }

    // domain.join — join a domain network
    {
        let s = state.clone();
        handler.register(
            "domain.join",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let domain = params
                        .get("domain")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| crate::ipc::protocol::RpcError {
                            code: INVALID_PARAMS,
                            message: "missing 'domain' parameter".into(),
                            data: None,
                        })?;

                    let network_id = crate::dns::domain::derive_network_id(domain);

                    Ok(serde_json::json!({
                        "domain": domain,
                        "network_id": network_id,
                        "node_id": s.node_id,
                        "status": "join_recorded",
                        "note": "connect daemon to sync with network peers",
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
    use std::time::Instant;

    async fn test_state() -> (Arc<DaemonState>, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("test: create tempdir");
        let data_dir = tmp.path().to_path_buf();
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let bc = Arc::new(NodeBlockchain::new(coord));
        let config = PersistenceConfig {
            storage_dir: data_dir.clone(),
            ..PersistenceConfig::default()
        };
        let persistence = Arc::new(
            PersistenceManager::new(config, "domain-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
        let dns = DnsResolver::default();

        let state = Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            shard_store: Arc::new(crate::network::shard_store::ShardStore::new()),
            shard_transport: None,
            coordinate: coord,
            node_id: "domain-test".into(),
            data_dir,
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: dns,
            dns_popularity_tracker: None,
        });
        (state, tmp)
    }

    #[tokio::test]
    async fn test_domain_register() {
        let (state, _tmp) = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "domain.register",
            serde_json::json!({"name": "test.hypermesh", "privacy": "Private"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["domain"], "test.hypermesh");
        assert_eq!(result["status"], "registered");
    }

    #[tokio::test]
    async fn test_domain_list_empty() {
        let (state, _tmp) = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("domain.list", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["count"], 0);
    }

    #[tokio::test]
    async fn test_domain_join() {
        let (state, _tmp) = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "domain.join",
            serde_json::json!({"domain": "my.domain"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["domain"], "my.domain");
        assert_eq!(result["status"], "join_recorded");
    }
}
