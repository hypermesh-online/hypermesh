// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Topology IPC handlers: info, neighbors, routing_cost, path.

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INVALID_PARAMS};
use crate::ipc::state::DaemonState;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::neighbors;
use crate::matrix::tensor::routing::{
    calculate_routing_path, calculate_routing_vector, score_route_quality,
};

/// Register topology-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // topology.info — this node's matrix position
    {
        let s = state.clone();
        handler.register(
            "topology.info",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move {
                    Ok(serde_json::json!({
                        "node_id": s.node_id,
                        "coordinate": {
                            "x": s.coordinate.x,
                            "y": s.coordinate.y,
                            "z": s.coordinate.z,
                        },
                    }))
                })
            }),
        );
    }

    // topology.neighbors — find neighbors within radius from connected peers
    {
        let s = state.clone();
        handler.register(
            "topology.neighbors",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let radius = params
                        .get("radius")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(5.0);

                    let center = s.coordinate;

                    // Get connected peer coordinates as candidates
                    let candidates: Vec<MatrixCoordinate> = match &s.network {
                        Some(net) => net.get_connected_coordinates().await,
                        None => Vec::new(),
                    };

                    let found =
                        neighbors::find_neighbors(&center, &candidates, radius);

                    let entries: Vec<serde_json::Value> = found
                        .iter()
                        .map(|c| {
                            serde_json::json!({
                                "x": c.x, "y": c.y, "z": c.z,
                            })
                        })
                        .collect();

                    Ok(serde_json::json!({
                        "center": {"x": center.x, "y": center.y, "z": center.z},
                        "radius": radius,
                        "count": entries.len(),
                        "neighbors": entries,
                    }))
                })
            }),
        );
    }

    // topology.routing_cost — cost between two coordinates
    handler.register(
        "topology.routing_cost",
        Arc::new(|params| {
            Box::pin(async move {
                let parse = |prefix: &str| -> Result<MatrixCoordinate, RpcError> {
                    let x = params
                        .get(&format!("{prefix}_x"))
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: format!("missing '{prefix}_x'"),
                            data: None,
                        })?;
                    let y = params
                        .get(&format!("{prefix}_y"))
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: format!("missing '{prefix}_y'"),
                            data: None,
                        })?;
                    let z = params
                        .get(&format!("{prefix}_z"))
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: format!("missing '{prefix}_z'"),
                            data: None,
                        })?;
                    MatrixCoordinate::new(x, y, z).map_err(|e| RpcError {
                        code: INVALID_PARAMS,
                        message: format!("invalid coordinate: {e}"),
                        data: None,
                    })
                };

                let from = parse("from")?;
                let to = parse("to")?;

                let vec = calculate_routing_vector(&from, &to);
                let quality = score_route_quality(&[from, to], 1.0);

                Ok(serde_json::json!({
                    "from": {"x": from.x, "y": from.y, "z": from.z},
                    "to": {"x": to.x, "y": to.y, "z": to.z},
                    "vector": {"x": vec.x, "y": vec.y, "z": vec.z},
                    "route_quality": quality,
                }))
            })
        }),
    );

    // topology.path — show routing path between two coordinates
    handler.register(
        "topology.path",
        Arc::new(|params| {
            Box::pin(async move {
                let parse = |prefix: &str| -> Result<MatrixCoordinate, RpcError> {
                    let x = params
                        .get(&format!("{prefix}_x"))
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: format!("missing '{prefix}_x'"),
                            data: None,
                        })?;
                    let y = params
                        .get(&format!("{prefix}_y"))
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: format!("missing '{prefix}_y'"),
                            data: None,
                        })?;
                    let z = params
                        .get(&format!("{prefix}_z"))
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: format!("missing '{prefix}_z'"),
                            data: None,
                        })?;
                    MatrixCoordinate::new(x, y, z).map_err(|e| RpcError {
                        code: INVALID_PARAMS,
                        message: format!("invalid coordinate: {e}"),
                        data: None,
                    })
                };

                let from = parse("from")?;
                let to = parse("to")?;

                let path = calculate_routing_path(&from, &to, 1.0);
                let hops: Vec<serde_json::Value> = path
                    .iter()
                    .map(|c| serde_json::json!({"x": c.x, "y": c.y, "z": c.z}))
                    .collect();

                Ok(serde_json::json!({
                    "from": {"x": from.x, "y": from.y, "z": from.z},
                    "to": {"x": to.x, "y": to.y, "z": to.z},
                    "hops": hops.len(),
                    "path": hops,
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
    use crate::persistence::{PersistenceConfig, PersistenceManager};
    use std::path::PathBuf;
    use std::time::Instant;

    async fn test_state() -> Arc<DaemonState> {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: coord");
        let bc = Arc::new(NodeBlockchain::new(coord));
        let config = PersistenceConfig {
            storage_dir: PathBuf::from("/tmp"),
            ..PersistenceConfig::default()
        };
        let persistence = Arc::new(
            PersistenceManager::new(config, "topo-test".into())
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
            node_id: "topo-test".into(),
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
            #[cfg(feature = "intelligence")]
            federation_manager: None,
            #[cfg(feature = "intelligence")]
            threshold_coordinator: None,

            transfer_coordinator: None,
            foundation_signing_key: None,
            dns_registrar: None,
            release_feed_subscriber: None,
            receipt_validator: Arc::new(
                crate::assets::cross_chain::CrossChainReceiptValidator::new(),
            ),
            capability_token_issuer: None,
            revocation_registry: Arc::new(crate::auth::RevocationRegistry::new()),
            light_sync_manager: None,
            catalog_registry: None,
            inbox_store: None,
        })
    }

    #[tokio::test]
    async fn test_topology_info() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("topology.info", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["coordinate"]["x"], 1);
        assert_eq!(result["coordinate"]["y"], 2);
        assert_eq!(result["coordinate"]["z"], 3);
    }

    #[tokio::test]
    async fn test_topology_neighbors() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "topology.neighbors",
            serde_json::json!({"radius": 1.5}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert!(result["count"].is_number());
    }

    #[tokio::test]
    async fn test_topology_routing_cost() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "topology.routing_cost",
            serde_json::json!({
                "from_x": 0, "from_y": 0, "from_z": 0,
                "to_x": 5, "to_y": 5, "to_z": 5,
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert!(result["route_quality"].is_number());
    }

    #[tokio::test]
    async fn test_topology_path() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "topology.path",
            serde_json::json!({
                "from_x": 0, "from_y": 0, "from_z": 0,
                "to_x": 3, "to_y": 3, "to_z": 3,
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert!(result["hops"].is_number());
        assert!(result["path"].is_array());
    }
}
