// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset validation types - validation status, errors, warnings, security scan results

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Asset validation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetValidationStatus {
    pub is_valid: bool,
    pub validated_at: DateTime<Utc>,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub security_results: SecurityScanResults,
    pub dependency_results: DependencyValidationResults,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub severity: ErrorSeverity,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Critical,
    Error,
    Warning,
    Info,
}

/// Security scan results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResults {
    pub security_score: u32,
    pub vulnerabilities: Vec<SecurityVulnerability>,
    pub recommendations: Vec<String>,
    pub scanned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityVulnerability {
    pub id: String,
    pub description: String,
    pub severity: VulnerabilitySeverity,
    pub affected_component: String,
    pub remediation: Option<String>,
    pub cve: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Dependency validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyValidationResults {
    pub dependencies_valid: bool,
    pub total_dependencies: usize,
    pub valid_dependencies: usize,
    pub invalid_dependencies: Vec<InvalidDependency>,
    pub conflicts: Vec<DependencyConflict>,
    #[serde(default)]
    pub validated_at: DateTime<Utc>,
}

impl Default for DependencyValidationResults {
    fn default() -> Self {
        Self {
            dependencies_valid: false,
            total_dependencies: 0,
            valid_dependencies: 0,
            invalid_dependencies: Vec::new(),
            conflicts: Vec::new(),
            validated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidDependency {
    pub name: String,
    pub requested_version: String,
    pub reason: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflict {
    pub dependency_a: String,
    pub dependency_b: String,
    pub conflict_reason: String,
    pub resolution_strategies: Vec<String>,
}
