// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Topology backup and restore
//!
//! Handles backing up and restoring the network topology including
//! geographic zones, clusters, and load balancing state.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::matrix::geospatial::topology::{TopologyNode, TopologyEdge};
use crate::matrix::geospatial::hierarchy::GeographicZone;
use super::{PersistenceError, PersistenceResult};

/// Placeholder for clustering results until Sprint 1.4 integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusteringResult {
    pub algorithm: String,
    pub clusters: Vec<Vec<String>>,  // Node IDs in each cluster
    pub metadata: HashMap<String, String>,
}

/// Temporary NetworkTopology wrapper until full integration
/// This will be replaced when Sprint 1.4 NetworkTopology is available
pub struct NetworkTopology {
    nodes: HashMap<String, TopologyNode>,
    edges: Vec<TopologyEdge>,
    zones: Vec<GeographicZone>,
    clusters: Option<Vec<ClusteringResult>>,
    load_balancing_state: Option<LoadBalancingState>,
}

impl NetworkTopology {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            zones: Vec::new(),
            clusters: None,
            load_balancing_state: None,
        }
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&String, &TopologyNode)> {
        self.nodes.iter()
    }

    pub fn edges(&self) -> impl Iterator<Item = &TopologyEdge> {
        self.edges.iter()
    }

    pub fn zones(&self) -> impl Iterator<Item = &GeographicZone> {
        self.zones.iter()
    }

    pub fn get_clusters(&self) -> Option<Vec<ClusteringResult>> {
        self.clusters.clone()
    }

    pub fn get_load_balancing_state(&self) -> Option<LoadBalancingState> {
        self.load_balancing_state.clone()
    }

    pub fn add_node(&mut self, node: TopologyNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, source: String, target: String, weight: f64) {
        self.edges.push(TopologyEdge {
            source,
            target,
            weight,
            metadata: HashMap::new(),
        });
    }
}

/// Backup mode options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackupMode {
    /// Complete topology snapshot
    Full,
    /// Only changed nodes and edges
    Incremental,
    /// Only critical state (zones and clusters)
    Essential,
}

/// Complete topology backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyBackupData {
    /// Backup version
    pub version: u32,
    /// Backup timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Backup mode used
    pub mode: String,
    /// Network nodes
    pub nodes: HashMap<String, TopologyNode>,
    /// Network edges
    pub edges: Vec<TopologyEdge>,
    /// Geographic zones
    pub zones: HashMap<String, GeographicZone>,
    /// Clustering results
    pub clusters: Vec<ClusteringResult>,
    /// Load balancing state
    pub load_balancing: Option<LoadBalancingState>,
    /// Backup metadata
    pub metadata: HashMap<String, String>,
}

impl TopologyBackupData {
    /// Create new backup data
    pub fn new(mode: BackupMode) -> Self {
        Self {
            version: 1,
            timestamp: chrono::Utc::now(),
            mode: format!("{:?}", mode),
            nodes: HashMap::new(),
            edges: Vec::new(),
            zones: HashMap::new(),
            clusters: Vec::new(),
            load_balancing: None,
            metadata: HashMap::new(),
        }
    }

    /// Get size estimate
    pub fn size_estimate(&self) -> usize {
        let nodes_size: usize = self.nodes.iter()
            .map(|(k, _v)| k.len() + 100) // Rough estimate per node
            .sum();
        let edges_size = self.edges.len() * 50;
        let zones_size = self.zones.len() * 200;
        let clusters_size = self.clusters.len() * 500;

        nodes_size + edges_size + zones_size + clusters_size
    }
}

/// Handles topology backup and restore operations
pub struct TopologyBackup {
    /// Storage directory
    storage_dir: PathBuf,
    /// Node ID
    _node_id: String,
    /// Compression enabled
    compress: bool,
}

impl TopologyBackup {
    /// Create new topology backup handler
    pub fn new(storage_dir: PathBuf, node_id: String) -> PersistenceResult<Self> {
        let backup_dir = storage_dir.join(&node_id).join("topology");
        std::fs::create_dir_all(&backup_dir)?;

        Ok(Self {
            storage_dir: backup_dir,
            _node_id: node_id,
            compress: true,
        })
    }

