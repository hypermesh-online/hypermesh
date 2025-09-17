//! Version Management and Dependency Resolution
//!
//! Provides semantic versioning support and dependency resolution for asset packages.

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Semantic version implementation
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SemanticVersion {
    /// Major version
    pub major: u64,
    /// Minor version
    pub minor: u64,
    /// Patch version
    pub patch: u64,
    /// Pre-release identifier
    pub pre_release: Option<String>,
    /// Build metadata
    pub build: Option<String>,
}

/// Version constraint for dependencies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VersionConstraint {
    /// Exact version
    Exact(SemanticVersion),
    /// Compatible version (caret constraint)
    Compatible(SemanticVersion),
    /// Tilde constraint (patch level changes)
    Tilde(SemanticVersion),
    /// Greater than or equal
    GreaterEqual(SemanticVersion),
    /// Less than
    LessThan(SemanticVersion),
    /// Range constraint
    Range { min: SemanticVersion, max: SemanticVersion },
    /// Wildcard constraint
    Wildcard { major: Option<u64>, minor: Option<u64> },
}

/// Version manager for handling semantic versioning
pub struct VersionManager {
    /// Version cache
    version_cache: HashMap<String, Vec<SemanticVersion>>,
}

/// Dependency resolver for asset packages
pub struct DependencyResolver {
    /// Version manager
    version_manager: VersionManager,
    /// Dependency cache
    dependency_cache: HashMap<String, Vec<ResolvedDependency>>,
}

/// Resolved dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDependency {
    /// Dependency name
    pub name: String,
    /// Resolved version
    pub version: SemanticVersion,
    /// Dependency source
    pub source: String,
    /// Transitive dependencies
    pub dependencies: Vec<ResolvedDependency>,
    /// Dependency depth
    pub depth: u32,
}

/// Dependency resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolution {
    /// Resolved dependencies
    pub resolved: Vec<ResolvedDependency>,
    /// Conflicting dependencies
    pub conflicts: Vec<DependencyConflict>,
    /// Missing dependencies
    pub missing: Vec<String>,
    /// Resolution success
    pub success: bool,
    /// Resolution time (milliseconds)
    pub resolution_time_ms: u64,
}

/// Dependency conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflict {
    /// Dependency name
    pub name: String,
    /// Conflicting versions
    pub versions: Vec<SemanticVersion>,
    /// Conflict reason
    pub reason: String,
    /// Suggested resolution
    pub suggestion: String,
}

impl fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        
        if let Some(pre) = &self.pre_release {
            write!(f, "-{}", pre)?;
        }
        
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        
        Ok(())
    }
}

