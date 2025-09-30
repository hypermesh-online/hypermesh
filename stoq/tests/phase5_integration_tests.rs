// STOQ Phase 5: Comprehensive Integration Testing Suite
// End-to-end validation of complete system functionality

use stoq::{StoqTransport, Config, NetworkTier};
use std::net::{SocketAddr, Ipv6Addr};
use std::sync::Arc;
use tokio::sync::Barrier;
use tokio::time::{timeout, Duration};
use bytes::Bytes;

mod connection_tests {
    use super::*;

    #[tokio::test]
    async fn test_end_to_end_connection() {
        // Setup server
        let server_config = Config::default();
        let server = StoqTransport::new(server_config);
        let server_addr: SocketAddr = "[::1]:0".parse().unwrap();
        let listener = server.listen(server_addr).await.unwrap();
        let actual_addr = listener.local_addr().unwrap();

        // Setup client
        let client_config = Config::default();
        let client = StoqTransport::new(client_config);

        // Server accept task
        let server_handle = tokio::spawn(async move {
            let (stream, peer_addr) = listener.accept().await.unwrap();
            assert!(peer_addr.is_ipv6());

            // Read data
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"Hello, STOQ!");

            // Echo back
            stream.write_all(b"Echo: Hello, STOQ!").await.unwrap();
        });

        // Client connect and send
        let mut stream = client.connect(actual_addr).await.unwrap();
        stream.write_all(b"Hello, STOQ!").await.unwrap();

        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"Echo: Hello, STOQ!");

        // Cleanup
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_concurrent_connections() {
        const NUM_CONNECTIONS: usize = 100;

        // Setup server
        let server_config = Config::default();
        let server = Arc::new(StoqTransport::new(server_config));
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Barrier for synchronization
        let barrier = Arc::new(Barrier::new(NUM_CONNECTIONS + 1));

        // Server accept loop
        let server_barrier = barrier.clone();
        let server_handle = tokio::spawn(async move {
            let mut handles = Vec::new();

            for i in 0..NUM_CONNECTIONS {
                let (stream, _) = listener.accept().await.unwrap();
                let barrier_clone = server_barrier.clone();

                let handle = tokio::spawn(async move {
                    // Read client ID
                    let mut buf = vec![0u8; 64];
                    let n = stream.read(&mut buf).await.unwrap();
                    let client_id: usize = String::from_utf8_lossy(&buf[..n])
                        .parse().unwrap();

                    assert_eq!(client_id, i);

                    // Wait for all connections
                    barrier_clone.wait().await;

                    // Echo response
                    let response = format!("ACK: {}", client_id);
                    stream.write_all(response.as_bytes()).await.unwrap();
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
            let handle = tokio::spawn(async move {
                let client = StoqTransport::new(Config::default());
                let mut stream = client.connect(server_addr).await.unwrap();

                // Send client ID
                stream.write_all(i.to_string().as_bytes()).await.unwrap();

                // Wait for all to connect
                barrier_clone.wait().await;

                // Read response
                let mut buf = vec![0u8; 64];
                let n = stream.read(&mut buf).await.unwrap();
                let response = String::from_utf8_lossy(&buf[..n]);
                assert_eq!(response, format!("ACK: {}", i));
            });
            client_handles.push(handle);
        }

        // Wait for all clients
        for handle in client_handles {
            handle.await.unwrap();
        }

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_connection_migration() {
        // Test QUIC connection migration capability
        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Server task
        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();

            // Read initial data
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"Before migration");

            // Connection should survive migration
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Read post-migration data
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"After migration");

            stream.write_all(b"Migration successful").await.unwrap();
        });

        // Client with simulated migration
        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        stream.write_all(b"Before migration").await.unwrap();

        // Simulate network change (connection migration)
        stream.migrate_connection().await.unwrap();

        stream.write_all(b"After migration").await.unwrap();

        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"Migration successful");

        server_handle.await.unwrap();
    }
}

mod protocol_extension_integration {
    use super::*;

