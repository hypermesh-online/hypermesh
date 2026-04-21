// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Shared application state and configuration for the Catalog STOQ API.

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

/// Shared application state for Catalog STOQ API handlers.
pub struct CatalogAppState {
    /// Service name
    pub service_name: String,
    /// Catalog version
    pub version: String,
    /// Package count (atomic counter)
    pub package_count: Arc<std::sync::atomic::AtomicU64>,
    /// Publisher count
    pub publisher_count: Arc<std::sync::atomic::AtomicU64>,
    /// Total downloads
    pub total_downloads: Arc<std::sync::atomic::AtomicU64>,
    /// Catalog registry for real lookups (optional for backward compat)
    pub registry: Option<crate::registry::CatalogRegistry>,
}

impl CatalogAppState {
    /// Create new state with defaults
    pub fn new() -> Self {
        Self {
            service_name: "catalog".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            package_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            publisher_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_downloads: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            registry: None,
        }
    }

    /// Create new state with a CatalogRegistry for real lookups
    pub fn with_registry(registry: crate::registry::CatalogRegistry) -> Self {
        Self {
            service_name: "catalog".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            package_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            publisher_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_downloads: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            registry: Some(registry),
        }
    }

    /// Update package count
    pub fn set_package_count(&self, count: u64) {
        self.package_count
            .store(count, std::sync::atomic::Ordering::Relaxed);
    }

    /// Update publisher count
    pub fn set_publisher_count(&self, count: u64) {
        self.publisher_count
            .store(count, std::sync::atomic::Ordering::Relaxed);
    }

    /// Increment download count
    pub fn increment_downloads(&self) {
        self.total_downloads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Sync atomic counters from registry statistics.
    pub async fn sync_from_registry(&self) {
        if let Some(ref registry) = self.registry {
            let stats = registry.get_statistics().await;
            self.set_package_count(stats.total_types as u64);
        }
    }
}

impl Default for CatalogAppState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Catalog STOQ API configuration
#[derive(Debug, Clone)]
pub struct CatalogStoqConfig {
    /// STOQ bind address (IPv6)
    pub bind_address: String,
    /// Service name
    pub service_name: String,
    /// Enable request logging
    pub enable_logging: bool,
}

impl Default for CatalogStoqConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::1]:9295".to_string(),
            service_name: "catalog".to_string(),
            enable_logging: true,
        }
    }
}
