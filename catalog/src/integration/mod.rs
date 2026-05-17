// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration adapters for downstream consumers.
//!
//! This module hosts the bridge between catalog-crate types and the
//! narrow trait surfaces exposed by other crates (e.g. blockmatrix's
//! `CatalogProvider`). Because catalog already depends on blockmatrix,
//! these adapters can only live here — putting them in blockmatrix
//! would form a dependency cycle.

pub mod blockmatrix_adapter;

pub use blockmatrix_adapter::{wire_catalog_registry, CatalogRegistryAdapter};
