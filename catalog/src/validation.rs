//! Asset Validation Framework
//!
//! Provides comprehensive validation for asset packages including syntax validation,
//! security analysis, dependency resolution, and compliance checking.

use crate::assets::*;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use chrono::{DateTime, Utc};
use async_trait::async_trait;

/// Asset validator for comprehensive package validation
pub struct AssetValidator {
    /// Validation configuration
    config: ValidationConfig,
    /// Registered validators by asset type
    type_validators: HashMap<String, Box<dyn TypeValidator>>,
    /// Security scanners
    security_scanners: Vec<Box<dyn SecurityScanner>>,
    /// Dependency resolver
    dependency_resolver: DependencyResolver,
}

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
    /// License compatibility checking
    pub check_license_compatibility: bool,
    /// Export control validation
    pub export_control_validation: bool,
    /// Privacy compliance checking
    pub privacy_compliance: bool,
    /// Required compliance standards
    pub required_standards: Vec<ComplianceStandard>,
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
    /// Pattern to match (regex)
    pub pattern: String,
    /// File patterns to apply rule to
    pub file_patterns: Vec<String>,
    /// Whether rule is enabled
    pub enabled: bool,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    /// Critical security issue
    Critical,
    /// High priority security issue
    High,
    /// Medium priority security issue
    Medium,
    /// Low priority security issue
    Low,
    /// Informational security finding
    Info,
}

/// Linting rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintRule {
    /// Rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule category
    pub category: LintCategory,
    /// Rule message
    pub message: String,
    /// Rule pattern (regex)
    pub pattern: String,
    /// Suggested fix
    pub suggested_fix: Option<String>,
    /// Rule enabled
    pub enabled: bool,
}

/// Linting categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LintCategory {
    /// Syntax errors
    Syntax,
    /// Style violations
    Style,
    /// Performance issues
    Performance,
    /// Best practice violations
    BestPractices,
    /// Security concerns
    Security,
    /// Maintainability issues
    Maintainability,
}

/// Compliance standard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStandard {
    /// Standard identifier
    pub id: String,
    /// Standard name
    pub name: String,
    /// Standard version
    pub version: String,
    /// Required compliance level
    pub level: ComplianceLevel,
    /// Validation rules
    pub rules: Vec<ComplianceRule>,
}

/// Compliance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceLevel {
    /// Basic compliance
    Basic,
    /// Standard compliance
    Standard,
    /// Strict compliance
    Strict,
    /// Maximum compliance
    Maximum,
}

/// Compliance rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    /// Rule identifier
    pub id: String,
    /// Rule description
    pub description: String,
    /// Check function name
    pub check_function: String,
    /// Rule parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Whether rule is mandatory
    pub mandatory: bool,
}

/// Comprehensive validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Overall validation success
    pub is_valid: bool,
    /// Validation timestamp
    pub validated_at: DateTime<Utc>,
    /// Validation duration (milliseconds)
    pub validation_time_ms: u64,
    /// Security validation results
    pub security_results: SecurityValidationResult,
    /// Syntax validation results
    pub syntax_results: SyntaxValidationResult,
    /// Dependency validation results
    pub dependency_results: DependencyValidationResults,
    /// Performance validation results
    pub performance_results: PerformanceValidationResult,
    /// Compliance validation results
    pub compliance_results: ComplianceValidationResult,
    /// Overall validation score (0-100)
    pub overall_score: u32,
    /// Validation summary
    pub summary: ValidationSummary,
}

/// Security validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityValidationResult {
    /// Security score (0-100)
    pub security_score: u32,
    /// Vulnerabilities found
    pub vulnerabilities: Vec<SecurityVulnerability>,
    /// Malware detections
    pub malware_detections: Vec<MalwareDetection>,
    /// Code injection risks
    pub injection_risks: Vec<InjectionRisk>,
    /// Security recommendations
    pub recommendations: Vec<String>,
    /// Passed security rules
    pub passed_rules: Vec<String>,
    /// Failed security rules
    pub failed_rules: Vec<SecurityRuleFailure>,
}

/// Malware detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MalwareDetection {
    /// Malware type
    pub malware_type: String,
    /// Detection confidence (0.0-1.0)
    pub confidence: f64,
    /// Affected file
    pub file: String,
    /// Detection signature
    pub signature: String,
    /// Remediation advice
    pub remediation: String,
}

/// Code injection risk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionRisk {
    /// Injection type
    pub injection_type: InjectionType,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Affected location
    pub location: CodeLocation,
    /// Risk description
    pub description: String,
    /// Mitigation advice
    pub mitigation: String,
}

/// Types of code injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjectionType {
    /// SQL injection
    SqlInjection,
    /// Command injection
    CommandInjection,
    /// Code injection
    CodeInjection,
    /// Path traversal
    PathTraversal,
    /// XSS (Cross-site scripting)
    Xss,
    /// LDAP injection
    LdapInjection,
    /// NoSQL injection
    NoSqlInjection,
}

