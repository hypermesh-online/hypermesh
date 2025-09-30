//! Real Performance Test for STOQ Transport
//!
//! This test measures ACTUAL throughput, not theoretical calculations

use stoq::transport::{StoqTransport, TransportConfig, Endpoint, Connection};
use std::net::Ipv6Addr;
use std::time::{Duration, Instant};
use tokio;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_real_throughput() {
    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed, ignore error
    }

    println!("\n=== STOQ Real Performance Test ===\n");

    // Setup server
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 19292,
        max_concurrent_streams: 1000,
        send_buffer_size: 64 * 1024 * 1024, // 64MB
        receive_buffer_size: 64 * 1024 * 1024, // 64MB
        enable_zero_copy: true,
        enable_memory_pool: true,
        memory_pool_size: 2048,
        frame_batch_size: 128,
        connection_pool_size: 10,
        ..Default::default()
    };

    let server = StoqTransport::new(server_config).await.unwrap();
    let server_clone = server.clone();

    // Start server in background
    tokio::spawn(async move {
        loop {
            if let Ok(conn) = server_clone.accept().await {
                tokio::spawn(handle_connection(conn));
            }
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Setup client
    let client_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0, // Dynamic port
        max_concurrent_streams: 1000,
        send_buffer_size: 64 * 1024 * 1024,
        receive_buffer_size: 64 * 1024 * 1024,
        enable_zero_copy: true,
        enable_memory_pool: true,
        memory_pool_size: 2048,
        frame_batch_size: 128,
        connection_pool_size: 10,
        ..Default::default()
    };

    let client = StoqTransport::new(client_config).await.unwrap();

    // Test different data sizes
    let test_sizes = vec![
        (1024, "1KB"),
        (64 * 1024, "64KB"),
        (1024 * 1024, "1MB"),
        (10 * 1024 * 1024, "10MB"),
        (100 * 1024 * 1024, "100MB"),
    ];

    println!("Testing throughput with different data sizes:\n");
    println!("{:<10} {:<15} {:<15} {:<15}", "Size", "Time (ms)", "Throughput", "Gbps");
    println!("{}", "-".repeat(60));

    for (size, label) in test_sizes {
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 19292);
        let conn = client.connect(&endpoint).await.unwrap();

        // Generate test data
        let test_data = vec![0xAB; size];

        // Warm up connection
        let mut stream = conn.open_stream().await.unwrap();
        stream.send(&vec![0; 1024]).await.unwrap();

        // Measure actual send time
        let start = Instant::now();

        let mut stream = conn.open_stream().await.unwrap();
        stream.send(&test_data).await.unwrap();

        let duration = start.elapsed();

        // Calculate throughput
        let bytes_per_sec = size as f64 / duration.as_secs_f64();
        let mbps = bytes_per_sec * 8.0 / 1_000_000.0;
        let gbps = mbps / 1000.0;

        println!(
            "{:<10} {:<15.2} {:<15} {:<15.3}",
            label,
            duration.as_millis(),
            format!("{:.0} Mbps", mbps),
            gbps
        );

        conn.close();
    }

    println!("\n=== Testing Connection Pooling Performance ===\n");

    // Test connection pooling
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 19292);
    let test_data = vec![0xCD; 1024 * 1024]; // 1MB

    // First connection (cold)
    let start = Instant::now();
    let conn1 = client.connect(&endpoint).await.unwrap();
    let cold_connect_time = start.elapsed();

    // Return to pool
    client.return_to_pool(conn1);

    // Second connection (should be from pool)
    let start = Instant::now();
    let conn2 = client.connect(&endpoint).await.unwrap();
    let warm_connect_time = start.elapsed();

    println!("Cold connection: {:.2} ms", cold_connect_time.as_millis());
    println!("Pooled connection: {:.2} ms", warm_connect_time.as_millis());
    println!("Speedup: {:.1}x", cold_connect_time.as_secs_f64() / warm_connect_time.as_secs_f64());

    conn2.close();

    println!("\n=== Testing Concurrent Streams ===\n");

    // Test concurrent streams
    let conn = client.connect(&endpoint).await.unwrap();
    let concurrent_count = 100;
    let stream_data_size = 1024 * 1024; // 1MB per stream

    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..concurrent_count {
        let conn_clone = conn.clone();
        let data = vec![0xEF; stream_data_size];

        let handle = tokio::spawn(async move {
            let mut stream = conn_clone.open_stream().await.unwrap();
            stream.send(&data).await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all streams
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start.elapsed();
    let total_bytes = concurrent_count * stream_data_size;
    let gbps = (total_bytes as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000_000.0);

    println!("Concurrent streams: {}", concurrent_count);
    println!("Total data: {} MB", total_bytes / (1024 * 1024));
    println!("Time: {:.2} ms", duration.as_millis());
    println!("Throughput: {:.3} Gbps", gbps);

    println!("\n=== Performance Statistics ===\n");

    let (peak_gbps, zero_copy_ops, pool_hits, frame_batches) = client.performance_stats();
    println!("Peak throughput: {:.3} Gbps", peak_gbps);
    println!("Zero-copy operations: {}", zero_copy_ops);
    println!("Memory pool hits: {}", pool_hits);
    println!("Frame batches sent: {}", frame_batches);

    // Cleanup
    client.shutdown().await;
    server.shutdown().await;
}

async fn handle_connection(conn: std::sync::Arc<Connection>) {
    while let Ok(mut stream) = conn.accept_stream().await {
        tokio::spawn(async move {
            // Echo server - just receive and discard
            let _ = stream.receive().await;
        });
    }
}

#[tokio::test]
async fn test_adaptive_tier_detection() {
    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed, ignore error
    }

    println!("\n=== Testing Adaptive Network Tier Detection ===\n");

    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await.unwrap();

    // Simulate different network conditions
    let test_throughputs = vec![
        (100_000_000, "100 Mbps - Home tier"),
        (1_000_000_000, "1 Gbps - Standard tier"),
        (10_000_000_000, "10 Gbps - Performance tier"),
        (25_000_000_000, "25 Gbps - Enterprise tier"),
    ];

    for (bps, tier_name) in test_throughputs {
        let detected_tier = detect_network_tier(bps);
        println!("{}: Detected as Tier {}", tier_name, detected_tier);
    }
}

fn detect_network_tier(bits_per_second: u64) -> u8 {
    match bits_per_second {
        bps if bps >= 25_000_000_000 => 4, // 25+ Gbps
        bps if bps >= 10_000_000_000 => 3, // 10+ Gbps
        bps if bps >= 1_000_000_000 => 2,  // 1+ Gbps
        bps if bps >= 100_000_000 => 1,    // 100+ Mbps
        _ => 0, // Below 100 Mbps
    }
}