    /// Create a full backup
    pub async fn create_full_backup(
        &self,
        topology: &NetworkTopology
    ) -> PersistenceResult<PathBuf> {
        info!("Creating full topology backup");

        let mut backup = TopologyBackupData::new(BackupMode::Full);

        // Copy all nodes
        for (id, node) in topology.nodes() {
            backup.nodes.insert(id.clone(), node.clone());
        }

        // Copy all edges
        backup.edges = topology.edges().cloned().collect();

        // Copy zones
        for zone in topology.zones() {
            backup.zones.insert(zone.id.clone(), zone.clone());
        }

        // Copy clustering results
        if let Some(clusters) = topology.get_clusters() {
            backup.clusters = clusters.clone();
        }

        // Copy load balancing state
        if let Some(lb_state) = topology.get_load_balancing_state() {
            backup.load_balancing = Some(lb_state.clone());
        }

        // Add metadata
        backup.metadata.insert("node_count".to_string(), backup.nodes.len().to_string());
        backup.metadata.insert("edge_count".to_string(), backup.edges.len().to_string());
        backup.metadata.insert("zone_count".to_string(), backup.zones.len().to_string());

        self.save_backup(&backup, "full").await
    }

    /// Create an incremental backup
    pub async fn create_incremental_backup(
        &self,
        topology: &NetworkTopology,
        previous_backup: &TopologyBackupData,
    ) -> PersistenceResult<PathBuf> {
        info!("Creating incremental topology backup");

        let mut backup = TopologyBackupData::new(BackupMode::Incremental);

        // Find changed nodes
        for (id, node) in topology.nodes() {
            if let Some(prev_node) = previous_backup.nodes.get(id) {
                if !Self::nodes_equal(node, prev_node) {
                    backup.nodes.insert(id.clone(), node.clone());
                }
            } else {
                // New node
                backup.nodes.insert(id.clone(), node.clone());
            }
        }

        // Find changed edges
        let _current_edges: HashSet<_> = topology.edges()
            .map(|e| (e.source.clone(), e.target.clone()))
            .collect();
        let previous_edges: HashSet<_> = previous_backup.edges.iter()
            .map(|e| (e.source.clone(), e.target.clone()))
            .collect();

        for edge in topology.edges() {
            let key = (edge.source.clone(), edge.target.clone());
            if !previous_edges.contains(&key) {
                backup.edges.push(edge.clone());
            }
        }

        // Always include current zones and clusters (they're relatively small)
        for zone in topology.zones() {
            backup.zones.insert(zone.id.clone(), zone.clone());
        }

        if let Some(clusters) = topology.get_clusters() {
            backup.clusters = clusters.clone();
        }

        // Add metadata
        backup.metadata.insert("changed_nodes".to_string(), backup.nodes.len().to_string());
        backup.metadata.insert("changed_edges".to_string(), backup.edges.len().to_string());
        backup.metadata.insert("base_backup".to_string(), previous_backup.timestamp.to_rfc3339());

        self.save_backup(&backup, "incremental").await
    }

    /// Create an essential backup (minimal critical state)
    pub async fn create_essential_backup(
        &self,
        topology: &NetworkTopology
    ) -> PersistenceResult<PathBuf> {
        info!("Creating essential topology backup");

        let mut backup = TopologyBackupData::new(BackupMode::Essential);

        // Only include zone leaders and critical nodes
        for (id, node) in topology.nodes() {
            if node.metadata.get("role") == Some(&"leader".to_string()) ||
               node.peer_count() > 10 {  // High connectivity nodes
                backup.nodes.insert(id.clone(), node.clone());
            }
        }

        // Include all zones (critical for hierarchy)
        for zone in topology.zones() {
            backup.zones.insert(zone.id.clone(), zone.clone());
        }

        // Include clustering results
        if let Some(clusters) = topology.get_clusters() {
            backup.clusters = clusters.clone();
        }

        backup.metadata.insert("essential_nodes".to_string(), backup.nodes.len().to_string());

        self.save_backup(&backup, "essential").await
    }

