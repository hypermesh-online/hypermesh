// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Manages active leases and matches supply with demand.
//!
//! The [`LeaseManager`] holds provider resource pools and tracks all
//! lease contracts through their lifecycle.

use std::collections::HashMap;

use hypermesh_lib::economic::{GoldGrams, MarketTier};
use hypermesh_lib::NodeId;

use super::lease_contract::{LeaseContract, LeaseError, LeaseState};
use super::resource_pool::{LeaseableResource, ResourcePool};

/// Manages provider pools and lease contracts.
pub struct LeaseManager {
    /// Provider pools: node_id string -> ResourcePool.
    pools: HashMap<String, ResourcePool>,
    /// All leases by lease_id string.
    leases: HashMap<String, LeaseContract>,
}

impl LeaseManager {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            leases: HashMap::new(),
        }
    }

    /// Register a provider's resource pool.
    pub fn register_pool(&mut self, pool: ResourcePool) {
        self.pools.insert(pool.node_id.0.clone(), pool);
    }

    /// Create a lease proposal between consumer and provider.
    ///
    /// Validates that the provider has a pool, the resource is configured,
    /// the requested allocation does not exceed the pool limit, and the
    /// tier is accepted. Returns the lease ID on success.
    pub fn propose_lease(
        &mut self,
        provider_id: &NodeId,
        consumer_id: NodeId,
        resource: LeaseableResource,
        allocation_pct: u8,
        price_per_epoch: GoldGrams,
        tier: MarketTier,
        duration: chrono::Duration,
    ) -> Result<String, ManagerError> {
        let pool = self
            .pools
            .get(&provider_id.0)
            .ok_or_else(|| ManagerError::ProviderNotFound(provider_id.0.clone()))?;

        let alloc = pool
            .get_allocation(&resource)
            .ok_or(ManagerError::ResourceNotConfigured(resource))?;

        if allocation_pct > alloc.percentage {
            return Err(ManagerError::AllocationExceeded {
                requested: allocation_pct,
                available: alloc.percentage,
            });
        }

        if !alloc.accepted_tiers.contains(&tier) {
            return Err(ManagerError::TierNotAccepted(tier));
        }

        let contract = LeaseContract::propose(
            provider_id.clone(),
            consumer_id,
            resource,
            allocation_pct,
            price_per_epoch,
            tier,
            duration,
        );

        let lease_id = contract.lease_id.0.clone();
        self.leases.insert(lease_id.clone(), contract);
        Ok(lease_id)
    }

    /// Activate a proposed lease (provider confirms).
    pub fn activate_lease(&mut self, lease_id: &str) -> Result<(), ManagerError> {
        let contract = self
            .leases
            .get_mut(lease_id)
            .ok_or_else(|| ManagerError::LeaseNotFound(lease_id.to_string()))?;
        contract.activate()?;
        Ok(())
    }

    /// Get a lease by ID.
    pub fn get_lease(&self, lease_id: &str) -> Option<&LeaseContract> {
        self.leases.get(lease_id)
    }

    /// List active leases for a provider.
    pub fn active_leases_for_provider(&self, provider_id: &str) -> Vec<&LeaseContract> {
        self.leases
            .values()
            .filter(|l| l.provider.0 == provider_id && l.state == LeaseState::Active)
            .collect()
    }

    /// List active leases for a consumer.
    pub fn active_leases_for_consumer(&self, consumer_id: &str) -> Vec<&LeaseContract> {
        self.leases
            .values()
            .filter(|l| l.consumer.0 == consumer_id && l.state == LeaseState::Active)
            .collect()
    }

    /// Total active lease count.
    pub fn active_lease_count(&self) -> usize {
        self.leases
            .values()
            .filter(|l| l.state == LeaseState::Active)
            .count()
    }

    /// Cancel a lease.
    pub fn cancel_lease(&mut self, lease_id: &str) -> Result<(), ManagerError> {
        let contract = self
            .leases
            .get_mut(lease_id)
            .ok_or_else(|| ManagerError::LeaseNotFound(lease_id.to_string()))?;
        contract.cancel()?;
        Ok(())
    }
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManagerError {
    #[error("provider not found: {0}")]
    ProviderNotFound(String),
    #[error("resource not configured: {0:?}")]
    ResourceNotConfigured(LeaseableResource),
    #[error("allocation exceeds pool limit: requested {requested}%, available {available}%")]
    AllocationExceeded { requested: u8, available: u8 },
    #[error("tier not accepted: {0:?}")]
    TierNotAccepted(MarketTier),
    #[error("lease not found: {0}")]
    LeaseNotFound(String),
    #[error("lease error: {0}")]
    LeaseError(#[from] LeaseError),
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketplace::resource_pool::AllocationConfig;
    use hypermesh_lib::PrivacyMode;
    use rust_decimal::Decimal;

    fn setup_manager() -> (LeaseManager, NodeId) {
        let mut mgr = LeaseManager::new();
        let provider = NodeId::from("provider-node");
        let mut pool = ResourcePool::new(provider.clone());
        let config = AllocationConfig::new(50, PrivacyMode::PUBLIC);
        pool.set_allocation(LeaseableResource::Cpu, config)
            .expect("test: set CPU allocation");
        mgr.register_pool(pool);
        (mgr, provider)
    }

    fn test_price() -> GoldGrams {
        GoldGrams::from_decimal(Decimal::new(1, 3))
    }

    #[test]
    fn register_pool_and_propose_lease() {
        let (mut mgr, provider) = setup_manager();
        let consumer = NodeId::from("consumer-node");
        let lease_id = mgr
            .propose_lease(
                &provider,
                consumer,
                LeaseableResource::Cpu,
                30,
                test_price(),
                MarketTier::L0,
                chrono::Duration::hours(1),
            )
            .expect("test: propose lease");

        let lease = mgr.get_lease(&lease_id).expect("test: get lease");
        assert_eq!(lease.state, LeaseState::Proposed);
        assert_eq!(lease.allocation_percentage, 30);
    }

    #[test]
    fn propose_fails_with_unregistered_provider() {
        let mut mgr = LeaseManager::new();
        let unknown = NodeId::from("unknown-provider");
        let consumer = NodeId::from("consumer-node");

        let result = mgr.propose_lease(
            &unknown,
            consumer,
            LeaseableResource::Cpu,
            10,
            test_price(),
            MarketTier::L0,
            chrono::Duration::hours(1),
        );
        assert!(result.is_err());
        match result {
            Err(ManagerError::ProviderNotFound(id)) => assert_eq!(id, "unknown-provider"),
            other => panic!("test: expected ProviderNotFound, got {:?}", other),
        }
    }

    #[test]
    fn propose_fails_with_unconfigured_resource() {
        let (mut mgr, provider) = setup_manager();
        let consumer = NodeId::from("consumer-node");

        let result = mgr.propose_lease(
            &provider,
            consumer,
            LeaseableResource::Gpu, // not configured
            10,
            test_price(),
            MarketTier::L0,
            chrono::Duration::hours(1),
        );
        assert!(result.is_err());
        match result {
            Err(ManagerError::ResourceNotConfigured(r)) => {
                assert_eq!(r, LeaseableResource::Gpu);
            }
            other => panic!("test: expected ResourceNotConfigured, got {:?}", other),
        }
    }

    #[test]
    fn propose_fails_when_allocation_exceeds_pool_limit() {
        let (mut mgr, provider) = setup_manager();
        let consumer = NodeId::from("consumer-node");

        let result = mgr.propose_lease(
            &provider,
            consumer,
            LeaseableResource::Cpu,
            80, // pool limit is 50
            test_price(),
            MarketTier::L0,
            chrono::Duration::hours(1),
        );
        assert!(result.is_err());
        match result {
            Err(ManagerError::AllocationExceeded {
                requested,
                available,
            }) => {
                assert_eq!(requested, 80);
                assert_eq!(available, 50);
            }
            other => panic!("test: expected AllocationExceeded, got {:?}", other),
        }
    }

    #[test]
    fn active_lease_counting() {
        let (mut mgr, provider) = setup_manager();
        assert_eq!(mgr.active_lease_count(), 0);

        let consumer = NodeId::from("consumer-node");
        let lease_id = mgr
            .propose_lease(
                &provider,
                consumer.clone(),
                LeaseableResource::Cpu,
                20,
                test_price(),
                MarketTier::L0,
                chrono::Duration::hours(1),
            )
            .expect("test: propose lease");

        // Proposed does not count as active.
        assert_eq!(mgr.active_lease_count(), 0);

        mgr.activate_lease(&lease_id)
            .expect("test: activate lease");
        assert_eq!(mgr.active_lease_count(), 1);

        let provider_leases = mgr.active_leases_for_provider("provider-node");
        assert_eq!(provider_leases.len(), 1);

        let consumer_leases = mgr.active_leases_for_consumer("consumer-node");
        assert_eq!(consumer_leases.len(), 1);

        mgr.cancel_lease(&lease_id).expect("test: cancel lease");
        assert_eq!(mgr.active_lease_count(), 0);
    }
}
