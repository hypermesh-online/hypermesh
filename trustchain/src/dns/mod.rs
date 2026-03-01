// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS-over-QUIC Resolver Implementation  
//!
//! TrustChain DNS resolver with IPv6-only networking, certificate DNS validation,
//! and integration with TrustChain domains (hypermesh, caesar, trust, assets).

use anyhow::Result as AnyhowResult;
use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

// ARCHITECTURAL ENFORCEMENT: Use STOQ transport instead of direct QUIC
use trust_dns_proto::op::ResponseCode;
use trust_dns_proto::rr::{DNSClass, Name, RData, Record, RecordType};

use crate::config::DnsConfig;
use crate::consensus::ConsensusContext;
use crate::errors::{DnsError, Result as TrustChainResult};

pub mod bootstrap;
pub mod cache;
pub mod cert_validator;
pub mod dns_over_stoq;
pub mod resolver;
pub mod stoq_transport;
// DEPRECATED: Legacy modules to be removed after full STOQ migration
pub mod authoritative_server;
pub mod dns_over_quic;
pub mod production_zones;

pub use bootstrap::*;
pub use cache::*;
pub use cert_validator::*;
pub use dns_over_stoq::*;
pub use resolver::*;
pub use stoq_transport::*;
// DEPRECATED: Legacy exports to be removed after full STOQ migration
pub use authoritative_server::*;
pub use dns_over_quic::*;
pub use production_zones::*;

/// TrustChain DNS resolver with STOQ transport (architectural compliance)
#[derive(Clone)]
pub struct DnsResolver {
    /// DNS server identifier
    server_id: String,
    /// STOQ transport for DNS-over-STOQ
    stoq_client: Arc<crate::stoq_client::TrustChainStoqClient>,
    /// DNS record resolver
    resolver: Arc<TrustChainResolver>,
    /// DNS cache
    cache: Arc<DnsCache>,
    /// Certificate validator
    cert_validator: Arc<CertificateValidator>,
    /// Configuration
    config: Arc<DnsConfig>,
    /// Consensus validation context  
    consensus_context: Arc<ConsensusContext>,
    /// Background task handles
    task_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

/// DNS query request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsQuery {
    /// Query ID
    pub id: u16,
    /// Domain name to resolve
    pub name: String,
    /// Record type (A, AAAA, CNAME, etc.)
    #[serde(with = "record_type_serde")]
    pub record_type: RecordType,
    /// DNS class (IN, etc.)
    #[serde(with = "dns_class_serde")]
    pub class: DNSClass,
    /// Client IPv6 address
    pub client_addr: Ipv6Addr,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// DNS query response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsResponse {
    /// Query ID
    pub id: u16,
    /// Response code
    #[serde(with = "response_code_serde")]
    pub response_code: ResponseCode,
    /// Answer records
    pub answers: Vec<DnsRecord>,
    /// Authority records
    pub authorities: Vec<DnsRecord>,
    /// Additional records
    pub additionals: Vec<DnsRecord>,
    /// Response timestamp
    pub timestamp: SystemTime,
    /// Cache TTL
    pub ttl: u32,
}

/// DNS record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsRecord {
    /// Record name
    pub name: String,
    /// Record type
    #[serde(with = "record_type_serde")]
    pub record_type: RecordType,
    /// Record class
    #[serde(with = "dns_class_serde")]
    pub class: DNSClass,
    /// TTL in seconds
    pub ttl: u32,
    /// Record data
    pub data: DnsRecordData,
}

// Custom serde implementations for DNS types
mod record_type_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use trust_dns_proto::rr::RecordType;

    pub fn serialize<S>(record_type: &RecordType, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{record_type:?}").serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<RecordType, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "A" => Ok(RecordType::A),
            "AAAA" => Ok(RecordType::AAAA),
            "CNAME" => Ok(RecordType::CNAME),
            "MX" => Ok(RecordType::MX),
            "TXT" => Ok(RecordType::TXT),
            "NS" => Ok(RecordType::NS),
            "SOA" => Ok(RecordType::SOA),
            _ => Ok(RecordType::Unknown(0)),
        }
    }
}

