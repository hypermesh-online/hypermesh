// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! NAT-like Address Translation System for HyperMesh
//!
//! CRITICAL COMPONENT: Implements the core NAT-like memory addressing system
//! that enables remote memory access via IPv6-like global addresses.

use std::collections::HashMap;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use libc::{mmap, munmap, PROT_READ, PROT_WRITE, PROT_EXEC, MAP_PRIVATE, MAP_ANONYMOUS, MAP_FAILED};

use crate::assets::core::{AssetId, AssetResult, AssetError, ProxyAddress};

/// Global address in HyperMesh ecosystem (IPv6-like addressing)
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct GlobalAddress {
    /// Network prefix (8 bytes) - identifies HyperMesh network segment
    pub network_prefix: [u8; 8],
    
    /// Node identifier (8 bytes) - identifies proxy node
    pub node_id: [u8; 8],
    
    /// Asset identifier (16 bytes) - derived from AssetId UUID
    pub asset_id: [u8; 16],
    
    /// Service port - identifies specific service on asset
    pub service_port: u16,
    
    /// Address type (memory, cpu, storage, etc.)
    pub address_type: GlobalAddressType,
    
    /// Creation timestamp for validation
    pub created_at: SystemTime,
}

/// Types of global addresses
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum GlobalAddressType {
    Memory,
    CPU,
    GPU,
    Storage,
    Network,
    Service,
}

/// Local address mapping for NAT translation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalAddressMapping {
    /// Global address
    pub global_address: GlobalAddress,

    /// Local memory/resource address
    pub local_address: usize,

    /// Size of the mapped region
    pub region_size: u64,

    /// Access permissions
    pub access_permissions: MemoryPermissions,

    /// Privacy configuration
    pub privacy_config: Option<PrivacyConfig>,

    /// Translation state
    pub translation_state: TranslationState,

    /// Usage statistics
    pub usage_stats: AddressUsageStats,

    /// Last accessed timestamp
    pub last_accessed: SystemTime,

    /// Expiration timestamp
    pub expires_at: SystemTime,
}

/// Memory access permissions for NAT translations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryPermissions {
    /// Read access
    pub read: bool,

    /// Write access
    pub write: bool,

    /// Execute access
    pub execute: bool,

    /// Share access with other nodes
    pub share: bool,

    /// Cache access (for performance)
    pub cache: bool,

    /// Prefetch access (for optimization)
    pub prefetch: bool,
}

// Use PrivacyLevel from core module
pub use crate::assets::core::PrivacyLevel;

/// Privacy configuration for NAT translations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Privacy level
    pub level: PrivacyLevel,
    /// Allowed network IDs
    pub allowed_networks: Vec<String>,
    /// Allowed peer IDs
    pub allowed_peers: Vec<String>,
    /// Maximum concurrent access
    pub max_concurrent_access: u32,
    /// Require consensus validation
    pub require_consensus: bool,
}

/// Translation state tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TranslationState {
    /// Translation is active and ready
    Active,
    
    /// Translation is pending setup
    Pending,
    
    /// Translation is suspended
    Suspended,
    
    /// Translation has expired
    Expired,
    
    /// Translation has error
    Error { message: String },
}

/// Usage statistics for address translations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressUsageStats {
    /// Total access count
    pub total_accesses: u64,

    /// Total bytes read
    pub total_bytes_read: u64,

    /// Total bytes written
    pub total_bytes_written: u64,

    /// Cache hit rate
    pub cache_hit_rate: f32,

    /// Average access latency in microseconds
    pub average_latency_us: u64,

    /// Last performance measurement
    pub last_measured: SystemTime,
}

impl Default for AddressUsageStats {
    fn default() -> Self {
        Self {
            total_accesses: 0,
            total_bytes_read: 0,
            total_bytes_written: 0,
            cache_hit_rate: 0.0,
            average_latency_us: 0,
            last_measured: SystemTime::now(),
        }
    }
}

/// The main NAT translator for memory addressing
pub struct NATTranslator {
    /// Global to local address mappings
    global_to_local: Arc<RwLock<HashMap<GlobalAddress, LocalAddressMapping>>>,
    
    /// Local to global address mappings (reverse lookup)
    local_to_global: Arc<RwLock<HashMap<usize, GlobalAddress>>>,
    
    /// Address allocation tracking
    address_allocator: Arc<RwLock<AddressAllocator>>,
    
