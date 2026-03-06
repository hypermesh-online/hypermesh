// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Dashboard asset manifest types and helpers.
//!
//! Dashboards are first-class HyperMesh assets deployed via TOML manifests.
//! This module lives in blockmatrix (where the CLI binary is) so the node
//! binary can parse and validate manifests without pulling in the full
//! catalog crate.

pub mod default;
pub mod deploy;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level dashboard manifest parsed from `dashboard.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardManifest {
    pub dashboard: DashboardMeta,
    pub access: DashboardAccess,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

/// Metadata about the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub domain: String,
}

/// Access scope definitions pointing to content directories.
///
/// Two scopes: `public` (landing/onboarding) and `private` (full node UI).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardAccess {
    pub public: Option<String>,
    pub private: Option<String>,
}

/// Parse a TOML string into a [`DashboardManifest`].
pub fn parse_manifest(toml_str: &str) -> Result<DashboardManifest, String> {
    toml::from_str(toml_str).map_err(|e| format!("Failed to parse dashboard.toml: {e}"))
}

/// Validate a parsed manifest against structural and filesystem rules.
///
/// Returns `Ok(())` when valid, or `Err` with a list of validation errors.
pub fn validate_manifest(
    manifest: &DashboardManifest,
    base_dir: &Path,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if manifest.dashboard.name.is_empty() {
        errors.push("name is required".into());
    }
    if manifest.dashboard.version.is_empty() {
        errors.push("version is required".into());
    }
    if manifest.dashboard.domain.is_empty() {
        errors.push("domain is required".into());
    }
    if manifest.dashboard.description.is_empty() {
        errors.push("description is required".into());
    }

    // At least one access scope must be defined
    if manifest.access.public.is_none()
        && manifest.access.private.is_none()
    {
        errors.push("at least one access scope (public/private) must be defined".into());
    }

    // Check referenced directories exist
    for (name, path_opt) in [
        ("public", &manifest.access.public),
        ("private", &manifest.access.private),
    ] {
        if let Some(rel_path) = path_opt {
            let full_path = base_dir.join(rel_path);
            if !full_path.exists() {
                errors.push(format!(
                    "access.{name} directory does not exist: {}",
                    full_path.display()
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Generate scaffold TOML content for a new dashboard project.
pub fn scaffold_manifest(project_name: &str) -> String {
    format!(
        r#"[dashboard]
name = "{project_name}"
version = "0.1.0"
description = "My HyperMesh Dashboard"
domain = "{project_name}.hypermesh"

[access]
public = "dist/public/"
private = "dist/private/"
"#
    )
}

/// Generate a minimal HTML page for scaffolding.
pub fn scaffold_html(project_name: &str, section: &str) -> String {
    let title = match section {
        "private" => format!("{project_name} Dashboard"),
        _ => format!("Welcome to {project_name}"),
    };
    format!("<!DOCTYPE html><html><body><h1>{title}</h1></body></html>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const FULL_TOML: &str = r#"
[dashboard]
name = "test-dash"
version = "1.0.0"
description = "A test dashboard"
domain = "test.hypermesh"

[access]
public = "dist/public/"
private = "dist/private/"

[dependencies]
charts = "0.2.0"
"#;

    #[test]
    fn test_parse_full_manifest() {
        let m = parse_manifest(FULL_TOML).expect("test: parse full");
        assert_eq!(m.dashboard.name, "test-dash");
        assert_eq!(m.dashboard.version, "1.0.0");
        assert_eq!(m.dashboard.domain, "test.hypermesh");
        assert_eq!(m.access.public.as_deref(), Some("dist/public/"));
        assert_eq!(
            m.dependencies.get("charts").map(String::as_str),
            Some("0.2.0")
        );
    }

    #[test]
    fn test_parse_minimal_manifest() {
        let toml = r#"
[dashboard]
name = "min"
version = "0.1.0"
description = "Minimal"
domain = "min.hypermesh"

[access]
public = "dist/"
"#;
        let m = parse_manifest(toml).expect("test: parse minimal");
        assert_eq!(m.dashboard.name, "min");
        assert!(m.access.private.is_none());
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn test_validate_missing_name() {
        let m = DashboardManifest {
            dashboard: DashboardMeta {
                name: String::new(),
                version: "1.0.0".into(),
                description: "d".into(),
                domain: "d.hypermesh".into(),
            },
            access: DashboardAccess {
                public: Some("x/".into()),
                private: None,
            },
            dependencies: HashMap::new(),
        };
        let errs = validate_manifest(&m, Path::new("/tmp")).expect_err("test: fail");
        assert!(errs.iter().any(|e| e.contains("name is required")));
    }

    #[test]
    fn test_validate_no_access_scope() {
        let m = DashboardManifest {
            dashboard: DashboardMeta {
                name: "ok".into(),
                version: "1.0.0".into(),
                description: "d".into(),
                domain: "d.hypermesh".into(),
            },
            access: DashboardAccess {
                public: None,
                private: None,
            },
            dependencies: HashMap::new(),
        };
        let errs = validate_manifest(&m, Path::new("/tmp")).expect_err("test: fail");
        assert!(errs.iter().any(|e| e.contains("at least one access scope")));
    }

    #[test]
    fn test_validate_directory_exists() {
        let tmp = TempDir::new().expect("test: tmpdir");
        std::fs::create_dir_all(tmp.path().join("dist")).expect("test: mkdir");
        let m = DashboardManifest {
            dashboard: DashboardMeta {
                name: "ok".into(),
                version: "1.0.0".into(),
                description: "d".into(),
                domain: "d.hypermesh".into(),
            },
            access: DashboardAccess {
                public: Some("dist".into()),
                private: None,
            },
            dependencies: HashMap::new(),
        };
        assert!(validate_manifest(&m, tmp.path()).is_ok());
    }

    #[test]
    fn test_validate_directory_missing() {
        let tmp = TempDir::new().expect("test: tmpdir");
        let m = DashboardManifest {
            dashboard: DashboardMeta {
                name: "ok".into(),
                version: "1.0.0".into(),
                description: "d".into(),
                domain: "d.hypermesh".into(),
            },
            access: DashboardAccess {
                public: Some("nonexistent/".into()),
                private: None,
            },
            dependencies: HashMap::new(),
        };
        let errs = validate_manifest(&m, tmp.path()).expect_err("test: fail");
        assert!(errs.iter().any(|e| e.contains("does not exist")));
    }

    #[test]
    fn test_manifest_serde_roundtrip() {
        let original = parse_manifest(FULL_TOML).expect("test: parse");
        let serialized = toml::to_string(&original).expect("test: serialize");
        let roundtrip: DashboardManifest =
            toml::from_str(&serialized).expect("test: roundtrip");
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_scaffold_manifest_content() {
        let content = scaffold_manifest("my-app");
        assert!(content.contains("name = \"my-app\""));
        assert!(content.contains("domain = \"my-app.hypermesh\""));
        assert!(content.contains("dist/public/"));
    }

    #[test]
    fn test_scaffold_html_sections() {
        assert!(scaffold_html("app", "public").contains("Welcome to app"));
        assert!(scaffold_html("app", "private").contains("app Dashboard"));
    }
}
