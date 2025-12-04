//! Geographic clustering algorithms
//!
//! Implements K-means, DBSCAN, and hierarchical clustering for
//! organizing nodes into geographic groups.

use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::geospatial::hierarchy::{GeographicZone, GeographicLevel, GeographicBounds};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Clustering algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ClusteringAlgorithm {
    /// K-means clustering with fixed number of clusters
    KMeans,
    /// Density-based spatial clustering
    DBSCAN,
    /// Hierarchical clustering based on geographic zones
    Hierarchical,
}

/// A cluster of nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    /// Unique cluster ID
    pub id: String,
    /// Cluster centroid (average position)
    pub centroid: MatrixCoordinate,
    /// Member node coordinates
    pub members: Vec<MatrixCoordinate>,
    /// Optional geographic zone association
    pub zone_id: Option<String>,
    /// Cluster metadata
    pub metadata: HashMap<String, String>,
}

impl Cluster {
    /// Create a new cluster
    pub fn new(id: String) -> Self {
        Self {
            id,
            centroid: MatrixCoordinate::origin(),
            members: Vec::new(),
            zone_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a member to the cluster
    pub fn add_member(&mut self, coord: MatrixCoordinate) {
        self.members.push(coord);
        self.update_centroid();
    }

    /// Remove a member from the cluster
    pub fn remove_member(&mut self, coord: &MatrixCoordinate) -> bool {
        if let Some(pos) = self.members.iter().position(|m| m == coord) {
            self.members.remove(pos);
            self.update_centroid();
            true
        } else {
            false
        }
    }

    /// Update the centroid based on current members
    fn update_centroid(&mut self) {
        if self.members.is_empty() {
            self.centroid = MatrixCoordinate::origin();
            return;
        }

        let sum_x: i64 = self.members.iter().map(|m| m.x).sum();
        let sum_y: i64 = self.members.iter().map(|m| m.y).sum();
        let sum_z: i64 = self.members.iter().map(|m| m.z).sum();
        let count = self.members.len() as i64;

        self.centroid = MatrixCoordinate::new(
            sum_x / count,
            sum_y / count,
            sum_z / count,
        ).unwrap_or_else(|_| MatrixCoordinate::origin());
    }

    /// Get the size of the cluster
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Check if cluster is empty
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Calculate cluster cohesion (average distance to centroid)
    pub fn cohesion(&self) -> f64 {
        if self.members.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.members.iter()
            .map(|m| self.centroid.euclidean_distance(m))
            .sum();

        sum / self.members.len() as f64
    }
}

/// Cluster quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetrics {
    /// Average cohesion across all clusters
    pub avg_cohesion: f64,
    /// Average separation between clusters
    pub avg_separation: f64,
    /// Davies-Bouldin index (lower is better)
    pub davies_bouldin_index: f64,
    /// Silhouette coefficient (-1 to 1, higher is better)
    pub silhouette_coefficient: f64,
}

/// Geographic clustering engine
#[derive(Debug)]
pub struct GeographicClustering {
    /// Current clusters
    clusters: HashMap<String, Cluster>,
    /// Node to cluster mapping
    node_clusters: HashMap<MatrixCoordinate, String>,
}

impl GeographicClustering {
    /// Create a new clustering engine
    pub fn new() -> Self {
        Self {
            clusters: HashMap::new(),
            node_clusters: HashMap::new(),
        }
    }

