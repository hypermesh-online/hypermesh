// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Blockchain query API.

use crate::client::HyperMeshClient;
use crate::error::SdkError;

/// Zero-cost wrapper providing blockchain operations.
#[derive(Debug)]
pub struct BlockchainApi<'a> {
    pub(crate) client: &'a HyperMeshClient,
}

/// Summary of a block in the local Device chain.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockInfo {
    /// Block index (height).
    pub index: u64,
    /// BLAKE3 hash of this block (hex).
    pub hash: String,
    /// BLAKE3 hash of the previous block (hex).
    pub previous_hash: String,
    /// Unix timestamp when the block was created.
    pub timestamp: u64,
    /// Number of transactions in this block.
    pub transaction_count: u32,
}

/// Result of a chain validation check.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    /// Whether the entire chain validated successfully.
    pub valid: bool,
    /// Number of blocks checked.
    pub blocks_checked: u64,
    /// Human-readable description if validation failed.
    pub error: Option<String>,
}

impl<'a> BlockchainApi<'a> {
    /// Get the current chain height (number of blocks).
    pub async fn height(&self) -> Result<u64, SdkError> {
        let val = self
            .client
            .raw_call("blockchain.height", serde_json::json!({}))
            .await?;
        val.as_u64()
            .ok_or_else(|| SdkError::Serialization("expected u64 height".into()))
    }

    /// Get a block by its index.
    pub async fn block(&self, index: u64) -> Result<BlockInfo, SdkError> {
        let val = self
            .client
            .raw_call("blockchain.block", serde_json::json!({"index": index}))
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }

    /// Validate the integrity of the local blockchain.
    pub async fn validate(&self) -> Result<ValidationResult, SdkError> {
        let val = self
            .client
            .raw_call("blockchain.validate", serde_json::json!({}))
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_block_info() {
        let json = serde_json::json!({
            "index": 7,
            "hash": "aabbccdd",
            "previous_hash": "00112233",
            "timestamp": 1700000000,
            "transaction_count": 3
        });
        let block: BlockInfo = serde_json::from_value(json).expect("test: deserialize BlockInfo");
        assert_eq!(block.index, 7);
        assert_eq!(block.transaction_count, 3);
    }

    #[test]
    fn deserialize_validation_result() {
        let json = serde_json::json!({
            "valid": true,
            "blocks_checked": 42,
            "error": null
        });
        let result: ValidationResult =
            serde_json::from_value(json).expect("test: deserialize ValidationResult");
        assert!(result.valid);
        assert_eq!(result.blocks_checked, 42);
        assert!(result.error.is_none());
    }
}
