// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Tier DNS Resolver
//!
//! Implements four-tier DNS resolution:
//! 1. P2P Direct - Direct peer connection
//! 2. Public DNS - Blockchain-registered global pool
//! 3. Federated - Network-scoped pool
//! 4. Fully Federated - Zero public access

use super::{
    DnsRecord, DnsError, DnsResult, Domain, DnsPoolManager, DnsValidator, DnsCache,
    TrustChainDnsClient, DnsRecordType,
};
use crate::consensus::ConsensusProof;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, info};

/// DNS resolution tier
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DnsResolutionTier {
    /// P2P direct connection (no DNS)
    P2PDirect,
    /// Public DNS pool (blockchain-registered)
    Public,
    /// Federated DNS pool (network-scoped)
    Federated { network_id: String },
    /// Fully federated (no public access)
    FullyFederated { network_id: String },
}

/// DNS query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsQuery {
    /// Domain to resolve
    pub domain: Domain,
    /// Record type
    pub record_type: DnsRecordType,
    /// Requester network ID (for federated access)
    pub requester_network: Option<String>,
    /// Consensus proof (for validation)
    pub proof: Option<ConsensusProof>,
    /// Query timestamp
    pub timestamp: SystemTime,
}

/// DNS response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsResponse {
    /// Query domain
    pub domain: Domain,
    /// Resolution tier used
    pub tier: DnsResolutionTier,
    /// DNS records
    pub records: Vec<DnsRecord>,
    /// Response timestamp
    pub timestamp: SystemTime,
    /// Cache hit indicator
    pub from_cache: bool,
}

/// DNS resolver
pub struct DnsResolver {
    /// Pool manager
    pool_manager: Arc<DnsPoolManager>,
    /// DNS validator
    validator: Arc<DnsValidator>,
    /// DNS cache
    cache: Arc<DnsCache>,
    /// TrustChain DNS client (service layer)
    trustchain_client: Option<Arc<TrustChainDnsClient>>,
}

impl DnsResolver {
    /// Create new DNS resolver
    pub fn new(
        pool_manager: Arc<DnsPoolManager>,
        validator: Arc<DnsValidator>,
        cache: Arc<DnsCache>,
    ) -> Self {
        Self {
            pool_manager,
            validator,
            cache,
            trustchain_client: None,
        }
    }

    /// Set TrustChain DNS client
    pub fn with_trustchain_client(mut self, client: Arc<TrustChainDnsClient>) -> Self {
        self.trustchain_client = Some(client);
        self
    }

    /// Resolve DNS query with multi-tier resolution
    pub async fn resolve(&self, query: DnsQuery) -> DnsResult<DnsResponse> {
        debug!(
            "Resolving DNS: {} ({:?})",
            query.domain.full, query.record_type
        );

        // Check cache first
        if let Some(records) = self
            .cache
            .get(&query.domain.full, &query.record_type)
            .await?
        {
            debug!("DNS cache hit: {}", query.domain.full);
            return Ok(DnsResponse {
                domain: query.domain.clone(),
                tier: self.determine_tier(&query),
                records,
                timestamp: SystemTime::now(),
                from_cache: true,
            });
        }

        // Determine resolution tier
        let tier = self.determine_tier(&query);

        // Resolve based on tier
        let records = match &tier {
            DnsResolutionTier::P2PDirect => {
                self.resolve_p2p_direct(&query).await?
            }
            DnsResolutionTier::Public => {
                self.resolve_public(&query).await?
            }
            DnsResolutionTier::Federated { network_id } => {
                self.resolve_federated(&query, network_id).await?
            }
            DnsResolutionTier::FullyFederated { network_id } => {
                self.resolve_fully_federated(&query, network_id).await?
            }
        };

        // Cache the result
        if !records.is_empty() {
            let ttl = records[0].ttl;
            self.cache
                .set(&query.domain.full, &query.record_type, records.clone(), ttl)
                .await?;
        }

        info!(
            "✅ DNS resolved: {} via {:?} ({} records)",
            query.domain.full,
            tier,
            records.len()
        );

        Ok(DnsResponse {
            domain: query.domain,
            tier,
            records,
            timestamp: SystemTime::now(),
            from_cache: false,
        })
    }