    /// Restore topology from backup
    pub async fn restore_backup(
        &self,
        backup_path: &Path
    ) -> PersistenceResult<TopologyBackupData> {
        info!("Restoring topology from {:?}", backup_path);

        let data = std::fs::read(backup_path)?;

        let backup: TopologyBackupData = if self.compress {
            let decompressed = zstd::decode_all(&data[..])
                .map_err(|e| PersistenceError::Decompression(e.to_string()))?;
            bincode::deserialize(&decompressed)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?
        } else {
            bincode::deserialize(&data)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))?
        };

        info!("Restored topology with {} nodes, {} edges, {} zones",
              backup.nodes.len(), backup.edges.len(), backup.zones.len());

        Ok(backup)
    }

    /// List available backups
    pub fn list_backups(&self) -> PersistenceResult<Vec<BackupInfo>> {
        let mut backups = Vec::new();

        for entry in std::fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("backup") {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    let name = path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    backups.push(BackupInfo {
                        name,
                        path,
                        size: metadata.len(),
                        created: metadata.modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| chrono::Utc::now() - chrono::Duration::seconds(d.as_secs() as i64))
                            .unwrap_or_else(chrono::Utc::now),
                    });
                }
            }
        }

        backups.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(backups)
    }

    /// Delete old backups keeping the last N
    pub fn cleanup_old_backups(&self, keep_count: usize) -> PersistenceResult<u32> {
        let backups = self.list_backups()?;
        let mut deleted = 0;

        for backup in backups.iter().skip(keep_count) {
            if std::fs::remove_file(&backup.path).is_ok() {
                deleted += 1;
                info!("Deleted old backup: {}", backup.name);
            }
        }

        Ok(deleted)
    }

    /// Save backup to disk
    async fn save_backup(
        &self,
        backup: &TopologyBackupData,
        prefix: &str
    ) -> PersistenceResult<PathBuf> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.backup", prefix, timestamp);
        let path = self.storage_dir.join(filename);

        let serialized = bincode::serialize(backup)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let data = if self.compress {
            zstd::encode_all(&serialized[..], 3)
                .map_err(|e| PersistenceError::Compression(e.to_string()))?
        } else {
            serialized
        };

        std::fs::write(&path, data)?;

        info!("Saved {} backup to {:?} ({} bytes)",
              prefix, path, backup.size_estimate());

        Ok(path)
    }

    /// Check if two nodes are equal
    fn nodes_equal(a: &TopologyNode, b: &TopologyNode) -> bool {
        a.matrix_coord == b.matrix_coord &&
        a.gps_coord == b.gps_coord &&
        a.zone_id == b.zone_id &&
        a.peers == b.peers
    }
}

/// Backup information
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Backup name
    pub name: String,
    /// File path
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Creation timestamp
    pub created: chrono::DateTime<chrono::Utc>,
}

