// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for persistence module

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::blockchain::block::Block;
    use crate::matrix::coordinate::MatrixCoordinate;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_full_persistence_integration() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");

        // 1. Test Matrix State Persistence
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let mut state = matrix_state::MatrixState::new(coord);
        state.add_neighbor("node1".to_string(), MatrixCoordinate::new(4, 5, 6).expect("test: valid coordinate"));

        let serializer = matrix_state::MatrixStateSerializer::new(
            matrix_state::SerializationFormat::Bincode,
            true,
        );

        let serialized = serializer.serialize(&state).expect("test: expected success");
        let deserialized = serializer.deserialize(&serialized).expect("test: expected success");
        assert_eq!(deserialized.coordinate, state.coordinate);
        assert_eq!(deserialized.neighbors.len(), 1);
        println!("✅ Matrix state persistence: PASSED");

        // 2. Test Blockchain Storage
        let storage = blockchain_storage::BlockchainStorage::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
        )
        .await
        .expect("test: expected success");

        let genesis = Block::genesis(coord);
        storage.write_block(&genesis).await.expect("test: async operation");

        let loaded = storage
            .read_block(blockchain_storage::BlockQuery::ByIndex(0))
            .await
            .expect("test: expected success");
        assert!(loaded.is_some());
        assert_eq!(loaded.expect("test: assertion value").hash, genesis.hash);
        println!("✅ Blockchain storage: PASSED");

        // 3. Test Topology Backup
        let backup_handler = topology_backup::TopologyBackup::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
        )
        .expect("test: expected success");

        let mut topology = topology_backup::NetworkTopology::new();
        let node =
            crate::matrix::geospatial::topology::TopologyNode::new("node1".to_string(), coord);
        topology.add_node(node);

        let backup_path = backup_handler.create_full_backup(&topology).await.expect("test: async operation");
        assert!(backup_path.exists());
        println!("✅ Topology backup: PASSED");

        // 4. Test Snapshot Manager
        let snapshot_mgr = snapshots::SnapshotManager::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
            snapshots::SnapshotSchedule::Manual,
        )
        .await
        .expect("test: expected success");

        #[derive(serde::Serialize, serde::Deserialize)]
        struct TestData {
            value: String,
        }

        let test_data = TestData {
            value: "test_snapshot".to_string(),
        };

        let snapshot_id = snapshot_mgr
            .create_snapshot(|| Ok(test_data), snapshots::SnapshotType::Full)
            .await
            .expect("test: expected success");

        assert!(!snapshot_id.is_empty());
        println!("✅ Snapshot manager: PASSED");

        // 5. Test Recovery Manager
        let mut recovery_mgr =
            recovery::RecoveryManager::new(temp_dir.path().to_path_buf(), "test_node".to_string());

        let report = recovery_mgr.recover_all().await.expect("test: async operation");
        assert!(
            report.status == recovery::RecoveryStatus::Completed
                || report.status == recovery::RecoveryStatus::Partial
        );
        println!("✅ Recovery manager: PASSED");

        // 6. Test Persistence Manager (full integration)
        let config = manager::PersistenceConfig {
            storage_dir: temp_dir.path().to_path_buf(),
            enable_background: false,
            ..Default::default()
        };

        let persistence_mgr = manager::PersistenceManager::new(config, "test_node".to_string())
            .await
            .expect("test: expected success");

        // Save matrix state
        persistence_mgr.save_matrix_state(&state).await.expect("test: async operation");

        // Save block
        let block = Block::genesis(coord);
        persistence_mgr.save_block(&block).await.expect("test: async operation");

        // Create snapshot
        let snapshot = persistence_mgr.create_snapshot().await.expect("test: async operation");
        assert!(!snapshot.is_empty());

        // Get stats
        let stats = persistence_mgr.get_stats().await;
        let _ = stats.total_used; // u64 always >= 0

        println!("✅ Persistence manager: PASSED");

        // Shutdown
        persistence_mgr.shutdown().await.expect("test: async operation");

        println!("\n🎉 ALL PERSISTENCE TESTS PASSED!");
    }

    #[test]
    fn test_persistence_error_types() {
        use crate::persistence::PersistenceError;

        let err1 = PersistenceError::ChecksumMismatch {
            expected: "abc".to_string(),
            actual: "def".to_string(),
        };
        assert!(err1.to_string().contains("Checksum"));

        let err2 = PersistenceError::VersionMismatch {
            expected: 1,
            actual: 2,
        };
        assert!(err2.to_string().contains("Version"));

        let err3 = PersistenceError::InsufficientDiskSpace {
            needed: 1000,
            available: 500,
        };
        assert!(err3.to_string().contains("disk space"));

        println!("✅ Error types test: PASSED");
    }
}
