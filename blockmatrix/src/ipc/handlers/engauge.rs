// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Engauge IPC handlers: capacity, traffic, throttle, routing.
//!
//! Exposes engauge analytics through the daemon IPC. Real data is returned from
//! [`DaemonState`] where available (shard count, peer count, uptime). Engauge-
//! specific metrics (streaming frames, differential privacy, marketplace) return
//! structured zeros until the full ingestion pipeline is wired to IPC.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::state::DaemonState;

/// Register engauge-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // engauge.capacity -- node capacity metrics
    {
        let s = state.clone();
        handler.register(
            "engauge.capacity",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_capacity(&s).await })
            }),
        );
    }

    // engauge.traffic -- traffic and throughput metrics
    {
        let s = state.clone();
        handler.register(
            "engauge.traffic",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_traffic(&s).await })
            }),
        );
    }

    // engauge.throttle -- throttle and rate-limit status
    {
        let s = state.clone();
        handler.register(
            "engauge.throttle",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_throttle(&s).await })
            }),
        );
    }

    // engauge.routing -- routing intelligence summary
    {
        let s = state.clone();
        handler.register(
            "engauge.routing",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_routing(&s).await })
            }),
        );
    }
}

/// Node capacity: shard count, peer count, storage utilization.
async fn handle_capacity(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let shard_count = state.shard_store.count().await;
    let peer_count = match &state.network {
        Some(net) => net.get_connected_nodes().await.len(),
        None => 0,
    };
    let uptime = state.started_at.elapsed().as_secs();

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "shards_stored": shard_count,
        "peers_connected": peer_count,
        "storage_used_bytes": 0,
        "storage_capacity_bytes": 0,
        "cpu_utilization": 0.0,
        "memory_utilization": 0.0,
        "uptime_secs": uptime,
        "intelligence_enabled": cfg!(feature = "intelligence"),
    }))
}

/// Traffic metrics: bytes in/out, request counts, throughput.
async fn handle_traffic(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let peer_count = match &state.network {
        Some(net) => net.get_connected_nodes().await.len(),
        None => 0,
    };

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "bytes_sent": 0,
        "bytes_received": 0,
        "requests_served": 0,
        "requests_received": 0,
        "active_connections": peer_count,
        "throughput_bps": 0.0,
        "privacy_mode": state.privacy_mode,
    }))
}

/// Throttle status: current rate limits, backpressure signals.
async fn handle_throttle(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    Ok(serde_json::json!({
        "node_id": state.node_id,
        "throttled": false,
        "current_rate_limit_rps": 0,
        "backpressure_active": false,
        "queue_depth": 0,
        "dropped_requests": 0,
        "privacy_mode": state.privacy_mode,
    }))
}

/// Routing intelligence: path recommendations, weight modifiers.
async fn handle_routing(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let peer_count = match &state.network {
        Some(net) => net.get_connected_nodes().await.len(),
        None => 0,
    };

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "coordinate": {
            "x": state.coordinate.x,
            "y": state.coordinate.y,
            "z": state.coordinate.z,
        },
        "known_peers": peer_count,
        "routing_table_size": 0,
        "preferred_paths": [],
        "weight_modifiers": {},
        "intelligence_enabled": cfg!(feature = "intelligence"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::protocol::RpcRequest;

    #[tokio::test]
    async fn test_engauge_capacity_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("engauge.capacity", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["shards_stored"], 0);
        assert_eq!(result["peers_connected"], 0);
        assert!(result["uptime_secs"].is_number());
        assert!(result["intelligence_enabled"].is_boolean());
    }

    #[tokio::test]
    async fn test_engauge_traffic_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("engauge.traffic", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["bytes_sent"], 0);
        assert_eq!(result["bytes_received"], 0);
        assert_eq!(result["active_connections"], 0);
        assert!(result["privacy_mode"].is_string());
    }

    #[tokio::test]
    async fn test_engauge_throttle_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("engauge.throttle", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["throttled"], false);
        assert_eq!(result["backpressure_active"], false);
        assert_eq!(result["queue_depth"], 0);
    }

    #[tokio::test]
    async fn test_engauge_routing_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("engauge.routing", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["known_peers"], 0);
        assert!(result["coordinate"].is_object());
        assert!(result["preferred_paths"].is_array());
        assert!(result["intelligence_enabled"].is_boolean());
    }
}
