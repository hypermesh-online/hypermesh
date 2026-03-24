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

/// Result of an `asset.register` call.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetRegistrationResult {
    /// Content-hash based asset identifier (hex).
    pub asset_id: String,
    /// Block index the asset was registered in (`null` for local-only).
    pub block_index: Option<u64>,
    /// Registration status: "registered" or "registered_local".
    pub status: String,
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

    /// Register a new asset on the blockchain.
    ///
    /// # Arguments
    /// * `category` - "system" or "application"
    /// * `type_name` - e.g. "Dns", "Message", "Dashboard"
    /// * `content` - raw asset data bytes
    /// * `type_hash` - optional Catalog type hash (hex, for application assets)
    /// * `metadata` - optional JSON metadata
    pub async fn register(
        &self,
        category: &str,
        type_name: &str,
        content: &[u8],
        type_hash: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> Result<AssetRegistrationResult, SdkError> {
        let params = serde_json::json!({
            "category": category,
            "type_name": type_name,
            "content": hex::encode(content),
            "type_hash": type_hash.unwrap_or(""),
            "metadata": metadata.unwrap_or_else(|| serde_json::json!({})),
        });
        let val = self.client.raw_call("asset.register", params).await?;
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

    #[test]
    fn deserialize_registration_result() {
        let json = serde_json::json!({
            "asset_id": "abc123",
            "block_index": 5,
            "status": "registered"
        });
        let result: AssetRegistrationResult =
            serde_json::from_value(json).expect("test: deserialize AssetRegistrationResult");
        assert_eq!(result.asset_id, "abc123");
        assert_eq!(result.block_index, Some(5));
        assert_eq!(result.status, "registered");
    }

    #[test]
    fn deserialize_registration_result_local() {
        let json = serde_json::json!({
            "asset_id": "def456",
            "block_index": null,
            "status": "registered_local"
        });
        let result: AssetRegistrationResult =
            serde_json::from_value(json).expect("test: deserialize local result");
        assert_eq!(result.asset_id, "def456");
        assert_eq!(result.block_index, None);
        assert_eq!(result.status, "registered_local");
    }
}
