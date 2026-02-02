//! Asset identification system with blockchain registration
//!
//! Content-based, network-scoped asset IDs that uniquely identify all HyperMesh assets
//! with cryptographic verification, blockchain registration, and network isolation.
//!
//! Key Features:
//! - Content-based hashing: Same asset data always produces same ID
//! - Network scoping: Assets isolated per network/federation/registry
//! - Non-fungible: Each asset instance uniquely identified
//! - Proof of State binding: PoS requirements tied to asset instantiation
//! - Security boundaries: System vs application asset separation

use std::fmt;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::matrix::coordinate::MatrixCoordinate;

/// Network scope defining asset isolation boundaries
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum NetworkScope {
    /// Global HyperMesh public network
    Global,
    /// Specific catalog registry
    Registry(RegistryId),
    /// Federated network group
    Federated(FederationId),
    /// Private node registry
    Private(NodeId),
}

/// Registry identifier (content hash of registry configuration)
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct RegistryId(pub [u8; 32]);

/// Federation identifier (content hash of federation agreement)
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct FederationId(pub [u8; 32]);

/// Node identifier (node's unique blockchain ID)
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 32]);

/// Asset category for security boundary enforcement
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum AssetCategory {
    /// Base system assets required for HyperMesh operation
    BaseSystem(BaseSystemType),
    /// Application-specific assets
    Application(ApplicationDomain),
}

/// Base system asset types
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum BaseSystemType {
    Cpu,
    Gpu,
    Memory,
    Storage,
    Network,
    Container,
    Economic,
    Blockchain,
}

/// Application domain for user assets
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplicationDomain {
    pub domain_name: String,
    pub domain_hash: [u8; 32],
}

/// Proof of State scope configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofScope {
    /// Which proofs are required for this asset
    pub required_proofs: ProofRequirements,
    /// Scope binding to instantiation context
    pub scope_binding: ScopeBinding,
}

/// Required proof types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofRequirements {
    pub require_space: bool,
    pub require_stake: bool,
    pub require_work: bool,
    pub require_time: bool,
}

/// Scope binding for instantiation context
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScopeBinding {
    /// Binding ID (hash of instantiation parameters)
    pub binding_id: [u8; 32],
    /// Scope-specific configuration
    pub scope_config: Vec<u8>,
}

/// Asset data for content-based hashing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetData {
    /// Asset configuration
    pub config: Vec<u8>,
    /// Asset definition
    pub definition: Vec<u8>,
    /// Asset metadata
    pub metadata: Vec<u8>,
}

/// Universal asset type enumeration (legacy support)
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
    /// Virtual machine instances and execution environments
    // STUB: Phase 3
    VirtualMachine,
    /// Library assets and code dependencies
    // STUB: Phase 3
    Library,
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
            AssetType::VirtualMachine => 7, // STUB: Phase 3
            AssetType::Library => 8, // STUB: Phase 3
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
            AssetType::VirtualMachine => "VirtualMachine", // STUB: Phase 3
            AssetType::Library => "Library", // STUB: Phase 3
        }
    }
}

impl fmt::Display for AssetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name())
    }
}

/// Universal asset identifier with content-based hashing and network scoping
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetId {
    /// Content-based hash (derived from asset data, not UUID)
    pub content_hash: [u8; 32],

    /// Network scope (which registry/network)
    pub network_scope: NetworkScope,

    /// Asset category (base system vs application)
    pub category: AssetCategory,

    /// Creation timestamp
    pub creation_timestamp: SystemTime,
}

impl AssetId {
    /// Create new asset ID from asset data (content-based)
    pub fn from_asset_data(
        data: &AssetData,
        network_scope: NetworkScope,
        category: AssetCategory,
    ) -> Self {
        let content_hash = Self::generate_content_hash(data, &network_scope, &category);

        Self {
            content_hash,
            network_scope,
            category,
            creation_timestamp: SystemTime::now(),
        }
    }


    /// Create asset ID from hash (for default/test purposes)
    pub fn new_from_hash(hash: &[u8; 32]) -> Self {
        Self {
            content_hash: *hash,
            network_scope: NetworkScope::Global,
            category: AssetCategory::BaseSystem(BaseSystemType::Container),
            creation_timestamp: SystemTime::now(),
        }
    }

