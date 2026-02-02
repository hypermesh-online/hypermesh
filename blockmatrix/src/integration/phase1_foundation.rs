//! Phase 1 Integration: Block-MATRIX Foundation
//!
//! This module provides the unified integration layer for all Phase 1 components:
//! - Matrix Coordinate System (Sprint 1.1)
//! - Tensor Operations Library (Sprint 1.2)
//! - Every-Node-Blockchain (Sprint 1.3)
//! - Geospatial Integration (Sprint 1.4)
//! - Matrix Persistence Layer (Sprint 1.5)

use crate::matrix::{MatrixCoordinate, CoordinateError};
use crate::blockchain::{NodeBlockchain, ChainStateManager, BlockPropagator, PropagationStrategy, Block};
use crate::persistence::{PersistenceManager, PersistenceConfig, RecoveryManager};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use thiserror::Error;

/// Phase 1 Foundation errors
#[derive(Debug, Error)]
pub enum Phase1Error {
    #[error("Matrix coordinate error: {0}")]
    Coordinate(#[from] CoordinateError),

    #[error("Blockchain error: {0}")]
    Blockchain(String),

    #[error("Persistence error: {0}")]
    Persistence(#[from] crate::persistence::PersistenceError),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Phase1Result<T> = Result<T, Phase1Error>;

/// Configuration for Phase 1 Matrix Foundation
#[derive(Debug, Clone)]
pub struct MatrixFoundationConfig {
    /// Base storage directory for all persistence
    pub storage_path: PathBuf,

    /// Default propagation strategy for blockchains
    pub propagation_strategy: PropagationStrategy,

    /// Enable automatic persistence snapshots
    pub enable_snapshots: bool,

    /// Snapshot interval in seconds
    pub snapshot_interval_secs: u64,

    /// Maximum number of nodes in the network
    pub max_nodes: usize,
}

impl Default for MatrixFoundationConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("./data/blockmatrix"),
            propagation_strategy: PropagationStrategy::Broadcast,
            enable_snapshots: true,
            snapshot_interval_secs: 300, // 5 minutes
            max_nodes: 10000,
        }
    }
}

/// Node information in the matrix network
#[derive(Clone)]
pub struct MatrixNode {
    /// Node identifier
    pub node_id: String,

    /// Matrix coordinate position
    pub coordinate: MatrixCoordinate,

    /// Node blockchain reference
    pub blockchain: Arc<RwLock<NodeBlockchain>>,

    /// Block propagator reference
    pub propagator: Arc<BlockPropagator>,
}

impl MatrixNode {
    /// Create a new matrix node
    pub fn new(
        node_id: String,
        coordinate: MatrixCoordinate,
        blockchain: NodeBlockchain,
        propagator: BlockPropagator,
    ) -> Self {
        Self {
            node_id,
            coordinate,
            blockchain: Arc::new(RwLock::new(blockchain)),
            propagator: Arc::new(propagator),
        }
    }

    /// Get the node's coordinate
    pub fn coordinate(&self) -> &MatrixCoordinate {
        &self.coordinate
    }

    /// Get node ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}

/// Phase 1 Matrix Foundation - Unified Integration Layer
pub struct MatrixFoundation {
    /// Configuration
    config: MatrixFoundationConfig,

    /// All nodes in the matrix network (node_id -> MatrixNode)
    nodes: Arc<RwLock<HashMap<String, MatrixNode>>>,

    /// State managers for each node (node_id -> ChainStateManager)
    state_managers: Arc<RwLock<HashMap<String, Arc<ChainStateManager>>>>,

    /// Global persistence manager
    persistence: Arc<PersistenceManager>,

    /// Recovery manager for disaster recovery
    recovery: Arc<RecoveryManager>,
}

impl MatrixFoundation {
    /// Create a new Matrix Foundation instance
    pub async fn new(config: MatrixFoundationConfig) -> Phase1Result<Self> {
        // Create storage directory if it doesn't exist
        tokio::fs::create_dir_all(&config.storage_path).await?;

        // Initialize persistence manager
        let persistence_config = PersistenceConfig {
            storage_dir: config.storage_path.clone(),
            enable_compression: true,
            compression_level: 3,
            matrix_format: crate::persistence::SerializationFormat::Bincode,
            snapshot_schedule: crate::persistence::SnapshotSchedule::TimeBased {
                interval_secs: config.snapshot_interval_secs,
            },
            max_snapshots: 10,
            max_backups: 5,
            enable_background: false, // Manual control for now
            background_interval_secs: config.snapshot_interval_secs,
            disk_warning_threshold: 100 * 1024 * 1024,
            disk_error_threshold: 10 * 1024 * 1024,
        };

        let persistence = Arc::new(
            PersistenceManager::new(persistence_config, "foundation".to_string()).await?
        );

        // Initialize recovery manager
        let recovery = Arc::new(RecoveryManager::new(
            config.storage_path.clone(),
            "foundation".to_string()
        ));

        Ok(Self {
            config,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            state_managers: Arc::new(RwLock::new(HashMap::new())),
            persistence,
            recovery,
        })
    }

