//! Core Asset Library Implementation
//!
//! The central component for managing asset packages in memory with
//! high-performance operations and zero-copy optimizations.

use super::types::*;
use super::cache::PackageCache;
use super::index::LibraryIndex;
use super::{LibraryConfig, LibraryInterface, LibraryMetrics, LibraryStats};
use super::{SearchQuery, PackageSummary, ValidationResult, DependencyResolution};

use anyhow::{Result, Context};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::Instant;

/// Core asset library for package collection management
pub struct AssetLibrary {
    /// Library configuration
    config: LibraryConfig,
    /// Package storage (in-memory)
    packages: Arc<RwLock<HashMap<Arc<str>, Arc<LibraryAssetPackage>>>>,
    /// Package cache for performance
    cache: Arc<PackageCache>,
    /// Search index for fast discovery
    index: Arc<LibraryIndex>,
    /// Performance metrics
    metrics: Arc<LibraryMetrics>,
}

impl AssetLibrary {
    /// Create a new asset library instance
    pub fn new() -> Self {
        Self::with_config(LibraryConfig::default())
    }

    /// Create a new asset library instance with config
    pub fn with_config(config: LibraryConfig) -> Self {
        let cache = Arc::new(PackageCache::new(
            config.l1_cache_size,
            config.l2_cache_size,
            config.l3_cache_path.clone(),
        ));

        let index = Arc::new(LibraryIndex::new());
        let metrics = Arc::new(LibraryMetrics::new());

        Self {
            config,
            packages: Arc::new(RwLock::new(HashMap::new())),
            cache,
            index,
            metrics,
        }
    }

    /// Add a package to the library
    pub async fn add_package(&self, package: LibraryAssetPackage) -> Result<()> {
        let start = Instant::now();

        // Wrap in Arc for zero-copy sharing
        let package_arc = Arc::new(package);
        let id = Arc::clone(&package_arc.id);

        // Update storage
        {
            let mut packages = self.packages.write().await;
            packages.insert(Arc::clone(&id), Arc::clone(&package_arc));
        }

        // Update cache
        if self.config.enable_cache {
            self.cache.insert(Arc::clone(&id), Arc::clone(&package_arc)).await?;
        }

        // Update index
        self.index.index_package(&package_arc).await?;

        // Record metrics
        let elapsed_us = start.elapsed().as_micros() as u64;
        self.metrics.record_operation(elapsed_us);

        Ok(())
    }

    /// Remove a package from the library
    pub async fn remove_package(&self, id: &str) -> Result<bool> {
        let start = Instant::now();
        let id_arc: Arc<str> = Arc::from(id);

        // Remove from storage
        let removed = {
            let mut packages = self.packages.write().await;
            packages.remove(&id_arc).is_some()
        };

        if removed {
            // Remove from cache
            if self.config.enable_cache {
                self.cache.remove(&id_arc).await?;
            }

            // Remove from index
            self.index.remove_package(&id_arc).await?;
        }

        // Record metrics
        let elapsed_us = start.elapsed().as_micros() as u64;
        self.metrics.record_operation(elapsed_us);

        Ok(removed)
    }

    /// Get a package by ID with caching
    async fn get_package_internal(&self, id: &str) -> Result<Option<Arc<LibraryAssetPackage>>> {
        let id_arc: Arc<str> = Arc::from(id);

        // Try cache first
        if self.config.enable_cache {
            if let Some(package) = self.cache.get(&id_arc).await? {
                self.metrics.record_cache_hit();
                return Ok(Some(package));
            }
            self.metrics.record_cache_miss();
        }

        // Fallback to storage
        let packages = self.packages.read().await;
        Ok(packages.get(&id_arc).cloned())
    }

    /// Bulk load packages for initialization
    pub async fn bulk_load(&self, packages: Vec<LibraryAssetPackage>) -> Result<()> {
        let start = Instant::now();

        // Pre-allocate capacity
        {
            let mut storage = self.packages.write().await;
            storage.reserve(packages.len());
        }

        // Process in parallel batches for performance
        use futures::stream::{self, StreamExt};
        let batch_size = 100;

        stream::iter(packages.chunks(batch_size))
            .for_each_concurrent(self.config.max_concurrent_ops, |batch| async move {
                for package in batch {
                    if let Err(e) = self.add_package(package.clone()).await {
                        eprintln!("Failed to load package {}: {}", package.id, e);
                    }
                }
            })
            .await;

        // Record metrics
        let elapsed_us = start.elapsed().as_micros() as u64;
        self.metrics.record_operation(elapsed_us);

        Ok(())
    }

