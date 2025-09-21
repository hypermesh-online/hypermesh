//! Dependency Resolution
//!
//! Dependency resolution and validation for assets.

use anyhow::{Result, Context};
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use crate::assets::Asset;

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Package name
    pub name: String,
    /// Version requirement
    pub version: String,
    /// Optional dependency
    pub optional: bool,
    /// Development dependency
    pub dev: bool,
    /// Dependency source
    pub source: Option<String>,
}

/// Dependency graph node
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// Asset identifier
    pub asset_id: String,
    /// Asset version
    pub version: String,
    /// Direct dependencies
    pub dependencies: Vec<Dependency>,
    /// Resolved version
    pub resolved_version: Option<String>,
}

/// Dependency resolver
pub struct DependencyResolver;

impl DependencyResolver {
    /// Create new dependency resolver
    pub fn new() -> Self {
        Self
    }

    /// Resolve asset dependencies
    pub async fn resolve(&self, asset: &Asset) -> Result<DependencyGraph> {
        let mut graph = DependencyGraph::new();
        let mut visited = HashSet::new();
        let mut stack = vec![asset.id.to_string()];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            // Get dependencies for current asset
            let deps = self.get_dependencies(&current).await?;

            // Add node to graph
            graph.add_node(DependencyNode {
                asset_id: current.clone(),
                version: asset.version.clone(),
                dependencies: deps.clone(),
                resolved_version: None,
            });

            // Add dependencies to stack
            for dep in deps {
                stack.push(dep.name);
            }
        }

        // Detect circular dependencies
        if let Some(cycle) = graph.detect_cycle() {
            return Err(anyhow::anyhow!("Circular dependency detected: {:?}", cycle));
        }

        // Resolve versions
        graph.resolve_versions()?;

        Ok(graph)
    }

    /// Get dependencies for an asset
    async fn get_dependencies(&self, asset_id: &str) -> Result<Vec<Dependency>> {
        // This would fetch from registry in real implementation
        // For now, return empty list
        Ok(Vec::new())
    }

    /// Check for version conflicts
    pub fn check_conflicts(&self, graph: &DependencyGraph) -> Vec<VersionConflict> {
        let mut conflicts = Vec::new();
        let mut version_map: HashMap<String, Vec<String>> = HashMap::new();

        // Collect all versions for each package
        for node in &graph.nodes {
            version_map.entry(node.asset_id.clone())
                .or_insert_with(Vec::new)
                .push(node.version.clone());
        }

        // Check for conflicts
        for (package, versions) in version_map {
            let unique_versions: HashSet<_> = versions.iter().collect();
            if unique_versions.len() > 1 {
                conflicts.push(VersionConflict {
                    package,
                    versions: unique_versions.into_iter().cloned().collect(),
                });
            }
        }

        conflicts
    }
}

/// Dependency graph
#[derive(Debug)]
pub struct DependencyGraph {
    /// Graph nodes
    pub nodes: Vec<DependencyNode>,
    /// Adjacency list
    pub edges: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    /// Create new dependency graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: HashMap::new(),
        }
    }

    /// Add node to graph
    pub fn add_node(&mut self, node: DependencyNode) {
        // Add edges
        for dep in &node.dependencies {
            self.edges
                .entry(node.asset_id.clone())
                .or_insert_with(Vec::new)
                .push(dep.name.clone());
        }

        self.nodes.push(node);
    }

    /// Detect cycles in dependency graph
    pub fn detect_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in &self.nodes {
            if !visited.contains(&node.asset_id) {
                if let Some(cycle) = self.detect_cycle_util(
                    &node.asset_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                ) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// Utility function for cycle detection
    fn detect_cycle_util(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = self.edges.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) = self.detect_cycle_util(neighbor, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        None
    }

    /// Resolve dependency versions
    pub fn resolve_versions(&mut self) -> Result<()> {
        // Simplified version resolution
        // In real implementation, this would use semver and constraint solving
        for node in &mut self.nodes {
            node.resolved_version = Some(node.version.clone());
        }
        Ok(())
    }

    /// Get dependency depth
    pub fn get_depth(&self) -> usize {
        let mut max_depth = 0;
        let mut visited = HashSet::new();

        for node in &self.nodes {
            if !visited.contains(&node.asset_id) {
                let depth = self.get_depth_util(&node.asset_id, &mut visited, 0);
                max_depth = max_depth.max(depth);
            }
        }

        max_depth
    }

    /// Utility function for depth calculation
    fn get_depth_util(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        current_depth: usize,
    ) -> usize {
        visited.insert(node.to_string());
        let mut max_depth = current_depth;

        if let Some(neighbors) = self.edges.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    let depth = self.get_depth_util(neighbor, visited, current_depth + 1);
                    max_depth = max_depth.max(depth);
                }
            }
        }

        max_depth
    }
}

/// Version conflict information
#[derive(Debug, Clone)]
pub struct VersionConflict {
    /// Package with conflict
    pub package: String,
    /// Conflicting versions
    pub versions: Vec<String>,
}