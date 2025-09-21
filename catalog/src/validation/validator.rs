//! Asset Validator
//!
//! Main validator implementation that orchestrates all validation components.

use anyhow::{Result, Context};
use chrono::Utc;
use std::collections::HashMap;
use std::time::Instant;

use crate::assets::Asset;
use super::config::{ValidationConfig, SecuritySeverity};
use super::dependency::{DependencyResolver, VersionConflict};
use super::results::{
    ValidationResult, ValidationSummary, SecurityValidationResult,
    SyntaxValidationResult, PerformanceValidationResult,
    ComplianceValidationResult, RiskLevel, ResourceUsage,
    ComplexityAnalysis, HalsteadMetrics
};
use super::scanners::StaticSecurityScanner;
use super::traits::{SecurityScanner, TypeValidator};
use super::validators::{JuliaValidator, LuaValidator};

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

impl AssetValidator {
    /// Create new validator with default configuration
    pub fn new() -> Self {
        Self::with_config(ValidationConfig::default())
    }

    /// Create new validator with specific configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        let mut validator = Self {
            config,
            type_validators: HashMap::new(),
            security_scanners: Vec::new(),
            dependency_resolver: DependencyResolver::new(),
        };

        // Register default validators
        validator.register_default_validators();
        validator.register_default_scanners();

        validator
    }

    /// Register default type validators
    fn register_default_validators(&mut self) {
        let julia = Box::new(JuliaValidator::new());
        for type_name in julia.supported_types() {
            self.type_validators.insert(type_name, julia.clone());
        }

        let lua = Box::new(LuaValidator::new());
        for type_name in lua.supported_types() {
            self.type_validators.insert(type_name, lua.clone());
        }
    }

    /// Register default security scanners
    fn register_default_scanners(&mut self) {
        self.security_scanners.push(Box::new(StaticSecurityScanner::new()));
    }

    /// Register a custom type validator
    pub fn register_validator(&mut self, asset_type: String, validator: Box<dyn TypeValidator>) {
        self.type_validators.insert(asset_type, validator);
    }

    /// Register a custom security scanner
    pub fn register_scanner(&mut self, scanner: Box<dyn SecurityScanner>) {
        self.security_scanners.push(scanner);
    }

    /// Validate an asset package
    pub async fn validate(&self, asset: &Asset) -> Result<ValidationResult> {
        let start = Instant::now();
        let mut security_result = None;
        let mut syntax_result = None;
        let mut performance_result = None;
        let mut compliance_result = None;

        // Syntax validation
        if self.config.syntax.strict_syntax || self.config.syntax.style_checking {
            syntax_result = Some(self.validate_syntax(asset).await?);
        }

        // Security validation
        if self.config.security.enable_vulnerability_scan
            || self.config.security.enable_malware_detection
            || self.config.security.enable_injection_detection
        {
            security_result = Some(self.validate_security(asset).await?);
        }

        // Performance validation
        if self.config.performance.analyze_resource_usage
            || self.config.performance.analyze_complexity
        {
            performance_result = Some(self.validate_performance(asset).await?);
        }

        // Compliance validation
        if self.config.compliance.check_license_compliance
            || self.config.compliance.check_export_control
            || self.config.compliance.check_privacy_compliance
        {
            compliance_result = Some(self.validate_compliance(asset).await?);
        }

        // Calculate summary
        let summary = self.calculate_summary(
            &security_result,
            &syntax_result,
            &performance_result,
            &compliance_result,
            start.elapsed().as_millis() as u64,
        );

        // Determine if validation passed
        let passed = self.check_passed(
            &security_result,
            &syntax_result,
            &performance_result,
            &compliance_result,
        );

        Ok(ValidationResult {
            passed,
            timestamp: Utc::now(),
            asset_id: asset.id.to_string(),
            version: asset.version.clone(),
            security: security_result,
            syntax: syntax_result,
            performance: performance_result,
            compliance: compliance_result,
            summary,
        })
    }

    /// Validate asset syntax
    async fn validate_syntax(&self, asset: &Asset) -> Result<SyntaxValidationResult> {
        if let Some(validator) = self.type_validators.get(&asset.asset_type) {
            validator.validate_syntax(asset).await
        } else {
            // No specific validator, return default result
            Ok(SyntaxValidationResult {
                valid: true,
                errors: Vec::new(),
                style_violations: Vec::new(),
                best_practices: Vec::new(),
                linting_issues: Vec::new(),
                total_issues: 0,
            })
        }
    }

    /// Validate asset security
    async fn validate_security(&self, asset: &Asset) -> Result<SecurityValidationResult> {
        let mut combined_result = SecurityValidationResult {
            score: 100,
            vulnerabilities: Vec::new(),
            malware: Vec::new(),
            injection_risks: Vec::new(),
            rule_failures: Vec::new(),
            recommendations: Vec::new(),
        };

        // Run all scanners
        for scanner in &self.security_scanners {
            let result = scanner.scan(asset).await?;

            // Combine results
            combined_result.vulnerabilities.extend(result.vulnerabilities);
            combined_result.malware.extend(result.malware);
            combined_result.injection_risks.extend(result.injection_risks);
            combined_result.rule_failures.extend(result.rule_failures);
            combined_result.recommendations.extend(result.recommendations);

            // Take minimum score
            combined_result.score = combined_result.score.min(result.score);
        }

        Ok(combined_result)
    }

    /// Validate asset performance
    async fn validate_performance(&self, _asset: &Asset) -> Result<PerformanceValidationResult> {
        // Simplified performance validation
        Ok(PerformanceValidationResult {
            resource_usage: ResourceUsage {
                cpu_usage: 10.0,
                memory_usage: 100 * 1024 * 1024, // 100MB
                disk_io: 1024 * 1024,             // 1MB/s
                network_io: 512 * 1024,           // 512KB/s
            },
            complexity: ComplexityAnalysis {
                cyclomatic_complexity: 10,
                cognitive_complexity: 8,
                lines_of_code: 500,
                halstead: HalsteadMetrics {
                    vocabulary: 100,
                    length: 500,
                    calculated_length: 480.0,
                    volume: 3321.0,
                    difficulty: 25.0,
                    effort: 83025.0,
                    time: 4612.5,
                    bugs: 1.1,
                },
                maintainability_index: 65.0,
            },
            issues: Vec::new(),
            benchmarks: None,
            score: 80,
        })
    }

    /// Validate asset compliance
    async fn validate_compliance(&self, _asset: &Asset) -> Result<ComplianceValidationResult> {
        // Simplified compliance validation
        Ok(ComplianceValidationResult {
            compliant: true,
            license: None,
            export_control: None,
            privacy: None,
            standards: Vec::new(),
            score: 100,
            issues: Vec::new(),
        })
    }

    /// Calculate validation summary
    fn calculate_summary(
        &self,
        security: &Option<SecurityValidationResult>,
        syntax: &Option<SyntaxValidationResult>,
        performance: &Option<PerformanceValidationResult>,
        compliance: &Option<ComplianceValidationResult>,
        duration_ms: u64,
    ) -> ValidationSummary {
        let mut total_issues = 0;
        let mut critical_issues = 0;
        let mut high_issues = 0;
        let mut medium_issues = 0;
        let mut low_issues = 0;
        let mut info_issues = 0;
        let mut recommendations = Vec::new();

        // Count security issues
        if let Some(sec) = security {
            for vuln in &sec.vulnerabilities {
                total_issues += 1;
                match vuln.severity {
                    SecuritySeverity::Critical => critical_issues += 1,
                    SecuritySeverity::High => high_issues += 1,
                    SecuritySeverity::Medium => medium_issues += 1,
                    SecuritySeverity::Low => low_issues += 1,
                    SecuritySeverity::Info => info_issues += 1,
                }
            }
            recommendations.extend(sec.recommendations.clone());
        }

        // Count syntax issues
        if let Some(syn) = syntax {
            total_issues += syn.total_issues;
            medium_issues += syn.errors.len();
            low_issues += syn.style_violations.len();
            info_issues += syn.linting_issues.len();
        }

        // Determine risk level
        let risk_level = if critical_issues > 0 {
            RiskLevel::Critical
        } else if high_issues > 0 {
            RiskLevel::High
        } else if medium_issues > 0 {
            RiskLevel::Medium
        } else if low_issues > 0 {
            RiskLevel::Low
        } else {
            RiskLevel::Minimal
        };

        ValidationSummary {
            total_issues,
            critical_issues,
            high_issues,
            medium_issues,
            low_issues,
            info_issues,
            risk_level,
            recommendations,
            duration_ms,
        }
    }

    /// Check if validation passed
    fn check_passed(
        &self,
        security: &Option<SecurityValidationResult>,
        syntax: &Option<SyntaxValidationResult>,
        _performance: &Option<PerformanceValidationResult>,
        compliance: &Option<ComplianceValidationResult>,
    ) -> bool {
        // Check security requirements
        if let Some(sec) = security {
            if sec.score < self.config.security.minimum_security_score {
                return false;
            }

            let critical_count = sec.vulnerabilities
                .iter()
                .filter(|v| v.severity == SecuritySeverity::Critical)
                .count() as u32;

            if critical_count > self.config.security.max_critical_vulnerabilities {
                return false;
            }
        }

        // Check syntax requirements
        if let Some(syn) = syntax {
            if !syn.valid {
                return false;
            }
        }

        // Check compliance requirements
        if let Some(comp) = compliance {
            if !comp.compliant {
                return false;
            }
        }

        true
    }

    /// Validate dependencies
    pub async fn validate_dependencies(&self, asset: &Asset) -> Result<Vec<VersionConflict>> {
        let graph = self.dependency_resolver.resolve(asset).await?;

        // Check dependency depth
        if graph.get_depth() > self.config.dependency.max_dependency_depth as usize {
            return Err(anyhow::anyhow!(
                "Dependency depth exceeds maximum allowed: {} > {}",
                graph.get_depth(),
                self.config.dependency.max_dependency_depth
            ));
        }

        // Check for conflicts
        Ok(self.dependency_resolver.check_conflicts(&graph))
    }
}