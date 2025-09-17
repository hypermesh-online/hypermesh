//! TrustChain DNS Resolver
//! 
//! Upstream DNS resolution with IPv6-only networking and TrustChain domain handling.

use std::sync::Arc;
use std::time::{SystemTime, Duration};
use std::net::{Ipv6Addr, SocketAddrV6};
use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use tracing::{debug, warn, error};

use trust_dns_client::{client::{AsyncClient, ClientHandle}, udp::UdpClientConnection};
use trust_dns_proto::op::{Message, MessageType, OpCode, ResponseCode};
use trust_dns_proto::rr::{DNSClass, Name, RData, Record, RecordType};

use crate::errors::{DnsError, Result as TrustChainResult};
use super::{DnsQuery, DnsResponse, DnsRecord, DnsRecordData};

/// TrustChain DNS resolver
pub struct TrustChainResolver {
    /// Upstream resolvers (IPv6 only)
    upstream_resolvers: Vec<Ipv6Addr>,
    /// TrustChain domains
    trustchain_domains: Vec<String>,
    /// Resolver statistics
    stats: Arc<RwLock<ResolverStats>>,
    /// Client pool
    client_pool: Arc<Mutex<Vec<AsyncClient>>>,
}

/// Resolver statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolverStats {
    pub queries_processed: u64,
    pub upstream_queries: u64,
    pub trustchain_queries: u64,
    pub failed_queries: u64,
    pub average_response_time_ms: f64,
    pub last_update: SystemTime,
}

impl Default for ResolverStats {
    fn default() -> Self {
        Self {
            queries_processed: 0,
            upstream_queries: 0,
            trustchain_queries: 0,
            failed_queries: 0,
            average_response_time_ms: 0.0,
            last_update: SystemTime::now(),
        }
    }
}

impl TrustChainResolver {
    /// Create new resolver
    pub async fn new(
        upstream_resolvers: Vec<Ipv6Addr>,
        trustchain_domains: Vec<String>,
    ) -> TrustChainResult<Self> {
        debug!("Initializing TrustChain resolver with {} upstream resolvers", upstream_resolvers.len());

        // Initialize client pool
        let client_pool = Arc::new(Mutex::new(Vec::new()));

        let resolver = Self {
            upstream_resolvers,
            trustchain_domains,
            stats: Arc::new(RwLock::new(ResolverStats::default())),
            client_pool,
        };

        // Initialize client connections
        resolver.initialize_clients().await?;

        debug!("TrustChain resolver initialized successfully");
        Ok(resolver)
    }

    /// Resolve query using upstream resolvers
    pub async fn resolve_upstream(&self, query: &DnsQuery) -> TrustChainResult<DnsResponse> {
        debug!("Resolving upstream query: {} ({:?})", query.name, query.record_type);

        let start_time = std::time::Instant::now();

        // Try each upstream resolver
        let mut last_error = None;
        
        for resolver_addr in &self.upstream_resolvers {
            match self.query_upstream_resolver(*resolver_addr, query).await {
                Ok(response) => {
                    let response_time = start_time.elapsed().as_millis() as f64;
                    self.update_stats(false, response_time).await;
                    return Ok(response);
                }
                Err(e) => {
                    warn!("Upstream resolver {} failed: {}", resolver_addr, e);
                    last_error = Some(e);
                }
            }
        }

        // All upstream resolvers failed
        self.update_stats_failure().await;
        Err(last_error.unwrap_or_else(|| DnsError::QueryFailed {
            query: query.name.clone(),
            reason: "All upstream resolvers failed".to_string(),
        }.into()))
    }

    /// Get resolver statistics
    pub async fn get_stats(&self) -> ResolverStats {
        self.stats.read().await.clone()
    }

    // Internal helper methods

    async fn initialize_clients(&self) -> TrustChainResult<()> {
        let mut client_pool = self.client_pool.lock().await;
        
        for resolver_addr in &self.upstream_resolvers {
            match self.create_client(*resolver_addr).await {
                Ok(client) => client_pool.push(client),
                Err(e) => warn!("Failed to create client for {}: {}", resolver_addr, e),
            }
        }

        if client_pool.is_empty() {
            return Err(DnsError::UpstreamResolver {
                resolver: "all".to_string(),
                reason: "Failed to create any upstream clients".to_string(),
            }.into());
        }

        debug!("Initialized {} DNS clients", client_pool.len());
        Ok(())
    }

    async fn create_client(&self, resolver_addr: Ipv6Addr) -> TrustChainResult<AsyncClient> {
        // Simplified DNS client creation for compilation
        // TODO: Implement proper async DNS client when trust-dns API is stable
        return Err(crate::errors::TrustChainError::Internal { 
            message: "DNS client creation not implemented - API compatibility issues".to_string() 
        });
    }

