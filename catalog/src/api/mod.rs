// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog STOQ API layer
//!
//! Provides the catalog.hypermesh.online API surface over STOQ protocol.
//! Mirrors Caesar's STOQ API pattern with catalog-specific handlers.

pub mod stoq_api;

pub use stoq_api::*;
