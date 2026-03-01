// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::Result;
use std::collections::{HashMap, HashSet};

impl super::NetworkTopology {
    /// Dijkstra's shortest path algorithm
    pub(super) async fn dijkstra_shortest_path(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;
        let links = self.links.read().await;

        let mut distances: HashMap<String, u64> = HashMap::new();
        let mut previous: HashMap<String, Option<String>> = HashMap::new();
        let mut unvisited: HashSet<String> = nodes.keys().cloned().collect();

        // Initialize distances
        for node_id in nodes.keys() {
            distances.insert(node_id.clone(), u64::MAX);
            previous.insert(node_id.clone(), None);
        }
        distances.insert(from.to_string(), 0);

        while !unvisited.is_empty() {
            // Find unvisited node with minimum distance
            let current = unvisited
                .iter()
                .min_by_key(|&n| distances.get(n).unwrap_or(&u64::MAX))
                .cloned();

            if let Some(current_node) = current {
                if current_node == to {
                    break;
                }

                unvisited.remove(&current_node);

                // Update distances to neighbors
                for link in links.iter() {
                    if link.from == current_node && unvisited.contains(&link.to) {
                        let alt = distances[&current_node].saturating_add(1);
                        if alt < distances[&link.to] {
                            distances.insert(link.to.clone(), alt);
                            previous.insert(link.to.clone(), Some(current_node.clone()));
                        }
                    }
                }
            } else {
                break;
            }
        }

        // Reconstruct path
        let mut path = Vec::new();
        let mut current = Some(to.to_string());

        while let Some(node) = current {
            path.push(node.clone());
            current = previous.get(&node).and_then(|p| p.clone());
            if current.as_ref() == Some(&from.to_string()) {
                path.push(from.to_string());
                break;
            }
        }

        path.reverse();

        if path.is_empty() || path[0] != from {
            return Err(anyhow::anyhow!("No route found"));
        }

        Ok(path)
    }

    /// Find lowest latency path
    pub(super) async fn lowest_latency_path(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;
        let links = self.links.read().await;

        let mut distances: HashMap<String, u64> = HashMap::new();
        let mut previous: HashMap<String, Option<String>> = HashMap::new();
        let mut unvisited: HashSet<String> = nodes.keys().cloned().collect();

        // Initialize with max latency
        for node_id in nodes.keys() {
            distances.insert(node_id.clone(), u64::MAX);
            previous.insert(node_id.clone(), None);
        }
        distances.insert(from.to_string(), 0);

        while !unvisited.is_empty() {
            let current = unvisited
                .iter()
                .min_by_key(|&n| distances.get(n).unwrap_or(&u64::MAX))
                .cloned();

            if let Some(current_node) = current {
                if current_node == to {
                    break;
                }

                unvisited.remove(&current_node);

                // Update based on latency
                for link in links.iter() {
                    if link.from == current_node && unvisited.contains(&link.to) {
                        let alt = distances[&current_node].saturating_add(link.latency);
                        if alt < distances[&link.to] {
                            distances.insert(link.to.clone(), alt);
                            previous.insert(link.to.clone(), Some(current_node.clone()));
                        }
                    }
                }
            } else {
                break;
            }
        }

        // Reconstruct path
        self.reconstruct_path(from, to, &previous)
    }

    /// Find highest bandwidth path (bottleneck shortest path).
    pub(super) async fn highest_bandwidth_path(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;
        let links = self.links.read().await;

        let mut max_bw: HashMap<String, u64> = HashMap::new();
        let mut previous: HashMap<String, Option<String>> = HashMap::new();
        let mut unvisited: HashSet<String> = nodes.keys().cloned().collect();

        for node_id in nodes.keys() {
            max_bw.insert(node_id.clone(), 0);
            previous.insert(node_id.clone(), None);
        }
        max_bw.insert(from.to_string(), u64::MAX);

        while !unvisited.is_empty() {
            let current = unvisited
                .iter()
                .max_by_key(|n| max_bw.get(*n).unwrap_or(&0))
                .cloned();

            if let Some(current_node) = current {
                if current_node == to {
                    break;
                }
                let current_bw = max_bw[&current_node];
                if current_bw == 0 {
                    break;
                }

                unvisited.remove(&current_node);

                for link in links.iter() {
                    if link.from == current_node && unvisited.contains(&link.to) {
                        let path_bw = current_bw.min(link.bandwidth);
                        if path_bw > max_bw[&link.to] {
                            max_bw.insert(link.to.clone(), path_bw);
                            previous.insert(link.to.clone(), Some(current_node.clone()));
                        }
                    }
                }
            } else {
                break;
            }
        }

        self.reconstruct_path(from, to, &previous)
    }

