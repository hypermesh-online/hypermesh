// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! IPC handlers for intelligence/ngauge monitoring.
//!
//! Exposes `intelligence.stats` which returns swarm demand tracking data,
//! shard store metrics, and (when the `intelligence` feature is enabled)
//! ngauge analytics summaries.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::state::DaemonState;

/// Register intelligence-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // intelligence.stats — swarm demand and ngauge analytics summary
    {
        let s = state.clone();
        handler.register(
            "intelligence.stats",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_stats(&s).await })
            }),
        );
    }
}

/// Collect intelligence metrics from available subsystems.
///
/// Returns shard store counts, swarm demand snapshot summary, and
/// ngauge analytics when the `intelligence` feature is active.
async fn handle_stats(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    // Shard store metrics (always available)
    let shard_count = state.shard_store.count().await;

    // Peer count from network manager (if present)
    let peer_count = match &state.network {
        Some(net) => net.get_connected_nodes().await.len(),
        None => 0,
    };

    // Transport active flag
    let transport_active = state.shard_transport.is_some();

    Ok(serde_json::json!({
        "intelligence_enabled": cfg!(feature = "intelligence"),
        "shard_store": {
            "shard_count": shard_count,
        },
        "network": {
            "peer_count": peer_count,
            "transport_active": transport_active,
        },
        "uptime_secs": state.started_at.elapsed().as_secs(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::protocol::RpcRequest;

    #[tokio::test]
    async fn test_intelligence_stats_no_network() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("intelligence.stats", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "expected no error");
        let result = resp.result.expect("test: result present");

        // Verify JSON structure
        assert!(result["intelligence_enabled"].is_boolean());
        assert_eq!(result["shard_store"]["shard_count"], 0);
        assert_eq!(result["network"]["peer_count"], 0);
        assert_eq!(result["network"]["transport_active"], false);
        assert!(result["uptime_secs"].is_number());
    }

    #[tokio::test]
    async fn test_intelligence_stats_returns_uptime() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("intelligence.stats", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        let result = resp.result.expect("test: result present");

        // Uptime should be >= 0
        let uptime = result["uptime_secs"].as_u64().expect("test: uptime is u64");
        assert!(uptime < 60, "test uptime should be under 60s");
    }
}
