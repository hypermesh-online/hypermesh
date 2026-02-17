// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! NATTranslator implementation - core translation operations

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use sha2::{Digest, Sha256};
use libc::{mmap, munmap, PROT_READ, PROT_WRITE, PROT_EXEC, MAP_PRIVATE, MAP_ANONYMOUS, MAP_FAILED};

use crate::assets::core::{AssetId, AssetResult, AssetError, ProxyAddress};
use super::types::*;

/// The main NAT translator for memory addressing
#[allow(dead_code)] // Fields used during NAT translation operations
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

impl NATTranslator {
    /// Create new NAT translator
    pub async fn new() -> AssetResult<Self> {
        let network_config = NetworkConfig {
            network_prefix: [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad],
            local_node_id: Self::generate_local_node_id(),
            address_space_start: 0x1000_0000,
            address_space_size: 0x4000_0000,
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
        let mut node_bytes = [0u8; 8];
        let node_id_bytes = node_id.as_bytes();
        let copy_len = node_id_bytes.len().min(8);
        node_bytes[..copy_len].copy_from_slice(&node_id_bytes[..copy_len]);

        let _global_addr = GlobalAddress::new(
            self.network_config.network_prefix,
            node_bytes,
            asset_id,
            service_port,
            GlobalAddressType::Memory,
        );

        let mut network_prefix_16 = [0u8; 16];
        network_prefix_16[..8].copy_from_slice(&self.network_config.network_prefix);
        let proxy_addr = ProxyAddress::new(network_prefix_16, node_bytes, service_port);

        Ok(proxy_addr)
    }

    /// Create NAT translation mapping with real memory
    #[allow(unsafe_code)]
    pub async fn create_translation(
        &self,
        global_addr: GlobalAddress,
        region_size: u64,
        permissions: MemoryPermissions,
    ) -> AssetResult<LocalAddressMapping> {
        let start_time = SystemTime::now();

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

        let mapping = LocalAddressMapping {
            global_address: global_addr.clone(),
            local_address,
            region_size,
            access_permissions: permissions,
            privacy_config: None,
            translation_state: TranslationState::Active,
            usage_stats: AddressUsageStats::default(),
            last_accessed: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
        };

        {
            let mut global_to_local = self.global_to_local.write().await;
            global_to_local.insert(global_addr.clone(), mapping.clone());
        }

        let global_addr_str = global_addr.to_string();

        {
            let mut local_to_global = self.local_to_global.write().await;
            local_to_global.insert(local_address, global_addr);
        }

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
        self.validate_privacy_config(&privacy).await?;

        let mut mapping = self.create_translation(global_addr, region_size, permissions).await?;
        mapping.privacy_config = Some(privacy);

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
        if privacy.max_concurrent_access == 0 {
            return Err(AssetError::AdapterError {
                message: "Max concurrent access must be greater than 0".to_string()
            });
        }

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
            PrivacyLevel::FullPublic => {},
        }

        Ok(())
    }

    /// Translate global address to local address
    pub async fn translate_to_local(&self, global_addr: &GlobalAddress) -> AssetResult<usize> {
        let global_to_local = self.global_to_local.read().await;
        let mapping = global_to_local.get(global_addr)
            .ok_or_else(|| AssetError::AdapterError {
                message: format!("No translation found for global address: {}", global_addr.to_string())
            })?;

        if !matches!(mapping.translation_state, TranslationState::Active) {
            return Err(AssetError::AdapterError {
                message: "Translation is not active".to_string()
            });
        }

        if mapping.expires_at < SystemTime::now() {
            return Err(AssetError::AdapterError {
                message: "Translation has expired".to_string()
            });
        }

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
    #[allow(unsafe_code)]
    pub async fn remove_translation(&self, global_addr: &GlobalAddress) -> AssetResult<()> {
        let mapping = {
            let mut global_to_local = self.global_to_local.write().await;
            global_to_local.remove(global_addr)
        };

        if let Some(mapping) = mapping {
            {
                let mut local_to_global = self.local_to_global.write().await;
                local_to_global.remove(&mapping.local_address);
            }

            unsafe {
                let result = munmap(mapping.local_address as *mut libc::c_void, mapping.region_size as usize);
                if result != 0 {
                    return Err(AssetError::AdapterError {
                        message: format!("munmap failed: {}", std::io::Error::last_os_error())
                    });
                }
            }

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

    /// Generate local node ID
    fn generate_local_node_id() -> [u8; 8] {
        let mut hasher = Sha256::new();

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