    /// Perform K-means clustering
    pub fn kmeans(&mut self, nodes: &[MatrixCoordinate], k: usize, max_iterations: usize) {
        if nodes.is_empty() || k == 0 {
            return;
        }

        // Clear existing clusters
        self.clusters.clear();
        self.node_clusters.clear();

        // Initialize k clusters with random centroids
        let mut centroids = self.initialize_kmeans_centroids(nodes, k);

        for iteration in 0..max_iterations {
            // Create empty clusters
            let mut new_clusters: Vec<Cluster> = (0..k)
                .map(|i| Cluster::new(format!("kmeans_{}", i)))
                .collect();

            // Assign nodes to nearest centroid
            for node in nodes {
                let nearest_idx = self.find_nearest_centroid(node, &centroids);
                new_clusters[nearest_idx].add_member(*node);
            }

            // Update centroids
            let mut converged = true;
            for (i, cluster) in new_clusters.iter().enumerate() {
                if !cluster.is_empty() {
                    let new_centroid = cluster.centroid;
                    if new_centroid != centroids[i] {
                        converged = false;
                        centroids[i] = new_centroid;
                    }
                }
            }

            // Store clusters
            for cluster in new_clusters {
                if !cluster.is_empty() {
                    for member in &cluster.members {
                        self.node_clusters.insert(*member, cluster.id.clone());
                    }
                    self.clusters.insert(cluster.id.clone(), cluster);
                }
            }

            // Check for convergence
            if converged {
                break;
            }
        }
    }

    /// Initialize K-means centroids using K-means++ algorithm
    fn initialize_kmeans_centroids(&self, nodes: &[MatrixCoordinate], k: usize) -> Vec<MatrixCoordinate> {
        let mut centroids = Vec::with_capacity(k);

        // First centroid: random node
        centroids.push(nodes[0]);

        // Remaining centroids: chosen with probability proportional to squared distance
        for _ in 1..k.min(nodes.len()) {
            let mut max_min_dist = 0.0;
            let mut best_node = nodes[0];

            for node in nodes {
                let min_dist = centroids.iter()
                    .map(|c| c.euclidean_distance(node))
                    .fold(f64::MAX, f64::min);

                if min_dist > max_min_dist {
                    max_min_dist = min_dist;
                    best_node = *node;
                }
            }

            centroids.push(best_node);
        }

        centroids
    }

    /// Find the nearest centroid to a node
    fn find_nearest_centroid(&self, node: &MatrixCoordinate, centroids: &[MatrixCoordinate]) -> usize {
        let mut min_dist = f64::MAX;
        let mut nearest_idx = 0;

        for (i, centroid) in centroids.iter().enumerate() {
            let dist = centroid.euclidean_distance(node);
            if dist < min_dist {
                min_dist = dist;
                nearest_idx = i;
            }
        }

        nearest_idx
    }

    /// Perform DBSCAN clustering
    pub fn dbscan(&mut self, nodes: &[MatrixCoordinate], eps: f64, min_points: usize) {
        if nodes.is_empty() {
            return;
        }

        // Clear existing clusters
        self.clusters.clear();
        self.node_clusters.clear();

        let mut visited = HashSet::new();
        let mut cluster_id = 0;

        for node in nodes {
            if visited.contains(node) {
                continue;
            }

            visited.insert(*node);

            // Find neighbors within eps distance
            let neighbors = self.find_neighbors(node, nodes, eps);

            if neighbors.len() >= min_points {
                // Start a new cluster
                let mut cluster = Cluster::new(format!("dbscan_{}", cluster_id));
                cluster_id += 1;

                // Expand cluster
                self.expand_cluster(
                    node,
                    &neighbors,
                    &mut cluster,
                    nodes,
                    eps,
                    min_points,
                    &mut visited,
                );

                // Store cluster
                for member in &cluster.members {
                    self.node_clusters.insert(*member, cluster.id.clone());
                }
                self.clusters.insert(cluster.id.clone(), cluster);
            }
        }
    }

    /// Find neighbors within epsilon distance
    fn find_neighbors(&self, node: &MatrixCoordinate, nodes: &[MatrixCoordinate], eps: f64) -> Vec<MatrixCoordinate> {
        nodes.iter()
            .filter(|n| {
                let dist = node.euclidean_distance(n);
                dist <= eps && **n != *node
            })
            .cloned()
            .collect()
    }

