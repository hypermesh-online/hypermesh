//! STOQ Throughput Test - Measures real performance
//!
//! Run with: cargo run --example throughput_test --release

use stoq::transport::{StoqTransport, TransportConfig, Endpoint};
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::time;

const TEST_DURATION_SECS: u64 = 10; // 10 second test
const DATA_CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed
    }

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║            STOQ REAL THROUGHPUT TEST                      ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Shared counters
    let bytes_sent = Arc::new(AtomicU64::new(0));
    let bytes_received = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));

    // Setup server with optimized settings
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 29292,
        max_concurrent_streams: 1000,
        send_buffer_size: 256 * 1024 * 1024, // 256MB
        receive_buffer_size: 256 * 1024 * 1024,
        enable_zero_copy: true,
        enable_memory_pool: true,
        memory_pool_size: 8192,
        frame_batch_size: 512,
        max_datagram_size: 65507, // Max UDP
        connection_pool_size: 50,
        enable_large_send_offload: true,
        enable_cpu_affinity: true,
        ..Default::default()
    };

    let server = Arc::new(StoqTransport::new(server_config).await?);
    let server_clone = server.clone();
    let bytes_received_clone = bytes_received.clone();
    let running_clone = running.clone();

    // Start server
    let server_handle = tokio::spawn(async move {
        println!("Server started on [::1]:29292");

        while running_clone.load(Ordering::Relaxed) {
            tokio::select! {
                conn_result = server_clone.accept() => {
                    if let Ok(conn) = conn_result {
                        let bytes_received = bytes_received_clone.clone();
                        let running = running_clone.clone();

                        tokio::spawn(async move {
                            while running.load(Ordering::Relaxed) {
                                if let Ok(mut stream) = conn.accept_stream().await {
                                    if let Ok(data) = stream.receive().await {
                                        bytes_received.fetch_add(data.len() as u64, Ordering::Relaxed);
                                    }
                                } else {
                                    break;
                                }
                            }
                        });
                    }
                }
                _ = time::sleep(Duration::from_millis(10)) => {
                    if !running_clone.load(Ordering::Relaxed) {
                        break;
                    }
                }
            }
        }
        println!("Server stopped");
    });

    // Wait for server to start
    time::sleep(Duration::from_millis(500)).await;

    // Setup client
    let client_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0, // Dynamic
        max_concurrent_streams: 1000,
        send_buffer_size: 256 * 1024 * 1024,
        receive_buffer_size: 256 * 1024 * 1024,
        enable_zero_copy: true,
        enable_memory_pool: true,
        memory_pool_size: 8192,
        frame_batch_size: 512,
        max_datagram_size: 65507,
        connection_pool_size: 50,
        enable_large_send_offload: true,
        enable_cpu_affinity: true,
        ..Default::default()
    };

    let client = Arc::new(StoqTransport::new(client_config).await?);
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 29292);

    println!("Test Configuration:");
    println!("  • Duration: {} seconds", TEST_DURATION_SECS);
    println!("  • Chunk size: {} MB", DATA_CHUNK_SIZE / (1024 * 1024));
    println!("  • Buffer size: 256 MB");
    println!("  • Zero-copy: Enabled");
    println!("  • Memory pool: 8192 buffers");
    println!("  • Frame batching: 512 frames\n");

    // Create connections
    println!("Establishing connections...");
    let mut connections = Vec::new();
    for i in 0..10 {
        let conn = client.connect(&endpoint).await?;
        connections.push(conn);
        println!("  Connection {} established", i + 1);
    }

    // Generate test data
    let test_data = vec![0xAB; DATA_CHUNK_SIZE];

    println!("\nStarting throughput test...\n");

    let test_start = Instant::now();
    let bytes_sent_clone = bytes_sent.clone();
    let running_clone = running.clone();

    // Start multiple sender tasks
    let mut sender_handles = Vec::new();
    for (idx, conn) in connections.into_iter().enumerate() {
        let test_data = test_data.clone();
        let bytes_sent = bytes_sent_clone.clone();
        let running = running_clone.clone();

        let handle = tokio::spawn(async move {
            println!("  Sender {} started", idx + 1);
            let mut send_count = 0u64;

            while running.load(Ordering::Relaxed) {
                if let Ok(mut stream) = conn.open_stream().await {
                    if stream.send(&test_data).await.is_ok() {
                        bytes_sent.fetch_add(test_data.len() as u64, Ordering::Relaxed);
                        send_count += 1;
                    }
                }
            }

            println!("  Sender {} stopped (sent {} chunks)", idx + 1, send_count);
        });
        sender_handles.push(handle);
    }

    // Monitor progress
    let monitor_bytes_sent = bytes_sent.clone();
    let monitor_bytes_received = bytes_received.clone();
    let monitor_running = running.clone();

    let monitor_handle = tokio::spawn(async move {
        let mut last_sent = 0u64;
        let mut last_received = 0u64;
        let start = Instant::now();

        while monitor_running.load(Ordering::Relaxed) {
            time::sleep(Duration::from_secs(1)).await;

            let current_sent = monitor_bytes_sent.load(Ordering::Relaxed);
            let current_received = monitor_bytes_received.load(Ordering::Relaxed);
            let elapsed = start.elapsed().as_secs_f64();

            let send_rate = (current_sent - last_sent) as f64 * 8.0 / 1_000_000_000.0; // Gbps
            let recv_rate = (current_received - last_received) as f64 * 8.0 / 1_000_000_000.0;

            let total_send_gbps = (current_sent as f64 * 8.0) / (elapsed * 1_000_000_000.0);
            let total_recv_gbps = (current_received as f64 * 8.0) / (elapsed * 1_000_000_000.0);

            println!("  [{:>3}s] Send: {:.2} Gbps (avg: {:.2}), Recv: {:.2} Gbps (avg: {:.2})",
                     elapsed as u64,
                     send_rate,
                     total_send_gbps,
                     recv_rate,
                     total_recv_gbps);

            last_sent = current_sent;
            last_received = current_received;
        }
    });

    // Run test for specified duration
    time::sleep(Duration::from_secs(TEST_DURATION_SECS)).await;

    // Stop test
    println!("\nStopping test...");
    running.store(false, Ordering::Relaxed);

    // Wait for tasks to finish
    for handle in sender_handles {
        let _ = handle.await;
    }
    monitor_handle.abort();

    // Give receiver time to process remaining data
    time::sleep(Duration::from_millis(500)).await;

    // Calculate results
    let test_duration = test_start.elapsed();
    let total_sent = bytes_sent.load(Ordering::Relaxed);
    let total_received = bytes_received.load(Ordering::Relaxed);

    let send_gbps = (total_sent as f64 * 8.0) / (test_duration.as_secs_f64() * 1_000_000_000.0);
    let recv_gbps = (total_received as f64 * 8.0) / (test_duration.as_secs_f64() * 1_000_000_000.0);

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║                    TEST RESULTS                           ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║ Duration:         {:.2} seconds", test_duration.as_secs_f64());
    println!("║ Data sent:        {:.2} GB", total_sent as f64 / (1024.0 * 1024.0 * 1024.0));
    println!("║ Data received:    {:.2} GB", total_received as f64 / (1024.0 * 1024.0 * 1024.0));
    println!("║ Send throughput:  {:.3} Gbps", send_gbps);
    println!("║ Recv throughput:  {:.3} Gbps", recv_gbps);
    println!("╠══════════════════════════════════════════════════════════╣");

    // Get performance stats
    let (peak_gbps, zero_copy_ops, pool_hits, frame_batches) = client.performance_stats();

    println!("║ Peak recorded:    {:.3} Gbps", peak_gbps);
    println!("║ Zero-copy ops:    {}", zero_copy_ops);
    println!("║ Memory pool hits: {}", pool_hits);
    println!("║ Frame batches:    {}", frame_batches);
    println!("╠══════════════════════════════════════════════════════════╣");

    // Determine performance tier
    let achieved_gbps = send_gbps.max(recv_gbps);
    if achieved_gbps >= 25.0 {
        println!("║ Performance Tier: ENTERPRISE (25+ Gbps) ✅");
    } else if achieved_gbps >= 10.0 {
        println!("║ Performance Tier: PERFORMANCE (10+ Gbps) ✅");
    } else if achieved_gbps >= 1.0 {
        println!("║ Performance Tier: STANDARD (1+ Gbps) ⚠️");
    } else if achieved_gbps >= 0.1 {
        println!("║ Performance Tier: HOME (100+ Mbps) ⚠️");
    } else {
        println!("║ Performance Tier: LIMITED (<100 Mbps) ❌");
    }

    println!("╚══════════════════════════════════════════════════════════╝");

    if achieved_gbps < 10.0 {
        println!("\n⚠️  Performance below 10 Gbps target!");
        println!("   Current: {:.3} Gbps", achieved_gbps);
        println!("   Target:  10+ Gbps\n");
        println!("   Optimization suggestions:");
        println!("   • Increase buffer sizes");
        println!("   • Use more concurrent connections");
        println!("   • Enable kernel bypass (DPDK/io_uring)");
        println!("   • Check CPU affinity settings");
        println!("   • Verify network interface capabilities");
    } else {
        println!("\n✅ Performance target achieved!");
    }

    // Cleanup
    server_handle.abort();
    client.shutdown().await;
    server.shutdown().await;

    Ok(())
}