    /// Add a node to the matrix network
    pub async fn add_node(
        &self,
        node_id: String,
        coordinate: MatrixCoordinate,
    ) -> Phase1Result<String> {
        let nodes = self.nodes.read().await;
        if nodes.len() >= self.config.max_nodes {
            return Err(Phase1Error::Configuration(
                format!("Maximum node count ({}) reached", self.config.max_nodes)
            ));
        }
        drop(nodes);

        // Create node storage path
        let node_storage = self.config.storage_path.join(&node_id);
        tokio::fs::create_dir_all(&node_storage).await?;

        // Create blockchain for this node
        let blockchain = NodeBlockchain::new(coordinate.clone());

        // Create state manager
        let state_manager = Arc::new(ChainStateManager::new(
            coordinate.clone(),
            &node_storage,
        ));
        state_manager.initialize().await
            .map_err(|e| Phase1Error::Blockchain(e))?;

        // Create propagator
        let propagator = BlockPropagator::new(
            coordinate.clone(),
            self.config.propagation_strategy.clone(),
        );

        // Create matrix node
        let matrix_node = MatrixNode::new(
            node_id.clone(),
            coordinate,
            blockchain,
            propagator,
        );

        // Store node and state manager
        let mut nodes = self.nodes.write().await;
        let mut state_managers = self.state_managers.write().await;

        nodes.insert(node_id.clone(), matrix_node);
        state_managers.insert(node_id.clone(), state_manager);

        Ok(node_id)
    }

    /// Remove a node from the matrix network
    pub async fn remove_node(&self, node_id: &str) -> Phase1Result<()> {
        let mut nodes = self.nodes.write().await;
        let mut state_managers = self.state_managers.write().await;

        nodes.remove(node_id)
            .ok_or_else(|| Phase1Error::NodeNotFound(node_id.to_string()))?;
        state_managers.remove(node_id);

        Ok(())
    }

    /// Get a node by ID
    pub async fn get_node(&self, node_id: &str) -> Phase1Result<MatrixNode> {
        let nodes = self.nodes.read().await;
        nodes.get(node_id)
            .cloned()
            .ok_or_else(|| Phase1Error::NodeNotFound(node_id.to_string()))
    }

    /// Get all nodes in the network
    pub async fn get_all_nodes(&self) -> Vec<MatrixNode> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    /// Get node count
    pub async fn node_count(&self) -> usize {
        let nodes = self.nodes.read().await;
        nodes.len()
    }

    /// Add a block to a node's blockchain
    pub async fn add_block(&self, node_id: &str, data: Vec<u8>) -> Phase1Result<Block> {
        let node = self.get_node(node_id).await?;
        let mut blockchain = node.blockchain.write().await;

        blockchain.add_block_with_data(data).await
            .map_err(|e| Phase1Error::Blockchain(e))
    }

    /// Get blockchain height for a node
    pub async fn get_blockchain_height(&self, node_id: &str) -> Phase1Result<u64> {
        let node = self.get_node(node_id).await?;
        let blockchain = node.blockchain.read().await;
        Ok(blockchain.get_height().await)
    }

    /// Find k nearest neighbors to a coordinate
    pub async fn find_k_nearest_nodes(
        &self,
        center: &MatrixCoordinate,
        k: usize,
    ) -> Vec<MatrixNode> {
        use crate::matrix::find_k_nearest;

        let nodes = self.nodes.read().await;
        let coordinates: Vec<MatrixCoordinate> = nodes.values()
            .map(|n| n.coordinate.clone())
            .collect();

        let nearest = find_k_nearest(center, &coordinates, k);

        // Map coordinates back to nodes
        nearest.into_iter()
            .filter_map(|(coord, _dist)| {
                nodes.values()
                    .find(|n| n.coordinate == coord)
                    .cloned()
            })
            .collect()
    }