    async fn query_upstream_resolver(
        &self,
        resolver_addr: Ipv6Addr,
        query: &DnsQuery,
    ) -> TrustChainResult<DnsResponse> {
        // Create a fresh client for this query
        let mut client = self.create_client(resolver_addr).await?;
        
        // Convert query to trust-dns format
        let name = Name::from_utf8(&query.name)
            .map_err(|e| DnsError::QueryFailed {
                query: query.name.clone(),
                reason: format!("Invalid domain name: {}", e),
            })?;

        // Perform query
        let response = client.query(name, query.class, query.record_type).await
            .map_err(|e| DnsError::QueryFailed {
                query: query.name.clone(),
                reason: e.to_string(),
            })?;

        // Convert response
        self.convert_response(query, response.into()).await
    }

    async fn convert_response(
        &self,
        query: &DnsQuery,
        response: Message,
    ) -> TrustChainResult<DnsResponse> {
        let mut answers = Vec::new();
        let mut authorities = Vec::new();
        let mut additionals = Vec::new();

        // Convert answers
        for record in response.answers() {
            if let Some(dns_record) = self.convert_record(record) {
                answers.push(dns_record);
            }
        }

        // Convert authorities
        for record in response.name_servers() {
            if let Some(dns_record) = self.convert_record(record) {
                authorities.push(dns_record);
            }
        }

        // Convert additionals
        for record in response.additionals() {
            if let Some(dns_record) = self.convert_record(record) {
                additionals.push(dns_record);
            }
        }

        // Determine TTL (use minimum from answers)
        let ttl = answers.iter().map(|r| r.ttl).min().unwrap_or(300);

        Ok(DnsResponse {
            id: query.id,
            response_code: response.response_code(),
            answers,
            authorities,
            additionals,
            timestamp: SystemTime::now(),
            ttl,
        })
    }