/// Risk levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Critical risk
    Critical,
    /// High risk
    High,
    /// Medium risk
    Medium,
    /// Low risk
    Low,
    /// Informational
    Info,
}

/// Code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    /// File path
    pub file: String,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Code snippet
    pub snippet: String,
}

/// Security rule failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRuleFailure {
    /// Rule that failed
    pub rule: SecurityRule,
    /// Failure locations
    pub locations: Vec<CodeLocation>,
    /// Failure count
    pub count: u32,
}

/// Syntax validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxValidationResult {
    /// Syntax errors found
    pub syntax_errors: Vec<SyntaxError>,
    /// Style violations
    pub style_violations: Vec<StyleViolation>,
    /// Best practice violations
    pub best_practice_violations: Vec<BestPracticeViolation>,
    /// Linting issues
    pub linting_issues: Vec<LintingIssue>,
    /// Code quality score (0-100)
    pub quality_score: u32,
}

/// Syntax error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxError {
    /// Error message
    pub message: String,
    /// Error code
    pub code: String,
    /// Error location
    pub location: CodeLocation,
    /// Error severity
    pub severity: ErrorSeverity,
    /// Suggested fix
    pub suggested_fix: Option<String>,
}

/// Style violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleViolation {
    /// Violation message
    pub message: String,
    /// Violated rule
    pub rule: String,
    /// Violation location
    pub location: CodeLocation,
    /// Suggested fix
    pub suggested_fix: Option<String>,
}

/// Best practice violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestPracticeViolation {
    /// Violation message
    pub message: String,
    /// Practice category
    pub category: String,
    /// Violation location
    pub location: CodeLocation,
    /// Impact description
    pub impact: String,
    /// Recommendation
    pub recommendation: String,
}

/// Linting issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintingIssue {
    /// Issue message
    pub message: String,
    /// Lint rule that triggered
    pub rule: LintRule,
    /// Issue location
    pub location: CodeLocation,
    /// Issue severity
    pub severity: LintSeverity,
}

/// Lint severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LintSeverity {
    /// Error level
    Error,
    /// Warning level
    Warning,
    /// Info level
    Info,
    /// Suggestion level
    Suggestion,
}

/// Performance validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceValidationResult {
    /// Estimated execution time (seconds)
    pub estimated_execution_time: f64,
    /// Estimated memory usage (bytes)
    pub estimated_memory_usage: u64,
    /// Estimated CPU usage
    pub estimated_cpu_usage: f64,
    /// Complexity analysis
    pub complexity_analysis: ComplexityAnalysis,
    /// Performance issues
    pub performance_issues: Vec<PerformanceIssue>,
    /// Performance score (0-100)
    pub performance_score: u32,
    /// Benchmark results
    pub benchmark_results: Option<BenchmarkResults>,
}

/// Complexity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityAnalysis {
    /// Cyclomatic complexity
    pub cyclomatic_complexity: u32,
    /// Cognitive complexity
    pub cognitive_complexity: u32,
    /// Halstead complexity metrics
    pub halstead_metrics: HalsteadMetrics,
    /// Lines of code
    pub lines_of_code: u32,
    /// Number of functions
    pub function_count: u32,
}

/// Halstead complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalsteadMetrics {
    /// Vocabulary size
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
    /// Number of bugs estimate
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
    pub optimization: String,
}

/// Performance issue types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceIssueType {
    /// Inefficient algorithm
    InefficiientAlgorithm,
    /// Memory leak potential
    MemoryLeak,
    /// Excessive memory allocation
    ExcessiveAllocation,
    /// CPU-intensive operation
    CpuIntensive,
    /// I/O bottleneck
    IoBottleneck,
    /// Inefficient data structure
    InefficiientDataStructure,
}

/// Performance impact levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceImpact {
    /// Critical performance impact
    Critical,
    /// High performance impact
    High,
    /// Medium performance impact
    Medium,
    /// Low performance impact
    Low,
    /// Negligible impact
    Negligible,
}

/// Benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    /// Execution time measurements
    pub execution_times: Vec<f64>,
    /// Memory usage measurements
    pub memory_usage: Vec<u64>,
    /// CPU usage measurements
    pub cpu_usage: Vec<f64>,
    /// Throughput measurements
    pub throughput: Vec<f64>,
    /// Statistical summary
    pub statistics: BenchmarkStatistics,
}

/// Benchmark statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkStatistics {
    /// Mean execution time
    pub mean_execution_time: f64,
    /// Median execution time
    pub median_execution_time: f64,
    /// Standard deviation
    pub std_deviation: f64,
    /// 95th percentile
    pub percentile_95: f64,
    /// 99th percentile
    pub percentile_99: f64,
}

