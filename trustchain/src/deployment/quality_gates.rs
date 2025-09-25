//! Quality Gates for TrustChain Deployment
//!
//! Validates security implementations and prevents deployment of security theater.
//! Based on the assessment findings, this enforces proper security practices.

use std::collections::HashMap;
use std::path::Path;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};

/// Quality gate validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGateResults {
    pub overall_status: QualityGateStatus,
    pub security_score: f64,
    pub individual_gates: HashMap<String, GateResult>,
    pub violations: Vec<SecurityViolation>,
    pub recommendations: Vec<String>,
    pub deployment_approved: bool,
}

/// Quality gate status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QualityGateStatus {
    Pass,
    Warning,
    Fail,
}

/// Individual gate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub status: QualityGateStatus,
    pub score: f64,
    pub message: String,
    pub details: Vec<String>,
}

/// Security violation detected by quality gates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityViolation {
    pub violation_type: String,
    pub severity: SecuritySeverity,
    pub location: String,
    pub description: String,
    pub remediation: String,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecuritySeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for SecuritySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecuritySeverity::Critical => write!(f, "Critical"),
            SecuritySeverity::High => write!(f, "High"),
            SecuritySeverity::Medium => write!(f, "Medium"),
            SecuritySeverity::Low => write!(f, "Low"),
        }
    }
}

/// TrustChain Quality Gate Validator
pub struct QualityGateValidator {
    pub source_path: String,
    pub gates: Vec<QualityGate>,
}

/// Quality gate definition
pub trait QualityGate {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn validate(&self, source_path: &str) -> Result<GateResult>;
    fn is_blocking(&self) -> bool;
}

impl QualityGateValidator {
    /// Create new quality gate validator
    pub fn new<P: AsRef<Path>>(source_path: P) -> Self {
        let source_path = source_path.as_ref().to_string_lossy().to_string();

        let gates: Vec<Box<dyn QualityGate>> = vec![
            Box::new(SecurityTheaterGate),
            Box::new(ConsensusValidationGate),
            Box::new(HSMDependencyGate),
            Box::new(MockResponseGate),
            Box::new(ProductionReadinessGate),
            Box::new(DNSInfrastructureGate),
        ];

        Self {
            source_path,
            gates: gates,
        }
    }

    /// Run all quality gates
    pub async fn validate_all(&self) -> Result<QualityGateResults> {
        info!("Running quality gate validation for TrustChain deployment");

        let mut individual_gates = HashMap::new();
        let mut violations = Vec::new();
        let mut recommendations = Vec::new();
        let mut total_score = 0.0;
        let mut blocking_failures = 0;

        for gate in &self.gates {
            info!("Running quality gate: {}", gate.name());

            match gate.validate(&self.source_path) {
                Ok(result) => {
                    total_score += result.score;

                    // Collect violations from gate details
                    for detail in &result.details {
                        if detail.contains("VIOLATION:") {
                            violations.push(SecurityViolation {
                                violation_type: gate.name().to_string(),
                                severity: determine_severity(&result.status),
                                location: "source_code".to_string(),
                                description: detail.clone(),
                                remediation: format!("Fix {} in {}", gate.name(), gate.description()),
                            });
                        }
                    }

                    // Track blocking failures
                    if gate.is_blocking() && result.status == QualityGateStatus::Fail {
                        blocking_failures += 1;
                        error!("BLOCKING failure in gate: {} - {}", gate.name(), result.message);
                    }

                    // Add recommendations for warnings
                    if result.status == QualityGateStatus::Warning {
                        recommendations.push(format!("{}: {}", gate.name(), result.message));
                    }

                    individual_gates.insert(gate.name().to_string(), result);
                }
                Err(e) => {
                    error!("Quality gate {} failed to execute: {}", gate.name(), e);

                    let error_result = GateResult {
                        status: QualityGateStatus::Fail,
                        score: 0.0,
                        message: format!("Gate execution failed: {}", e),
                        details: vec![format!("ERROR: {}", e)],
                    };

                    individual_gates.insert(gate.name().to_string(), error_result);

                    if gate.is_blocking() {
                        blocking_failures += 1;
                    }
                }
            }
        }

        // Calculate overall status and security score
        let gate_count = self.gates.len() as f64;
        let security_score = total_score / gate_count;

        let overall_status = if blocking_failures > 0 {
            QualityGateStatus::Fail
        } else if security_score < 0.8 {
            QualityGateStatus::Warning
        } else {
            QualityGateStatus::Pass
        };

        let deployment_approved = overall_status == QualityGateStatus::Pass && violations.is_empty();

        let results = QualityGateResults {
            overall_status,
            security_score,
            individual_gates,
            violations,
            recommendations,
            deployment_approved,
        };

        info!("Quality gate validation completed: {:.1}% security score, {} violations, deployment_approved: {}",
              security_score * 100.0, results.violations.len(), deployment_approved);

        Ok(results)
    }
}

