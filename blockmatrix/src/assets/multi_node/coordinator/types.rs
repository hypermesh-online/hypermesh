// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Types for Multi-Network Coordinator - node info, capabilities, and configuration

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use super::super::PeerIdentity;
use crate::assets::core::AssetType;

/// Node information and capabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node identifier
    pub node_id: PeerIdentity,
    /// Node capabilities
    pub capabilities: NodeCapabilities,
    /// Node status
    pub status: NodeStatus,
    /// Last heartbeat received
    pub last_heartbeat: SystemTime,
    /// Node location
    pub location: NodeLocation,
    /// Resource availability
    pub available_resources: AvailableResources,
    /// Performance metrics
    pub performance_metrics: NodePerformanceMetrics,
}

/// Node capabilities specification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// CPU cores available
    pub cpu_cores: u32,
    /// Total memory in bytes
    pub memory_bytes: u64,
    /// GPU devices available
    pub gpu_devices: u32,
    /// Storage capacity in bytes
    pub storage_bytes: u64,
    /// Network bandwidth in Mbps
    pub bandwidth_mbps: u64,
    /// Supported asset types
    pub supported_assets: Vec<AssetType>,
    /// Hardware features
    pub hardware_features: HardwareFeatures,
    /// Software capabilities
    pub software_capabilities: Vec<String>,
}

/// Hardware features available on node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HardwareFeatures {
    /// Intel SGX support
    pub sgx_enabled: bool,
    /// AMD SEV support
    pub sev_enabled: bool,
    /// TPM 2.0 available
    pub tpm_available: bool,
    /// Hardware random number generator
    pub hw_rng: bool,
    /// NVMe storage
    pub nvme_storage: bool,
    /// RDMA network support
    pub rdma_capable: bool,
    /// SR-IOV support
    pub sriov_enabled: bool,
}

/// Node status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    /// Node is healthy and active
    Active,
    /// Node is degraded but operational
    Degraded,
    /// Node is in maintenance mode
    Maintenance,
    /// Node is suspected to be Byzantine
    Suspected,
    /// Node has failed
    Failed,
    /// Node is partitioned from network
    Partitioned,
}

/// Node geographic location
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeLocation {
    /// Data center identifier
    pub datacenter: String,
    /// Geographic region
    pub region: String,
    /// Country code
    pub country: String,
    /// Latitude
    pub latitude: f64,
    /// Longitude
    pub longitude: f64,
    /// Network zone
    pub zone: String,
}

/// Available resources on node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvailableResources {
    /// Available CPU cores
    pub cpu_cores: f32,
    /// Available memory in bytes
    pub memory_bytes: u64,
    /// Available GPU units
    pub gpu_units: u32,
    /// Available storage in bytes
    pub storage_bytes: u64,
    /// Available bandwidth in Mbps
    pub bandwidth_mbps: u64,
}

/// Node performance metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodePerformanceMetrics {
    /// CPU utilization (0.0 to 1.0)
    pub cpu_utilization: f32,
    /// Memory utilization (0.0 to 1.0)
    pub memory_utilization: f32,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f32,
    /// Number of active assets
    pub active_assets: u64,
    /// Data processed in last hour (bytes)
    pub data_processed_bytes: u64,
}

/// Coordinator configuration
#[derive(Clone, Debug)]
pub struct CoordinatorConfig {
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Node failure timeout
    pub failure_timeout: Duration,
    /// Consensus timeout
    pub consensus_timeout: Duration,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Minimum nodes for consensus
    pub min_consensus_nodes: u32,
    /// Byzantine fault tolerance threshold
    pub byzantine_threshold: f32,
    /// Enable automatic migration
    pub auto_migration: bool,
    /// Enable load balancing
    pub load_balancing: bool,
    /// Resource pricing model
    pub pricing_enabled: bool,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(10),
            failure_timeout: Duration::from_secs(30),
            consensus_timeout: Duration::from_secs(5),
            max_retries: 3,
            min_consensus_nodes: 3,
            byzantine_threshold: 0.33,
            auto_migration: true,
            load_balancing: true,
            pricing_enabled: false,
        }
    }
}
