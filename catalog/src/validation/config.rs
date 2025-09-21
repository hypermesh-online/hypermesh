//! Validation Configuration
//!
//! Configuration structures for asset validation framework.

use serde::{Deserialize, Serialize};

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Security validation settings
    pub security: SecurityValidationConfig,
    /// Syntax validation settings
    pub syntax: SyntaxValidationConfig,
    /// Dependency validation settings
    pub dependency: DependencyValidationConfig,
    /// Performance validation settings
    pub performance: PerformanceValidationConfig,
    /// Compliance validation settings
    pub compliance: ComplianceValidationConfig,
}

/// Security validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityValidationConfig {
    /// Enable vulnerability scanning
    pub enable_vulnerability_scan: bool,
    /// Enable malware detection
    pub enable_malware_detection: bool,
    /// Enable code injection detection
    pub enable_injection_detection: bool,
    /// Minimum security score required (0-100)
    pub minimum_security_score: u32,
    /// Maximum critical vulnerabilities allowed
    pub max_critical_vulnerabilities: u32,
    /// Security rules to apply
    pub security_rules: Vec<SecurityRule>,
}

/// Syntax validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxValidationConfig {
    /// Enable strict syntax checking
    pub strict_syntax: bool,
    /// Enable style checking
    pub style_checking: bool,
    /// Enable best practices validation
    pub best_practices: bool,
    /// Custom linting rules
    pub custom_rules: Vec<LintRule>,
}

/// Dependency validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyValidationConfig {
    /// Enable dependency resolution
    pub resolve_dependencies: bool,
    /// Enable circular dependency detection
    pub detect_circular_deps: bool,
    /// Enable version conflict detection
    pub detect_version_conflicts: bool,
    /// Maximum dependency depth allowed
    pub max_dependency_depth: u32,
    /// Trusted dependency sources
    pub trusted_sources: Vec<String>,
}

/// Performance validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceValidationConfig {
    /// Enable resource usage analysis
    pub analyze_resource_usage: bool,
    /// Enable complexity analysis
    pub analyze_complexity: bool,
    /// Maximum execution time estimate (seconds)
    pub max_execution_time: u64,
    /// Maximum memory usage estimate (bytes)
    pub max_memory_usage: u64,
    /// Performance benchmarking enabled
    pub enable_benchmarking: bool,
}

/// Compliance validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceValidationConfig {
    /// Enable license compliance checking
    pub check_license_compliance: bool,
    /// Enable export control checking
    pub check_export_control: bool,
    /// Enable privacy compliance checking
    pub check_privacy_compliance: bool,
    /// Required compliance standards
    pub required_standards: Vec<ComplianceStandard>,
    /// Allowed licenses (allowlist)
    pub allowed_licenses: Vec<String>,
    /// Forbidden licenses (blocklist)
    pub forbidden_licenses: Vec<String>,
}

/// Security rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRule {
    /// Rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule severity
    pub severity: SecuritySeverity,
    /// Rule pattern to match
    pub pattern: String,
    /// Rule category
    pub category: String,
    /// Auto-fix available
    pub auto_fix: bool,
    /// Fix suggestion
    pub fix_suggestion: Option<String>,
}

/// Security severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecuritySeverity {
    /// Informational finding
    Info,
    /// Low severity issue
    Low,
    /// Medium severity issue
    Medium,
    /// High severity issue
    High,
    /// Critical severity issue
    Critical,
}

/// Lint rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintRule {
    /// Rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule category
    pub category: LintCategory,
    /// Rule severity
    pub severity: LintSeverity,
    /// Auto-fix available
    pub auto_fix: bool,
    /// Fix suggestion
    pub fix_suggestion: Option<String>,
    /// File patterns to include
    pub include_patterns: Vec<String>,
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
}

/// Lint category
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LintCategory {
    /// Code style violations
    Style,
    /// Performance issues
    Performance,
    /// Best practices violations
    BestPractices,
    /// Potential bugs
    Bugs,
    /// Security issues
    Security,
    /// Code complexity
    Complexity,
    /// Documentation issues
    Documentation,
    /// Maintainability issues
    Maintainability,
}

/// Lint severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum LintSeverity {
    /// Suggestion
    Suggestion,
    /// Warning
    Warning,
    /// Error
    Error,
}

/// Compliance standard definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStandard {
    /// Standard identifier
    pub id: String,
    /// Standard name
    pub name: String,
    /// Standard version
    pub version: String,
    /// Compliance level
    pub level: ComplianceLevel,
    /// Compliance rules
    pub rules: Vec<ComplianceRule>,
}

/// Compliance level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplianceLevel {
    /// Informational compliance
    Informational,
    /// Recommended compliance
    Recommended,
    /// Required compliance
    Required,
    /// Critical compliance
    Critical,
}

/// Compliance rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    /// Rule identifier
    pub id: String,
    /// Rule description
    pub description: String,
    /// Rule category
    pub category: String,
    /// Validation pattern
    pub validation: String,
    /// Remediation guidance
    pub remediation: String,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            security: SecurityValidationConfig {
                enable_vulnerability_scan: true,
                enable_malware_detection: true,
                enable_injection_detection: true,
                minimum_security_score: 70,
                max_critical_vulnerabilities: 0,
                security_rules: vec![],
            },
            syntax: SyntaxValidationConfig {
                strict_syntax: true,
                style_checking: true,
                best_practices: true,
                custom_rules: vec![],
            },
            dependency: DependencyValidationConfig {
                resolve_dependencies: true,
                detect_circular_deps: true,
                detect_version_conflicts: true,
                max_dependency_depth: 10,
                trusted_sources: vec![
                    "crates.io".to_string(),
                    "npm.org".to_string(),
                    "pypi.org".to_string(),
                ],
            },
            performance: PerformanceValidationConfig {
                analyze_resource_usage: true,
                analyze_complexity: true,
                max_execution_time: 300,
                max_memory_usage: 1024 * 1024 * 1024, // 1GB
                enable_benchmarking: false,
            },
            compliance: ComplianceValidationConfig {
                check_license_compliance: true,
                check_export_control: false,
                check_privacy_compliance: false,
                required_standards: vec![],
                allowed_licenses: vec![
                    "MIT".to_string(),
                    "Apache-2.0".to_string(),
                    "BSD-3-Clause".to_string(),
                ],
                forbidden_licenses: vec![],
            },
        }
    }
}

impl ValidationConfig {
    /// Create configuration for strict validation
    pub fn strict() -> Self {
        Self {
            security: SecurityValidationConfig {
                enable_vulnerability_scan: true,
                enable_malware_detection: true,
                enable_injection_detection: true,
                minimum_security_score: 90,
                max_critical_vulnerabilities: 0,
                security_rules: vec![],
            },
            ..Default::default()
        }
    }

    /// Create configuration for development validation
    pub fn development() -> Self {
        Self {
            security: SecurityValidationConfig {
                enable_vulnerability_scan: true,
                enable_malware_detection: false,
                enable_injection_detection: true,
                minimum_security_score: 60,
                max_critical_vulnerabilities: 3,
                security_rules: vec![],
            },
            performance: PerformanceValidationConfig {
                analyze_resource_usage: false,
                analyze_complexity: true,
                max_execution_time: 600,
                max_memory_usage: 2 * 1024 * 1024 * 1024, // 2GB
                enable_benchmarking: false,
            },
            ..Default::default()
        }
    }
}