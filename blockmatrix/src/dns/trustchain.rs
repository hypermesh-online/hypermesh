// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain DNS Service Integration
//!
//! Integrates with TrustChain DNS|CA|CT service layer.
//! TrustChain provides DNS service similar to how UDP provides DNS transport.

use super::{DnsRecord, DnsRecordType, DnsRecordData, DnsError, DnsResult};
use std::sync::Arc;
use std::net::Ipv6Addr;
use tracing::{debug, warn};

/// TrustChain DNS service
/// This represents the service layer that STOQ uses (like DNS over UDP)
pub struct TrustChainDnsService {
    /// Service endpoint
    endpoint: Ipv6Addr,
    /// Service port
    port: u16,
}

impl TrustChainDnsService {
    /// Create new TrustChain DNS service
    pub fn new(endpoint: Ipv6Addr, port: u16) -> Self {
        Self { endpoint, port }
    }

    /// Get service endpoint
    pub fn endpoint(&self) -> Ipv6Addr {
        self.endpoint
    }

    /// Get service port
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// TrustChain DNS client
/// Client for querying TrustChain DNS service
pub struct TrustChainDnsClient {
    /// TrustChain service
    service: Arc<TrustChainDnsService>,
    /// Enable integration
    enabled: bool,
}

impl TrustChainDnsClient {
    /// Create new TrustChain DNS client
    pub fn new(service: Arc<TrustChainDnsService>) -> Self {
        Self {
            service,
            enabled: true,
        }
    }

    /// Query TrustChain DNS service
    pub async fn query(
        &self,
        domain: &str,
        record_type: &DnsRecordType,
    ) -> DnsResult<Vec<DnsRecord>> {
        if !self.enabled {
            return Err(DnsError::TrustChainError(
                "TrustChain DNS client is disabled".to_string(),
            ));
        }

        debug!(
            "Querying TrustChain DNS: {} ({:?}) at [{}]:{}",
            domain,
            record_type,
            self.service.endpoint(),
            self.service.port()
        );

        // TODO: Implement actual STOQ-based query to TrustChain DNS service
        // For now, return error indicating service integration is pending
        warn!(
            "TrustChain DNS service integration pending for: {}",
            domain
        );

        // Placeholder: Return known TrustChain domains for testing
        if self.is_trustchain_domain(domain) {
            let ipv6 = self.service.endpoint();
            Ok(vec![DnsRecord::new(
                domain.to_string(),
                DnsRecordType::AAAA,
                DnsRecordData::AAAA(ipv6),
                300,
                "trustchain".to_string(),
            )])
        } else {
            Err(DnsError::DomainNotFound {
                domain: domain.to_string(),
            })
        }
    }

    /// Check if domain is a TrustChain service domain
    fn is_trustchain_domain(&self, domain: &str) -> bool {
        matches!(
            domain,
            "hypermesh" | "caesar" | "trust" | "assets" | "catalog"
        )
    }

    /// Enable/disable client
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if client is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_trustchain_dns_service() {
        let service = Arc::new(TrustChainDnsService::new(Ipv6Addr::LOCALHOST, 8053));
        assert_eq!(service.endpoint(), Ipv6Addr::LOCALHOST);
        assert_eq!(service.port(), 8053);
    }

    #[tokio::test]
    async fn test_trustchain_dns_client_query() {
        let service = Arc::new(TrustChainDnsService::new(Ipv6Addr::LOCALHOST, 8053));
        let client = TrustChainDnsClient::new(service);

        // Query TrustChain domain (placeholder implementation)
        let result = client.query("hypermesh", &DnsRecordType::AAAA).await;
        assert!(result.is_ok());

        let records = result.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].domain, "hypermesh");
    }

    #[tokio::test]
    async fn test_trustchain_domain_detection() {
        let service = Arc::new(TrustChainDnsService::new(Ipv6Addr::LOCALHOST, 8053));
        let client = TrustChainDnsClient::new(service);

        assert!(client.is_trustchain_domain("hypermesh"));
        assert!(client.is_trustchain_domain("caesar"));
        assert!(client.is_trustchain_domain("trust"));
        assert!(client.is_trustchain_domain("assets"));
        assert!(!client.is_trustchain_domain("google"));
    }

    #[tokio::test]
    async fn test_client_enable_disable() {
        let service = Arc::new(TrustChainDnsService::new(Ipv6Addr::LOCALHOST, 8053));
        let mut client = TrustChainDnsClient::new(service);

        assert!(client.is_enabled());

        client.set_enabled(false);
        assert!(!client.is_enabled());

        let result = client.query("hypermesh", &DnsRecordType::AAAA).await;
        assert!(result.is_err());
    }
}
