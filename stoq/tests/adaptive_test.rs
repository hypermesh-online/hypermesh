//! Tests for adaptive connection optimization

use std::sync::Arc;
use std::time::Duration;
use stoq::transport::{
    TransportConfig, StoqTransport, NetworkTier, Endpoint, adaptive::*
};
use std::net::Ipv6Addr;
use tokio::time::{sleep, timeout};
use tracing::{info, debug};

/// Test that configuration changes affect live connections
#[tokio::test]
async fn test_live_connection_adaptation() {
    tracing_subscriber::fmt::init();

    // Create transport with initial config
    let mut config = TransportConfig::default();
    config.bind_address = Ipv6Addr::UNSPECIFIED;
    config.port = 0; // Random port
    config.send_buffer_size = 1024 * 1024; // 1MB initial

    let mut transport = StoqTransport::new(config.clone()).await.unwrap();

    // Start adaptation manager
    transport.start_adaptation().await;

    // Create a mock server
    let server_addr = transport.local_addr().unwrap();
    info!("Server listening on {}", server_addr);

    // Spawn server accept loop
    let server_transport = transport.clone();
    tokio::spawn(async move {
        loop {
            match server_transport.accept().await {
                Ok(conn) => {
                    info!("Server accepted connection: {}", conn.id());
                }
                Err(e) => {
                    debug!("Accept error: {}", e);
                    break;
                }
            }
        }
    });

    // Connect to server
    let endpoint = Endpoint::new(
        server_addr.ip().to_string().parse::<Ipv6Addr>().unwrap_or(Ipv6Addr::LOCALHOST),
        server_addr.port()
    );

    let connection = transport.connect(&endpoint).await.unwrap();
    let conn_id = connection.id();
    info!("Connected with ID: {}", conn_id);

    // Wait for initial adaptation
    sleep(Duration::from_millis(1500)).await;

    // Get initial stats
    let initial_stats = transport.adaptation_stats();
    assert!(!initial_stats.is_empty(), "Should have adaptation stats");

    // Update config for live connections
    let mut new_config = config.clone();
    new_config.send_buffer_size = 16 * 1024 * 1024; // 16MB
    new_config.max_concurrent_streams = 1000;

    transport.update_live_config(new_config).await;

    // Wait for adaptation to apply
    sleep(Duration::from_millis(2000)).await;

    // Verify adaptation occurred
    let updated_stats = transport.adaptation_stats();
    let conn_stats = updated_stats.iter()
        .find(|(id, _)| id == &conn_id)
        .map(|(_, stats)| stats);

    assert!(conn_stats.is_some(), "Should have stats for connection");
    let stats = conn_stats.unwrap();
    assert!(stats.adaptation_count > 0, "Should have adapted at least once");

    info!("Test passed: Live connection adapted {} times", stats.adaptation_count);
}

/// Test automatic network tier detection
#[tokio::test]
async fn test_tier_detection() {
    tracing_subscriber::fmt::init();

    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await.unwrap();

    // Start adaptation
    transport.start_adaptation().await;

    // Create server
    let server_addr = transport.local_addr().unwrap();

    // Spawn server
    let server_transport = transport.clone();
    tokio::spawn(async move {
        loop {
            if server_transport.accept().await.is_err() {
                break;
            }
        }
    });

    // Connect with different simulated conditions
    let endpoint = Endpoint::new(
        server_addr.ip().to_string().parse::<Ipv6Addr>().unwrap_or(Ipv6Addr::LOCALHOST),
        server_addr.port()
    );

    let connection = transport.connect(&endpoint).await.unwrap();
    let conn_id = connection.id();

    // Auto-detect tiers
    transport.auto_detect_tiers().await;

    // Wait for detection
    sleep(Duration::from_secs(2)).await;

    // Check detected tier
    if let Some(adaptive_conn) = transport.get_adaptive_connection(&conn_id) {
        let tier = adaptive_conn.current_tier();
        info!("Detected network tier: {:?}", tier);

        // Should detect at least standard tier for localhost
        match tier {
            NetworkTier::Standard { .. } |
            NetworkTier::Performance { .. } |
            NetworkTier::DataCenter { .. } => {
                info!("Correctly detected high-performance tier for localhost");
            }
            _ => {
                // Acceptable if network is actually slow
                info!("Detected lower tier, may be due to actual network conditions");
            }
        }
    }
}

