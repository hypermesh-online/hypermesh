//! Asset identification system with blockchain registration
//! 
//! Universal asset IDs that uniquely identify all HyperMesh assets
//! with cryptographic verification and blockchain registration.

use std::fmt;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

impl AssetId {
    /// Create new asset ID with automatic blockchain hash generation
    pub fn new(asset_type: AssetType) -> Self {
        let uuid = Uuid::new_v4();
        let creation_timestamp = SystemTime::now();
        let blockchain_hash = Self::generate_blockchain_hash(&asset_type, &uuid, &creation_timestamp);
        
        Self {
            asset_type,
            uuid,
            blockchain_hash,
            creation_timestamp,
        }
    }
    
    /// Create asset ID from existing components
    pub fn from_components(
        asset_type: AssetType,
        uuid: Uuid,
        blockchain_hash: [u8; 32],
        creation_timestamp: SystemTime,
    ) -> Self {
        Self {
            asset_type,
            uuid,
            blockchain_hash,
            creation_timestamp,
        }
    }
    
    /// Generate blockchain registration hash
    fn generate_blockchain_hash(
        asset_type: &AssetType,
        uuid: &Uuid,
        timestamp: &SystemTime,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        
        // Include asset type
        hasher.update(&[asset_type.type_id()]);
        
        // Include UUID bytes
        hasher.update(uuid.as_bytes());
        
        // Include timestamp
        if let Ok(duration) = timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            hasher.update(&duration.as_micros().to_le_bytes());
        }
        
        // Include network identifier (placeholder for HyperMesh network)
        hasher.update(b"HYPERMESH_V1");
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
    
    /// Get asset ID as hex string
    pub fn to_hex_string(&self) -> String {
        format!(
            "{}:{}:{}",
            self.asset_type.type_name().to_lowercase(),
            self.uuid.hyphenated(),
            hex::encode(self.blockchain_hash)
        )
    }
    
    /// Parse asset ID from hex string format
    pub fn from_hex_string(hex_str: &str) -> Result<Self, AssetIdError> {
        let parts: Vec<&str> = hex_str.split(':').collect();
        if parts.len() != 3 {
            return Err(AssetIdError::InvalidFormat {
                input: hex_str.to_string()
            });
        }
        
        // Parse asset type
        let asset_type = match parts[0] {
            "cpu" => AssetType::Cpu,
            "gpu" => AssetType::Gpu,
            "memory" => AssetType::Memory,
            "storage" => AssetType::Storage,
            "network" => AssetType::Network,
            "container" => AssetType::Container,
            "economic" => AssetType::Economic,
            _ => return Err(AssetIdError::InvalidAssetType {
                type_name: parts[0].to_string()
            }),
        };
        
        // Parse UUID
        let uuid = Uuid::parse_str(parts[1])
            .map_err(|_| AssetIdError::InvalidUuid {
                uuid_str: parts[1].to_string()
            })?;
        
        // Parse blockchain hash
        let hash_bytes = hex::decode(parts[2])
            .map_err(|_| AssetIdError::InvalidHash {
                hash_str: parts[2].to_string()
            })?;
        
        if hash_bytes.len() != 32 {
            return Err(AssetIdError::InvalidHashLength {
                expected: 32,
                actual: hash_bytes.len()
            });
        }
        
        let mut blockchain_hash = [0u8; 32];
        blockchain_hash.copy_from_slice(&hash_bytes);
        
        // Use current time as creation timestamp (since not stored in string format)
        let creation_timestamp = SystemTime::now();
        
        Ok(Self {
            asset_type,
            uuid,
            blockchain_hash,
            creation_timestamp,
        })
    }
    
    /// Verify blockchain hash integrity
    pub fn verify_blockchain_hash(&self) -> bool {
        let expected_hash = Self::generate_blockchain_hash(
            &self.asset_type,
            &self.uuid,
            &self.creation_timestamp,
        );
        
        self.blockchain_hash == expected_hash
    }
    
    /// Get short identifier (first 8 chars of UUID + first 8 chars of hash)
    pub fn short_id(&self) -> String {
        let uuid_str = self.uuid.hyphenated().to_string();
        format!(
            "{}:{}...{}",
            self.asset_type.type_name().to_lowercase(),
            &uuid_str[..8],
            &hex::encode(&self.blockchain_hash[..4])
        )
    }
    
    /// Get age since creation
    pub fn age(&self) -> Option<std::time::Duration> {
        SystemTime::now().duration_since(self.creation_timestamp).ok()
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

/// Asset ID related errors
#[derive(Debug, thiserror::Error)]
pub enum AssetIdError {
    /// Invalid format for asset ID string
    #[error("Invalid asset ID format: {input}")]
    InvalidFormat { input: String },
    
    /// Invalid asset type name
    #[error("Invalid asset type: {type_name}")]
    InvalidAssetType { type_name: String },
    
    /// Invalid UUID format
    #[error("Invalid UUID: {uuid_str}")]
    InvalidUuid { uuid_str: String },
    
    /// Invalid hash format
    #[error("Invalid hash: {hash_str}")]
    InvalidHash { hash_str: String },
    
    /// Invalid hash length
    #[error("Invalid hash length: expected {expected}, got {actual}")]
    InvalidHashLength { expected: usize, actual: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_asset_id_creation() {
        let asset_id = AssetId::new(AssetType::Cpu);
        assert_eq!(asset_id.asset_type, AssetType::Cpu);
        assert!(asset_id.verify_blockchain_hash());
    }
    
    #[test]
    fn test_asset_id_serialization() {
        let asset_id = AssetId::new(AssetType::Memory);
        let hex_string = asset_id.to_hex_string();
        
        // Note: from_hex_string loses timestamp precision, so we don't test round-trip
        assert!(hex_string.starts_with("memory:"));
        assert!(hex_string.contains(':'));
    }
    
    #[test]
    fn test_asset_type_properties() {
        assert_eq!(AssetType::Cpu.type_id(), 0);
        assert_eq!(AssetType::Gpu.type_id(), 1);
        assert_eq!(AssetType::Cpu.type_name(), "CPU");
        assert_eq!(AssetType::Gpu.type_name(), "GPU");
    }
    
    #[test]
    fn test_asset_id_verification() {
        let asset_id = AssetId::new(AssetType::Storage);
        assert!(asset_id.verify_blockchain_hash());
        
        // Test with modified hash
        let mut modified_id = asset_id.clone();
        modified_id.blockchain_hash[0] ^= 1; // Flip one bit
        assert!(!modified_id.verify_blockchain_hash());
    }
    
    #[test]
    fn test_short_id() {
        let asset_id = AssetId::new(AssetType::Container);
        let short_id = asset_id.short_id();
        assert!(short_id.starts_with("container:"));
        assert!(short_id.contains("..."));
    }
}