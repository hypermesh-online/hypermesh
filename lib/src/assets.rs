//! Shared asset types used across all HyperMesh components

pub use blockmatrix::assets::core::AssetType;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

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