    /// Expand a DBSCAN cluster
    fn expand_cluster(
        &self,
        node: &MatrixCoordinate,
        neighbors: &[MatrixCoordinate],
        cluster: &mut Cluster,
        nodes: &[MatrixCoordinate],
        eps: f64,
        min_points: usize,
        visited: &mut HashSet<MatrixCoordinate>,
    ) {
        cluster.add_member(*node);

        let mut queue = neighbors.to_vec();
        let mut i = 0;

        while i < queue.len() {
            let neighbor = queue[i];
            i += 1;

            if !visited.contains(&neighbor) {
                visited.insert(neighbor);

                let neighbor_neighbors = self.find_neighbors(&neighbor, nodes, eps);
                if neighbor_neighbors.len() >= min_points {
                    // Add new neighbors to queue
                    for nn in neighbor_neighbors {
                        if !queue.contains(&nn) {
                            queue.push(nn);
                        }
                    }
                }
            }

            // Add to cluster if not already in one
            if !cluster.members.contains(&neighbor) {
                cluster.add_member(neighbor);
            }
        }
    }

    /// Perform hierarchical clustering based on geographic zones
    pub fn hierarchical(&mut self, nodes: &[MatrixCoordinate], zones: &[GeographicZone]) {
        // Clear existing clusters
        self.clusters.clear();
        self.node_clusters.clear();

        // Create clusters for each zone
        for zone in zones {
            let mut cluster = Cluster::new(zone.id.clone());
            cluster.zone_id = Some(zone.id.clone());

            // Add metadata
            cluster.metadata.insert("zone_name".to_string(), zone.name.clone());
            cluster.metadata.insert("zone_level".to_string(), format!("{:?}", zone.level));

            self.clusters.insert(cluster.id.clone(), cluster);
        }

        // Assign nodes to zones based on proximity to zone centers
        // (In a real implementation, this would use GPS conversion)
        for node in nodes {
            // Find nearest zone cluster
            let mut min_dist = f64::MAX;
            let mut nearest_zone_id = String::new();

            for cluster in self.clusters.values() {
                let dist = node.euclidean_distance(&cluster.centroid);
                if dist < min_dist {
                    min_dist = dist;
                    nearest_zone_id = cluster.id.clone();
                }
            }

            if !nearest_zone_id.is_empty() {
                if let Some(cluster) = self.clusters.get_mut(&nearest_zone_id) {
                    cluster.add_member(*node);
                    self.node_clusters.insert(*node, nearest_zone_id);
                }
            }
        }
    }

    /// Calculate clustering quality metrics
    pub fn calculate_metrics(&self) -> ClusterMetrics {
        let clusters: Vec<&Cluster> = self.clusters.values().collect();

        // Calculate average cohesion
        let avg_cohesion = if clusters.is_empty() {
            0.0
        } else {
            let sum: f64 = clusters.iter().map(|c| c.cohesion()).sum();
            sum / clusters.len() as f64
        };

        // Calculate average separation
        let avg_separation = self.calculate_avg_separation(&clusters);

        // Calculate Davies-Bouldin index
        let davies_bouldin_index = self.calculate_davies_bouldin(&clusters);

        // Calculate Silhouette coefficient
        let silhouette_coefficient = self.calculate_silhouette();

        ClusterMetrics {
            avg_cohesion,
            avg_separation,
            davies_bouldin_index,
            silhouette_coefficient,
        }
    }

    /// Calculate average separation between clusters
    fn calculate_avg_separation(&self, clusters: &[&Cluster]) -> f64 {
        if clusters.len() < 2 {
            return 0.0;
        }

        let mut total_separation = 0.0;
        let mut count = 0;

        for i in 0..clusters.len() {
            for j in (i + 1)..clusters.len() {
                let dist = clusters[i].centroid.euclidean_distance(&clusters[j].centroid);
                total_separation += dist;
                count += 1;
            }
        }

        if count > 0 {
            total_separation / count as f64
        } else {
            0.0
        }
    }

