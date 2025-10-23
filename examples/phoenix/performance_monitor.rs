//! Real-time Performance Monitoring Tool
//!
//! This tool continuously monitors STOQ transport performance and reports
//! REAL metrics, replacing all hardcoded fantasy values with actual measurements.

use stoq::{
    StoqBuilder, StoqConfig,
    transport::{StoqTransport, TransportConfig, Endpoint},
    performance_monitor::{PerformanceMonitor, NetworkTier, HealthStatus},
};
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{info, warn, Level};
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "stoq-perf")]
#[command(about = "STOQ Performance Monitor - Real metrics, not fantasies")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run continuous performance monitoring
    Monitor {
        /// Target server address
        #[arg(short, long, default_value = "::1")]
        server: String,

        /// Target server port
        #[arg(short, long, default_value_t = 9292)]
        port: u16,

        /// Monitoring interval in seconds
        #[arg(short, long, default_value_t = 1)]
        interval: u64,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run performance benchmark
    Benchmark {
        /// Number of iterations
        #[arg(short, long, default_value_t = 100)]
        iterations: u32,

        /// Data size in MB
        #[arg(short, long, default_value_t = 10)]
        size_mb: u32,
    },

    /// Validate performance claims
    Validate {
        /// Expected throughput in Gbps
        #[arg(short, long, default_value_t = 1.0)]
        expected_gbps: f64,

        /// Expected latency in ms
        #[arg(short, long, default_value_t = 10.0)]
        expected_ms: f64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Initialize crypto
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("Failed to install crypto provider"))?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Monitor { server, port, interval: interval_secs, verbose } => {
            run_monitoring(server, port, interval_secs, verbose).await?;
        }
        Commands::Benchmark { iterations, size_mb } => {
            run_benchmark(iterations, size_mb).await?;
        }
        Commands::Validate { expected_gbps, expected_ms } => {
            run_validation(expected_gbps, expected_ms).await?;
        }
    }

    Ok(())
}

async fn run_monitoring(
    server: String,
    port: u16,
    interval_secs: u64,
    verbose: bool,
) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           STOQ REAL-TIME PERFORMANCE MONITOR                ║");
    println!("║         Measuring Actual Performance, Not Fantasies         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Parse server address
    let server_addr: Ipv6Addr = server.parse()
        .unwrap_or(Ipv6Addr::LOCALHOST);

    // Create performance monitor
    let monitor = Arc::new(PerformanceMonitor::new(1.0, 10.0));
    monitor.start_monitoring().await;

    // Setup transport
    let config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 0, // Dynamic
        enable_zero_copy: true,
        enable_memory_pool: true,
        ..Default::default()
    };

    let transport = Arc::new(StoqTransport::new(config).await?);
    let endpoint = Endpoint::new(server_addr, port);

    println!("Monitoring server at [{}]:{}", server_addr, port);
    println!("Press Ctrl+C to stop monitoring\n");

    // Start monitoring loop
    let mut monitoring_interval = interval(Duration::from_secs(interval_secs));
    let mut iteration = 0;

    loop {
        monitoring_interval.tick().await;
        iteration += 1;

        // Perform test transfer
        match perform_test_transfer(&transport, &endpoint, &monitor).await {
            Ok(gbps) => {
                if verbose || iteration % 10 == 1 {
                    print_performance_summary(&monitor, iteration, gbps);
                } else {
                    print_compact_status(&monitor, gbps);
                }
            }
            Err(e) => {
                warn!("Test transfer failed: {}", e);
            }
        }

        // Check for performance degradation
        let snapshot = monitor.get_snapshot();
        match snapshot.health_status {
            HealthStatus::Critical { message } => {
                println!("\n⚠️  CRITICAL ALERT: {}", message);
            }
            _ => {}
        }
    }
}

