//! Asset Package Manager
//!
//! Handles package lifecycle operations including installation, updates, and removal.

use super::types::*;
use super::asset_library::AssetLibrary;
use super::resolver::DependencyResolver;
use super::{LibraryConfig, ValidationResult, DependencyResolution};

use anyhow::{Result, Context, bail};
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

/// Package operation results
#[derive(Debug, Clone)]
pub struct OperationResult {
    /// Operation success
    pub success: bool,
    /// Packages affected
    pub packages_affected: Vec<String>,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
    /// Operation duration in milliseconds
    pub duration_ms: u64,
}

/// Package manager for lifecycle operations
pub struct AssetPackageManager {
    /// Core library
    library: Arc<AssetLibrary>,
    /// Dependency resolver
    resolver: Arc<DependencyResolver>,
    /// Installed packages tracking
    installed: Arc<RwLock<HashMap<Arc<str>, InstalledPackage>>>,
    /// Installation locks to prevent concurrent installs
    install_locks: Arc<RwLock<HashSet<Arc<str>>>>,
    /// Package manager configuration
    config: PackageManagerConfig,
}

/// Installed package information
#[derive(Debug, Clone)]
struct InstalledPackage {
    /// Package ID
    id: Arc<str>,
    /// Installed version
    version: Arc<str>,
    /// Installation timestamp
    installed_at: i64,
    /// Dependencies installed with this package
    dependencies: Vec<Arc<str>>,
    /// Packages that depend on this one
    dependents: HashSet<Arc<str>>,
    /// Installation source
    source: InstallSource,
}

/// Installation source
#[derive(Debug, Clone)]
enum InstallSource {
    /// From library
    Library,
    /// From remote registry
    Registry(String),
    /// From local file
    Local(String),
    /// As dependency
    Dependency(Arc<str>),
}

/// Package manager configuration
#[derive(Debug, Clone)]
pub struct PackageManagerConfig {
    /// Auto-install dependencies
    pub auto_install_deps: bool,
    /// Validate packages before installation
    pub validate_before_install: bool,
    /// Keep old versions when updating
    pub keep_old_versions: bool,
    /// Maximum dependency depth
    pub max_dependency_depth: u32,
    /// Allow pre-release versions
    pub allow_prerelease: bool,
}

impl Default for PackageManagerConfig {
    fn default() -> Self {
        Self {
            auto_install_deps: true,
            validate_before_install: true,
            keep_old_versions: false,
            max_dependency_depth: 10,
            allow_prerelease: false,
        }
    }
}

impl AssetPackageManager {
    /// Create a new package manager
    pub fn new(library: Arc<AssetLibrary>, config: PackageManagerConfig) -> Self {
        let resolver = Arc::new(DependencyResolver::new());

        Self {
            library,
            resolver,
            installed: Arc::new(RwLock::new(HashMap::new())),
            install_locks: Arc::new(RwLock::new(HashSet::new())),
            config,
        }
    }

