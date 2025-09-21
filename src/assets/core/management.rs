//! Asset Management
//!
//! Core asset management functionality for HyperMesh.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::SystemTime;
use dashmap::DashMap;
use tracing::{info, debug, warn};
use uuid::Uuid;

use super::types::*;

/// Asset Manager for HyperMesh
pub struct AssetManager {
    /// Asset registry
    assets: Arc<DashMap<AssetId, Arc<Asset>>>,

    /// Assets by type index
    assets_by_type: Arc<DashMap<AssetType, Vec<AssetId>>>,

    /// Assets by owner index
    assets_by_owner: Arc<DashMap<String, Vec<AssetId>>>,
}

impl AssetManager {
    /// Create new asset manager
    pub fn new() -> Self {
        Self {
            assets: Arc::new(DashMap::new()),
            assets_by_type: Arc::new(DashMap::new()),
            assets_by_owner: Arc::new(DashMap::new()),
        }
    }

    /// Register a new asset
    pub fn register_asset(&self, mut asset: Asset) -> Result<AssetId> {
        // Generate ID if not provided
        if asset.id.is_empty() {
            asset.id = format!("asset_{}", Uuid::new_v4());
        }

        let asset_id = asset.id.clone();
        let asset_type = asset.asset_type;
        let owner = asset.owner.clone();

        // Set timestamps
        let now = SystemTime::now();
        asset.created_at = now;
        asset.updated_at = now;

        // Store asset
        let asset_arc = Arc::new(asset);
        self.assets.insert(asset_id.clone(), asset_arc);

        // Update indices
        self.assets_by_type
            .entry(asset_type)
            .or_insert_with(Vec::new)
            .push(asset_id.clone());

        self.assets_by_owner
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(asset_id.clone());

        info!("Registered asset: {} (type: {})", asset_id, asset_type);
        Ok(asset_id)
    }

    /// Get asset by ID
    pub fn get_asset(&self, asset_id: &str) -> Result<Arc<Asset>> {
        self.assets
            .get(asset_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| anyhow!("Asset not found: {}", asset_id))
    }

    /// Update asset status
    pub fn update_status(&self, asset_id: &str, status: AssetStatus) -> Result<()> {
        let mut asset = self.assets
            .get_mut(asset_id)
            .ok_or_else(|| anyhow!("Asset not found: {}", asset_id))?;

        // Clone and update
        let mut updated_asset = (**asset).clone();
        updated_asset.status = status;
        updated_asset.updated_at = SystemTime::now();

        *asset = Arc::new(updated_asset);

        debug!("Updated asset {} status to {:?}", asset_id, status);
        Ok(())
    }

    /// Update asset allocation
    pub fn update_allocation(
        &self,
        asset_id: &str,
        allocated: ResourceAllocation,
    ) -> Result<()> {
        let mut asset = self.assets
            .get_mut(asset_id)
            .ok_or_else(|| anyhow!("Asset not found: {}", asset_id))?;

        // Clone and update
        let mut updated_asset = (**asset).clone();
        updated_asset.allocated = allocated.clone();

        // Calculate available resources
        updated_asset.available = Self::calculate_available(
            &updated_asset.capacity,
            &allocated,
        )?;

        // Update status based on allocation
        updated_asset.status = Self::determine_status(&updated_asset);
        updated_asset.updated_at = SystemTime::now();

        *asset = Arc::new(updated_asset);

        debug!("Updated asset {} allocation", asset_id);
        Ok(())
    }

    /// Calculate available resources
    fn calculate_available(
        capacity: &ResourceAllocation,
        allocated: &ResourceAllocation,
    ) -> Result<ResourceAllocation> {
        let mut available = ResourceAllocation::empty();

        if let (Some(cap), Some(alloc)) = (capacity.cpu_units, allocated.cpu_units) {
            available.cpu_units = Some((cap - alloc).max(0.0));
        }

        if let (Some(cap), Some(alloc)) = (capacity.gpu_units, allocated.gpu_units) {
            available.gpu_units = Some((cap - alloc).max(0.0));
        }

        if let (Some(cap), Some(alloc)) = (capacity.memory_bytes, allocated.memory_bytes) {
            available.memory_bytes = Some(cap.saturating_sub(alloc));
        }

        if let (Some(cap), Some(alloc)) = (capacity.storage_bytes, allocated.storage_bytes) {
            available.storage_bytes = Some(cap.saturating_sub(alloc));
        }

        if let (Some(cap), Some(alloc)) = (capacity.bandwidth_bps, allocated.bandwidth_bps) {
            available.bandwidth_bps = Some(cap.saturating_sub(alloc));
        }

        Ok(available)
    }

