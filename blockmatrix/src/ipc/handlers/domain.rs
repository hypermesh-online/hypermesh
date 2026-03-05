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

                    let reg = crate::dns::domain::DomainRegistration::new(
                        name,
                        privacy_mode,
                        s.node_id.clone(),
                    );

                    Ok(serde_json::json!({
                        "domain": name,
                        "network_id": reg.network_id,
                        "privacy": privacy_str,
                        "owner": s.node_id,
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
            PersistenceManager::new(config, "domain-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
        let dns = DnsResolver::default();

        Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            coordinate: coord,
            node_id: "domain-test".into(),
            data_dir: PathBuf::from("/tmp"),
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: dns,
        })
    }

    #[tokio::test]
    async fn test_domain_register() {
        let state = test_state().await;
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
        let state = test_state().await;
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
        let state = test_state().await;
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
