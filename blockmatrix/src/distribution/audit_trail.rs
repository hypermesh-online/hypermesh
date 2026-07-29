// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Bounded In-Memory Placement-Audit Ledger
//!
//! Records shard placement, redistribution, and migration decisions in a
//! bounded in-process ledger for local transparency and verification. This
//! is NOT blockchain persistence — records live in memory only and are
//! evicted once the ledger's caps are reached (see [`BoundedAuditLedger`]).
//!
//! FUTURE: durable on-chain placement-audit persistence, if ever wanted, is a
//! separate feature — it would append these records to the node's blockchain
//! and require a real on-chain consumer. It is deliberately NOT built here:
//! no production code reads the audit today, so building chain persistence now
//! would be speculative decorative code.

use crate::assets::core::AssetResult;
use crate::distribution::audit_ledger::BoundedAuditLedger;
use crate::distribution::ShardPlacement;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::SystemTime;
use tokio::sync::RwLock;

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

/// Audit record for a single placement decision (in-memory).
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
    /// BLAKE3 content-hash record id (set after the record is appended)
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

/// Record shard placement in the bounded in-memory placement-audit ledger.
///
/// Appends one audit record per placement for local transparency and
/// verification. This is an in-memory record, NOT blockchain persistence.
///
/// # Arguments
///
/// * `asset_id` - Asset identifier
/// * `placements` - Shard placements to record
///
/// # Returns
///
/// List of audit records with their content-hash record ids
pub async fn record_shard_placement(
    asset_id: &str,
    placements: &[ShardPlacement],
) -> AssetResult<Vec<AuditRecord>> {
    let mut records = Vec::new();

    for placement in placements {
        let mut record =
            AuditRecord::from_placement(asset_id, placement, PlacementEvent::InitialPlacement);

        let tx_hash = append_to_ledger(&record).await?;
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

        let tx_hash = append_to_ledger(&record).await?;
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

    let tx_hash = append_to_ledger(&record).await?;
    record.tx_hash = Some(tx_hash);

    Ok(record)
}

/// Global bounded placement-audit ledger (caps + eviction in
/// [`super::audit_ledger`]).
static AUDIT_LEDGER: LazyLock<RwLock<BoundedAuditLedger>> =
    LazyLock::new(|| RwLock::new(BoundedAuditLedger::default()));

/// Query the placement-audit trail for an asset.
///
/// Returns the retained audit records for `asset_id` ordered by timestamp
/// (oldest first), or an empty list if none are retained. Bounded eviction
/// removes the oldest records first, so the most recent placements remain
/// queryable.
pub async fn query_audit_trail(asset_id: &str) -> AssetResult<Vec<AuditRecord>> {
    tracing::info!("Querying audit trail for asset: {}", asset_id);
    Ok(AUDIT_LEDGER.read().await.query(asset_id))
}

/// Verify that a shard is currently placed on the expected node.
///
/// Looks up the latest retained audit record for (`asset_id`, `shard_index`)
/// and checks whether its `node_id` matches `expected_node`.
pub async fn verify_placement(
    asset_id: &str,
    shard_index: usize,
    expected_node: &str,
) -> AssetResult<bool> {
    tracing::info!(
        "Verifying placement: asset={}, shard={}, node={}",
        asset_id,
        shard_index,
        expected_node
    );
    Ok(AUDIT_LEDGER
        .read()
        .await
        .verify(asset_id, shard_index, expected_node))
}

/// Append an audit record to the bounded in-memory ledger and return its
/// BLAKE3 content-hash record id.
///
/// The record is serialized to JSON and hashed with BLAKE3 to form a stable
/// record id, then appended under the bounded eviction policy. This is an
/// in-memory append, NOT blockchain persistence.
async fn append_to_ledger(record: &AuditRecord) -> AssetResult<String> {
    tracing::debug!(
        "Appending placement audit: asset={}, shard={}, node={}",
        record.asset_id,
        record.shard_index,
        record.node_id
    );

    // Serialize the record to produce deterministic input for hashing.
    let serialized = serde_json::to_vec(record).map_err(|e| {
        crate::assets::core::AssetError::ValidationError {
            message: format!("audit record serialization failed: {e}"),
        }
    })?;

    let hash = blake3::hash(&serialized);
    let tx_hash = format!("0x{}", hex::encode(hash.as_bytes()));

    let mut stored = record.clone();
    stored.tx_hash = Some(tx_hash.clone());
    AUDIT_LEDGER.write().await.append(stored);

    Ok(tx_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;

    fn create_test_placement() -> ShardPlacement {
        ShardPlacement {
            shard_index: 0,
            position: MatrixCoordinate::new(10, 20, 30).expect("test: valid coordinate"),
            node_id: "test-node".to_string(),
            octant: 0,
            distance_from_origin: 37.4,
        }
    }

    #[test]
    fn test_audit_record_creation() {
        let placement = create_test_placement();
        let record =
            AuditRecord::from_placement("test-asset", &placement, PlacementEvent::InitialPlacement);

        assert_eq!(record.asset_id, "test-asset");
        assert_eq!(record.shard_index, 0);
        assert_eq!(record.node_id, "test-node");
        assert_eq!(record.octant, 0);
        assert!(record.tx_hash.is_none());
    }

    #[tokio::test]
    async fn test_record_placement() {
        let placements = vec![create_test_placement()];
        let records = record_shard_placement("test-asset", &placements)
            .await
            .expect("test: expected success");

        assert_eq!(records.len(), 1);
        assert!(records[0].tx_hash.is_some());
    }

    #[tokio::test]
    async fn test_record_redistribution() {
        let placements = vec![create_test_placement()];
        let records = record_redistribution("test-asset", &placements, "PoS revocation")
            .await
            .expect("test: expected success");

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
        .expect("test: expected success");

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
    async fn test_query_audit_trail_returns_recorded_entries() {
        // Use a unique asset ID to avoid cross-test ledger contamination
        let asset_id = format!("query-test-{}", uuid::Uuid::new_v4());
        let placements = vec![create_test_placement()];

        // Record something first
        record_shard_placement(&asset_id, &placements)
            .await
            .expect("test: record placement");

        // Now query
        let records = query_audit_trail(&asset_id)
            .await
            .expect("test: query audit trail");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].asset_id, asset_id);
        assert_eq!(records[0].shard_index, 0);
        assert!(records[0].tx_hash.is_some());
    }

    #[tokio::test]
    async fn test_verify_placement_matches_recorded_node() {
        let asset_id = format!("verify-test-{}", uuid::Uuid::new_v4());
        let placements = vec![create_test_placement()];

        record_shard_placement(&asset_id, &placements)
            .await
            .expect("test: record placement");

        // Correct node should verify
        let correct = verify_placement(&asset_id, 0, "test-node")
            .await
            .expect("test: verify correct");
        assert!(correct);

        // Wrong node should fail
        let wrong = verify_placement(&asset_id, 0, "wrong-node")
            .await
            .expect("test: verify wrong");
        assert!(!wrong);
    }

    #[tokio::test]
    async fn test_verify_placement_unknown_asset_returns_false() {
        let result = verify_placement("nonexistent-asset", 0, "any-node")
            .await
            .expect("test: verify unknown");
        assert!(!result);
    }

    #[tokio::test]
    async fn test_append_to_ledger_uses_blake3() {
        let asset_id = format!("blake3-test-{}", uuid::Uuid::new_v4());
        let placements = vec![create_test_placement()];
        let records = record_shard_placement(&asset_id, &placements)
            .await
            .expect("test: record");

        let tx_hash = records[0].tx_hash.as_ref().expect("test: tx_hash");
        // BLAKE3 produces 64-char hex (32 bytes), prefixed with "0x"
        assert!(tx_hash.starts_with("0x"));
        assert_eq!(tx_hash.len(), 66, "0x + 64 hex chars");
    }
}
