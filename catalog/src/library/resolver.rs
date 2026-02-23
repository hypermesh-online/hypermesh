// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Smart Dependency Resolution
//!
//! Provides intelligent dependency resolution with conflict detection and version constraint solving.
//! MIGRATION: Now queries CatalogRegistry for dependency resolution.

use super::types::*;
use super::{LibraryInterface, DependencyResolution, ResolvedDependency, DependencyConflict};

use anyhow::{Result, Context, bail};
use std::sync::Arc;
use std::collections::{HashMap, HashSet, VecDeque};

// Import Asset Registry
use crate::registry::CatalogRegistry;

/// Dependency resolver for smart package management
pub struct DependencyResolver {
    /// Resolution strategy
    strategy: ResolutionStrategy,
    /// Version constraint parser
    constraint_parser: VersionConstraintParser,
}

/// Resolution strategies
#[derive(Debug, Clone, Copy)]
pub enum ResolutionStrategy {
    /// Always use the latest compatible version
    Latest,
    /// Prefer minimal version that satisfies constraints
    Minimal,
    /// Balance between latest and stability
    Balanced,
}

/// Version constraint parser
struct VersionConstraintParser;

impl VersionConstraintParser {
    /// Parse version constraint string
    fn parse(&self, constraint: &str) -> Result<VersionConstraint> {
        // Simple implementation - would be more complex in production
        if constraint.starts_with("^") {
            // Caret constraint (compatible)
            Ok(VersionConstraint::Compatible(constraint[1..].to_string()))
        } else if constraint.starts_with("~") {
            // Tilde constraint
            Ok(VersionConstraint::Tilde(constraint[1..].to_string()))
        } else if constraint.starts_with(">=") {
            // Greater than or equal
            Ok(VersionConstraint::GreaterEqual(constraint[2..].to_string()))
        } else if constraint.starts_with(">") {
            // Greater than
            Ok(VersionConstraint::Greater(constraint[1..].to_string()))
        } else if constraint.starts_with("<=") {
            // Less than or equal
            Ok(VersionConstraint::LessEqual(constraint[2..].to_string()))
        } else if constraint.starts_with("<") {
            // Less than
            Ok(VersionConstraint::Less(constraint[1..].to_string()))
        } else if constraint.starts_with("=") {
            // Exact
            Ok(VersionConstraint::Exact(constraint[1..].to_string()))
        } else if constraint.contains("||") {
            // OR constraint
            let parts: Vec<&str> = constraint.split("||").collect();
            Ok(VersionConstraint::Or(
                parts.into_iter().map(|p| p.trim().to_string()).collect()
            ))
        } else if constraint == "*" {
            // Any version
            Ok(VersionConstraint::Any)
        } else {
            // Default to exact
            Ok(VersionConstraint::Exact(constraint.to_string()))
        }
    }

    /// Check if a version satisfies a constraint
    fn satisfies(&self, version: &str, constraint: &VersionConstraint) -> bool {
        match constraint {
            VersionConstraint::Any => true,
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::Compatible(v) => {
                // Compatible with same major version
                let version_parts: Vec<&str> = version.split('.').collect();
                let constraint_parts: Vec<&str> = v.split('.').collect();

                if version_parts.is_empty() || constraint_parts.is_empty() {
                    return false;
                }

                version_parts[0] == constraint_parts[0]
            }
            VersionConstraint::Tilde(v) => {
                // Compatible with same major.minor
                let version_parts: Vec<&str> = version.split('.').collect();
                let constraint_parts: Vec<&str> = v.split('.').collect();

                if version_parts.len() < 2 || constraint_parts.len() < 2 {
                    return false;
                }

                version_parts[0] == constraint_parts[0] &&
                version_parts[1] == constraint_parts[1]
            }
            VersionConstraint::Greater(v) => version > v.as_str(),
            VersionConstraint::GreaterEqual(v) => version >= v.as_str(),
            VersionConstraint::Less(v) => version < v.as_str(),
            VersionConstraint::LessEqual(v) => version <= v.as_str(),
            VersionConstraint::Or(constraints) => {
                constraints.iter().any(|c| {
                    if let Ok(parsed) = self.parse(c) {
                        self.satisfies(version, &parsed)
                    } else {
                        false
                    }
                })
            }
        }
    }
}

