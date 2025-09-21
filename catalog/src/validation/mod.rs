//! Asset Validation Framework
//!
//! Provides comprehensive validation for asset packages including syntax validation,
//! security analysis, dependency resolution, and compliance checking.
//!
//! # Module Organization
//!
//! - `config` - Validation configuration structures
//! - `results` - Validation result types
//! - `traits` - Core validation traits
//! - `validators` - Type-specific validator implementations
//! - `scanners` - Security scanner implementations
//! - `dependency` - Dependency resolution and analysis
//! - `validator` - Main validator orchestration

pub mod config;
pub mod dependency;
pub mod results;
pub mod scanners;
pub mod traits;
pub mod validator;
pub mod validators;

// Re-export main types for convenience
pub use config::{
    ComplianceLevel, ComplianceRule, ComplianceStandard, ComplianceValidationConfig,
    DependencyValidationConfig, LintCategory, LintRule, LintSeverity,
    PerformanceValidationConfig, SecurityRule, SecuritySeverity, SecurityValidationConfig,
    SyntaxValidationConfig, ValidationConfig,
};

pub use dependency::{Dependency, DependencyGraph, DependencyNode, DependencyResolver, VersionConflict};

pub use results::{
    BenchmarkResults, BenchmarkStatistics, BestPracticeViolation, CodeLocation,
    ComplianceIssue, ComplianceRuleFailure, ComplianceValidationResult, ComplexityAnalysis,
    DataCollectionAnalysis, ExportControlResult, ExportRestriction, HalsteadMetrics,
    InjectionRisk, InjectionType, LicenseComplianceResult, LicenseCompatibilityIssue,
    LicenseConflict, LintingIssue, MalwareDetection, PerformanceImpact, PerformanceIssue,
    PerformanceIssueType, PerformanceValidationResult, PrivacyComplianceResult,
    PrivacyViolation, ResourceUsage, RiskLevel, SecurityRuleFailure, SecurityValidationResult,
    StandardComplianceResult, StyleViolation, SyntaxError, SyntaxValidationResult,
    ThirdPartySharing, ValidationResult, ValidationSummary, ViolationSeverity, Vulnerability,
};

pub use traits::{SecurityScanner, TypeValidator};

pub use validator::AssetValidator;

pub use validators::{JuliaValidator, LuaValidator};

pub use scanners::StaticSecurityScanner;