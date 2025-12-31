//! Integration Testing Framework
//!
//! Comprehensive end-to-end testing validating components work together.
//! NO STUBS - all tests perform actual operations.

pub mod test_harness;
pub mod full_stack;
pub mod multi_node;
pub mod dns_asset;
pub mod privacy_tiers;

pub use test_harness::{IntegrationTestHarness, TestContext, NodeConfig};