    /// Calculate Davies-Bouldin index
    fn calculate_davies_bouldin(&self, clusters: &[&Cluster]) -> f64 {
        if clusters.len() < 2 {
            return 0.0;
        }

        let mut db_sum = 0.0;

        for i in 0..clusters.len() {
            let mut max_ratio = 0.0;

            for j in 0..clusters.len() {
                if i != j {
                    let cohesion_i = clusters[i].cohesion();
                    let cohesion_j = clusters[j].cohesion();
                    let separation = clusters[i].centroid.euclidean_distance(&clusters[j].centroid);

                    if separation > 0.0 {
                        let ratio = (cohesion_i + cohesion_j) / separation;
                        max_ratio = f64::max(max_ratio, ratio);
                    }
                }
            }

            db_sum += max_ratio;
        }

        db_sum / clusters.len() as f64
    }

    /// Calculate Silhouette coefficient
    fn calculate_silhouette(&self) -> f64 {
        if self.node_clusters.is_empty() {
            return 0.0;
        }

        let mut total_silhouette = 0.0;

        for (node, cluster_id) in &self.node_clusters {
            let a = self.calculate_intra_cluster_distance(node, cluster_id);
            let b = self.calculate_nearest_cluster_distance(node, cluster_id);

            let s = if a.max(b) > 0.0 {
                (b - a) / a.max(b)
            } else {
                0.0
            };

            total_silhouette += s;
        }

        total_silhouette / self.node_clusters.len() as f64
    }

    /// Calculate average distance to other nodes in same cluster
    fn calculate_intra_cluster_distance(&self, node: &MatrixCoordinate, cluster_id: &str) -> f64 {
        if let Some(cluster) = self.clusters.get(cluster_id) {
            if cluster.members.len() <= 1 {
                return 0.0;
            }

            let sum: f64 = cluster.members.iter()
                .filter(|m| *m != node)
                .map(|m| node.euclidean_distance(m))
                .sum();

            sum / (cluster.members.len() - 1) as f64
        } else {
            0.0
        }
    }

    /// Calculate average distance to nearest cluster
    fn calculate_nearest_cluster_distance(&self, node: &MatrixCoordinate, cluster_id: &str) -> f64 {
        let mut min_avg_dist = f64::MAX;

        for (other_id, other_cluster) in &self.clusters {
            if other_id != cluster_id && !other_cluster.is_empty() {
                let sum: f64 = other_cluster.members.iter()
                    .map(|m| node.euclidean_distance(m))
                    .sum();
                let avg = sum / other_cluster.members.len() as f64;
                min_avg_dist = min_avg_dist.min(avg);
            }
        }

        if min_avg_dist == f64::MAX {
            0.0
        } else {
            min_avg_dist
        }
    }

    /// Get all clusters
    pub fn get_clusters(&self) -> Vec<&Cluster> {
        self.clusters.values().collect()
    }

    /// Get cluster for a specific node
    pub fn get_node_cluster(&self, node: &MatrixCoordinate) -> Option<&Cluster> {
        self.node_clusters.get(node)
            .and_then(|id| self.clusters.get(id))
    }

    /// Update cluster dynamically when nodes join
    pub fn add_node(&mut self, node: MatrixCoordinate, cluster_id: Option<String>) {
        let cluster_id = cluster_id.unwrap_or_else(|| {
            // Find nearest cluster
            self.find_nearest_cluster(&node)
                .unwrap_or_else(|| format!("dynamic_{}", self.clusters.len()))
        });

        // Add to cluster or create new one
        let cluster = self.clusters.entry(cluster_id.clone())
            .or_insert_with(|| Cluster::new(cluster_id.clone()));
        cluster.add_member(node);
        self.node_clusters.insert(node, cluster_id);
    }

    /// Update cluster dynamically when nodes leave
    pub fn remove_node(&mut self, node: &MatrixCoordinate) -> Option<String> {
        if let Some(cluster_id) = self.node_clusters.remove(node) {
            if let Some(cluster) = self.clusters.get_mut(&cluster_id) {
                cluster.remove_member(node);

                // Remove empty clusters
                if cluster.is_empty() {
                    self.clusters.remove(&cluster_id);
                }
            }
            Some(cluster_id)
        } else {
            None
        }
    }