/// Compliance validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceValidationResult {
    /// Overall compliance status
    pub compliant: bool,
    /// Compliance score (0-100)
    pub compliance_score: u32,
    /// License compliance results
    pub license_compliance: LicenseComplianceResult,
    /// Export control results
    pub export_control: ExportControlResult,
    /// Privacy compliance results
    pub privacy_compliance: PrivacyComplianceResult,
    /// Standards compliance results
    pub standards_compliance: Vec<StandardComplianceResult>,
}

/// License compliance result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseComplianceResult {
    /// License compatibility issues
    pub compatibility_issues: Vec<LicenseCompatibilityIssue>,
    /// Missing license information
    pub missing_licenses: Vec<String>,
    /// Conflicting licenses
    pub license_conflicts: Vec<LicenseConflict>,
    /// Compliance status
    pub compliant: bool,
}

/// License compatibility issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseCompatibilityIssue {
    /// Component with incompatible license
    pub component: String,
    /// Component license
    pub component_license: String,
    /// Project license
    pub project_license: String,
    /// Compatibility issue description
    pub issue: String,
    /// Suggested resolution
    pub resolution: String,
}

/// License conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseConflict {
    /// First conflicting license
    pub license_a: String,
    /// Second conflicting license
    pub license_b: String,
    /// Conflict description
    pub conflict: String,
    /// Resolution options
    pub resolution_options: Vec<String>,
}

/// Export control result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportControlResult {
    /// Export control classification
    pub classification: Option<String>,
    /// Export restrictions
    pub restrictions: Vec<ExportRestriction>,
    /// Compliance status
    pub compliant: bool,
    /// Required notices
    pub required_notices: Vec<String>,
}

/// Export restriction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRestriction {
    /// Restriction type
    pub restriction_type: String,
    /// Restricted countries/entities
    pub restricted_targets: Vec<String>,
    /// Restriction description
    pub description: String,
    /// Compliance requirements
    pub requirements: Vec<String>,
}

/// Privacy compliance result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyComplianceResult {
    /// Privacy regulations checked
    pub regulations: Vec<String>,
    /// Privacy violations found
    pub violations: Vec<PrivacyViolation>,
    /// Data collection practices
    pub data_collection: DataCollectionAnalysis,
    /// Compliance status
    pub compliant: bool,
}

/// Privacy violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyViolation {
    /// Violation type
    pub violation_type: String,
    /// Affected regulation
    pub regulation: String,
    /// Violation description
    pub description: String,
    /// Required remediation
    pub remediation: String,
    /// Severity level
    pub severity: ViolationSeverity,
}

/// Data collection analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCollectionAnalysis {
    /// Types of data collected
    pub data_types: Vec<String>,
    /// Collection methods
    pub collection_methods: Vec<String>,
    /// Storage locations
    pub storage_locations: Vec<String>,
    /// Retention periods
    pub retention_periods: HashMap<String, String>,
    /// Third-party sharing
    pub third_party_sharing: Vec<ThirdPartySharing>,
}

/// Third-party data sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThirdPartySharing {
    /// Third party name
    pub party: String,
    /// Shared data types
    pub data_types: Vec<String>,
    /// Sharing purpose
    pub purpose: String,
    /// User consent required
    pub consent_required: bool,
}

/// Violation severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Critical violation
    Critical,
    /// Major violation
    Major,
    /// Minor violation
    Minor,
    /// Warning level
    Warning,
}

/// Standards compliance result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardComplianceResult {
    /// Standard that was checked
    pub standard: ComplianceStandard,
    /// Compliance status
    pub compliant: bool,
    /// Compliance level achieved
    pub level_achieved: ComplianceLevel,
    /// Failed rules
    pub failed_rules: Vec<ComplianceRuleFailure>,
    /// Compliance percentage
    pub compliance_percentage: f64,
}

/// Compliance rule failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRuleFailure {
    /// Failed rule
    pub rule: ComplianceRule,
    /// Failure reason
    pub reason: String,
    /// Evidence/details
    pub evidence: String,
    /// Required actions
    pub required_actions: Vec<String>,
}

/// Validation summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    /// Total issues found
    pub total_issues: u32,
    /// Critical issues
    pub critical_issues: u32,
    /// High priority issues
    pub high_priority_issues: u32,
    /// Medium priority issues
    pub medium_priority_issues: u32,
    /// Low priority issues
    pub low_priority_issues: u32,
    /// Top recommendations
    pub top_recommendations: Vec<String>,
    /// Validation categories passed
    pub categories_passed: Vec<String>,
    /// Validation categories failed
    pub categories_failed: Vec<String>,
}

/// Type-specific validator trait
#[async_trait]
pub trait TypeValidator: Send + Sync {
    /// Asset type this validator handles
    fn asset_type(&self) -> &str;
    
