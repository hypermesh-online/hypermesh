//! Staging deployment runner
//!
//! Deploys and tests a complete Hypermesh Nexus staging environment

use nexus_tests::staging::{deploy_and_test_staging};
use nexus_tests::{init_test_logging, TestResult};
use tracing::{info, error};

#[tokio::main]
async fn main() -> TestResult {
    init_test_logging();
    
    info!("🚀 Starting Hypermesh Nexus staging deployment");
    
    match deploy_and_test_staging().await {
        Ok(()) => {
            info!("✅ Staging deployment completed successfully");
            std::process::exit(0);
        },
        Err(e) => {
            error!("❌ Staging deployment failed: {}", e);
            std::process::exit(1);
        }
    }
}