    /// Install a package
    pub async fn install_package(&self, package_id: &str) -> Result<OperationResult> {
        let start = std::time::Instant::now();
        let mut result = OperationResult {
            success: false,
            packages_affected: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms: 0,
        };

        // Check if already installing
        {
            let locks = self.install_locks.read().await;
            if locks.contains(package_id) {
                result.errors.push(format!("Package {} is already being installed", package_id));
                return Ok(result);
            }
        }

        // Acquire install lock
        {
            let mut locks = self.install_locks.write().await;
            locks.insert(Arc::from(package_id));
        }

        // Ensure we release the lock
        let _guard = InstallLockGuard {
            locks: Arc::clone(&self.install_locks),
            package_id: Arc::from(package_id),
        };

        // Get package from library
        let package = match self.library.get_package(package_id).await? {
            Some(pkg) => pkg,
            None => {
                result.errors.push(format!("Package {} not found", package_id));
                return Ok(result);
            }
        };

        // Validate if required
        if self.config.validate_before_install {
            let validation = self.library.validate_package(&package).await?;
            if !validation.valid {
                result.errors.push(format!(
                    "Package {} failed validation: {:?}",
                    package_id, validation.errors
                ));
                return Ok(result);
            }
        }

        // Resolve dependencies
        let mut packages_to_install = vec![(Arc::from(package_id), package)];

        if self.config.auto_install_deps {
            let dep_resolution = self.resolve_all_dependencies(&packages_to_install[0].1).await?;

            if !dep_resolution.success {
                result.errors.push(format!(
                    "Dependency resolution failed: {} conflicts, {} missing",
                    dep_resolution.conflicts.len(),
                    dep_resolution.missing.len()
                ));
                return Ok(result);
            }

            // Add resolved dependencies to install list
            for dep in dep_resolution.resolved {
                if let Some(dep_package) = self.library.get_package(&dep.name).await? {
                    packages_to_install.push((Arc::from(dep.name.as_str()), dep_package));
                }
            }
        }

        // Install all packages
        let mut installed_packages = self.installed.write().await;

        for (pkg_id, pkg) in packages_to_install {
            // Check if already installed
            if installed_packages.contains_key(&pkg_id) {
                result.warnings.push(format!("Package {} is already installed", pkg_id));
                continue;
            }

            // Create installation record
            let installed = InstalledPackage {
                id: Arc::clone(&pkg_id),
                version: Arc::clone(&pkg.metadata.version),
                installed_at: chrono::Utc::now().timestamp(),
                dependencies: pkg.spec.dependencies
                    .iter()
                    .map(|d| Arc::clone(&d.name))
                    .collect(),
                dependents: HashSet::new(),
                source: if &pkg_id == package_id {
                    InstallSource::Library
                } else {
                    InstallSource::Dependency(Arc::from(package_id))
                },
            };

            // Update dependents for dependencies
            for dep_name in &installed.dependencies {
                if let Some(dep_installed) = installed_packages.get_mut(dep_name) {
                    dep_installed.dependents.insert(Arc::clone(&pkg_id));
                }
            }

            installed_packages.insert(Arc::clone(&pkg_id), installed);
            result.packages_affected.push(pkg_id.to_string());
        }

        result.success = true;
        result.duration_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }

    /// Uninstall a package
    pub async fn uninstall_package(&self, package_id: &str) -> Result<OperationResult> {
        let start = std::time::Instant::now();
        let mut result = OperationResult {
            success: false,
            packages_affected: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms: 0,
        };

        let mut installed = self.installed.write().await;

        // Check if installed
        let package_info = match installed.get(package_id) {
            Some(info) => info.clone(),
            None => {
                result.errors.push(format!("Package {} is not installed", package_id));
                return Ok(result);
            }
        };

        // Check for dependents
        if !package_info.dependents.is_empty() {
            result.errors.push(format!(
                "Cannot uninstall {}: {} packages depend on it",
                package_id,
                package_info.dependents.len()
            ));
            return Ok(result);
        }

        // Remove package
        installed.remove(package_id);
        result.packages_affected.push(package_id.to_string());

        // Update dependents tracking
        for dep_name in &package_info.dependencies {
            if let Some(dep_installed) = installed.get_mut(dep_name) {
                dep_installed.dependents.remove(package_id);
            }
        }

        // Optionally uninstall orphaned dependencies
        let orphans = self.find_orphaned_dependencies(&installed);
        if !orphans.is_empty() {
            result.warnings.push(format!(
                "{} orphaned dependencies can be removed: {:?}",
                orphans.len(),
                orphans
            ));
        }

        result.success = true;
        result.duration_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }

    /// Update a package to latest version
    pub async fn update_package(&self, package_id: &str) -> Result<OperationResult> {
        let start = std::time::Instant::now();
        let mut result = OperationResult {
            success: false,
            packages_affected: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms: 0,
        };

        // Check if installed
        let current_version = {
            let installed = self.installed.read().await;
            match installed.get(package_id) {
                Some(info) => info.version.to_string(),
                None => {
                    result.errors.push(format!("Package {} is not installed", package_id));
                    return Ok(result);
                }
            }
        };

        // Get latest version from library
        let latest_package = match self.library.get_package(package_id).await? {
            Some(pkg) => pkg,
            None => {
                result.errors.push(format!("Package {} not found in library", package_id));
                return Ok(result);
            }
        };

        // Check if update is needed
        if latest_package.metadata.version.as_ref() == current_version {
            result.warnings.push(format!(
                "Package {} is already at latest version {}",
                package_id, current_version
            ));
            result.success = true;
            return Ok(result);
        }

        // Uninstall old version (if not keeping)
        if !self.config.keep_old_versions {
            let uninstall_result = self.uninstall_package(package_id).await?;
            if !uninstall_result.success {
                result.errors = uninstall_result.errors;
                return Ok(result);
            }
        }

        // Install new version
        let install_result = self.install_package(package_id).await?;
        result.success = install_result.success;
        result.packages_affected = install_result.packages_affected;
        result.errors = install_result.errors;
        result.warnings = install_result.warnings;

        if result.success {
            result.warnings.push(format!(
                "Updated {} from {} to {}",
                package_id,
                current_version,
                latest_package.metadata.version
            ));
        }

        result.duration_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }

    /// List installed packages
    pub async fn list_installed(&self) -> Result<Vec<InstalledPackageInfo>> {
        let installed = self.installed.read().await;

        let mut packages: Vec<InstalledPackageInfo> = installed
            .values()
            .map(|pkg| InstalledPackageInfo {
                id: pkg.id.to_string(),
                version: pkg.version.to_string(),
                installed_at: pkg.installed_at,
                dependencies: pkg.dependencies.iter().map(|d| d.to_string()).collect(),
                dependents: pkg.dependents.iter().map(|d| d.to_string()).collect(),
            })
            .collect();

        packages.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(packages)
    }

    /// Check package dependencies
    pub async fn check_dependencies(&self, package_id: &str) -> Result<DependencyCheckResult> {
        let package = match self.library.get_package(package_id).await? {
            Some(pkg) => pkg,
            None => bail!("Package {} not found", package_id),
        };

        let resolution = self.library.resolve_dependencies(&package).await?;

        Ok(DependencyCheckResult {
            package_id: package_id.to_string(),
            total_dependencies: resolution.resolved.len(),
            missing: resolution.missing,
            conflicts: resolution.conflicts
                .into_iter()
                .map(|c| DependencyConflictInfo {
                    name: c.name,
                    versions: c.versions,
                    reason: c.reason,
                })
                .collect(),
            satisfied: resolution.success,
        })
    }

    /// Find orphaned dependencies (not required by any package)
    fn find_orphaned_dependencies(&self, installed: &HashMap<Arc<str>, InstalledPackage>) -> Vec<String> {
        installed
            .values()
            .filter(|pkg| {
                pkg.dependents.is_empty() &&
                matches!(pkg.source, InstallSource::Dependency(_))
            })
            .map(|pkg| pkg.id.to_string())
            .collect()
    }

    /// Resolve all dependencies recursively
    async fn resolve_all_dependencies(&self, package: &LibraryAssetPackage) -> Result<DependencyResolution> {
        self.resolver.resolve_full(
            package,
            &*self.library,
            self.config.max_dependency_depth,
        ).await
    }

    /// Clean orphaned packages
    pub async fn clean_orphaned(&self) -> Result<OperationResult> {
        let start = std::time::Instant::now();
        let mut result = OperationResult {
            success: true,
            packages_affected: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms: 0,
        };

        let orphans = {
            let installed = self.installed.read().await;
            self.find_orphaned_dependencies(&installed)
        };

        for orphan in orphans {
            let uninstall_result = self.uninstall_package(&orphan).await?;
            if uninstall_result.success {
                result.packages_affected.push(orphan);
            } else {
                result.errors.extend(uninstall_result.errors);
            }
        }

        result.duration_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }
}

