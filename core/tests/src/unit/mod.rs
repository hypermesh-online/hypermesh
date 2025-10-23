//! Unit test suite for individual Nexus components
//!
//! Tests each component in isolation with mocked dependencies.

pub mod shared_tests;
pub mod transport_tests;
pub mod state_tests;
pub mod runtime_tests;
pub mod scheduler_tests;
pub mod networking_tests;
pub mod ebpf_tests;

use crate::{TestResult, init_test_logging};
use tracing::{info, error};

/// Run all unit tests
pub async fn run_all_unit_tests() -> TestResult {
    init_test_logging();
    info!("Starting unit test suite");

    let mut failed_tests = Vec::new();

    // Test each component
    let test_suites = vec![
        ("shared", shared_tests::run_shared_tests),
        ("transport", transport_tests::run_transport_tests),
        ("state", state_tests::run_state_tests),
        ("runtime", runtime_tests::run_runtime_tests),
        ("scheduler", scheduler_tests::run_scheduler_tests),
        ("networking", networking_tests::run_networking_tests),
        ("ebpf", ebpf_tests::run_ebpf_tests),
    ];

    for (component_name, test_fn) in test_suites {
        info!("Running {} unit tests", component_name);
        
        match test_fn().await {
            Ok(()) => {
                info!("✅ {} unit tests passed", component_name);
            }
            Err(e) => {
                error!("❌ {} unit tests failed: {}", component_name, e);
                failed_tests.push(component_name);
            }
        }
    }

    if failed_tests.is_empty() {
        info!("🎉 All unit tests passed!");
        Ok(())
    } else {
        Err(format!("Unit tests failed for: {}", failed_tests.join(", ")).into())
    }
}

/// Unit test helper macros
#[macro_export]
macro_rules! unit_test {
    ($test_name:ident, $component:expr, $test_body:block) => {
        #[tokio::test]
        async fn $test_name() -> TestResult {
            init_test_logging();
            tracing::info!("Running unit test: {} for {}", stringify!($test_name), $component);
            
            let result: TestResult = async $test_body.await;
            
            match &result {
                Ok(()) => tracing::info!("✅ Unit test passed: {}", stringify!($test_name)),
                Err(e) => tracing::error!("❌ Unit test failed: {}: {}", stringify!($test_name), e),
            }
            
            result
        }
    };
}

/// Mock helper for testing
pub struct MockHelper;

impl MockHelper {
    /// Create a mock node ID
    pub fn mock_node_id() -> nexus_shared::NodeId {
        nexus_shared::NodeId::random()
    }

    /// Create mock configuration
    pub fn mock_config() -> nexus_shared::NexusConfig {
        crate::test_utils::test_nexus_config()
    }

    /// Create temporary test directory
    pub fn temp_dir() -> Result<tempfile::TempDir, std::io::Error> {
        tempfile::TempDir::new()
    }
}