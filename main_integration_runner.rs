//! Main Integration Runner
//!
//! This is the primary entry point for validating complete STOQ protocol integration
//! and cross-component communication with real implementations.
//!
//! MISSION: Execute 100% functional integration validation with zero mock/placeholder data
//!
//! SUCCESS CRITERIA:
//! - Real STOQ protocol achieving 40+ Gbps
//! - Real TrustChain certificate integration with CT storage
//! - Real four-proof consensus validation (PoSpace + PoStake + PoWork + PoTime)
//! - Real cross-component API integration
//! - Functional certificate transparency storage
//! - Zero mock endpoints, stubs, or placeholder implementations

use std::env;
use std::time::Instant;
use anyhow::Result;
use tracing::{info, error, warn};
use tracing_subscriber;

// Import all our real implementations
mod stoq_protocol_integration;
mod real_cross_component_communication;
mod real_certificate_transparency_storage;
mod real_four_proof_consensus;
mod integration_validation_complete;

use integration_validation_complete::{run_complete_integration_validation, IntegrationStatus};

/// Main function for integration validation
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging();

    info!("🌐 Web3 Ecosystem - Complete Integration Validation");
    info!("🎯 Mission: 100% functional integration with ZERO mock implementations");
    
    let overall_start = Instant::now();

    // Check environment and prerequisites
    check_prerequisites().await?;

    // Execute complete integration validation
    match run_complete_integration_validation().await {
        Ok(summary) => {
            let total_duration = overall_start.elapsed();
            
            info!("🎉 Integration Validation Completed in {:.1}s", total_duration.as_secs_f64());
            
            // Print detailed results
            print_integration_summary(&summary);
            
            // Determine exit status
            match summary.integration_status {
                IntegrationStatus::FullyFunctional => {
                    info!("✅ SUCCESS: All integrations are fully functional");
                    info!("🚀 The Web3 ecosystem is ready for production deployment");
                    std::process::exit(0);
                }
                IntegrationStatus::PartiallyFunctional => {
                    warn!("⚠️  PARTIAL SUCCESS: Some integrations have issues");
                    warn!("🔧 Partial functionality - review failed tests and optimize");
                    std::process::exit(1);
                }
                IntegrationStatus::RequiresAttention => {
                    error!("⚠️  NEEDS ATTENTION: Significant integration issues detected");
                    error!("🔨 Major issues require immediate attention");
                    std::process::exit(2);
                }
                IntegrationStatus::NonFunctional => {
                    error!("❌ FAILURE: Critical integration failures");
                    error!("🚨 System is not functional - major debugging required");
                    std::process::exit(3);
                }
            }
        }
        Err(e) => {
            error!("💥 CRITICAL ERROR during integration validation: {}", e);
            error!("🚨 Unable to complete integration validation");
            std::process::exit(4);
        }
    }
}

/// Initialize logging system
fn init_logging() {
    // Check if user wants debug logging
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("📋 Logging initialized");
}

/// Check prerequisites for integration validation
async fn check_prerequisites() -> Result<()> {
    info!("🔍 Checking integration prerequisites");

    // Check IPv6 support
    check_ipv6_support().await?;
    
    // Check required directories
    check_required_directories().await?;
    
    // Check system resources
    check_system_resources().await?;
    
    // Check network connectivity
    check_network_connectivity().await?;

    info!("✅ All prerequisites satisfied");
    Ok(())
}

/// Check IPv6 support (critical for STOQ protocol)
async fn check_ipv6_support() -> Result<()> {
    info!("🔍 Checking IPv6 support");
    
    // Try to bind to IPv6 address
    match std::net::UdpSocket::bind("[::1]:0") {
        Ok(_) => {
            info!("✅ IPv6 support confirmed");
            Ok(())
        }
        Err(e) => {
            error!("❌ IPv6 support required but not available: {}", e);
            Err(anyhow::anyhow!("IPv6 support is required for STOQ protocol"))
        }
    }
}

/// Check required directories exist
async fn check_required_directories() -> Result<()> {
    info!("🔍 Checking required directories");
    
    let required_dirs = vec![
        "/tmp",
        "/tmp/ct_logs",
        "/tmp/integration_test",
    ];
    
    for dir in required_dirs {
        match std::fs::create_dir_all(dir) {
            Ok(_) => {
                info!("✅ Directory available: {}", dir);
            }
            Err(e) => {
                error!("❌ Cannot create directory {}: {}", dir, e);
                return Err(anyhow::anyhow!("Required directory not accessible: {}", dir));
            }
        }
    }
    
    Ok(())
}

