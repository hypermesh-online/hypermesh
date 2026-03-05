// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Dashboard IPC handlers: list, info.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INVALID_PARAMS};
use crate::ipc::state::DaemonState;

/// Register dashboard-related IPC methods.
pub fn register(handler: &mut RequestHandler, _state: &Arc<DaemonState>) {
    // dashboard.list -- returns registered dashboards (placeholder)
    handler.register(
        "dashboard.list",
        Arc::new(|_params| {
            Box::pin(async {
                Ok(serde_json::json!({
                    "count": 0,
                    "dashboards": [],
                }))
            })
        }),
    );

    // dashboard.info -- lookup a dashboard by name
    handler.register(
        "dashboard.info",
        Arc::new(|params| {
            Box::pin(async move {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError {
                        code: INVALID_PARAMS,
                        message: "missing 'name' parameter".into(),
                        data: None,
                    })?;

                Ok(serde_json::json!({
                    "name": name,
                    "found": false,
                }))
            })
        }),
    );
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
            PersistenceManager::new(config, "dash-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);

        Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            coordinate: coord,
            node_id: "dash-test".into(),
            data_dir: PathBuf::from("/tmp"),
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: DnsResolver::default(),
        })
    }

    #[tokio::test]
    async fn test_dashboard_list_empty() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("dashboard.list", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["count"], 0);
    }

    #[tokio::test]
    async fn test_dashboard_info_not_found() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "dashboard.info",
            serde_json::json!({"name": "nonexistent"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["found"], false);
    }

    #[tokio::test]
    async fn test_dashboard_info_missing_param() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("dashboard.info", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
    }
}