    /// Network configuration
    network_config: NetworkConfig,
    
    /// Translation statistics
    translation_stats: Arc<RwLock<TranslationStats>>,
}

/// Address allocation management
#[derive(Debug)]
struct AddressAllocator {
    /// Next available local address
    next_local_address: usize,
    
    /// Address space size
    address_space_size: u64,
    
    /// Allocated address ranges
    allocated_ranges: Vec<AddressRange>,
    
    /// Free address ranges
    free_ranges: Vec<AddressRange>,
}

/// Address range specification
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AddressRange {
    /// Start address
    start: usize,
    
    /// End address (inclusive)
    end: usize,
    
    /// Size in bytes
    size: u64,
}

/// Network configuration for NAT translation
#[derive(Clone, Debug)]
struct NetworkConfig {
    /// HyperMesh network prefix
    network_prefix: [u8; 8],
    
    /// Local node identifier
    local_node_id: [u8; 8],
    
    /// Address space start
    address_space_start: usize,
    
    /// Address space size
    address_space_size: u64,
}

/// Translation system statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TranslationStats {
    /// Total translations created
    pub total_translations: u64,
    
    /// Active translations
    pub active_translations: u64,
    
    /// Total translation requests
    pub translation_requests: u64,
    
    /// Successful translations
    pub successful_translations: u64,
    
    /// Failed translations
    pub failed_translations: u64,
    
    /// Average translation time in microseconds
    pub average_translation_time_us: u64,
    
    /// Total memory mapped in bytes
    pub total_memory_mapped: u64,
    
    /// Cache performance stats
    pub cache_stats: CacheStats,
}

/// Cache performance statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Cache hits
    pub hits: u64,
    
    /// Cache misses
    pub misses: u64,
    
    /// Hit rate percentage
    pub hit_rate: f32,
    
    /// Cache size in entries
    pub cache_size: u64,
    
    /// Cache memory usage in bytes
    pub cache_memory_usage: u64,
}

impl GlobalAddress {
    /// Create new global address
    pub fn new(
        network_prefix: [u8; 8],
        node_id: [u8; 8],
        asset_id: &AssetId,
        service_port: u16,
        address_type: GlobalAddressType,
    ) -> Self {
        let mut asset_bytes = [0u8; 16];
        asset_bytes.copy_from_slice(&asset_id.content_hash[..16]);
        
        Self {
            network_prefix,
            node_id,
            asset_id: asset_bytes,
            service_port,
            address_type,
            created_at: SystemTime::now(),
        }
    }
    
    /// Convert to IPv6 address representation for network compatibility
    pub fn to_ipv6(&self) -> Ipv6Addr {
        // Construct IPv6 address from components
        let mut ipv6_bytes = [0u8; 16];
        
        // First 8 bytes: network prefix
        ipv6_bytes[0..8].copy_from_slice(&self.network_prefix);
        
        // Next 8 bytes: node ID
        ipv6_bytes[8..16].copy_from_slice(&self.node_id);
        
        Ipv6Addr::from(ipv6_bytes)
    }
    
    /// Convert to socket address
    pub fn to_socket_addr(&self) -> SocketAddrV6 {
        SocketAddrV6::new(self.to_ipv6(), self.service_port, 0, 0)
    }
    
    /// Get string representation
    pub fn to_string(&self) -> String {
        format!(
            "hypermesh://{}/{}/{}:{}",
            hex::encode(self.network_prefix),
            hex::encode(self.node_id),
            hex::encode(self.asset_id),
            self.service_port
        )
    }
    
