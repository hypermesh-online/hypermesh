// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration Test for Blockchain-Native Compute System
//!
//! Tests the complete end-to-end flow of blockchain-native compute execution
//! based on Proof of State patterns without smart contract abstraction.
//!
//! NOTE: These tests depend on the external `hypermesh_catalog` crate which
//! is not a direct dependency of blockmatrix. They are preserved as reference
//! tests for when the catalog integration is wired up via STOQ.

// Tests disabled: hypermesh_catalog is not a blockmatrix dependency.
// The catalog crate (package name "catalog") provides its own tests.
// BlockMatrix interacts with Catalog via STOQ protocol, not direct linking.
//
// When catalog integration tests are needed, they should be added as
// integration tests that communicate via STOQ transport, not direct imports.
