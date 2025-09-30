//! Real Performance Benchmark for STOQ Transport
//!
//! Run with: cargo run --example benchmark_real --release

use stoq::transport::{StoqTransport, TransportConfig, Endpoint};
use std::net::Ipv6Addr;
use std::time::{Duration, Instant};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed, ignore error
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("         STOQ REAL PERFORMANCE BENCHMARK");
    println!("═══════════════════════════════════════════════════════════\n");

    // Setup server
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 19292,
        max_concurrent_streams: 1000,
        send_buffer_size: 128 * 1024 * 1024, // 128MB
        receive_buffer_size: 128 * 1024 * 1024, // 128MB
        enable_zero_copy: true,
        enable_memory_pool: true,
        memory_pool_size: 4096,
        frame_batch_size: 256,
        connection_pool_size: 20,
        enable_large_send_offload: true,
        ..Default::default()
    };

    let server = StoqTransport::new(server_config).await?;
    let server_clone = server.clone();

    // Start echo server
    tokio::spawn(async move {
        loop {
            if let Ok(conn) = server_clone.accept().await {
                let server_clone2 = server_clone.clone();
                tokio::spawn(async move {
                    while let Ok(mut stream) = conn.accept_stream().await {
                        tokio::spawn(async move {
                            // Echo server - receive and echo back
                            if let Ok(data) = stream.receive().await {
                                let _ = stream.send(&data).await;
                            }
                        });
                    }
                });
            }
        }
    });

    // Small delay for server startup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Setup client with optimized settings
    let client_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0, // Dynamic port
        max_concurrent_streams: 1000,
        send_buffer_size: 128 * 1024 * 1024,
        receive_buffer_size: 128 * 1024 * 1024,
        enable_zero_copy: true,
        enable_memory_pool: true,
        memory_pool_size: 4096,
        frame_batch_size: 256,
        connection_pool_size: 20,
        enable_large_send_offload: true,
        ..Default::default()
    };

    let client = StoqTransport::new(client_config).await?;
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 19292);

    println!("Test Configuration:");
    println!("  • Zero-copy: Enabled");
    println!("  • Memory pool: 4096 buffers");
    println!("  • Frame batching: 256 frames");
    println!("  • Buffer size: 128MB");
    println!("  • Connection pool: 20 connections");
    println!();

    // Warmup
    println!("Warming up connection...");
    let conn = client.connect(&endpoint).await?;
    let mut stream = conn.open_stream().await?;
    stream.send(&vec![0; 1024]).await?;
    conn.close();

    println!("\n┌─────────────────────────────────────────────────────────┐");
    println!("│              THROUGHPUT BENCHMARKS                         │");
    println!("├─────────────────────────────────────────────────────────┤");

    // Test different data sizes
    let test_sizes = vec![
        (1024, "1 KB"),
        (64 * 1024, "64 KB"),
        (1024 * 1024, "1 MB"),
        (10 * 1024 * 1024, "10 MB"),
        (100 * 1024 * 1024, "100 MB"),
        (500 * 1024 * 1024, "500 MB"),
    ];

    println!("│ {:^12} │ {:^10} │ {:^12} │ {:^12} │", "Size", "Time", "Throughput", "Speed");
    println!("├──────────────┼────────────┼──────────────┼──────────────┤");

    let mut peak_gbps = 0.0f64;

    for (size, label) in &test_sizes {
        // Fresh connection for each test
        let conn = client.connect(&endpoint).await?;

        // Generate test data
        let test_data = vec![0xAB; *size];

        // Measure send time
        let start = Instant::now();

        let mut stream = conn.open_stream().await?;
        stream.send(&test_data).await?;

        let duration = start.elapsed();

        // Calculate throughput
        let bytes_per_sec = *size as f64 / duration.as_secs_f64();
        let mbps = bytes_per_sec * 8.0 / 1_000_000.0;
        let gbps = mbps / 1000.0;

        if gbps > peak_gbps {
            peak_gbps = gbps;
        }

        println!(
            "│ {:^12} │ {:>8.2}ms │ {:>9.1} Mbps │ {:>9.3} Gbps │",
            label,
            duration.as_millis(),
            mbps,
            gbps
        );

        conn.close();
    }

    println!("└─────────────────────────────────────────────────────────┘\n");

    // Connection pooling test
    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│            CONNECTION POOLING TEST                         │");
    println!("├─────────────────────────────────────────────────────────┤");

    // Test connection reuse
    let mut cold_times = Vec::new();
    let mut warm_times = Vec::new();

    for i in 0..10 {
        let start = Instant::now();
        let conn = client.connect(&endpoint).await?;
        let cold_time = start.elapsed();
        cold_times.push(cold_time.as_micros());

        // Return to pool
        client.return_to_pool(conn.clone());

        // Get from pool (should be fast)
        let start = Instant::now();
        let conn2 = client.connect(&endpoint).await?;
        let warm_time = start.elapsed();
        warm_times.push(warm_time.as_micros());

        conn2.close();
    }

    let avg_cold = cold_times.iter().sum::<u128>() / cold_times.len() as u128;
    let avg_warm = warm_times.iter().sum::<u128>() / warm_times.len() as u128;

    println!("│ Cold connection (avg): {:>8} μs                        │", avg_cold);
    println!("│ Pooled connection (avg): {:>6} μs                        │", avg_warm);
    println!("│ Speedup: {:>6.1}x                                          │", avg_cold as f64 / avg_warm as f64);
    println!("└─────────────────────────────────────────────────────────┘\n");

    // Concurrent streams test
    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│           CONCURRENT STREAMS TEST                          │");
    println!("├─────────────────────────────────────────────────────────┤");

    let conn = client.connect(&endpoint).await?;
    let stream_counts = vec![10, 50, 100, 500];
    let stream_data_size = 1024 * 1024; // 1MB per stream

    for count in stream_counts {
        let start = Instant::now();

        let mut handles = Vec::new();
        for _ in 0..count {
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
            handle.await?;
        }

        let duration = start.elapsed();
        let total_bytes = count * stream_data_size;
        let gbps = (total_bytes as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000_000.0);

        println!(
            "│ {:>3} streams: {:>6.2}ms, {:>8.3} Gbps                    │",
            count,
            duration.as_millis(),
            gbps
        );

        if gbps > peak_gbps {
            peak_gbps = gbps;
        }
    }

    println!("└─────────────────────────────────────────────────────────┘\n");

    // Multiplexing test
    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│          CONNECTION MULTIPLEXING TEST                      │");
    println!("├─────────────────────────────────────────────────────────┤");

    // Enable multiplexing with multiple connections
    client.enable_multiplexing(&endpoint, 10).await?;

    let test_data = vec![0xDD; 100 * 1024 * 1024]; // 100MB

    let start = Instant::now();

    // Send using multiplexed connections
    for _ in 0..10 {
        client.send_multiplexed(&endpoint, &test_data[0..10*1024*1024]).await?;
    }

    let duration = start.elapsed();
    let total_bytes = 10 * 10 * 1024 * 1024; // 100MB total
    let gbps = (total_bytes as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000_000.0);

    println!("│ 10x multiplexed: {:>8.2}ms, {:>8.3} Gbps              │", duration.as_millis(), gbps);

    if gbps > peak_gbps {
        peak_gbps = gbps;
    }

    println!("└─────────────────────────────────────────────────────────┘\n");

    // Performance Statistics
    let (stats_peak, zero_copy, pool_hits, batches) = client.performance_stats();

    println!("═══════════════════════════════════════════════════════════");
    println!("                 PERFORMANCE SUMMARY");
    println!("═══════════════════════════════════════════════════════════");
    println!();
    println!("  Peak Throughput:     {:.3} Gbps", peak_gbps);
    println!("  Recorded Peak:       {:.3} Gbps", stats_peak);
    println!("  Zero-copy ops:       {}", zero_copy);
    println!("  Memory pool hits:    {}", pool_hits);
    println!("  Frame batches:       {}", batches);
    println!();

    // Determine network tier
    let tier = if peak_gbps >= 25.0 {
        "Enterprise (25+ Gbps)"
    } else if peak_gbps >= 10.0 {
        "Performance (10+ Gbps)"
    } else if peak_gbps >= 1.0 {
        "Standard (1+ Gbps)"
    } else if peak_gbps >= 0.1 {
        "Home (100+ Mbps)"
    } else {
        "Limited (<100 Mbps)"
    };

    println!("  Detected Network Tier: {}", tier);
    println!();

    if peak_gbps < 10.0 {
        println!("⚠️  WARNING: Performance below 10 Gbps target!");
        println!("  Current: {:.3} Gbps", peak_gbps);
        println!("  Target:  10+ Gbps");
        println!();
        println!("  Possible bottlenecks:");
        println!("    • CPU saturation");
        println!("    • Network interface limits");
        println!("    • Kernel buffer constraints");
        println!("    • Loopback interface overhead");
    } else {
        println!("✅ Performance target achieved!");
    }

    println!("\n═══════════════════════════════════════════════════════════\n");

    // Cleanup
    client.shutdown().await;
    server.shutdown().await;

    Ok(())
}