    /// Parse from string representation
    pub fn from_string(s: &str) -> AssetResult<Self> {
        if !s.starts_with("hypermesh://") {
            return Err(AssetError::AdapterError {
                message: "Invalid global address scheme".to_string()
            });
        }
        
        let addr_part = &s[12..]; // Remove "hypermesh://"
        let parts: Vec<&str> = addr_part.split('/').collect();
        
        if parts.len() != 3 {
            return Err(AssetError::AdapterError {
                message: "Invalid global address format".to_string()
            });
        }
        
        // Parse network prefix
        let network_bytes = hex::decode(parts[0])
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid network prefix".to_string()
            })?;
        if network_bytes.len() != 8 {
            return Err(AssetError::AdapterError {
                message: "Network prefix must be 8 bytes".to_string()
            });
        }
        let mut network_prefix = [0u8; 8];
        network_prefix.copy_from_slice(&network_bytes);
        
        // Parse node ID
        let node_bytes = hex::decode(parts[1])
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid node ID".to_string()
            })?;
        if node_bytes.len() != 8 {
            return Err(AssetError::AdapterError {
                message: "Node ID must be 8 bytes".to_string()
            });
        }
        let mut node_id = [0u8; 8];
        node_id.copy_from_slice(&node_bytes);
        
        // Parse asset ID and port
        let asset_port: Vec<&str> = parts[2].split(':').collect();
        if asset_port.len() != 2 {
            return Err(AssetError::AdapterError {
                message: "Invalid asset:port format".to_string()
            });
        }
        
        let asset_bytes = hex::decode(asset_port[0])
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid asset ID".to_string()
            })?;
        if asset_bytes.len() != 16 {
            return Err(AssetError::AdapterError {
                message: "Asset ID must be 16 bytes".to_string()
            });
        }
        let mut asset_id = [0u8; 16];
        asset_id.copy_from_slice(&asset_bytes);
        
        let service_port: u16 = asset_port[1].parse()
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid service port".to_string()
            })?;
        
        Ok(Self {
            network_prefix,
            node_id,
            asset_id,
            service_port,
            address_type: GlobalAddressType::Memory, // Default
            created_at: SystemTime::now(),
        })
    }
    
    /// Generate address hash for validation
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.network_prefix);
        hasher.update(&self.node_id);
        hasher.update(&self.asset_id);
        hasher.update(&self.service_port.to_le_bytes());
        hasher.update(&format!("{:?}", self.address_type).as_bytes());
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

impl NATTranslator {
    /// Create new NAT translator
    pub async fn new() -> AssetResult<Self> {
        let network_config = NetworkConfig {
            network_prefix: [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            local_node_id: Self::generate_local_node_id(),
            address_space_start: 0x1000_0000, // Start at 256MB
            address_space_size: 0x4000_0000,  // 1GB address space
        };
        
        let address_allocator = AddressAllocator {
            next_local_address: network_config.address_space_start,
            address_space_size: network_config.address_space_size,
            allocated_ranges: Vec::new(),
            free_ranges: vec![AddressRange {
                start: network_config.address_space_start,
                end: network_config.address_space_start + network_config.address_space_size as usize - 1,
                size: network_config.address_space_size,
            }],
        };
        
        Ok(Self {
            global_to_local: Arc::new(RwLock::new(HashMap::new())),
            local_to_global: Arc::new(RwLock::new(HashMap::new())),
            address_allocator: Arc::new(RwLock::new(address_allocator)),
            network_config,
            translation_stats: Arc::new(RwLock::new(TranslationStats::default())),
        })
    }
    
    /// Generate global address for asset
    pub async fn generate_global_address(
        &self,
        node_id: &str,
        asset_id: &AssetId,
        service_port: u16,
    ) -> AssetResult<ProxyAddress> {
        // Convert node_id string to bytes
        let mut node_bytes = [0u8; 8];
        let node_id_bytes = node_id.as_bytes();
        let copy_len = node_id_bytes.len().min(8);
        node_bytes[..copy_len].copy_from_slice(&node_id_bytes[..copy_len]);
        
        // Create global address
        let global_addr = GlobalAddress::new(
            self.network_config.network_prefix,
            node_bytes,
            asset_id,
            service_port,
            GlobalAddressType::Memory, // Default to memory
        );
        
        // Convert to ProxyAddress for compatibility
        // Pad 8-byte network prefix to 16 bytes for IPv6-style addressing
        let mut network_prefix_16 = [0u8; 16];
        network_prefix_16[..8].copy_from_slice(&self.network_config.network_prefix);
        let proxy_addr = ProxyAddress::new(
            network_prefix_16,
            node_bytes,
            service_port,
        );
        
        Ok(proxy_addr)
    }
    
    /// Create NAT translation mapping with real memory
    pub async fn create_translation(
        &self,
        global_addr: GlobalAddress,
        region_size: u64,
        permissions: MemoryPermissions,
    ) -> AssetResult<LocalAddressMapping> {
        let start_time = SystemTime::now();

        // Map real memory using mmap
        let prot = self.permissions_to_prot(&permissions);
        let local_address = unsafe {
            let ptr = mmap(
                std::ptr::null_mut(),
                region_size as usize,
                prot,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );

            if ptr == MAP_FAILED {
                return Err(AssetError::AdapterError {
                    message: format!("mmap failed: {}", std::io::Error::last_os_error())
                });
            }

            ptr as usize
        };

        // Create mapping
        let mapping = LocalAddressMapping {
            global_address: global_addr.clone(),
            local_address,
            region_size,
            access_permissions: permissions,
            privacy_config: None,
            translation_state: TranslationState::Active,
            usage_stats: AddressUsageStats::default(),
            last_accessed: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600), // 1 hour default
        };

        // Store mappings
        {
            let mut global_to_local = self.global_to_local.write().await;
            global_to_local.insert(global_addr.clone(), mapping.clone());
        }

        let global_addr_str = global_addr.to_string();

        {
            let mut local_to_global = self.local_to_global.write().await;
            local_to_global.insert(local_address, global_addr);
        }

        // Update statistics
        {
            let mut stats = self.translation_stats.write().await;
            stats.total_translations += 1;
            stats.active_translations += 1;
            stats.successful_translations += 1;
            stats.total_memory_mapped += region_size;

            if let Ok(duration) = start_time.elapsed() {
                stats.average_translation_time_us =
                    (stats.average_translation_time_us + duration.as_micros() as u64) / 2;
            }
        }

        tracing::info!(
            "Created NAT translation: {} -> 0x{:x} ({} bytes) with real memory mapping",
            global_addr_str,
            local_address,
            region_size
        );

        Ok(mapping)
    }