    /// Geographic proximity routing
    pub(super) async fn geographic_routing(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;

        let target_location = nodes
            .get(to)
            .and_then(|n| n.location.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Target location unknown"))?;

        // Route through geographically closer nodes
        let mut current = from.to_string();
        let mut path = vec![current.clone()];
        let mut visited = HashSet::new();

        while current != to {
            visited.insert(current.clone());

            // Find closest neighbor to target
            let current_node = nodes
                .get(&current)
                .ok_or_else(|| anyhow::anyhow!("Node not found"))?;

            let mut best_next = None;
            let mut best_distance = f64::MAX;

            for peer_id in &current_node.peers {
                if visited.contains(peer_id) {
                    continue;
                }

                if let Some(peer) = nodes.get(peer_id) {
                    if let Some(peer_location) = &peer.location {
                        let distance = peer_location.distance_to(target_location);
                        if distance < best_distance {
                            best_distance = distance;
                            best_next = Some(peer_id.clone());
                        }
                    }
                }
            }

            if let Some(next) = best_next {
                path.push(next.clone());
                current = next;
            } else {
                // Fall back to shortest path
                return self
                    .dijkstra_shortest_path(&current, to)
                    .await
                    .map(|sub_path| {
                        path.extend_from_slice(&sub_path[1..]);
                        path
                    });
            }
        }

        Ok(path)
    }

    /// Load balanced routing
    pub(super) async fn load_balanced_routing(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;
        let path = self.dijkstra_shortest_path(from, to).await?;

        let overloaded: HashSet<String> = path[1..path.len().saturating_sub(1)]
            .iter()
            .filter(|node_id| {
                nodes.get(*node_id).is_some_and(|node| {
                    let conn_ratio = if node.capacity.max_connections > 0 {
                        node.capacity.current_connections as f64
                            / node.capacity.max_connections as f64
                    } else {
                        0.0
                    };
                    node.capacity.network_usage > 0.8 || conn_ratio > 0.8
                })
            })
            .cloned()
            .collect();

        if overloaded.is_empty() {
            return Ok(path);
        }

        drop(nodes);
        match self.find_disjoint_path(from, to, &overloaded).await {
            Ok(alt_path) => Ok(alt_path),
            Err(_) => Ok(path),
        }
    }

    /// Fault tolerant routing with multiple paths
    pub(super) async fn fault_tolerant_routing(
        &self,
        from: &str,
        to: &str,
        redundancy: u32,
    ) -> Result<Vec<String>> {
        // Find multiple disjoint paths
        let mut paths = Vec::new();
        let mut excluded_nodes = HashSet::new();

        for _ in 0..redundancy {
            // Find path excluding already used nodes
            let path = self.find_disjoint_path(from, to, &excluded_nodes).await?;

            // Add intermediate nodes to excluded set
            for node in &path[1..path.len() - 1] {
                excluded_nodes.insert(node.clone());
            }

            paths.push(path);
        }

        // Return primary path (could return all for redundancy)
        Ok(paths.into_iter().next().unwrap_or_else(Vec::new))
    }

    /// Find path avoiding specific nodes
    pub(super) async fn find_disjoint_path(
        &self,
        from: &str,
        to: &str,
        excluded: &HashSet<String>,
    ) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;
        let links = self.links.read().await;

        let mut distances: HashMap<String, u64> = HashMap::new();
        let mut previous: HashMap<String, Option<String>> = HashMap::new();
        let mut unvisited: HashSet<String> = nodes
            .keys()
            .filter(|n| !excluded.contains(*n) || *n == from || *n == to)
            .cloned()
            .collect();

        // Initialize distances
        for node_id in &unvisited {
            distances.insert(node_id.clone(), u64::MAX);
            previous.insert(node_id.clone(), None);
        }
        distances.insert(from.to_string(), 0);

        while !unvisited.is_empty() {
            let current = unvisited
                .iter()
                .min_by_key(|&n| distances.get(n).unwrap_or(&u64::MAX))
                .cloned();

            if let Some(current_node) = current {
                if current_node == to {
                    break;
                }

                unvisited.remove(&current_node);

                for link in links.iter() {
                    if link.from == current_node
                        && unvisited.contains(&link.to)
                        && !excluded.contains(&link.to)
                    {
                        let alt = distances[&current_node].saturating_add(1);
                        if alt < distances[&link.to] {
                            distances.insert(link.to.clone(), alt);
                            previous.insert(link.to.clone(), Some(current_node.clone()));
                        }
                    }
                }
            } else {
                break;
            }
        }

        self.reconstruct_path(from, to, &previous)
    }

    /// Reconstruct path from previous nodes map
    pub(super) fn reconstruct_path(
        &self,
        from: &str,
        to: &str,
        previous: &HashMap<String, Option<String>>,
    ) -> Result<Vec<String>> {
        let mut path = Vec::new();
        let mut current = Some(to.to_string());

        while let Some(node) = current {
            path.push(node.clone());
            current = previous.get(&node).and_then(|p| p.clone());
            if current.as_ref() == Some(&from.to_string()) {
                path.push(from.to_string());
                break;
            }
        }

        path.reverse();

        if path.is_empty() || path[0] != from {
            return Err(anyhow::anyhow!("No route found"));
        }

        Ok(path)
    }
}