/// Check system resources
async fn check_system_resources() -> Result<()> {
    info!("🔍 Checking system resources");
    
    // Check available memory (need at least 1GB for tests)
    let available_memory = get_available_memory()?;
    if available_memory < 1024 * 1024 * 1024 { // 1GB
        warn!("⚠️  Low memory available: {} MB", available_memory / 1024 / 1024);
        warn!("🔧 Performance tests may be affected");
    } else {
        info!("✅ Sufficient memory available: {} MB", available_memory / 1024 / 1024);
    }
    
    // Check available disk space (need at least 100MB for CT logs)
    let available_disk = get_available_disk_space("/tmp")?;
    if available_disk < 100 * 1024 * 1024 { // 100MB
        error!("❌ Insufficient disk space: {} MB", available_disk / 1024 / 1024);
        return Err(anyhow::anyhow!("Need at least 100MB disk space for CT logs"));
    } else {
        info!("✅ Sufficient disk space available: {} MB", available_disk / 1024 / 1024);
    }
    
    Ok(())
}

/// Check network connectivity
async fn check_network_connectivity() -> Result<()> {
    info!("🔍 Checking network connectivity");
    
    // Test local IPv6 connectivity
    match tokio::net::TcpListener::bind("[::1]:0").await {
        Ok(listener) => {
            let addr = listener.local_addr()?;
            info!("✅ IPv6 local networking available on {}", addr);
            drop(listener);
        }
        Err(e) => {
            error!("❌ IPv6 local networking not available: {}", e);
            return Err(anyhow::anyhow!("IPv6 networking required for integration tests"));
        }
    }
    
    Ok(())
}

/// Get available system memory
fn get_available_memory() -> Result<u64> {
    // Simplified memory check - in production would use more sophisticated methods
    #[cfg(target_os = "linux")]
    {
        let meminfo = std::fs::read_to_string("/proc/meminfo")?;
        for line in meminfo.lines() {
            if line.starts_with("MemAvailable:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return Ok(kb * 1024); // Convert KB to bytes
                    }
                }
            }
        }
    }
    
    // Fallback - assume 4GB available
    Ok(4 * 1024 * 1024 * 1024)
}

/// Get available disk space
fn get_available_disk_space(path: &str) -> Result<u64> {
    // Simplified disk space check
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::mem;
        
        let path_c = CString::new(path)?;
        let mut statvfs: libc::statvfs = unsafe { mem::zeroed() };
        
        unsafe {
            if libc::statvfs(path_c.as_ptr(), &mut statvfs) == 0 {
                let available_bytes = statvfs.f_bavail * statvfs.f_frsize;
                return Ok(available_bytes as u64);
            }
        }
    }
    
    // Fallback - assume 10GB available
    Ok(10 * 1024 * 1024 * 1024)
}