    /// Create translation with privacy controls
    pub async fn create_translation_with_privacy(
        &self,
        global_addr: GlobalAddress,
        region_size: u64,
        permissions: MemoryPermissions,
        privacy: PrivacyConfig,
    ) -> AssetResult<LocalAddressMapping> {
        // Validate privacy settings
        self.validate_privacy_config(&privacy).await?;

        // Create translation with privacy metadata
        let mut mapping = self.create_translation(global_addr, region_size, permissions).await?;

        // Attach privacy configuration
        mapping.privacy_config = Some(privacy);

        // Update the stored mapping
        {
            let mut global_to_local = self.global_to_local.write().await;
            global_to_local.insert(mapping.global_address.clone(), mapping.clone());
        }

        Ok(mapping)
    }

    /// Convert MemoryPermissions to PROT flags
    fn permissions_to_prot(&self, perms: &MemoryPermissions) -> i32 {
        let mut prot = 0;
        if perms.read { prot |= PROT_READ; }
        if perms.write { prot |= PROT_WRITE; }
        if perms.execute { prot |= PROT_EXEC; }
        prot
    }

    /// Validate privacy configuration
    async fn validate_privacy_config(&self, privacy: &PrivacyConfig) -> AssetResult<()> {
        // Validate max concurrent access
        if privacy.max_concurrent_access == 0 {
            return Err(AssetError::AdapterError {
                message: "Max concurrent access must be greater than 0".to_string()
            });
        }

        // Validate privacy level settings
        match privacy.level {
            PrivacyLevel::Private => {
                if !privacy.allowed_networks.is_empty() || !privacy.allowed_peers.is_empty() {
                    return Err(AssetError::AdapterError {
                        message: "Private level should not have allowed networks or peers".to_string()
                    });
                }
            },
            PrivacyLevel::PrivateNetwork | PrivacyLevel::PublicNetwork => {
                if privacy.allowed_networks.is_empty() {
                    return Err(AssetError::AdapterError {
                        message: "Network privacy level requires allowed networks".to_string()
                    });
                }
            },
            PrivacyLevel::P2P => {
                if privacy.allowed_peers.is_empty() {
                    return Err(AssetError::AdapterError {
                        message: "P2P privacy level requires allowed peers".to_string()
                    });
                }
            },
            PrivacyLevel::FullPublic => {
                // No restrictions for full public
            }
        }

        Ok(())
    }
    
