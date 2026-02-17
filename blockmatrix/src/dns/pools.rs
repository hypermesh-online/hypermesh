// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS Pool Management
//!
//! Manages public and federated DNS pools with privacy boundaries.

use super::{DnsRecord, DnsError, DnsResult};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

/// DNS pool type
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DnsPoolType {
    /// Public pool - globally queryable, blockchain-registered
    Public,
    /// Federated pool - network-scoped, isolated
    Federated { network_id: String },
}

/// Pool visibility
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PoolVisibility {
    /// Publicly accessible
    Public,
    /// Network-restricted
    NetworkRestricted,
    /// Fully federated (no public access)
    FullyFederated,
}

/// DNS pool
#[derive(Clone, Debug)]
pub struct DnsPool {
    /// Pool identifier
    pub pool_id: String,
    /// Pool type
    pub pool_type: DnsPoolType,
    /// Pool visibility
    pub visibility: PoolVisibility,
    /// DNS records (domain -> records)
    records: Arc<RwLock<HashMap<String, Vec<DnsRecord>>>>,
}

impl DnsPool {
    /// Create new DNS pool
    pub fn new(pool_id: String, pool_type: DnsPoolType, visibility: PoolVisibility) -> Self {
        Self {
            pool_id,
            pool_type,
            visibility,
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add record to pool
    pub async fn add_record(&self, record: DnsRecord) -> DnsResult<()> {
        let mut records = self.records.write().await;
        records
            .entry(record.domain.clone())
            .or_insert_with(Vec::new)
            .push(record);
        Ok(())
    }

    /// Get records for domain
    pub async fn get_records(&self, domain: &str) -> DnsResult<Vec<DnsRecord>> {
        let records = self.records.read().await;
        Ok(records.get(domain).cloned().unwrap_or_default())
    }

    /// Remove expired records
    pub async fn cleanup_expired(&self) -> DnsResult<usize> {
        let mut records = self.records.write().await;
        let mut removed = 0;

        for domain_records in records.values_mut() {
            let original_len = domain_records.len();
            domain_records.retain(|r| !r.is_expired());
            removed += original_len - domain_records.len();
        }

        // Remove empty domain entries
        records.retain(|_, v| !v.is_empty());

        Ok(removed)
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let records = self.records.read().await;
        let total_domains = records.len();
        let total_records: usize = records.values().map(|v| v.len()).sum();

        PoolStats {
            pool_id: self.pool_id.clone(),
            pool_type: self.pool_type.clone(),
            total_domains,
            total_records,
        }
    }
}

/// Pool statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolStats {
    pub pool_id: String,
    pub pool_type: DnsPoolType,
    pub total_domains: usize,
    pub total_records: usize,
}

/// DNS pool manager
pub struct DnsPoolManager {
    /// Public DNS pool
    public_pool: Arc<DnsPool>,
    /// Federated pools (network_id -> pool)
    federated_pools: Arc<RwLock<HashMap<String, Arc<DnsPool>>>>,
}

impl DnsPoolManager {
    /// Create new pool manager
    pub fn new() -> Self {
        let public_pool = Arc::new(DnsPool::new(
            "public".to_string(),
            DnsPoolType::Public,
            PoolVisibility::Public,
        ));

        Self {
            public_pool,
            federated_pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register public DNS record
    pub async fn register_public(&self, record: DnsRecord) -> DnsResult<()> {
        info!("Registering public DNS: {}", record.domain);
        self.public_pool.add_record(record).await
    }

    /// Register federated DNS record
    pub async fn register_federated(
        &self,
        network_id: String,
        record: DnsRecord,
    ) -> DnsResult<()> {
        info!(
            "Registering federated DNS: {} (network: {})",
            record.domain, network_id
        );

        let mut pools = self.federated_pools.write().await;
        let pool = pools.entry(network_id.clone()).or_insert_with(|| {
            Arc::new(DnsPool::new(
                format!("federated-{}", network_id),
                DnsPoolType::Federated {
                    network_id: network_id.clone(),
                },
                PoolVisibility::NetworkRestricted,
            ))
        });

        pool.add_record(record).await
    }

    /// Query public DNS pool
    pub async fn query_public(&self, domain: &str) -> DnsResult<Vec<DnsRecord>> {
        debug!("Querying public DNS: {}", domain);
        self.public_pool.get_records(domain).await
    }

    /// Query federated DNS pool
    pub async fn query_federated(
        &self,
        network_id: &str,
        domain: &str,
    ) -> DnsResult<Vec<DnsRecord>> {
        debug!(
            "Querying federated DNS: {} (network: {})",
            domain, network_id
        );

        let pools = self.federated_pools.read().await;
        match pools.get(network_id) {
            Some(pool) => pool.get_records(domain).await,
            None => Err(DnsError::PoolNotFound {
                pool_id: network_id.to_string(),
            }),
        }
    }

    /// Enforce privacy boundaries
    pub async fn can_access(
        &self,
        requester_network: Option<&str>,
        pool_type: &DnsPoolType,
    ) -> bool {
        match (requester_network, pool_type) {
            // Public pool is always accessible
            (_, DnsPoolType::Public) => true,
            // Federated pool requires matching network ID
            (Some(req_network), DnsPoolType::Federated { network_id }) => {
                req_network == network_id
            }
            // No network ID provided, cannot access federated
            (None, DnsPoolType::Federated { .. }) => false,
        }
    }

    /// Cleanup expired records in all pools
    pub async fn cleanup_all(&self) -> DnsResult<usize> {
        let mut total_removed = 0;

        // Cleanup public pool
        total_removed += self.public_pool.cleanup_expired().await?;

        // Cleanup federated pools
        let pools = self.federated_pools.read().await;
        for pool in pools.values() {
            total_removed += pool.cleanup_expired().await?;
        }

        if total_removed > 0 {
            info!("Cleaned up {} expired DNS records", total_removed);
        }

        Ok(total_removed)
    }

    /// Get all pool statistics
    pub async fn get_all_stats(&self) -> Vec<PoolStats> {
        let mut stats = vec![self.public_pool.stats().await];

        let pools = self.federated_pools.read().await;
        for pool in pools.values() {
            stats.push(pool.stats().await);
        }

        stats
    }
}

impl Default for DnsPoolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::DnsRecordType;
    use crate::dns::DnsRecordData;
    use std::net::Ipv6Addr;

    fn create_test_record(domain: &str, owner: &str) -> DnsRecord {
        DnsRecord::new(
            domain.to_string(),
            DnsRecordType::AAAA,
            DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
            300,
            owner.to_string(),
        )
    }

    #[tokio::test]
    async fn test_public_pool_registration() {
        let manager = DnsPoolManager::new();
        let record = create_test_record("nike", "node-1");

        manager.register_public(record).await.unwrap();

        let records = manager.query_public("nike").await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].domain, "nike");
    }