    /// Create a genesis asset for a node's blockchain
    pub fn genesis(node_coordinate: MatrixCoordinate) -> Self {
        let genesis_data = AssetData {
            config: format!("Genesis asset for node at ({}, {}, {})",
                node_coordinate.x, node_coordinate.y, node_coordinate.z
            ).into_bytes(),
            definition: b"GENESIS_BLOCK".to_vec(),
            metadata: format!("Created at {:?}", SystemTime::now()).into_bytes(),
        };

        Self::from_asset_data(
            &genesis_data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Blockchain),
        )
    }

    /// Generate content-based hash
    fn generate_content_hash(
        data: &AssetData,
        network_scope: &NetworkScope,
        category: &AssetCategory,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Hash network scope
        match network_scope {
            NetworkScope::Global => hasher.update(b"GLOBAL"),
            NetworkScope::Registry(id) => {
                hasher.update(b"REGISTRY");
                hasher.update(&id.0);
            }
            NetworkScope::Federated(id) => {
                hasher.update(b"FEDERATED");
                hasher.update(&id.0);
            }
            NetworkScope::Private(id) => {
                hasher.update(b"PRIVATE");
                hasher.update(&id.0);
            }
        }

        // Hash category
        match category {
            AssetCategory::BaseSystem(system_type) => {
                hasher.update(b"SYSTEM");
                hasher.update(&[match system_type {
                    BaseSystemType::Cpu => 0,
                    BaseSystemType::Gpu => 1,
                    BaseSystemType::Memory => 2,
                    BaseSystemType::Storage => 3,
                    BaseSystemType::Network => 4,
                    BaseSystemType::Container => 5,
                    BaseSystemType::Economic => 6,
                    BaseSystemType::Blockchain => 7,
                }]);
            }
            AssetCategory::Application(domain) => {
                hasher.update(b"APPLICATION");
                hasher.update(domain.domain_name.as_bytes());
                hasher.update(&domain.domain_hash);
            }
        }

        // Hash asset data (this ensures content-based uniqueness)
        hasher.update(&data.config);
        hasher.update(&data.definition);
        hasher.update(&data.metadata);

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Verify content matches hash
    pub fn verify_content(&self, data: &AssetData) -> bool {
        let expected_hash = Self::generate_content_hash(data, &self.network_scope, &self.category);
        self.content_hash == expected_hash
    }

    /// Check if asset can exist in network
    pub fn can_exist_in_network(&self, network: &NetworkScope) -> bool {
        match (&self.network_scope, network) {
            (NetworkScope::Global, _) => true, // Global assets visible everywhere
            (NetworkScope::Registry(id1), NetworkScope::Registry(id2)) => id1 == id2,
            (NetworkScope::Federated(id1), NetworkScope::Federated(id2)) => id1 == id2,
            (NetworkScope::Private(id1), NetworkScope::Private(id2)) => id1 == id2,
            _ => false,
        }
    }

    /// Validate security boundaries
    pub fn validate_security_boundary(&self) -> Result<(), SecurityError> {
        match &self.category {
            AssetCategory::BaseSystem(_) => {
                // System assets have strict security requirements
                if !matches!(self.network_scope, NetworkScope::Global | NetworkScope::Registry(_)) {
                    return Err(SecurityError::InvalidScope {
                        asset_category: "BaseSystem".to_string(),
                        network_scope: format!("{:?}", self.network_scope),
                    });
                }
                Ok(())
            }
            AssetCategory::Application(_) => {
                // Application assets can exist in any scope
                Ok(())
            }
        }
    }


    /// Get asset ID as hex string
    pub fn to_hex_string(&self) -> String {
        // New format: scope:category:hash
        let scope_str = match &self.network_scope {
            NetworkScope::Global => "global".to_string(),
            NetworkScope::Registry(id) => format!("reg:{}", hex::encode(&id.0[..4])),
            NetworkScope::Federated(id) => format!("fed:{}", hex::encode(&id.0[..4])),
            NetworkScope::Private(id) => format!("priv:{}", hex::encode(&id.0[..4])),
        };

        let category_str = match &self.category {
            AssetCategory::BaseSystem(sys) => format!("sys:{:?}", sys).to_lowercase(),
            AssetCategory::Application(app) => format!("app:{}", &app.domain_name),
        };

        format!("{}:{}:{}", scope_str, category_str, hex::encode(self.content_hash))
    }

    /// Parse asset ID from hex string format
    pub fn from_hex_string(hex_str: &str) -> Result<Self, AssetIdError> {
        // Parse new format: scope:category:hash
        let parts: Vec<&str> = hex_str.split(':').collect();
        if parts.len() < 3 {
            return Err(AssetIdError::InvalidFormat {
                input: hex_str.to_string(),
            });
        }

        // Parse network scope
        let network_scope = match parts[0] {
            "global" => NetworkScope::Global,
            scope if scope.starts_with("reg") => {
                let hash = hex::decode(parts[1])
                    .map_err(|_| AssetIdError::InvalidFormat {
                        input: hex_str.to_string(),
                    })?;
                let mut id = [0u8; 32];
                id[..hash.len().min(32)].copy_from_slice(&hash[..hash.len().min(32)]);
                NetworkScope::Registry(RegistryId(id))
            }
            scope if scope.starts_with("fed") => {
                let hash = hex::decode(parts[1])
                    .map_err(|_| AssetIdError::InvalidFormat {
                        input: hex_str.to_string(),
                    })?;
                let mut id = [0u8; 32];
                id[..hash.len().min(32)].copy_from_slice(&hash[..hash.len().min(32)]);
                NetworkScope::Federated(FederationId(id))
            }
            scope if scope.starts_with("priv") => {
                let hash = hex::decode(parts[1])
                    .map_err(|_| AssetIdError::InvalidFormat {
                        input: hex_str.to_string(),
                    })?;
                let mut id = [0u8; 32];
                id[..hash.len().min(32)].copy_from_slice(&hash[..hash.len().min(32)]);
                NetworkScope::Private(NodeId(id))
            }
            _ => NetworkScope::Global, // Default to global
        };

        // Parse content hash (last part)
        let hash_bytes = hex::decode(parts[parts.len() - 1])
            .map_err(|_| AssetIdError::InvalidHash {
                hash_str: parts[parts.len() - 1].to_string(),
            })?;

        if hash_bytes.len() != 32 {
            return Err(AssetIdError::InvalidHashLength {
                expected: 32,
                actual: hash_bytes.len(),
            });
        }

        let mut content_hash = [0u8; 32];
        content_hash.copy_from_slice(&hash_bytes);

        // Simple category detection for now
        let category = AssetCategory::Application(ApplicationDomain {
            domain_name: "imported".to_string(),
            domain_hash: [0u8; 32],
        });

        Ok(Self {
            content_hash,
            network_scope,
            category,
            creation_timestamp: SystemTime::now(),
        })
    }


    /// Get short identifier
    pub fn short_id(&self) -> String {
        let scope_prefix = match &self.network_scope {
            NetworkScope::Global => "G",
            NetworkScope::Registry(_) => "R",
            NetworkScope::Federated(_) => "F",
            NetworkScope::Private(_) => "P",
        };

        format!("{}:{}", scope_prefix, &hex::encode(&self.content_hash[..8]))
    }
    
    /// Get age since creation
    pub fn age(&self) -> Option<std::time::Duration> {
        SystemTime::now().duration_since(self.creation_timestamp).ok()
    }

    /// Get AssetType from category (for legacy compatibility)
    pub fn asset_type(&self) -> Option<AssetType> {
        match &self.category {
            AssetCategory::BaseSystem(base_type) => Some(match base_type {
                BaseSystemType::Cpu => AssetType::Cpu,
                BaseSystemType::Gpu => AssetType::Gpu,
                BaseSystemType::Memory => AssetType::Memory,
                BaseSystemType::Storage => AssetType::Storage,
                BaseSystemType::Network => AssetType::Network,
                BaseSystemType::Container => AssetType::Container,
                BaseSystemType::Economic => AssetType::Economic,
                BaseSystemType::Blockchain => AssetType::Container, // Map to Container for legacy
            }),
            AssetCategory::Application(_) => Some(AssetType::Library),
        }
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

/// Security-related errors for asset validation
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    /// Invalid network scope for asset category
    #[error("Invalid scope {network_scope} for {asset_category} asset")]
    InvalidScope {
        asset_category: String,
        network_scope: String,
    },

    /// Asset cannot cross security boundary
    #[error("Asset cannot cross security boundary from {from} to {to}")]
    BoundaryViolation { from: String, to: String },

    /// Insufficient proof requirements
    #[error("Insufficient proof requirements: {missing}")]
    InsufficientProof { missing: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_content_based_hashing() {
        // Same data should produce same ID
        let data1 = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };

        let data2 = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };

        let id1 = AssetId::from_asset_data(
            &data1,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Cpu),
        );

        let id2 = AssetId::from_asset_data(
            &data2,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Cpu),
        );

        // Same data, same network, same category = same ID
        assert_eq!(id1.content_hash, id2.content_hash);
    }

    #[test]
    fn test_network_scoping() {
        let data = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };

        let global_id = AssetId::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Cpu),
        );

        let registry_id = AssetId::from_asset_data(
            &data,
            NetworkScope::Registry(RegistryId([1u8; 32])),
            AssetCategory::BaseSystem(BaseSystemType::Cpu),
        );

        // Same data but different network scope = different ID
        assert_ne!(global_id.content_hash, registry_id.content_hash);

        // Global assets can exist in any network
        assert!(global_id.can_exist_in_network(&NetworkScope::Global));
        assert!(global_id.can_exist_in_network(&NetworkScope::Registry(RegistryId([2u8; 32]))));

        // Registry assets only exist in their specific registry
        assert!(!registry_id.can_exist_in_network(&NetworkScope::Global));
        assert!(registry_id.can_exist_in_network(&NetworkScope::Registry(RegistryId([1u8; 32]))));
        assert!(!registry_id.can_exist_in_network(&NetworkScope::Registry(RegistryId([2u8; 32]))));
    }

    #[test]
    fn test_security_boundaries() {
        let data = AssetData {
            config: vec![],
            definition: vec![],
            metadata: vec![],
        };

        // Base system assets can exist in global or registry scope
        let system_global = AssetId::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Cpu),
        );
        assert!(system_global.validate_security_boundary().is_ok());

        let system_registry = AssetId::from_asset_data(
            &data,
            NetworkScope::Registry(RegistryId([1u8; 32])),
            AssetCategory::BaseSystem(BaseSystemType::Cpu),
        );
        assert!(system_registry.validate_security_boundary().is_ok());

        // Base system assets cannot exist in private scope
        let system_private = AssetId::from_asset_data(
            &data,
            NetworkScope::Private(NodeId([1u8; 32])),
            AssetCategory::BaseSystem(BaseSystemType::Cpu),
        );
        assert!(system_private.validate_security_boundary().is_err());

        // Application assets can exist in any scope
        let app_private = AssetId::from_asset_data(
            &data,
            NetworkScope::Private(NodeId([1u8; 32])),
            AssetCategory::Application(ApplicationDomain {
                domain_name: "test".to_string(),
                domain_hash: [0u8; 32],
            }),
        );
        assert!(app_private.validate_security_boundary().is_ok());
    }

    #[test]
    fn test_content_verification() {
        let data = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };

        let asset_id = AssetId::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Memory),
        );

        // Verify with same data
        assert!(asset_id.verify_content(&data));

        // Verify with different data
        let different_data = AssetData {
            config: vec![9, 8, 7],
            definition: vec![6, 5, 4],
            metadata: vec![3, 2, 1],
        };
        assert!(!asset_id.verify_content(&different_data));
    }

    #[test]
    fn test_short_id() {
        let data = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };
        let asset_id = AssetId::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Container),
        );
        let short_id = asset_id.short_id();
        assert!(short_id.starts_with("G:")); // Global scope
    }
}