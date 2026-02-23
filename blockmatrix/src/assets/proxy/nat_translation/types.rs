// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Type definitions for the NAT-like Address Translation System

use std::time::SystemTime;
use serde::{Deserialize, Serialize};


// Re-export PrivacyMode from hypermesh_lib via core
pub use crate::assets::core::PrivacyMode;

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

/// Privacy configuration for NAT translations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Privacy level
    pub level: PrivacyMode,
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

/// Address allocation management
#[derive(Debug)]
pub(crate) struct AddressAllocator {
    /// Next available local address
    pub _next_local_address: usize,
    /// Address space size
    pub _address_space_size: u64,
    /// Allocated address ranges
    pub _allocated_ranges: Vec<AddressRange>,
    /// Free address ranges
    pub _free_ranges: Vec<AddressRange>,
}

/// Address range specification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AddressRange {
    /// Start address
    pub start: usize,
    /// End address (inclusive)
    pub end: usize,
    /// Size in bytes
    pub size: u64,
}

/// Network configuration for NAT translation
#[derive(Clone, Debug)]
pub(crate) struct NetworkConfig {
    /// HyperMesh network prefix
    pub network_prefix: [u8; 8],
    /// Local node identifier
    pub _local_node_id: [u8; 8],
    /// Address space start
    pub address_space_start: usize,
    /// Address space size
    pub address_space_size: u64,
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