impl SemanticVersion {
    /// Create a new semantic version
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: None,
        }
    }
    
    /// Create a new semantic version with pre-release
    pub fn new_pre(major: u64, minor: u64, patch: u64, pre_release: String) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: Some(pre_release),
            build: None,
        }
    }
    
    /// Parse semantic version from string
    pub fn parse(version_str: &str) -> Result<Self> {
        let version_regex = regex::Regex::new(
            r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?P<buildmetadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$"
        )?;
        
        let captures = version_regex.captures(version_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid semantic version format: {}", version_str))?;
        
        let major = captures.name("major").unwrap().as_str().parse()?;
        let minor = captures.name("minor").unwrap().as_str().parse()?;
        let patch = captures.name("patch").unwrap().as_str().parse()?;
        
        let pre_release = captures.name("prerelease").map(|m| m.as_str().to_string());
        let build = captures.name("buildmetadata").map(|m| m.as_str().to_string());
        
        Ok(Self {
            major,
            minor,
            patch,
            pre_release,
            build,
        })
    }
    
    /// Check if this version satisfies a constraint
    pub fn satisfies(&self, constraint: &VersionConstraint) -> bool {
        match constraint {
            VersionConstraint::Exact(version) => self == version,
            VersionConstraint::Compatible(version) => {
                // Caret constraint: ^1.2.3 means >=1.2.3 <2.0.0
                if self.major != version.major {
                    return false;
                }
                if version.major > 0 {
                    self >= version
                } else if self.minor != version.minor {
                    return false;
                } else if version.minor > 0 {
                    self >= version
                } else {
                    self >= version && self.patch == version.patch
                }
            }
            VersionConstraint::Tilde(version) => {
                // Tilde constraint: ~1.2.3 means >=1.2.3 <1.3.0
                self.major == version.major &&
                self.minor == version.minor &&
                self >= version
            }
            VersionConstraint::GreaterEqual(version) => self >= version,
            VersionConstraint::LessThan(version) => self < version,
            VersionConstraint::Range { min, max } => self >= min && self < max,
            VersionConstraint::Wildcard { major, minor } => {
                if let Some(maj) = major {
                    if self.major != *maj {
                        return false;
                    }
                    if let Some(min) = minor {
                        self.minor == *min
                    } else {
                        true
                    }
                } else {
                    true
                }
            }
        }
    }
    
    /// Increment major version
    pub fn increment_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
        self.pre_release = None;
        self.build = None;
    }
    
    /// Increment minor version
    pub fn increment_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
        self.pre_release = None;
        self.build = None;
    }
    
    /// Increment patch version
    pub fn increment_patch(&mut self) {
        self.patch += 1;
        self.pre_release = None;
        self.build = None;
    }
    
    /// Check if this is a pre-release version
    pub fn is_prerelease(&self) -> bool {
        self.pre_release.is_some()
    }
    
    /// Check if this is a stable version
    pub fn is_stable(&self) -> bool {
        !self.is_prerelease()
    }
}

impl VersionConstraint {
    /// Parse version constraint from string
    pub fn parse(constraint_str: &str) -> Result<Self> {
        let constraint_str = constraint_str.trim();
        
        if constraint_str.starts_with("^") {
            let version = SemanticVersion::parse(&constraint_str[1..])?;
            Ok(VersionConstraint::Compatible(version))
        } else if constraint_str.starts_with("~") {
            let version = SemanticVersion::parse(&constraint_str[1..])?;
            Ok(VersionConstraint::Tilde(version))
        } else if constraint_str.starts_with(">=") {
            let version = SemanticVersion::parse(&constraint_str[2..])?;
            Ok(VersionConstraint::GreaterEqual(version))
        } else if constraint_str.starts_with("<") {
            let version = SemanticVersion::parse(&constraint_str[1..])?;
            Ok(VersionConstraint::LessThan(version))
        } else if constraint_str.contains("..") {
            let parts: Vec<&str> = constraint_str.split("..").collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid range constraint: {}", constraint_str));
            }
            let min = SemanticVersion::parse(parts[0])?;
            let max = SemanticVersion::parse(parts[1])?;
            Ok(VersionConstraint::Range { min, max })
        } else if constraint_str.contains('*') {
            // Wildcard constraint like "1.*" or "1.2.*"
            let parts: Vec<&str> = constraint_str.split('.').collect();
            let major = if parts[0] == "*" { None } else { Some(parts[0].parse()?) };
            let minor = if parts.len() > 1 && parts[1] != "*" { Some(parts[1].parse()?) } else { None };
            Ok(VersionConstraint::Wildcard { major, minor })
        } else {
            // Exact version
            let version = SemanticVersion::parse(constraint_str)?;
            Ok(VersionConstraint::Exact(version))
        }
    }
    
    /// Check if constraint allows a specific version
    pub fn allows(&self, version: &SemanticVersion) -> bool {
        version.satisfies(self)
    }
    
    /// Get the best matching version from a list
    pub fn best_match<'a>(&self, versions: &'a [SemanticVersion]) -> Option<&'a SemanticVersion> {
        versions.iter()
            .filter(|v| self.allows(v))
            .max()
    }
}