    #[tokio::test]
    async fn test_federated_pool_registration() {
        let manager = DnsPoolManager::new();
        let record = create_test_record("admin.nike", "node-1");

        manager
            .register_federated("nike-internal".to_string(), record)
            .await
            .unwrap();

        let records = manager
            .query_federated("nike-internal", "admin.nike")
            .await
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].domain, "admin.nike");
    }

    #[tokio::test]
    async fn test_privacy_boundary_enforcement() {
        let manager = DnsPoolManager::new();

        // Public pool accessible
        assert!(
            manager
                .can_access(None, &DnsPoolType::Public)
                .await
        );

        // Federated pool requires matching network
        let federated = DnsPoolType::Federated {
            network_id: "nike-internal".to_string(),
        };
        assert!(
            manager
                .can_access(Some("nike-internal"), &federated)
                .await
        );
        assert!(
            !manager
                .can_access(Some("other-network"), &federated)
                .await
        );
        assert!(!manager.can_access(None, &federated).await);
    }

    #[tokio::test]
    async fn test_pool_cleanup() {
        let manager = DnsPoolManager::new();
        let mut record = create_test_record("test", "node-1");
        record.ttl = 0;
        record.expires_at = std::time::SystemTime::now();

        manager.register_public(record).await.unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        let removed = manager.cleanup_all().await.unwrap();
        assert_eq!(removed, 1);

        let records = manager.query_public("test").await.unwrap();
        assert_eq!(records.len(), 0);
    }
}