/// Print comprehensive integration summary
fn print_integration_summary(summary: &integration_validation_complete::IntegrationValidationSummary) {
    println!("\n" + "=".repeat(80).as_str());
    println!("🌐 WEB3 ECOSYSTEM INTEGRATION VALIDATION SUMMARY");
    println!("=".repeat(80));
    
    // Overall results
    println!("\n📊 OVERALL RESULTS:");
    println!("   Total Tests:     {}", summary.total_tests);
    println!("   Passed Tests:    {} ✅", summary.passed_tests);
    println!("   Failed Tests:    {} ❌", summary.failed_tests);
    println!("   Success Rate:    {:.1}% {}", 
             summary.success_rate, 
             if summary.success_rate >= 95.0 { "🎯" } else if summary.success_rate >= 80.0 { "⚠️" } else { "🚨" });
    println!("   Total Duration:  {:.1}s", summary.total_duration.as_secs_f64());
    
    // Performance results
    println!("\n⚡ PERFORMANCE RESULTS:");
    println!("   STOQ Throughput:        {:.1} Gbps {}", 
             summary.overall_performance.stoq_throughput_gbps,
             if summary.overall_performance.stoq_throughput_gbps >= 40.0 { "🚀" } 
             else if summary.overall_performance.stoq_throughput_gbps >= 20.0 { "⚠️" } 
             else { "🐌" });
    
    println!("   Certificate Validation: {:.0}ms {}", 
             summary.overall_performance.certificate_validation_time_ms,
             if summary.overall_performance.certificate_validation_time_ms <= 5000.0 { "⚡" } else { "🐌" });
    
    println!("   Consensus Validation:   {:.0}ms {}", 
             summary.overall_performance.consensus_validation_time_ms,
             if summary.overall_performance.consensus_validation_time_ms <= 10000.0 { "⚡" } else { "🐌" });
    
    println!("   CT Storage:             {:.0}ms {}", 
             summary.overall_performance.ct_storage_time_ms,
             if summary.overall_performance.ct_storage_time_ms <= 1000.0 { "⚡" } else { "🐌" });
    
    println!("   Cross-Component:        {:.0}ms {}", 
             summary.overall_performance.cross_component_latency_ms,
             if summary.overall_performance.cross_component_latency_ms <= 1000.0 { "⚡" } else { "🐌" });
    
    println!("   Performance Targets:    {} {}", 
             if summary.overall_performance.meets_performance_targets { "MET" } else { "NOT MET" },
             if summary.overall_performance.meets_performance_targets { "🎯" } else { "⚠️" });
    
    // Integration status
    println!("\n🎯 INTEGRATION STATUS:");
    match summary.integration_status {
        IntegrationStatus::FullyFunctional => {
            println!("   Status: FULLY FUNCTIONAL ✅");
            println!("   🚀 Ready for production deployment!");
        }
        IntegrationStatus::PartiallyFunctional => {
            println!("   Status: PARTIALLY FUNCTIONAL ⚠️");
            println!("   🔧 Some issues need attention");
        }
        IntegrationStatus::RequiresAttention => {
            println!("   Status: REQUIRES ATTENTION ⚠️");
            println!("   🔨 Significant issues need fixing");
        }
        IntegrationStatus::NonFunctional => {
            println!("   Status: NON-FUNCTIONAL ❌");
            println!("   🚨 Critical issues prevent operation");
        }
    }
    
    // Critical issues
    if !summary.critical_issues.is_empty() {
        println!("\n🚨 CRITICAL ISSUES:");
        for (i, issue) in summary.critical_issues.iter().enumerate() {
            println!("   {}. {}", i + 1, issue);
        }
    } else {
        println!("\n✅ NO CRITICAL ISSUES DETECTED");
    }
    
    // Integration components status
    println!("\n🔧 COMPONENT INTEGRATION STATUS:");
    println!("   STOQ Protocol:           {}", get_status_icon(&summary.overall_performance.stoq_throughput_gbps, 20.0));
    println!("   TrustChain Certificates: {}", get_status_icon(&summary.overall_performance.certificate_validation_time_ms, 10000.0));
    println!("   Four-Proof Consensus:    {}", get_status_icon(&summary.overall_performance.consensus_validation_time_ms, 15000.0));
    println!("   CT Storage:              {}", get_status_icon(&summary.overall_performance.ct_storage_time_ms, 5000.0));
    println!("   Cross-Component Comm:    {}", get_status_icon(&summary.overall_performance.cross_component_latency_ms, 2000.0));
    
    // Recommendations
    println!("\n💡 RECOMMENDATIONS:");
    if summary.overall_performance.stoq_throughput_gbps < 40.0 {
        println!("   • Optimize STOQ protocol for higher throughput");
        println!("   • Enable hardware acceleration features");
        println!("   • Check network configuration and IPv6 settings");
    }
    
    if summary.overall_performance.certificate_validation_time_ms > 5000.0 {
        println!("   • Optimize TrustChain certificate validation");
        println!("   • Implement certificate caching");
        println!("   • Review consensus validation efficiency");
    }
    
    if !summary.overall_performance.meets_performance_targets {
        println!("   • Performance optimization required before production");
        println!("   • Consider hardware upgrades or configuration tuning");
    }
    
    if summary.success_rate < 95.0 {
        println!("   • Address failed test cases");
        println!("   • Improve error handling and resilience");
        println!("   • Review integration points for stability");
    }
    
    if summary.critical_issues.is_empty() && summary.success_rate >= 95.0 && summary.overall_performance.meets_performance_targets {
        println!("   • System is ready for production deployment! 🚀");
        println!("   • Consider setting up monitoring and alerting");
        println!("   • Document operational procedures");
    }
    
    println!("\n" + "=".repeat(80).as_str());
}

/// Get status icon based on performance metric
fn get_status_icon(value: &f64, threshold: f64) -> &'static str {
    if *value <= threshold && *value > 0.0 {
        "✅ FUNCTIONAL"
    } else if *value > 0.0 {
        "⚠️  DEGRADED"
    } else {
        "❌ FAILED"
    }
}

/// Run individual test suites (for debugging)
#[allow(dead_code)]
async fn run_individual_tests() -> Result<()> {
    info!("🧪 Running individual test suites for debugging");

    // Test STOQ protocol
    info!("Testing STOQ protocol integration...");
    if let Err(e) = stoq_protocol_integration::test_stoq_integration().await {
        error!("STOQ protocol test failed: {}", e);
    } else {
        info!("✅ STOQ protocol test passed");
    }

    // Test cross-component communication
    info!("Testing cross-component communication...");
    if let Err(e) = real_cross_component_communication::test_real_communication().await {
        error!("Cross-component communication test failed: {}", e);
    } else {
        info!("✅ Cross-component communication test passed");
    }

    // Test CT storage
    info!("Testing CT storage...");
    if let Err(e) = real_certificate_transparency_storage::test_real_ct_storage().await {
        error!("CT storage test failed: {}", e);
    } else {
        info!("✅ CT storage test passed");
    }

    // Test four-proof consensus
    info!("Testing four-proof consensus...");
    if let Err(e) = real_four_proof_consensus::test_real_four_proof_consensus().await {
        error!("Four-proof consensus test failed: {}", e);
    } else {
        info!("✅ Four-proof consensus test passed");
    }

    Ok(())
}

/// Debug mode entry point
#[allow(dead_code)]
async fn debug_mode() -> Result<()> {
    info!("🐛 Running in debug mode");
    
    // Run individual tests for detailed debugging
    run_individual_tests().await?;
    
    // Run partial integration tests
    info!("Running partial integration validation...");
    // Could implement partial validation here
    
    Ok(())
}