    /// Find all neighbors within a radius
    pub async fn find_neighbors_in_radius(
        &self,
        center: &MatrixCoordinate,
        radius: f64,
    ) -> Vec<MatrixNode> {
        use crate::matrix::find_neighbors;

        let nodes = self.nodes.read().await;
        let coordinates: Vec<MatrixCoordinate> = nodes.values()
            .map(|n| n.coordinate.clone())
            .collect();

        let neighbors = find_neighbors(center, &coordinates, radius);

        // Map coordinates back to nodes
        neighbors.into_iter()
            .filter_map(|coord| {
                nodes.values()
                    .find(|n| n.coordinate == coord)
                    .cloned()
            })
            .collect()
    }

    /// Save the entire network state
    pub async fn save_network_state(&self) -> Phase1Result<String> {
        let snapshot_id = self.persistence.create_snapshot().await?;
        Ok(snapshot_id)
    }

    /// Recover network state from snapshot
    pub async fn recover_network_state(&mut self) -> Phase1Result<()> {
        // Note: This is a simplified version - full recovery would need
        // to be implemented with proper node registry persistence
        let recovery_manager = Arc::get_mut(&mut self.recovery)
            .ok_or_else(|| Phase1Error::Configuration(
                "Cannot recover with multiple references".to_string()
            ))?;

        let report = recovery_manager.recover_all().await?;

        // Check if recovery was successful
        use crate::persistence::RecoveryStatus;
        match report.status {
            RecoveryStatus::Completed => Ok(()),
            RecoveryStatus::NotNeeded => Ok(()),
            RecoveryStatus::Partial => {
                tracing::warn!("Partial recovery: {} errors", report.errors.len());
                Ok(())
            }
            RecoveryStatus::Failed | RecoveryStatus::InProgress => {
                Err(Phase1Error::Persistence(
                    crate::persistence::PersistenceError::RecoveryFailed(
                        format!("Recovery failed: {:?}, {} errors", report.status, report.errors.len())
                    )
                ))
            }
        }?;

        Ok(())
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> NetworkStats {
        let nodes = self.nodes.read().await;
        let node_count = nodes.len();

        let coordinates: Vec<MatrixCoordinate> = nodes.values()
            .map(|n| n.coordinate.clone())
            .collect();

        // Calculate network dimensions
        let (min_x, max_x, min_y, max_y, min_z, max_z) = if coordinates.is_empty() {
            (0, 0, 0, 0, 0, 0)
        } else {
            coordinates.iter().fold(
                (i64::MAX, i64::MIN, i64::MAX, i64::MIN, i64::MAX, i64::MIN),
                |(min_x, max_x, min_y, max_y, min_z, max_z), coord| {
                    (
                        min_x.min(coord.x),
                        max_x.max(coord.x),
                        min_y.min(coord.y),
                        max_y.max(coord.y),
                        min_z.min(coord.z),
                        max_z.max(coord.z),
                    )
                }
            )
        };

        NetworkStats {
            node_count,
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        }
    }

    /// Shutdown the foundation cleanly
    pub async fn shutdown(&self) -> Phase1Result<()> {
        // Save final snapshot
        if self.config.enable_snapshots {
            let _ = self.save_network_state().await;
        }

        // Shutdown persistence manager
        self.persistence.shutdown().await?;

        Ok(())
    }
}

/// Network statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub node_count: usize,
    pub min_x: i64,
    pub max_x: i64,
    pub min_y: i64,
    pub max_y: i64,
    pub min_z: i64,
    pub max_z: i64,
}

impl NetworkStats {
    /// Get network volume (bounding box)
    pub fn volume(&self) -> u64 {
        let width = (self.max_x - self.min_x).max(1) as u64;
        let height = (self.max_y - self.min_y).max(1) as u64;
        let depth = (self.max_z - self.min_z).max(1) as u64;
        width * height * depth
    }

