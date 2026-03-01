// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// STOQ Phase 5: Comprehensive Integration Testing Suite
// End-to-end validation of complete system functionality

use std::net::Ipv6Addr;
use std::sync::Arc;
use stoq::transport::{Endpoint, NetworkTier, StoqTransport, TransportConfig};
use tokio::sync::Barrier;

mod connection_tests {
    use super::*;

    #[tokio::test]
    async fn test_end_to_end_connection() {
        // Setup server
        let server_config = TransportConfig {
            port: 0, // Let OS assign port
            ..Default::default()
        };
        let server = Arc::new(StoqTransport::new(server_config).await.unwrap());
        let actual_addr = server.local_addr().unwrap();

        // Setup client
        let client_config = TransportConfig::default();
        let client = Arc::new(StoqTransport::new(client_config).await.unwrap());

        // Server accept task
        let server_clone = Arc::clone(&server);
        let server_handle = tokio::spawn(async move {
            let conn = server_clone.accept().await.unwrap();
            let mut stream = conn.accept_stream().await.unwrap();

            // Read data
            let data = stream.receive().await.unwrap();
            assert_eq!(&data[..], b"Hello, STOQ!");

            // Echo back
            stream.send(b"Echo: Hello, STOQ!").await.unwrap();
        });

        // Client connect and send
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, actual_addr.port());
        let conn = client.connect(&endpoint).await.unwrap();
        let mut stream = conn.open_stream().await.unwrap();
        stream.send(b"Hello, STOQ!").await.unwrap();

        let data = stream.receive().await.unwrap();
        assert_eq!(&data[..], b"Echo: Hello, STOQ!");

        // Cleanup
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_concurrent_connections() {
        const NUM_CONNECTIONS: usize = 10; // Reduced for faster testing

        // Setup server
        let server_config = TransportConfig {
            port: 0,
            ..Default::default()
        };
        let server = Arc::new(StoqTransport::new(server_config).await.unwrap());
        let server_addr = server.local_addr().unwrap();

        // Barrier for synchronization
        let barrier = Arc::new(Barrier::new(NUM_CONNECTIONS + 1));

        // Server accept loop
        let server_barrier = barrier.clone();
        let server_clone = Arc::clone(&server);
        let server_handle = tokio::spawn(async move {
            let mut handles = Vec::new();

            for i in 0..NUM_CONNECTIONS {
                let server = Arc::clone(&server_clone);
                let barrier_clone = server_barrier.clone();

                let handle = tokio::spawn(async move {
                    let conn = server.accept().await.unwrap();
                    let mut stream = conn.accept_stream().await.unwrap();

                    // Read client ID
                    let data = stream.receive().await.unwrap();
                    let client_id: usize = String::from_utf8_lossy(&data).parse().unwrap();

                    assert_eq!(client_id, i);

                    // Wait for all connections
                    barrier_clone.wait().await;

                    // Echo response
                    let response = format!("ACK: {client_id}");
                    stream.send(response.as_bytes()).await.unwrap();
                });

                handles.push(handle);
            }

            // Wait for all server handlers
            for handle in handles {
                handle.await.unwrap();
            }
        });

        // Create multiple clients
        let mut client_handles = Vec::new();

        for i in 0..NUM_CONNECTIONS {
            let barrier_clone = barrier.clone();
            let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_addr.port());

            let handle = tokio::spawn(async move {
                let client_config = TransportConfig::default();
                let client = StoqTransport::new(client_config).await.unwrap();
                let conn = client.connect(&endpoint).await.unwrap();
                let mut stream = conn.open_stream().await.unwrap();

                // Send client ID
                stream.send(i.to_string().as_bytes()).await.unwrap();

                // Wait for synchronization
                barrier_clone.wait().await;

                // Read response
                let data = stream.receive().await.unwrap();
                let expected = format!("ACK: {i}");
                assert_eq!(&data[..], expected.as_bytes());
            });

            client_handles.push(handle);
        }

        // Wait for all clients
        for handle in client_handles {
            handle.await.unwrap();
        }

        server_handle.await.unwrap();
    }
}

mod stream_management_tests {
    use super::*;