    fn convert_record(&self, record: &Record) -> Option<DnsRecord> {
        let data = match record.data()? {
            RData::A(addr) => DnsRecordData::A((*addr).into()),
            RData::AAAA(addr) => DnsRecordData::AAAA((*addr).into()),
            RData::CNAME(name) => DnsRecordData::CNAME(name.to_string()),
            RData::MX(mx) => DnsRecordData::MX {
                priority: mx.preference(),
                exchange: mx.exchange().to_string(),
            },
            RData::TXT(txt) => DnsRecordData::TXT(
                txt.txt_data().iter()
                    .map(|bytes| String::from_utf8_lossy(bytes).to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            RData::NS(ns) => DnsRecordData::NS(ns.to_string()),
            RData::SOA(soa) => DnsRecordData::SOA {
                mname: soa.mname().to_string(),
                rname: soa.rname().to_string(),
                serial: soa.serial(),
                refresh: soa.refresh(),
                retry: soa.retry(),
                expire: soa.expire(),
                minimum: soa.minimum(),
            },
            _ => return None, // Unsupported record type
        };

        Some(DnsRecord {
            name: record.name().to_string(),
            record_type: record.record_type(),
            class: record.dns_class(),
            ttl: record.ttl(),
            data,
        })
    }

    async fn update_stats(&self, is_trustchain: bool, response_time_ms: f64) {
        let mut stats = self.stats.write().await;
        stats.queries_processed += 1;
        
        if is_trustchain {
            stats.trustchain_queries += 1;
        } else {
            stats.upstream_queries += 1;
        }

        // Update average response time (exponential moving average)
        if stats.average_response_time_ms == 0.0 {
            stats.average_response_time_ms = response_time_ms;
        } else {
            stats.average_response_time_ms = 0.9 * stats.average_response_time_ms + 0.1 * response_time_ms;
        }

        stats.last_update = SystemTime::now();
    }

    async fn update_stats_failure(&self) {
        let mut stats = self.stats.write().await;
        stats.queries_processed += 1;
        stats.failed_queries += 1;
        stats.last_update = SystemTime::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;
    use trust_dns_proto::rr::RecordType;

    fn create_test_query() -> DnsQuery {
        DnsQuery {
            id: 1234,
            name: "example.com".to_string(),
            record_type: RecordType::AAAA,
            class: DNSClass::IN,
            client_addr: Ipv6Addr::LOCALHOST,
            timestamp: SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn test_resolver_creation() {
        let upstream_resolvers = vec![
            "2001:4860:4860::8888".parse().unwrap(), // Google IPv6
            "2606:4700:4700::1111".parse().unwrap(), // Cloudflare IPv6
        ];
        let trustchain_domains = vec!["hypermesh".to_string(), "caesar".to_string()];

        let resolver = TrustChainResolver::new(upstream_resolvers, trustchain_domains).await;
        assert!(resolver.is_ok());
    }

    #[tokio::test]
    async fn test_stats_initialization() {
        let resolver = TrustChainResolver::new(
            vec!["2001:4860:4860::8888".parse().unwrap()],
            vec!["hypermesh".to_string()],
        ).await.unwrap();

        let stats = resolver.get_stats().await;
        assert_eq!(stats.queries_processed, 0);
        assert_eq!(stats.upstream_queries, 0);
        assert_eq!(stats.trustchain_queries, 0);
    }

    #[tokio::test]
    async fn test_stats_update() {
        let resolver = TrustChainResolver::new(
            vec!["2001:4860:4860::8888".parse().unwrap()],
            vec!["hypermesh".to_string()],
        ).await.unwrap();

        resolver.update_stats(false, 150.0).await;
        resolver.update_stats(true, 50.0).await;

        let stats = resolver.get_stats().await;
        assert_eq!(stats.queries_processed, 2);
        assert_eq!(stats.upstream_queries, 1);
        assert_eq!(stats.trustchain_queries, 1);
        assert_eq!(stats.average_response_time_ms, 100.0); // (150 + 50) / 2
    }

    #[tokio::test]
    async fn test_failure_stats_update() {
        let resolver = TrustChainResolver::new(
            vec!["2001:4860:4860::8888".parse().unwrap()],
            vec!["hypermesh".to_string()],
        ).await.unwrap();

        resolver.update_stats_failure().await;

        let stats = resolver.get_stats().await;
        assert_eq!(stats.queries_processed, 1);
        assert_eq!(stats.failed_queries, 1);
    }

    #[tokio::test]
    async fn test_record_conversion_ipv6() {
        let resolver = TrustChainResolver::new(
            vec!["2001:4860:4860::8888".parse().unwrap()],
            vec!["hypermesh".to_string()],
        ).await.unwrap();

        let name = Name::from_utf8("test.example.com").unwrap();
        let ipv6_addr = Ipv6Addr::LOCALHOST;
        let trust_dns_record = Record::from_rdata(name, 300, RData::AAAA(ipv6_addr));

        let dns_record = resolver.convert_record(&trust_dns_record);
        assert!(dns_record.is_some());

        let dns_record = dns_record.unwrap();
        assert_eq!(dns_record.name, "test.example.com");
        assert_eq!(dns_record.record_type, RecordType::AAAA);
        assert_eq!(dns_record.ttl, 300);

        if let DnsRecordData::AAAA(addr) = dns_record.data {
            assert_eq!(addr, ipv6_addr);
        } else {
            panic!("Expected AAAA record data");
        }
    }

    #[tokio::test]
    async fn test_record_conversion_cname() {
        let resolver = TrustChainResolver::new(
            vec!["2001:4860:4860::8888".parse().unwrap()],
            vec!["hypermesh".to_string()],
        ).await.unwrap();

        let name = Name::from_utf8("alias.example.com").unwrap();
        let target = Name::from_utf8("target.example.com").unwrap();
        let trust_dns_record = Record::from_rdata(name, 300, RData::CNAME(target));

        let dns_record = resolver.convert_record(&trust_dns_record);
        assert!(dns_record.is_some());

        let dns_record = dns_record.unwrap();
        assert_eq!(dns_record.record_type, RecordType::CNAME);

        if let DnsRecordData::CNAME(target) = dns_record.data {
            assert_eq!(target, "target.example.com");
        } else {
            panic!("Expected CNAME record data");
        }
    }

    #[tokio::test]
    async fn test_record_conversion_mx() {
        let resolver = TrustChainResolver::new(
            vec!["2001:4860:4860::8888".parse().unwrap()],
            vec!["hypermesh".to_string()],
        ).await.unwrap();

        let name = Name::from_utf8("example.com").unwrap();
        let exchange = Name::from_utf8("mail.example.com").unwrap();
        let mx_data = trust_dns_proto::rr::rdata::MX::new(10, exchange);
        let trust_dns_record = Record::from_rdata(name, 300, RData::MX(mx_data));

        let dns_record = resolver.convert_record(&trust_dns_record);
        assert!(dns_record.is_some());

        let dns_record = dns_record.unwrap();
        assert_eq!(dns_record.record_type, RecordType::MX);

        if let DnsRecordData::MX { priority, exchange } = dns_record.data {
            assert_eq!(priority, 10);
            assert_eq!(exchange, "mail.example.com");
        } else {
            panic!("Expected MX record data");
        }
    }
}