    /// Validate syntax for the specific asset type
    async fn validate_syntax(&self, package: &AssetPackage) -> Result<SyntaxValidationResult>;
    
    /// Analyze performance characteristics
    async fn analyze_performance(&self, package: &AssetPackage) -> Result<PerformanceValidationResult>;
    
    /// Validate type-specific security concerns
    async fn validate_security(&self, package: &AssetPackage) -> Result<SecurityValidationResult>;
}

/// Security scanner trait
#[async_trait]
pub trait SecurityScanner: Send + Sync {
    /// Scanner name
    fn name(&self) -> &str;
    
    /// Scan for vulnerabilities
    async fn scan_vulnerabilities(&self, package: &AssetPackage) -> Result<Vec<SecurityVulnerability>>;
    
    /// Detect malware
    async fn detect_malware(&self, package: &AssetPackage) -> Result<Vec<MalwareDetection>>;
    
    /// Analyze injection risks
    async fn analyze_injection_risks(&self, package: &AssetPackage) -> Result<Vec<InjectionRisk>>;
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
                security_rules: Self::default_security_rules(),
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
                    "https://registry.hypermesh.online".to_string(),
                    "https://github.com".to_string(),
                ],
            },
            performance: PerformanceValidationConfig {
                analyze_resource_usage: true,
                analyze_complexity: true,
                max_execution_time: 300, // 5 minutes
                max_memory_usage: 2 * 1024 * 1024 * 1024, // 2GB
                enable_benchmarking: false,
            },
            compliance: ComplianceValidationConfig {
                check_license_compatibility: true,
                export_control_validation: false,
                privacy_compliance: true,
                required_standards: vec![],
            },
        }
    }
}

impl ValidationConfig {
    /// Get default security rules
    fn default_security_rules() -> Vec<SecurityRule> {
        vec![
            SecurityRule {
                id: "SEC001".to_string(),
                name: "No hardcoded passwords".to_string(),
                description: "Detect hardcoded passwords in source code".to_string(),
                severity: SecuritySeverity::Critical,
                pattern: r#"(?i)(password|passwd|pwd)\s*[:=]\s*["'][^"']{3,}["']"#.to_string(),
                file_patterns: vec!["*.jl".to_string(), "*.lua".to_string(), "*.py".to_string()],
                enabled: true,
            },
            SecurityRule {
                id: "SEC002".to_string(),
                name: "No API keys in code".to_string(),
                description: "Detect API keys and secrets in source code".to_string(),
                severity: SecuritySeverity::Critical,
                pattern: r#"(?i)(api[_-]?key|secret[_-]?key|access[_-]?token)\s*[:=]\s*["'][A-Za-z0-9+/]{20,}["']"#.to_string(),
                file_patterns: vec!["*.jl".to_string(), "*.lua".to_string(), "*.py".to_string()],
                enabled: true,
            },
            SecurityRule {
                id: "SEC003".to_string(),
                name: "No system command injection".to_string(),
                description: "Detect potential command injection vulnerabilities".to_string(),
                severity: SecuritySeverity::High,
                pattern: r#"(?i)(system|exec|shell|popen|spawn)\s*\([^)]*\$|[`]"#.to_string(),
                file_patterns: vec!["*.jl".to_string(), "*.lua".to_string()],
                enabled: true,
            },
        ]
    }
}

impl AssetValidator {
    /// Create a new asset validator
    pub fn new(config: ValidationConfig) -> Self {
        let mut validator = Self {
            config,
            type_validators: HashMap::new(),
            security_scanners: vec![],
            dependency_resolver: DependencyResolver::new(),
        };
        
        // Register built-in type validators
        validator.register_type_validator(Box::new(JuliaValidator::new()));
        validator.register_type_validator(Box::new(LuaValidator::new()));
        
        // Register built-in security scanners
        validator.register_security_scanner(Box::new(StaticSecurityScanner::new()));
        
        validator
    }
    
    /// Register a type-specific validator
    pub fn register_type_validator(&mut self, validator: Box<dyn TypeValidator>) {
        let asset_type = validator.asset_type().to_string();
        self.type_validators.insert(asset_type, validator);
    }
    
    /// Register a security scanner
    pub fn register_security_scanner(&mut self, scanner: Box<dyn SecurityScanner>) {
        self.security_scanners.push(scanner);
    }
    
