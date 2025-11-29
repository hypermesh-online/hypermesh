//! Comprehensive test runner for Nexus
//! 
//! Usage: cargo run --bin test-runner [OPTIONS]

use clap::{Arg, Command};
use nexus_integration_tests::{TestConfig, TestSuite, init_test_logging};
use tracing::{info, error};
use std::process;

#[tokio::main]
async fn main() {
    let matches = Command::new("nexus-test-runner")
        .version("0.1.0")
        .author("Nexus Team")
        .about("Comprehensive test runner for Nexus core components")
        .arg(
            Arg::new("suite")
                .long("suite")
                .short('s')
                .help("Test suite to run")
                .value_parser(["unit", "integration", "performance", "chaos", "e2e", "all"])
                .default_value("all")
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .short('t')
                .help("Test timeout in seconds")
                .value_parser(clap::value_parser!(u64))
                .default_value("300")
        )
        .arg(
            Arg::new("retries")
                .long("retries")
                .short('r')
                .help("Maximum test retries")
                .value_parser(clap::value_parser!(u32))
                .default_value("3")
        )
        .arg(
            Arg::new("parallel")
                .long("parallel")
                .short('p')
                .help("Run tests in parallel")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("performance")
                .long("enable-performance")
                .help("Enable performance tests")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("chaos")
                .long("enable-chaos")
                .help("Enable chaos engineering tests")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("filter")
                .long("filter")
                .short('f')
                .help("Filter tests by name pattern")
                .value_name("PATTERN")
        )
        .get_matches();

    // Initialize logging
    init_test_logging();

    // Parse configuration
    let config = TestConfig {
        timeout_seconds: *matches.get_one::<u64>("timeout").unwrap(),
        max_retries: *matches.get_one::<u32>("retries").unwrap(),
        parallel_tests: matches.get_flag("parallel"),
        enable_performance_tests: matches.get_flag("performance") || 
            std::env::var("NEXUS_PERF_TESTS").is_ok(),
        enable_chaos_tests: matches.get_flag("chaos") || 
            std::env::var("NEXUS_CHAOS_TESTS").is_ok(),
        log_level: if matches.get_flag("verbose") { 
            "debug".to_string() 
        } else { 
            "info".to_string() 
        },
    };

    info!("🚀 Starting Nexus test runner with config: {:?}", config);

    let suite = matches.get_one::<String>("suite").unwrap();
    let mut test_suite = TestSuite::new(config);

    let result = match suite.as_str() {
        "unit" => {
            info!("Running unit tests only");
            nexus_integration_tests::unit::run_all_unit_tests().await
        }
        "integration" => {
            info!("Running integration tests only");
            nexus_integration_tests::integration::run_all_integration_tests().await
        }
        "performance" => {
            info!("Running performance tests only");
            nexus_integration_tests::performance::run_all_performance_tests().await
        }
        "chaos" => {
            info!("Running chaos tests only");
            nexus_integration_tests::chaos::run_all_chaos_tests().await
        }
        "e2e" => {
            info!("Running end-to-end tests only");
            nexus_integration_tests::e2e::run_all_e2e_tests().await
        }
        "all" => {
            info!("Running all test suites");
            test_suite.run_all().await
        }
        _ => {
            error!("Unknown test suite: {}", suite);
            process::exit(1);
        }
    };

    match result {
        Ok(()) => {
            info!("🎉 All tests completed successfully!");
            process::exit(0);
        }
        Err(e) => {
            error!("❌ Test suite failed: {}", e);
            process::exit(1);
        }
    }
}

/// Run tests with coverage reporting
#[cfg(feature = "coverage")]
async fn run_with_coverage() -> nexus_integration_tests::TestResult {
    use std::process::Command;
    
    info!("Running tests with coverage reporting");
    
    let output = Command::new("cargo")
        .args(&["tarpaulin", "--out", "Html", "--output-dir", "coverage"])
        .output()
        .expect("Failed to run tarpaulin");
    
    if !output.status.success() {
        return Err("Coverage run failed".into());
    }
    
    info!("Coverage report generated in coverage/");
    Ok(())
}

/// Generate test report
fn generate_test_report() {
    // TODO: Implement test report generation
    info!("Test report generation not yet implemented");
}