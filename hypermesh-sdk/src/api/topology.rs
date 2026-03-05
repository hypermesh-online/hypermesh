// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Block-MATRIX topology API.

use crate::client::HyperMeshClient;
use crate::error::SdkError;

/// Zero-cost wrapper providing topology operations.
#[derive(Debug)]
pub struct TopologyApi<'a> {
    pub(crate) client: &'a HyperMeshClient,
}

/// This node's position and topology metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TopologyInfo {
    /// Matrix x coordinate.
    pub x: f64,
    /// Matrix y coordinate.
    pub y: f64,
    /// Matrix z coordinate.
    pub z: f64,
    /// Number of known matrix neighbors.
    pub neighbor_count: usize,
}

/// A neighbor node in the matrix.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Neighbor {
    /// The neighbor's node ID.
    pub node_id: String,
    /// Matrix x coordinate.
    pub x: f64,
    /// Matrix y coordinate.
    pub y: f64,
    /// Matrix z coordinate.
    pub z: f64,
    /// Euclidean distance from the queried center.
    pub distance: f64,
}

/// Cost estimate for routing between two matrix positions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingCost {
    /// Estimated hop count.
    pub hops: u32,
    /// Estimated latency in milliseconds.
    pub latency_ms: f64,
}

impl<'a> TopologyApi<'a> {
    /// Get this node's position in the Block-MATRIX.
    pub async fn info(&self) -> Result<TopologyInfo, SdkError> {
        let val = self
            .client
            .raw_call("topology.info", serde_json::json!({}))
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }

    /// Find neighbors within `radius` of the given matrix position.
    pub async fn neighbors(
        &self,
        x: f64,
        y: f64,
        z: f64,
        radius: f64,
    ) -> Result<Vec<Neighbor>, SdkError> {
        let val = self
            .client
            .raw_call(
                "topology.neighbors",
                serde_json::json!({"x": x, "y": y, "z": z, "radius": radius}),
            )
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }

    /// Estimate the routing cost between two matrix positions.
    pub async fn routing_cost(
        &self,
        from: [f64; 3],
        to: [f64; 3],
    ) -> Result<RoutingCost, SdkError> {
        let val = self
            .client
            .raw_call(
                "topology.routing_cost",
                serde_json::json!({
                    "from": {"x": from[0], "y": from[1], "z": from[2]},
                    "to": {"x": to[0], "y": to[1], "z": to[2]}
                }),
            )
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_topology_info() {
        let json = serde_json::json!({
            "x": 1.0, "y": 2.0, "z": 3.0,
            "neighbor_count": 8
        });
        let info: TopologyInfo =
            serde_json::from_value(json).expect("test: deserialize TopologyInfo");
        assert_eq!(info.neighbor_count, 8);
    }

    #[test]
    fn deserialize_neighbor() {
        let json = serde_json::json!({
            "node_id": "n-1",
            "x": 1.5, "y": 2.5, "z": 3.5,
            "distance": 0.866
        });
        let neighbor: Neighbor =
            serde_json::from_value(json).expect("test: deserialize Neighbor");
        assert_eq!(neighbor.node_id, "n-1");
    }

    #[test]
    fn deserialize_routing_cost() {
        let json = serde_json::json!({"hops": 3, "latency_ms": 12.5});
        let cost: RoutingCost =
            serde_json::from_value(json).expect("test: deserialize RoutingCost");
        assert_eq!(cost.hops, 3);
    }
}