    /// Validate an asset package comprehensively
    pub async fn validate(&self, package: &AssetPackage) -> Result<ValidationResult> {
        let start_time = std::time::Instant::now();
        
        // Run all validation categories in parallel
        let (security_results, syntax_results, dependency_results, performance_results, compliance_results) = tokio::try_join!(
            self.validate_security(package),
            self.validate_syntax(package),
            self.validate_dependencies(package),
            self.validate_performance(package),
            self.validate_compliance(package)
        )?;
        
        let validation_time = start_time.elapsed().as_millis() as u64;
        
        // Calculate overall score
        let overall_score = self.calculate_overall_score(
            &security_results,
            &syntax_results,
            &dependency_results,
            &performance_results,
            &compliance_results,
        );
        
        // Generate summary
        let summary = self.generate_summary(
            &security_results,
            &syntax_results,
            &dependency_results,
            &performance_results,
            &compliance_results,
        );
        
        // Determine if package is valid
        let is_valid = self.determine_validity(
            &security_results,
            &syntax_results,
            &dependency_results,
            &performance_results,
            &compliance_results,
        );
        
        Ok(ValidationResult {
            is_valid,
            validated_at: Utc::now(),
            validation_time_ms: validation_time,
            security_results,
            syntax_results,
            dependency_results,
            performance_results,
            compliance_results,
            overall_score,
            summary,
        })
    }
    
    /// Validate security aspects
    async fn validate_security(&self, package: &AssetPackage) -> Result<SecurityValidationResult> {
        let mut vulnerabilities = Vec::new();
        let mut malware_detections = Vec::new();
        let mut injection_risks = Vec::new();
        
        // Run security scanners
        for scanner in &self.security_scanners {
            vulnerabilities.extend(scanner.scan_vulnerabilities(package).await?);
            malware_detections.extend(scanner.detect_malware(package).await?);
            injection_risks.extend(scanner.analyze_injection_risks(package).await?);
        }
        
        // Apply security rules
        let (passed_rules, failed_rules) = self.apply_security_rules(package).await?;
        
        // Calculate security score
        let security_score = self.calculate_security_score(
            &vulnerabilities,
            &malware_detections,
            &injection_risks,
            &failed_rules,
        );
        
        // Generate recommendations
        let recommendations = self.generate_security_recommendations(
            &vulnerabilities,
            &injection_risks,
            &failed_rules,
        );
        
        Ok(SecurityValidationResult {
            security_score,
            vulnerabilities,
            malware_detections,
            injection_risks,
            recommendations,
            passed_rules,
            failed_rules,
        })
    }
    
    /// Validate syntax and code quality
    async fn validate_syntax(&self, package: &AssetPackage) -> Result<SyntaxValidationResult> {
        // Get type-specific validator
        if let Some(validator) = self.type_validators.get(&package.spec.spec.asset_type) {
            validator.validate_syntax(package).await
        } else {
            // Default syntax validation
            Ok(SyntaxValidationResult {
                syntax_errors: vec![],
                style_violations: vec![],
                best_practice_violations: vec![],
                linting_issues: vec![],
                quality_score: 85, // Default score for unknown types
            })
        }
    }
    
    /// Validate dependencies
    async fn validate_dependencies(&self, package: &AssetPackage) -> Result<DependencyValidationResults> {
        self.dependency_resolver.validate_dependencies(&package.spec.spec.dependencies).await
    }
    
    /// Validate performance characteristics
    async fn validate_performance(&self, package: &AssetPackage) -> Result<PerformanceValidationResult> {
        // Get type-specific validator
        if let Some(validator) = self.type_validators.get(&package.spec.spec.asset_type) {
            validator.analyze_performance(package).await
        } else {
            // Default performance analysis
            Ok(PerformanceValidationResult {
                estimated_execution_time: 30.0,
                estimated_memory_usage: 512 * 1024 * 1024, // 512MB
                estimated_cpu_usage: 50.0,
                complexity_analysis: ComplexityAnalysis {
                    cyclomatic_complexity: 5,
                    cognitive_complexity: 8,
                    halstead_metrics: HalsteadMetrics {
                        vocabulary: 50,
                        length: 200,
                        calculated_length: 180.0,
                        volume: 1200.0,
                        difficulty: 15.0,
                        effort: 18000.0,
                        time: 1000.0,
                        bugs: 0.4,
                    },
                    lines_of_code: 100,
                    function_count: 5,
                },
                performance_issues: vec![],
                performance_score: 75,
                benchmark_results: None,
            })
        }
    }
    
    /// Validate compliance requirements
    async fn validate_compliance(&self, _package: &AssetPackage) -> Result<ComplianceValidationResult> {
        // TODO: Implement comprehensive compliance validation
        Ok(ComplianceValidationResult {
            compliant: true,
            compliance_score: 90,
            license_compliance: LicenseComplianceResult {
                compatibility_issues: vec![],
                missing_licenses: vec![],
                license_conflicts: vec![],
                compliant: true,
            },
            export_control: ExportControlResult {
                classification: None,
                restrictions: vec![],
                compliant: true,
                required_notices: vec![],
            },
            privacy_compliance: PrivacyComplianceResult {
                regulations: vec!["GDPR".to_string()],
                violations: vec![],
                data_collection: DataCollectionAnalysis {
                    data_types: vec![],
                    collection_methods: vec![],
                    storage_locations: vec![],
                    retention_periods: HashMap::new(),
                    third_party_sharing: vec![],
                },
                compliant: true,
            },
            standards_compliance: vec![],
        })
    }
    