    /// Find nearest cluster to a node
    fn find_nearest_cluster(&self, node: &MatrixCoordinate) -> Option<String> {
        let mut min_dist = f64::MAX;
        let mut nearest_id = None;

        for (id, cluster) in &self.clusters {
            if !cluster.is_empty() {
                let dist = cluster.centroid.euclidean_distance(node);
                if dist < min_dist {
                    min_dist = dist;
                    nearest_id = Some(id.clone());
                }
            }
        }

        nearest_id
    }
}

impl Default for GeographicClustering {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_nodes() -> Vec<MatrixCoordinate> {
        vec![
            // Cluster 1 (around origin)
            MatrixCoordinate::new(0, 0, 0).unwrap(),
            MatrixCoordinate::new(1, 1, 0).unwrap(),
            MatrixCoordinate::new(-1, 1, 0).unwrap(),
            MatrixCoordinate::new(1, -1, 0).unwrap(),
            // Cluster 2 (around 10,10)
            MatrixCoordinate::new(10, 10, 0).unwrap(),
            MatrixCoordinate::new(11, 10, 0).unwrap(),
            MatrixCoordinate::new(10, 11, 0).unwrap(),
            MatrixCoordinate::new(9, 10, 0).unwrap(),
            // Cluster 3 (around -10,-10)
            MatrixCoordinate::new(-10, -10, 0).unwrap(),
            MatrixCoordinate::new(-11, -10, 0).unwrap(),
            MatrixCoordinate::new(-10, -11, 0).unwrap(),
            MatrixCoordinate::new(-9, -10, 0).unwrap(),
        ]
    }

    #[test]
    fn test_cluster_operations() {
        let mut cluster = Cluster::new("test".to_string());

        // Add members
        cluster.add_member(MatrixCoordinate::new(0, 0, 0).unwrap());
        cluster.add_member(MatrixCoordinate::new(2, 0, 0).unwrap());
        cluster.add_member(MatrixCoordinate::new(0, 2, 0).unwrap());

        assert_eq!(cluster.size(), 3);
        assert!(!cluster.is_empty());

        // Centroid should be average
        assert_eq!(cluster.centroid.x, 0); // (0+2+0)/3 = 0
        assert_eq!(cluster.centroid.y, 0); // (0+0+2)/3 = 0

        // Test cohesion
        let cohesion = cluster.cohesion();
        assert!(cohesion > 0.0);

        // Remove member
        let coord = MatrixCoordinate::new(0, 0, 0).unwrap();
        assert!(cluster.remove_member(&coord));
        assert_eq!(cluster.size(), 2);
    }

    #[test]
    fn test_kmeans_clustering() {
        let mut clustering = GeographicClustering::new();
        let nodes = create_test_nodes();

        // Perform K-means with k=3
        clustering.kmeans(&nodes, 3, 100);

        // Should have 3 clusters
        assert_eq!(clustering.clusters.len(), 3);

        // All nodes should be assigned
        for node in &nodes {
            assert!(clustering.get_node_cluster(node).is_some());
        }

        // Calculate metrics
        let metrics = clustering.calculate_metrics();
        assert!(metrics.avg_cohesion > 0.0);
        assert!(metrics.avg_separation > 0.0);
    }

    #[test]
    fn test_dbscan_clustering() {
        let mut clustering = GeographicClustering::new();
        let nodes = create_test_nodes();

        // Perform DBSCAN
        clustering.dbscan(&nodes, 3.0, 2);

        // Should find 3 clusters
        assert_eq!(clustering.clusters.len(), 3);

        // Each cluster should have 4 members
        for cluster in clustering.get_clusters() {
            assert_eq!(cluster.size(), 4);
        }
    }