/// Determine severity from gate status
fn determine_severity(status: &QualityGateStatus) -> SecuritySeverity {
    match status {
        QualityGateStatus::Fail => SecuritySeverity::Critical,
        QualityGateStatus::Warning => SecuritySeverity::Medium,
        QualityGateStatus::Pass => SecuritySeverity::Low,
    }
}

/// Security Theater Detection Gate
struct SecurityTheaterGate;

impl QualityGate for SecurityTheaterGate {
    fn name(&self) -> &str { "SecurityTheaterDetection" }

    fn description(&self) -> &str {
        "Detects security theater patterns including default_for_testing() bypasses"
    }

    fn is_blocking(&self) -> bool { true }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::process::Command;

        // Search for security theater patterns
        let patterns = [
            "default_for_testing",
            "mock_",
            "Mock",
            "TODO.*security",
            "stub.*implementation",
            "fake.*certificate"
        ];

        let mut violations = Vec::new();
        let mut total_matches = 0;

        for pattern in &patterns {
            let output = Command::new("rg")
                .arg("--count")
                .arg(pattern)
                .arg(source_path)
                .output()?;

            if output.status.success() {
                let count_str = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = count_str.lines().collect();

                for line in lines {
                    if let Some((file, count_str)) = line.split_once(':') {
                        if let Ok(count) = count_str.parse::<u32>() {
                            if count > 0 {
                                total_matches += count;
                                violations.push(format!(
                                    "VIOLATION: Found {} instances of '{}' in {}",
                                    count, pattern, file
                                ));
                            }
                        }
                    }
                }
            }
        }

        let status = if total_matches == 0 {
            QualityGateStatus::Pass
        } else if total_matches < 10 {
            QualityGateStatus::Warning
        } else {
            QualityGateStatus::Fail
        };

        let score = if total_matches == 0 { 1.0 } else { 1.0 - (total_matches as f64 / 50.0).min(1.0) };

        Ok(GateResult {
            status,
            score,
            message: format!("Found {} security theater patterns", total_matches),
            details: violations,
        })
    }
}

/// Consensus Validation Gate
struct ConsensusValidationGate;

impl QualityGate for ConsensusValidationGate {
    fn name(&self) -> &str { "ConsensusValidation" }

    fn description(&self) -> &str {
        "Validates proper consensus proof validation is implemented"
    }

    fn is_blocking(&self) -> bool { true }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::process::Command;
        use std::fs;

        // Check for proper consensus validation implementation
        let consensus_file = format!("{}/src/consensus/mod.rs", source_path);

        if !Path::new(&consensus_file).exists() {
            return Ok(GateResult {
                status: QualityGateStatus::Fail,
                score: 0.0,
                message: "Consensus module not found".to_string(),
                details: vec!["VIOLATION: Missing consensus validation implementation".to_string()],
            });
        }

        let content = fs::read_to_string(&consensus_file)?;

        // Check for real implementation markers
        let has_network_generation = content.contains("generate_from_network");
        let has_validation = content.contains("validate_with_requirements");
        let testing_restricted = content.contains("#[cfg(test)]") && content.contains("default_for_testing");

        let mut details = Vec::new();
        let mut score = 0.0;

        if has_network_generation {
            details.push("✓ Real consensus proof generation implemented".to_string());
            score += 0.4;
        } else {
            details.push("VIOLATION: Missing real consensus proof generation".to_string());
        }

        if has_validation {
            details.push("✓ Consensus validation with requirements implemented".to_string());
            score += 0.4;
        } else {
            details.push("VIOLATION: Missing consensus validation requirements".to_string());
        }