    /// Determine asset status based on allocation
    fn determine_status(asset: &Asset) -> AssetStatus {
        if asset.available.is_empty() {
            AssetStatus::FullyAllocated
        } else if asset.allocated.is_empty() {
            AssetStatus::Available
        } else {
            AssetStatus::PartiallyAllocated
        }
    }

    /// Get assets by type
    pub fn get_assets_by_type(&self, asset_type: AssetType) -> Vec<Arc<Asset>> {
        self.assets_by_type
            .get(&asset_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.assets.get(id).map(|e| e.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get assets by owner
    pub fn get_assets_by_owner(&self, owner: &str) -> Vec<Arc<Asset>> {
        self.assets_by_owner
            .get(owner)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.assets.get(id).map(|e| e.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find available assets matching criteria
    pub fn find_available_assets(
        &self,
        asset_type: AssetType,
        min_resources: &ResourceAllocation,
        privacy_level: Option<PrivacyLevel>,
    ) -> Vec<Arc<Asset>> {
        self.get_assets_by_type(asset_type)
            .into_iter()
            .filter(|asset| {
                // Check status
                if !matches!(
                    asset.status,
                    AssetStatus::Available | AssetStatus::PartiallyAllocated
                ) {
                    return false;
                }

                // Check privacy level
                if let Some(level) = privacy_level {
                    if asset.privacy_level != level {
                        return false;
                    }
                }

                // Check available resources
                Self::has_sufficient_resources(&asset.available, min_resources)
            })
            .collect()
    }

    /// Check if asset has sufficient resources
    fn has_sufficient_resources(
        available: &ResourceAllocation,
        required: &ResourceAllocation,
    ) -> bool {
        if let (Some(avail), Some(req)) = (available.cpu_units, required.cpu_units) {
            if avail < req {
                return false;
            }
        }

        if let (Some(avail), Some(req)) = (available.gpu_units, required.gpu_units) {
            if avail < req {
                return false;
            }
        }

        if let (Some(avail), Some(req)) = (available.memory_bytes, required.memory_bytes) {
            if avail < req {
                return false;
            }
        }

        if let (Some(avail), Some(req)) = (available.storage_bytes, required.storage_bytes) {
            if avail < req {
                return false;
            }
        }

        if let (Some(avail), Some(req)) = (available.bandwidth_bps, required.bandwidth_bps) {
            if avail < req {
                return false;
            }
        }

        true
    }

    /// Decommission an asset
    pub fn decommission_asset(&self, asset_id: &str) -> Result<()> {
        // Update status
        self.update_status(asset_id, AssetStatus::Decommissioned)?;

        // Remove from indices
        if let Some(asset) = self.assets.get(asset_id) {
            // Remove from type index
            if let Some(mut ids) = self.assets_by_type.get_mut(&asset.asset_type) {
                ids.retain(|id| id != asset_id);
            }

            // Remove from owner index
            if let Some(mut ids) = self.assets_by_owner.get_mut(&asset.owner) {
                ids.retain(|id| id != asset_id);
            }
        }

        warn!("Decommissioned asset: {}", asset_id);
        Ok(())
    }

    /// Get asset statistics
    pub fn get_statistics(&self) -> AssetSystemStats {
        let mut stats = AssetSystemStats::default();

        for entry in self.assets.iter() {
            let asset = entry.value();
            stats.total_assets += 1;

            match asset.status {
                AssetStatus::Available => stats.available_assets += 1,
                AssetStatus::PartiallyAllocated => stats.partially_allocated += 1,
                AssetStatus::FullyAllocated => stats.fully_allocated += 1,
                AssetStatus::Offline => stats.offline_assets += 1,
                _ => {}
            }

            // Sum statistics
            stats.total_allocations += asset.statistics.total_allocations;
            stats.total_operations += asset.statistics.operations_completed;
            stats.total_tokens_earned += asset.statistics.tokens_earned;
        }

        stats
    }
}

/// Asset system statistics
#[derive(Debug, Clone, Default)]
pub struct AssetSystemStats {
    pub total_assets: usize,
    pub available_assets: usize,
    pub partially_allocated: usize,
    pub fully_allocated: usize,
    pub offline_assets: usize,
    pub total_allocations: u64,
    pub total_operations: u64,
    pub total_tokens_earned: f64,
}