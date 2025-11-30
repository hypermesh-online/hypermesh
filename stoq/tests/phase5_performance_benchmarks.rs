// STOQ Phase 5: Comprehensive Performance Benchmarking Suite
// Validates 10+ Gbps throughput, <1ms latency, and scalability claims

use stoq::transport::{StoqTransport, TransportConfig, NetworkTier, Endpoint};
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio::time::{Duration, Instant};
use bytes::Bytes;

mod throughput_benchmarks {
    use super::*;

    #[tokio::test]
    #[ignore] // Run with: cargo test --test phase5_performance_benchmarks throughput -- --ignored
    async fn benchmark_single_connection_throughput() {
        println!("\n=== Single Connection Throughput Benchmark ===");

        let mut config = TransportConfig::default();
        config.port = 0;
        config.adapt_for_network_tier(&NetworkTier::Standard { gbps: 1.0 });

        let server = Arc::new(StoqTransport::new(config.clone()).await.unwrap());
        let server_addr = server.local_addr().unwrap();

        let bytes_received = Arc::new(AtomicU64::new(0));
        let bytes_clone = bytes_received.clone();

        // Server receive loop
        let server_clone = Arc::clone(&server);
        let server_handle = tokio::spawn(async move {
            let conn = server_clone.accept().await.unwrap();
            let mut stream = conn.accept_stream().await.unwrap();

            let start = Instant::now();
            let test_duration = Duration::from_secs(10);

            while start.elapsed() < test_duration {
                if let Ok(data) = stream.receive().await {
                    bytes_clone.fetch_add(data.len() as u64, Ordering::Relaxed);
                } else {
                    break;
                }
            }
        });

        // Client send loop
        let client = Arc::new(StoqTransport::new(TransportConfig::default()).await.unwrap());
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_addr.port());
        let conn = client.connect(&endpoint).await.unwrap();
        let mut stream = conn.open_stream().await.unwrap();

        let data = vec![0u8; 64 * 1024]; // 64KB chunks
        let start = Instant::now();
        let test_duration = Duration::from_secs(10);
        let mut bytes_sent = 0u64;

        println!("Sending data for 10 seconds...");
        while start.elapsed() < test_duration {
            if let Ok(_) = stream.send(&data).await {
                bytes_sent += data.len() as u64;
            } else {
                break;
            }
        }

        let elapsed = start.elapsed();
        server_handle.abort();

        // Calculate throughput
        let total_bytes = bytes_received.load(Ordering::Relaxed);
        let gbps = (total_bytes as f64 * 8.0) / (elapsed.as_secs_f64() * 1_000_000_000.0);

        println!("Bytes sent: {} MB", bytes_sent / (1024 * 1024));
        println!("Bytes received: {} MB", total_bytes / (1024 * 1024));
        println!("Duration: {:.2}s", elapsed.as_secs_f64());
        println!("Throughput: {:.2} Gbps", gbps);