async fn perform_test_transfer(
    transport: &Arc<StoqTransport>,
    endpoint: &Endpoint,
    monitor: &Arc<PerformanceMonitor>,
) -> Result<f64> {
    // Connect to server
    let conn = transport.connect(endpoint).await?;

    // Generate test data (1MB)
    let test_data = vec![0xAB; 1024 * 1024];

    // Measure transfer
    let start = Instant::now();

    let mut stream = conn.open_stream().await?;
    stream.send(&test_data).await?;
    stream.finish().await?;

    let duration = start.elapsed();

    // Record metrics
    monitor.record_bytes(test_data.len());
    monitor.record_latency(duration);
    monitor.record_connection(true, Some(duration));

    // Calculate throughput
    let gbps = (test_data.len() as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000_000.0);

    conn.close();
    Ok(gbps)
}

fn print_performance_summary(monitor: &Arc<PerformanceMonitor>, iteration: u32, current_gbps: f64) {
    let snapshot = monitor.get_snapshot();

    println!("\n════════════════════════════════════════════════");
    println!(" Iteration #{} - Performance Summary", iteration);
    println!("────────────────────────────────────────────────");

    // Throughput
    println!("📊 Throughput:");
    println!("   Current:  {:.3} Gbps", current_gbps);
    println!("   Average:  {:.3} Gbps", snapshot.throughput.average_gbps);
    println!("   Peak:     {:.3} Gbps", snapshot.throughput.peak_gbps);
    println!("   P95:      {:.3} Gbps", snapshot.throughput.p95_gbps);

    // Latency
    println!("⏱️  Latency:");
    println!("   Current:  {:.2} ms", snapshot.latency.current_ms);
    println!("   Average:  {:.2} ms", snapshot.latency.average_ms);
    println!("   P95:      {:.2} ms", snapshot.latency.p95_ms);
    println!("   P99:      {:.2} ms", snapshot.latency.p99_ms);

    // Network Tier
    print!("🌐 Network Tier: ");
    match snapshot.performance_tier {
        NetworkTier::Slow { mbps } => println!("SLOW ({:.1} Mbps)", mbps),
        NetworkTier::Home { mbps } => println!("HOME ({:.1} Mbps)", mbps),
        NetworkTier::Standard { gbps } => println!("STANDARD ({:.1} Gbps)", gbps),
        NetworkTier::Performance { gbps } => println!("PERFORMANCE ({:.1} Gbps)", gbps),
        NetworkTier::Enterprise { gbps } => println!("ENTERPRISE ({:.1} Gbps)", gbps),
        NetworkTier::DataCenter { gbps } => println!("DATA CENTER ({:.1} Gbps)", gbps),
    }

    // Health Status
    print!("🏥 Health: ");
    match snapshot.health_status {
        HealthStatus::Healthy { .. } => println!("✅ HEALTHY"),
        HealthStatus::Warning { ref message } => println!("⚠️  WARNING - {}", message),
        HealthStatus::Critical { ref message } => println!("❌ CRITICAL - {}", message),
    }

    println!("════════════════════════════════════════════════");
}

fn print_compact_status(monitor: &Arc<PerformanceMonitor>, current_gbps: f64) {
    let snapshot = monitor.get_snapshot();
    print!("\r📊 {:.3} Gbps | ⏱️  {:.2} ms | 🌐 ",
           current_gbps,
           snapshot.latency.current_ms);

    match snapshot.performance_tier {
        NetworkTier::Slow { .. } => print!("SLOW"),
        NetworkTier::Home { .. } => print!("HOME"),
        NetworkTier::Standard { .. } => print!("STD"),
        NetworkTier::Performance { .. } => print!("PERF"),
        NetworkTier::Enterprise { .. } => print!("ENT"),
        NetworkTier::DataCenter { .. } => print!("DC"),
    }

    print!(" | ");

    match snapshot.health_status {
        HealthStatus::Healthy { .. } => print!("✅"),
        HealthStatus::Warning { .. } => print!("⚠️"),
        HealthStatus::Critical { .. } => print!("❌"),
    }

    print!("     "); // Clear line remainder
    use std::io::{self, Write};
    io::stdout().flush().unwrap();
}

