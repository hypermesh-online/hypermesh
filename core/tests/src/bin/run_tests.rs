//! Simple test runner that doesn't depend on transport layer
//!
//! Runs the core test suites we've built

use nexus_integration_tests::{
    deployment, metrics, staging,
    init_test_logging, TestResult
};
use tracing::{info, error};

#[tokio::main]
async fn main() -> TestResult {
    init_test_logging();
    
    info!("🚀 Starting Hypermesh Nexus comprehensive test suite");
    
    // Run deployment tests
    info!("🚀 Running deployment tests...");
    match deployment::run_all_deployment_tests().await {
        Ok(()) => info!("✅ Deployment tests passed"),
        Err(e) => {
            error!("❌ Deployment tests failed: {}", e);
            return Err(e);
        }
    }
    
    // Run metrics tests
    info!("📊 Running metrics and analytics tests...");
    match metrics::run_metrics_tests().await {
        Ok(()) => info!("✅ Metrics tests passed"),
        Err(e) => {
            error!("❌ Metrics tests failed: {}", e);
            return Err(e);
        }
    }
    
    // Run staging deployment
    info!("🎯 Running staging deployment and integration tests...");
    match staging::deploy_and_test_staging().await {
        Ok(()) => info!("✅ Staging deployment and tests passed"),
        Err(e) => {
            error!("❌ Staging tests failed: {}", e);
            return Err(e);
        }
    }
    
    info!("🎉 ALL TESTS PASSED! Hypermesh Nexus test suite completed successfully");
    info!("📊 Test Summary:");
    info!("  ✅ Deployment validation: PASSED");
    info!("  ✅ Metrics and analytics: PASSED"); 
    info!("  ✅ Staging integration: PASSED");
    
    Ok(())
}