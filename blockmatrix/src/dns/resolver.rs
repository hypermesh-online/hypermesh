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
    DnsCache, DnsError, DnsPoolManager, DnsRecord, DnsRecordType, DnsResult, DnsValidator, Domain,
    DomainRegistration, TrustChainDnsClient,
};
use crate::dns::domain::derive_network_id;
use crate::proof_of_state::StateProof;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
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
    /// Resolved via hierarchical domain walk (parent chain lookup)
    Hierarchical {
        /// The domain whose pool contained the answer
        authoritative_domain: String,
        /// Network ID of the authoritative domain
        network_id: String,
    },
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
    /// State proof (for validation)
    pub proof: Option<StateProof>,
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
    /// Domain registry for hierarchical resolution (optional)
    domain_registry: Option<Arc<RwLock<HashMap<String, DomainRegistration>>>>,
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
            domain_registry: None,
        }
    }

    /// Set TrustChain DNS client
    pub fn with_trustchain_client(mut self, client: Arc<TrustChainDnsClient>) -> Self {
        self.trustchain_client = Some(client);
        self
    }

    /// Set domain registry for hierarchical resolution.
    ///
    /// When set, multi-component domains will walk up the parent chain
    /// looking for a federated pool that contains the queried name.
    pub fn with_domain_registry(
        mut self,
        registry: Arc<RwLock<HashMap<String, DomainRegistration>>>,
    ) -> Self {
        self.domain_registry = Some(registry);
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

        // Try hierarchical resolution for multi-component domains
        if self.domain_registry.is_some() && query.domain.is_federated() {
            if let Some(response) = self.resolve_hierarchical(&query).await {
                // Cache the hierarchical result
                if !response.records.is_empty() {
                    let ttl = response.records[0].ttl;
                    self.cache
                        .set(
                            &query.domain.full,
                            &query.record_type,
                            response.records.clone(),
                            ttl,
                        )
                        .await?;
                }
                return Ok(response);
            }
        }

        // Determine resolution tier
        let tier = self.determine_tier(&query);

        // Resolve based on tier
        let records = match &tier {
            DnsResolutionTier::P2PDirect => self.resolve_p2p_direct(&query).await?,
            DnsResolutionTier::Public => self.resolve_public(&query).await?,
            DnsResolutionTier::Federated { network_id } => {
                self.resolve_federated(&query, network_id).await?
            }
            DnsResolutionTier::FullyFederated { network_id } => {
                self.resolve_fully_federated(&query, network_id).await?
            }
            DnsResolutionTier::Hierarchical { .. } => {
                // Hierarchical is resolved above; this arm is unreachable in
                // normal flow but required for exhaustive matching.
                vec![]
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
                reason: format!("Not a member of network: {network_id}"),
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
                reason: "Fully federated domain requires state proof".to_string(),
            });
        }

        let proof = query.proof.as_ref().expect("proof required for federated domain resolution");

        // Validate access with proof
        let validation = self
            .validator
            .validate_dns_access(&query.domain, proof)
            .await?;

        if !validation.valid {
            return Err(DnsError::AccessDenied {
                reason: validation
                    .reason
                    .unwrap_or_else(|| "Validation failed".to_string()),
            });
        }

        // Validate network access
        if !self
            .validator
            .validate_network_access(&query.domain, query.requester_network.as_deref())?
        {
            return Err(DnsError::PrivacyViolation {
                reason: format!(
                    "Cannot access fully federated domain outside network: {network_id}"
                ),
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

    /// Walk up the domain hierarchy looking for a federated pool that contains
    /// the queried domain name.
    ///
    /// For "host.lab.hypermesh" the walk order is:
    ///   1. pool for "lab.hypermesh" (network_id derived from "lab.hypermesh")
    ///   2. pool for "hypermesh"     (network_id derived from "hypermesh")
    ///
    /// Returns `Some(DnsResponse)` on the first pool that has records,
    /// or `None` if no parent pool contains the name.
    async fn resolve_hierarchical(&self, query: &DnsQuery) -> Option<DnsResponse> {
        let mut current = query.domain.parent();

        while let Some(parent_domain) = current {
            let parent_name = parent_domain.full.clone();
            let network_id = derive_network_id(&parent_name);

            debug!(
                "Hierarchical walk: trying parent '{}' (network {})",
                parent_name, network_id
            );

            // Try querying the federated pool for this parent
            if let Ok(records) = self
                .pool_manager
                .query_federated(&network_id, &query.domain.full)
                .await
            {
                if !records.is_empty() {
                    info!(
                        "Hierarchical resolution: {} found in parent domain '{}' pool",
                        query.domain.full, parent_name
                    );

                    return Some(DnsResponse {
                        domain: query.domain.clone(),
                        tier: DnsResolutionTier::Hierarchical {
                            authoritative_domain: parent_name,
                            network_id,
                        },
                        records,
                        timestamp: SystemTime::now(),
                        from_cache: false,
                    });
                }
            }

            current = parent_domain.parent();
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_of_state::proof_of_state_integration::{
        SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
    };
    use crate::dns::DnsRecordData;
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

    fn _create_test_proof() -> StateProof {
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

        StateProof::new(stake, time, space, work)
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
        pool_manager.register_public(record).await.expect("test: async operation");

        // Query
        let query = DnsQuery {
            domain: Domain::parse("nike").expect("test: expected success"),
            record_type: DnsRecordType::AAAA,
            requester_network: None,
            proof: None,
            timestamp: SystemTime::now(),
        };

        let response = resolver.resolve(query).await.expect("test: async operation");
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
            .expect("test: expected success");

        // Query with network membership
        let query = DnsQuery {
            domain: Domain::parse("admin.nike").expect("test: expected success"),
            record_type: DnsRecordType::AAAA,
            requester_network: Some("nike-internal".to_string()),
            proof: None,
            timestamp: SystemTime::now(),
        };

        let response = resolver.resolve(query).await.expect("test: async operation");
        assert!(matches!(response.tier, DnsResolutionTier::Federated { .. }));
        assert_eq!(response.records.len(), 1);
    }

    #[tokio::test]
    async fn test_p2p_direct_tier() {
        let (resolver, _) = setup_resolver().await;

        let query = DnsQuery {
            domain: Domain::parse("peer-12345").expect("test: expected success"),
            record_type: DnsRecordType::AAAA,
            requester_network: None,
            proof: None,
            timestamp: SystemTime::now(),
        };

        let response = resolver.resolve(query).await.expect("test: async operation");
        assert_eq!(response.tier, DnsResolutionTier::P2PDirect);
        assert_eq!(response.records.len(), 0); // P2P returns empty
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let (resolver, pool_manager) = setup_resolver().await;

        // Register and query once
        let record = create_test_record("nike");
        pool_manager.register_public(record).await.expect("test: async operation");

        let query = DnsQuery {
            domain: Domain::parse("nike").expect("test: expected success"),
            record_type: DnsRecordType::AAAA,
            requester_network: None,
            proof: None,
            timestamp: SystemTime::now(),
        };

        let response1 = resolver.resolve(query.clone()).await.expect("test: async operation");
        assert!(!response1.from_cache);

        // Second query should hit cache
        let response2 = resolver.resolve(query).await.expect("test: async operation");
        assert!(response2.from_cache);
    }

    #[tokio::test]
    async fn test_hierarchical_resolve_finds_in_parent_pool() {
        let pool_manager = Arc::new(DnsPoolManager::new());
        let validator = Arc::new(DnsValidator::new(false));
        let cache = Arc::new(DnsCache::new(100));
        let domain_registry = Arc::new(RwLock::new(HashMap::new()));

        let resolver = DnsResolver::new(pool_manager.clone(), validator, cache)
            .with_domain_registry(domain_registry);

        // Register a record in the parent domain's pool (keyed by network_id)
        let parent_network_id = crate::dns::domain::derive_network_id("hypermesh");
        let record = create_test_record("host.hypermesh");
        pool_manager
            .register_federated(parent_network_id, record)
            .await
            .expect("test: register");

        // Query for "host.hypermesh" — hierarchical walk should find it in "hypermesh" pool
        let query = DnsQuery {
            domain: Domain::parse("host.hypermesh").expect("test: parse"),
            record_type: DnsRecordType::AAAA,
            requester_network: None,
            proof: None,
            timestamp: SystemTime::now(),
        };

        let response = resolver.resolve(query).await.expect("test: resolve");
        assert!(
            matches!(response.tier, DnsResolutionTier::Hierarchical { .. }),
            "expected Hierarchical tier, got {:?}",
            response.tier
        );
        assert_eq!(response.records.len(), 1);
    }

    #[tokio::test]
    async fn test_flat_domain_unaffected() {
        let pool_manager = Arc::new(DnsPoolManager::new());
        let validator = Arc::new(DnsValidator::new(false));
        let cache = Arc::new(DnsCache::new(100));
        let domain_registry = Arc::new(RwLock::new(HashMap::new()));

        let resolver = DnsResolver::new(pool_manager.clone(), validator, cache)
            .with_domain_registry(domain_registry);

        // Register a public record for a flat (single-component) domain
        let record = create_test_record("nike");
        pool_manager
            .register_public(record)
            .await
            .expect("test: register");

        let query = DnsQuery {
            domain: Domain::parse("nike").expect("test: parse"),
            record_type: DnsRecordType::AAAA,
            requester_network: None,
            proof: None,
            timestamp: SystemTime::now(),
        };

        let response = resolver.resolve(query).await.expect("test: resolve");
        // Flat domains skip hierarchical, resolve via Public
        assert_eq!(response.tier, DnsResolutionTier::Public);
        assert_eq!(response.records.len(), 1);
    }

    #[tokio::test]
    async fn test_hierarchical_falls_to_public() {
        let pool_manager = Arc::new(DnsPoolManager::new());
        let validator = Arc::new(DnsValidator::new(false));
        let cache = Arc::new(DnsCache::new(100));
        let domain_registry = Arc::new(RwLock::new(HashMap::new()));

        let resolver = DnsResolver::new(pool_manager.clone(), validator, cache)
            .with_domain_registry(domain_registry);

        // Register in the public pool (not in any parent domain pool)
        let record = create_test_record("admin.nike");
        pool_manager
            .register_public(record)
            .await
            .expect("test: register");

        let query = DnsQuery {
            domain: Domain::parse("admin.nike").expect("test: parse"),
            record_type: DnsRecordType::AAAA,
            requester_network: Some("nike".to_string()),
            proof: None,
            timestamp: SystemTime::now(),
        };

        // Hierarchical walk finds nothing, so falls through to Federated/Public
        let response = resolver.resolve(query).await;
        // The federated path will be tried since the domain is_federated.
        // It may succeed or fail depending on network access validation.
        // The key assertion is that we did NOT get a Hierarchical tier.
        match response {
            Ok(resp) => {
                assert!(
                    !matches!(resp.tier, DnsResolutionTier::Hierarchical { .. }),
                    "should not be Hierarchical when no parent pool has the record"
                );
            }
            Err(_) => {
                // Federated access denied or pool not found is expected —
                // the point is hierarchical did not resolve it
            }
        }
    }
}