mod dns_class_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use trust_dns_proto::rr::DNSClass;

    pub fn serialize<S>(dns_class: &DNSClass, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{dns_class:?}").serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DNSClass, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "IN" => Ok(DNSClass::IN),
            "CH" => Ok(DNSClass::CH),
            "HS" => Ok(DNSClass::HS),
            _ => Ok(DNSClass::NONE),
        }
    }
}

mod response_code_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use trust_dns_proto::op::ResponseCode;

    pub fn serialize<S>(response_code: &ResponseCode, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{response_code:?}").serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ResponseCode, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "NoError" => Ok(ResponseCode::NoError),
            "FormErr" => Ok(ResponseCode::FormErr),
            "ServFail" => Ok(ResponseCode::ServFail),
            "NXDomain" => Ok(ResponseCode::NXDomain),
            "NotImp" => Ok(ResponseCode::NotImp),
            "Refused" => Ok(ResponseCode::Refused),
            _ => Ok(ResponseCode::NoError),
        }
    }
}

/// DNS record data variants
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DnsRecordData {
    /// IPv4 address
    A(std::net::Ipv4Addr),
    /// IPv6 address  
    AAAA(Ipv6Addr),
    /// Canonical name
    CNAME(String),
    /// Mail exchange
    MX { priority: u16, exchange: String },
    /// Text record
    TXT(String),
    /// Name server
    NS(String),
    /// Start of authority
    SOA {
        mname: String,
        rname: String,
        serial: u32,
        refresh: i32,
        retry: i32,
        expire: i32,
        minimum: u32,
    },
}

impl DnsResolver {
    /// Create new DNS resolver
    pub async fn new(config: DnsConfig) -> TrustChainResult<Self> {
        info!("Initializing TrustChain DNS resolver: {}", config.server_id);

        // Initialize DNS cache
        let cache = Arc::new(DnsCache::new(config.cache_ttl).await?);

        // Initialize certificate validator
        let cert_validator =
            Arc::new(CertificateValidator::new(config.enable_cert_validation).await?);

        // Initialize TrustChain resolver
        let resolver = Arc::new(
            TrustChainResolver::new(
                config.upstream_resolvers.clone(),
                config.trustchain_domains.clone(),
            )
            .await?,
        );

        // Initialize STOQ client (architectural enforcement)
        let stoq_config = crate::stoq_client::TrustChainStoqConfig {
            bind_address: config.bind_address,
            ..Default::default()
        };
        let stoq_client =
            Arc::new(crate::stoq_client::TrustChainStoqClient::new(stoq_config).await?);

        // Initialize consensus context
        let consensus_context = Arc::new(ConsensusContext::new(
            config.server_id.clone(),
            "trustchain_dns_network".to_string(),
        ));

        let dns_resolver = Self {
            server_id: config.server_id.clone(),
            stoq_client,
            resolver,
            cache,
            cert_validator,
            config: Arc::new(config),
            consensus_context,
            task_handles: Arc::new(Mutex::new(Vec::new())),
        };

        // Start background tasks
        dns_resolver.start_background_tasks().await?;

        info!("TrustChain DNS resolver initialized successfully");
        Ok(dns_resolver)
    }

    /// Start DNS resolver service
    pub async fn start(&self) -> TrustChainResult<()> {
        info!("Starting TrustChain DNS resolver");

        // Start STOQ DNS server (proper architectural separation)
        let _stoq_client_clone = Arc::clone(&self.stoq_client);
        let _resolver_clone = self.clone_for_task();

        let handle = tokio::spawn(async move {
            loop {
                // STOQ handles connection acceptance internally
                // DNS service listens via STOQ transport
                tokio::time::sleep(Duration::from_secs(1)).await;
                // TODO: Implement proper STOQ DNS service listener
                // This should use STOQ's accept() method when available
                // Placeholder for STOQ DNS service implementation
                // The STOQ client will handle incoming DNS requests
            }
        });

        {
            let mut handles = self.task_handles.lock().await;
            handles.push(handle);
        }

        info!("TrustChain DNS resolver started successfully");
        Ok(())
    }

