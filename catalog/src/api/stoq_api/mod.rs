// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog STOQ API — package registry operations over STOQ protocol.
//!
//! Provides the catalog.hypermesh.online API surface for package browsing,
//! searching, package details, publisher info, registry stats, and health.
//!
//! All handlers hold a shared [`CatalogAppState`] wrapping the catalog's
//! registry and reputation system behind async-aware locks.

mod config_state;
mod handlers;
mod message_types;
mod server;

pub use config_state::{CatalogAppState, CatalogStoqConfig};
pub use handlers::{
    BrowseHandler, CatalogHealthHandler, GetPackageHandler, GetPublisherHandler,
    RegistryStatsHandler, SearchHandler, TypeLookupHandler, TypePublishHandler,
};
pub use message_types::{
    BrowseRequest, BrowseResponse, GetPackageRequest, GetPackageResponse, GetPublisherRequest,
    GetPublisherResponse, HealthResponse, PackageSummary, RegistryStatsResponse, SearchRequest,
    SearchResponse, TypeLookupRequest, TypeLookupResponse, TypePublishRequest, TypePublishResponse,
};
pub use server::CatalogStoqApi;