    /// Apply security rules to package
    async fn apply_security_rules(&self, package: &AssetPackage) -> Result<(Vec<String>, Vec<SecurityRuleFailure>)> {
        let mut passed_rules = Vec::new();
        let mut failed_rules = Vec::new();
        
        for rule in &self.config.security.security_rules {
            if !rule.enabled {
                continue;
            }
            
            let regex = regex::Regex::new(&rule.pattern)?;
            let mut failures = Vec::new();
            
            // Check main content
            if self.matches_file_patterns(&rule.file_patterns, "main") {
                let matches: Vec<_> = regex.find_iter(&package.content.main_content).collect();
                for m in matches {
                    let line_num = package.content.main_content[..m.start()].matches('\n').count() as u32 + 1;
                    failures.push(CodeLocation {
                        file: "main".to_string(),
                        line: line_num,
                        column: 0,
                        snippet: m.as_str().to_string(),
                    });
                }
            }
            
            // Check additional files
            for (file_path, content) in &package.content.file_contents {
                if self.matches_file_patterns(&rule.file_patterns, file_path) {
                    let matches: Vec<_> = regex.find_iter(content).collect();
                    for m in matches {
                        let line_num = content[..m.start()].matches('\n').count() as u32 + 1;
                        failures.push(CodeLocation {
                            file: file_path.clone(),
                            line: line_num,
                            column: 0,
                            snippet: m.as_str().to_string(),
                        });
                    }
                }
            }
            
            if failures.is_empty() {
                passed_rules.push(rule.id.clone());
            } else {
                failed_rules.push(SecurityRuleFailure {
                    rule: rule.clone(),
                    locations: failures.clone(),
                    count: failures.len() as u32,
                });
            }
        }
        
        Ok((passed_rules, failed_rules))
    }
    
    /// Check if file matches patterns
    fn matches_file_patterns(&self, patterns: &[String], file_path: &str) -> bool {
        if patterns.is_empty() {
            return true; // No patterns means match all
        }
        
        for pattern in patterns {
            if glob::Pattern::new(pattern).map(|p| p.matches(file_path)).unwrap_or(false) {
                return true;
            }
        }
        
        false
    }
    
    /// Calculate security score
    fn calculate_security_score(
        &self,
        vulnerabilities: &[SecurityVulnerability],
        malware_detections: &[MalwareDetection],
        injection_risks: &[InjectionRisk],
        failed_rules: &[SecurityRuleFailure],
    ) -> u32 {
        let mut score = 100u32;
        
        // Deduct for vulnerabilities
        for vuln in vulnerabilities {
            match vuln.severity {
                VulnerabilitySeverity::Critical => score = score.saturating_sub(25),
                VulnerabilitySeverity::High => score = score.saturating_sub(15),
                VulnerabilitySeverity::Medium => score = score.saturating_sub(10),
                VulnerabilitySeverity::Low => score = score.saturating_sub(5),
                VulnerabilitySeverity::Info => score = score.saturating_sub(1),
            }
        }
        
        // Deduct for malware
        score = score.saturating_sub(malware_detections.len() as u32 * 30);
        
        // Deduct for injection risks
        for risk in injection_risks {
            match risk.risk_level {
                RiskLevel::Critical => score = score.saturating_sub(20),
                RiskLevel::High => score = score.saturating_sub(15),
                RiskLevel::Medium => score = score.saturating_sub(10),
                RiskLevel::Low => score = score.saturating_sub(5),
                RiskLevel::Info => score = score.saturating_sub(1),
            }
        }
        
        // Deduct for failed security rules
        for failure in failed_rules {
            match failure.rule.severity {
                SecuritySeverity::Critical => score = score.saturating_sub(20),
                SecuritySeverity::High => score = score.saturating_sub(15),
                SecuritySeverity::Medium => score = score.saturating_sub(10),
                SecuritySeverity::Low => score = score.saturating_sub(5),
                SecuritySeverity::Info => score = score.saturating_sub(1),
            }
        }
        
        score
    }
    
    /// Generate security recommendations
    fn generate_security_recommendations(
        &self,
        vulnerabilities: &[SecurityVulnerability],
        injection_risks: &[InjectionRisk],
        failed_rules: &[SecurityRuleFailure],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if !vulnerabilities.is_empty() {
            recommendations.push("Address identified security vulnerabilities".to_string());
        }
        
        if !injection_risks.is_empty() {
            recommendations.push("Implement input validation to prevent injection attacks".to_string());
        }
        
        if !failed_rules.is_empty() {
            recommendations.push("Fix security rule violations".to_string());
        }
        
        recommendations
    }
    