    #[test]
    fn test_hierarchical_clustering() {
        let mut clustering = GeographicClustering::new();
        let nodes = vec![
            MatrixCoordinate::new(0, 0, 0).unwrap(),
            MatrixCoordinate::new(100, 100, 0).unwrap(),
        ];

        // Create mock zones
        let zones = vec![
            GeographicZone::new(
                "zone1".to_string(),
                "Zone 1".to_string(),
                GeographicLevel::City,
                GeographicBounds::new(-10.0, 10.0, -10.0, 10.0).unwrap(),
            ),
            GeographicZone::new(
                "zone2".to_string(),
                "Zone 2".to_string(),
                GeographicLevel::City,
                GeographicBounds::new(90.0, 110.0, 90.0, 110.0).unwrap(),
            ),
        ];

        clustering.hierarchical(&nodes, &zones);

        // Should have 2 clusters (one per zone)
        assert_eq!(clustering.clusters.len(), 2);

        // Each cluster should have zone metadata
        for cluster in clustering.get_clusters() {
            assert!(cluster.zone_id.is_some());
            assert!(cluster.metadata.contains_key("zone_name"));
            assert!(cluster.metadata.contains_key("zone_level"));
        }
    }

    #[test]
    fn test_dynamic_node_management() {
        let mut clustering = GeographicClustering::new();

        // Add first node (creates new cluster)
        let node1 = MatrixCoordinate::new(0, 0, 0).unwrap();
        clustering.add_node(node1, None);
        assert_eq!(clustering.clusters.len(), 1);

        // Add nearby node (joins existing cluster)
        let node2 = MatrixCoordinate::new(1, 1, 0).unwrap();
        clustering.add_node(node2, None);
        assert_eq!(clustering.clusters.len(), 1);

        // Add distant node (might create new cluster or join existing)
        let node3 = MatrixCoordinate::new(100, 100, 0).unwrap();
        clustering.add_node(node3, Some("custom_cluster".to_string()));
        assert_eq!(clustering.clusters.len(), 2);

        // Remove node
        let removed = clustering.remove_node(&node1);
        assert!(removed.is_some());

        // Remove all nodes from a cluster (cluster should be removed)
        clustering.remove_node(&node2);
        assert_eq!(clustering.clusters.len(), 1);
    }

    #[test]
    fn test_cluster_metrics() {
        let mut clustering = GeographicClustering::new();

        // Create well-separated clusters
        let nodes = vec![
            // Tight cluster 1
            MatrixCoordinate::new(0, 0, 0).unwrap(),
            MatrixCoordinate::new(1, 0, 0).unwrap(),
            MatrixCoordinate::new(0, 1, 0).unwrap(),
            // Tight cluster 2 (far away)
            MatrixCoordinate::new(100, 100, 0).unwrap(),
            MatrixCoordinate::new(101, 100, 0).unwrap(),
            MatrixCoordinate::new(100, 101, 0).unwrap(),
        ];

        clustering.kmeans(&nodes, 2, 100);
        let metrics = clustering.calculate_metrics();

        // Good clustering should have:
        // - Low cohesion (tight clusters)
        assert!(metrics.avg_cohesion < 2.0);
        // - High separation (clusters far apart)
        assert!(metrics.avg_separation > 100.0);
        // - Low Davies-Bouldin index
        assert!(metrics.davies_bouldin_index < 1.0);
        // - High Silhouette coefficient (close to 1)
        assert!(metrics.silhouette_coefficient > 0.5);
    }

    #[test]
    fn test_empty_clustering() {
        let mut clustering = GeographicClustering::new();

        // Empty nodes should not crash
        clustering.kmeans(&[], 3, 100);
        assert_eq!(clustering.clusters.len(), 0);

        clustering.dbscan(&[], 1.0, 2);
        assert_eq!(clustering.clusters.len(), 0);

        // Metrics for empty clustering
        let metrics = clustering.calculate_metrics();
        assert_eq!(metrics.avg_cohesion, 0.0);
        assert_eq!(metrics.avg_separation, 0.0);
    }

    #[test]
    fn test_single_node_clustering() {
        let mut clustering = GeographicClustering::new();
        let nodes = vec![MatrixCoordinate::new(5, 5, 5).unwrap()];

        // K-means with single node
        clustering.kmeans(&nodes, 1, 10);
        assert_eq!(clustering.clusters.len(), 1);

        // DBSCAN with single node
        clustering.dbscan(&nodes, 1.0, 1);
        assert_eq!(clustering.clusters.len(), 1);
    }
}