//! Example demonstrating STOQ's built-in monitoring capabilities

use stoq::{
    StoqTransport, StoqMonitor, MonitoringAPI, Endpoint, TransportConfig,
    HealthLevel,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== STOQ Monitoring Demo ===\n");

    // Create transport with default config
    let config = TransportConfig::default();
    let transport = Arc::new(StoqTransport::new(config).await?);

    // Create monitor
    let mut monitor = StoqMonitor::new(transport.clone());

    // Simulate some activity
    println!("Starting simulated network activity...\n");

    // Create some connections
    let endpoints = vec![
        Endpoint::new(std::net::Ipv6Addr::LOCALHOST, 9293),
        Endpoint::new(std::net::Ipv6Addr::LOCALHOST, 9294),
        Endpoint::new(std::net::Ipv6Addr::LOCALHOST, 9295),
    ];

    // Spawn background task to generate traffic
    let transport_clone = transport.clone();
    tokio::spawn(async move {
        for _ in 0..10 {
            // Simulate connection attempts (will fail but generate metrics)
            for endpoint in &endpoints {
                let _ = transport_clone.connect(endpoint).await;
            }
            sleep(Duration::from_millis(100)).await;
        }
    });

    // Monitor and display metrics
    for i in 0..5 {
        sleep(Duration::from_secs(2)).await;

        println!("--- Collection #{} ---", i + 1);

        // Get comprehensive metrics
        let metrics = monitor.get_metrics();
        println!("Transport Metrics:");
        println!("  Bytes sent: {}", metrics.bytes_sent);
        println!("  Bytes received: {}", metrics.bytes_received);
        println!("  Active connections: {}", metrics.active_connections);
        println!("  Total connections: {}", metrics.total_connections);
        println!("  Throughput: {:.2} Gbps", metrics.throughput_gbps);
        println!("  Avg latency: {} μs", metrics.avg_latency_us);

        // Get protocol metrics
        println!("\nProtocol Metrics:");
        println!("  Packets tokenized: {}", metrics.packets_tokenized);
        println!("  Packets sharded: {}", metrics.packets_sharded);
        println!("  Shards reassembled: {}", metrics.shards_reassembled);
        println!("  Hop routes: {}", metrics.hop_routes_processed);

        // Get performance metrics
        println!("\nPerformance Metrics:");
        println!("  Peak throughput: {:.2} Gbps", metrics.peak_throughput_gbps);
        println!("  Zero-copy ops: {}", metrics.zero_copy_operations);
        println!("  Memory pool hits: {}", metrics.memory_pool_hits);
        println!("  Frame batches: {}", metrics.frame_batches_sent);

        // Get error metrics
        println!("\nError Metrics:");
        println!("  Connection failures: {}", metrics.connection_failures);
        println!("  Packet drops: {}", metrics.packet_drops);
        println!("  Sharding errors: {}", metrics.sharding_errors);
        println!("  Reassembly errors: {}", metrics.reassembly_errors);

        // Get health status
        let health = monitor.get_health();
        println!("\nHealth Status: {:?}", health.level);
        if health.level != HealthLevel::Healthy {
            println!("Issues detected:");
            for issue in &health.issues {
                println!("  - {}", issue);
            }
        }

        // Get summary for dashboard
        let summary = monitor.get_summary();
        println!("\nDashboard Summary:");
        println!("  Current: {:.2} Gbps (trend: {}%)",
                 summary.current_throughput_gbps,
                 summary.throughput_trend_percent);
        println!("  Connections: {} active / {} total",
                 summary.active_connections,
                 summary.total_connections);
        println!("  Latency: {:.2}ms avg / {:.2}ms P99",
                 summary.avg_latency_ms,
                 summary.p99_latency_ms);
        println!("  Packets/sec: {:.0}", summary.packets_per_sec);
        println!("  Error rate: {:.2}%", summary.error_rate_percent);
        println!("  Memory efficiency: {:.1}%", summary.memory_efficiency_percent);

        println!();
    }

    // Export final metrics as JSON
    println!("=== Final Metrics (JSON) ===");
    let json_export = monitor.export_json();
    println!("{}", json_export);

    // Shutdown
    transport.shutdown().await;

    println!("\n=== Demo Complete ===");
    Ok(())
}