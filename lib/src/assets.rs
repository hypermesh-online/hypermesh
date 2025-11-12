//! Shared asset types used across all HyperMesh components

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::SystemTime;
use uuid::Uuid;

/// Universal asset type enumeration
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum AssetType {
    /// CPU cores and processing units
    Cpu,
    /// GPU compute units and memory
    Gpu,
    /// RAM and memory allocation
    Memory,
    /// Storage devices and capacity
    Storage,
    /// Network interfaces and bandwidth
    Network,
    /// Container instances and services
    Container,
    /// Economic system assets (Caesar tokens, wallets, stakes)
    Economic,
}

impl AssetType {
    /// Get asset type identifier for hashing
    pub fn type_id(&self) -> u8 {
        match self {
            AssetType::Cpu => 0,
            AssetType::Gpu => 1,
            AssetType::Memory => 2,
            AssetType::Storage => 3,
            AssetType::Network => 4,
            AssetType::Container => 5,
            AssetType::Economic => 6,
        }
    }

    /// Get human-readable asset type name
    pub fn type_name(&self) -> &'static str {
        match self {
            AssetType::Cpu => "CPU",
            AssetType::Gpu => "GPU",
            AssetType::Memory => "Memory",
            AssetType::Storage => "Storage",
            AssetType::Network => "Network",
            AssetType::Container => "Container",
            AssetType::Economic => "Economic",
        }
    }
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name())
    }
}

/// Universal asset identifier with blockchain registration
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetId {
    /// Type of asset
    pub asset_type: AssetType,
    /// Unique identifier within type
    pub uuid: Uuid,
    /// Blockchain registration hash (32 bytes)
    pub blockchain_hash: [u8; 32],
    /// Creation timestamp
    pub creation_timestamp: SystemTime,
}

/// Asset metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub id: AssetId,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}
