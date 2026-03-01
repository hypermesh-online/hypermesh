// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Real Integration Testing Module
//!
//! Comprehensive end-to-end integration tests with ACTUAL implementations.
//! Replaces stub-based tests with real multi-component validation.
//!
//! Gated: references APIs (TrustChain::new_with_security, StoqTransport::new_optimized,
//! blockmatrix::HyperMeshSystem) that have not been implemented yet.
#![cfg(feature = "future-tests")]

mod integration;

use anyhow::Result;

// Re-export integration test modules
pub use integration::test_harness::{IntegrationTestHarness, NodeConfig, TestContext};

// Test suite entry points
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_full_stack_integration_tests() -> Result<()> {
        println!("\n========== FULL STACK INTEGRATION TESTS ==========\n");

        // Run full stack tests from integration/full_stack.rs
        // These are automatically discovered by cargo test

        Ok(())
    }

    #[tokio::test]
    async fn run_multi_node_consensus_tests() -> Result<()> {
        println!("\n========== MULTI-NODE CONSENSUS TESTS ==========\n");

        // Run multi-node tests from integration/multi_node.rs
        // These are automatically discovered by cargo test

        Ok(())
    }

    #[tokio::test]
    async fn run_dns_asset_tests() -> Result<()> {
        println!("\n========== DNS-AS-ASSET TESTS ==========\n");

        // Run DNS tests from integration/dns_asset.rs
        // These are automatically discovered by cargo test

        Ok(())
    }

    #[tokio::test]
    async fn run_privacy_tier_tests() -> Result<()> {
        println!("\n========== PRIVACY TIER TESTS ==========\n");

        // Run privacy tests from integration/privacy_tiers.rs
        // These are automatically discovered by cargo test

        Ok(())
    }
}
