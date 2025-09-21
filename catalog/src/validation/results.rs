//! Validation Results
//!
//! Result structures for asset validation framework.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::config::{SecuritySeverity, LintSeverity, ComplianceLevel};

/// Combined validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the validation passed overall
    pub passed: bool,
    /// Validation timestamp
    pub timestamp: DateTime<Utc>,
    /// Asset being validated
    pub asset_id: String,
    /// Package version being validated
    pub version: String,
    /// Security validation results
    pub security: Option<SecurityValidationResult>,
    /// Syntax validation results
    pub syntax: Option<SyntaxValidationResult>,
    /// Performance validation results
    pub performance: Option<PerformanceValidationResult>,
    /// Compliance validation results
    pub compliance: Option<ComplianceValidationResult>,
    /// Validation summary
    pub summary: ValidationSummary,
}

/// Security validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityValidationResult {
    /// Overall security score (0-100)
    pub score: u32,
    /// Vulnerabilities found
    pub vulnerabilities: Vec<Vulnerability>,
    /// Malware detections
    pub malware: Vec<MalwareDetection>,
    /// Code injection risks
    pub injection_risks: Vec<InjectionRisk>,
    /// Security rule failures
    pub rule_failures: Vec<SecurityRuleFailure>,
    /// Security recommendations
    pub recommendations: Vec<String>,
}

/// Vulnerability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    /// CVE identifier if applicable
    pub cve: Option<String>,
    /// Vulnerability description
    pub description: String,
    /// Severity level
    pub severity: SecuritySeverity,
    /// Affected component
    pub component: String,
    /// Fix available
    pub fix_available: bool,
    /// Fix version if available
    pub fix_version: Option<String>,
}

/// Malware detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MalwareDetection {
    /// Type of malware detected
    pub malware_type: String,
    /// Detection confidence (0-100)
    pub confidence: u32,
    /// Affected files
    pub affected_files: Vec<String>,
    /// Detection signature
    pub signature: String,
    /// Risk level
    pub risk_level: RiskLevel,
}

/// Code injection risk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionRisk {
    /// Type of injection
    pub injection_type: InjectionType,
    /// Risk description
    pub description: String,
    /// Code location
    pub location: CodeLocation,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Mitigation suggestion
    pub mitigation: String,
}

/// Types of code injection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InjectionType {
    /// SQL injection
    Sql,
    /// Command injection
    Command,
    /// Script injection (XSS)
    Script,
    /// Path traversal
    PathTraversal,
    /// Template injection
    Template,
    /// LDAP injection
    Ldap,
    /// XPath injection
    Xpath,
    /// Other injection type
    Other,
}

/// Risk level classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// Minimal risk
    Minimal,
    /// Low risk
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
    /// Critical risk
    Critical,
}

/// Code location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    /// File path
    pub file: String,
    /// Line number
    pub line: Option<u32>,
    /// Column number
    pub column: Option<u32>,
    /// Code snippet
    pub snippet: Option<String>,
}

/// Security rule failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRuleFailure {
    /// Rule ID that failed
    pub rule_id: String,
    /// Failure description
    pub description: String,
    /// Failure location
    pub location: CodeLocation,
    /// Rule severity
    pub severity: SecuritySeverity,
}

/// Syntax validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxValidationResult {
    /// Whether syntax is valid
    pub valid: bool,
    /// Syntax errors found
    pub errors: Vec<SyntaxError>,
    /// Style violations found
    pub style_violations: Vec<StyleViolation>,
    /// Best practice violations
    pub best_practices: Vec<BestPracticeViolation>,
    /// Linting issues
    pub linting_issues: Vec<LintingIssue>,
    /// Total issue count
    pub total_issues: usize,
}

/// Syntax error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxError {
    /// Error message
    pub message: String,
    /// Error location
    pub location: CodeLocation,
    /// Error code if applicable
    pub error_code: Option<String>,
    /// Fix suggestion
    pub fix_suggestion: Option<String>,
}

/// Style violation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleViolation {
    /// Rule violated
    pub rule: String,
    /// Violation description
    pub description: String,
    /// Violation location
    pub location: CodeLocation,
    /// Auto-fixable
    pub auto_fixable: bool,
}

/// Best practice violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestPracticeViolation {
    /// Practice violated
    pub practice: String,
    /// Violation description
    pub description: String,
    /// Violation location
    pub location: CodeLocation,
    /// Recommendation
    pub recommendation: String,
    /// Impact level
    pub impact: String,
}

