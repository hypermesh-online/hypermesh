// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Main test runner for Web3 ecosystem
// Orchestrates comprehensive testing and validation

mod integration;
mod performance;
mod security;
mod test_framework;
// TODO: Re-enable when modules are implemented
// mod chaos;
// mod validation;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use test_framework::{TestConfig, TestExecutor};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "web3-test")]
#[command(about = "Comprehensive testing framework for Web3 ecosystem")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable parallel test execution
    #[arg(long, default_value_t = true)]
    parallel: bool,

    /// Test timeout in seconds
    #[arg(long, default_value_t = 300)]
    timeout: u64,

    /// Number of retry attempts
    #[arg(long, default_value_t = 3)]
    retries: usize,

    /// Output format (text, json, html)
    #[arg(long, default_value = "text")]
    format: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all tests
    All,

    /// Run unit tests only
    Unit,

    /// Run integration tests only
    Integration,

    /// Run security tests
    Security,

    /// Run performance benchmarks
    Performance,

    /// Run chaos engineering tests
    Chaos,

    /// Validate production readiness
    Validate,

    /// Run specific component tests
    Component {
        /// Component name (stoq, trustchain, hypermesh, caesar, catalog)
        name: String,
    },

    /// Generate test report
    Report {
        /// Output file path
        #[arg(long, default_value = "test-report.html")]
        output: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Configure test executor
    let config = TestConfig {
        parallel_execution: cli.parallel,
        max_threads: num_cpus::get(),
        timeout: std::time::Duration::from_secs(cli.timeout),
        retry_count: cli.retries,
        capture_metrics: true,
        security_testing: true,
        performance_testing: true,
        chaos_testing: false,
    };

    let executor = TestExecutor::new(config.clone());

    match cli.command {
        Commands::All => {
            info!("Running comprehensive test suite");
            run_all_tests(executor, &cli).await?;
        }
        Commands::Unit => {
            info!("Running unit tests");
            run_unit_tests().await?;
        }
        Commands::Integration => {
            info!("Running integration tests");
            run_integration_tests().await?;
        }
        Commands::Security => {
            info!("Running security validation");
            run_security_tests().await?;
        }
        Commands::Performance => {
            info!("Running performance benchmarks");
            run_performance_tests().await?;
        }
        Commands::Chaos => {
            info!("Running chaos engineering tests");
            run_chaos_tests().await?;
        }
        Commands::Validate => {
            info!("Validating production readiness");
            run_validation().await?;
        }
        Commands::Component { name } => {
            info!("Running tests for component: {}", name);
            run_component_tests(&name).await?;
        }
        Commands::Report { output } => {
            info!("Generating test report to: {}", output);
            generate_report(&output, executor).await?;
        }
    }

    Ok(())
}