/// Version constraints
enum VersionConstraint {
    Any,
    Exact(String),
    Compatible(String),
    Tilde(String),
    Greater(String),
    GreaterEqual(String),
    Less(String),
    LessEqual(String),
    Or(Vec<String>),
}

/// Resolution context for tracking state
struct ResolutionContext {
    /// Resolved packages
    resolved: HashMap<Arc<str>, ResolvedPackage>,
    /// Pending packages to resolve
    pending: VecDeque<PendingPackage>,
    /// Visited packages (for cycle detection)
    visited: HashSet<Arc<str>>,
    /// Conflicts found
    conflicts: Vec<DependencyConflict>,
    /// Missing packages
    missing: Vec<String>,
    /// Current depth
    _current_depth: u32,
    /// Maximum depth
    _max_depth: u32,
}

/// Resolved package information
struct ResolvedPackage {
    name: Arc<str>,
    version: Arc<str>,
    source: Arc<str>,
    dependencies: Vec<Arc<str>>,
    _depth: u32,
}

impl ResolvedPackage {
    /// Get package ID (compatibility method)
    pub fn id(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }

    /// Get package dependencies (compatibility method - returns borrowed slice)
    pub fn _dependencies(&self) -> Vec<PackageDependency> {
        // Convert Arc<str> dependencies to PackageDependency structs
        self.dependencies.iter().map(|name| PackageDependency {
            name: Arc::clone(name),
            version_constraint: Arc::from("*"),
            optional: false,
            platform: None,
        }).collect()
    }
}

/// Pending package to resolve
struct PendingPackage {
    name: Arc<str>,
    constraint: Arc<str>,
    _parent: Option<Arc<str>>,
    depth: u32,
}

impl DependencyResolver {
    /// Create a new dependency resolver
    pub fn new() -> Self {
        Self {
            strategy: ResolutionStrategy::Balanced,
            constraint_parser: VersionConstraintParser,
        }
    }

