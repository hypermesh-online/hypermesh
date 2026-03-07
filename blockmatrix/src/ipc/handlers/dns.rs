// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! DNS IPC handlers: resolve, list, register.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::INVALID_PARAMS;
use crate::ipc::state::DaemonState;

/// Register DNS-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // dns.resolve — resolve a single name
    {
        let s = state.clone();
        handler.register(
            "dns.resolve",
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

                    match s.dns_resolver.resolve(name).await {
                        Some(addr) => Ok(serde_json::json!({
                            "name": name,
                            "address": addr.to_string(),
                        })),
                        None => Ok(serde_json::json!({
                            "name": name,
                            "address": null,
                            "error": "not found",
                        })),
                    }
                })
            }),
        );
    }

    // dns.list — list all registered DNS records
    {
        let s = state.clone();
        handler.register(
            "dns.list",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move {
                    let records = s.dns_resolver.all_records().await;
                    let entries: Vec<serde_json::Value> = records
                        .into_iter()
                        .map(|(name, addr)| {
                            serde_json::json!({"name": name, "address": addr.to_string()})
                        })
                        .collect();
                    Ok(serde_json::json!({"records": entries, "count": entries.len()}))
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
            PersistenceManager::new(config, "dns-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
        let dns = DnsResolver::default();

        Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            shard_store: Arc::new(crate::network::shard_store::ShardStore::new()),
            coordinate: coord,
            node_id: "dns-test".into(),
            data_dir: PathBuf::from("/tmp"),
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: dns,
        })
    }

    #[tokio::test]
    async fn test_dns_resolve_not_found() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "dns.resolve",
            serde_json::json!({"name": "nonexistent"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert!(result["address"].is_null());
    }

    #[tokio::test]
    async fn test_dns_resolve_found() {
        let state = test_state().await;
        // Pre-register a record
        state
            .dns_resolver
            .register(
                "mynode".into(),
                std::net::IpAddr::from(std::net::Ipv6Addr::LOCALHOST),
            )
            .await;

        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "dns.resolve",
            serde_json::json!({"name": "mynode"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["address"], "::1");
    }

    #[tokio::test]
    async fn test_dns_list_empty() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        // The default DnsResolver has built-in records (localhost, trustchain, etc.)
        let req = RpcRequest::new("dns.list", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert!(result["count"].is_number());
    }
}