    /// Resolve DNS query
    pub async fn resolve_query(&self, query: &DnsQuery) -> TrustChainResult<DnsResponse> {
        debug!(
            "Resolving DNS query: {} ({:?})",
            query.name, query.record_type
        );

        // Check cache first
        if let Some(cached_response) = self.cache.get(&query.name, query.record_type).await? {
            debug!("Cache hit for {}", query.name);
            return Ok(cached_response);
        }

        // Check if this is a TrustChain domain
        let response = if self.is_trustchain_domain(&query.name) {
            self.resolve_trustchain_domain(query).await?
        } else {
            // Forward to upstream resolver
            self.resolver.resolve_upstream(query).await?
        };

        // Validate certificate if enabled
        if self.config.enable_cert_validation {
            if let Err(e) = self.validate_domain_certificate(&query.name).await {
                warn!("Certificate validation failed for {}: {}", query.name, e);
                // Continue with resolution but log the warning
            }
        }

        // Cache the response
        self.cache
            .set(&query.name, query.record_type, &response, response.ttl)
            .await?;

        debug!("Resolved DNS query successfully: {}", query.name);
        Ok(response)
    }

    /// Resolve TrustChain-specific domain using production addresses
    pub async fn resolve_trustchain_domain(
        &self,
        query: &DnsQuery,
    ) -> TrustChainResult<DnsResponse> {
        debug!("Resolving TrustChain domain: {}", query.name);

        // Use production domain resolver instead of localhost stubs
        let production_resolver = ProductionDomainResolver::new();
        let mut answers = Vec::new();

        if query.record_type == RecordType::AAAA {
            if let Some(ipv6_addr) = production_resolver.resolve_domain(&query.name) {
                info!(
                    "✅ Resolved {} to production address: [{}]",
                    query.name, ipv6_addr
                );
                answers.push(DnsRecord {
                    name: query.name.clone(),
                    record_type: RecordType::AAAA,
                    class: DNSClass::IN,
                    ttl: 300, // 5 minutes TTL
                    data: DnsRecordData::AAAA(ipv6_addr),
                });
            } else {
                // Fall back to localhost only for development/testing
                if self.config.bind_address == Ipv6Addr::LOCALHOST {
                    warn!("⚠️ Domain {} not found in production resolver, using localhost (development mode)", query.name);
                    answers.push(DnsRecord {
                        name: query.name.clone(),
                        record_type: RecordType::AAAA,
                        class: DNSClass::IN,
                        ttl: 60, // Short TTL for development
                        data: DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
                    });
                } else {
                    // Production mode - domain not found
                    warn!("❌ Domain {} not found in production DNS zones", query.name);
                    return Err(DnsError::DomainNotFound {
                        domain: query.name.clone(),
                    }
                    .into());
                }
            }
        }

        Ok(DnsResponse {
            id: query.id,
            response_code: ResponseCode::NoError,
            answers,
            authorities: vec![],
            additionals: vec![],
            timestamp: SystemTime::now(),
            ttl: 300,
        })
    }

    /// Get DNS resolver statistics
    pub async fn get_stats(&self) -> TrustChainResult<DnsStats> {
        let cache_stats = self.cache.get_stats().await;
        let resolver_stats = self.resolver.get_stats().await;

        Ok(DnsStats {
            server_id: self.server_id.clone(),
            queries_processed: resolver_stats.queries_processed,
            cache_hits: cache_stats.hits,
            cache_misses: cache_stats.misses,
            upstream_queries: resolver_stats.upstream_queries,
            trustchain_queries: resolver_stats.trustchain_queries,
            cert_validations: self.cert_validator.get_validation_count().await,
            last_update: SystemTime::now(),
        })
    }

