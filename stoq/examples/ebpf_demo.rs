//! eBPF demonstration for STOQ transport layer
//!
//! This example shows how to use eBPF acceleration features
//! for maximum performance with STOQ.
//!
//! Run with: sudo cargo run --example ebpf_demo --features ebpf

use anyhow::Result;
use stoq::transport::{StoqTransport, TransportConfig, Endpoint};
use std::net::Ipv6Addr;
use std::time::{Duration, Instant};
use tokio;
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed
    }

    info!("Starting STOQ eBPF demonstration");

    // Create configuration optimized for eBPF
    let mut config = TransportConfig::default();
    config.enable_zero_copy = true;
    config.enable_memory_pool = true;
    config.memory_pool_size = 2048;
    config.frame_batch_size = 64;
    config.enable_cpu_affinity = true;
    config.enable_large_send_offload = true;

    // Create transport
    let transport = StoqTransport::new(config).await?;

    // Check eBPF capabilities
    check_ebpf_status(&transport);

    // Try to attach XDP to network interface
    #[cfg(feature = "ebpf")]
    attach_xdp(&transport).await?;

    // Run performance test
    info!("\n=== Running Performance Test ===");
    run_performance_test(transport.clone()).await?;

    // Monitor eBPF metrics
    #[cfg(feature = "ebpf")]
    monitor_ebpf_metrics(&transport).await?;

    // Shutdown
    transport.shutdown().await;
    info!("STOQ eBPF demonstration complete");

    Ok(())
}

fn check_ebpf_status(transport: &StoqTransport) {
    info!("\n=== eBPF Capability Check ===");

    #[cfg(feature = "ebpf")]
    {
        if let Some(status) = transport.get_ebpf_status() {
            info!("eBPF Status:");
            info!("  Kernel version: {}", status.kernel_version);
            info!("  XDP support: {}", if status.xdp_available { "✓" } else { "✗" });
            info!("  AF_XDP support: {}", if status.af_xdp_available { "✓" } else { "✗" });
            info!("  CAP_NET_ADMIN: {}", if status.has_cap_net_admin { "✓" } else { "✗" });
            info!("  BPF filesystem: {}", if status.bpf_fs_mounted { "✓" } else { "✗" });

            if !status.has_cap_net_admin {
                warn!("CAP_NET_ADMIN not available!");
                warn!("To enable full eBPF features, run with:");
                warn!("  sudo cargo run --example ebpf_demo --features ebpf");
                warn!("Or add capability:");
                warn!("  sudo setcap cap_net_admin+ep target/release/examples/ebpf_demo");
            }

            if status.xdp_available && status.af_xdp_available {
                info!("✓ Full eBPF acceleration available!");
            } else if status.xdp_available {
                info!("⚡ XDP packet filtering available");
            } else {
                info!("⚠ eBPF not available, using standard transport");
            }
        } else {
            info!("eBPF transport not initialized");
        }
    }

    #[cfg(not(feature = "ebpf"))]
    {
        warn!("eBPF feature not compiled!");
        warn!("Build with: cargo build --features ebpf");
    }
}

#[cfg(feature = "ebpf")]
async fn attach_xdp(transport: &StoqTransport) -> Result<()> {
    info!("\n=== Attaching XDP Program ===");

    // List network interfaces
    let interfaces = ["lo", "eth0", "ens33", "enp0s3", "wlan0"];

    for interface in &interfaces {
        match transport.attach_xdp_to_interface(interface) {
            Ok(_) => {
                info!("✓ XDP attached to {}", interface);
                return Ok(());
            }
            Err(e) => {
                warn!("Failed to attach XDP to {}: {}", interface, e);
            }
        }
    }

    warn!("Could not attach XDP to any interface");
    warn!("XDP requires CAP_NET_ADMIN and a supported interface");

    Ok(())
}

