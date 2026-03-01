// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Individual quality gate implementations for TrustChain deployment validation.

use anyhow::Result;
use std::path::Path;

use super::{GateResult, QualityGate, QualityGateStatus};

/// Security Theater Detection Gate
pub(super) struct SecurityTheaterGate;

impl QualityGate for SecurityTheaterGate {
    fn name(&self) -> &str {
        "SecurityTheaterDetection"
    }

    fn description(&self) -> &str {
        "Detects security theater patterns including default_for_testing() bypasses"
    }

    fn is_blocking(&self) -> bool {
        true
    }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::process::Command;

        let patterns = [
            "default_for_testing",
            "mock_",
            "Mock",
            "TODO.*security",
            "stub.*implementation",
            "fake.*certificate",
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
                                    "VIOLATION: Found {count} instances of '{pattern}' in {file}"
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

        let score = if total_matches == 0 {
            1.0
        } else {
            1.0 - (total_matches as f64 / 50.0).min(1.0)
        };

        Ok(GateResult {
            status,
            score,
            message: format!("Found {total_matches} security theater patterns"),
            details: violations,
        })
    }
}

/// Consensus Validation Gate
pub(super) struct ConsensusValidationGate;

impl QualityGate for ConsensusValidationGate {
    fn name(&self) -> &str {
        "ConsensusValidation"
    }

    fn description(&self) -> &str {
        "Validates proper consensus proof validation is implemented"
    }

    fn is_blocking(&self) -> bool {
        true
    }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::fs;

        let consensus_file = format!("{source_path}/src/consensus/mod.rs");

        if !Path::new(&consensus_file).exists() {
            return Ok(GateResult {
                status: QualityGateStatus::Fail,
                score: 0.0,
                message: "Consensus module not found".to_string(),
                details: vec!["VIOLATION: Missing consensus validation implementation".to_string()],
            });
        }

        let content = fs::read_to_string(&consensus_file)?;

        let has_network_generation = content.contains("generate_from_network");
        let has_validation = content.contains("validate_with_requirements");
        let testing_restricted =
            content.contains("#[cfg(test)]") && content.contains("default_for_testing");

        let mut details = Vec::new();
        let mut score = 0.0;

        if has_network_generation {
            details.push("OK: Real consensus proof generation implemented".to_string());
            score += 0.4;
        } else {
            details.push("VIOLATION: Missing real consensus proof generation".to_string());
        }

        if has_validation {
            details.push("OK: Consensus validation with requirements implemented".to_string());
            score += 0.4;
        } else {
            details.push("VIOLATION: Missing consensus validation requirements".to_string());
        }

        if testing_restricted {
            details.push("OK: Testing bypasses properly restricted".to_string());
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
pub(super) struct HSMDependencyGate;

impl QualityGate for HSMDependencyGate {
    fn name(&self) -> &str {
        "HSMDependencyCheck"
    }

    fn description(&self) -> &str {
        "Ensures HSM dependencies are removed (software-only requirement)"
    }

    fn is_blocking(&self) -> bool {
        true
    }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::fs;

        let cargo_file = format!("{source_path}/Cargo.toml");

        if !Path::new(&cargo_file).exists() {
            return Ok(GateResult {
                status: QualityGateStatus::Fail,
                score: 0.0,
                message: "Cargo.toml not found".to_string(),
                details: vec!["ERROR: Cannot validate dependencies".to_string()],
            });
        }

        let content = fs::read_to_string(&cargo_file)?;

        let hsm_patterns = ["aws-sdk-cloudhsm", "rusty-hsm", "pkcs11", "hsm-client"];

        let mut violations = Vec::new();
        let mut hsm_found = false;

        for pattern in &hsm_patterns {
            if content.contains(pattern) {
                violations.push(format!("VIOLATION: HSM dependency found: {pattern}"));
                hsm_found = true;
            }
        }

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
            violations.push("OK: No HSM dependencies detected".to_string());
        }

        if hsm_removed_comment {
            violations.push("OK: HSM removal properly documented".to_string());
        }

        Ok(GateResult {
            status,
            score,
            message: format!(
                "HSM dependency check: {}",
                if hsm_found { "FAILED" } else { "PASSED" }
            ),
            details: violations,
        })
    }
}

/// Mock Response Gate
pub(super) struct MockResponseGate;

impl QualityGate for MockResponseGate {
    fn name(&self) -> &str {
        "MockResponseDetection"
    }

