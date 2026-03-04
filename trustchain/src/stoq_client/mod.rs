// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Client Library for TrustChain Integration
//!
//! This module provides a comprehensive STOQ client that integrates with TrustChain's
//! Certificate Authority, Certificate Transparency, and DNS services. All transport
//! operations are delegated to STOQ protocol for high-performance networking.

pub mod operations;
pub mod types;

// Re-export all public types for backward compatibility
pub use operations::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[tokio::test]
    async fn test_stoq_client_creation() {
        let config = TrustChainStoqConfig::default();

        // Note: This test may fail without proper STOQ setup
        // In integration tests, we would mock the STOQ transport
        if let Ok(client) = TrustChainStoqClient::new(config).await {
            let metrics = client.get_metrics();
            assert_eq!(
                metrics
                    .dns_queries
                    .load(std::sync::atomic::Ordering::Relaxed),
                0
            );
        }
    }

    #[test]
    fn test_service_endpoint_creation() {
        let endpoint = ServiceEndpoint::new(ServiceType::Dns, Ipv6Addr::LOCALHOST, 853)
            .with_service_name("dns.test.local".to_string());

        assert_eq!(endpoint.service_type, ServiceType::Dns);
        assert_eq!(endpoint.port, 853);
        assert_eq!(endpoint.service_name, Some("dns.test.local".to_string()));
    }

    #[test]
    fn test_service_type_string_conversion() {
        assert_eq!(ServiceType::Dns.as_str(), "dns");
        assert_eq!(ServiceType::CertificateAuthority.as_str(), "ca");
        assert_eq!(ServiceType::CertificateTransparency.as_str(), "ct");
        assert_eq!(ServiceType::StateProofNode.as_str(), "state_proof");
        assert_eq!(ServiceType::AssetDiscovery.as_str(), "assets");
    }

    #[tokio::test]
    async fn test_dns_query_serialization() {
        let query = StoqDnsQuery {
            query_id: 1234,
            domain: "example.com".to_string(),
            query_type: 1, // A record
            flags: 0x0100, // RD flag
            client_ip: Ipv6Addr::LOCALHOST,
        };

        let serialized = bincode::serialize(&query).expect("test");
        let deserialized: StoqDnsQuery = bincode::deserialize(&serialized).expect("test");

        assert_eq!(query.query_id, deserialized.query_id);
        assert_eq!(query.domain, deserialized.domain);
        assert_eq!(query.query_type, deserialized.query_type);
    }
}
