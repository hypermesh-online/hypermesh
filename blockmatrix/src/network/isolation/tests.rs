// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Tests for network isolation manager

use super::*;
use crate::network::isolation::default::DefaultIsolationManager;
use crate::network::trust::new_random_network_id;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_isolation_cross_boundary() {
        // Create isolation manager
        let isolation = DefaultIsolationManager::new();

        // Configure two separate networks
        let net1 = new_random_network_id();
        let net2 = new_random_network_id();

        isolation.configure_network(net1, NetworkType::Anonymous).await.expect("test: isolation op");
        isolation.configure_network(net2, NetworkType::P2P).await.expect("test: isolation op");

        // Create packet attempting to cross network boundary
        let packet = Packet {
            id: PacketId::new_v4(),
            source_network: net1,
            destination_network: net2,
            payload_hash: zero_hash(),
            timestamp: Utc::now(),
        };

        // Should be rejected due to boundary violation
        let result = isolation.validate_packet(&packet).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot cross network boundary"));

        // Check violation was logged
        let violations = isolation.check_violations().await;
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_type, ViolationType::CrossNetworkPacket);
        assert_eq!(violations[0].source_network, net1);
        assert_eq!(violations[0].destination_network, net2);

        // Check statistics
        let stats = isolation.get_stats().await;
        assert_eq!(stats.packets_validated, 1);
        assert_eq!(stats.packets_rejected, 1);
        assert_eq!(stats.violations_detected, 1);
    }

    #[tokio::test]
    async fn test_same_network_allowed() {
        let isolation = DefaultIsolationManager::new();
        let net_id = new_random_network_id();

        isolation.configure_network(net_id, NetworkType::Public).await.expect("test: isolation op");

        // Packet within same network should be allowed
        let packet = Packet {
            id: PacketId::new_v4(),
            source_network: net_id,
            destination_network: net_id,
            payload_hash: zero_hash(),
            timestamp: Utc::now(),
        };

        // Should be allowed
        let result = isolation.validate_packet(&packet).await;
        assert!(result.is_ok());

        // No violations should be recorded
        let violations = isolation.check_violations().await;
        assert_eq!(violations.len(), 0);

        // Check statistics
        let stats = isolation.get_stats().await;
        assert_eq!(stats.packets_validated, 1);
        assert_eq!(stats.packets_rejected, 0);
        assert_eq!(stats.violations_detected, 0);
    }

    #[tokio::test]
    async fn test_connection_pool_isolation() {
        let isolation = DefaultIsolationManager::new();

        let net1 = new_random_network_id();
        let net2 = new_random_network_id();

        // Configure two networks
        isolation.configure_network(net1, NetworkType::Anonymous).await.expect("test: isolation op");
        isolation.configure_network(net2, NetworkType::Federated {
            gateway_url: "test.federation.local".to_string()
        }).await.expect("test: isolation op");

        // Get connection pools
        let pool1 = isolation.get_connection_pool(net1).await.expect("test: isolation op");
        let pool2 = isolation.get_connection_pool(net2).await.expect("test: isolation op");

        // Pools must be different instances
        assert!(!Arc::ptr_eq(&pool1, &pool2));

        // Each pool should belong to its network
        assert_eq!(pool1.network_id, net1);
        assert_eq!(pool2.network_id, net2);

        // Add connections to each pool
        pool1.add_connection(Connection::new("192.168.1.1:8080".to_string())).await.expect("test: isolation op");
        pool2.add_connection(Connection::new("192.168.1.2:8080".to_string())).await.expect("test: isolation op");

        // Verify isolation
        assert_eq!(pool1.connection_count().await, 1);
        assert_eq!(pool2.connection_count().await, 1);
    }

    #[tokio::test]
    async fn test_network_removal() {
        let isolation = DefaultIsolationManager::new();
        let net_id = new_random_network_id();

        // Configure network
        isolation.configure_network(net_id, NetworkType::P2P).await.expect("test: isolation op");

        // Add some connections
        let pool = isolation.get_connection_pool(net_id).await.expect("test: isolation op");
        pool.add_connection(Connection::new("10.0.0.1:5000".to_string())).await.expect("test: isolation op");
        pool.add_connection(Connection::new("10.0.0.2:5000".to_string())).await.expect("test: isolation op");

        assert_eq!(pool.connection_count().await, 2);

        // Track some packets
        let packet = Packet::new(net_id, net_id, zero_hash());
        isolation.validate_packet(&packet).await.expect("test: isolation op");

        // Remove network
        isolation.remove_network(net_id).await.expect("test: isolation op");

        // Should no longer be able to get pool
        let result = isolation.get_connection_pool(net_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_networks() {
        let isolation = DefaultIsolationManager::new();

        // Create 4 different network types
        let anon_net = new_random_network_id();
        let p2p_net = new_random_network_id();
        let fed_net = new_random_network_id();
        let pub_net = new_random_network_id();

        // Configure all networks
        isolation.configure_network(anon_net, NetworkType::Anonymous).await.expect("test: isolation op");
        isolation.configure_network(p2p_net, NetworkType::P2P).await.expect("test: isolation op");
        isolation.configure_network(fed_net, NetworkType::Federated {
            gateway_url: "fed.example.com".to_string()
        }).await.expect("test: isolation op");
        isolation.configure_network(pub_net, NetworkType::Public).await.expect("test: isolation op");

        // Create packets for each network (same-network only)
        let packets = vec![
            Packet::new(anon_net, anon_net, zero_hash()),
            Packet::new(p2p_net, p2p_net, zero_hash()),
            Packet::new(fed_net, fed_net, zero_hash()),
            Packet::new(pub_net, pub_net, zero_hash()),
        ];

        // All should be valid
        for packet in packets {
            let result = isolation.validate_packet(&packet).await;
            assert!(result.is_ok(), "Packet validation failed: {:?}", result);
        }

        // Create cross-network packets (should all fail)
        let cross_packets = vec![
            Packet::new(anon_net, p2p_net, zero_hash()),
            Packet::new(p2p_net, fed_net, zero_hash()),
            Packet::new(fed_net, pub_net, zero_hash()),
            Packet::new(pub_net, anon_net, zero_hash()),
        ];

        for packet in cross_packets {
            let result = isolation.validate_packet(&packet).await;
            assert!(result.is_err(), "Cross-network packet should have been rejected");
        }

        // Check violations
        let violations = isolation.check_violations().await;
        assert_eq!(violations.len(), 4);
        for violation in violations {
            assert_eq!(violation.violation_type, ViolationType::CrossNetworkPacket);
        }

        // Check stats
        let stats = isolation.get_stats().await;
        assert_eq!(stats.active_networks, 4);
        assert_eq!(stats.packets_validated, 8);
        assert_eq!(stats.packets_rejected, 4);
        assert_eq!(stats.violations_detected, 4);
    }

    #[tokio::test]
    async fn test_violation_history_limit() {
        let isolation = DefaultIsolationManager::with_violation_limit(5);

        let net1 = new_random_network_id();
        let net2 = new_random_network_id();

        isolation.configure_network(net1, NetworkType::Anonymous).await.expect("test: isolation op");
        isolation.configure_network(net2, NetworkType::P2P).await.expect("test: isolation op");

        // Create 10 violations
        for _i in 0..10 {
            let packet = Packet {
                id: PacketId::new_v4(),
                source_network: net1,
                destination_network: net2,
                payload_hash: zero_hash(),
                timestamp: Utc::now(),
            };
            let _ = isolation.validate_packet(&packet).await;
        }

        // Should only keep last 5 violations
        let violations = isolation.check_violations().await;
        assert_eq!(violations.len(), 5);

        // Stats should show all violations
        let stats = isolation.get_stats().await;
        assert_eq!(stats.violations_detected, 10);
    }

    #[tokio::test]
    async fn test_clear_violations() {
        let isolation = DefaultIsolationManager::new();

        let net1 = new_random_network_id();
        let net2 = new_random_network_id();

        isolation.configure_network(net1, NetworkType::Anonymous).await.expect("test: isolation op");
        isolation.configure_network(net2, NetworkType::Public).await.expect("test: isolation op");

        // Create some violations
        for _ in 0..3 {
            let packet = Packet::new(net1, net2, zero_hash());
            let _ = isolation.validate_packet(&packet).await;
        }

        // Verify violations exist
        assert_eq!(isolation.check_violations().await.len(), 3);

        // Clear violations
        isolation.clear_violations().await.expect("test: isolation op");

        // Verify violations cleared
        assert_eq!(isolation.check_violations().await.len(), 0);

        // Stats should also be cleared
        let stats = isolation.get_stats().await;
        assert_eq!(stats.violations_detected, 0);
        assert!(stats.violations_by_type.is_empty());
    }

    #[tokio::test]
    async fn test_connection_pool_capacity() {
        let isolation = DefaultIsolationManager::new();
        let net_id = new_random_network_id();

        isolation.configure_network(net_id, NetworkType::P2P).await.expect("test: isolation op");

        let pool = isolation.get_connection_pool(net_id).await.expect("test: isolation op");

        // Pool should have capacity initially
        assert!(pool.has_capacity().await);

        // Add connections up to limit
        for i in 0..100 {
            let conn = Connection::new(format!("192.168.1.{}:8080", i + 1));
            pool.add_connection(conn).await.expect("test: isolation op");
        }

        // Should be at capacity
        assert!(!pool.has_capacity().await);
        assert_eq!(pool.connection_count().await, 100);

        // Adding another should fail
        let result = pool.add_connection(Connection::new("192.168.1.200:8080".to_string())).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Connection pool full"));
    }

    #[tokio::test]
    async fn test_duplicate_network_configuration() {
        let isolation = DefaultIsolationManager::new();
        let net_id = new_random_network_id();

        // First configuration should succeed
        isolation.configure_network(net_id, NetworkType::Anonymous).await.expect("test: isolation op");

        // Second configuration with same ID should fail
        let result = isolation.configure_network(net_id, NetworkType::P2P).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already configured"));
    }

    #[tokio::test]
    async fn test_unconfigured_network_packet() {
        let isolation = DefaultIsolationManager::new();

        // Create packet for unconfigured network
        let net_id = new_random_network_id();
        let packet = Packet::new(net_id, net_id, zero_hash());

        // Should fail validation
        let result = isolation.validate_packet(&packet).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not configured"));
    }

    #[tokio::test]
    async fn test_statistics_accuracy() {
        let isolation = DefaultIsolationManager::new();

        // Configure multiple networks
        let net1 = new_random_network_id();
        let net2 = new_random_network_id();
        let net3 = new_random_network_id();

        isolation.configure_network(net1, NetworkType::Anonymous).await.expect("test: isolation op");
        isolation.configure_network(net2, NetworkType::P2P).await.expect("test: isolation op");
        isolation.configure_network(net3, NetworkType::Public).await.expect("test: isolation op");

        // Valid packets
        for _ in 0..5 {
            let packet = Packet::new(net1, net1, zero_hash());
            isolation.validate_packet(&packet).await.expect("test: isolation op");
        }

        // Invalid packets (cross-network)
        for _ in 0..3 {
            let packet = Packet::new(net1, net2, zero_hash());
            let _ = isolation.validate_packet(&packet).await;
        }

        // Get stats
        let stats = isolation.get_stats().await;

        assert_eq!(stats.active_networks, 3);
        assert_eq!(stats.packets_validated, 8);
        assert_eq!(stats.packets_rejected, 3);
        assert_eq!(stats.violations_detected, 3);
        assert_eq!(
            stats.violations_by_type.get(&ViolationType::CrossNetworkPacket.to_string()),
            Some(&3)
        );
    }
}
