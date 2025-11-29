//! Deployment testing suite for bare metal/VM Nexus deployments
//!
//! Tests the actual deployment of Nexus nodes across multiple environments

pub mod bare_metal_tests;
pub mod vm_tests;
pub mod cluster_tests;
pub mod network_tests;
pub mod performance_tests;

use crate::{TestResult, init_test_logging};
use tracing::{info, error};

/// Run all deployment tests
pub async fn run_all_deployment_tests() -> TestResult {
    init_test_logging();
    info!("Starting deployment test suite");

    let mut failed_tests = Vec::new();

    // Test different deployment scenarios
    let test_suites = vec![
        ("bare_metal", bare_metal_tests::run_bare_metal_tests),
        ("vm", vm_tests::run_vm_tests),
        ("cluster", cluster_tests::run_cluster_tests),
        ("network", network_tests::run_network_tests),
        ("performance", performance_tests::run_performance_tests),
    ];

    for (test_name, test_fn) in test_suites {
        info!("Running {} deployment tests", test_name);
        
        match test_fn().await {
            Ok(()) => {
                info!("✅ {} deployment tests passed", test_name);
            }
            Err(e) => {
                error!("❌ {} deployment tests failed: {}", test_name, e);
                failed_tests.push(test_name);
            }
        }
    }

    if failed_tests.is_empty() {
        info!("🎉 All deployment tests passed!");
        Ok(())
    } else {
        Err(format!("Deployment tests failed for: {}", failed_tests.join(", ")).into())
    }
}