/// Linting issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintingIssue {
    /// Lint rule ID
    pub rule_id: String,
    /// Issue message
    pub message: String,
    /// Issue location
    pub location: CodeLocation,
    /// Issue severity
    pub severity: LintSeverity,
    /// Auto-fixable
    pub auto_fixable: bool,
}

/// Performance validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceValidationResult {
    /// Estimated resource usage
    pub resource_usage: ResourceUsage,
    /// Complexity analysis
    pub complexity: ComplexityAnalysis,
    /// Performance issues found
    pub issues: Vec<PerformanceIssue>,
    /// Benchmark results if available
    pub benchmarks: Option<BenchmarkResults>,
    /// Performance score (0-100)
    pub score: u32,
}

/// Resource usage estimates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Estimated CPU usage (percentage)
    pub cpu_usage: f64,
    /// Estimated memory usage (bytes)
    pub memory_usage: u64,
    /// Estimated disk I/O (bytes/sec)
    pub disk_io: u64,
    /// Estimated network I/O (bytes/sec)
    pub network_io: u64,
}

/// Code complexity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityAnalysis {
    /// Cyclomatic complexity
    pub cyclomatic_complexity: u32,
    /// Cognitive complexity
    pub cognitive_complexity: u32,
    /// Lines of code
    pub lines_of_code: usize,
    /// Halstead metrics
    pub halstead: HalsteadMetrics,
    /// Maintainability index
    pub maintainability_index: f64,
}

/// Halstead complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalsteadMetrics {
    /// Program vocabulary
    pub vocabulary: u32,
    /// Program length
    pub length: u32,
    /// Calculated program length
    pub calculated_length: f64,
    /// Volume
    pub volume: f64,
    /// Difficulty
    pub difficulty: f64,
    /// Effort
    pub effort: f64,
    /// Time to implement (seconds)
    pub time: f64,
    /// Delivered bugs estimate
    pub bugs: f64,
}

/// Performance issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceIssue {
    /// Issue type
    pub issue_type: PerformanceIssueType,
    /// Issue description
    pub description: String,
    /// Issue location
    pub location: CodeLocation,
    /// Performance impact
    pub impact: PerformanceImpact,
    /// Optimization suggestion
    pub suggestion: String,
}

/// Types of performance issues
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PerformanceIssueType {
    /// Inefficient algorithm
    Algorithm,
    /// Memory leak
    MemoryLeak,
    /// Excessive allocations
    Allocations,
    /// Blocking I/O
    BlockingIo,
    /// Inefficient loop
    Loop,
    /// Redundant computation
    Redundant,
    /// Other performance issue
    Other,
}

/// Performance impact level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PerformanceImpact {
    /// Negligible impact
    Negligible,
    /// Minor impact
    Minor,
    /// Moderate impact
    Moderate,
    /// Major impact
    Major,
    /// Severe impact
    Severe,
}

/// Benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    /// Benchmark name
    pub name: String,
    /// Benchmark iterations
    pub iterations: u32,
    /// Average execution time (nanoseconds)
    pub avg_time_ns: u64,
    /// Minimum execution time (nanoseconds)
    pub min_time_ns: u64,
    /// Maximum execution time (nanoseconds)
    pub max_time_ns: u64,
    /// Standard deviation
    pub std_dev_ns: u64,
    /// Detailed statistics
    pub statistics: BenchmarkStatistics,
}

/// Benchmark statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkStatistics {
    /// Median time (nanoseconds)
    pub median_ns: u64,
    /// 95th percentile time (nanoseconds)
    pub p95_ns: u64,
    /// 99th percentile time (nanoseconds)
    pub p99_ns: u64,
    /// Operations per second
    pub ops_per_sec: f64,
    /// Memory allocated per operation (bytes)
    pub memory_per_op: u64,
    /// Allocations per operation
    pub allocs_per_op: u32,
}

/// Compliance validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceValidationResult {
    /// Overall compliance status
    pub compliant: bool,
    /// License compliance results
    pub license: Option<LicenseComplianceResult>,
    /// Export control results
    pub export_control: Option<ExportControlResult>,
    /// Privacy compliance results
    pub privacy: Option<PrivacyComplianceResult>,
    /// Standards compliance results
    pub standards: Vec<StandardComplianceResult>,
    /// Compliance score (0-100)
    pub score: u32,
    /// Compliance issues
    pub issues: Vec<ComplianceIssue>,
}

