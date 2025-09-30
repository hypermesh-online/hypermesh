//! HyperMesh Asset Library Core Components
//!
//! Lightweight, high-performance asset library functionality extracted from
//! the standalone Catalog service for integration with HyperMesh plugin architecture.
//!
//! ARCHITECTURE:
//! - Zero dependencies on service infrastructure
//! - In-process operation optimized for performance
//! - Clean interfaces for HyperMesh AssetManager integration
//! - Multi-tier caching (L1/L2/L3) for optimal performance

pub mod asset_library;
pub mod package_manager;
pub mod index;
pub mod cache;
pub mod resolver;
pub mod types;

// Re-export core types for convenience
pub use asset_library::AssetLibrary;
pub use package_manager::AssetPackageManager;
pub use index::LibraryIndex;
pub use cache::{PackageCache, CacheLayer};
pub use resolver::DependencyResolver;
pub use types::*;

use anyhow::Result;
use std::sync::Arc;
use async_trait::async_trait;

/// Library configuration for HyperMesh integration
#[derive(Debug, Clone)]
pub struct LibraryConfig {
    /// Enable multi-tier caching
    pub enable_cache: bool,
    /// L1 cache size (in-memory, hot assets)
    pub l1_cache_size: usize,
    /// L2 cache size (in-memory, warm assets)
    pub l2_cache_size: usize,
    /// L3 cache path (disk-based, cold assets)
    pub l3_cache_path: Option<String>,
    /// Enable zero-copy optimizations
    pub enable_zero_copy: bool,
    /// Maximum concurrent operations
    pub max_concurrent_ops: usize,
    /// Enable performance metrics collection
    pub enable_metrics: bool,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            l1_cache_size: 100,  // Hot assets
            l2_cache_size: 1000, // Warm assets
            l3_cache_path: Some("/var/cache/catalog".to_string()),
            enable_zero_copy: true,
            max_concurrent_ops: 100,
            enable_metrics: true,
        }
    }
}

/// Core library interface for HyperMesh integration
#[async_trait]
pub trait LibraryInterface: Send + Sync {
    /// Initialize the library with configuration
    async fn initialize(&mut self, config: LibraryConfig) -> Result<()>;

    /// Get an asset package by ID
    async fn get_package(&self, id: &str) -> Result<Option<LibraryAssetPackage>>;

    /// List all available packages
    async fn list_packages(&self) -> Result<Vec<PackageSummary>>;

    /// Search packages by query
    async fn search_packages(&self, query: &SearchQuery) -> Result<Vec<PackageSummary>>;

    /// Validate a package
    async fn validate_package(&self, package: &LibraryAssetPackage) -> Result<ValidationResult>;

    /// Resolve dependencies for a package
    async fn resolve_dependencies(&self, package: &LibraryAssetPackage) -> Result<DependencyResolution>;

    /// Get library statistics
    async fn get_stats(&self) -> Result<LibraryStats>;
}

/// Library statistics for monitoring
#[derive(Debug, Clone)]
pub struct LibraryStats {
    /// Total packages in library
    pub total_packages: usize,
    /// Cache hit rate (percentage)
    pub cache_hit_rate: f64,
    /// L1 cache hits
    pub l1_hits: u64,
    /// L2 cache hits
    pub l2_hits: u64,
    /// L3 cache hits
    pub l3_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Average operation latency (microseconds)
    pub avg_latency_us: u64,
    /// Total operations processed
    pub total_operations: u64,
}

/// Search query for package discovery
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Text search terms
    pub query: String,
    /// Filter by tags
    pub tags: Vec<String>,
    /// Filter by asset type
    pub asset_type: Option<String>,
    /// Filter by author
    pub author: Option<String>,
    /// Maximum results to return
    pub limit: usize,
    /// Offset for pagination
    pub offset: usize,
}

/// Package summary for lightweight operations
#[derive(Debug, Clone)]
pub struct PackageSummary {
    /// Package ID
    pub id: String,
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description
    pub description: Option<String>,
    /// Package tags
    pub tags: Vec<String>,
    /// Asset type
    pub asset_type: String,
    /// Package size in bytes
    pub size: u64,
    /// Last modified timestamp
    pub last_modified: i64,
}

/// Validation result from package validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the package is valid
    pub valid: bool,
    /// List of validation errors
    pub errors: Vec<ValidationError>,
    /// List of validation warnings
    pub warnings: Vec<ValidationWarning>,
    /// Security score (0-100)
    pub security_score: u32,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Severity level
    pub severity: ValidationSeverity,
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,
    /// Warning message
    pub message: String,
}

/// Validation severity levels
#[derive(Debug, Clone, Copy)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// Dependency resolution results
#[derive(Debug, Clone)]
pub struct DependencyResolution {
    /// Successfully resolved dependencies
    pub resolved: Vec<ResolvedDependency>,
    /// Dependency conflicts
    pub conflicts: Vec<DependencyConflict>,
    /// Missing dependencies
    pub missing: Vec<String>,
    /// Whether resolution was successful
    pub success: bool,
    /// Resolution time in microseconds
    pub resolution_time_us: u64,
}

