// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Quality Gates for TrustChain Deployment
//!
//! Validates security implementations and prevents deployment of security theater.
//! Based on the assessment findings, this enforces proper security practices.

mod gates;

use std::collections::HashMap;
use std::path::Path;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tracing::{info, error};

use gates::{
    SecurityTheaterGate, ConsensusValidationGate, HSMDependencyGate,
    MockResponseGate, ProductionReadinessGate, DNSInfrastructureGate,
};

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
    pub gates: Vec<Box<dyn QualityGate>>,
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
            gates,
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

                    if gate.is_blocking() && result.status == QualityGateStatus::Fail {
                        blocking_failures += 1;
                        error!("BLOCKING failure in gate: {} - {}", gate.name(), result.message);
                    }

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