    #[tokio::test]
    async fn test_tokenized_stream_transfer() {
        // Setup server with tokenization enabled
        let mut server_config = Config::default();
        server_config.enable_tokenization = true;
        let server = StoqTransport::new(server_config);
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();

            // Receive tokenized data
            let token = stream.receive_token().await.unwrap();
            assert_eq!(token.stream_id(), 1);

            let data = stream.read_tokenized(&token).await.unwrap();
            assert_eq!(data, b"Tokenized data transfer");

            // Send tokenized response
            let response_token = stream.create_token(b"Tokenized response");
            stream.send_token(response_token).await.unwrap();
        });

        // Client with tokenization
        let mut client_config = Config::default();
        client_config.enable_tokenization = true;
        let client = StoqTransport::new(client_config);
        let mut stream = client.connect(server_addr).await.unwrap();

        // Send tokenized data
        let token = stream.create_token(b"Tokenized data transfer");
        stream.send_token(token).await.unwrap();

        // Receive tokenized response
        let response_token = stream.receive_token().await.unwrap();
        let response = stream.read_tokenized(&response_token).await.unwrap();
        assert_eq!(response, b"Tokenized response");

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_sharded_large_transfer() {
        const DATA_SIZE: usize = 10 * 1024 * 1024; // 10MB

        let mut server_config = Config::default();
        server_config.enable_sharding = true;
        server_config.shard_size = 64 * 1024; // 64KB shards
        let server = StoqTransport::new(server_config);
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Generate test data
        let test_data = vec![0xAB; DATA_SIZE];
        let test_data_clone = test_data.clone();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();

            // Receive sharded data
            let mut received = Vec::new();
            while received.len() < DATA_SIZE {
                let shard = stream.receive_shard().await.unwrap();
                received.extend_from_slice(&shard.data());
            }

            assert_eq!(received.len(), DATA_SIZE);
            assert_eq!(received, test_data_clone);

            stream.write_all(b"Transfer complete").await.unwrap();
        });

        // Client sends sharded data
        let mut client_config = Config::default();
        client_config.enable_sharding = true;
        client_config.shard_size = 64 * 1024;
        let client = StoqTransport::new(client_config);
        let mut stream = client.connect(server_addr).await.unwrap();

        // Send as shards
        stream.send_sharded(&test_data).await.unwrap();

        let mut buf = vec![0u8; 64];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"Transfer complete");

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_protocol_fallback() {
        // Server with all extensions
        let mut server_config = Config::default();
        server_config.enable_tokenization = true;
        server_config.enable_sharding = true;
        let server = StoqTransport::new(server_config);
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();

            // Check negotiated capabilities
            let caps = stream.negotiated_capabilities();
            assert!(!caps.contains("tokenization")); // Client doesn't support
            assert!(!caps.contains("sharding")); // Client doesn't support

            // Fallback to standard transfer
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"Standard transfer");

            stream.write_all(b"Fallback successful").await.unwrap();
        });

        // Client without extensions
        let client_config = Config::default(); // No extensions
        let client = StoqTransport::new(client_config);
        let mut stream = client.connect(server_addr).await.unwrap();

        stream.write_all(b"Standard transfer").await.unwrap();

        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"Fallback successful");

        server_handle.await.unwrap();
    }
}

mod tier_adaptation_tests {
    use super::*;

    #[tokio::test]
    async fn test_live_tier_switching() {
        let mut config = Config::default();
        config.adaptive_optimization = true;
        let transport = Arc::new(StoqTransport::new(config));

        // Start with auto-detection
        assert_eq!(transport.current_tier(), NetworkTier::Auto);

        // Simulate LAN conditions
        transport.inject_metrics(0.5, 10_000.0).await; // 0.5ms, 10 Gbps
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(transport.current_tier(), NetworkTier::Lan);

        // Simulate degradation to Metro
        transport.inject_metrics(4.0, 2_000.0).await; // 4ms, 2 Gbps
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(transport.current_tier(), NetworkTier::Metro);

        // Verify parameters updated
        let params = transport.current_parameters();
        assert!(params.stream_receive_window.unwrap() < 16 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_hysteresis_prevention() {
        let mut config = Config::default();
        config.adaptive_optimization = true;
        config.hysteresis_threshold = 0.1; // 10% threshold
        let transport = StoqTransport::new(config);

        // Set initial tier
        transport.inject_metrics(1.0, 5_000.0).await; // Border between LAN/Metro
        let initial_tier = transport.current_tier();

        // Small fluctuations shouldn't cause tier change
        for _ in 0..10 {
            transport.inject_metrics(0.9, 5_100.0).await; // Slight improvement
            transport.inject_metrics(1.1, 4_900.0).await; // Slight degradation
        }

        assert_eq!(transport.current_tier(), initial_tier); // No flapping
    }

    #[tokio::test]
    async fn test_multi_connection_tier_sync() {
        let mut config = Config::default();
        config.adaptive_optimization = true;
        let transport = Arc::new(StoqTransport::new(config));

        // Create multiple connections
        let mut connections = Vec::new();
        for _ in 0..5 {
            let conn = transport.create_connection().await.unwrap();
            connections.push(conn);
        }

        // Update tier on transport
        transport.set_tier(NetworkTier::Metro).await;

        // Verify all connections updated
        for conn in &connections {
            assert_eq!(conn.tier(), NetworkTier::Metro);
            let params = conn.parameters();
            assert_eq!(params.initial_rtt, Some(Duration::from_millis(5)));
        }
    }
}

mod error_recovery_tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_recovery() {
        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Server that closes connection after first message
        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 1024];
            let _ = stream.read(&mut buf).await.unwrap();
            drop(stream); // Force close

            // Accept reconnection
            let (mut stream2, _) = listener.accept().await.unwrap();
            let n = stream2.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"Reconnected");
            stream2.write_all(b"Recovery successful").await.unwrap();
        });

        let mut client_config = Config::default();
        client_config.auto_reconnect = true;
        let client = StoqTransport::new(client_config);
        let mut stream = client.connect(server_addr).await.unwrap();

        // First write succeeds
        stream.write_all(b"Initial").await.unwrap();

        // Connection drops, auto-reconnect
        tokio::time::sleep(Duration::from_millis(100)).await;
        stream.write_all(b"Reconnected").await.unwrap();

        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"Recovery successful");

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_graceful_degradation() {
        let mut config = Config::default();
        config.enable_tokenization = true;
        config.enable_sharding = true;
        config.fallback_on_error = true;
        let transport = StoqTransport::new(config);

        // Simulate extension failures
        transport.simulate_extension_failure("tokenization").await;
        assert!(!transport.is_tokenization_active());

        transport.simulate_extension_failure("sharding").await;
        assert!(!transport.is_sharding_active());

        // Verify still functional with basic QUIC
        assert!(transport.is_operational());
        assert_eq!(transport.active_extensions().len(), 0);
    }

    #[tokio::test]
    async fn test_packet_loss_recovery() {
        let mut config = Config::default();
        config.max_packet_loss = 0.05; // 5% threshold
        let transport = Arc::new(StoqTransport::new(config));

        // Simulate packet loss
        transport.inject_packet_loss(0.02).await; // 2% loss
        assert!(transport.is_healthy());

        // High packet loss triggers recovery
        transport.inject_packet_loss(0.10).await; // 10% loss
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify recovery actions taken
        assert!(transport.in_recovery_mode());
        let params = transport.current_parameters();
        assert!(params.congestion_window.unwrap() < 65536); // Reduced window
    }
}

