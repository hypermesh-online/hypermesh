// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Blockchain Audit Trail for Shard Placement
//!
//! Records all shard placement decisions on blockchain for transparency
//! and verification.

use crate::assets::core::{AssetError, AssetResult};
use crate::distribution::ShardPlacement;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Placement event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlacementEvent {
    /// Initial shard placement
    InitialPlacement,
    /// Shard redistributed due to PoS revocation
    Redistribution { reason: String },
    /// Shard replicated for redundancy
    Replication { original_node: String },
    /// Shard migrated to new node
    Migration {
        from_node: String,
        to_node: String,
        reason: String,
    },
}

/// Audit record for blockchain storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Asset identifier
    pub asset_id: String,
    /// Shard index
    pub shard_index: usize,
    /// Node hosting the shard
    pub node_id: String,
    /// Matrix position
    pub position_x: i64,
    pub position_y: i64,
    pub position_z: i64,
    /// Octant assignment
    pub octant: u8,
    /// Distance from origin
    pub distance_from_origin: f64,
    /// Event type
    pub event: PlacementEvent,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Blockchain transaction hash (set after recording)
    pub tx_hash: Option<String>,
}

impl AuditRecord {
    /// Create audit record from placement
    pub fn from_placement(
        asset_id: &str,
        placement: &ShardPlacement,
        event: PlacementEvent,
    ) -> Self {
        Self {
            asset_id: asset_id.to_string(),
            shard_index: placement.shard_index,
            node_id: placement.node_id.clone(),
            position_x: placement.position.x,
            position_y: placement.position.y,
            position_z: placement.position.z,
            octant: placement.octant,
            distance_from_origin: placement.distance_from_origin,
            event,
            timestamp: SystemTime::now(),
            tx_hash: None,
        }
    }
}

/// Record shard placement on blockchain
///
/// Creates audit trail of placement decisions for verification and compliance.
///
/// # Arguments
///
/// * `asset_id` - Asset identifier
/// * `placements` - Shard placements to record
///
/// # Returns
///
/// List of audit records with blockchain transaction hashes
pub async fn record_shard_placement_on_chain(
    asset_id: &str,
    placements: &[ShardPlacement],
) -> AssetResult<Vec<AuditRecord>> {
    let mut records = Vec::new();

    for placement in placements {
        let mut record = AuditRecord::from_placement(
            asset_id,
            placement,
            PlacementEvent::InitialPlacement,
        );

        // Record on blockchain (stub implementation)
        let tx_hash = record_to_blockchain(&record).await?;
        record.tx_hash = Some(tx_hash);

        records.push(record);
    }

    Ok(records)
}

/// Record redistribution event
pub async fn record_redistribution(
    asset_id: &str,
    placements: &[ShardPlacement],
    reason: &str,
) -> AssetResult<Vec<AuditRecord>> {
    let mut records = Vec::new();

    for placement in placements {
        let mut record = AuditRecord::from_placement(
            asset_id,
            placement,
            PlacementEvent::Redistribution {
                reason: reason.to_string(),
            },
        );

        let tx_hash = record_to_blockchain(&record).await?;
        record.tx_hash = Some(tx_hash);

        records.push(record);
    }

    Ok(records)
}

/// Record migration event
pub async fn record_migration(
    asset_id: &str,
    from_node: &str,
    placement: &ShardPlacement,
    reason: &str,
) -> AssetResult<AuditRecord> {
    let mut record = AuditRecord::from_placement(
        asset_id,
        placement,
        PlacementEvent::Migration {
            from_node: from_node.to_string(),
            to_node: placement.node_id.clone(),
            reason: reason.to_string(),
        },
    );

    let tx_hash = record_to_blockchain(&record).await?;
    record.tx_hash = Some(tx_hash);

    Ok(record)
}

/// Query audit trail for asset
pub async fn query_audit_trail(asset_id: &str) -> AssetResult<Vec<AuditRecord>> {
    // Stub implementation - would query blockchain
    tracing::info!("Querying audit trail for asset: {}", asset_id);

    // In production, this would:
    // 1. Query blockchain for all placement records
    // 2. Filter by asset_id
    // 3. Order by timestamp
    // 4. Return complete audit history

    Ok(Vec::new())
}