/// Resolved dependency information
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    /// Dependency name
    pub name: String,
    /// Resolved version
    pub version: String,
    /// Source of the dependency
    pub source: String,
    /// Transitive dependencies
    pub dependencies: Vec<ResolvedDependency>,
}

/// Dependency conflict information
#[derive(Debug, Clone)]
pub struct DependencyConflict {
    /// Conflicting dependency name
    pub name: String,
    /// Conflicting versions
    pub versions: Vec<String>,
    /// Conflict reason
    pub reason: String,
}

/// Library performance metrics
#[derive(Debug, Default)]
pub struct LibraryMetrics {
    total_operations: u64,
    total_latency_us: u64,
    cache_hits: u64,
    cache_misses: u64,
}

impl LibraryMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an operation
    pub fn record_operation(&self, _latency_us: u64) {
        // Thread-safe implementation would use atomics
    }

    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        // Thread-safe implementation would use atomics
    }

    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        // Thread-safe implementation would use atomics
    }

    /// Get current stats
    pub fn get_stats(&self) -> LibraryStats {
        LibraryStats {
            total_packages: 0,
            cache_hit_rate: 0.0,
            l1_hits: 0,
            l2_hits: 0,
            l3_hits: 0,
            cache_misses: self.cache_misses,
            avg_latency_us: 0,
            total_operations: self.total_operations,
        }
    }
    /// Successfully resolved dependencies
    pub resolved: Vec<ResolvedDependency>,
    /// Conflicting dependencies
    pub conflicts: Vec<DependencyConflict>,
    /// Missing dependencies
    pub missing: Vec<String>,
    /// Resolution successful
    pub success: bool,
    /// Resolution time in microseconds
    pub resolution_time_us: u64,
}

/// Resolved dependency information
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    /// Package name
    pub name: String,
    /// Resolved version
    pub version: String,
    /// Source location
    pub source: String,
    /// Transitive dependencies
    pub dependencies: Vec<ResolvedDependency>,
}

/// Dependency conflict information
#[derive(Debug, Clone)]
pub struct DependencyConflict {
    /// Package name
    pub name: String,
    /// Conflicting versions
    pub versions: Vec<String>,
    /// Conflict reason
    pub reason: String,
}

/// Validation result for package integrity
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Validation passed
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
    /// Security score (0-100)
    pub security_score: u32,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Severity level
    pub severity: ValidationSeverity,
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,
    /// Warning message
    pub message: String,
}

/// Validation severity levels
#[derive(Debug, Clone)]
pub enum ValidationSeverity {
    Critical,
    Error,
    Warning,
    Info,
}

/// Performance metrics for monitoring
pub struct LibraryMetrics {
    /// Operation counter
    operations: Arc<std::sync::atomic::AtomicU64>,
    /// Cache hit counter
    cache_hits: Arc<std::sync::atomic::AtomicU64>,
    /// Cache miss counter
    cache_misses: Arc<std::sync::atomic::AtomicU64>,
    /// Total latency accumulator (for averaging)
    total_latency_us: Arc<std::sync::atomic::AtomicU64>,
}

impl LibraryMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        use std::sync::atomic::AtomicU64;
        Self {
            operations: Arc::new(AtomicU64::new(0)),
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
            total_latency_us: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record an operation
    pub fn record_operation(&self, latency_us: u64) {
        use std::sync::atomic::Ordering;
        self.operations.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }

    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        use std::sync::atomic::Ordering;
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        use std::sync::atomic::Ordering;
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current statistics
    pub fn get_stats(&self) -> LibraryStats {
        use std::sync::atomic::Ordering;
        let ops = self.operations.load(Ordering::Relaxed);
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total_latency = self.total_latency_us.load(Ordering::Relaxed);

        let cache_hit_rate = if hits + misses > 0 {
            (hits as f64 / (hits + misses) as f64) * 100.0
        } else {
            0.0
        };

        let avg_latency = if ops > 0 {
            total_latency / ops
        } else {
            0
        };

        LibraryStats {
            total_packages: 0, // Will be filled by implementation
            cache_hit_rate,
            l1_hits: 0, // Will be filled by cache implementation
            l2_hits: 0,
            l3_hits: 0,
            cache_misses: misses,
            avg_latency_us: avg_latency,
            total_operations: ops,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_config_default() {
        let config = LibraryConfig::default();
        assert!(config.enable_cache);
        assert_eq!(config.l1_cache_size, 100);
        assert_eq!(config.l2_cache_size, 1000);
        assert!(config.enable_zero_copy);
    }

    #[test]
    fn test_metrics_recording() {
        let metrics = LibraryMetrics::new();

        metrics.record_operation(100);
        metrics.record_operation(200);
        metrics.record_cache_hit();
        metrics.record_cache_miss();

        let stats = metrics.get_stats();
        assert_eq!(stats.total_operations, 2);
        assert_eq!(stats.avg_latency_us, 150);
        assert_eq!(stats.cache_hit_rate, 50.0);
    }
}