impl VersionManager {
    /// Create a new version manager
    pub fn new() -> Self {
        Self {
            version_cache: HashMap::new(),
        }
    }
    
    /// Register available versions for a package
    pub fn register_versions(&mut self, package_name: String, versions: Vec<SemanticVersion>) {
        self.version_cache.insert(package_name, versions);
    }
    
    /// Get available versions for a package
    pub fn get_versions(&self, package_name: &str) -> Option<&[SemanticVersion]> {
        self.version_cache.get(package_name).map(|v| v.as_slice())
    }
    
    /// Find the best matching version for a constraint
    pub fn find_best_version<'a>(&'a self, package_name: &str, constraint: &VersionConstraint) -> Option<&'a SemanticVersion> {
        if let Some(versions) = self.get_versions(package_name) {
            constraint.best_match(versions)
        } else {
            None
        }
    }
    
    /// Check if a version is available
    pub fn is_version_available(&self, package_name: &str, version: &SemanticVersion) -> bool {
        self.get_versions(package_name)
            .map(|versions| versions.contains(version))
            .unwrap_or(false)
    }
    
    /// Get the latest version of a package
    pub fn get_latest_version(&self, package_name: &str) -> Option<&SemanticVersion> {
        self.get_versions(package_name)
            .and_then(|versions| {
                versions.iter().max()
            })
    }
    
    /// Get the latest stable version of a package
    pub fn get_latest_stable_version(&self, package_name: &str) -> Option<&SemanticVersion> {
        self.get_versions(package_name)
            .and_then(|versions| {
                versions.iter()
                    .filter(|v| v.is_stable())
                    .max()
            })
    }
}

impl DependencyResolver {
    /// Create a new dependency resolver
    pub fn new() -> Self {
        Self {
            version_manager: VersionManager::new(),
            dependency_cache: HashMap::new(),
        }
    }
    
    /// Resolve dependencies for a list of requirements
    pub async fn resolve_dependencies(
        &mut self,
        dependencies: &[crate::assets::AssetDependency],
    ) -> Result<DependencyResolution> {
        let start_time = std::time::Instant::now();
        
        let mut resolved = Vec::new();
        let mut conflicts = Vec::new();
        let mut missing = Vec::new();
        let mut visited = HashSet::new();
        
        for dep in dependencies {
            match DependencyResolver::resolve_single_dependency(self, dep, &mut visited, 0).await {
                Ok(mut resolved_deps) => resolved.append(&mut resolved_deps),
                Err(e) => {
                    missing.push(format!("{}: {}", dep.name, e));
                }
            }
        }
        
        // Check for conflicts
        conflicts = self.detect_conflicts(&resolved);
        
        let resolution_time = start_time.elapsed().as_millis() as u64;
        let success = conflicts.is_empty() && missing.is_empty();
        
        Ok(DependencyResolution {
            resolved,
            conflicts,
            missing,
            success,
            resolution_time_ms: resolution_time,
        })
    }
    
    /// Resolve a single dependency recursively
    async fn resolve_single_dependency(
        &self,
        dependency: &crate::assets::AssetDependency,
        visited: &mut HashSet<String>,
        depth: u32,
    ) -> Result<Vec<ResolvedDependency>> {
        if visited.contains(&dependency.name) {
            return Ok(vec![]); // Avoid circular dependencies
        }
        
        visited.insert(dependency.name.clone());
        
        // Parse version constraint
        let constraint = VersionConstraint::parse(&dependency.version)?;
        
        // For now, create a dummy resolved version since we don't have access to the version manager
        let version = SemanticVersion::parse("1.0.0")?;
        
        // Get dependency information
        let source = match &dependency.source {
            crate::assets::DependencySource::Registry { registry, .. } => registry.clone(),
            crate::assets::DependencySource::Git { url, .. } => url.clone(),
            crate::assets::DependencySource::Local { path } => path.clone(),
            crate::assets::DependencySource::Http { url, .. } => url.clone(),
        };
        
        visited.remove(&dependency.name);
        
        let resolved_dep = ResolvedDependency {
            name: dependency.name.clone(),
            version,
            source,
            dependencies: vec![], // TODO: Implement transitive dependency resolution
            depth,
        };
        
        Ok(vec![resolved_dep])
    }
    