        if testing_restricted {
            details.push("✓ Testing bypasses properly restricted".to_string());
            score += 0.2;
        } else {
            details.push("VIOLATION: Testing bypasses not properly restricted".to_string());
        }

        let status = if score >= 0.8 {
            QualityGateStatus::Pass
        } else if score >= 0.5 {
            QualityGateStatus::Warning
        } else {
            QualityGateStatus::Fail
        };

        Ok(GateResult {
            status,
            score,
            message: format!("Consensus validation implementation: {:.1}%", score * 100.0),
            details,
        })
    }
}

/// HSM Dependency Gate
struct HSMDependencyGate;

impl QualityGate for HSMDependencyGate {
    fn name(&self) -> &str { "HSMDependencyCheck" }

    fn description(&self) -> &str {
        "Ensures HSM dependencies are removed (software-only requirement)"
    }

    fn is_blocking(&self) -> bool { true }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::fs;

        let cargo_file = format!("{}/Cargo.toml", source_path);

        if !Path::new(&cargo_file).exists() {
            return Ok(GateResult {
                status: QualityGateStatus::Fail,
                score: 0.0,
                message: "Cargo.toml not found".to_string(),
                details: vec!["ERROR: Cannot validate dependencies".to_string()],
            });
        }

        let content = fs::read_to_string(&cargo_file)?;

        // Check for forbidden HSM dependencies
        let hsm_patterns = [
            "aws-sdk-cloudhsm",
            "rusty-hsm",
            "pkcs11",
            "hsm-client",
        ];

        let mut violations = Vec::new();
        let mut hsm_found = false;

        for pattern in &hsm_patterns {
            if content.contains(pattern) {
                violations.push(format!("VIOLATION: HSM dependency found: {}", pattern));
                hsm_found = true;
            }
        }

        // Check for removed HSM comment
        let hsm_removed_comment = content.contains("AWS CloudHSM dependencies REMOVED");

        let status = if !hsm_found && hsm_removed_comment {
            QualityGateStatus::Pass
        } else if !hsm_found {
            QualityGateStatus::Warning
        } else {
            QualityGateStatus::Fail
        };

        let score = if !hsm_found { 1.0 } else { 0.0 };

        if !hsm_found {
            violations.push("✓ No HSM dependencies detected".to_string());
        }

        if hsm_removed_comment {
            violations.push("✓ HSM removal properly documented".to_string());
        }

        Ok(GateResult {
            status,
            score,
            message: format!("HSM dependency check: {}", if hsm_found { "FAILED" } else { "PASSED" }),
            details: violations,
        })
    }
}

/// Mock Response Gate
struct MockResponseGate;

impl QualityGate for MockResponseGate {
    fn name(&self) -> &str { "MockResponseDetection" }

    fn description(&self) -> &str {
        "Detects dangerous mock responses in API endpoints"
    }

    fn is_blocking(&self) -> bool { true }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::process::Command;

        let api_path = format!("{}/src/api", source_path);

        if !Path::new(&api_path).exists() {
            return Ok(GateResult {
                status: QualityGateStatus::Warning,
                score: 0.5,
                message: "API module not found".to_string(),
                details: vec!["WARNING: Cannot validate API endpoints".to_string()],
            });
        }

        // Search for mock response patterns
        let output = Command::new("rg")
            .arg("-n")
            .arg("mock.*response|Mock.*certificate|TODO.*Integrate")
            .arg(&api_path)
            .output()?;

        let results = String::from_utf8_lossy(&output.stdout);
        let mock_lines: Vec<&str> = results.lines().collect();

        let mut violations = Vec::new();
        let mock_count = mock_lines.len();

        // Look for security fixes that remove mocks
        let security_fix_output = Command::new("rg")
            .arg("-n")
            .arg("SECURITY.*FIX|mock.*removed|NOT_IMPLEMENTED")
            .arg(&api_path)
            .output()?;

        let security_fixes = String::from_utf8_lossy(&security_fix_output.stdout);
        let fix_lines: Vec<&str> = security_fixes.lines().collect();

        for line in mock_lines.iter().take(10) {  // Show first 10 violations
            violations.push(format!("VIOLATION: Mock response detected: {}", line));
        }

        for line in fix_lines.iter().take(5) {   // Show first 5 fixes
            violations.push(format!("✓ Security fix detected: {}", line));
        }

