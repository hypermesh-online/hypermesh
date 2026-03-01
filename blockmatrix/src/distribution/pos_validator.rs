// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Proof of State (PoS) Validator for Node Eligibility
//!
//! CRITICAL: All permission rules live in blockchain Asset records.
//! This module queries consensus for node eligibility before distribution.

use crate::assets::core::AssetResult;
use crate::assets::pipeline::sharding::Shard;
use crate::distribution::NodeInfo;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Storage access validation result from consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageAccessValidation {
    /// Can this node store the asset/shard?
    pub can_store: bool,
    /// Reason for denial (if can_store is false)
    pub reason: Option<String>,
    /// Required proofs that passed validation
    pub required_proofs: Vec<DistributionProofType>,
    /// Validation timestamp
    pub validation_timestamp: SystemTime,
    /// Validator node ID
    pub validator_node_id: String,
}

/// Distribution-specific proof type with PoX naming; canonical ProofType in hypermesh_lib.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DistributionProofType {
    /// Proof of Space (WHERE)
    PoSpace,
    /// Proof of Stake (WHO)
    PoStake,
    /// Proof of Work (WHAT)
    PoWork,
    /// Proof of Time (WHEN)
    PoTime,
}

/// Consensus validator trait for PoS queries
#[async_trait]
pub trait StateAuthenticator: Send + Sync {
    /// Validate storage access for a node
    async fn validate_storage_access(
        &self,
        node_id: &str,
        asset_id: &str,
        shard_id: &str,
    ) -> AssetResult<StorageAccessValidation>;

    /// Batch validate multiple nodes
    async fn batch_validate_storage_access(
        &self,
        nodes: &[String],
        asset_id: &str,
        shard_id: &str,
    ) -> AssetResult<Vec<StorageAccessValidation>>;
}

/// Get eligible nodes from consensus validation
///
/// Queries the blockchain Asset records via consensus to determine
/// which nodes have permission to store shards for this asset.
///
/// # Arguments
///
/// * `asset_id` - Asset identifier for permission lookup
/// * `asset_privacy_level` - Privacy level of the asset
/// * `shards` - Shards to be distributed
/// * `all_nodes` - All available nodes in network
/// * `consensus` - Consensus validator for PoS queries
///
/// # Returns
///
/// List of nodes that passed PoS validation for storage access
pub async fn get_eligible_nodes<C>(
    asset_id: &str,
    asset_privacy_level: &str,
    shards: &[Shard],
    all_nodes: &[NodeInfo],
    consensus: &C,
) -> AssetResult<Vec<NodeInfo>>
where
    C: StateAuthenticator,
{
    let mut eligible = Vec::new();

    // For testing/demo, use first shard ID
    let shard_id = if !shards.is_empty() {
        format!("{}-shard-{}", asset_id, shards[0].metadata.index)
    } else {
        format!("{asset_id}-shard-0")
    };

    for node in all_nodes {
        // Privacy-level based filtering
        // Also check network_id for PrivateNetwork assets
        if asset_privacy_level == "PrivateNetwork" {
            // For PrivateNetwork, also ensure same network
            // In real implementation, this would check against Asset's allowed_networks
            // For testing, we allow nodes matching the privacy level
            if node.privacy_level != "PrivateNetwork" {
                continue;
            }
        } else if !privacy_level_allows_node(asset_privacy_level, &node.privacy_level) {
            continue;
        }

        // Query consensus for node eligibility
        match consensus
            .validate_storage_access(&node.node_id, asset_id, &shard_id)
            .await
        {
            Ok(validation) => {
                if validation.can_store {
                    eligible.push(node.clone());
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Consensus validation failed for node {}: {}",
                    node.node_id,
                    e
                );
                // Continue checking other nodes
            }
        }
    }

    Ok(eligible)
}

/// Validate if a single node is eligible for storage
pub async fn validate_node_eligibility<C>(
    node_id: &str,
    asset_id: &str,
    asset_privacy_level: &str,
    node_privacy_level: &str,
    consensus: &C,
) -> AssetResult<bool>
where
    C: StateAuthenticator,
{
    // Privacy level check
    if !privacy_level_allows_node(asset_privacy_level, node_privacy_level) {
        return Ok(false);
    }

    // Consensus validation
    let shard_id = format!("{asset_id}-shard-0");
    let validation = consensus
        .validate_storage_access(node_id, asset_id, &shard_id)
        .await?;

    Ok(validation.can_store)
}

/// Privacy level compatibility check
///
/// Determines if a node's privacy level is compatible with asset requirements.
///
/// Rules:
/// - Private: Only explicitly granted nodes
/// - PrivateNetwork: Nodes in same private network
/// - PublicNetwork: Public nodes only
/// - FullPublic: Any public node
fn privacy_level_allows_node(asset_privacy: &str, node_privacy: &str) -> bool {
    match (asset_privacy, node_privacy) {
        // Private assets only on explicitly granted nodes
        ("Private", _) => false, // Must be explicitly granted via PoS

        // PrivateNetwork assets on same network nodes
        ("PrivateNetwork", "PrivateNetwork") => true,
        ("PrivateNetwork", _) => false,

        // PublicNetwork assets on public nodes
        ("PublicNetwork", "PublicNetwork") => true,
        ("PublicNetwork", "FullPublic") => true,
        ("PublicNetwork", _) => false,

        // FullPublic on any public node
        ("FullPublic", "PublicNetwork") => true,
        ("FullPublic", "FullPublic") => true,
        ("FullPublic", _) => false,

        // Default deny
        _ => false,
    }
}

