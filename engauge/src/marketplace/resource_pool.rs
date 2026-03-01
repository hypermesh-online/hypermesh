// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Node operator resource allocation configuration.
//!
//! Each node operator configures which resources they are willing to lease
//! and under what constraints. Resource allocation is always sovereign --
//! the node operator sets percentages, and the protocol enforces limits.

use std::collections::HashMap;
use std::time::Duration;

use hypermesh_lib::economic::MarketTier;
use hypermesh_lib::{NodeId, PrivacyMode};
use serde::{Deserialize, Serialize};

/// System asset kinds that can be leased on the marketplace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeaseableResource {
    Cpu,
    Gpu,
    Memory,
    Storage,
    Bandwidth,
}

/// Per-resource allocation configuration set by the node operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationConfig {
    /// Percentage of this resource allocated to the lease pool (0-100).
    pub percentage: u8,
    /// Minimum lease duration.
    pub min_lease_duration: Duration,
    /// Maximum lease duration.
    pub max_lease_duration: Duration,
    /// Which market tiers are accepted.
    pub accepted_tiers: Vec<MarketTier>,
    /// Who can lease (federation/public). Anonymous cannot participate.
    pub privacy_scope: PrivacyMode,
}

impl AllocationConfig {
    /// Create a config with the given percentage (clamped 0-100).
    pub fn new(percentage: u8, privacy_scope: PrivacyMode) -> Self {
        Self {
            percentage: percentage.min(100),
            min_lease_duration: Duration::from_secs(60),
            max_lease_duration: Duration::from_secs(86400),
            accepted_tiers: vec![
                MarketTier::L0,
                MarketTier::L1,
                MarketTier::L2,
                MarketTier::L3,
            ],
            privacy_scope,
        }
    }
}

/// Per-node resource pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePool {
    pub node_id: NodeId,
    pub allocations: HashMap<LeaseableResource, AllocationConfig>,
}

impl ResourcePool {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            allocations: HashMap::new(),
        }
    }

    /// Set allocation for a resource. Rejects Anonymous privacy mode.
    pub fn set_allocation(
        &mut self,
        resource: LeaseableResource,
        config: AllocationConfig,
    ) -> Result<(), PoolError> {
        if config.privacy_scope == PrivacyMode::ANONYMOUS {
            return Err(PoolError::AnonymousNotAllowed);
        }
        if config.percentage > 100 {
            return Err(PoolError::InvalidPercentage(config.percentage));
        }
        self.allocations.insert(resource, config);
        Ok(())
    }

    /// Get allocation for a resource (None if not configured).
    pub fn get_allocation(&self, resource: &LeaseableResource) -> Option<&AllocationConfig> {
        self.allocations.get(resource)
    }

    /// Total number of configured resources.
    pub fn resource_count(&self) -> usize {
        self.allocations.len()
    }

    /// Check if a given tier is accepted for a resource.
    pub fn accepts_tier(&self, resource: &LeaseableResource, tier: &MarketTier) -> bool {
        self.allocations
            .get(resource)
            .map(|c| c.accepted_tiers.contains(tier))
            .unwrap_or(false)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("anonymous nodes cannot participate in the marketplace")]
    AnonymousNotAllowed,
    #[error("invalid allocation percentage: {0} (must be 0-100)")]
    InvalidPercentage(u8),
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node() -> NodeId {
        NodeId::from_public_key(b"pool-test-node")
    }

    #[test]
    fn create_pool_and_set_allocation() {
        let mut pool = ResourcePool::new(test_node());
        let config = AllocationConfig::new(50, PrivacyMode::PUBLIC);
        pool.set_allocation(LeaseableResource::Cpu, config)
            .expect("test: set CPU allocation");
        assert_eq!(pool.resource_count(), 1);

        let alloc = pool
            .get_allocation(&LeaseableResource::Cpu)
            .expect("test: get CPU allocation");
        assert_eq!(alloc.percentage, 50);
    }

    #[test]
    fn reject_anonymous_privacy_mode() {
        let mut pool = ResourcePool::new(test_node());
        let config = AllocationConfig::new(30, PrivacyMode::ANONYMOUS);
        let result = pool.set_allocation(LeaseableResource::Gpu, config);
        assert!(result.is_err());
        match result {
            Err(PoolError::AnonymousNotAllowed) => {}
            other => unreachable!("test: expected AnonymousNotAllowed, got {other:?}"),
        }
    }

    #[test]
    fn percentage_clamped_to_100() {
        let config = AllocationConfig::new(200, PrivacyMode::PRIVATE);
        assert_eq!(config.percentage, 100);
    }

    #[test]
    fn get_allocation_returns_correct_config() {
        let mut pool = ResourcePool::new(test_node());
        let cpu_config = AllocationConfig::new(25, PrivacyMode::PRIVATE);
        let gpu_config = AllocationConfig::new(75, PrivacyMode::PUBLIC);

        pool.set_allocation(LeaseableResource::Cpu, cpu_config)
            .expect("test: set CPU");
        pool.set_allocation(LeaseableResource::Gpu, gpu_config)
            .expect("test: set GPU");

        let cpu = pool
            .get_allocation(&LeaseableResource::Cpu)
            .expect("test: get CPU");
        assert_eq!(cpu.percentage, 25);
        assert_eq!(cpu.privacy_scope, PrivacyMode::PRIVATE);

        let gpu = pool
            .get_allocation(&LeaseableResource::Gpu)
            .expect("test: get GPU");
        assert_eq!(gpu.percentage, 75);
        assert_eq!(gpu.privacy_scope, PrivacyMode::PUBLIC);
    }

    #[test]
    fn accepts_tier_for_configured_resources() {
        let mut pool = ResourcePool::new(test_node());
        let mut config = AllocationConfig::new(40, PrivacyMode::PUBLIC);
        config.accepted_tiers = vec![MarketTier::L0, MarketTier::L1];
        pool.set_allocation(LeaseableResource::Storage, config)
            .expect("test: set storage");

        assert!(pool.accepts_tier(&LeaseableResource::Storage, &MarketTier::L0));
        assert!(pool.accepts_tier(&LeaseableResource::Storage, &MarketTier::L1));
        assert!(!pool.accepts_tier(&LeaseableResource::Storage, &MarketTier::L2));
        assert!(!pool.accepts_tier(&LeaseableResource::Storage, &MarketTier::L3));
    }

    #[test]
    fn accepts_tier_false_for_unconfigured_resource() {
        let pool = ResourcePool::new(test_node());
        assert!(!pool.accepts_tier(&LeaseableResource::Bandwidth, &MarketTier::L0));
    }
}
