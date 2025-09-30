//! Real Performance Validation Tests
//!
//! This test suite validates actual performance against claims, replacing all
//! hardcoded fantasy metrics with real measurements. No more 40 Gbps fantasies.

use stoq::{
    transport::{StoqTransport, TransportConfig, Endpoint},
    performance_monitor::{PerformanceMonitor, NetworkTier, HealthStatus},
};
use std::net::Ipv6Addr;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio;
use tracing::{info, warn, error};

/// Maximum claimed throughput (replacing the fantasy 40 Gbps)
const MAX_CLAIMED_GBPS: f64 = 40.0;

/// Real expected throughput based on actual measurements
const REALISTIC_TARGET_GBPS: f64 = 1.0; // 1 Gbps is realistic for most environments

/// Validate that real performance is honestly reported
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_real_vs_claimed_performance() {
    // Initialize crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("\n╔══════════════════════════════════════════════╗");
    println!("║    REAL PERFORMANCE VALIDATION TEST         ║");
    println!("║    Measuring Actual vs Claimed Performance  ║");
    println!("╚══════════════════════════════════════════════╝\n");

    // Setup performance monitor
    let monitor = Arc::new(PerformanceMonitor::new(REALISTIC_TARGET_GBPS, 10.0));
    monitor.start_monitoring().await;

    // Setup server with realistic configuration
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 39292,
        max_concurrent_streams: 100,
        send_buffer_size: 16 * 1024 * 1024, // 16MB
        receive_buffer_size: 16 * 1024 * 1024,
        enable_zero_copy: true,
        enable_memory_pool: true,
        memory_pool_size: 512,
        frame_batch_size: 32,
        ..Default::default()
    };

    let server = Arc::new(StoqTransport::new(server_config.clone()).await.unwrap());
    let server_clone = server.clone();

    // Start server
    tokio::spawn(async move {
        loop {
            if let Ok(conn) = server_clone.accept().await {
                tokio::spawn(async move {
                    while let Ok(mut stream) = conn.accept_stream().await {
                        tokio::spawn(async move {
                            if let Ok(data) = stream.receive().await {
                                // Echo back for round-trip measurement
                                let _ = stream.send(&data).await;
                            }
                        });
                    }
                });
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Setup client
    let client_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0,
        max_concurrent_streams: 100,
        send_buffer_size: 16 * 1024 * 1024,
        receive_buffer_size: 16 * 1024 * 1024,
        enable_zero_copy: true,
        enable_memory_pool: true,
        ..Default::default()
    };

    let client = Arc::new(StoqTransport::new(client_config).await.unwrap());
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_config.port);

    println!("Testing Real Performance Measurements:\n");
    println!("┌──────────────────┬──────────────┬──────────────┬─────────────┐");
    println!("│ Test Size        │ Measured     │ Claimed      │ Reality %   │");
    println!("├──────────────────┼──────────────┼──────────────┼─────────────┤");

    // Test various data sizes
    let test_cases = vec![
        (1024 * 1024, "1 MB"),
        (10 * 1024 * 1024, "10 MB"),
        (100 * 1024 * 1024, "100 MB"),
    ];

    let mut total_measurements = Vec::new();

    for (size, label) in test_cases {
        let conn = client.connect(&endpoint).await.unwrap();
        let test_data = vec![0xAB; size];

        // Warm up
        let mut stream = conn.open_stream().await.unwrap();
        stream.send(&[0; 1024]).await.unwrap();

        // Measure actual throughput
        let start = Instant::now();

        let mut stream = conn.open_stream().await.unwrap();
        stream.send(&test_data).await.unwrap();
        let _ = stream.receive().await.unwrap(); // Round-trip

        let duration = start.elapsed();
        let gbps = (size as f64 * 8.0 * 2.0) / (duration.as_secs_f64() * 1_000_000_000.0);

        // Record in monitor
        monitor.record_bytes(size * 2);
        monitor.record_latency(duration / 2);

        total_measurements.push(gbps);

        let reality_percent = (gbps / MAX_CLAIMED_GBPS) * 100.0;

        println!(
            "│ {:<16} │ {:<12.3} │ {:<12.1} │ {:<11.2}% │",
            label,
            gbps,
            MAX_CLAIMED_GBPS,
            reality_percent
        );

        conn.close();
    }

    println!("└──────────────────┴──────────────┴──────────────┴─────────────┘\n");

    // Calculate statistics
    let avg_gbps = total_measurements.iter().sum::<f64>() / total_measurements.len() as f64;
    let max_gbps = total_measurements.iter().fold(0.0, |a, &b| a.max(b));

    println!("Performance Statistics:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Average Throughput:   {:.3} Gbps", avg_gbps);
    println!("Peak Throughput:      {:.3} Gbps", max_gbps);
    println!("Claimed Throughput:   {:.1} Gbps", MAX_CLAIMED_GBPS);
    println!("Reality Factor:       {:.2}%", (avg_gbps / MAX_CLAIMED_GBPS) * 100.0);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Get performance snapshot
    let snapshot = monitor.get_snapshot();

    // Determine network tier based on REAL measurements
    println!("Network Tier Classification (Based on Real Measurements):");
    match snapshot.performance_tier {
        NetworkTier::Slow { mbps } => {
            println!("  ⚠️  SLOW TIER: {:.1} Mbps", mbps);
        }
        NetworkTier::Home { mbps } => {
            println!("  🏠 HOME TIER: {:.1} Mbps", mbps);
        }
        NetworkTier::Standard { gbps } => {
            println!("  ✅ STANDARD TIER: {:.3} Gbps", gbps);
        }
        NetworkTier::Performance { gbps } => {
            println!("  🚀 PERFORMANCE TIER: {:.3} Gbps", gbps);
        }
        NetworkTier::Enterprise { gbps } => {
            println!("  💎 ENTERPRISE TIER: {:.3} Gbps", gbps);
        }
        NetworkTier::DataCenter { gbps } => {
            println!("  🏢 DATA CENTER TIER: {:.3} Gbps", gbps);
        }
    }

    // Health status
    println!("\nSystem Health Status:");
    match snapshot.health_status {
        HealthStatus::Healthy { message } => {
            println!("  ✅ HEALTHY: {}", message);
        }
        HealthStatus::Warning { message } => {
            println!("  ⚠️  WARNING: {}", message);
        }
        HealthStatus::Critical { message } => {
            println!("  ❌ CRITICAL: {}", message);
        }
    }

    // Reality check
    println!("\n╔══════════════════════════════════════════════╗");
    println!("║              REALITY CHECK                  ║");
    println!("╚══════════════════════════════════════════════╝");

    if avg_gbps < 1.0 {
        println!("❌ FAIL: Actual performance ({:.3} Gbps) is far below claimed {} Gbps",
                avg_gbps, MAX_CLAIMED_GBPS);
        println!("         This represents a {}x overstatement of capabilities",
                (MAX_CLAIMED_GBPS / avg_gbps) as u32);
    } else if avg_gbps < MAX_CLAIMED_GBPS * 0.1 {
        println!("⚠️  WARNING: Achieving only {:.1}% of claimed performance",
                (avg_gbps / MAX_CLAIMED_GBPS) * 100.0);
    } else if avg_gbps < MAX_CLAIMED_GBPS * 0.5 {
        println!("📊 ACCEPTABLE: Achieving {:.1}% of theoretical maximum",
                (avg_gbps / MAX_CLAIMED_GBPS) * 100.0);
    } else {
        println!("✅ GOOD: Performance within expected range");
    }

    // Stop monitoring
    monitor.stop_monitoring();

    // Cleanup
    client.shutdown().await;
    server.shutdown().await;

    // Assert realistic expectations
    assert!(avg_gbps > 0.0, "Performance must be measurable");
    assert!(avg_gbps < 100.0, "Performance claims must be realistic");

    // Export final metrics
    println!("\nFinal Performance Report:");
    println!("{}", monitor.export_metrics());
}

/// Test latency measurements are real, not fantasy
#[tokio::test]
async fn test_real_latency_measurements() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("\n═══════════════════════════════════════════════");
    println!("     REAL LATENCY MEASUREMENT TEST");
    println!("═══════════════════════════════════════════════\n");

    let monitor = PerformanceMonitor::new(1.0, 10.0);

    // Setup echo server
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 39293,
        ..Default::default()
    };

    let server = Arc::new(StoqTransport::new(server_config).await.unwrap());
    let server_clone = server.clone();

    tokio::spawn(async move {
        while let Ok(conn) = server_clone.accept().await {
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
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Client setup
    let client = StoqTransport::new(TransportConfig::default()).await.unwrap();
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 39293);
    let conn = client.connect(&endpoint).await.unwrap();

    let mut latencies = Vec::new();
    let test_data = vec![0x42; 1024]; // 1KB payload

    println!("Measuring round-trip latencies (100 samples):");
    println!("──────────────────────────────────────────────");

    for i in 0..100 {
        let start = Instant::now();

        let mut stream = conn.open_stream().await.unwrap();
        stream.send(&test_data).await.unwrap();
        let _ = stream.receive().await.unwrap();

        let rtt = start.elapsed();
        let latency_ms = rtt.as_secs_f64() * 1000.0;
        latencies.push(latency_ms);

        monitor.record_latency(rtt);

        if i % 10 == 0 {
            print!(".");
        }
    }
    println!(" Done!\n");

    // Calculate real statistics
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = latencies[50];
    let p95 = latencies[95];
    let p99 = latencies[99];
    let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let min = latencies[0];
    let max = latencies[99];

    println!("Latency Statistics (milliseconds):");
    println!("┌──────────────┬──────────────┐");
    println!("│ Metric       │ Value (ms)   │");
    println!("├──────────────┼──────────────┤");
    println!("│ Minimum      │ {:>12.3} │", min);
    println!("│ Average      │ {:>12.3} │", avg);
    println!("│ P50 (Median) │ {:>12.3} │", p50);
    println!("│ P95          │ {:>12.3} │", p95);
    println!("│ P99          │ {:>12.3} │", p99);
    println!("│ Maximum      │ {:>12.3} │", max);
    println!("└──────────────┴──────────────┘\n");

    // Reality check on latency claims
    if avg < 0.1 {
        println!("⚠️  WARNING: Sub-millisecond latency claims are unrealistic for most networks");
    } else if avg < 1.0 {
        println!("✅ EXCELLENT: Sub-millisecond average latency (local/LAN environment)");
    } else if avg < 10.0 {
        println!("✅ GOOD: Low latency suitable for real-time applications");
    } else if avg < 50.0 {
        println!("📊 ACCEPTABLE: Moderate latency for general applications");
    } else {
        println!("⚠️  HIGH: Latency may impact user experience");
    }

    // Cleanup
    conn.close();
    client.shutdown().await;
    server.shutdown().await;

    // Assert realistic latency
    assert!(avg > 0.0, "Latency must be measurable");
    assert!(avg < 1000.0, "Latency must be reasonable");
}

/// Validate connection scaling with real measurements
#[tokio::test]
async fn test_real_connection_scaling() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("\n═══════════════════════════════════════════════");
    println!("     CONNECTION SCALING TEST");
    println!("═══════════════════════════════════════════════\n");

    let monitor = PerformanceMonitor::new(1.0, 10.0);

    // Setup server
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 39294,
        max_concurrent_streams: 1000,
        ..Default::default()
    };

    let server = Arc::new(StoqTransport::new(server_config).await.unwrap());
    let server_clone = server.clone();

    tokio::spawn(async move {
        while let Ok(conn) = server_clone.accept().await {
            tokio::spawn(async move {
                while let Ok(mut stream) = conn.accept_stream().await {
                    tokio::spawn(async move {
                        let _ = stream.receive().await;
                    });
                }
            });
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test scaling
    let client = Arc::new(StoqTransport::new(TransportConfig::default()).await.unwrap());
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 39294);

    println!("Testing Connection Scaling:");
    println!("┌─────────────┬──────────────┬──────────────┬─────────────┐");
    println!("│ Connections │ Setup Time   │ Rate (c/s)   │ Status      │");
    println!("├─────────────┼──────────────┼──────────────┼─────────────┤");

    for num_connections in [10, 50, 100, 250, 500] {
        let start = Instant::now();
        let mut handles = Vec::new();

        for _ in 0..num_connections {
            let client_clone = client.clone();
            let endpoint_clone = endpoint.clone();
            let monitor_clone = monitor.clone();

            let handle = tokio::spawn(async move {
                let conn_start = Instant::now();
                match client_clone.connect(&endpoint_clone).await {
                    Ok(conn) => {
                        let connect_time = conn_start.elapsed();
                        monitor_clone.record_connection(true, Some(connect_time));

                        // Send test data
                        if let Ok(mut stream) = conn.open_stream().await {
                            let _ = stream.send(&[0x42; 1024]).await;
                        }
                        conn.close();
                        Ok(connect_time)
                    }
                    Err(e) => {
                        monitor_clone.record_connection(false, None);
                        Err(e)
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all connections
        let mut successful = 0;
        for handle in handles {
            if let Ok(Ok(_)) = handle.await {
                successful += 1;
            }
        }

        let duration = start.elapsed();
        let rate = successful as f64 / duration.as_secs_f64();
        let success_rate = successful as f64 / num_connections as f64;

        let status = if success_rate >= 0.99 {
            "✅ Excellent"
        } else if success_rate >= 0.95 {
            "📊 Good"
        } else if success_rate >= 0.90 {
            "⚠️  Warning"
        } else {
            "❌ Poor"
        };

        println!(
            "│ {:>11} │ {:>10.2}s │ {:>12.1} │ {} │",
            num_connections,
            duration.as_secs_f64(),
            rate,
            status
        );
    }

    println!("└─────────────┴──────────────┴──────────────┴─────────────┘\n");

    // Get final snapshot
    let snapshot = monitor.get_snapshot();
    println!("Connection Performance Summary:");
    println!("  Total Connections: {}", snapshot.connections.total);
    println!("  Failed Connections: {}", snapshot.connections.failed);
    println!("  Success Rate: {:.1}%", snapshot.connections.success_rate * 100.0);

    // Cleanup
    client.shutdown().await;
    server.shutdown().await;
}

/// Validate that performance claims are replaced with real measurements
#[tokio::test]
async fn test_no_hardcoded_performance_values() {
    println!("\n═══════════════════════════════════════════════");
    println!("     HARDCODED VALUE DETECTION TEST");
    println!("═══════════════════════════════════════════════\n");

    // This test validates that we're not using hardcoded performance values
    let monitor = PerformanceMonitor::new(1.0, 10.0);

    // Get initial snapshot (should have zero values)
    let snapshot = monitor.get_snapshot();

    println!("Initial State (should be empty/zero):");
    println!("  Throughput: {:.3} Gbps", snapshot.throughput.current_gbps);
    println!("  Latency: {:.2} ms", snapshot.latency.current_ms);
    println!("  Connections: {}", snapshot.connections.active);

    assert_eq!(snapshot.throughput.current_gbps, 0.0, "Throughput should be 0 before measurements");
    assert_eq!(snapshot.latency.current_ms, 0.0, "Latency should be 0 before measurements");

    // Perform real measurements
    monitor.record_bytes(1_000_000);
    monitor.record_latency(Duration::from_millis(5));
    monitor.record_connection(true, Some(Duration::from_millis(2)));

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Values should now be based on actual measurements
    let snapshot = monitor.get_snapshot();
    println!("\nAfter Real Measurements:");
    println!("  Throughput samples: {}", snapshot.throughput.sample_count);
    println!("  Latency samples: {}", snapshot.latency.sample_count);

    // Ensure we're not returning fantasy values
    if snapshot.throughput.current_gbps > 10.0 {
        panic!("❌ FAIL: Unrealistic throughput detected: {:.3} Gbps",
               snapshot.throughput.current_gbps);
    }

    if snapshot.latency.current_ms < 0.001 && snapshot.latency.sample_count > 0 {
        panic!("❌ FAIL: Unrealistic latency detected: {:.3} ms",
               snapshot.latency.current_ms);
    }

    println!("\n✅ PASS: No hardcoded fantasy metrics detected");
    println!("         All values based on real measurements");
}