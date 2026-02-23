// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::time::SystemTime;

/// Node location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeLocation {
    /// Geographic region
    pub region: String,
    /// Country code
    pub country: String,
    /// City
    pub city: Option<String>,
    /// Latitude
    pub latitude: Option<f64>,
    /// Longitude
    pub longitude: Option<f64>,
    /// Data center/provider
    pub provider: Option<String>,
    /// Network ASN
    pub asn: Option<u32>,
}

impl NodeLocation {
    /// Calculate distance to another location (in km)
    pub fn distance_to(&self, other: &NodeLocation) -> f64 {
        if let (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) =
            (self.latitude, self.longitude, other.latitude, other.longitude) {
            // Haversine formula
            let r = 6371.0; // Earth radius in km
            let dlat = (lat2 - lat1).to_radians();
            let dlon = (lon2 - lon1).to_radians();
            let a = (dlat / 2.0).sin() * (dlat / 2.0).sin() +
                    lat1.to_radians().cos() * lat2.to_radians().cos() *
                    (dlon / 2.0).sin() * (dlon / 2.0).sin();
            let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
            r * c
        } else {
            // Fallback to region-based distance
            if self.region == other.region {
                100.0 // Same region
            } else if self.country == other.country {
                500.0 // Same country
            } else {
                2000.0 // Different countries
            }
        }
    }
}

/// Routing strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Shortest path routing
    ShortestPath,
    /// Lowest latency routing
    LowestLatency,
    /// Highest bandwidth routing
    HighestBandwidth,
    /// Geographic proximity
    GeographicProximity,
    /// Load balanced routing
    LoadBalanced,
    /// Fault tolerant routing (multiple paths)
    FaultTolerant { redundancy: u32 },
}

/// Network link between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLink {
    /// Source node ID
    pub from: String,
    /// Destination node ID
    pub to: String,
    /// Link latency (ms)
    pub latency: u64,
    /// Available bandwidth (bytes/sec)
    pub bandwidth: u64,
    /// Packet loss rate (0-1)
    pub packet_loss: f64,
    /// Link reliability score (0-1)
    pub reliability: f64,
    /// Last measured
    pub last_measured: SystemTime,
}

/// Node status in the network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    /// Node is online and healthy
    Online,
    /// Node is degraded (high latency/packet loss)
    Degraded,
    /// Node is offline
    Offline,
    /// Node is in maintenance
    Maintenance,
    /// Node status unknown
    Unknown,
}

/// Network node information
#[derive(Debug, Clone)]
pub struct NetworkNode {
    /// Node ID
    pub id: String,
    /// Node address
    pub address: String,
    /// Node location
    pub location: Option<NodeLocation>,
    /// Node status
    pub status: NodeStatus,
    /// Connected peers
    pub peers: HashSet<String>,
    /// Node capacity metrics
    pub capacity: NodeCapacity,
    /// Last health check
    pub last_health_check: SystemTime,
}

/// Node capacity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapacity {
    /// CPU utilization (0-1)
    pub cpu_usage: f64,
    /// Memory usage (0-1)
    pub memory_usage: f64,
    /// Storage usage (0-1)
    pub storage_usage: f64,
    /// Network utilization (0-1)
    pub network_usage: f64,
    /// Maximum connections
    pub max_connections: u32,
    /// Current connections
    pub current_connections: u32,
}

impl Default for NodeCapacity {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            storage_usage: 0.0,
            network_usage: 0.0,
            max_connections: 1000,
            current_connections: 0,
        }
    }
}