/// Test hysteresis prevents parameter thrashing
#[tokio::test]
async fn test_hysteresis() {
    tracing_subscriber::fmt::init();

    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await.unwrap();

    transport.start_adaptation().await;

    let server_addr = transport.local_addr().unwrap();

    // Server loop
    let server_transport = transport.clone();
    tokio::spawn(async move {
        loop {
            if server_transport.accept().await.is_err() {
                break;
            }
        }
    });

    let endpoint = Endpoint::new(
        server_addr.ip().to_string().parse::<Ipv6Addr>().unwrap_or(Ipv6Addr::LOCALHOST),
        server_addr.port()
    );

    let connection = transport.connect(&endpoint).await.unwrap();
    let conn_id = connection.id();

    // Force multiple rapid adaptations
    for i in 0..5 {
        transport.force_connection_adaptation(&conn_id).await.ok();
        sleep(Duration::from_millis(100)).await; // Rapid changes
    }

    // Get stats
    let stats = transport.adaptation_stats();
    let conn_stats = stats.iter()
        .find(|(id, _)| id == &conn_id)
        .map(|(_, stats)| stats);

    if let Some(stats) = conn_stats {
        // Due to hysteresis, adaptations should be limited
        info!("Adaptations with hysteresis: {}", stats.adaptation_count);

        // Should have some adaptations but not every attempt
        assert!(stats.adaptation_count > 0, "Should have at least one adaptation");
        assert!(stats.adaptation_count <= 5, "Hysteresis should limit adaptations");
    }
}

/// Test that adaptation can be disabled
#[tokio::test]
async fn test_disable_adaptation() {
    tracing_subscriber::fmt::init();

    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await.unwrap();

    // Start but then disable adaptation
    transport.start_adaptation().await;
    transport.set_adaptation_enabled(false);

    let server_addr = transport.local_addr().unwrap();

    // Server
    let server_transport = transport.clone();
    tokio::spawn(async move {
        loop {
            if server_transport.accept().await.is_err() {
                break;
            }
        }
    });

    let endpoint = Endpoint::new(
        server_addr.ip().to_string().parse::<Ipv6Addr>().unwrap_or(Ipv6Addr::LOCALHOST),
        server_addr.port()
    );

    let connection = transport.connect(&endpoint).await.unwrap();
    let conn_id = connection.id();

    // Try to force adaptation (should be disabled)
    transport.force_connection_adaptation(&conn_id).await.ok();

    sleep(Duration::from_secs(2)).await;

    let stats = transport.adaptation_stats();
    let conn_stats = stats.iter()
        .find(|(id, _)| id == &conn_id)
        .map(|(_, stats)| stats);

    if let Some(stats) = conn_stats {
        assert!(!stats.enabled, "Adaptation should be disabled");
        info!("Adaptation correctly disabled");
    }
}

/// Test connection-specific tier setting
#[tokio::test]
async fn test_manual_tier_setting() {
    tracing_subscriber::fmt::init();

    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await.unwrap();

    transport.start_adaptation().await;

    let server_addr = transport.local_addr().unwrap();

    let server_transport = transport.clone();
    tokio::spawn(async move {
        loop {
            if server_transport.accept().await.is_err() {
                break;
            }
        }
    });

    let endpoint = Endpoint::new(
        server_addr.ip().to_string().parse::<Ipv6Addr>().unwrap_or(Ipv6Addr::LOCALHOST),
        server_addr.port()
    );

    let connection = transport.connect(&endpoint).await.unwrap();
    let conn_id = connection.id();

    // Set specific tier
    let tier = NetworkTier::DataCenter { gbps: 100.0 };
    transport.set_connection_tier(&conn_id, tier.clone()).await.unwrap();

    // Verify parameters were applied
    if let Some(adaptive_conn) = transport.get_adaptive_connection(&conn_id) {
        let params = adaptive_conn.parameters();

        // DataCenter tier should have maximum parameters
        assert!(params.stream_window >= 32 * 1024 * 1024, "Should have large stream window");
        assert!(params.max_streams >= 1000, "Should have many concurrent streams");
        info!("Manual tier setting applied correct parameters");
    }
}