    /// Optimize library for performance
    pub async fn optimize(&self) -> Result<()> {
        // Rebuild index for optimal search performance
        self.index.rebuild().await?;

        // Optimize cache tiers
        if self.config.enable_cache {
            self.cache.optimize().await?;
        }

        Ok(())
    }

    /// Create package summary from full package
    fn create_summary(package: &LibraryAssetPackage) -> PackageSummary {
        PackageSummary {
            id: package.id.to_string(),
            name: package.name.clone(),
            version: package.version.clone(),
            description: package.description.clone(),
            tags: package.metadata.as_ref()
                .map(|m| m.tags.iter().map(|t| t.to_string()).collect())
                .unwrap_or_default(),
            asset_type: package.asset_type.clone(),
            size: package.size,
            last_modified: package.metadata.as_ref()
                .map(|m| m.modified)
                .unwrap_or(0),
        }
    }

    /// Store a package for HyperMesh bridge integration
    pub async fn store_package(&self, id: String, package: crate::assets::AssetPackage) -> Result<()> {
        // Convert AssetPackage to LibraryAssetPackage
        let lib_package = self.convert_to_library_package(id, package)?;
        self.add_package(lib_package).await
    }

    /// Convert AssetPackage to LibraryAssetPackage
    fn convert_to_library_package(&self, id: String, package: crate::assets::AssetPackage) -> Result<LibraryAssetPackage> {
        let asset_type = AssetType::from_str(&package.spec.spec.asset_type)
            .unwrap_or(AssetType::Custom);

        Ok(LibraryAssetPackage {
            id: Arc::from(id.as_str()),
            name: package.spec.metadata.name.clone(),
            version: package.spec.metadata.version.clone(),
            description: package.spec.metadata.description.clone(),
            asset_type: asset_type.as_str().to_string(),
            size: package.content.main_content.len() as u64,
            hash: package.package_hash.clone(),
            content: package.content.main_content.clone(),
        })
    }
}

#[async_trait]
impl LibraryInterface for AssetLibrary {
    async fn initialize(&mut self, config: LibraryConfig) -> Result<()> {
        self.config = config;

        // Re-initialize components with new config
        self.cache = Arc::new(PackageCache::new(
            self.config.l1_cache_size,
            self.config.l2_cache_size,
            self.config.l3_cache_path.clone(),
        ));

        // Initialize cache
        self.cache.initialize().await?;

        // Initialize index
        self.index.initialize().await?;

        Ok(())
    }

    async fn get_package(&self, id: &str) -> Result<Option<LibraryAssetPackage>> {
        let start = Instant::now();

        let result = self.get_package_internal(id).await?;

        // Record metrics
        let elapsed_us = start.elapsed().as_micros() as u64;
        self.metrics.record_operation(elapsed_us);

        // Return dereferenced clone (still efficient with Arc)
        Ok(result.map(|arc| (*arc).clone()))
    }

    async fn list_packages(&self) -> Result<Vec<PackageSummary>> {
        let start = Instant::now();

        let packages = self.packages.read().await;
        let summaries: Vec<PackageSummary> = packages
            .values()
            .map(|p| Self::create_summary(p))
            .collect();

        // Record metrics
        let elapsed_us = start.elapsed().as_micros() as u64;
        self.metrics.record_operation(elapsed_us);

        Ok(summaries)
    }

    async fn search_packages(&self, query: &SearchQuery) -> Result<Vec<PackageSummary>> {
        let start = Instant::now();

        // Use index for fast search
        let package_ids = self.index.search(query).await?;

        // Fetch packages and create summaries
        let mut summaries = Vec::new();
        for id in package_ids.iter().take(query.limit) {
            if let Some(package) = self.get_package_internal(id).await? {
                summaries.push(Self::create_summary(&package));
            }
        }

        // Record metrics
        let elapsed_us = start.elapsed().as_micros() as u64;
        self.metrics.record_operation(elapsed_us);

        Ok(summaries)
    }