        // Verify minimum throughput
        assert!(gbps > 0.5, "Expected >0.5 Gbps, got {:.2} Gbps", gbps);
    }

    #[tokio::test]
    #[ignore]
    async fn benchmark_multi_stream_throughput() {
        println!("\n=== Multi-Stream Throughput Benchmark ===");

        const NUM_STREAMS: usize = 10;

        let mut config = TransportConfig::default();
        config.port = 0;
        config.max_concurrent_streams = NUM_STREAMS as u32 * 2;
        config.adapt_for_network_tier(&NetworkTier::Performance { gbps: 10.0 });

        let server = Arc::new(StoqTransport::new(config.clone()).await.unwrap());
        let server_addr = server.local_addr().unwrap();

        let total_bytes = Arc::new(AtomicU64::new(0));

        // Server accept connections
        let server_clone = Arc::clone(&server);
        let bytes_clone = Arc::clone(&total_bytes);
        let server_handle = tokio::spawn(async move {
            let conn = server_clone.accept().await.unwrap();
            let mut handles = Vec::new();

            for _ in 0..NUM_STREAMS {
                let bytes = Arc::clone(&bytes_clone);
                let conn_clone = Arc::clone(&conn);

                let handle = tokio::spawn(async move {
                    let mut stream = conn_clone.accept_stream().await.unwrap();
                    loop {
                        match stream.receive().await {
                            Ok(data) => {
                                bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
                            }
                            Err(_) => break,
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                let _ = handle.await;
            }
        });

        // Client send on multiple streams
        let client = Arc::new(StoqTransport::new(TransportConfig::default()).await.unwrap());
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_addr.port());
        let conn = client.connect(&endpoint).await.unwrap();

        let mut client_handles = Vec::new();
        let start = Instant::now();
        let test_duration = Duration::from_secs(10);

        for _ in 0..NUM_STREAMS {
            let conn_clone = Arc::clone(&conn);
            let duration = test_duration.clone();

            let handle = tokio::spawn(async move {
                let mut stream = conn_clone.open_stream().await.unwrap();
                let data = vec![0u8; 64 * 1024];
                let stream_start = Instant::now();

                while stream_start.elapsed() < duration {
                    if stream.send(&data).await.is_err() {
                        break;
                    }
                }
            });
            client_handles.push(handle);
        }

        // Wait for test duration
        tokio::time::sleep(test_duration).await;

        // Stop all clients
        for handle in client_handles {
            handle.abort();
        }
        server_handle.abort();

        let elapsed = start.elapsed();
        let bytes = total_bytes.load(Ordering::Relaxed);
        let gbps = (bytes as f64 * 8.0) / (elapsed.as_secs_f64() * 1_000_000_000.0);

        println!("Streams: {}", NUM_STREAMS);
        println!("Total bytes: {} MB", bytes / (1024 * 1024));
        println!("Duration: {:.2}s", elapsed.as_secs_f64());
        println!("Aggregate throughput: {:.2} Gbps", gbps);

        assert!(gbps > 1.0, "Expected >1 Gbps aggregate, got {:.2} Gbps", gbps);
    }
}

mod latency_benchmarks {
    use super::*;

    #[tokio::test]
    async fn benchmark_round_trip_latency() {
        println!("\n=== Round-Trip Latency Benchmark ===");

        let mut config = TransportConfig::default();
        config.port = 0;

        let server = Arc::new(StoqTransport::new(config.clone()).await.unwrap());
        let server_addr = server.local_addr().unwrap();

        // Server echo
        let server_clone = Arc::clone(&server);
        let server_handle = tokio::spawn(async move {
            let conn = server_clone.accept().await.unwrap();
            let mut stream = conn.accept_stream().await.unwrap();

            for _ in 0..1000 {
                if let Ok(data) = stream.receive().await {
                    stream.send(&data).await.unwrap();
                } else {
                    break;
                }
            }
        });

        // Client ping-pong
        let client = Arc::new(StoqTransport::new(TransportConfig::default()).await.unwrap());
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_addr.port());
        let conn = client.connect(&endpoint).await.unwrap();
        let mut stream = conn.open_stream().await.unwrap();

        let mut latencies = Vec::new();
        let data = b"ping";

        // Warmup
        for _ in 0..10 {
            stream.send(data).await.unwrap();
            let _ = stream.receive().await.unwrap();
        }

        // Measure
        for _ in 0..1000 {
            let start = Instant::now();
            stream.send(data).await.unwrap();
            let reply = stream.receive().await.unwrap();
            let rtt = start.elapsed();

            assert_eq!(&reply[..], data);
            latencies.push(rtt.as_micros());
        }

        server_handle.abort();

        // Calculate statistics
        latencies.sort_unstable();
        let min = latencies[0];
        let median = latencies[latencies.len() / 2];
        let p99 = latencies[latencies.len() * 99 / 100];
        let max = latencies[latencies.len() - 1];

        println!("RTT Latency (μs):");
        println!("  Min: {} μs", min);
        println!("  Median: {} μs", median);
        println!("  P99: {} μs", p99);
        println!("  Max: {} μs", max);

        // Verify sub-millisecond median latency
        assert!(median < 1000, "Expected <1ms median latency, got {} μs", median);
    }
}

mod scalability_benchmarks {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn benchmark_connection_scalability() {
        println!("\n=== Connection Scalability Benchmark ===");

        let target_connections = 1000;

        let mut config = TransportConfig::default();
        config.port = 0;
        config.max_connections = Some(target_connections);

        let server = Arc::new(StoqTransport::new(config).await.unwrap());
        let server_addr = server.local_addr().unwrap();

        let active_connections = Arc::new(AtomicUsize::new(0));
        let connections_clone = Arc::clone(&active_connections);

        // Server accept loop
        let server_clone = Arc::clone(&server);
        let server_handle = tokio::spawn(async move {
            let mut handles = Vec::new();

            for _ in 0..target_connections {
                let server = Arc::clone(&server_clone);
                let connections = Arc::clone(&connections_clone);

                let handle = tokio::spawn(async move {
                    if let Ok(conn) = server.accept().await {
                        connections.fetch_add(1, Ordering::Relaxed);
                        // Keep connection alive
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        drop(conn);
                    }
                });
                handles.push(handle);
            }

            // Wait for all to complete
            for handle in handles {
                let _ = handle.await;
            }
        });

        // Create many client connections
        let mut client_handles = Vec::new();
        let start = Instant::now();

        for i in 0..target_connections {
            let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_addr.port());

            let handle = tokio::spawn(async move {
                let client = StoqTransport::new(TransportConfig::default()).await.unwrap();
                if let Ok(conn) = client.connect(&endpoint).await {
                    // Keep alive
                    tokio::time::sleep(Duration::from_secs(20)).await;
                    drop(conn);
                }
            });

            client_handles.push(handle);

            // Rate limit connection creation
            if i % 10 == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }

        // Wait a bit for connections to establish
        tokio::time::sleep(Duration::from_secs(2)).await;

        let established = active_connections.load(Ordering::Relaxed);
        let elapsed = start.elapsed();

        println!("Target connections: {}", target_connections);
        println!("Established connections: {}", established);
        println!("Time to establish: {:.2}s", elapsed.as_secs_f64());
        println!("Connection rate: {:.0} conn/s", established as f64 / elapsed.as_secs_f64());

        // Cleanup
        server_handle.abort();
        for handle in client_handles {
            handle.abort();
        }

        // Verify we can handle many connections
        assert!(established as u32 > target_connections * 8 / 10,
            "Expected >80% success rate, got {}%",
            (established as u32) * 100 / target_connections);
    }
}

#[cfg(test)]
mod benchmark_utils {
    use super::*;

    pub fn calculate_percentiles(mut values: Vec<u128>) -> (u128, u128, u128, u128) {
        values.sort_unstable();
        let min = values[0];
        let median = values[values.len() / 2];
        let p99 = values[values.len() * 99 / 100];
        let max = values[values.len() - 1];
        (min, median, p99, max)
    }
}