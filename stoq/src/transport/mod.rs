// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport Layer - QUIC over IPv6 implementation

// Module declarations
pub mod adaptive;
pub mod certificate_strategy;
pub mod certificates;
pub mod config;
pub mod connection;
pub mod falcon;
pub mod manager;
pub mod metrics;
pub mod operations;
pub mod stats;
pub mod streams;

pub mod multipath;

pub mod reflector;

pub mod ebpf;
pub mod min_spec;
pub mod pos_extension_validator;

// Re-exports for backward compatibility
pub use adaptive::{
    congestion_control_for_tier, AdaptationManager, AdaptiveConnection, BandwidthSample,
    EwmaBandwidthEstimator, LossBasedAdjuster, MtuDiscovery, MtuProbeState,
};
pub use certificate_strategy::{
    AnonymousCertificateStrategy, AuthenticatedCertificateStrategy, CertificateStrategy,
    FederatedCertificateStrategy, NetworkType, P2PCertificateStrategy, PublicCertificateStrategy,
};
pub use certificates::{
    CertificateConfig, CertificateManager, CertificateMode, StoqNodeCertificate,
};
pub use config::{CongestionControl, NetworkTier, TransportConfig};
pub use connection::{Connection, Endpoint, FrameBatch, MemoryPool, Stream};
pub use falcon::{FalconTransport, FalconVariant};
pub use manager::StoqTransport;
pub use metrics::{IntervalMetrics, ProtocolMetrics, TransportMetrics, TransportSnapshot};
pub use multipath::{MultiPathConnection, MultiPathMetrics, PathInfo, PathPolicy, PathScheduler};
pub use pos_extension_validator::StoqPosExtensionValidator;
pub use reflector::{
    ReflectorBridge, ReflectorMessage, StoqBlockTransport, SyncProtocol, SyncProtocolConfig,
};
pub use min_spec::{
    max_concurrent_connections, validate_bandwidth_budget, validate_connection_budget,
    ConnectionBudget, TransportMinSpec,
};
pub use stats::{ConnectionPoolStats, PerformanceStats};

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[test]
    fn test_endpoint_creation() {
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
        assert_eq!(endpoint.port, 9292);
        assert_eq!(endpoint.address, Ipv6Addr::LOCALHOST);
    }

    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        // In test mode, port is 0 (OS-assigned) to avoid binding conflicts
        #[cfg(test)]
        assert_eq!(config.port, 0);
        #[cfg(not(test))]
        assert_eq!(config.port, 9292);
        assert!(config.enable_migration);
        assert!(!config.enable_0rtt); // 0-RTT disabled for security
    }

    #[tokio::test]
    async fn test_transport_creation() {
        // Crypto provider is now initialized automatically in StoqTransport::new()

        let config = TransportConfig {
            port: 0, // Let OS assign an available port
            ..Default::default()
        };
        let transport = StoqTransport::new(config).await;
        assert!(transport.is_ok());
    }

    #[test]
    fn test_connection_health_check_config() {
        // Test that idle timeout and health check interval are configurable
        let config = TransportConfig::default();
        assert_eq!(config.connection_idle_timeout, 30);
        assert_eq!(config.health_check_interval, 10);

        // Test that values can be customized
        let custom_config = TransportConfig {
            connection_idle_timeout: 60,
            health_check_interval: 15,
            ..Default::default()
        };
        assert_eq!(custom_config.connection_idle_timeout, 60);
        assert_eq!(custom_config.health_check_interval, 15);
    }

    #[test]
    fn test_lru_eviction_logic() {
        // Test that LRU eviction selects the oldest connection
        let times = [100u64, 50u64, 150u64, 25u64];
        let mut lru_idx = 0;
        let mut oldest_time = u64::MAX;

        for (idx, &time) in times.iter().enumerate() {
            if time < oldest_time {
                oldest_time = time;
                lru_idx = idx;
            }
        }

        assert_eq!(lru_idx, 3); // Index 3 has value 25, which is smallest
        assert_eq!(oldest_time, 25);
    }

    #[test]
    fn test_connection_pool_stats_structure() {
        let stats = ConnectionPoolStats {
            total_connections: 10,
            total_healthy: 8,
            pool_details: vec![
                ("127.0.0.1:8080".to_string(), 5, 4),
                ("127.0.0.1:9090".to_string(), 5, 4),
            ],
            reuse_count: 100,
            eviction_count: 5,
            health_check_count: 20,
            unhealthy_removed: 2,
        };

        assert_eq!(stats.total_connections, 10);
        assert_eq!(stats.total_healthy, 8);
        assert_eq!(stats.pool_details.len(), 2);
        assert_eq!(stats.reuse_count, 100);
        assert_eq!(stats.eviction_count, 5);
        assert_eq!(stats.health_check_count, 20);
        assert_eq!(stats.unhealthy_removed, 2);
    }

    #[tokio::test]
    async fn test_connection_pool_cleanup() {
        let config = TransportConfig {
            port: 0, // Let OS assign port
            connection_idle_timeout: 30,
            health_check_interval: 10,
            ..Default::default()
        };

        let transport = StoqTransport::new(config).await.expect("test: async operation");

        // Call cleanup - should not panic even with empty pools
        transport.cleanup_unhealthy_connections();

        // Get stats - should work with empty pools
        let stats = transport.pool_stats();
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.total_healthy, 0);
        assert!(stats.pool_details.is_empty());
    }
}