    /// Shutdown DNS resolver
    pub async fn shutdown(&self) -> TrustChainResult<()> {
        info!("Shutting down TrustChain DNS resolver");

        // Cancel background tasks
        let mut handles = self.task_handles.lock().await;
        for handle in handles.drain(..) {
            handle.abort();
        }

        // Shutdown STOQ client (proper cleanup)
        self.stoq_client.shutdown().await?;

        // Flush cache
        self.cache.flush().await?;

        info!("TrustChain DNS resolver shut down successfully");
        Ok(())
    }

    // Internal helper methods

    async fn start_background_tasks(&self) -> TrustChainResult<()> {
        let mut handles = self.task_handles.lock().await;

        // Cache cleanup task
        let cache_clone = Arc::clone(&self.cache);
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                if let Err(e) = cache_clone.cleanup().await {
                    error!("DNS cache cleanup failed: {}", e);
                }
            }
        });
        handles.push(handle);

        info!("DNS resolver background tasks started");
        Ok(())
    }

    fn clone_for_task(&self) -> Self {
        Self {
            server_id: self.server_id.clone(),
            stoq_client: Arc::clone(&self.stoq_client),
            resolver: Arc::clone(&self.resolver),
            cache: Arc::clone(&self.cache),
            cert_validator: Arc::clone(&self.cert_validator),
            config: Arc::clone(&self.config),
            consensus_context: Arc::clone(&self.consensus_context),
            task_handles: Arc::clone(&self.task_handles),
        }
    }

    // REMOVED: handle_connection - STOQ handles connection management
    // DNS queries are processed through STOQ client interface

    // REMOVED: handle_query_stream - replaced by STOQ DNS query interface
    // DNS queries are processed through stoq_client.resolve_dns() method

    fn is_trustchain_domain(&self, domain: &str) -> bool {
        self.config
            .trustchain_domains
            .iter()
            .any(|td| domain == td || domain.ends_with(&format!(".{td}")))
    }

    async fn validate_domain_certificate(&self, domain: &str) -> TrustChainResult<()> {
        if self.config.enable_cert_validation {
            self.cert_validator
                .validate_domain_certificate(domain)
                .await
        } else {
            Ok(())
        }
    }

    /// Convert DNS record to trust-dns Record format (used for DNS response construction)
    fn _dns_record_to_trust_dns(&self, record: &DnsRecord) -> AnyhowResult<Record> {
        let name = Name::from_utf8(&record.name)?;
        let rdata = match &record.data {
            DnsRecordData::A(addr) => RData::A(trust_dns_proto::rr::rdata::A(*addr)),
            DnsRecordData::AAAA(addr) => RData::AAAA(trust_dns_proto::rr::rdata::AAAA(*addr)),
            DnsRecordData::CNAME(name) => {
                RData::CNAME(trust_dns_proto::rr::rdata::CNAME(Name::from_utf8(name)?))
            }
            DnsRecordData::MX { priority, exchange } => RData::MX(
                trust_dns_proto::rr::rdata::MX::new(*priority, Name::from_utf8(exchange)?),
            ),
            DnsRecordData::TXT(text) => {
                RData::TXT(trust_dns_proto::rr::rdata::TXT::new(vec![text.clone()]))
            }
            DnsRecordData::NS(ns) => {
                RData::NS(trust_dns_proto::rr::rdata::NS(Name::from_utf8(ns)?))
            }
            DnsRecordData::SOA {
                mname,
                rname,
                serial,
                refresh,
                retry,
                expire,
                minimum,
            } => RData::SOA(trust_dns_proto::rr::rdata::SOA::new(
                Name::from_utf8(mname)?,
                Name::from_utf8(rname)?,
                *serial,
                *refresh,
                *retry,
                *expire,
                *minimum,
            )),
        };

        Ok(Record::from_rdata(name, record.ttl, rdata))
    }
}