    /// Translate global address to local address (now with real memory mapping)
    pub async fn translate_to_local(&self, global_addr: &GlobalAddress) -> AssetResult<usize> {
        let global_to_local = self.global_to_local.read().await;
        let mapping = global_to_local.get(global_addr)
            .ok_or_else(|| AssetError::AdapterError {
                message: format!("No translation found for global address: {}", global_addr.to_string())
            })?;
        
        // Check if translation is active
        if !matches!(mapping.translation_state, TranslationState::Active) {
            return Err(AssetError::AdapterError {
                message: "Translation is not active".to_string()
            });
        }
        
        // Check if translation has expired
        if mapping.expires_at < SystemTime::now() {
            return Err(AssetError::AdapterError {
                message: "Translation has expired".to_string()
            });
        }
        
        // Update statistics
        {
            let mut stats = self.translation_stats.write().await;
            stats.translation_requests += 1;
        }
        
        Ok(mapping.local_address)
    }
    
    /// Translate local address to global address  
    pub async fn translate_to_global(&self, local_addr: usize) -> AssetResult<GlobalAddress> {
        let local_to_global = self.local_to_global.read().await;
        local_to_global.get(&local_addr)
            .cloned()
            .ok_or_else(|| AssetError::AdapterError {
                message: format!("No translation found for local address: 0x{:x}", local_addr)
            })
    }
    
    /// Remove translation and unmap memory
    pub async fn remove_translation(&self, global_addr: &GlobalAddress) -> AssetResult<()> {
        let mapping = {
            let mut global_to_local = self.global_to_local.write().await;
            global_to_local.remove(global_addr)
        };

        if let Some(mapping) = mapping {
            // Remove reverse mapping
            {
                let mut local_to_global = self.local_to_global.write().await;
                local_to_global.remove(&mapping.local_address);
            }

            // Unmap the actual memory
            unsafe {
                let result = munmap(mapping.local_address as *mut libc::c_void, mapping.region_size as usize);
                if result != 0 {
                    return Err(AssetError::AdapterError {
                        message: format!("munmap failed: {}", std::io::Error::last_os_error())
                    });
                }
            }

            // Update statistics
            {
                let mut stats = self.translation_stats.write().await;
                stats.active_translations = stats.active_translations.saturating_sub(1);
                stats.total_memory_mapped = stats.total_memory_mapped.saturating_sub(mapping.region_size);
            }

            tracing::info!(
                "Removed NAT translation and unmapped memory: {} -> 0x{:x}",
                global_addr.to_string(),
                mapping.local_address
            );
        }

        Ok(())
    }
    
    // Note: allocate_local_address and free_local_address methods removed
    // since we now use mmap/munmap directly for real memory management
    
    /// Generate local node ID
    fn generate_local_node_id() -> [u8; 8] {
        // TODO: Generate based on actual node characteristics
        // For now, use a hash of current time and hostname
        let mut hasher = Sha256::new();

        // Use system time with fallback to zero
        let time_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        hasher.update(&time_secs.to_le_bytes());
        hasher.update(b"hypermesh-node");

        let result = hasher.finalize();
        let mut node_id = [0u8; 8];
        node_id.copy_from_slice(&result[..8]);
        node_id
    }
    
    /// Get translation statistics
    pub async fn get_stats(&self) -> AssetResult<TranslationStats> {
        let stats = self.translation_stats.read().await;
        Ok(stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::{AssetType, AssetId};
    use crate::test_utils::test_asset_id;

    #[test]
    fn test_global_address_creation() {
        let asset_id = test_asset_id(AssetType::Memory);
        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
            &asset_id,
            8080,
            GlobalAddressType::Memory,
        );
        
        assert_eq!(global_addr.service_port, 8080);
        assert!(matches!(global_addr.address_type, GlobalAddressType::Memory));
    }
    
    #[test]
    fn test_global_address_string_conversion() {
        let asset_id = test_asset_id(AssetType::Memory);
        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
            &asset_id,
            8080,
            GlobalAddressType::Memory,
        );
        
        let addr_str = global_addr.to_string();
        assert!(addr_str.starts_with("hypermesh://"));
        assert!(addr_str.contains("8080"));
    }
    
    #[tokio::test]
    async fn test_nat_translator_creation() {
        let translator = NATTranslator::new().await.unwrap();
        let stats = translator.get_stats().await.unwrap();
        assert_eq!(stats.total_translations, 0);
        assert_eq!(stats.active_translations, 0);
    }
    
    #[tokio::test]
    async fn test_translation_creation() {
        let translator = NATTranslator::new().await.unwrap();
        let asset_id = test_asset_id(AssetType::Memory);
        
        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
            &asset_id,
            8080,
            GlobalAddressType::Memory,
        );
        