async fn run_benchmark(iterations: u32, size_mb: u32) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              STOQ PERFORMANCE BENCHMARK                     ║");
    println!("║                 Real Measurements Only                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let monitor = Arc::new(PerformanceMonitor::new(1.0, 10.0));

    // Setup test server
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 19292,
        max_concurrent_streams: 100,
        send_buffer_size: 32 * 1024 * 1024,
        receive_buffer_size: 32 * 1024 * 1024,
        enable_zero_copy: true,
        enable_memory_pool: true,
        ..Default::default()
    };

    let server = Arc::new(StoqTransport::new(server_config.clone()).await?);
    let server_clone = server.clone();

    // Start echo server
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

    // Setup client
    let client = Arc::new(StoqTransport::new(TransportConfig::default()).await?);
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_config.port);

    println!("Running {} iterations with {}MB data size", iterations, size_mb);
    println!("────────────────────────────────────────────────\n");

    let test_data = vec![0xAB; (size_mb * 1024 * 1024) as usize];
    let mut measurements = Vec::new();

    // Progress bar
    for i in 0..iterations {
        if i % 10 == 0 {
            print!("Progress: [{:>3}%] ", (i * 100 / iterations));
            for _ in 0..(i * 20 / iterations) {
                print!("█");
            }
            for _ in (i * 20 / iterations)..20 {
                print!("░");
            }
            print!("\r");
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        }

        let conn = client.connect(&endpoint).await?;
        let start = Instant::now();

        let mut stream = conn.open_stream().await?;
        stream.send(&test_data).await?;
        let _ = stream.receive().await?;

        let duration = start.elapsed();
        let gbps = (test_data.len() as f64 * 8.0 * 2.0) / (duration.as_secs_f64() * 1_000_000_000.0);

        measurements.push(gbps);
        monitor.record_bytes(test_data.len() * 2);
        monitor.record_latency(duration / 2);

        conn.close();
    }

    println!("\n\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    BENCHMARK RESULTS                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Calculate statistics
    measurements.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg = measurements.iter().sum::<f64>() / measurements.len() as f64;
    let min = measurements[0];
    let max = measurements[measurements.len() - 1];
    let p50 = measurements[measurements.len() / 2];
    let p95 = measurements[measurements.len() * 95 / 100];
    let p99 = measurements[measurements.len() * 99 / 100];

    println!("Throughput Statistics (Gbps):");
    println!("┌────────────────┬──────────────┐");
    println!("│ Metric         │ Value        │");
    println!("├────────────────┼──────────────┤");
    println!("│ Minimum        │ {:>12.3} │", min);
    println!("│ Average        │ {:>12.3} │", avg);
    println!("│ Median (P50)   │ {:>12.3} │", p50);
    println!("│ P95            │ {:>12.3} │", p95);
    println!("│ P99            │ {:>12.3} │", p99);
    println!("│ Maximum        │ {:>12.3} │", max);
    println!("└────────────────┴──────────────┘\n");

    // Reality check against 40 Gbps claim
    println!("Reality Check:");
    println!("──────────────────────────────────");
    let claimed = 40.0;
    let reality_percent = (avg / claimed) * 100.0;

    if reality_percent < 1.0 {
        println!("❌ SEVERE: Achieving only {:.2}% of claimed {} Gbps", reality_percent, claimed);
        println!("          This is a {}x overstatement", (claimed / avg) as u32);
    } else if reality_percent < 10.0 {
        println!("⚠️  WARNING: Achieving {:.1}% of claimed performance", reality_percent);
    } else if reality_percent < 50.0 {
        println!("📊 MODERATE: Achieving {:.1}% of theoretical maximum", reality_percent);
    } else {
        println!("✅ GOOD: Performance within reasonable range of claims");
    }

    println!("\nRecommended honest claim: {:.1} Gbps (based on P95)", p95);

    // Cleanup
    client.shutdown().await;
    server.shutdown().await;

    Ok(())
}