async fn run_performance_test(transport: StoqTransport) -> Result<()> {
    // Create endpoint for testing
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9300);

    // Start server in background
    let server_transport = transport.clone();
    let server_task = tokio::spawn(async move {
        info!("Starting server on [::1]:9300");

        for _ in 0..10 {
            match tokio::time::timeout(
                Duration::from_secs(1),
                server_transport.accept()
            ).await {
                Ok(Ok(conn)) => {
                    info!("Server accepted connection: {}", conn.id());

                    // Echo server
                    let transport = server_transport.clone();
                    tokio::spawn(async move {
                        loop {
                            match transport.receive(&conn).await {
                                Ok(data) => {
                                    let _ = transport.send(&conn, &data).await;
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }
                Ok(Err(e)) => {
                    error!("Accept error: {}", e);
                    break;
                }
                Err(_) => {
                    // Timeout, continue
                }
            }
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect as client
    info!("Connecting to server...");
    match transport.connect(&endpoint).await {
        Ok(conn) => {
            info!("✓ Connected: {}", conn.id());

            // Test different packet sizes
            let sizes = [64, 256, 1024, 4096, 16384, 65536];
            let iterations = 1000;

            for size in sizes {
                let data = vec![0x42u8; size];
                let start = Instant::now();

                for _ in 0..iterations {
                    if let Err(e) = transport.send(&conn, &data).await {
                        error!("Send error: {}", e);
                        break;
                    }
                }

                let duration = start.elapsed();
                let total_bytes = size * iterations;
                let throughput_mbps = (total_bytes as f64 * 8.0) / duration.as_secs_f64() / 1_000_000.0;

                info!("  {} bytes: {:.2} Mbps ({} packets in {:?})",
                    size, throughput_mbps, iterations, duration);
            }

            // Get performance statistics
            let (peak_gbps, zero_copy_ops, pool_hits, frame_batches) = transport.performance_stats();

            info!("\n=== Performance Statistics ===");
            info!("  Peak throughput: {:.2} Gbps", peak_gbps);
            info!("  Zero-copy operations: {}", zero_copy_ops);
            info!("  Memory pool hits: {}", pool_hits);
            info!("  Frame batches sent: {}", frame_batches);
        }
        Err(e) => {
            error!("Connection failed: {}", e);
            warn!("Make sure no other process is using port 9300");
        }
    }

    // Clean up server
    server_task.abort();

    Ok(())
}

#[cfg(feature = "ebpf")]
async fn monitor_ebpf_metrics(transport: &StoqTransport) -> Result<()> {
    info!("\n=== eBPF Metrics Monitoring ===");

    // Monitor metrics for 5 seconds
    let monitor_duration = Duration::from_secs(5);
    let start = Instant::now();

    while start.elapsed() < monitor_duration {
        if let Some(metrics) = transport.get_ebpf_metrics() {
            // Clear screen and print metrics
            print!("\x1B[2J\x1B[1;1H");
            println!("=== eBPF Real-time Metrics ===\n");

            // Packet metrics
            println!("📊 Packet Metrics:");
            println!("  Total packets: {}", metrics.packet_metrics.total_packets);
            println!("  Packets/sec: {:.2}", metrics.packet_metrics.packets_per_second);
            println!("  Throughput: {:.2} Gbps",
                metrics.packet_metrics.bytes_per_second * 8.0 / 1_000_000_000.0);
            println!("  Kernel drops: {}", metrics.packet_metrics.kernel_drops);

            // Size distribution
            println!("\n📏 Packet Size Distribution:");
            println!("  <64B: {}", metrics.packet_metrics.size_distribution.tiny);
            println!("  64-256B: {}", metrics.packet_metrics.size_distribution.small);
            println!("  256-1KB: {}", metrics.packet_metrics.size_distribution.medium);
            println!("  1-1.5KB: {}", metrics.packet_metrics.size_distribution.large);
            println!("  >1.5KB: {}", metrics.packet_metrics.size_distribution.jumbo);

            // Connection metrics
            println!("\n🔗 Connection Metrics:");
            println!("  Active: {}", metrics.connection_metrics.active_connections);
            println!("  New/sec: {:.2}", metrics.connection_metrics.new_connections_per_sec);

            // Latency metrics
            println!("\n⏱ Latency Metrics:");
            println!("  Min: {} µs", metrics.latency_metrics.min_us);
            println!("  Avg: {} µs", metrics.latency_metrics.avg_us);
            println!("  p50: {} µs", metrics.latency_metrics.p50_us);
            println!("  p95: {} µs", metrics.latency_metrics.p95_us);
            println!("  p99: {} µs", metrics.latency_metrics.p99_us);
            println!("  Max: {} µs", metrics.latency_metrics.max_us);

            // CPU metrics
            println!("\n💻 CPU Metrics:");
            println!("  Active cores: {}", metrics.cpu_metrics.cores_active);
            println!("  Utilization: {:.1}%", metrics.cpu_metrics.avg_utilization);

            // Memory metrics
            println!("\n💾 Memory Metrics:");
            println!("  UMEM pages: {}/{}",
                metrics.memory_metrics.umem_pages_used,
                metrics.memory_metrics.umem_pages);
            println!("  Ring utilization: {:.1}%", metrics.memory_metrics.ring_utilization);
            println!("  Zero-copy ops: {}", metrics.memory_metrics.zero_copy_ops);
            println!("  Memcpy ops: {}", metrics.memory_metrics.memcpy_ops);

            println!("\nPress Ctrl+C to stop monitoring...");
        } else {
            info!("No eBPF metrics available");
            break;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}

#[cfg(not(feature = "ebpf"))]
async fn monitor_ebpf_metrics(_transport: &StoqTransport) -> Result<()> {
    warn!("eBPF metrics not available (feature not compiled)");
    Ok(())
}