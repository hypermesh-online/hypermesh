// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Type definitions for extension registry

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::super::{ExtensionCategory, ExtensionMetadata};

/// Extension registry entry
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    /// Extension metadata
    pub metadata: ExtensionMetadata,

    /// Extension state
    pub state: ExtensionEntryState,

    /// Registration timestamp
    pub registered_at: std::time::SystemTime,

    /// Last updated timestamp
    pub updated_at: std::time::SystemTime,

    /// Extension location
    pub location: ExtensionLocation,

    /// Health status
    pub health: HealthStatus,

    /// Performance metrics
    pub metrics: ExtensionMetrics,
}

/// Extension entry state
#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionEntryState {
    /// Extension is registered but not loaded
    Registered,

    /// Extension is being loaded
    Loading,

    /// Extension is loaded and active
    Active,

    /// Extension is paused
    Paused,

    /// Extension is being unloaded
    Unloading,

    /// Extension failed to load
    Failed(String),
}

/// Extension location information
#[derive(Debug, Clone)]
pub struct ExtensionLocation {
    /// File system path
    pub path: PathBuf,

    /// Remote URL if downloaded
    pub url: Option<String>,

    /// IPFS/STOQ hash for P2P distribution
    pub distribution_hash: Option<String>,
}

/// Extension health status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Overall health state
    pub state: HealthState,

    /// Last health check
    pub last_check: std::time::SystemTime,

    /// Health check failures
    pub failures: u32,

    /// Error messages
    pub errors: Vec<String>,
}

/// Health state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum HealthState {
    /// Extension is healthy
    Healthy,

    /// Extension is degraded
    Degraded,

    /// Extension is unhealthy
    Unhealthy,

    /// Health unknown
    Unknown,
}

/// Extension performance metrics
#[derive(Debug, Clone, Default)]
pub struct ExtensionMetrics {
    /// Total requests handled
    pub total_requests: u64,

    /// Failed requests
    pub failed_requests: u64,

    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,

    /// CPU usage percentage
    pub cpu_usage_percent: f32,

    /// Memory usage in bytes
    pub memory_usage_bytes: u64,

    /// Uptime duration
    pub uptime: std::time::Duration,
}

/// Registry configuration
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Maximum registry size
    pub max_entries: usize,

    /// Enable automatic dependency resolution
    pub auto_resolve_deps: bool,

    /// Enable health monitoring
    pub health_monitoring: bool,

    /// Health check interval
    pub health_check_interval: std::time::Duration,

    /// Enable metrics collection
    pub collect_metrics: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            auto_resolve_deps: true,
            health_monitoring: true,
            health_check_interval: std::time::Duration::from_secs(60),
            collect_metrics: true,
        }
    }
}

/// Dependency graph for extensions
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Forward dependencies (extension -> dependencies)
    pub(crate) forward: HashMap<String, HashSet<String>>,

    /// Reverse dependencies (extension -> dependents)
    pub(crate) reverse: HashMap<String, HashSet<String>>,

    /// Topological order for loading
    load_order: Vec<String>,
}

impl DependencyGraph {
    /// Add extension with dependencies
    pub fn add_extension(&mut self, id: String, deps: Vec<String>) {
        self.forward.insert(id.clone(), deps.iter().cloned().collect());

        for dep in deps {
            self.reverse.entry(dep).or_default().insert(id.clone());
        }

        self.update_load_order();
    }

    /// Remove extension
    pub fn remove_extension(&mut self, id: &str) {
        if let Some(deps) = self.forward.remove(id) {
            for dep in deps {
                if let Some(rev_deps) = self.reverse.get_mut(&dep) {
                    rev_deps.remove(id);
                }
            }
        }

        self.reverse.remove(id);
        self.update_load_order();
    }

    /// Get load order
    pub fn get_load_order(&self) -> Vec<String> {
        self.load_order.clone()
    }

    /// Check if loading extension would create cycle
    pub fn would_create_cycle(&self, id: &str, new_deps: &[String]) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for dep in new_deps {
            if self.has_path_to(dep, id, &mut visited, &mut stack) {
                return true;
            }
        }

        false
    }

    /// Check if there's a path from source to target
    fn has_path_to(
        &self,
        source: &str,
        target: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> bool {
        if source == target {
            return true;
        }

        if stack.contains(source) || visited.contains(source) {
            return false;
        }

        visited.insert(source.to_string());
        stack.insert(source.to_string());

        if let Some(deps) = self.forward.get(source) {
            for dep in deps {
                if self.has_path_to(dep, target, visited, stack) {
                    stack.remove(source);
                    return true;
                }
            }
        }

        stack.remove(source);
        false
    }

    /// Update topological load order
    fn update_load_order(&mut self) {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_stack = HashSet::new();

        for id in self.forward.keys() {
            if !visited.contains(id) {
                self.topological_sort(id, &mut visited, &mut temp_stack, &mut order);
            }
        }

        self.load_order = order;
    }

    /// Topological sort helper
    fn topological_sort(
        &self,
        id: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) {
        if stack.contains(id) || visited.contains(id) {
            return;
        }

        stack.insert(id.to_string());

        if let Some(deps) = self.forward.get(id) {
            for dep in deps {
                self.topological_sort(dep, visited, stack, order);
            }
        }

        stack.remove(id);
        visited.insert(id.to_string());
        order.push(id.to_string());
    }
}

/// Registry event listener trait
#[async_trait::async_trait]
pub trait RegistryListener: Send + Sync {
    /// Extension registered event
    async fn on_extension_registered(&self, id: &str, metadata: &ExtensionMetadata);

    /// Extension loaded event
    async fn on_extension_loaded(&self, id: &str);

    /// Extension unloaded event
    async fn on_extension_unloaded(&self, id: &str);

    /// Extension health changed event
    async fn on_health_changed(&self, id: &str, health: &HealthState);

    /// Extension failed event
    async fn on_extension_failed(&self, id: &str, error: &str);
}

/// Search criteria for extensions
#[derive(Debug, Clone)]
pub struct SearchCriteria {
    /// Filter by category
    pub category: Option<ExtensionCategory>,

    /// Filter by state
    pub state: Option<ExtensionEntryState>,

    /// Filter by health
    pub health: Option<HealthState>,

    /// Filter by name pattern
    pub name_pattern: Option<String>,

    /// Filter by author
    pub author: Option<String>,
}
