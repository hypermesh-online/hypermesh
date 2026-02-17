// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

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