async fn run_validation(expected_gbps: f64, expected_ms: f64) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║            PERFORMANCE CLAIM VALIDATION                     ║");
    println!("║         Validating Against Expected Performance             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Expected Performance:");
    println!("  Throughput: {:.1} Gbps", expected_gbps);
    println!("  Latency: {:.1} ms\n", expected_ms);

    // Run actual performance test
    let monitor = Arc::new(PerformanceMonitor::new(expected_gbps, expected_ms));

    // Setup test environment
    let server_config = TransportConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 29292,
        enable_zero_copy: true,
        enable_memory_pool: true,
        ..Default::default()
    };

    let server = Arc::new(StoqTransport::new(server_config.clone()).await?);
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

    let client = Arc::new(StoqTransport::new(TransportConfig::default()).await?);
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, server_config.port);

    println!("Running validation tests...\n");

    // Run 50 test iterations
    let mut throughputs = Vec::new();
    let mut latencies = Vec::new();

    for _ in 0..50 {
        let conn = client.connect(&endpoint).await?;
        let test_data = vec![0xAB; 10 * 1024 * 1024]; // 10MB

        let start = Instant::now();
        let mut stream = conn.open_stream().await?;
        stream.send(&test_data).await?;
        let _ = stream.receive().await?;
        let duration = start.elapsed();

        let gbps = (test_data.len() as f64 * 8.0 * 2.0) / (duration.as_secs_f64() * 1_000_000_000.0);
        let latency_ms = (duration.as_secs_f64() * 1000.0) / 2.0;

        throughputs.push(gbps);
        latencies.push(latency_ms);

        monitor.record_bytes(test_data.len() * 2);
        monitor.record_latency(duration / 2);

        conn.close();
    }

    // Calculate results
    let avg_throughput = throughputs.iter().sum::<f64>() / throughputs.len() as f64;
    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                   VALIDATION RESULTS                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("┌────────────────┬──────────┬──────────┬──────────┐");
    println!("│ Metric         │ Expected │ Actual   │ Status   │");
    println!("├────────────────┼──────────┼──────────┼──────────┤");

    let throughput_ratio = avg_throughput / expected_gbps;
    let throughput_status = if throughput_ratio >= 0.9 {
        "✅ PASS"
    } else if throughput_ratio >= 0.5 {
        "⚠️  WARN"
    } else {
        "❌ FAIL"
    };

    println!("│ Throughput     │ {:>7.1} │ {:>7.3} │ {} │",
            expected_gbps, avg_throughput, throughput_status);

    let latency_ratio = avg_latency / expected_ms;
    let latency_status = if latency_ratio <= 1.1 {
        "✅ PASS"
    } else if latency_ratio <= 2.0 {
        "⚠️  WARN"
    } else {
        "❌ FAIL"
    };

    println!("│ Latency (ms)   │ {:>7.1} │ {:>7.2} │ {} │",
            expected_ms, avg_latency, latency_status);

    println!("└────────────────┴──────────┴──────────┴──────────┘\n");

    // Overall verdict
    println!("Overall Validation:");
    if throughput_ratio >= 0.9 && latency_ratio <= 1.1 {
        println!("✅ PASS - Performance meets expectations");
    } else if throughput_ratio >= 0.5 && latency_ratio <= 2.0 {
        println!("⚠️  WARNING - Performance below expectations");
        println!("   Throughput: {:.1}% of expected", throughput_ratio * 100.0);
        println!("   Latency: {:.1}x higher than expected", latency_ratio);
    } else {
        println!("❌ FAIL - Performance significantly below claims");
        println!("   Throughput: Only {:.1}% of expected", throughput_ratio * 100.0);
        println!("   Latency: {:.1}x worse than expected", latency_ratio);
    }

    // Cleanup
    client.shutdown().await;
    server.shutdown().await;

    Ok(())
}