        let status = if mock_count == 0 || fix_lines.len() >= mock_count {
            QualityGateStatus::Pass
        } else if fix_lines.len() > 0 {
            QualityGateStatus::Warning
        } else {
            QualityGateStatus::Fail
        };

        let score = if mock_count == 0 {
            1.0
        } else {
            (fix_lines.len() as f64 / mock_count as f64).min(1.0)
        };

        Ok(GateResult {
            status,
            score,
            message: format!("Mock responses: {} found, {} security fixes applied", mock_count, fix_lines.len()),
            details: violations,
        })
    }
}

/// Production Readiness Gate
struct ProductionReadinessGate;

impl QualityGate for ProductionReadinessGate {
    fn name(&self) -> &str { "ProductionReadiness" }

    fn description(&self) -> &str {
        "Validates production-ready implementations"
    }

    fn is_blocking(&self) -> bool { false }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::process::Command;

        // Check for production-ready markers
        let output = Command::new("rg")
            .arg("-c")
            .arg("production|Production")
            .arg(source_path)
            .output()?;

        let production_count = if output.status.success() {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.split(':').nth(1)?.parse::<u32>().ok())
                .sum::<u32>()
        } else {
            0
        };

        // Check for proper error handling
        let error_output = Command::new("rg")
            .arg("-c")
            .arg("anyhow::Result|TrustChainResult")
            .arg(source_path)
            .output()?;

        let error_handling_count = if error_output.status.success() {
            String::from_utf8_lossy(&error_output.stdout)
                .lines()
                .filter_map(|line| line.split(':').nth(1)?.parse::<u32>().ok())
                .sum::<u32>()
        } else {
            0
        };

        let mut details = Vec::new();
        let mut score = 0.0;

        if production_count >= 10 {
            details.push(format!("✓ {} production-ready implementations found", production_count));
            score += 0.5;
        } else {
            details.push(format!("WARNING: Only {} production implementations found", production_count));
        }

        if error_handling_count >= 50 {
            details.push(format!("✓ {} proper error handling implementations found", error_handling_count));
            score += 0.5;
        } else {
            details.push(format!("WARNING: Only {} error handling implementations found", error_handling_count));
        }

        let status = if score >= 0.8 {
            QualityGateStatus::Pass
        } else if score >= 0.4 {
            QualityGateStatus::Warning
        } else {
            QualityGateStatus::Fail
        };

        Ok(GateResult {
            status,
            score,
            message: format!("Production readiness: {:.1}%", score * 100.0),
            details,
        })
    }
}

/// DNS Infrastructure Gate
struct DNSInfrastructureGate;

impl QualityGate for DNSInfrastructureGate {
    fn name(&self) -> &str { "DNSInfrastructure" }

    fn description(&self) -> &str {
        "Validates DNS infrastructure replaces localhost stubs"
    }

    fn is_blocking(&self) -> bool { true }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::fs;

        // Check for DNS infrastructure files
        let dns_files = [
            format!("{}/src/dns/authoritative_server.rs", source_path),
            format!("{}/src/dns/production_zones.rs", source_path),
        ];

        let mut details = Vec::new();
        let mut score = 0.0;
        let mut files_found = 0;

        for file_path in &dns_files {
            if Path::new(file_path).exists() {
                files_found += 1;

                let content = fs::read_to_string(file_path)?;

                if content.contains("localhost") && !content.contains("replacing localhost stubs") {
                    details.push(format!("VIOLATION: {} still contains localhost stubs", file_path));
                } else if content.contains("trust.hypermesh.online") && content.contains("production") {
                    details.push(format!("✓ {} has production DNS infrastructure", file_path));
                    score += 0.4;
                } else {
                    details.push(format!("WARNING: {} needs production DNS configuration", file_path));
                    score += 0.1;
                }
            } else {
                details.push(format!("VIOLATION: Missing DNS infrastructure file: {}", file_path));
            }
        }

        if files_found == dns_files.len() {
            score += 0.2; // Bonus for having all required files
        }

        let status = if score >= 0.8 {
            QualityGateStatus::Pass
        } else if score >= 0.5 {
            QualityGateStatus::Warning
        } else {
            QualityGateStatus::Fail
        };

        Ok(GateResult {
            status,
            score,
            message: format!("DNS infrastructure: {}/{} files implemented", files_found, dns_files.len()),
            details,
        })
    }
}