    /// Calculate overall validation score
    fn calculate_overall_score(
        &self,
        security: &SecurityValidationResult,
        syntax: &SyntaxValidationResult,
        dependency: &DependencyValidationResults,
        performance: &PerformanceValidationResult,
        compliance: &ComplianceValidationResult,
    ) -> u32 {
        // Weighted average of all scores
        let weights = [30, 25, 20, 15, 10]; // Security, syntax, dependency, performance, compliance
        let scores = [
            security.security_score,
            syntax.quality_score,
            if dependency.dependencies_valid { 100 } else { 50 },
            performance.performance_score,
            compliance.compliance_score,
        ];
        
        let weighted_sum: u32 = weights.iter().zip(scores.iter()).map(|(w, s)| w * s).sum();
        let total_weight: u32 = weights.iter().sum();
        
        weighted_sum / total_weight
    }
    
    /// Generate validation summary
    fn generate_summary(
        &self,
        security: &SecurityValidationResult,
        syntax: &SyntaxValidationResult,
        dependency: &DependencyValidationResults,
        performance: &PerformanceValidationResult,
        compliance: &ComplianceValidationResult,
    ) -> ValidationSummary {
        let mut total_issues = 0u32;
        let mut critical_issues = 0u32;
        let mut high_priority_issues = 0u32;
        let mut medium_priority_issues = 0u32;
        let mut low_priority_issues = 0u32;
        
        // Count security issues
        for vuln in &security.vulnerabilities {
            total_issues += 1;
            match vuln.severity {
                VulnerabilitySeverity::Critical => critical_issues += 1,
                VulnerabilitySeverity::High => high_priority_issues += 1,
                VulnerabilitySeverity::Medium => medium_priority_issues += 1,
                VulnerabilitySeverity::Low => low_priority_issues += 1,
                VulnerabilitySeverity::Info => {}
            }
        }
        
        // Count syntax issues
        total_issues += syntax.syntax_errors.len() as u32;
        critical_issues += syntax.syntax_errors.iter()
            .filter(|e| matches!(e.severity, ErrorSeverity::Critical))
            .count() as u32;
        
        let mut categories_passed = Vec::new();
        let mut categories_failed = Vec::new();
        
        if security.security_score >= 70 { categories_passed.push("Security".to_string()); } else { categories_failed.push("Security".to_string()); }
        if syntax.quality_score >= 70 { categories_passed.push("Syntax".to_string()); } else { categories_failed.push("Syntax".to_string()); }
        if dependency.dependencies_valid { categories_passed.push("Dependencies".to_string()); } else { categories_failed.push("Dependencies".to_string()); }
        if performance.performance_score >= 70 { categories_passed.push("Performance".to_string()); } else { categories_failed.push("Performance".to_string()); }
        if compliance.compliant { categories_passed.push("Compliance".to_string()); } else { categories_failed.push("Compliance".to_string()); }
        
        ValidationSummary {
            total_issues,
            critical_issues,
            high_priority_issues,
            medium_priority_issues,
            low_priority_issues,
            top_recommendations: vec![
                "Address critical security vulnerabilities".to_string(),
                "Fix syntax errors and improve code quality".to_string(),
                "Resolve dependency conflicts".to_string(),
            ],
            categories_passed,
            categories_failed,
        }
    }
    
    /// Determine if package is valid for execution
    fn determine_validity(
        &self,
        security: &SecurityValidationResult,
        syntax: &SyntaxValidationResult,
        dependency: &DependencyValidationResults,
        _performance: &PerformanceValidationResult,
        compliance: &ComplianceValidationResult,
    ) -> bool {
        // Package is valid if:
        // 1. Security score meets minimum threshold
        // 2. No critical syntax errors
        // 3. Dependencies are valid
        // 4. Compliant with requirements
        
        security.security_score >= self.config.security.minimum_security_score &&
        !syntax.syntax_errors.iter().any(|e| matches!(e.severity, ErrorSeverity::Critical)) &&
        dependency.dependencies_valid &&
        compliance.compliant
    }
}

/// Dependency resolver for validating asset dependencies
pub struct DependencyResolver;

impl DependencyResolver {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn validate_dependencies(&self, dependencies: &[AssetDependency]) -> Result<DependencyValidationResults> {
        // TODO: Implement comprehensive dependency validation
        Ok(DependencyValidationResults {
            dependencies_valid: true,
            total_dependencies: dependencies.len(),
            valid_dependencies: dependencies.len(),
            invalid_dependencies: vec![],
            conflicts: vec![],
            validated_at: Utc::now(),
        })
    }
}

/// Julia-specific validator
pub struct JuliaValidator;