mod stress_tests {
    use super::*;
    use tokio::sync::Semaphore;

    #[tokio::test]
    #[ignore] // Run with: cargo test --test phase5_integration_tests stress_tests -- --ignored
    async fn test_connection_storm() {
        const CONNECTIONS_PER_SECOND: usize = 1000;
        const TEST_DURATION_SECS: u64 = 10;

        let server = Arc::new(StoqTransport::new(Config::default()));
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Connection counter
        let connection_count = Arc::new(tokio::sync::RwLock::new(0usize));

        // Server accept loop
        let count_clone = connection_count.clone();
        let server_handle = tokio::spawn(async move {
            loop {
                match timeout(Duration::from_millis(100), listener.accept()).await {
                    Ok(Ok((stream, _))) => {
                        let count = count_clone.clone();
                        tokio::spawn(async move {
                            *count.write().await += 1;
                            let mut buf = vec![0u8; 16];
                            let _ = stream.read(&mut buf).await;
                        });
                    }
                    _ => continue,
                }
            }
        });

        // Generate connection storm
        let start = tokio::time::Instant::now();
        let semaphore = Arc::new(Semaphore::new(CONNECTIONS_PER_SECOND));

        while start.elapsed() < Duration::from_secs(TEST_DURATION_SECS) {
            let permits = semaphore.clone().acquire_many(CONNECTIONS_PER_SECOND as u32).await.unwrap();

            for _ in 0..CONNECTIONS_PER_SECOND {
                tokio::spawn(async move {
                    let client = StoqTransport::new(Config::default());
                    if let Ok(mut stream) = client.connect(server_addr).await {
                        let _ = stream.write_all(b"storm").await;
                    }
                });
            }

            drop(permits);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // Check results
        tokio::time::sleep(Duration::from_secs(1)).await;
        let total_connections = *connection_count.read().await;
        let expected = CONNECTIONS_PER_SECOND * TEST_DURATION_SECS as usize;

        assert!(total_connections > expected * 95 / 100); // >95% success rate
        server_handle.abort();
    }

    #[tokio::test]
    #[ignore]
    async fn test_sustained_high_throughput() {
        const DATA_SIZE: usize = 1024 * 1024 * 1024; // 1GB
        const TARGET_THROUGHPUT: u64 = 1_000_000_000; // 1 Gbps

        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut total_received = 0usize;
            let mut buf = vec![0u8; 65536]; // 64KB buffer

            let start = tokio::time::Instant::now();
            while total_received < DATA_SIZE {
                let n = stream.read(&mut buf).await.unwrap();
                total_received += n;
            }
            let duration = start.elapsed();

            let throughput_bps = (total_received as f64 * 8.0) / duration.as_secs_f64();
            assert!(throughput_bps > TARGET_THROUGHPUT as f64 * 0.9); // >90% of target
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        // Send 1GB of data
        let data = vec![0xFF; 65536]; // 64KB chunks
        let chunks = DATA_SIZE / 65536;

        for _ in 0..chunks {
            stream.write_all(&data).await.unwrap();
        }

        server_handle.await.unwrap();
    }
}

// Run all integration tests with: cargo test --test phase5_integration_tests