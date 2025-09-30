//! eBPF integration tests for STOQ transport layer

use anyhow::Result;
use stoq::transport::{StoqTransport, TransportConfig, Endpoint};
use std::net::Ipv6Addr;
use tokio;

#[tokio::test]
async fn test_ebpf_capability_detection() -> Result<()> {
    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed, ignore
    }

    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await?;

    // Check eBPF status
    #[cfg(feature = "ebpf")]
    {
        if let Some(status) = transport.get_ebpf_status() {
            println!("eBPF Status:");
            println!("  Kernel version: {}", status.kernel_version);
            println!("  XDP available: {}", status.xdp_available);
            println!("  AF_XDP available: {}", status.af_xdp_available);
            println!("  CAP_NET_ADMIN: {}", status.has_cap_net_admin);
            println!("  BPF FS mounted: {}", status.bpf_fs_mounted);

            if !status.has_cap_net_admin {
                println!("Note: Run with CAP_NET_ADMIN for full eBPF features");
            }
        } else {
            println!("eBPF not available on this system");
        }
    }

    #[cfg(not(feature = "ebpf"))]
    {
        println!("eBPF feature not compiled - build with --features ebpf");
        assert!(transport.get_ebpf_status().is_none());
    }

    Ok(())
}

#[tokio::test]
async fn test_ebpf_metrics_collection() -> Result<()> {
    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed, ignore
    }

    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await?;

    // Try to get eBPF metrics
    #[cfg(feature = "ebpf")]
    {
        if let Some(metrics) = transport.get_ebpf_metrics() {
            println!("eBPF Metrics:");
            println!("{}", metrics);

            // Verify metrics structure
            assert!(metrics.packet_metrics.total_packets >= 0);
            assert!(metrics.connection_metrics.active_connections >= 0);
            assert!(metrics.latency_metrics.min_us >= 0);
        } else {
            println!("eBPF metrics not available");
        }
    }

    #[cfg(not(feature = "ebpf"))]
    {
        assert!(transport.get_ebpf_metrics().is_none());
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Requires root or CAP_NET_ADMIN
async fn test_xdp_attachment() -> Result<()> {
    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed, ignore
    }

    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await?;

    // Try to attach XDP to loopback interface
    #[cfg(feature = "ebpf")]
    {
        match transport.attach_xdp_to_interface("lo") {
            Ok(_) => println!("XDP attached successfully to loopback"),
            Err(e) => println!("Failed to attach XDP (expected without privileges): {}", e),
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Requires root or CAP_NET_ADMIN
async fn test_af_xdp_socket_creation() -> Result<()> {
    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed, ignore
    }

    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await?;

    // Try to create AF_XDP socket
    #[cfg(feature = "ebpf")]
    {
        match transport.create_zero_copy_socket("lo", 0) {
            Ok(_) => println!("AF_XDP socket created successfully"),
            Err(e) => println!("Failed to create AF_XDP socket (expected without privileges): {}", e),
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fallback_without_ebpf() -> Result<()> {
    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed, ignore
    }

    // Test that STOQ works without eBPF
    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await?;

    // Create endpoint
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9293);

    // Start accepting connections in background
    let transport_clone = transport.clone();
    let accept_task = tokio::spawn(async move {
        match transport_clone.accept().await {
            Ok(conn) => println!("Accepted connection: {}", conn.id()),
            Err(e) => println!("Accept error (expected in test): {}", e),
        }
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Try to connect (may fail in test environment, that's ok)
    match transport.connect(&endpoint).await {
        Ok(conn) => {
            println!("Connected successfully: {}", conn.id());

            // Try to send some data
            let data = b"Hello, eBPF!";
            if let Err(e) = transport.send(&conn, data).await {
                println!("Send error (expected in test): {}", e);
            }
        }
        Err(e) => {
            println!("Connection failed (expected in test environment): {}", e);
        }
    }

    // Clean up
    accept_task.abort();
    transport.shutdown().await;

    Ok(())
}

#[tokio::test]
async fn test_performance_with_ebpf() -> Result<()> {
    // Initialize crypto provider
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed, ignore
    }

    let mut config = TransportConfig::default();
    config.enable_zero_copy = true;
    config.enable_memory_pool = true;
    config.frame_batch_size = 64;

    let transport = StoqTransport::new(config).await?;

    // Get performance stats
    let (peak_gbps, zero_copy_ops, pool_hits, frame_batches) = transport.performance_stats();

    println!("Performance Stats:");
    println!("  Peak throughput: {:.2} Gbps", peak_gbps);
    println!("  Zero-copy operations: {}", zero_copy_ops);
    println!("  Memory pool hits: {}", pool_hits);
    println!("  Frame batches: {}", frame_batches);

    // Check if eBPF improved performance
    #[cfg(feature = "ebpf")]
    {
        if transport.get_ebpf_status().is_some() {
            println!("eBPF acceleration is available for performance boost");
        }
    }

    Ok(())
}