impl JuliaValidator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TypeValidator for JuliaValidator {
    fn asset_type(&self) -> &str {
        "julia-program"
    }
    
    async fn validate_syntax(&self, package: &AssetPackage) -> Result<SyntaxValidationResult> {
        // TODO: Implement Julia syntax validation
        Ok(SyntaxValidationResult {
            syntax_errors: vec![],
            style_violations: vec![],
            best_practice_violations: vec![],
            linting_issues: vec![],
            quality_score: 85,
        })
    }
    
    async fn analyze_performance(&self, _package: &AssetPackage) -> Result<PerformanceValidationResult> {
        // TODO: Implement Julia performance analysis
        Ok(PerformanceValidationResult {
            estimated_execution_time: 30.0,
            estimated_memory_usage: 512 * 1024 * 1024,
            estimated_cpu_usage: 60.0,
            complexity_analysis: ComplexityAnalysis {
                cyclomatic_complexity: 8,
                cognitive_complexity: 12,
                halstead_metrics: HalsteadMetrics {
                    vocabulary: 75,
                    length: 300,
                    calculated_length: 280.0,
                    volume: 1800.0,
                    difficulty: 20.0,
                    effort: 36000.0,
                    time: 2000.0,
                    bugs: 0.6,
                },
                lines_of_code: 150,
                function_count: 8,
            },
            performance_issues: vec![],
            performance_score: 80,
            benchmark_results: None,
        })
    }
    
    async fn validate_security(&self, _package: &AssetPackage) -> Result<SecurityValidationResult> {
        // TODO: Implement Julia-specific security validation
        Ok(SecurityValidationResult {
            security_score: 85,
            vulnerabilities: vec![],
            malware_detections: vec![],
            injection_risks: vec![],
            recommendations: vec![],
            passed_rules: vec![],
            failed_rules: vec![],
        })
    }
}

/// Lua-specific validator
pub struct LuaValidator;

impl LuaValidator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TypeValidator for LuaValidator {
    fn asset_type(&self) -> &str {
        "lua-script"
    }
    
    async fn validate_syntax(&self, _package: &AssetPackage) -> Result<SyntaxValidationResult> {
        // TODO: Implement Lua syntax validation
        Ok(SyntaxValidationResult {
            syntax_errors: vec![],
            style_violations: vec![],
            best_practice_violations: vec![],
            linting_issues: vec![],
            quality_score: 90,
        })
    }
    
    async fn analyze_performance(&self, _package: &AssetPackage) -> Result<PerformanceValidationResult> {
        // TODO: Implement Lua performance analysis
        Ok(PerformanceValidationResult {
            estimated_execution_time: 10.0,
            estimated_memory_usage: 64 * 1024 * 1024,
            estimated_cpu_usage: 30.0,
            complexity_analysis: ComplexityAnalysis {
                cyclomatic_complexity: 3,
                cognitive_complexity: 5,
                halstead_metrics: HalsteadMetrics {
                    vocabulary: 40,
                    length: 120,
                    calculated_length: 110.0,
                    volume: 600.0,
                    difficulty: 8.0,
                    effort: 4800.0,
                    time: 267.0,
                    bugs: 0.2,
                },
                lines_of_code: 60,
                function_count: 3,
            },
            performance_issues: vec![],
            performance_score: 85,
            benchmark_results: None,
        })
    }
    
    async fn validate_security(&self, _package: &AssetPackage) -> Result<SecurityValidationResult> {
        // TODO: Implement Lua-specific security validation
        Ok(SecurityValidationResult {
            security_score: 80,
            vulnerabilities: vec![],
            malware_detections: vec![],
            injection_risks: vec![],
            recommendations: vec![],
            passed_rules: vec![],
            failed_rules: vec![],
        })
    }
}

/// Static security scanner implementation
pub struct StaticSecurityScanner;

impl StaticSecurityScanner {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SecurityScanner for StaticSecurityScanner {
    fn name(&self) -> &str {
        "Static Security Scanner"
    }
    
    async fn scan_vulnerabilities(&self, _package: &AssetPackage) -> Result<Vec<SecurityVulnerability>> {
        // TODO: Implement vulnerability scanning
        Ok(vec![])
    }
    
    async fn detect_malware(&self, _package: &AssetPackage) -> Result<Vec<MalwareDetection>> {
        // TODO: Implement malware detection
        Ok(vec![])
    }
    
    async fn analyze_injection_risks(&self, _package: &AssetPackage) -> Result<Vec<InjectionRisk>> {
        // TODO: Implement injection risk analysis
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_validator_creation() {
        let config = ValidationConfig::default();
        let validator = AssetValidator::new(config);
        
        // Test that validators are registered
        assert!(validator.type_validators.contains_key("julia-program"));
        assert!(validator.type_validators.contains_key("lua-script"));
        assert!(!validator.security_scanners.is_empty());
    }
}