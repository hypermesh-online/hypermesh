// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Entity Asset Coordinator - manages asset allocation across entity blockchains

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;
use uuid::Uuid;

use super::super::PrivacyMode;
use super::types::*;

/// Entity asset allocation coordinator
pub struct EntityAssetCoordinator {
    /// Available assets per entity
    entity_assets: Arc<std::sync::Mutex<HashMap<String, EntityAssetPool>>>,
    /// Active asset allocations
    active_allocations: Arc<std::sync::Mutex<HashMap<String, Vec<ActiveAllocation>>>>,
    /// Asset request queue
    _request_queue: Arc<std::sync::Mutex<Vec<EntityAssetRequest>>>,
}

impl EntityAssetCoordinator {
    pub fn new() -> Self {
        Self {
            entity_assets: Arc::new(std::sync::Mutex::new(HashMap::new())),
            active_allocations: Arc::new(std::sync::Mutex::new(HashMap::new())),
            _request_queue: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn update_entity_pool(
        &self,
        entity_domain: &str,
        config: &EntityVMConfig,
    ) -> Result<()> {
        let pool = EntityAssetPool {
            cpu_available: config.max_external_allocation.get("cpu").copied().unwrap_or(0),
            gpu_available: config.max_external_allocation.get("gpu").copied().unwrap_or(0),
            memory_available: config.max_external_allocation.get("memory").copied().unwrap_or(0),
            storage_available: config.max_external_allocation.get("storage").copied().unwrap_or(0),
            privacy_constraints: EntityPrivacyConstraints {
                entity_domain: entity_domain.to_string(),
                max_compute_allocation: config.max_external_allocation.clone(),
                allowed_operations: vec![],
                resource_privacy_level: PrivacyMode::PRIVATE,
                max_duration_seconds: 3600,
            },
        };

        self.entity_assets.lock().expect("mutex poisoned")
            .insert(entity_domain.to_string(), pool);

        Ok(())
    }

    pub async fn allocate_asset_from_entity(
        &self,
        request: &EntityAssetRequest,
    ) -> Result<EntityAssetAllocation> {
        let mut entity_assets = self.entity_assets.lock().expect("mutex poisoned");
        let pool = entity_assets.get_mut(&request.entity_domain)
            .ok_or_else(|| anyhow::anyhow!("Entity not found: {}", request.entity_domain))?;

        let available = match request.asset_type.as_str() {
            "cpu" => pool.cpu_available,
            "gpu" => pool.gpu_available,
            "memory" => pool.memory_available,
            "storage" => pool.storage_available,
            _ => return Err(anyhow::anyhow!("Unknown asset type: {}", request.asset_type)),
        };

        if available < request.requested_amount {
            return Err(anyhow::anyhow!(
                "Insufficient {} available from {}: requested {}, available {}",
                request.asset_type, request.entity_domain, request.requested_amount, available
            ));
        }

        let allocation_id = Uuid::new_v4();
        let allocation = EntityAssetAllocation {
            allocation_id,
            entity_domain: request.entity_domain.clone(),
            asset_type: request.asset_type.clone(),
            allocated_capacity: request.requested_amount,
            total_capacity: available,
            privacy_level: pool.privacy_constraints.resource_privacy_level.clone(),
            expires_at: std::time::SystemTime::now() +
                std::time::Duration::from_secs(request.duration_seconds),
        };

        match request.asset_type.as_str() {
            "cpu" => pool.cpu_available -= request.requested_amount,
            "gpu" => pool.gpu_available -= request.requested_amount,
            "memory" => pool.memory_available -= request.requested_amount,
            "storage" => pool.storage_available -= request.requested_amount,
            _ => {},
        }

        let active_allocation = ActiveAllocation {
            allocation_id,
            _entity_domain: request.entity_domain.clone(),
            asset_type: request.asset_type.clone(),
            allocated_amount: request.requested_amount,
            _start_time: std::time::SystemTime::now(),
            _expires_at: allocation.expires_at,
            _executing_workflow: None,
        };

        self.active_allocations.lock().expect("mutex poisoned")
            .entry(request.entity_domain.clone())
            .or_insert_with(Vec::new)
            .push(active_allocation);

        Ok(allocation)
    }

    pub async fn release_allocation(&self, allocation_id: &Uuid) -> Result<()> {
        let mut active_allocations = self.active_allocations.lock().expect("mutex poisoned");

        for (entity_domain, allocations) in active_allocations.iter_mut() {
            if let Some(pos) = allocations.iter().position(|a| &a.allocation_id == allocation_id) {
                let allocation = allocations.remove(pos);

                let mut entity_assets = self.entity_assets.lock().expect("mutex poisoned");
                if let Some(pool) = entity_assets.get_mut(entity_domain) {
                    match allocation.asset_type.as_str() {
                        "cpu" => pool.cpu_available += allocation.allocated_amount,
                        "gpu" => pool.gpu_available += allocation.allocated_amount,
                        "memory" => pool.memory_available += allocation.allocated_amount,
                        "storage" => pool.storage_available += allocation.allocated_amount,
                        _ => {},
                    }
                }

                return Ok(());
            }
        }

        Err(anyhow::anyhow!("Allocation not found: {}", allocation_id))
    }
}