    async fn validate_package(&self, package: &LibraryAssetPackage) -> Result<ValidationResult> {
        let start = Instant::now();

        // Check if we have cached validation
        if let Some(validation) = &package.validation {
            if validation.expires_at > chrono::Utc::now().timestamp() {
                // Return cached validation
                return Ok(ValidationResult {
                    valid: validation.valid,
                    errors: vec![],
                    warnings: vec![],
                    security_score: validation.security_score,
                });
            }
        }

        // Perform validation (simplified for library extraction)
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check required fields
        if package.name.is_empty() {
            errors.push(super::ValidationError {
                code: "MISSING_NAME".to_string(),
                message: "Package name is required".to_string(),
                severity: super::ValidationSeverity::Error,
            });
        }

        if package.version.is_empty() {
            errors.push(super::ValidationError {
                code: "MISSING_VERSION".to_string(),
                message: "Package version is required".to_string(),
                severity: super::ValidationSeverity::Error,
            });
        }

        // Check content
        if package.size == 0 {
            warnings.push(super::ValidationWarning {
                code: "NO_CONTENT".to_string(),
                message: "Package has no content".to_string(),
            });
        }

        // Calculate security score (simplified)
        let security_score = package.spec.as_ref()
            .map(|s| match s.security.sandbox_level {
                SandboxLevel::Strict => 90,
                SandboxLevel::Standard => 70,
                SandboxLevel::None => 30,
            })
            .unwrap_or(50);

        // Record metrics
        let elapsed_us = start.elapsed().as_micros() as u64;
        self.metrics.record_operation(elapsed_us);

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
            security_score,
        })
    }

    async fn resolve_dependencies(&self, package: &LibraryAssetPackage) -> Result<DependencyResolution> {
        let start = Instant::now();

        let mut resolved = Vec::new();
        let mut conflicts = Vec::new();
        let mut missing = Vec::new();

        // Simple dependency resolution (will be enhanced by DependencyResolver)
        if let Some(spec) = &package.spec {
            for dep in spec.dependencies.iter() {
                // Check if dependency exists in library
                if let Some(dep_package) = self.get_package_internal(&dep.name).await? {
                    // Check version constraint (simplified)
                    if dep_package.version == dep.version_constraint.as_ref() {
                        resolved.push(super::ResolvedDependency {
                            name: dep.name.to_string(),
                            version: dep_package.version.clone(),
                            source: "library".to_string(),
                            dependencies: vec![], // Would recurse in full implementation
                        });
                    } else {
                        conflicts.push(super::DependencyConflict {
                            name: dep.name.to_string(),
                            versions: vec![
                                dep.version_constraint.to_string(),
                                dep_package.version.clone(),
                            ],
                            reason: "Version mismatch".to_string(),
                        });
                    }
                } else {
                    missing.push(dep.name.to_string());
                }
            }
        }

        let elapsed_us = start.elapsed().as_micros() as u64;

        Ok(DependencyResolution {
            resolved,
            conflicts,
            missing,
            success: conflicts.is_empty() && missing.is_empty(),
            resolution_time_us: elapsed_us,
        })
    }

    async fn get_stats(&self) -> Result<LibraryStats> {
        let packages = self.packages.read().await;
        let total_packages = packages.len();

        let mut stats = self.metrics.get_stats();
        stats.total_packages = total_packages;

        // Get cache-specific stats
        if self.config.enable_cache {
            let cache_stats = self.cache.get_stats().await?;
            stats.l1_hits = cache_stats.l1_hits;
            stats.l2_hits = cache_stats.l2_hits;
            stats.l3_hits = cache_stats.l3_hits;
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_asset_library_creation() {
        let library = AssetLibrary::new();

        let stats = library.get_stats().await.unwrap();
        assert_eq!(stats.total_packages, 0);
    }

    #[tokio::test]
    async fn test_add_and_get_package() {
        let library = AssetLibrary::new();

        // Create test package
        let package = create_test_package();
        let package_id = package.id.to_string();

        // Add package
        library.add_package(package).await.unwrap();

        // Get package
        let retrieved = library.get_package(&package_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id.as_ref(), package_id);

        // Check stats
        let stats = library.get_stats().await.unwrap();
        assert_eq!(stats.total_packages, 1);
    }

    #[tokio::test]
    async fn test_search_packages() {
        let library = AssetLibrary::new();

        // Add test packages
        for i in 0..5 {
            let mut package = create_test_package();
            package.id = Arc::from(format!("test-{}", i));
            package.name = format!("Package {}", i);
            library.add_package(package).await.unwrap();
        }

        // Search packages
        let query = SearchQuery {
            query: "Package".to_string(),
            tags: vec![],
            asset_type: None,
            author: None,
            limit: 10,
            offset: 0,
        };

        let results = library.search_packages(&query).await.unwrap();
        assert_eq!(results.len(), 5);
    }

    fn create_test_package() -> LibraryAssetPackage {
        LibraryAssetPackage {
            id: Arc::from("test-package"),
            name: "Test Package".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A test package".to_string()),
            asset_type: "julia".to_string(),
            size: 1024,
            hash: "package-hash".to_string(),
            content: "test content".to_string(),
            metadata: None,
            spec: None,
            content_refs: None,
            validation: None,
        }
    }
}