    /// Get network density (nodes per cubic unit)
    pub fn density(&self) -> f64 {
        if self.node_count == 0 {
            return 0.0;
        }
        self.node_count as f64 / self.volume() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_foundation() -> (MatrixFoundation, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = MatrixFoundationConfig {
            storage_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let foundation = MatrixFoundation::new(config).await.unwrap();
        (foundation, temp_dir)
    }

    #[tokio::test]
    async fn test_foundation_creation() {
        let (foundation, _temp_dir) = create_test_foundation().await;
        assert_eq!(foundation.node_count().await, 0);
    }

    #[tokio::test]
    async fn test_add_node() {
        let (foundation, _temp_dir) = create_test_foundation().await;

        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        let node_id = foundation.add_node("node1".to_string(), coord).await.unwrap();

        assert_eq!(node_id, "node1");
        assert_eq!(foundation.node_count().await, 1);
    }

    #[tokio::test]
    async fn test_add_multiple_nodes() {
        let (foundation, _temp_dir) = create_test_foundation().await;

        for i in 0..10 {
            let coord = MatrixCoordinate::new(i, i, i).unwrap();
            foundation.add_node(format!("node{}", i), coord).await.unwrap();
        }

        assert_eq!(foundation.node_count().await, 10);
    }

    #[tokio::test]
    async fn test_get_node() {
        let (foundation, _temp_dir) = create_test_foundation().await;

        let coord = MatrixCoordinate::new(10, 20, 30).unwrap();
        foundation.add_node("test_node".to_string(), coord.clone()).await.unwrap();

        let node = foundation.get_node("test_node").await.unwrap();
        assert_eq!(node.coordinate, coord);
        assert_eq!(node.node_id, "test_node");
    }

    #[tokio::test]
    async fn test_remove_node() {
        let (foundation, _temp_dir) = create_test_foundation().await;

        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        foundation.add_node("node1".to_string(), coord).await.unwrap();
        assert_eq!(foundation.node_count().await, 1);

        foundation.remove_node("node1").await.unwrap();
        assert_eq!(foundation.node_count().await, 0);
    }

    #[tokio::test]
    async fn test_add_block() {
        let (foundation, _temp_dir) = create_test_foundation().await;

        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        foundation.add_node("node1".to_string(), coord).await.unwrap();

        let block = foundation.add_block_with_data("node1", b"test data".to_vec()).await.unwrap();
        assert_eq!(block.asset_count(), 1); // Block should contain one asset

        let height = foundation.get_blockchain_height("node1").await.unwrap();
        assert_eq!(height, 1); // Genesis + 1 block
    }

    #[tokio::test]
    async fn test_find_k_nearest() {
        let (foundation, _temp_dir) = create_test_foundation().await;

        // Add nodes in a grid
        for x in 0..5 {
            for y in 0..5 {
                let coord = MatrixCoordinate::new(x, y, 0).unwrap();
                foundation.add_node(format!("node_{}_{}", x, y), coord).await.unwrap();
            }
        }

        let center = MatrixCoordinate::new(2, 2, 0).unwrap();
        let nearest = foundation.find_k_nearest_nodes(&center, 5).await;

        assert_eq!(nearest.len(), 5);
    }

    #[tokio::test]
    async fn test_find_neighbors_in_radius() {
        let (foundation, _temp_dir) = create_test_foundation().await;

        // Add nodes
        let coords = vec![
            (0, 0, 0),
            (1, 0, 0),
            (0, 1, 0),
            (10, 10, 10),
        ];

        for (i, (x, y, z)) in coords.iter().enumerate() {
            let coord = MatrixCoordinate::new(*x, *y, *z).unwrap();
            foundation.add_node(format!("node{}", i), coord).await.unwrap();
        }

        let center = MatrixCoordinate::new(0, 0, 0).unwrap();
        let neighbors = foundation.find_neighbors_in_radius(&center, 2.0).await;

        // Should find node at (1,0,0) and (0,1,0), but not (10,10,10)
        assert!(neighbors.len() >= 2);
        assert!(neighbors.len() < 4);
    }

    #[tokio::test]
    async fn test_network_stats() {
        let (foundation, _temp_dir) = create_test_foundation().await;

        // Add nodes in a 10x10x10 cube
        for x in 0..10 {
            for y in 0..10 {
                for z in 0..10 {
                    let coord = MatrixCoordinate::new(x, y, z).unwrap();
                    foundation.add_node(format!("node_{}_{}", x, y * 10 + z), coord).await.unwrap();
                }
            }
        }

        let stats = foundation.get_network_stats().await;
        assert_eq!(stats.node_count, 1000);
        assert_eq!(stats.min_x, 0);
        assert_eq!(stats.max_x, 9);
        assert!(stats.density() > 0.0);
    }
}
