//! STOQ Realistic Performance Demonstration
//!
//! This example demonstrates ACTUAL STOQ performance with real network operations.
//! No fantasy metrics - only measurable reality.

use stoq::{transport::{StoqTransport, TransportConfig, Endpoint}, performance_monitor::PerformanceMonitor};
use std::net::Ipv6Addr;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Initialize crypto provider for Rustls
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("🚀 STOQ Realistic Performance Demo");
    println!("====================================");

    // Test 1: Basic loopback throughput
    println!("\n📈 Test 1: Basic Loopback Throughput");
    let throughput = measure_loopback_throughput().await?;
    println!("Measured throughput: {:.2} Mbps (realistic for loopback)", throughput);

    // Test 2: Latency measurement
    println!("\n⏱️  Test 2: Round-Trip Latency");
    let latency = measure_round_trip_latency().await?;
    println!("Average round-trip latency: {:.2} ms", latency);

    // Test 3: Connection scaling
    println!("\n🔗 Test 3: Connection Scaling");
    let connections_per_sec = measure_connection_rate().await?;
    println!("Connection establishment rate: {:.0} connections/sec", connections_per_sec);

    // Test 4: Adaptive tier detection demo
    println!("\n🎯 Test 4: Adaptive Tier Detection");
    demonstrate_adaptive_tiers().await?;

    println!("\n✅ Performance demonstration complete");
    println!("Note: Results are from localhost loopback - real network performance will vary");

    Ok(())
}

async fn measure_loopback_throughput() -> anyhow::Result<f64> {
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 19292,
        max_concurrent_streams: 100,
        ..Default::default()
    };

    let server = Arc::new(StoqTransport::new(server_config.clone()).await?);
    let server_clone = server.clone();

    // Simple echo server
    tokio::spawn(async move {
        for _ in 0..3 { // Accept up to 3 connections
            if let Ok(conn) = server_clone.accept().await {
                tokio::spawn(async move {
                    while let Ok(mut stream) = conn.accept_stream().await {
                        tokio::spawn(async move {
                            if let Ok(data) = stream.receive().await {
                                let _ = stream.send(&data).await;
                            }
                        });
                    }
                });
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Client
    let client_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0,
        ..Default::default()
    };

    let client = StoqTransport::new(client_config).await?;
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_config.port);
    let conn = client.connect(&endpoint).await?;

    // Transfer 1MB of data
    let test_data = vec![0xAB; 1024 * 1024]; // 1MB
    let start = Instant::now();

    let mut stream = conn.open_stream().await?;
    stream.send(&test_data).await?;
    let _response = stream.receive().await?;

    let duration = start.elapsed();
    let mbps = (test_data.len() as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0);

    // Cleanup
    conn.close();
    client.shutdown().await;
    server.shutdown().await;

    Ok(mbps)
}

async fn measure_round_trip_latency() -> anyhow::Result<f64> {
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 19293,
        ..Default::default()
    };

    let server = Arc::new(StoqTransport::new(server_config.clone()).await?);
    let server_clone = server.clone();

    // Echo server
    tokio::spawn(async move {
        if let Ok(conn) = server_clone.accept().await {
            while let Ok(mut stream) = conn.accept_stream().await {
                tokio::spawn(async move {
                    if let Ok(data) = stream.receive().await {
                        let _ = stream.send(&data).await;
                    }
                });
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0,
        ..Default::default()
    };

    let client = StoqTransport::new(client_config).await?;
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_config.port);
    let conn = client.connect(&endpoint).await?;

    // Measure multiple round trips
    let mut latencies = Vec::new();
    let test_data = vec![0x42; 100]; // 100 bytes

    for _ in 0..10 {
        let start = Instant::now();

        let mut stream = conn.open_stream().await?;
        stream.send(&test_data).await?;
        let _response = stream.receive().await?;

        let latency = start.elapsed().as_micros() as f64 / 1000.0; // Convert to ms
        latencies.push(latency);
    }

    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;

    // Cleanup
    conn.close();
    client.shutdown().await;
    server.shutdown().await;

    Ok(avg_latency)
}

async fn measure_connection_rate() -> anyhow::Result<f64> {
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 19294,
        max_concurrent_streams: 50,
        ..Default::default()
    };

    let server = Arc::new(StoqTransport::new(server_config.clone()).await?);
    let server_clone = server.clone();

    // Connection acceptor
    tokio::spawn(async move {
        for _ in 0..20 { // Accept up to 20 connections
            if let Ok(conn) = server_clone.accept().await {
                tokio::spawn(async move {
                    let _ = conn.accept_stream().await;
                });
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0,
        ..Default::default()
    };

    let client = Arc::new(StoqTransport::new(client_config).await?);
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_config.port);

    // Measure connection establishment rate
    let start = Instant::now();
    let mut handles = Vec::new();

    for _ in 0..10 {
        let client_clone = client.clone();
        let endpoint_clone = endpoint.clone();

        let handle = tokio::spawn(async move {
            let conn = client_clone.connect(&endpoint_clone).await.unwrap();
            let mut stream = conn.open_stream().await.unwrap();
            stream.send(&[0x42; 10]).await.unwrap();
            conn.close();
        });
        handles.push(handle);
    }

    // Wait for all connections
    for handle in handles {
        let _ = handle.await;
    }

    let duration = start.elapsed();
    let connections_per_sec = 10.0 / duration.as_secs_f64();

    // Cleanup
    client.shutdown().await;
    server.shutdown().await;

    Ok(connections_per_sec)
}

async fn demonstrate_adaptive_tiers() -> anyhow::Result<()> {
    let config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0,
        ..Default::default()
    };

    let mut transport = StoqTransport::new(config).await?;
    let monitor = Arc::new(PerformanceMonitor::new(1.0, 10.0)); // 1 Gbps threshold, 10ms latency

    println!("Starting performance monitoring...");
    monitor.start_monitoring().await;

    // Start adaptive optimization
    transport.start_adaptation().await;

    // Simulate some network activity to generate metrics
    monitor.record_throughput(0.5).await; // 500 Mbps
    tokio::time::sleep(Duration::from_secs(2)).await;

    monitor.record_throughput(1.2).await; // 1.2 Gbps
    tokio::time::sleep(Duration::from_secs(2)).await;

    monitor.record_throughput(0.1).await; // 100 Mbps
    tokio::time::sleep(Duration::from_secs(2)).await;

    println!("Adaptive tier detection completed - check logs for tier changes");

    transport.shutdown().await;
    Ok(())
}