    /// Set resolution strategy
    pub fn with_strategy(mut self, strategy: ResolutionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Resolve dependencies using registry
    pub async fn resolve_with_registry(
        &self,
        package: &LibraryAssetPackage,
        registry: &CatalogRegistry,
        _max_depth: u32,
    ) -> Result<DependencyResolution> {
        let start = std::time::Instant::now();

        // Query registry for dependencies
        let registry_deps = registry
            .resolve_dependencies(&package.name)
            .await
            .unwrap_or_default();

        // Convert to resolved dependencies
        let resolved: Vec<ResolvedDependency> = registry_deps
            .iter()
            .map(|dep_id| ResolvedDependency {
                name: dep_id.to_string(),
                version: "registry".to_string(),
                source: "catalog-registry".to_string(),
                dependencies: Vec::new(),
            })
            .collect();

        let duration_us = start.elapsed().as_micros() as u64;

        Ok(DependencyResolution {
            resolved,
            conflicts: Vec::new(),
            missing: Vec::new(),
            success: true,
            resolution_time_us: duration_us,
        })
    }

    /// Resolve dependencies for a package
    pub async fn resolve(
        &self,
        package: &LibraryAssetPackage,
        library: &dyn LibraryInterface,
    ) -> Result<DependencyResolution> {
        self.resolve_full(package, library, 10).await
    }

    /// Resolve dependencies with depth limit
    pub async fn resolve_full(
        &self,
        package: &LibraryAssetPackage,
        library: &dyn LibraryInterface,
        max_depth: u32,
    ) -> Result<DependencyResolution> {
        let start = std::time::Instant::now();

        let mut context = ResolutionContext {
            resolved: HashMap::new(),
            pending: VecDeque::new(),
            visited: HashSet::new(),
            conflicts: Vec::new(),
            missing: Vec::new(),
            _current_depth: 0,
            _max_depth: max_depth,
        };

        // Add initial dependencies to pending
        if let Some(spec) = &package.spec {
            for dep in spec.dependencies.iter() {
                if !dep.optional {
                    context.pending.push_back(PendingPackage {
                        name: Arc::clone(&dep.name),
                        constraint: Arc::clone(&dep.version_constraint),
                        _parent: Some(Arc::clone(&package.id)),
                        depth: 1,
                    });
                }
            }
        }

        // Process pending packages
        while let Some(pending) = context.pending.pop_front() {
            if pending.depth > max_depth {
                context.conflicts.push(DependencyConflict {
                    name: pending.name.to_string(),
                    versions: vec![pending.constraint.to_string()],
                    reason: format!("Maximum dependency depth {} exceeded", max_depth),
                });
                continue;
            }

            // Check for cycles
            if context.visited.contains(&pending.name) {
                // Already processed, check for version conflict
                if let Some(resolved) = context.resolved.get(&pending.name) {
                    let constraint = self.constraint_parser.parse(&pending.constraint)?;
                    if !self.constraint_parser.satisfies(&resolved.version, &constraint) {
                        context.conflicts.push(DependencyConflict {
                            name: pending.name.to_string(),
                            versions: vec![
                                resolved.version.to_string(),
                                pending.constraint.to_string(),
                            ],
                            reason: "Version conflict".to_string(),
                        });
                    }
                }
                continue;
            }

            context.visited.insert(Arc::clone(&pending.name));

            // Try to resolve package
            match self.resolve_package(&pending, library, &mut context).await {
                Ok(()) => {
                    // Successfully resolved
                }
                Err(_e) => {
                    context.missing.push(pending.name.to_string());
                }
            }
        }

        // Build resolution result
        let resolved: Vec<ResolvedDependency> = context.resolved
            .values()
            .map(|pkg| ResolvedDependency {
                name: pkg.name.to_string(),
                version: pkg.id().to_string(),
                source: pkg.source.to_string(),
                dependencies: self.build_dependency_tree(&pkg.name, &context.resolved),
            })
            .collect();

        let duration_us = start.elapsed().as_micros() as u64;

        let success = context.conflicts.is_empty() && context.missing.is_empty();
        Ok(DependencyResolution {
            resolved,
            conflicts: context.conflicts,
            missing: context.missing,
            success,
            resolution_time_us: duration_us,
        })
    }

    /// Resolve a single package
    async fn resolve_package(
        &self,
        pending: &PendingPackage,
        library: &dyn LibraryInterface,
        context: &mut ResolutionContext,
    ) -> Result<()> {
        // Get package from library
        let package = library.get_package(&pending.name).await?
            .context(format!("Package {} not found", pending.name))?;

        // Check version constraint
        let constraint = self.constraint_parser.parse(&pending.constraint)?;
        let package_version = package.metadata.as_ref()
            .map(|m| m.version.as_ref())
            .unwrap_or(&package.version);
        if !self.constraint_parser.satisfies(package_version, &constraint) {
            bail!(
                "Version {} does not satisfy constraint {}",
                package_version,
                pending.constraint
            );
        }

        // Add to resolved
        context.resolved.insert(
            Arc::clone(&package.id),
            ResolvedPackage {
                name: Arc::clone(&package.id),
                version: package.metadata.as_ref()
                    .map(|m| Arc::clone(&m.version))
                    .unwrap_or_else(|| Arc::from(package.version.as_str())),
                source: Arc::from("library"),
                dependencies: package.spec.as_ref()
                    .map(|s| s.dependencies
                        .iter()
                        .map(|d| Arc::clone(&d.name))
                        .collect())
                    .unwrap_or_else(Vec::new),
                _depth: pending.depth,
            }
        );

        // Add transitive dependencies to pending
        if let Some(spec) = &package.spec {
            for dep in spec.dependencies.iter() {
                if !dep.optional && !context.visited.contains(&dep.name) {
                    context.pending.push_back(PendingPackage {
                        name: Arc::clone(&dep.name),
                        constraint: Arc::clone(&dep.version_constraint),
                        _parent: Some(Arc::clone(&package.id)),
                        depth: pending.depth + 1,
                    });
                }
            }
        }

        Ok(())
    }

    /// Build dependency tree for a package
    fn build_dependency_tree(
        &self,
        package_name: &Arc<str>,
        resolved: &HashMap<Arc<str>, ResolvedPackage>,
    ) -> Vec<ResolvedDependency> {
        let mut tree = Vec::new();

        if let Some(package) = resolved.get(package_name) {
            for dep_name in &package.dependencies {
                if let Some(dep) = resolved.get(dep_name) {
                    tree.push(ResolvedDependency {
                        name: dep.name.to_string(),
                        version: dep.version.to_string(),
                        source: dep.source.to_string(),
                        dependencies: self.build_dependency_tree(dep_name, resolved),
                    });
                }
            }
        }

        tree
    }

    /// Find best version that satisfies constraints
    pub async fn find_best_version(
        &self,
        name: &str,
        constraint: &str,
        library: &dyn LibraryInterface,
    ) -> Result<Option<String>> {
        // In a real implementation, this would:
        // 1. Query all available versions from library
        // 2. Filter by constraint
        // 3. Apply resolution strategy to pick best

        // For now, just get the package and check if it satisfies
        if let Some(package) = library.get_package(name).await? {
            let parsed_constraint = self.constraint_parser.parse(constraint)?;
            let package_version = package.metadata.as_ref()
                .map(|m| m.version.as_ref())
                .unwrap_or(&package.version);
            if self.constraint_parser.satisfies(package_version, &parsed_constraint) {
                return Ok(Some(package_version.to_string()));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constraint_parsing() {
        let parser = VersionConstraintParser;

        // Test various constraint formats
        assert!(matches!(parser.parse("^1.0.0").unwrap(), VersionConstraint::Compatible(_)));
        assert!(matches!(parser.parse("~1.0.0").unwrap(), VersionConstraint::Tilde(_)));
        assert!(matches!(parser.parse(">=1.0.0").unwrap(), VersionConstraint::GreaterEqual(_)));
        assert!(matches!(parser.parse("<2.0.0").unwrap(), VersionConstraint::Less(_)));
        assert!(matches!(parser.parse("*").unwrap(), VersionConstraint::Any));
        assert!(matches!(parser.parse("1.0.0").unwrap(), VersionConstraint::Exact(_)));
    }

    #[test]
    fn test_version_satisfaction() {
        let parser = VersionConstraintParser;

        // Test exact match
        let exact = VersionConstraint::Exact("1.0.0".to_string());
        assert!(parser.satisfies("1.0.0", &exact));
        assert!(!parser.satisfies("1.0.1", &exact));

        // Test compatible (caret)
        let compat = VersionConstraint::Compatible("1.0.0".to_string());
        assert!(parser.satisfies("1.0.0", &compat));
        assert!(parser.satisfies("1.9.9", &compat));
        assert!(!parser.satisfies("2.0.0", &compat));

        // Test any
        let any = VersionConstraint::Any;
        assert!(parser.satisfies("0.0.1", &any));
        assert!(parser.satisfies("999.999.999", &any));
    }

    #[tokio::test]
    async fn test_resolver_creation() {
        let resolver = DependencyResolver::new();
        assert!(matches!(resolver.strategy, ResolutionStrategy::Balanced));

        let resolver = DependencyResolver::new()
            .with_strategy(ResolutionStrategy::Latest);
        assert!(matches!(resolver.strategy, ResolutionStrategy::Latest));
    }
}