/// DNS resolver statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsStats {
    pub server_id: String,
    pub queries_processed: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub upstream_queries: u64,
    pub trustchain_queries: u64,
    pub cert_validations: u64,
    pub last_update: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    async fn create_test_resolver() -> DnsResolver {
        let config = DnsConfig {
            bind_address: Ipv6Addr::LOCALHOST,
            port: 0,                       // Use random available port for testing
            enable_cert_validation: false, // Disable for testing
            ..Default::default()
        };

        DnsResolver::new(config)
            .await
            .expect("Failed to create test DNS resolver")
    }

    #[tokio::test]
    async fn test_dns_resolver_creation() {
        let resolver = create_test_resolver().await;
        let stats = resolver.get_stats().await.expect("Failed to get DNS stats");
        assert_eq!(stats.server_id, "trustchain-dns-localhost");
    }

    #[tokio::test]
    async fn test_trustchain_domain_detection() {
        let resolver = create_test_resolver().await;

        assert!(resolver.is_trustchain_domain("hypermesh"));
        assert!(resolver.is_trustchain_domain("caesar"));
        assert!(resolver.is_trustchain_domain("trust"));
        assert!(resolver.is_trustchain_domain("assets"));
        assert!(!resolver.is_trustchain_domain("google.com"));
    }

    #[tokio::test]
    async fn test_trustchain_domain_resolution() {
        let resolver = create_test_resolver().await;

        let query = DnsQuery {
            id: 1234,
            name: "hypermesh".to_string(),
            record_type: RecordType::AAAA,
            class: DNSClass::IN,
            client_addr: Ipv6Addr::LOCALHOST,
            timestamp: SystemTime::now(),
        };

        let response = resolver
            .resolve_trustchain_domain(&query)
            .await
            .expect("Failed to resolve trustchain domain");
        assert_eq!(response.response_code, ResponseCode::NoError);
        assert!(!response.answers.is_empty(), "Expected at least one answer");

        if let DnsRecordData::AAAA(addr) = &response.answers[0].data {
            // Test resolver is bound to localhost, should return localhost fallback
            // OR production address from ProductionDomainResolver
            assert!(
                *addr == Ipv6Addr::LOCALHOST
                    || *addr
                        == crate::dns::production_zones::ProductionAddresses::HYPERMESH_DASHBOARD,
                "Expected localhost or production address, got {addr}"
            );
        } else {
            panic!("Expected AAAA record");
        }
    }

    #[tokio::test]
    async fn test_unknown_trustchain_domain() {
        let resolver = create_test_resolver().await;

        let query = DnsQuery {
            id: 1234,
            name: "unknown".to_string(),
            record_type: RecordType::AAAA,
            class: DNSClass::IN,
            client_addr: Ipv6Addr::LOCALHOST,
            timestamp: SystemTime::now(),
        };

        let response = resolver
            .resolve_trustchain_domain(&query)
            .await
            .expect("Failed to resolve unknown domain");
        // Test resolver is bound to localhost, so unknown domains get localhost fallback
        assert_eq!(response.response_code, ResponseCode::NoError);
        assert!(
            !response.answers.is_empty(),
            "Localhost fallback should provide an answer"
        );

        if let DnsRecordData::AAAA(addr) = &response.answers[0].data {
            assert_eq!(
                *addr,
                Ipv6Addr::LOCALHOST,
                "Unknown domain should resolve to localhost in test mode"
            );
        }
    }

    #[tokio::test]
    async fn test_dns_stats() {
        let resolver = create_test_resolver().await;
        let stats = resolver.get_stats().await.expect("Failed to get DNS stats");

        assert_eq!(stats.server_id, "trustchain-dns-localhost");
        assert_eq!(stats.queries_processed, 0);
    }

    #[tokio::test]
    async fn test_dns_record_conversion() {
        let resolver = create_test_resolver().await;

        let dns_record = DnsRecord {
            name: "test.example.com".to_string(),
            record_type: RecordType::AAAA,
            class: DNSClass::IN,
            ttl: 300,
            data: DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
        };

        let trust_dns_record = resolver
            ._dns_record_to_trust_dns(&dns_record)
            .expect("Failed to convert DNS record to trust-dns format");
        assert_eq!(trust_dns_record.record_type(), RecordType::AAAA);
        assert_eq!(trust_dns_record.ttl(), 300);
    }
}
