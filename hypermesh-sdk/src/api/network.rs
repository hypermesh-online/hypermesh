// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Network peer management API.

use crate::client::HyperMeshClient;
use crate::error::SdkError;

/// Zero-cost wrapper providing network operations.
#[derive(Debug)]
pub struct NetworkApi<'a> {
    pub(crate) client: &'a HyperMeshClient,
}

/// Information about a connected peer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerInfo {
    /// The peer's node ID.
    pub node_id: String,
    /// The peer's STOQ address.
    pub address: String,
    /// Seconds since the peer was last seen.
    pub last_seen_secs: u64,
}

impl<'a> NetworkApi<'a> {
    /// List all connected peers.
    pub async fn peers(&self) -> Result<Vec<PeerInfo>, SdkError> {
        let val = self
            .client
            .raw_call("network.peers", serde_json::json!({}))
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }

    /// Initiate a connection to a peer at the given STOQ address.
    pub async fn connect_peer(&self, address: &str) -> Result<PeerInfo, SdkError> {
        let val = self
            .client
            .raw_call(
                "network.connect",
                serde_json::json!({"address": address}),
            )
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_peer_info() {
        let json = serde_json::json!({
            "node_id": "peer-42",
            "address": "[fd00::2]:8444",
            "last_seen_secs": 10
        });
        let peer: PeerInfo = serde_json::from_value(json).expect("test: deserialize PeerInfo");
        assert_eq!(peer.node_id, "peer-42");
        assert_eq!(peer.last_seen_secs, 10);
    }
}