    /// Determine resolution tier for query
    fn determine_tier(&self, query: &DnsQuery) -> DnsResolutionTier {
        // Check if P2P direct (peer-id format)
        if query.domain.full.starts_with("peer-") {
            return DnsResolutionTier::P2PDirect;
        }

        // Check if federated
        if query.domain.is_federated() {
            let network_id = query
                .requester_network
                .clone()
                .unwrap_or_else(|| query.domain.root.clone());

            // Fully federated if multiple subdomain levels
            if self.validator.is_fully_federated(&query.domain) {
                return DnsResolutionTier::FullyFederated { network_id };
            }

            return DnsResolutionTier::Federated { network_id };
        }

        // Default to public
        DnsResolutionTier::Public
    }

    /// Resolve P2P direct connection (no DNS)
    async fn resolve_p2p_direct(&self, query: &DnsQuery) -> DnsResult<Vec<DnsRecord>> {
        debug!("P2P direct resolution: {}", query.domain.full);

        // P2P direct returns empty - application handles direct connection
        // No DNS resolution needed for peer-to-peer
        Ok(vec![])
    }

    /// Resolve from public DNS pool
    async fn resolve_public(&self, query: &DnsQuery) -> DnsResult<Vec<DnsRecord>> {
        debug!("Public DNS resolution: {}", query.domain.full);

        // Validate access if proof provided
        if let Some(proof) = &query.proof {
            self.validator
                .validate_dns_access(&query.domain, proof)
                .await?;
        }

        // Query public pool
        let records = self.pool_manager.query_public(&query.domain.full).await?;

        if records.is_empty() {
            // Fall back to TrustChain DNS service if available
            if let Some(client) = &self.trustchain_client {
                return client.query(&query.domain.full, &query.record_type).await;
            }

            return Err(DnsError::DomainNotFound {
                domain: query.domain.full.clone(),
            });
        }

        Ok(records)
    }

    /// Resolve from federated DNS pool
    async fn resolve_federated(
        &self,
        query: &DnsQuery,
        network_id: &str,
    ) -> DnsResult<Vec<DnsRecord>> {
        debug!(
            "Federated DNS resolution: {} (network: {})",
            query.domain.full, network_id
        );

        // Validate network access
        if !self
            .validator
            .validate_network_access(&query.domain, query.requester_network.as_deref())?
        {
            return Err(DnsError::AccessDenied {
                reason: format!("Not a member of network: {}", network_id),
            });
        }

        // Validate access if proof provided
        if let Some(proof) = &query.proof {
            self.validator
                .validate_dns_access(&query.domain, proof)
                .await?;
        }

        // Query federated pool
        let records = self
            .pool_manager
            .query_federated(network_id, &query.domain.full)
            .await?;

        if records.is_empty() {
            return Err(DnsError::DomainNotFound {
                domain: query.domain.full.clone(),
            });
        }

        Ok(records)
    }