/// Load balancing state (placeholder for Sprint 1.4 integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingState {
    /// Strategy name
    pub strategy: String,
    /// Node loads
    pub node_loads: HashMap<String, f64>,
    /// Last rebalance time
    pub last_rebalance: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::coordinate::MatrixCoordinate;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let backup = TopologyBackup::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string()
        ).unwrap();

        // Create test topology
        let mut topology = NetworkTopology::new();

        let mut node1 = TopologyNode::new("node1".to_string(), MatrixCoordinate::new(0, 0, 0).unwrap());
        node1.add_peer("node2".to_string());
        topology.add_node(node1);

        let node2 = TopologyNode::new("node2".to_string(), MatrixCoordinate::new(1, 1, 1).unwrap());
        topology.add_node(node2);

        topology.add_edge("node1".to_string(), "node2".to_string(), 1.0);

        // Create full backup
        let backup_path = backup.create_full_backup(&topology).await.unwrap();
        assert!(backup_path.exists());
    }

    #[tokio::test]
    async fn test_backup_restore() {
        let temp_dir = TempDir::new().unwrap();
        let backup = TopologyBackup::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string()
        ).unwrap();

        // Create backup data
        let mut data = TopologyBackupData::new(BackupMode::Full);

        let node = TopologyNode::new("node1".to_string(), MatrixCoordinate::new(5, 5, 5).unwrap());
        data.nodes.insert("node1".to_string(), node);

        let edge = TopologyEdge {
            source: "node1".to_string(),
            target: "node2".to_string(),
            weight: 1.5,
            metadata: HashMap::new(),
        };
        data.edges.push(edge);

        // Save and restore
        let path = backup.save_backup(&data, "test").await.unwrap();
        let restored = backup.restore_backup(&path).await.unwrap();

        assert_eq!(restored.nodes.len(), 1);
        assert_eq!(restored.edges.len(), 1);
        assert!(restored.nodes.contains_key("node1"));
    }

    #[tokio::test]
    async fn test_incremental_backup() {
        let temp_dir = TempDir::new().unwrap();
        let backup = TopologyBackup::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string()
        ).unwrap();

        // Create initial topology
        let mut topology = NetworkTopology::new();
        let node1 = TopologyNode::new("node1".to_string(), MatrixCoordinate::new(0, 0, 0).unwrap());
        topology.add_node(node1);

        // Create full backup
        let full_path = backup.create_full_backup(&topology).await.unwrap();
        let full_data = backup.restore_backup(&full_path).await.unwrap();

        // Modify topology
        let node2 = TopologyNode::new("node2".to_string(), MatrixCoordinate::new(1, 1, 1).unwrap());
        topology.add_node(node2);

        // Create incremental backup
        let inc_path = backup.create_incremental_backup(&topology, &full_data).await.unwrap();
        let inc_data = backup.restore_backup(&inc_path).await.unwrap();

        // Incremental should only contain the new node
        assert_eq!(inc_data.nodes.len(), 1);
        assert!(inc_data.nodes.contains_key("node2"));
    }

    #[tokio::test]
    async fn test_essential_backup() {
        let temp_dir = TempDir::new().unwrap();
        let backup = TopologyBackup::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string()
        ).unwrap();

        let mut topology = NetworkTopology::new();

        // Add leader node
        let mut leader = TopologyNode::new("leader".to_string(), MatrixCoordinate::new(0, 0, 0).unwrap());
        leader.metadata.insert("role".to_string(), "leader".to_string());
        topology.add_node(leader);

        // Add regular nodes
        for i in 1..5 {
            let node = TopologyNode::new(format!("node{}", i), MatrixCoordinate::new(i, i, i).unwrap());
            topology.add_node(node);
        }

        // Create essential backup
        let path = backup.create_essential_backup(&topology).await.unwrap();
        let data = backup.restore_backup(&path).await.unwrap();

        // Should only contain the leader
        assert_eq!(data.nodes.len(), 1);
        assert!(data.nodes.contains_key("leader"));
    }

    #[tokio::test]
    async fn test_backup_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let backup = TopologyBackup::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string()
        ).unwrap();

        // Create multiple backups
        for i in 0..5 {
            let data = TopologyBackupData::new(BackupMode::Full);
            backup.save_backup(&data, &format!("test{}", i)).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // List backups
        let list = backup.list_backups().unwrap();
        assert_eq!(list.len(), 5);

        // Cleanup keeping only 2
        let deleted = backup.cleanup_old_backups(2).unwrap();
        assert_eq!(deleted, 3);

        // Verify only 2 remain
        let list = backup.list_backups().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_backup_size_estimate() {
        let mut data = TopologyBackupData::new(BackupMode::Full);

        for i in 0..10 {
            let node = TopologyNode::new(format!("node{}", i), MatrixCoordinate::new(i, i, i).unwrap());
            data.nodes.insert(format!("node{}", i), node);
        }

        for i in 0..5 {
            data.edges.push(TopologyEdge {
                source: format!("node{}", i),
                target: format!("node{}", i + 1),
                weight: 1.0,
                metadata: HashMap::new(),
            });
        }

        let size = data.size_estimate();
        assert!(size > 0);
        assert!(size < 10000); // Reasonable upper bound
    }

    #[test]
    fn test_nodes_equal() {
        let coord = MatrixCoordinate::new(1, 2, 3).unwrap();
        let mut node1 = TopologyNode::new("test".to_string(), coord.clone());
        let mut node2 = TopologyNode::new("test".to_string(), coord.clone());

        assert!(TopologyBackup::nodes_equal(&node1, &node2));

        node1.add_peer("peer1".to_string());
        assert!(!TopologyBackup::nodes_equal(&node1, &node2));

        node2.add_peer("peer1".to_string());
        assert!(TopologyBackup::nodes_equal(&node1, &node2));
    }
}