/// Verify placement against audit trail
pub async fn verify_placement(
    asset_id: &str,
    shard_index: usize,
    expected_node: &str,
) -> AssetResult<bool> {
    // Stub implementation - would verify against blockchain
    tracing::info!(
        "Verifying placement: asset={}, shard={}, node={}",
        asset_id,
        shard_index,
        expected_node
    );

    // In production, this would:
    // 1. Query latest audit record for this shard
    // 2. Verify node_id matches expected_node
    // 3. Validate blockchain signature
    // 4. Check timestamp is recent

    Ok(true)
}

/// Record audit record to blockchain (stub implementation)
async fn record_to_blockchain(record: &AuditRecord) -> AssetResult<String> {
    // Stub implementation - would submit blockchain transaction
    tracing::debug!(
        "Recording to blockchain: asset={}, shard={}, node={}",
        record.asset_id,
        record.shard_index,
        record.node_id
    );

    // In production, this would:
    // 1. Serialize audit record
    // 2. Create blockchain transaction
    // 3. Sign with node key
    // 4. Submit to blockchain
    // 5. Wait for confirmation
    // 6. Return transaction hash

    // Generate mock transaction hash
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    record.asset_id.hash(&mut hasher);
    record.shard_index.hash(&mut hasher);
    record.node_id.hash(&mut hasher);
    let hash_value = hasher.finish();

    let tx_hash = format!("0x{:016x}", hash_value);

    Ok(tx_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;

    fn create_test_placement() -> ShardPlacement {
        ShardPlacement {
            shard_index: 0,
            position: MatrixCoordinate::new(10, 20, 30).unwrap(),
            node_id: "test-node".to_string(),
            octant: 0,
            distance_from_origin: 37.4,
        }
    }

    #[test]
    fn test_audit_record_creation() {
        let placement = create_test_placement();
        let record = AuditRecord::from_placement(
            "test-asset",
            &placement,
            PlacementEvent::InitialPlacement,
        );

        assert_eq!(record.asset_id, "test-asset");
        assert_eq!(record.shard_index, 0);
        assert_eq!(record.node_id, "test-node");
        assert_eq!(record.octant, 0);
        assert!(record.tx_hash.is_none());
    }

    #[tokio::test]
    async fn test_record_placement() {
        let placements = vec![create_test_placement()];
        let records =
            record_shard_placement_on_chain("test-asset", &placements)
                .await
                .unwrap();

        assert_eq!(records.len(), 1);
        assert!(records[0].tx_hash.is_some());
    }

    #[tokio::test]
    async fn test_record_redistribution() {
        let placements = vec![create_test_placement()];
        let records = record_redistribution(
            "test-asset",
            &placements,
            "PoS revocation",
        )
        .await
        .unwrap();

        assert_eq!(records.len(), 1);
        assert!(records[0].tx_hash.is_some());

        match &records[0].event {
            PlacementEvent::Redistribution { reason } => {
                assert_eq!(reason, "PoS revocation");
            }
            _ => panic!("Expected Redistribution event"),
        }
    }

    #[tokio::test]
    async fn test_record_migration() {
        let placement = create_test_placement();
        let record = record_migration(
            "test-asset",
            "old-node",
            &placement,
            "Node capacity exceeded",
        )
        .await
        .unwrap();

        assert!(record.tx_hash.is_some());

        match &record.event {
            PlacementEvent::Migration {
                from_node,
                to_node,
                reason,
            } => {
                assert_eq!(from_node, "old-node");
                assert_eq!(to_node, "test-node");
                assert_eq!(reason, "Node capacity exceeded");
            }
            _ => panic!("Expected Migration event"),
        }
    }

    #[tokio::test]
    async fn test_query_audit_trail() {
        let result = query_audit_trail("test-asset").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_placement() {
        let result = verify_placement("test-asset", 0, "test-node").await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