async fn run_all_tests(executor: TestExecutor, cli: &Cli) -> Result<()> {
    let report = executor.run_all_tests().await?;

    // Display results
    println!("\n{}", "=".repeat(80));
    println!("TEST EXECUTION SUMMARY");
    println!("{}", "=".repeat(80));
    println!("Total Tests: {}", report.total_tests);
    println!("Passed: {} ✓", report.passed);
    println!("Failed: {} ✗", report.failed);
    println!("Duration: {:.2}s", report.duration.as_secs_f64());
    println!("Coverage: {:.1}%", report.coverage);
    println!("{}", "=".repeat(80));

    // Display failures if any
    if report.failed > 0 {
        println!("\nFAILED TESTS:");
        for result in report.results.iter().filter(|r| !r.passed) {
            println!("  ✗ {} - {}", result.name, result.component);
            for error in &result.errors {
                println!("    └─ {error}");
            }
        }
    }

    // Display warnings
    let warnings: Vec<_> = report
        .results
        .iter()
        .filter(|r| !r.warnings.is_empty())
        .collect();

    if !warnings.is_empty() {
        println!("\nWARNINGS:");
        for result in warnings {
            println!("  ⚠ {} - {}", result.name, result.component);
            for warning in &result.warnings {
                println!("    └─ {warning}");
            }
        }
    }

    // Save report if requested
    match cli.format.as_str() {
        "html" => {
            let html = report.generate_html_report();
            fs::write("test-report.html", html)?;
            info!("HTML report saved to test-report.html");
        }
        "json" => {
            let json = serde_json::to_string_pretty(&report)?;
            fs::write("test-report.json", json)?;
            info!("JSON report saved to test-report.json");
        }
        _ => {}
    }

    // Exit with appropriate code
    if report.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

async fn run_unit_tests() -> Result<()> {
    use tokio::process::Command;

    println!("Running unit tests for all components...\n");

    let components = vec!["stoq", "trustchain", "hypermesh", "caesar", "catalog"];

    for component in components {
        println!("Testing {component}...");
        let output = Command::new("cargo")
            .args(["test", "--package", component, "--lib"])
            .output()
            .await?;

        if output.status.success() {
            println!("  ✓ {component} unit tests passed");
        } else {
            println!("  ✗ {component} unit tests failed");
            println!("{}", String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}

async fn run_integration_tests() -> Result<()> {
    println!("Running integration tests...\n");

    // TODO: Re-enable when integration test module is implemented
    println!("⚠️  Integration tests not yet implemented");

    Ok(())
}

/*
// Disabled until integration module is implemented
async fn _run_integration_tests_disabled() -> Result<()> {
    println!("Running integration tests...\n");

    let test_suites = vec![
        ("STOQ-TrustChain", integration::test_stoq_trustchain_integration()),
        ("HyperMesh-Caesar", integration::test_hypermesh_caesar_integration()),
        ("Catalog-HyperMesh", integration::test_catalog_hypermesh_integration()),
        ("Full Stack", integration::test_full_stack_integration()),
    ];

    for (name, test_future) in test_suites {
        print!("Testing {}... ", name);
        let (passed, errors) = test_future.await;

        if passed {
            println!("✓ PASSED");
        } else {
            println!("✗ FAILED");
            for error in errors {
                println!("  └─ {}", error);
            }
        }
    }

    Ok(())
}
*/

async fn run_security_tests() -> Result<()> {
    println!("Running security validation...\n");

    // Call each test individually to avoid future type mismatch
    print!("Testing Cryptography... ");
    let (passed, metrics, errors) = security::test_cryptographic_implementations().await;
    print_test_result(passed, &metrics, &errors);

    print!("Testing Quantum Resistance... ");
    let (passed, metrics, errors) = security::test_quantum_resistance().await;
    print_test_result(passed, &metrics, &errors);

    print!("Testing Byzantine Tolerance... ");
    let (passed, metrics, errors) = security::test_byzantine_fault_tolerance().await;
    print_test_result(passed, &metrics, &errors);

    print!("Testing Certificate Validation... ");
    let (passed, metrics, errors) = security::test_certificate_validation().await;
    print_test_result(passed, &metrics, &errors);

    print!("Testing Memory Safety... ");
    let (passed, metrics, errors) = security::test_memory_safety().await;
    print_test_result(passed, &metrics, &errors);

    Ok(())
}

fn print_test_result(passed: bool, metrics: &HashMap<String, f64>, errors: &[String]) {
    if passed {
        println!("✓ PASSED");
        for (key, value) in metrics {
            println!("  └─ {key}: {value:.2}");
        }
    } else {
        println!("✗ FAILED");
        for error in errors {
            println!("  └─ {error}");
        }
    }
}

async fn run_performance_tests() -> Result<()> {
    println!("Running performance benchmarks...\n");

    // Call each benchmark individually to avoid future type mismatch
    println!("Benchmarking STOQ Throughput...");
    let metrics = performance::benchmark_stoq_throughput().await;
    for (key, value) in metrics {
        println!("  └─ {key}: {value:.2}");
    }
    println!();

    println!("Benchmarking TrustChain Ops...");
    let metrics = performance::benchmark_trustchain_operations().await;
    for (key, value) in metrics {
        println!("  └─ {key}: {value:.2}");
    }
    println!();

    println!("Benchmarking Asset Operations...");
    let metrics = performance::benchmark_asset_operations().await;
    for (key, value) in metrics {
        println!("  └─ {key}: {value:.2}");
    }
    println!();

    println!("Benchmarking Consensus Latency...");
    let metrics = performance::benchmark_consensus_latency().await;
    for (key, value) in metrics {
        println!("  └─ {key}: {value:.2}");
    }
    println!();

    println!("Benchmarking Memory Usage...");
    let metrics = performance::benchmark_memory_usage().await;
    for (key, value) in metrics {
        println!("  └─ {key}: {value:.2}");
    }
    println!();

    Ok(())
}

async fn run_chaos_tests() -> Result<()> {
    println!("Running chaos engineering tests...\n");
    warn!("⚠ Chaos tests may impact system stability");

    // TODO: Re-enable when chaos module is implemented
    println!("⚠️  Chaos tests not yet implemented");

    Ok(())
}

async fn run_validation() -> Result<()> {
    println!("Validating production readiness...\n");

    // TODO: Re-enable when validation module is implemented
    println!("⚠️  Production validation not yet implemented");
    Ok(())

    /* Disabled until validation module is implemented
    let report = validation::validate_production_readiness().await;

    println!("PRODUCTION READINESS REPORT");
    println!("{}", "=".repeat(60));
    println!("Overall Score: {:.1}%", report.readiness_score);
    println!("Ready for Production: {}", if report.overall_ready { "YES ✓" } else { "NO ✗" });
    println!();

    // Code Quality
    println!("Code Quality:");
    println!("  Test Coverage: {:.1}% {}",
        report.code_quality.test_coverage,
        if report.code_quality.coverage_passed { "✓" } else { "✗" }
    );
    println!("  Complexity Score: {:.1} {}",
        report.code_quality.complexity_score,
        if report.code_quality.complexity_passed { "✓" } else { "✗" }
    );

    // Security
    println!("\nSecurity:");
    println!("  Critical Vulnerabilities: {} {}",
        report.security.critical_vulns,
        if report.security.critical_vulns == 0 { "✓" } else { "✗" }
    );
    println!("  High Vulnerabilities: {} {}",
        report.security.high_vulns,
        if report.security.high_vulns <= 3 { "✓" } else { "✗" }
    );

    // Performance
    println!("\nPerformance:");
    println!("  STOQ Throughput: {:.2} Gbps {}",
        report.performance.stoq_throughput_gbps,
        if report.performance.throughput_passed { "✓" } else { "✗" }
    );
    println!("  Max Connections: {} {}",
        report.performance.max_connections,
        if report.performance.scalability_passed { "✓" } else { "✗" }
    );

    // Blocking Issues
    if !report.blocking_issues.is_empty() {
        println!("\n⚠ BLOCKING ISSUES:");
        for issue in report.blocking_issues {
            println!("  • {}", issue);
        }
    }

    if !report.overall_ready {
        std::process::exit(1);
    }

    Ok(())
    */
}

async fn run_component_tests(component: &str) -> Result<()> {
    use tokio::process::Command;

    println!("Running tests for component: {component}\n");

    let output = Command::new("cargo")
        .args(["test", "--package", component, "--", "--nocapture"])
        .output()
        .await?;

    println!("{}", String::from_utf8_lossy(&output.stdout));

    if !output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }

    Ok(())
}

async fn generate_report(output: &str, executor: TestExecutor) -> Result<()> {
    info!("Generating comprehensive test report");

    let report = executor.run_all_tests().await?;
    let html = report.generate_html_report();

    fs::write(output, html)?;
    info!("Test report saved to: {}", output);

    // Open in browser if possible
    if cfg!(target_os = "linux") {
        let _ = std::process::Command::new("xdg-open").arg(output).spawn();
    } else if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("open").arg(output).spawn();
    }

    Ok(())
}
