// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Memory adapter types - allocation records, pools, proxy mappings, and permissions

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::assets::core::{AssetRegistration, PrivacyMode, ProxyAddress};

/// Memory allocation record with NAT-like addressing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryAllocation {
    /// Asset ID
    pub asset_id: AssetRegistration,
    /// Local memory address
    pub local_address: usize,
    /// Size in bytes
    pub size_bytes: u64,
    /// Memory type (DDR4, DDR5, etc.)
    pub memory_type: String,
    /// ECC enabled
    pub ecc_enabled: bool,
    /// NUMA node
    pub numa_node: Option<u32>,
    /// Privacy level
    pub privacy_level: PrivacyMode,
    /// Remote proxy address for NAT-like access
    pub proxy_address: Option<ProxyAddress>,
    /// Allocation timestamp
    pub allocated_at: SystemTime,
    /// Reference count for sharing
    pub reference_count: u32,
    /// Copy-on-write enabled
    pub cow_enabled: bool,
    /// Deduplication hash for memory content
    pub dedup_hash: Option<[u8; 32]>,
}

/// Memory pool for distributed management
#[derive(Clone, Debug)]
pub struct MemoryPool {
    /// Pool identifier
    pub pool_id: String,
    /// Total pool size in bytes
    pub total_size: u64,
    /// Available size in bytes
    pub available_size: u64,
    /// Memory type in pool
    pub memory_type: String,
    /// NUMA node affinity
    pub numa_node: Option<u32>,
    /// Pool privacy level
    pub privacy_level: PrivacyMode,
    /// Active allocations
    pub allocations: Vec<AssetRegistration>,
}

/// Memory proxy address mapping for NAT-like system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryProxyMapping {
    /// Remote proxy address (IPv6-like)
    pub proxy_address: ProxyAddress,
    /// Local asset ID
    pub local_asset_id: AssetRegistration,
    /// Local memory address
    pub local_address: usize,
    /// Size in bytes
    pub size_bytes: u64,
    /// Access permissions
    pub permissions: MemoryPermissions,
    /// Expiration time for security
    pub expires_at: SystemTime,
    /// FALCON-1024 signature for quantum security
    pub access_signature: Vec<u8>,
}

/// Memory access permissions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryPermissions {
    /// Read access allowed
    pub read: bool,
    /// Write access allowed
    pub write: bool,
    /// Execute access allowed (for code segments)
    pub execute: bool,
    /// Share access allowed
    pub share: bool,
}

/// Memory usage statistics
#[derive(Clone, Debug, Default)]
pub struct MemoryUsageStats {
    /// Total allocations made
    pub total_allocations: u64,
    /// Total deallocations made
    pub total_deallocations: u64,
    /// Current active allocations
    pub active_allocations: u64,
    /// Total bytes allocated
    pub total_bytes_allocated: u64,
    /// Total bytes deallocated
    pub total_bytes_deallocated: u64,
    /// Peak memory usage
    pub peak_memory_usage: u64,
    /// Deduplication savings in bytes
    pub dedup_savings_bytes: u64,
    /// Copy-on-write savings in bytes
    pub cow_savings_bytes: u64,
}

/// Memory access types for permission validation
#[derive(Clone, Debug)]
pub(crate) enum _MemoryAccessType {
    _Read,
    _Write,
    _Execute,
    _Share,
}

/// Memory operations for statistics
#[derive(Clone, Debug)]
pub(crate) enum MemoryOperation {
    Allocate,
    Deallocate,
}
