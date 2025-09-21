//! Asset Core Types
//!
//! Core type definitions for the HyperMesh asset system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::SystemTime;

/// Universal Asset ID
pub type AssetId = String;

/// Allocation ID
pub type AllocationId = String;

/// Universal Asset - Everything in HyperMesh is an Asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Unique asset ID
    pub id: AssetId,

    /// Asset type (CPU, GPU, Memory, Storage, Network, VM, Service)
    pub asset_type: AssetType,

    /// Asset name and description
    pub name: String,
    pub description: String,

    /// Asset owner (node ID or user ID)
    pub owner: String,

    /// Current status
    pub status: AssetStatus,

    /// Privacy level for sharing
    pub privacy_level: PrivacyLevel,

    /// Physical or network location
    pub location: AssetLocation,

    /// Resource capacity and current allocation
    pub capacity: ResourceAllocation,
    pub allocated: ResourceAllocation,
    pub available: ResourceAllocation,

    /// Associated consensus proofs
    pub consensus_proofs: HashMap<String, Vec<u8>>,

    /// Remote proxy address (for NAT-like addressing)
    pub proxy_address: Option<String>,

    /// Asset metadata
    pub metadata: HashMap<String, String>,

    /// Creation and update timestamps
    pub created_at: SystemTime,
    pub updated_at: SystemTime,

    /// Asset statistics
    pub statistics: AssetStatistics,
}

/// Asset types in HyperMesh ecosystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetType {
    /// CPU compute resources
    Cpu,
    /// GPU compute resources
    Gpu,
    /// Memory (RAM) resources
    Memory,
    /// Storage resources
    Storage,
    /// Network connection resources
    Network,
    /// Virtual machine
    Vm,
    /// Service endpoint
    Service,
    /// Container
    Container,
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cpu => write!(f, "CPU"),
            Self::Gpu => write!(f, "GPU"),
            Self::Memory => write!(f, "Memory"),
            Self::Storage => write!(f, "Storage"),
            Self::Network => write!(f, "Network"),
            Self::Vm => write!(f, "VM"),
            Self::Service => write!(f, "Service"),
            Self::Container => write!(f, "Container"),
        }
    }
}

/// Asset status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetStatus {
    /// Asset is available for allocation
    Available,
    /// Asset is partially allocated
    PartiallyAllocated,
    /// Asset is fully allocated
    FullyAllocated,
    /// Asset is offline or unavailable
    Offline,
    /// Asset is in maintenance mode
    Maintenance,
    /// Asset has been decommissioned
    Decommissioned,
}

/// Privacy level for asset sharing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// No public access
    Private,
    /// Specific private networks only
    PrivateNetwork,
    /// Trusted peer sharing
    P2P,
    /// Specific public networks
    PublicNetwork,
    /// Full public access (maximum CAESAR rewards)
    FullPublic,
}

/// Asset physical or network location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLocation {
    /// Node ID where asset is located
    pub node_id: String,

    /// Geographic region
    pub region: Option<String>,

    /// Data center or zone
    pub zone: Option<String>,

    /// Network segment
    pub network_segment: Option<String>,

    /// IPv6 address (if applicable)
    pub ipv6_address: Option<String>,
}

/// Resource allocation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// CPU cores or compute units
    pub cpu_units: Option<f64>,

    /// GPU compute units
    pub gpu_units: Option<f64>,

    /// Memory in bytes
    pub memory_bytes: Option<u64>,

    /// Storage in bytes
    pub storage_bytes: Option<u64>,

    /// Network bandwidth in bits per second
    pub bandwidth_bps: Option<u64>,

    /// IOPS for storage assets
    pub iops: Option<u64>,

    /// Custom resource units
    pub custom_units: HashMap<String, f64>,
}

impl ResourceAllocation {
    /// Create empty allocation
    pub fn empty() -> Self {
        Self {
            cpu_units: None,
            gpu_units: None,
            memory_bytes: None,
            storage_bytes: None,
            bandwidth_bps: None,
            iops: None,
            custom_units: HashMap::new(),
        }
    }

    /// Check if allocation is empty
    pub fn is_empty(&self) -> bool {
        self.cpu_units.is_none()
            && self.gpu_units.is_none()
            && self.memory_bytes.is_none()
            && self.storage_bytes.is_none()
            && self.bandwidth_bps.is_none()
            && self.iops.is_none()
            && self.custom_units.is_empty()
    }
}

/// Asset usage and performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetStatistics {
    /// Total allocations
    pub total_allocations: u64,

    /// Current active allocations
    pub active_allocations: u32,

    /// Total usage time in seconds
    pub total_usage_seconds: u64,

    /// Average utilization percentage
    pub avg_utilization: f64,

    /// Total data processed (bytes)
    pub data_processed: u64,

    /// Total operations completed
    pub operations_completed: u64,

    /// Success rate (percentage)
    pub success_rate: f64,

    /// Average response time (milliseconds)
    pub avg_response_ms: f64,

    /// CAESAR tokens earned
    pub tokens_earned: f64,

    /// Last access timestamp
    pub last_accessed: Option<SystemTime>,
}

impl Default for AssetStatistics {
    fn default() -> Self {
        Self {
            total_allocations: 0,
            active_allocations: 0,
            total_usage_seconds: 0,
            avg_utilization: 0.0,
            data_processed: 0,
            operations_completed: 0,
            success_rate: 100.0,
            avg_response_ms: 0.0,
            tokens_earned: 0.0,
            last_accessed: None,
        }
    }
}