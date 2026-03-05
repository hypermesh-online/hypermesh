// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Asset storage and retrieval API.

use crate::client::HyperMeshClient;
use crate::error::SdkError;

/// Zero-cost wrapper providing asset operations.
#[derive(Debug)]
pub struct AssetApi<'a> {
    pub(crate) client: &'a HyperMeshClient,
}

/// Summary of a stored asset.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetInfo {
    /// Content-addressed asset identifier.
    pub asset_id: String,
    /// Size in bytes.
    pub size: u64,
    /// BLAKE3 content hash (hex).
    pub content_hash: String,
    /// Number of Reed-Solomon shards.
    pub shard_count: u32,
}

/// Result of a store operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoreResult {
    /// The asset ID assigned to the stored content.
    pub asset_id: String,
}

impl<'a> AssetApi<'a> {
    /// Store a file from the given path and return its asset ID.
    pub async fn store(&self, path: &str) -> Result<StoreResult, SdkError> {
        let val = self
            .client
            .raw_call("asset.store", serde_json::json!({"path": path}))
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }

    /// Fetch an asset by ID and write it to the given output path.
    pub async fn fetch(&self, asset_id: &str, output: &str) -> Result<(), SdkError> {
        self.client
            .raw_call(
                "asset.fetch",
                serde_json::json!({"asset_id": asset_id, "output": output}),
            )
            .await?;
        Ok(())
    }

    /// List all assets stored on this node.
    pub async fn list(&self) -> Result<Vec<AssetInfo>, SdkError> {
        let val = self
            .client
            .raw_call("asset.list", serde_json::json!({}))
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }

    /// Get detailed info about a specific asset.
    pub async fn info(&self, asset_id: &str) -> Result<AssetInfo, SdkError> {
        let val = self
            .client
            .raw_call("asset.info", serde_json::json!({"asset_id": asset_id}))
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_asset_info() {
        let json = serde_json::json!({
            "asset_id": "asset-001",
            "size": 1048576,
            "content_hash": "abcdef1234567890",
            "shard_count": 14
        });
        let info: AssetInfo = serde_json::from_value(json).expect("test: deserialize AssetInfo");
        assert_eq!(info.asset_id, "asset-001");
        assert_eq!(info.size, 1048576);
        assert_eq!(info.shard_count, 14);
    }

    #[test]
    fn deserialize_store_result() {
        let json = serde_json::json!({"asset_id": "asset-002"});
        let result: StoreResult =
            serde_json::from_value(json).expect("test: deserialize StoreResult");
        assert_eq!(result.asset_id, "asset-002");
    }
}