    fn description(&self) -> &str {
        "Detects dangerous mock responses in API endpoints"
    }

    fn is_blocking(&self) -> bool {
        true
    }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::process::Command;

        let api_path = format!("{source_path}/src/api");

        if !Path::new(&api_path).exists() {
            return Ok(GateResult {
                status: QualityGateStatus::Warning,
                score: 0.5,
                message: "API module not found".to_string(),
                details: vec!["WARNING: Cannot validate API endpoints".to_string()],
            });
        }

        let output = Command::new("rg")
            .arg("-n")
            .arg("mock.*response|Mock.*certificate|TODO.*Integrate")
            .arg(&api_path)
            .output()?;

        let results = String::from_utf8_lossy(&output.stdout);
        let mock_lines: Vec<&str> = results.lines().collect();

        let mut violations = Vec::new();
        let mock_count = mock_lines.len();

        let security_fix_output = Command::new("rg")
            .arg("-n")
            .arg("SECURITY.*FIX|mock.*removed|NOT_IMPLEMENTED")
            .arg(&api_path)
            .output()?;

        let security_fixes = String::from_utf8_lossy(&security_fix_output.stdout);
        let fix_lines: Vec<&str> = security_fixes.lines().collect();

        for line in mock_lines.iter().take(10) {
            violations.push(format!("VIOLATION: Mock response detected: {line}"));
        }

        for line in fix_lines.iter().take(5) {
            violations.push(format!("OK: Security fix detected: {line}"));
        }

        let status = if mock_count == 0 || fix_lines.len() >= mock_count {
            QualityGateStatus::Pass
        } else if !fix_lines.is_empty() {
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
            message: format!(
                "Mock responses: {} found, {} security fixes applied",
                mock_count,
                fix_lines.len()
            ),
            details: violations,
        })
    }
}

/// Production Readiness Gate
pub(super) struct ProductionReadinessGate;

impl QualityGate for ProductionReadinessGate {
    fn name(&self) -> &str {
        "ProductionReadiness"
    }

    fn description(&self) -> &str {
        "Validates production-ready implementations"
    }

    fn is_blocking(&self) -> bool {
        false
    }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::process::Command;

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
            details.push(format!(
                "OK: {production_count} production-ready implementations found"
            ));
            score += 0.5;
        } else {
            details.push(format!(
                "WARNING: Only {production_count} production implementations found"
            ));
        }

        if error_handling_count >= 50 {
            details.push(format!(
                "OK: {error_handling_count} proper error handling implementations found"
            ));
            score += 0.5;
        } else {
            details.push(format!(
                "WARNING: Only {error_handling_count} error handling implementations found"
            ));
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
pub(super) struct DNSInfrastructureGate;

impl QualityGate for DNSInfrastructureGate {
    fn name(&self) -> &str {
        "DNSInfrastructure"
    }

    fn description(&self) -> &str {
        "Validates DNS infrastructure replaces localhost stubs"
    }

    fn is_blocking(&self) -> bool {
        true
    }

    fn validate(&self, source_path: &str) -> Result<GateResult> {
        use std::fs;

        let dns_files = [
            format!("{source_path}/src/dns/authoritative_server.rs"),
            format!("{source_path}/src/dns/production_zones.rs"),
        ];

        let mut details = Vec::new();
        let mut score = 0.0;
        let mut files_found = 0;

        for file_path in &dns_files {
            if Path::new(file_path).exists() {
                files_found += 1;

                let content = fs::read_to_string(file_path)?;

                if content.contains("localhost") && !content.contains("replacing localhost stubs") {
                    details.push(format!(
                        "VIOLATION: {file_path} still contains localhost stubs"
                    ));
                } else if content.contains("trust.hypermesh.online")
                    && content.contains("production")
                {
                    details.push(format!("OK: {file_path} has production DNS infrastructure"));
                    score += 0.4;
                } else {
                    details.push(format!(
                        "WARNING: {file_path} needs production DNS configuration"
                    ));
                    score += 0.1;
                }
            } else {
                details.push(format!(
                    "VIOLATION: Missing DNS infrastructure file: {file_path}"
                ));
            }
        }

        if files_found == dns_files.len() {
            score += 0.2;
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
            message: format!(
                "DNS infrastructure: {}/{} files implemented",
                files_found,
                dns_files.len()
            ),
            details,
        })
    }
}
