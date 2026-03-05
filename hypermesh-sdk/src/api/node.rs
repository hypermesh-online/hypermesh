// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Node status API.

use crate::client::HyperMeshClient;
use crate::error::SdkError;

/// Zero-cost wrapper providing node operations.
#[derive(Debug)]
pub struct NodeApi<'a> {
    pub(crate) client: &'a HyperMeshClient,
}

/// Snapshot of the local node's status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeStatus {
    /// Unique identifier for this node.
    pub node_id: String,
    /// Current privacy mode (Anonymous, Private, Public).
    pub privacy_mode: String,
    /// Height of the local Device blockchain.
    pub chain_height: u64,
    /// Number of connected peers.
    pub peers: usize,
    /// Seconds since the daemon started.
    pub uptime_secs: u64,
}

impl<'a> NodeApi<'a> {
    /// Fetch the current node status.
    pub async fn status(&self) -> Result<NodeStatus, SdkError> {
        let val = self
            .client
            .raw_call("node.status", serde_json::json!({}))
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_node_status() {
        let json = serde_json::json!({
            "node_id": "abc-123",
            "privacy_mode": "Public",
            "chain_height": 100,
            "peers": 5,
            "uptime_secs": 3600
        });
        let status: NodeStatus =
            serde_json::from_value(json).expect("test: deserialize NodeStatus");
        assert_eq!(status.node_id, "abc-123");
        assert_eq!(status.chain_height, 100);
        assert_eq!(status.peers, 5);
    }
}