/// License compliance result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseComplianceResult {
    /// Primary license
    pub primary_license: String,
    /// All licenses found
    pub licenses_found: Vec<String>,
    /// License compatibility issues
    pub compatibility_issues: Vec<LicenseCompatibilityIssue>,
    /// License conflicts
    pub conflicts: Vec<LicenseConflict>,
}

/// License compatibility issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseCompatibilityIssue {
    /// Source license
    pub source_license: String,
    /// Target license
    pub target_license: String,
    /// Issue description
    pub description: String,
    /// Severity
    pub severity: SecuritySeverity,
    /// Resolution suggestion
    pub resolution: String,
}

/// License conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseConflict {
    /// First conflicting license
    pub license1: String,
    /// Second conflicting license
    pub license2: String,
    /// Conflict reason
    pub reason: String,
    /// Impact
    pub impact: String,
}

/// Export control result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportControlResult {
    /// Export restrictions found
    pub restrictions: Vec<ExportRestriction>,
    /// Controlled technologies detected
    pub controlled_tech: Vec<String>,
    /// Countries with restrictions
    pub restricted_countries: Vec<String>,
    /// Export classification
    pub classification: String,
}

/// Export restriction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRestriction {
    /// Restriction type
    pub restriction_type: String,
    /// Restriction description
    pub description: String,
    /// Applicable countries
    pub countries: Vec<String>,
    /// Legal reference
    pub legal_reference: String,
}

/// Privacy compliance result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyComplianceResult {
    /// Privacy violations found
    pub violations: Vec<PrivacyViolation>,
    /// Data collection analysis
    pub data_collection: DataCollectionAnalysis,
    /// Third-party sharing detected
    pub third_party_sharing: Vec<ThirdPartySharing>,
    /// GDPR compliance status
    pub gdpr_compliant: bool,
    /// CCPA compliance status
    pub ccpa_compliant: bool,
}

/// Privacy violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyViolation {
    /// Violation type
    pub violation_type: String,
    /// Violation description
    pub description: String,
    /// Affected data types
    pub affected_data: Vec<String>,
    /// Severity
    pub severity: ViolationSeverity,
    /// Remediation required
    pub remediation: String,
}

/// Data collection analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCollectionAnalysis {
    /// Types of data collected
    pub data_types: Vec<String>,
    /// Personal data collected
    pub personal_data: bool,
    /// Sensitive data collected
    pub sensitive_data: bool,
    /// Data retention period (days)
    pub retention_period: Option<u32>,
    /// Data encryption used
    pub encryption_used: bool,
}

/// Third-party data sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThirdPartySharing {
    /// Third party name
    pub party: String,
    /// Data shared
    pub data_shared: Vec<String>,
    /// Purpose of sharing
    pub purpose: String,
    /// User consent required
    pub consent_required: bool,
}

/// Violation severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    /// Minor violation
    Minor,
    /// Moderate violation
    Moderate,
    /// Major violation
    Major,
    /// Critical violation
    Critical,
}

/// Standard compliance result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardComplianceResult {
    /// Standard identifier
    pub standard_id: String,
    /// Standard name
    pub standard_name: String,
    /// Compliance status
    pub compliant: bool,
    /// Compliance level achieved
    pub level: ComplianceLevel,
    /// Failed rules
    pub failed_rules: Vec<ComplianceRuleFailure>,
    /// Compliance percentage
    pub compliance_percentage: f64,
}

/// Compliance rule failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRuleFailure {
    /// Rule ID
    pub rule_id: String,
    /// Rule description
    pub description: String,
    /// Failure reason
    pub reason: String,
    /// Remediation required
    pub remediation: String,
    /// Evidence location
    pub evidence: Vec<CodeLocation>,
}

/// Compliance issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceIssue {
    /// Issue type
    pub issue_type: String,
    /// Issue description
    pub description: String,
    /// Affected components
    pub affected_components: Vec<String>,
    /// Issue severity
    pub severity: SecuritySeverity,
    /// Resolution required
    pub resolution: String,
}

/// Validation summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    /// Total issues found
    pub total_issues: usize,
    /// Critical issues
    pub critical_issues: usize,
    /// High severity issues
    pub high_issues: usize,
    /// Medium severity issues
    pub medium_issues: usize,
    /// Low severity issues
    pub low_issues: usize,
    /// Informational issues
    pub info_issues: usize,
    /// Overall risk assessment
    pub risk_level: RiskLevel,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Validation duration (milliseconds)
    pub duration_ms: u64,
}