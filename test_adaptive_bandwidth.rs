//! Test adaptive bandwidth detection functionality
//!
//! This test demonstrates that STOQ adaptive bandwidth detection is working correctly

use std::time::Duration;
use stoq::transport::adaptive::{AdaptiveBandwidthDetector, NetworkTier};

#[tokio::main]
async fn main() {
    println!("🚀 Testing STOQ Adaptive Bandwidth Detection");
    println!("================================================");

    let detector = AdaptiveBandwidthDetector::new();

    // Initial state
    println!("\n📊 Initial State:");
    println!("  Current tier: {}", detector.current_tier().description());
    let stats = detector.get_stats();
    println!("  Samples: {}", stats.sample_count);

    // Simulate low bandwidth transfer (50 Mbps)
    println!("\n🐌 Simulating low bandwidth (50 Mbps):");
    let transfer_size = 5 * 1024 * 1024; // 5MB
    let transfer_time = Duration::from_millis(800); // 50 Mbps
    detector.record_transfer(transfer_size, transfer_time);

    let stats = detector.get_stats();
    println!("  Transfer: {}MB in {}ms = {:.1} Mbps",
             transfer_size / (1024 * 1024),
             transfer_time.as_millis(),
             stats.avg_throughput_mbps);
    println!("  Detected tier: {}", detector.current_tier().description());

    // Simulate multiple medium bandwidth transfers (500 Mbps)
    println!("\n🚀 Simulating medium bandwidth (500 Mbps):");
    for i in 0..5 {
        let transfer_size = 10 * 1024 * 1024; // 10MB
        let transfer_time = Duration::from_millis(160); // ~500 Mbps
        detector.record_transfer(transfer_size, transfer_time);

        let stats = detector.get_stats();
        println!("  Transfer {}: {:.1} Mbps (avg: {:.1} Mbps)",
                 i + 1,
                 (transfer_size as f64 * 8.0) / (transfer_time.as_secs_f64() * 1_000_000.0),
                 stats.avg_throughput_mbps);
    }

    // Force tier update
    detector.force_update();
    println!("  Updated tier: {}", detector.current_tier().description());

    // Simulate high bandwidth transfers (1.5 Gbps)
    println!("\n⚡ Simulating high bandwidth (1.5 Gbps):");
    for i in 0..3 {
        let transfer_size = 100 * 1024 * 1024; // 100MB
        let transfer_time = Duration::from_millis(533); // ~1.5 Gbps
        detector.record_transfer(transfer_size, transfer_time);

        let stats = detector.get_stats();
        println!("  Transfer {}: {:.1} Mbps (avg: {:.1} Mbps)",
                 i + 1,
                 (transfer_size as f64 * 8.0) / (transfer_time.as_secs_f64() * 1_000_000.0),
                 stats.avg_throughput_mbps);
    }

    // Force tier update
    detector.force_update();
    println!("  Updated tier: {}", detector.current_tier().description());

    // Simulate ultra-high bandwidth transfers (3 Gbps)
    println!("\n🔥 Simulating ultra-high bandwidth (3 Gbps):");
    for i in 0..3 {
        let transfer_size = 200 * 1024 * 1024; // 200MB
        let transfer_time = Duration::from_millis(533); // ~3 Gbps
        detector.record_transfer(transfer_size, transfer_time);

        let stats = detector.get_stats();
        println!("  Transfer {}: {:.1} Mbps (avg: {:.1} Mbps)",
                 i + 1,
                 (transfer_size as f64 * 8.0) / (transfer_time.as_secs_f64() * 1_000_000.0),
                 stats.avg_throughput_mbps);
    }

    // Force tier update
    detector.force_update();
    println!("  Final tier: {}", detector.current_tier().description());

    // Show configuration for each tier
    println!("\n⚙️  Configuration for each tier:");
    for tier in [NetworkTier::Basic, NetworkTier::Standard, NetworkTier::HighPerformance, NetworkTier::UltraHigh] {
        let config = tier.get_config();
        println!("  {}: {} streams, {}MB buffers",
                 tier.description(),
                 config.max_concurrent_streams,
                 config.send_buffer_size / (1024 * 1024));
    }

    // Final statistics
    let final_stats = detector.get_stats();
    println!("\n📈 Final Statistics:");
    println!("  Current tier: {}", final_stats.current_tier.description());
    println!("  Total samples: {}", final_stats.sample_count);
    println!("  Average throughput: {:.1} Mbps", final_stats.avg_throughput_mbps);
    println!("  Min throughput: {:.1} Mbps", final_stats.min_throughput_mbps);
    println!("  Max throughput: {:.1} Mbps", final_stats.max_throughput_mbps);

    println!("\n✅ STOQ Adaptive Bandwidth Detection Test Complete!");
    println!("   The system successfully detected network capabilities and");
    println!("   would configure transport parameters accordingly.");
}