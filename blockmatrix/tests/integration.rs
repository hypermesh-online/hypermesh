// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! BlockMatrix integration tests
//!
//! Test modules organized in tests/integration/ directory
//!
//! Gated behind `future-tests` feature: these tests reference APIs
//! that have not been implemented yet (extension lifecycle, catalog plugin operations).

#![cfg(feature = "future-tests")]

#[path = "integration/catalog_plugin/mod.rs"]
mod catalog_plugin;
