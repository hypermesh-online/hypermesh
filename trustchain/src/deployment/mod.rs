//! Deployment Management and Quality Gates
//!
//! Ensures secure deployment practices and validates security implementations.
//! Prevents deployment of systems with security theater.

use std::path::Path;
use anyhow::Result;
use tracing::{info, error};

pub mod quality_gates;

pub use quality_gates::{
    QualityGateValidator, QualityGateResults, QualityGateStatus,
    SecurityViolation, SecuritySeverity
};

/// TrustChain deployment validator
pub struct DeploymentValidator {
    validator: QualityGateValidator,
}

impl DeploymentValidator {
    /// Create new deployment validator
    pub fn new<P: AsRef<Path>>(source_path: P) -> Self {
        Self {
            validator: QualityGateValidator::new(source_path),
        }
    }

    /// Validate deployment readiness
    pub async fn validate_deployment(&self) -> Result<DeploymentValidationResult> {
        info!("Starting TrustChain deployment validation");

        let quality_results = self.validator.validate_all().await?;

        let deployment_decision = DeploymentDecision::from_quality_results(&quality_results);

        let result = DeploymentValidationResult {
            quality_gates: quality_results.clone(),
            deployment_decision,
            summary: DeploymentSummary::from_results(&quality_results),
        };

        match &result.deployment_decision {
            DeploymentDecision::Approved => {
                info!("✅ DEPLOYMENT APPROVED: All quality gates passed");
            }
            DeploymentDecision::ConditionalApproval { conditions } => {
                info!("⚠️  CONDITIONAL APPROVAL: {} conditions must be resolved", conditions.len());
            }
            DeploymentDecision::Rejected { reason } => {
                error!("❌ DEPLOYMENT REJECTED: {}", reason);
            }
        }

        Ok(result)
    }
}

/// Deployment validation result
#[derive(Debug, Clone)]
pub struct DeploymentValidationResult {
    pub quality_gates: QualityGateResults,
    pub deployment_decision: DeploymentDecision,
    pub summary: DeploymentSummary,
}

/// Deployment decision
#[derive(Debug, Clone)]
pub enum DeploymentDecision {
    Approved,
    ConditionalApproval { conditions: Vec<String> },
    Rejected { reason: String },
}

impl DeploymentDecision {
    /// Create deployment decision from quality gate results
    fn from_quality_results(results: &QualityGateResults) -> Self {
        if !results.deployment_approved {
            if results.overall_status == QualityGateStatus::Fail {
                return Self::Rejected {
                    reason: format!(
                        "Critical security violations detected: {} violations, {:.1}% security score",
                        results.violations.len(),
                        results.security_score * 100.0
                    ),
                };
            }
        }

        if results.violations.is_empty() && results.security_score >= 0.9 {
            Self::Approved
        } else if results.security_score >= 0.7 {
            let mut conditions = Vec::new();

            if results.security_score < 0.9 {
                conditions.push(format!("Improve security score to 90% (currently {:.1}%)", results.security_score * 100.0));
            }

            for violation in &results.violations {
                if violation.severity == SecuritySeverity::Critical || violation.severity == SecuritySeverity::High {
                    conditions.push(format!("Resolve {} violation: {}", violation.severity, violation.description));
                }
            }

            Self::ConditionalApproval { conditions }
        } else {
            Self::Rejected {
                reason: format!(
                    "Security score too low: {:.1}% (minimum 70% required)",
                    results.security_score * 100.0
                ),
            }
        }
    }
}

/// Deployment summary
#[derive(Debug, Clone)]
pub struct DeploymentSummary {
    pub security_score: f64,
    pub total_gates: usize,
    pub passed_gates: usize,
    pub failed_gates: usize,
    pub warning_gates: usize,
    pub critical_violations: usize,
    pub high_violations: usize,
    pub medium_violations: usize,
    pub low_violations: usize,
    pub key_improvements: Vec<String>,
}

impl DeploymentSummary {
    /// Create summary from quality gate results
    fn from_results(results: &QualityGateResults) -> Self {
        let total_gates = results.individual_gates.len();
        let mut passed_gates = 0;
        let mut failed_gates = 0;
        let mut warning_gates = 0;

        for (_, gate_result) in &results.individual_gates {
            match gate_result.status {
                QualityGateStatus::Pass => passed_gates += 1,
                QualityGateStatus::Fail => failed_gates += 1,
                QualityGateStatus::Warning => warning_gates += 1,
            }
        }

        let mut critical_violations = 0;
        let mut high_violations = 0;
        let mut medium_violations = 0;
        let mut low_violations = 0;

        for violation in &results.violations {
            match violation.severity {
                SecuritySeverity::Critical => critical_violations += 1,
                SecuritySeverity::High => high_violations += 1,
                SecuritySeverity::Medium => medium_violations += 1,
                SecuritySeverity::Low => low_violations += 1,
            }
        }

        let key_improvements = extract_key_improvements(results);

        Self {
            security_score: results.security_score,
            total_gates,
            passed_gates,
            failed_gates,
            warning_gates,
            critical_violations,
            high_violations,
            medium_violations,
            low_violations,
            key_improvements,
        }
    }
}