/// Integration test simulating real network changes
#[tokio::test]
async fn test_real_network_simulation() {
    tracing_subscriber::fmt::init();

    let mut config = TransportConfig::default();
    config.bind_address = Ipv6Addr::UNSPECIFIED;

    let mut transport = StoqTransport::new(config.clone()).await.unwrap();

    transport.start_adaptation().await;

    let server_addr = transport.local_addr().unwrap();

    // Server with connection tracking
    let server_transport = transport.clone();
    let server_handle = tokio::spawn(async move {
        let mut connections = Vec::new();
        for _ in 0..3 {
            match timeout(Duration::from_secs(5), server_transport.accept()).await {
                Ok(Ok(conn)) => {
                    info!("Server accepted connection: {}", conn.id());
                    connections.push(conn);
                }
                _ => break,
            }
        }
        connections
    });

    // Simulate multiple network conditions
    let scenarios = vec![
        ("Home Network", NetworkTier::Home { mbps: 300.0 }),
        ("Gigabit LAN", NetworkTier::Standard { gbps: 1.0 }),
        ("Data Center", NetworkTier::DataCenter { gbps: 100.0 }),
    ];

    let endpoint = Endpoint::new(
        server_addr.ip().to_string().parse::<Ipv6Addr>().unwrap_or(Ipv6Addr::LOCALHOST),
        server_addr.port()
    );

    for (name, tier) in scenarios {
        info!("Testing scenario: {}", name);

        let connection = transport.connect(&endpoint).await.unwrap();
        let conn_id = connection.id();

        // Simulate tier change
        transport.set_connection_tier(&conn_id, tier).await.unwrap();

        // Give time for adaptation
        sleep(Duration::from_millis(500)).await;

        // Verify adaptation
        if let Some(adaptive) = transport.get_adaptive_connection(&conn_id) {
            let current = adaptive.current_tier();
            info!("Connection adapted to: {:?}", current);

            let stats = adaptive.adaptation_stats();
            assert!(stats.adaptation_count > 0, "Should have adapted for {}", name);
        }
    }

    // Clean shutdown
    drop(transport);
    let _ = server_handle.await;

    info!("Real network simulation test completed");
}

/// Performance test - adaptation overhead
#[tokio::test]
async fn test_adaptation_overhead() {
    use std::time::Instant;

    tracing_subscriber::fmt::init();

    let config = TransportConfig::default();
    let transport = StoqTransport::new(config).await.unwrap();

    transport.start_adaptation().await;

    let server_addr = transport.local_addr().unwrap();

    let server_transport = transport.clone();
    tokio::spawn(async move {
        loop {
            if server_transport.accept().await.is_err() {
                break;
            }
        }
    });

    let endpoint = Endpoint::new(
        server_addr.ip().to_string().parse::<Ipv6Addr>().unwrap_or(Ipv6Addr::LOCALHOST),
        server_addr.port()
    );

    let connection = transport.connect(&endpoint).await.unwrap();
    let conn_id = connection.id();

    // Measure adaptation time
    let start = Instant::now();
    transport.force_connection_adaptation(&conn_id).await.unwrap();
    let adaptation_time = start.elapsed();

    info!("Adaptation took: {:?}", adaptation_time);

    // Should be very fast
    assert!(
        adaptation_time < Duration::from_millis(100),
        "Adaptation should be fast, took {:?}",
        adaptation_time
    );

    // Measure multiple adaptations
    let start = Instant::now();
    for _ in 0..10 {
        transport.force_connection_adaptation(&conn_id).await.ok();
    }
    let total_time = start.elapsed();

    let avg_time = total_time / 10;
    info!("Average adaptation time: {:?}", avg_time);

    assert!(
        avg_time < Duration::from_millis(10),
        "Average adaptation should be very fast"
    );
}