        let permissions = MemoryPermissions {
            read: true,
            write: true,
            execute: false,
            share: false,
            cache: true,
            prefetch: true,
        };
        
        let mapping = translator.create_translation(
            global_addr.clone(),
            1024 * 1024, // 1MB
            permissions,
        ).await.unwrap();
        
        assert_eq!(mapping.region_size, 1024 * 1024);
        assert!(matches!(mapping.translation_state, TranslationState::Active));
        
        // Test address translation
        let local_addr = translator.translate_to_local(&global_addr).await.unwrap();
        assert_eq!(local_addr, mapping.local_address);
        
        // Test reverse translation
        let reverse_global = translator.translate_to_global(local_addr).await.unwrap();
        assert_eq!(reverse_global.hash(), global_addr.hash());
    }

    #[tokio::test]
    async fn test_real_memory_mapping() {
        let translator = NATTranslator::new().await.unwrap();
        let asset_id = test_asset_id(AssetType::Memory);

        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
            &asset_id,
            8081,
            GlobalAddressType::Memory,
        );

        let permissions = MemoryPermissions {
            read: true,
            write: true,
            execute: false,
            share: false,
            cache: true,
            prefetch: false,
        };

        // Create mapping with real memory
        let mapping = translator.create_translation(
            global_addr.clone(),
            4096, // 4KB page
            permissions,
        ).await.unwrap();

        // Verify memory is actually mapped and usable
        let local_ptr = mapping.local_address as *mut u8;
        unsafe {
            // Write to memory
            *local_ptr = 42;
            // Read back
            assert_eq!(*local_ptr, 42);

            // Write a sequence
            for i in 0..256 {
                *local_ptr.add(i) = i as u8;
            }

            // Verify sequence
            for i in 0..256 {
                assert_eq!(*local_ptr.add(i), i as u8);
            }
        }

        // Clean up - remove translation and unmap memory
        translator.remove_translation(&global_addr).await.unwrap();

        // Verify stats updated
        let stats = translator.get_stats().await.unwrap();
        assert_eq!(stats.active_translations, 0);
    }

    #[tokio::test]
    async fn test_translation_with_privacy() {
        let translator = NATTranslator::new().await.unwrap();
        let asset_id = test_asset_id(AssetType::Memory);

        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22],
            &asset_id,
            8082,
            GlobalAddressType::Memory,
        );

        let permissions = MemoryPermissions {
            read: true,
            write: false,
            execute: false,
            share: true,
            cache: true,
            prefetch: false,
        };

        let privacy_config = PrivacyConfig {
            level: PrivacyLevel::P2P,
            allowed_networks: vec![],
            allowed_peers: vec!["peer1".to_string(), "peer2".to_string()],
            max_concurrent_access: 5,
            require_consensus: false,
        };

        // Create translation with privacy controls
        let mapping = translator.create_translation_with_privacy(
            global_addr.clone(),
            8192, // 8KB
            permissions,
            privacy_config.clone(),
        ).await.unwrap();

        // Verify privacy config is attached
        assert!(mapping.privacy_config.is_some());
        let attached_privacy = mapping.privacy_config.unwrap();
        assert_eq!(attached_privacy.level, PrivacyLevel::P2P);
        assert_eq!(attached_privacy.allowed_peers.len(), 2);
        assert_eq!(attached_privacy.max_concurrent_access, 5);

        // Clean up
        translator.remove_translation(&global_addr).await.unwrap();
    }

    #[tokio::test]
    async fn test_invalid_privacy_config() {
        let translator = NATTranslator::new().await.unwrap();
        let asset_id = test_asset_id(AssetType::Memory);

        let global_addr = GlobalAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11],
            &asset_id,
            8083,
            GlobalAddressType::Memory,
        );

        let permissions = MemoryPermissions {
            read: true,
            write: true,
            execute: false,
            share: false,
            cache: false,
            prefetch: false,
        };

        // Invalid privacy config - P2P without peers
        let invalid_privacy = PrivacyConfig {
            level: PrivacyLevel::P2P,
            allowed_networks: vec![],
            allowed_peers: vec![], // Should fail - P2P needs peers
            max_concurrent_access: 1,
            require_consensus: false,
        };

        let result = translator.create_translation_with_privacy(
            global_addr,
            4096,
            permissions,
            invalid_privacy,
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("P2P privacy level requires allowed peers"));
    }
}