    /// Resolve from fully federated DNS pool
    async fn resolve_fully_federated(
        &self,
        query: &DnsQuery,
        network_id: &str,
    ) -> DnsResult<Vec<DnsRecord>> {
        debug!(
            "Fully federated DNS resolution: {} (network: {})",
            query.domain.full, network_id
        );

        // Fully federated requires strict validation
        if query.proof.is_none() {
            return Err(DnsError::AccessDenied {
                reason: "Fully federated domain requires consensus proof".to_string(),
            });
        }

        let proof = query.proof.as_ref().unwrap();

        // Validate access with proof
        let validation = self
            .validator
            .validate_dns_access(&query.domain, proof)
            .await?;

        if !validation.valid {
            return Err(DnsError::AccessDenied {
                reason: validation.reason.unwrap_or_else(|| "Validation failed".to_string()),
            });
        }

        // Validate network access
        if !self
            .validator
            .validate_network_access(&query.domain, query.requester_network.as_deref())?
        {
            return Err(DnsError::PrivacyViolation {
                reason: format!("Cannot access fully federated domain outside network: {}", network_id),
            });
        }

        // Query federated pool
        let records = self
            .pool_manager
            .query_federated(network_id, &query.domain.full)
            .await?;

        if records.is_empty() {
            return Err(DnsError::DomainNotFound {
                domain: query.domain.full.clone(),
            });
        }

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::{DnsRecordData};
    use crate::consensus::proof_of_state_integration::{
        SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
    };
    use std::net::Ipv6Addr;
    use std::time::Duration;

    fn create_test_record(domain: &str) -> DnsRecord {
        DnsRecord::new(
            domain.to_string(),
            DnsRecordType::AAAA,
            DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
            300,
            "node-1".to_string(),
        )
    }

    fn create_test_proof() -> ConsensusProof {
        let stake = StakeProof::new("holder".to_string(), "holder-id".to_string(), 1000);
        let time = TimeProof::new(Duration::from_secs(10));
        let space = SpaceProof::new("node".to_string(), "/storage".to_string(), 1024 * 1024);
        let work = WorkProof::new(
            "owner".to_string(),
            "workload".to_string(),
            12345,
            100,
            WorkloadType::Compute,
            WorkState::Completed,
        );

        ConsensusProof::new(stake, time, space, work)
    }

    async fn setup_resolver() -> (DnsResolver, Arc<DnsPoolManager>) {
        let pool_manager = Arc::new(DnsPoolManager::new());
        let validator = Arc::new(DnsValidator::new(false));
        let cache = Arc::new(DnsCache::new(100));
        let resolver = DnsResolver::new(pool_manager.clone(), validator, cache);
        (resolver, pool_manager)
    }

    #[tokio::test]
    async fn test_public_resolution() {
        let (resolver, pool_manager) = setup_resolver().await;

        // Register public record
        let record = create_test_record("nike");
        pool_manager.register_public(record).await.unwrap();

        // Query
        let query = DnsQuery {
            domain: Domain::parse("nike").unwrap(),
            record_type: DnsRecordType::AAAA,
            requester_network: None,
            proof: None,
            timestamp: SystemTime::now(),
        };

        let response = resolver.resolve(query).await.unwrap();
        assert_eq!(response.tier, DnsResolutionTier::Public);
        assert_eq!(response.records.len(), 1);
    }

    #[tokio::test]
    async fn test_federated_resolution() {
        let (resolver, pool_manager) = setup_resolver().await;

        // Register federated record
        let record = create_test_record("admin.nike");
        pool_manager
            .register_federated("nike-internal".to_string(), record)
            .await
            .unwrap();

        // Query with network membership
        let query = DnsQuery {
            domain: Domain::parse("admin.nike").unwrap(),
            record_type: DnsRecordType::AAAA,
            requester_network: Some("nike-internal".to_string()),
            proof: None,
            timestamp: SystemTime::now(),
        };

        let response = resolver.resolve(query).await.unwrap();
        assert!(matches!(
            response.tier,
            DnsResolutionTier::Federated { .. }
        ));
        assert_eq!(response.records.len(), 1);
    }

    #[tokio::test]
    async fn test_p2p_direct_tier() {
        let (resolver, _) = setup_resolver().await;

        let query = DnsQuery {
            domain: Domain::parse("peer-12345").unwrap(),
            record_type: DnsRecordType::AAAA,
            requester_network: None,
            proof: None,
            timestamp: SystemTime::now(),
        };

        let response = resolver.resolve(query).await.unwrap();
        assert_eq!(response.tier, DnsResolutionTier::P2PDirect);
        assert_eq!(response.records.len(), 0); // P2P returns empty
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let (resolver, pool_manager) = setup_resolver().await;

        // Register and query once
        let record = create_test_record("nike");
        pool_manager.register_public(record).await.unwrap();

        let query = DnsQuery {
            domain: Domain::parse("nike").unwrap(),
            record_type: DnsRecordType::AAAA,
            requester_network: None,
            proof: None,
            timestamp: SystemTime::now(),
        };

        let response1 = resolver.resolve(query.clone()).await.unwrap();
        assert!(!response1.from_cache);

        // Second query should hit cache
        let response2 = resolver.resolve(query).await.unwrap();
        assert!(response2.from_cache);
    }
}