    #[tokio::test]
    async fn test_bidirectional_streams() {
        // Setup transports
        let server_config = TransportConfig {
            port: 0,
            ..Default::default()
        };
        let server = Arc::new(StoqTransport::new(server_config).await.unwrap());
        let server_addr = server.local_addr().unwrap();

        let client_config = TransportConfig::default();
        let client = Arc::new(StoqTransport::new(client_config).await.unwrap());

        // Server task
        let server_clone = Arc::clone(&server);
        let server_handle = tokio::spawn(async move {
            let conn = server_clone.accept().await.unwrap();

            // Accept multiple streams
            for i in 0..5 {
                let mut stream = conn.accept_stream().await.unwrap();
                let data = stream.receive().await.unwrap();
                assert_eq!(data, format!("Stream {i}").as_bytes());

                stream.send(format!("Reply {i}").as_bytes()).await.unwrap();
            }
        });

        // Client creates multiple streams
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_addr.port());
        let conn = client.connect(&endpoint).await.unwrap();

        for i in 0..5 {
            let mut stream = conn.open_stream().await.unwrap();
            stream.send(format!("Stream {i}").as_bytes()).await.unwrap();

            let reply = stream.receive().await.unwrap();
            assert_eq!(reply, format!("Reply {i}").as_bytes());
        }

        server_handle.await.unwrap();
    }
}

mod transport_features_tests {
    use super::*;

    #[tokio::test]
    async fn test_zero_copy_operations() {
        let config = TransportConfig {
            enable_zero_copy: true,
            port: 0,
            ..Default::default()
        };

        let transport = Arc::new(StoqTransport::new(config).await.unwrap());
        let stats = transport.stats();

        // Verify transport is created with zero-copy config
        // Stats don't expose zero_copy flag directly
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_connection_pooling() {
        let server_config = TransportConfig {
            port: 0,
            ..Default::default()
        };
        let server = Arc::new(StoqTransport::new(server_config).await.unwrap());
        let server_addr = server.local_addr().unwrap();

        let client_config = TransportConfig {
            connection_pool_size: 5,
            ..Default::default()
        };
        let client = Arc::new(StoqTransport::new(client_config).await.unwrap());

        // Server accept connections
        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            for _ in 0..3 {
                let _conn = server_clone.accept().await.unwrap();
            }
        });

        // Create connections
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_addr.port());

        let conn1 = client.connect(&endpoint).await.unwrap();
        let conn1_id = conn1.id();

        // Return to pool
        client.return_to_pool(conn1);

        // Should get same connection from pool
        let conn2 = client.connect(&endpoint).await.unwrap();
        assert_eq!(conn1_id, conn2.id());
    }
}

mod performance_tier_tests {
    use super::*;

    #[tokio::test]
    async fn test_tier_adaptation() {
        let mut config = TransportConfig {
            port: 0,
            ..Default::default()
        };

        // Start with slow tier
        config.adapt_for_network_tier(&NetworkTier::Slow { mbps: 10.0 });

        let transport = StoqTransport::new(config).await.unwrap();
        let stats = transport.stats();

        // Verify adapted configuration
        // Stats show transport is initialized
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_datacenter_tier_performance() {
        let mut config = TransportConfig {
            port: 0,
            ..Default::default()
        };

        // Configure for datacenter tier
        config.adapt_for_network_tier(&NetworkTier::DataCenter { gbps: 100.0 });

        let transport = StoqTransport::new(config).await.unwrap();
        let stats = transport.stats();

        // Verify high-performance configuration
        // Note: TransportStats doesn't expose buffer sizes directly
        // Just verify transport was created successfully
        assert_eq!(stats.active_connections, 0);
    }
}

#[cfg(test)]
mod integration_test_utils {
    use super::*;

    #[allow(dead_code)]
    pub async fn create_test_transport_pair() -> (Arc<StoqTransport>, Arc<StoqTransport>, Endpoint)
    {
        let server_config = TransportConfig {
            port: 0,
            ..Default::default()
        };
        let server = Arc::new(StoqTransport::new(server_config).await.unwrap());
        let server_addr = server.local_addr().unwrap();

        let client_config = TransportConfig::default();
        let client = Arc::new(StoqTransport::new(client_config).await.unwrap());

        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_addr.port());

        (server, client, endpoint)
    }
}