/// Public installed package information
#[derive(Debug, Clone)]
pub struct InstalledPackageInfo {
    pub id: String,
    pub version: String,
    pub installed_at: i64,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
}

/// Dependency check result
#[derive(Debug, Clone)]
pub struct DependencyCheckResult {
    pub package_id: String,
    pub total_dependencies: usize,
    pub missing: Vec<String>,
    pub conflicts: Vec<DependencyConflictInfo>,
    pub satisfied: bool,
}

/// Dependency conflict information
#[derive(Debug, Clone)]
pub struct DependencyConflictInfo {
    pub name: String,
    pub versions: Vec<String>,
    pub reason: String,
}

/// Guard for releasing install locks
struct InstallLockGuard {
    locks: Arc<RwLock<HashSet<Arc<str>>>>,
    package_id: Arc<str>,
}

impl Drop for InstallLockGuard {
    fn drop(&mut self) {
        let locks = Arc::clone(&self.locks);
        let package_id = Arc::clone(&self.package_id);

        tokio::spawn(async move {
            let mut locks = locks.write().await;
            locks.remove(&package_id);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::asset_library::AssetLibrary;

    #[tokio::test]
    async fn test_package_manager_creation() {
        let library = Arc::new(AssetLibrary::new(LibraryConfig::default()));
        let manager = AssetPackageManager::new(library, PackageManagerConfig::default());

        let installed = manager.list_installed().await.unwrap();
        assert_eq!(installed.len(), 0);
    }

    #[tokio::test]
    async fn test_install_and_uninstall() {
        let library = Arc::new(AssetLibrary::new(LibraryConfig::default()));

        // Add a test package to the library
        let package = create_test_package("test-pkg");
        library.add_package(package).await.unwrap();

        let manager = AssetPackageManager::new(Arc::clone(&library), PackageManagerConfig::default());

        // Install package
        let result = manager.install_package("test-pkg").await.unwrap();
        assert!(result.success);
        assert_eq!(result.packages_affected.len(), 1);

        // Check it's installed
        let installed = manager.list_installed().await.unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].id, "test-pkg");

        // Uninstall package
        let result = manager.uninstall_package("test-pkg").await.unwrap();
        assert!(result.success);

        // Check it's gone
        let installed = manager.list_installed().await.unwrap();
        assert_eq!(installed.len(), 0);
    }

    fn create_test_package(id: &str) -> LibraryAssetPackage {
        LibraryAssetPackage {
            id: Arc::from(id),
            metadata: PackageMetadata {
                name: Arc::from(id),
                version: Arc::from("1.0.0"),
                description: None,
                author: None,
                license: None,
                tags: Arc::new([]),
                keywords: Arc::new([]),
                created: 0,
                modified: 0,
            },
            spec: PackageSpec {
                asset_type: AssetType::JuliaProgram,
                resources: ResourceRequirements::default(),
                security: SecurityConfig {
                    consensus_required: false,
                    sandbox_level: SandboxLevel::Standard,
                    network_access: false,
                    filesystem_access: FilesystemAccess::ReadOnly,
                    permissions: Arc::new([]),
                },
                execution: ExecutionConfig {
                    strategy: ExecutionStrategy::NearestNode,
                    min_consensus: 1,
                    max_concurrent: None,
                    priority: ExecutionPriority::Normal,
                    retry_policy: RetryPolicy::default(),
                },
                dependencies: Arc::new([]),
                environment: Arc::new(HashMap::new()),
            },
            content_refs: ContentReferences {
                main_ref: ContentRef {
                    path: Arc::from("main.jl"),
                    hash: Arc::from("hash"),
                    size: 100,
                    content_type: ContentType::Source,
                },
                file_refs: Arc::new([]),
                binary_refs: Arc::new([]),
                total_size: 100,
            },
            validation: None,
            hash: Arc::from("hash"),
        }
    }
}