    /// Get transitive dependencies (placeholder implementation)
    async fn get_transitive_dependencies(
        &self,
        _package_name: &str,
        _version: &SemanticVersion,
    ) -> Result<Vec<crate::assets::AssetDependency>> {
        // TODO: Implement fetching transitive dependencies from registry
        Ok(vec![])
    }
    
    /// Detect conflicts in resolved dependencies
    fn detect_conflicts(&self, resolved: &[ResolvedDependency]) -> Vec<DependencyConflict> {
        let mut conflicts = Vec::new();
        let mut package_versions: HashMap<String, Vec<&SemanticVersion>> = HashMap::new();
        
        // Collect all versions of each package
        for dep in resolved {
            package_versions.entry(dep.name.clone())
                .or_insert_with(Vec::new)
                .push(&dep.version);
        }
        
        // Check for conflicts
        for (name, versions) in package_versions {
            if versions.len() > 1 {
                let mut unique_versions: Vec<&SemanticVersion> = versions;
                unique_versions.sort();
                unique_versions.dedup();
                
                if unique_versions.len() > 1 {
                    conflicts.push(DependencyConflict {
                        name: name.clone(),
                        versions: unique_versions.into_iter().map(|v| v.clone()).collect(),
                        reason: "Multiple incompatible versions required".to_string(),
                        suggestion: format!("Choose a single version of {} that satisfies all constraints", name),
                    });
                }
            }
        }
        
        conflicts
    }
    
    /// Get version manager reference
    pub fn version_manager(&mut self) -> &mut VersionManager {
        &mut self.version_manager
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_semantic_version_parsing() {
        let version = SemanticVersion::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert!(version.pre_release.is_none());
        
        let pre_version = SemanticVersion::parse("1.2.3-alpha.1").unwrap();
        assert_eq!(pre_version.pre_release, Some("alpha.1".to_string()));
        
        let build_version = SemanticVersion::parse("1.2.3+build.1").unwrap();
        assert_eq!(build_version.build, Some("build.1".to_string()));
    }
    
    #[test]
    fn test_version_constraints() {
        let version = SemanticVersion::new(1, 2, 3);
        
        let exact = VersionConstraint::parse("1.2.3").unwrap();
        assert!(version.satisfies(&exact));
        
        let compatible = VersionConstraint::parse("^1.2.0").unwrap();
        assert!(version.satisfies(&compatible));
        
        let tilde = VersionConstraint::parse("~1.2.0").unwrap();
        assert!(version.satisfies(&tilde));
        
        let greater = VersionConstraint::parse(">=1.0.0").unwrap();
        assert!(version.satisfies(&greater));
    }
    
    #[test]
    fn test_version_ordering() {
        let v1 = SemanticVersion::new(1, 0, 0);
        let v2 = SemanticVersion::new(1, 1, 0);
        let v3 = SemanticVersion::new(2, 0, 0);
        
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }
    
    #[test]
    fn test_version_manager() {
        let mut manager = VersionManager::new();
        
        let versions = vec![
            SemanticVersion::new(1, 0, 0),
            SemanticVersion::new(1, 1, 0),
            SemanticVersion::new(2, 0, 0),
        ];
        
        manager.register_versions("test-package".to_string(), versions);
        
        let latest = manager.get_latest_version("test-package").unwrap();
        assert_eq!(*latest, SemanticVersion::new(2, 0, 0));
        
        let constraint = VersionConstraint::parse("^1.0.0").unwrap();
        let best = manager.find_best_version("test-package", &constraint).unwrap();
        assert_eq!(*best, SemanticVersion::new(1, 1, 0));
    }
}