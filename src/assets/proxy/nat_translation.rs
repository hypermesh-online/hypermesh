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
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
        asset_bytes.copy_from_slice(asset_id.uuid.as_bytes());
        
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
        let proxy_addr = ProxyAddress::new(
            self.network_config.network_prefix.try_into().unwrap_or([0u8; 16]),
            node_bytes,
            service_port,
        );
        
        Ok(proxy_addr)
    }
    
    /// Create NAT translation mapping
    pub async fn create_translation(
        &self,
        global_addr: GlobalAddress,
        region_size: u64,
        permissions: MemoryPermissions,
    ) -> AssetResult<LocalAddressMapping> {
        
        let start_time = SystemTime::now();
        
        // Allocate local address
        let local_address = self.allocate_local_address(region_size).await?;
        
        // Create mapping
        let mapping = LocalAddressMapping {
            global_address: global_addr.clone(),
            local_address,
            region_size,
            access_permissions: permissions,
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
            "Created NAT translation: {} -> 0x{:x} ({} bytes)",
            global_addr.to_string(),
            local_address,
            region_size
        );
        
        Ok(mapping)
    }
    
    /// Translate global address to local address
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
    
    /// Remove translation
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
            
            // Free local address
            self.free_local_address(mapping.local_address, mapping.region_size).await?;
            
            // Update statistics
            {
                let mut stats = self.translation_stats.write().await;
                stats.active_translations = stats.active_translations.saturating_sub(1);
                stats.total_memory_mapped = stats.total_memory_mapped.saturating_sub(mapping.region_size);
            }
            
            tracing::info!(
                "Removed NAT translation: {} -> 0x{:x}",
                global_addr.to_string(),
                mapping.local_address
            );
        }
        
        Ok(())
    }
    
    /// Allocate local address from address space
    async fn allocate_local_address(&self, size: u64) -> AssetResult<usize> {
        let mut allocator = self.address_allocator.write().await;
        
        // Find suitable free range
        for (i, range) in allocator.free_ranges.iter().enumerate() {
            if range.size >= size {
                let allocated_addr = range.start;
                
                // Update free ranges
                if range.size == size {
                    // Exact fit - remove the range
                    allocator.free_ranges.remove(i);
                } else {
                    // Split the range
                    allocator.free_ranges[i] = AddressRange {
                        start: range.start + size as usize,
                        end: range.end,
                        size: range.size - size,
                    };
                }
                
                // Add to allocated ranges
                allocator.allocated_ranges.push(AddressRange {
                    start: allocated_addr,
                    end: allocated_addr + size as usize - 1,
                    size,
                });
                
                return Ok(allocated_addr);
            }
        }
        
        Err(AssetError::AdapterError {
            message: format!("Cannot allocate {} bytes - insufficient address space", size)
        })
    }
    
    /// Free local address back to address space
    async fn free_local_address(&self, addr: usize, size: u64) -> AssetResult<()> {
        let mut allocator = self.address_allocator.write().await;
        
        // Remove from allocated ranges
        allocator.allocated_ranges.retain(|range| range.start != addr);
        
        // Add back to free ranges
        let free_range = AddressRange {
            start: addr,
            end: addr + size as usize - 1,
            size,
        };
        
        // Insert in sorted order and merge adjacent ranges
        let mut insert_pos = allocator.free_ranges.len();
        for (i, range) in allocator.free_ranges.iter().enumerate() {
            if range.start > addr {
                insert_pos = i;
                break;
            }
        }
        
        allocator.free_ranges.insert(insert_pos, free_range);
        
        // Merge adjacent ranges
        self.merge_free_ranges(&mut allocator.free_ranges);
        
        Ok(())
    }
    
    /// Merge adjacent free ranges for efficiency
    fn merge_free_ranges(&self, ranges: &mut Vec<AddressRange>) {
        ranges.sort_by_key(|r| r.start);
        
        let mut i = 0;
        while i + 1 < ranges.len() {
            if ranges[i].end + 1 == ranges[i + 1].start {
                // Merge ranges
                ranges[i].end = ranges[i + 1].end;
                ranges[i].size += ranges[i + 1].size;
                ranges.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }
    
    /// Generate local node ID
    fn generate_local_node_id() -> [u8; 8] {
        // TODO: Generate based on actual node characteristics
        // For now, use a hash of current time and hostname
        let mut hasher = Sha256::new();
        hasher.update(&SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_le_bytes());
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
    
    #[test]
    fn test_global_address_creation() {
        let asset_id = AssetId::new(AssetType::Memory);
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
        let asset_id = AssetId::new(AssetType::Memory);
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
        let asset_id = AssetId::new(AssetType::Memory);
        
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
}