/// Extract key improvements from quality gate results
fn extract_key_improvements(results: &QualityGateResults) -> Vec<String> {
    let mut improvements = Vec::new();

    // Security theater improvements
    if let Some(theater_gate) = results.individual_gates.get("SecurityTheaterDetection") {
        if theater_gate.status != QualityGateStatus::Pass {
            improvements.push("🎭 Remove all security theater patterns (default_for_testing, mocks)".to_string());
        }
    }

    // Consensus validation improvements
    if let Some(consensus_gate) = results.individual_gates.get("ConsensusValidation") {
        if consensus_gate.status != QualityGateStatus::Pass {
            improvements.push("🔐 Implement proper consensus validation with network proofs".to_string());
        }
    }

    // HSM dependency improvements
    if let Some(hsm_gate) = results.individual_gates.get("HSMDependencyCheck") {
        if hsm_gate.status != QualityGateStatus::Pass {
            improvements.push("🔧 Remove HSM dependencies for software-only operation".to_string());
        }
    }

    // Mock response improvements
    if let Some(mock_gate) = results.individual_gates.get("MockResponseDetection") {
        if mock_gate.status != QualityGateStatus::Pass {
            improvements.push("📡 Replace mock API responses with real implementations".to_string());
        }
    }

    // DNS infrastructure improvements
    if let Some(dns_gate) = results.individual_gates.get("DNSInfrastructure") {
        if dns_gate.status != QualityGateStatus::Pass {
            improvements.push("🌐 Complete DNS infrastructure for trust.hypermesh.online".to_string());
        }
    }

    improvements
}

/// Command-line deployment validation tool
pub async fn validate_deployment_cli<P: AsRef<Path>>(source_path: P) -> Result<()> {
    println!("🚀 TrustChain Deployment Validation");
    println!("===================================");

    let validator = DeploymentValidator::new(source_path);
    let results = validator.validate_deployment().await?;

    // Print summary
    println!("\n📊 DEPLOYMENT SUMMARY");
    println!("Security Score: {:.1}%", results.summary.security_score * 100.0);
    println!("Quality Gates: {}/{} passed, {} warnings, {} failures",
             results.summary.passed_gates,
             results.summary.total_gates,
             results.summary.warning_gates,
             results.summary.failed_gates);

    println!("\nSecurity Violations:");
    println!("  🔴 Critical: {}", results.summary.critical_violations);
    println!("  🟠 High: {}", results.summary.high_violations);
    println!("  🟡 Medium: {}", results.summary.medium_violations);
    println!("  🟢 Low: {}", results.summary.low_violations);

    // Print deployment decision
    println!("\n🚀 DEPLOYMENT DECISION");
    match &results.deployment_decision {
        DeploymentDecision::Approved => {
            println!("✅ APPROVED - Ready for production deployment");
        }
        DeploymentDecision::ConditionalApproval { conditions } => {
            println!("⚠️  CONDITIONAL APPROVAL - Address the following:");
            for (i, condition) in conditions.iter().enumerate() {
                println!("  {}. {}", i + 1, condition);
            }
        }
        DeploymentDecision::Rejected { reason } => {
            println!("❌ REJECTED - {}", reason);
        }
    }

    // Print key improvements
    if !results.summary.key_improvements.is_empty() {
        println!("\n💡 KEY IMPROVEMENTS NEEDED:");
        for improvement in &results.summary.key_improvements {
            println!("  {}", improvement);
        }
    }

    // Print individual gate results
    println!("\n🔍 DETAILED GATE RESULTS:");
    for (gate_name, gate_result) in &results.quality_gates.individual_gates {
        let status_emoji = match gate_result.status {
            QualityGateStatus::Pass => "✅",
            QualityGateStatus::Warning => "⚠️",
            QualityGateStatus::Fail => "❌",
        };

        println!("\n{} {} ({:.1}%)", status_emoji, gate_name, gate_result.score * 100.0);
        println!("   {}", gate_result.message);

        // Show details for failures and warnings
        if gate_result.status != QualityGateStatus::Pass {
            for detail in gate_result.details.iter().take(3) {
                println!("     {}", detail);
            }
            if gate_result.details.len() > 3 {
                println!("     ... and {} more", gate_result.details.len() - 3);
            }
        }
    }

    println!("\n====================================");

    match &results.deployment_decision {
        DeploymentDecision::Approved => {
            println!("🎉 TrustChain is ready for production deployment!");
            std::process::exit(0);
        }
        DeploymentDecision::ConditionalApproval { .. } => {
            println!("⚡ TrustChain deployment approved with conditions");
            std::process::exit(1);
        }
        DeploymentDecision::Rejected { .. } => {
            println!("🛑 TrustChain deployment rejected - security fixes required");
            std::process::exit(2);
        }
    }
}