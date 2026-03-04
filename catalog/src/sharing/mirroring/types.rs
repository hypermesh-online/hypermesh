// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};

use super::super::topology::NodeLocation;
use crate::AssetRegistration;

/// Mirror strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirrorStrategy {
    /// Mirror based on popularity
    Popularity { threshold: f64, max_mirrors: u32 },
    /// Mirror based on geographic distribution
    Geographic {
        regions: Vec<String>,
        mirrors_per_region: u32,
    },
    /// Mirror based on access patterns
    AccessPattern {
        min_accesses: u64,
        time_window: Duration,
    },
    /// Mirror based on package importance
    Priority {
        min_priority: f64,
        replication_factor: u32,
    },
    /// Adaptive mirroring based on network conditions
    Adaptive {
        target_availability: f64,
        max_latency_ms: u64,
    },
}

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Default replication factor
    pub default_factor: u32,
    /// Maximum replication factor
    pub max_factor: u32,
    /// Minimum mirror replicas for redundancy
    pub min_replicas: u32,
    /// Geographic distribution requirements
    pub geo_distribution: bool,
    /// Prefer nodes with high uptime
    pub prefer_stable_nodes: bool,
    /// Replication timeout
    pub replication_timeout: Duration,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            default_factor: 3,
            max_factor: 10,
            min_replicas: 2,
            geo_distribution: true,
            prefer_stable_nodes: true,
            replication_timeout: Duration::from_secs(60),
        }
    }
}

/// Mirror node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorNode {
    /// Node ID
    pub node_id: String,
    /// Node location
    pub location: Option<NodeLocation>,
    /// Storage capacity (bytes)
    pub storage_capacity: u64,
    /// Used storage (bytes)
    pub storage_used: u64,
    /// Node uptime percentage
    pub uptime: f64,
    /// Average response time (ms)
    pub avg_response_time: u64,
    /// Packages mirrored
    pub mirrored_packages: HashSet<AssetRegistration>,
    /// Last health check
    pub last_health_check: SystemTime,
}

/// Package popularity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopularityMetrics {
    /// Total downloads
    pub downloads: u64,
    /// Downloads in last 24 hours
    pub downloads_24h: u64,
    /// Downloads in last 7 days
    pub downloads_7d: u64,
    /// Unique users
    pub unique_users: HashSet<String>,
    /// Average rating
    pub avg_rating: f64,
    /// Popularity score (0-1)
    pub score: f64,
    /// Trend (positive/negative)
    pub trend: f64,
}

/// Mirror status for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorStatus {
    /// Package ID
    pub asset_id: AssetRegistration,
    /// Mirror nodes
    pub mirror_nodes: Vec<String>,
    /// Replication factor achieved
    pub replication_factor: u32,
    /// Geographic coverage
    pub geographic_coverage: HashMap<String, u32>,
    /// Last mirroring operation
    pub last_mirrored: SystemTime,
    /// Mirror health score
    pub health_score: f64,
}

/// Priority queue item for mirroring decisions
#[derive(Debug, Clone)]
pub(in crate::sharing) struct MirrorCandidate {
    pub asset_id: AssetRegistration,
    pub priority: f64,
    pub size: u64,
}

impl Ord for MirrorCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .partial_cmp(&other.priority)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for MirrorCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MirrorCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for MirrorCandidate {}
