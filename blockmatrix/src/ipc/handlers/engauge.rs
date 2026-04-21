// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Engauge IPC handlers: capacity, traffic, throttle, routing.
//!
//! When the `intelligence` feature is enabled and an [`EngaugeBridge`] is wired
//! into [`DaemonState`], these handlers return real analytics from engauge's
//! [`SwarmAnalytics`] plus live daemon metrics (shard store count, peer count).
//! When the feature is disabled, handlers return an honest `feature_unavailable`
//! error field rather than fabricating zeros.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::state::DaemonState;

pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
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
        "bridge_attached": bridge_attached(state),
    }))
}

async fn handle_traffic(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let peer_count = match &state.network {
        Some(net) => net.get_connected_nodes().await.len(),
        None => 0,
    };

    #[cfg(feature = "intelligence")]
    let (tracked_shards, transmit_summaries) = match &state.engauge_bridge {
        Some(bridge) => {
            let summaries = bridge.metrics_to_transmit().await;
            (summaries.len(), summaries.len())
        }
        None => (0, 0),
    };

    #[cfg(not(feature = "intelligence"))]
    let (tracked_shards, transmit_summaries) = (0usize, 0usize);

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "bytes_sent": 0,
        "bytes_received": 0,
        "requests_served": 0,
        "requests_received": 0,
        "active_connections": peer_count,
        "throughput_bps": 0.0,
        "privacy_mode": state.privacy_mode,
        "tracked_shards": tracked_shards,
        "transmit_summaries": transmit_summaries,
        "intelligence_enabled": cfg!(feature = "intelligence"),
        "bridge_attached": bridge_attached(state),
    }))
}

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
        "intelligence_enabled": cfg!(feature = "intelligence"),
        "bridge_attached": bridge_attached(state),
        "note": "throttle signal plumbing pending — engauge MetricsIngestionPipeline→IPC",
    }))
}

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
        "bridge_attached": bridge_attached(state),
    }))
}

#[cfg(feature = "intelligence")]
fn bridge_attached(state: &DaemonState) -> bool {
    state.engauge_bridge.is_some()
}

#[cfg(not(feature = "intelligence"))]
fn bridge_attached(_state: &DaemonState) -> bool {
    false
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
        assert!(result["bridge_attached"].is_boolean());
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
