// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! STOQ transport IPC handlers: stats, connections, performance.
//!
//! Exposes STOQ transport state through the daemon IPC. Connection data comes
//! from [`DaemonState::network`] which wraps the STOQ transport layer.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::state::DaemonState;

/// Register STOQ-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // stoq.stats -- connection count, transport status
    {
        let s = state.clone();
        handler.register(
            "stoq.stats",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_stats(&s).await })
            }),
        );
    }

    // stoq.connections -- list active QUIC connections
    {
        let s = state.clone();
        handler.register(
            "stoq.connections",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_connections(&s).await })
            }),
        );
    }

    // stoq.performance -- latency and throughput per connection
    {
        let s = state.clone();
        handler.register(
            "stoq.performance",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move { handle_performance(&s).await })
            }),
        );
    }
}

/// Transport stats: connection count, bytes sent/received, transport status.
async fn handle_stats(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let (peer_count, peers) = match &state.network {
        Some(net) => {
            let nodes = net.get_connected_nodes().await;
            (nodes.len(), nodes)
        }
        None => (0, vec![]),
    };

    let transport_active = state.shard_transport.is_some();
    let uptime = state.started_at.elapsed().as_secs();

    // Derive unique addresses from connected peers
    let unique_addresses: std::collections::HashSet<String> =
        peers.iter().map(|p| p.address.to_string()).collect();

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "connections": peer_count,
        "unique_endpoints": unique_addresses.len(),
        "transport_active": transport_active,
        "shard_transport_active": state.shard_transport.is_some(),
        "bytes_sent": 0,
        "bytes_received": 0,
        "protocol": "QUIC",
        "privacy_mode": state.privacy_mode,
        "uptime_secs": uptime,
    }))
}

/// List active QUIC connections with peer details.
async fn handle_connections(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    match &state.network {
        Some(net) => {
            let nodes = net.get_connected_nodes().await;
            let connections: Vec<serde_json::Value> = nodes
                .iter()
                .map(|n| {
                    serde_json::json!({
                        "node_id": n.node_id,
                        "address": n.address.to_string(),
                        "coordinate": {
                            "x": n.coordinate.x,
                            "y": n.coordinate.y,
                            "z": n.coordinate.z,
                        },
                        "protocol": "QUIC",
                    })
                })
                .collect();

            Ok(serde_json::json!({
                "count": connections.len(),
                "connections": connections,
            }))
        }
        None => Ok(serde_json::json!({
            "count": 0,
            "connections": [],
            "note": "no network manager (private mode)",
        })),
    }
}

/// Performance metrics: latency, throughput estimates per connection.
async fn handle_performance(
    state: &DaemonState,
) -> Result<serde_json::Value, crate::ipc::protocol::RpcError> {
    let peer_count = match &state.network {
        Some(net) => net.get_connected_nodes().await.len(),
        None => 0,
    };
    let uptime = state.started_at.elapsed().as_secs();

    Ok(serde_json::json!({
        "node_id": state.node_id,
        "active_connections": peer_count,
        "avg_latency_ms": 0.0,
        "min_latency_ms": 0.0,
        "max_latency_ms": 0.0,
        "throughput_bps": 0.0,
        "packet_loss_rate": 0.0,
        "congestion_window": 0,
        "uptime_secs": uptime,
        "status": "alpha",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::protocol::RpcRequest;

    #[tokio::test]
    async fn test_stoq_stats_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("stoq.stats", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["connections"], 0);
        assert_eq!(result["protocol"], "QUIC");
        assert_eq!(result["transport_active"], false);
        assert!(result["uptime_secs"].is_number());
    }

    #[tokio::test]
    async fn test_stoq_connections_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("stoq.connections", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["count"], 0);
        assert!(result["connections"].is_array());
    }

    #[tokio::test]
    async fn test_stoq_performance_returns_json() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("stoq.performance", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["active_connections"], 0);
        assert!(result["avg_latency_ms"].is_number());
        assert!(result["throughput_bps"].is_number());
        assert_eq!(result["status"], "alpha");
    }
}