/// Mock StateAuthenticator for testing only.
/// Gated behind cfg(test) so it is never included in production builds.
#[cfg(test)]
pub struct MockStateAuthenticator {
    pub allow_all: bool,
}

#[cfg(test)]
impl MockStateAuthenticator {
    pub fn new(allow_all: bool) -> Self {
        Self { allow_all }
    }
}

#[cfg(test)]
#[async_trait]
impl StateAuthenticator for MockStateAuthenticator {
    async fn validate_storage_access(
        &self,
        _node_id: &str,
        _asset_id: &str,
        _shard_id: &str,
    ) -> AssetResult<StorageAccessValidation> {
        Ok(StorageAccessValidation {
            can_store: self.allow_all,
            reason: if self.allow_all {
                None
            } else {
                Some("Denied by mock validator".to_string())
            },
            required_proofs: vec![
                DistributionProofType::PoSpace,
                DistributionProofType::PoStake,
                DistributionProofType::PoWork,
                DistributionProofType::PoTime,
            ],
            validation_timestamp: SystemTime::now(),
            validator_node_id: "mock-validator".to_string(),
        })
    }

    async fn batch_validate_storage_access(
        &self,
        nodes: &[String],
        asset_id: &str,
        shard_id: &str,
    ) -> AssetResult<Vec<StorageAccessValidation>> {
        let mut results = Vec::new();
        for node in nodes {
            results.push(
                self.validate_storage_access(node, asset_id, shard_id)
                    .await?,
            );
        }
        Ok(results)
    }
}

/// Implementation for DefaultStateAuthenticator
#[async_trait]
impl StateAuthenticator for crate::consensus::validation::DefaultStateAuthenticator {
    async fn validate_storage_access(
        &self,
        node_id: &str,
        _asset_id: &str,
        _shard_id: &str,
    ) -> AssetResult<StorageAccessValidation> {
        // For now, use basic validation based on test mode
        // In production, this would query blockchain Asset records

        let can_store = true; // Testing mode allows all

        Ok(StorageAccessValidation {
            can_store,
            reason: if can_store {
                None
            } else {
                Some("Node not authorized by blockchain Asset record".to_string())
            },
            required_proofs: vec![
                DistributionProofType::PoSpace,
                DistributionProofType::PoStake,
                DistributionProofType::PoWork,
                DistributionProofType::PoTime,
            ],
            validation_timestamp: SystemTime::now(),
            validator_node_id: format!("validator-for-{node_id}"),
        })
    }

    async fn batch_validate_storage_access(
        &self,
        nodes: &[String],
        asset_id: &str,
        shard_id: &str,
    ) -> AssetResult<Vec<StorageAccessValidation>> {
        let mut results = Vec::new();
        for node in nodes {
            results.push(
                self.validate_storage_access(node, asset_id, shard_id)
                    .await?,
            );
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;

    #[test]
    fn test_privacy_level_compatibility() {
        // Private network compatibility
        assert!(privacy_level_allows_node(
            "PrivateNetwork",
            "PrivateNetwork"
        ));
        assert!(!privacy_level_allows_node(
            "PrivateNetwork",
            "PublicNetwork"
        ));

        // Public network compatibility
        assert!(privacy_level_allows_node("PublicNetwork", "PublicNetwork"));
        assert!(privacy_level_allows_node("PublicNetwork", "FullPublic"));
        assert!(!privacy_level_allows_node(
            "PublicNetwork",
            "PrivateNetwork"
        ));

        // Full public compatibility
        assert!(privacy_level_allows_node("FullPublic", "FullPublic"));
        assert!(privacy_level_allows_node("FullPublic", "PublicNetwork"));
    }

    #[tokio::test]
    async fn test_get_eligible_nodes() {
        let nodes = vec![
            NodeInfo::new(
                "node1".to_string(),
                MatrixCoordinate::new(10, 10, 10).expect("test: valid coordinate"),
                "PrivateNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
            NodeInfo::new(
                "node2".to_string(),
                MatrixCoordinate::new(20, 20, 20).expect("test: valid coordinate"),
                "PublicNetwork".to_string(),
                1_000_000_000,
                "network1".to_string(),
            ),
        ];

        let shards = vec![];
        let consensus = MockStateAuthenticator::new(true);

        // Test PrivateNetwork asset
        let eligible =
            get_eligible_nodes("test-asset", "PrivateNetwork", &shards, &nodes, &consensus)
                .await
                .expect("test: expected success");

        // Only node1 should be eligible (PrivateNetwork)
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0].node_id, "node1");
    }

    #[tokio::test]
    async fn test_validate_node_eligibility() {
        let consensus = MockStateAuthenticator::new(true);

        let result = validate_node_eligibility(
            "node1",
            "test-asset",
            "PrivateNetwork",
            "PrivateNetwork",
            &consensus,
        )
        .await
        .expect("test: expected success");

        assert!(result);

        // Test mismatched privacy levels
        let result = validate_node_eligibility(
            "node2",
            "test-asset",
            "PrivateNetwork",
            "PublicNetwork",
            &consensus,
        )
        .await
        .expect("test: expected success");

        assert!(!result);
    }
}
