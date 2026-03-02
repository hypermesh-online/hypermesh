// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Security Scanners
//!
//! Security scanning implementations for asset validation.
//! Migrated to use Asset Registry architecture with BlockMatrix Assets.

use anyhow::Result;
use async_trait::async_trait;

// Use local Catalog AssetPackage
use super::config::SecuritySeverity;
use super::results::{
    CodeLocation, InjectionRisk, InjectionType, MalwareDetection, RiskLevel, SecurityRuleFailure,
    SecurityValidationResult, Vulnerability,
};
use super::traits::SecurityScanner;
use crate::assets::AssetPackage;

/// Short BLAKE3-based identifier from a string (first 8 hex chars).
fn blake3_short(input: &str) -> String {
    let hash = blake3::hash(input.as_bytes());
    hash.to_hex()[..8].to_string()
}

/// Static security scanner
pub struct StaticSecurityScanner;

impl Default for StaticSecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl StaticSecurityScanner {
    /// Create new static security scanner
    pub fn new() -> Self {
        Self
    }

    /// Scan for SQL injection risks
    fn scan_sql_injection(&self, code: &str) -> Vec<InjectionRisk> {
        let mut risks = Vec::new();

        let sql_patterns = [
            "SELECT.*FROM",
            "INSERT.*INTO",
            "UPDATE.*SET",
            "DELETE.*FROM",
            "DROP.*TABLE",
            "CREATE.*TABLE",
        ];

        for pattern in &sql_patterns {
            if code.contains(pattern) && code.contains("$") {
                risks.push(InjectionRisk {
                    injection_type: InjectionType::Sql,
                    description: "Potential SQL injection vulnerability detected".to_string(),
                    location: CodeLocation {
                        file: "unknown".to_string(),
                        line: None,
                        column: None,
                        snippet: Some(pattern.to_string()),
                    },
                    risk_level: RiskLevel::High,
                    mitigation: "Use parameterized queries or prepared statements".to_string(),
                });
            }
        }

        risks
    }

    /// Scan for command injection risks
    fn scan_command_injection(&self, code: &str) -> Vec<InjectionRisk> {
        let mut risks = Vec::new();

        let cmd_patterns = [
            "system(",
            "exec(",
            "shell_exec(",
            "eval(",
            "popen(",
            "proc_open(",
            "passthru(",
            "`",
        ];

        for pattern in &cmd_patterns {
            if code.contains(pattern) {
                risks.push(InjectionRisk {
                    injection_type: InjectionType::Command,
                    description: "Potential command injection vulnerability detected".to_string(),
                    location: CodeLocation {
                        file: "unknown".to_string(),
                        line: None,
                        column: None,
                        snippet: Some(pattern.to_string()),
                    },
                    risk_level: RiskLevel::Critical,
                    mitigation: "Avoid executing system commands with user input".to_string(),
                });
            }
        }

        risks
    }

    /// Scan for path traversal risks
    fn scan_path_traversal(&self, code: &str) -> Vec<InjectionRisk> {
        let mut risks = Vec::new();

        if code.contains("../") || code.contains("..\\") {
            risks.push(InjectionRisk {
                injection_type: InjectionType::PathTraversal,
                description: "Potential path traversal vulnerability detected".to_string(),
                location: CodeLocation {
                    file: "unknown".to_string(),
                    line: None,
                    column: None,
                    snippet: None,
                },
                risk_level: RiskLevel::High,
                mitigation: "Validate and sanitize file paths".to_string(),
            });
        }

        risks
    }

    /// Check for known vulnerabilities by inspecting declared dependencies
    /// from the asset's specification.
    fn check_vulnerabilities(&self, asset: &AssetPackage) -> Vec<Vulnerability> {
        let mut vulnerabilities = Vec::new();

        // Scan declared dependencies from asset spec
        for dep in &asset.spec.spec.dependencies {
            let name = &dep.name;
            let version = &dep.version;

            // Check against known-vulnerable patterns.
            // In production this would query a real advisory database.
            // For now we flag common unsafe patterns.
            if name.contains("vulnerable") || name.contains("insecure") {
                vulnerabilities.push(Vulnerability {
                    cve: Some(format!("HM-VULN-{}", blake3_short(name))),
                    description: format!(
                        "Known vulnerability pattern in dependency {name} {version}"
                    ),
                    severity: SecuritySeverity::High,
                    component: name.clone(),
                    fix_available: false,
                    fix_version: None,
                });
            }
        }

        vulnerabilities
    }

    /// Validate package size is within acceptable limits (max 500MB default).
    fn validate_size(&self, asset: &AssetPackage) -> Option<Vulnerability> {
        const MAX_PACKAGE_SIZE: usize = 500 * 1024 * 1024; // 500 MB
        let total_size = asset.content.main_content.len()
            + asset
                .content
                .file_contents
                .values()
                .map(|c| c.len())
                .sum::<usize>()
            + asset
                .content
                .binary_contents
                .values()
                .map(|c| c.len())
                .sum::<usize>();

        if total_size > MAX_PACKAGE_SIZE {
            return Some(Vulnerability {
                cve: None,
                description: format!(
                    "Package size ({} bytes) exceeds maximum allowed ({MAX_PACKAGE_SIZE} bytes)",
                    total_size
                ),
                severity: SecuritySeverity::Medium,
                component: "package-size".to_string(),
                fix_available: false,
                fix_version: None,
            });
        }
        None
    }

    /// Verify the package_hash matches the BLAKE3 hash of the content.
    fn verify_content_hash(&self, asset: &AssetPackage) -> Option<Vulnerability> {
        if asset.package_hash.is_empty() {
            return Some(Vulnerability {
                cve: None,
                description: "Package hash is empty; content integrity cannot be verified"
                    .to_string(),
                severity: SecuritySeverity::High,
                component: "content-hash".to_string(),
                fix_available: false,
                fix_version: None,
            });
        }

        // Compute BLAKE3 hash of the main content
        let computed_hash = blake3::hash(asset.content.main_content.as_bytes());
        let computed_hex = computed_hash.to_hex().to_string();

        // Only flag if the hash is clearly wrong and looks like it was meant to be a hex hash
        if asset.package_hash.len() == 64 && asset.package_hash != computed_hex {
            return Some(Vulnerability {
                cve: None,
                description: format!(
                    "Content hash mismatch: declared={}, computed={}",
                    &asset.package_hash[..16],
                    &computed_hex[..16]
                ),
                severity: SecuritySeverity::Critical,
                component: "content-hash".to_string(),
                fix_available: false,
                fix_version: None,
            });
        }
        None
    }

    /// Check that required metadata fields are present.
    fn check_metadata_completeness(&self, asset: &AssetPackage) -> Vec<Vulnerability> {
        let mut issues = Vec::new();

        if asset.spec.metadata.name.is_empty() {
            issues.push(Vulnerability {
                cve: None,
                description: "Package name is empty".to_string(),
                severity: SecuritySeverity::Medium,
                component: "metadata".to_string(),
                fix_available: false,
                fix_version: None,
            });
        }

        if asset.spec.metadata.version.is_empty() {
            issues.push(Vulnerability {
                cve: None,
                description: "Package version is empty".to_string(),
                severity: SecuritySeverity::Medium,
                component: "metadata".to_string(),
                fix_available: false,
                fix_version: None,
            });
        }

        issues
    }

    /// Perform type-specific validation (WASM magic bytes, JSON schema, etc.)
    fn type_specific_validation(&self, asset: &AssetPackage) -> Vec<Vulnerability> {
        let mut issues = Vec::new();
        let asset_type = asset.spec.spec.asset_type.to_lowercase();
        let content = asset.content.main_content.as_bytes();

        match asset_type.as_str() {
            "wasm" | "webassembly" => {
                // WASM magic bytes: \0asm (0x00, 0x61, 0x73, 0x6D)
                if content.len() >= 4 && &content[..4] != b"\0asm" {
                    issues.push(Vulnerability {
                        cve: None,
                        description: "WASM content missing magic bytes (\\0asm)".to_string(),
                        severity: SecuritySeverity::High,
                        component: "type-validation".to_string(),
                        fix_available: false,
                        fix_version: None,
                    });
                }
            }
            "json" | "json-schema" => {
                if !content.is_empty() {
                    if let Err(e) =
                        serde_json::from_slice::<serde_json::Value>(content)
                    {
                        issues.push(Vulnerability {
                            cve: None,
                            description: format!("Invalid JSON content: {e}"),
                            severity: SecuritySeverity::Medium,
                            component: "type-validation".to_string(),
                            fix_available: false,
                            fix_version: None,
                        });
                    }
                }
            }
            _ => {
                // No specific validation for unknown types
            }
        }

        issues
    }

    /// Calculate security score
    fn calculate_score(&self, result: &SecurityValidationResult) -> u32 {
        let mut score = 100u32;

        // Deduct points for vulnerabilities
        for vuln in &result.vulnerabilities {
            match vuln.severity {
                SecuritySeverity::Critical => score = score.saturating_sub(30),
                SecuritySeverity::High => score = score.saturating_sub(20),
                SecuritySeverity::Medium => score = score.saturating_sub(10),
                SecuritySeverity::Low => score = score.saturating_sub(5),
                SecuritySeverity::Info => score = score.saturating_sub(1),
            }
        }

        // Deduct points for injection risks
        for risk in &result.injection_risks {
            match risk.risk_level {
                RiskLevel::Critical => score = score.saturating_sub(25),
                RiskLevel::High => score = score.saturating_sub(15),
                RiskLevel::Medium => score = score.saturating_sub(8),
                RiskLevel::Low => score = score.saturating_sub(3),
                RiskLevel::Minimal => score = score.saturating_sub(1),
            }
        }

        // Deduct points for malware
        for detection in &result.malware {
            if detection.confidence > 80 {
                score = score.saturating_sub(50);
            } else if detection.confidence > 50 {
                score = score.saturating_sub(30);
            } else {
                score = score.saturating_sub(10);
            }
        }

        score
    }
}

#[async_trait]
impl SecurityScanner for StaticSecurityScanner {
    fn name(&self) -> &str {
        "StaticSecurityScanner"
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "vulnerability-detection".to_string(),
            "injection-detection".to_string(),
            "malware-scanning".to_string(),
            "dependency-scanning".to_string(),
        ]
    }

    async fn scan(&self, asset: &AssetPackage) -> Result<SecurityValidationResult> {
        let mut vulnerabilities = Vec::new();
        let mut malware = Vec::new();
        let mut injection_risks = Vec::new();
        let mut rule_failures = Vec::new();
        let mut recommendations = Vec::new();

        // Size validation
        if let Some(size_issue) = self.validate_size(asset) {
            vulnerabilities.push(size_issue);
        }

        // Content hash verification (BLAKE3)
        if let Some(hash_issue) = self.verify_content_hash(asset) {
            vulnerabilities.push(hash_issue);
        }

        // Metadata completeness check
        vulnerabilities.extend(self.check_metadata_completeness(asset));

        // Type-specific validation (WASM magic bytes, JSON schema, etc.)
        vulnerabilities.extend(self.type_specific_validation(asset));

        // Check for vulnerabilities in dependencies
        vulnerabilities.extend(self.check_vulnerabilities(asset));

        // Scan code for security issues from BlockMatrix Asset metadata
        // STUB: PackageSpecMetadata doesn't have code - would need to check content
        // if let Some(code) = asset.metadata().get("code") {
        if !asset.content.main_content.is_empty() {
            let code_str = &asset.content.main_content;
            {
                // Scan for various injection types
                injection_risks.extend(self.scan_sql_injection(code_str));
                injection_risks.extend(self.scan_command_injection(code_str));
                injection_risks.extend(self.scan_path_traversal(code_str));

                // Check for suspicious patterns (simplified malware detection)
                if code_str.contains("ransomware") || code_str.contains("cryptolocker") {
                    malware.push(MalwareDetection {
                        malware_type: "Ransomware".to_string(),
                        confidence: 90,
                        affected_files: vec![asset.package_hash.clone()],
                        signature: "RANSOMWARE_PATTERN_001".to_string(),
                        risk_level: RiskLevel::Critical,
                    });
                }

                // Check for hardcoded credentials
                if code_str.contains("password = \"") || code_str.contains("api_key = \"") {
                    rule_failures.push(SecurityRuleFailure {
                        rule_id: "no-hardcoded-credentials".to_string(),
                        description: "Hardcoded credentials detected".to_string(),
                        location: CodeLocation {
                            file: asset.package_hash.clone(),
                            line: None,
                            column: None,
                            snippet: None,
                        },
                        severity: SecuritySeverity::High,
                    });
                }
            }
        }

        // Generate recommendations
        if !vulnerabilities.is_empty() {
            recommendations
                .push("Update vulnerable dependencies to latest secure versions".to_string());
        }
        if !injection_risks.is_empty() {
            recommendations.push("Implement input validation and sanitization".to_string());
        }
        if !rule_failures.is_empty() {
            recommendations
                .push("Use environment variables or secure vaults for credentials".to_string());
        }

        let mut result = SecurityValidationResult {
            score: 100,
            vulnerabilities,
            malware,
            injection_risks,
            rule_failures,
            recommendations,
        };

        // Calculate final score
        result.score = self.calculate_score(&result);

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{
        AssetContentResolved, AssetSpec, AssetSpecification, PackageSpecMetadata,
    };

    fn make_test_package(name: &str, content: &str, hash: &str) -> AssetPackage {
        AssetPackage {
            spec: AssetSpec {
                api_version: "v1".to_string(),
                kind: "test".to_string(),
                metadata: PackageSpecMetadata {
                    name: name.to_string(),
                    version: "1.0.0".to_string(),
                    tags: vec![],
                    description: None,
                    author: Some("test-author".to_string()),
                    license: None,
                    homepage: None,
                    repository: None,
                    download_count: 0,
                    featured: false,
                    keywords: vec![],
                    created: None,
                    updated: None,
                },
                spec: AssetSpecification {
                    asset_type: "package".to_string(),
                    content: crate::AssetContent {
                        main: String::new(),
                        files: vec![],
                        inline: None,
                        binary: vec![],
                        templates: vec![],
                    },
                    security: crate::AssetSecurity {
                        consensus_required: false,
                        certificate_pinning: false,
                        hash_validation: "blake3".to_string(),
                        sandbox_level: "strict".to_string(),
                        allowed_syscalls: vec![],
                        network_access: crate::assets::types::NetworkAccess {
                            enabled: false,
                            allowed_domains: vec![],
                            allowed_ports: vec![],
                            require_tls: true,
                        },
                        file_access: crate::assets::types::FileAccess {
                            level: "none".to_string(),
                            allowed_paths: vec![],
                            denied_paths: vec![],
                            allow_temp: false,
                        },
                        permissions: vec![],
                    },
                    resources: crate::AssetResources {
                        cpu_limit: "1000m".to_string(),
                        memory_limit: "512Mi".to_string(),
                        execution_timeout: "30s".to_string(),
                        storage_required: None,
                        network_bandwidth: None,
                        gpu_required: false,
                        hardware_requirements: vec![],
                    },
                    execution: crate::AssetExecution {
                        delegation_strategy: "any".to_string(),
                        minimum_consensus: 1,
                        retry_policy: "none".to_string(),
                        max_concurrent: None,
                        priority: "normal".to_string(),
                        timeout_config: crate::assets::types::TimeoutConfig {
                            execution: "30s".to_string(),
                            network: "10s".to_string(),
                            io: "5s".to_string(),
                            compilation: None,
                        },
                        scheduling: crate::assets::types::SchedulingConfig {
                            timing: "immediate".to_string(),
                            allocation_strategy: "best_fit".to_string(),
                            node_affinity: vec![],
                            anti_affinity: vec![],
                        },
                    },
                    dependencies: vec![],
                    environment: std::collections::HashMap::new(),
                    config_schema: None,
                },
            },
            content: AssetContentResolved {
                main_content: content.to_string(),
                file_contents: std::collections::HashMap::new(),
                binary_contents: std::collections::HashMap::new(),
                template_content: std::collections::HashMap::new(),
                resolved_dependencies: vec![],
            },
            validation: crate::assets::registry::AssetValidationStatus {
                is_valid: true,
                validated_at: chrono::Utc::now(),
                errors: vec![],
                warnings: vec![],
                security_results: crate::assets::registry::SecurityScanResults {
                    security_score: 100,
                    vulnerabilities: vec![],
                    recommendations: vec![],
                    scanned_at: chrono::Utc::now(),
                },
                dependency_results: Default::default(),
            },
            package_hash: hash.to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            signature: None,
        }
    }

    #[tokio::test]
    async fn test_scanner_validates_size_and_hash() {
        let scanner = StaticSecurityScanner::new();
        let package = make_test_package("good-pkg", "hello world", "some-hash");

        let result = scanner.scan(&package).await.expect("test: scan should succeed");
        // Should pass: small size, non-hex hash (not treated as BLAKE3)
        assert!(result.score > 0, "score should be positive for valid package");
    }

    #[tokio::test]
    async fn test_scanner_detects_empty_metadata() {
        let scanner = StaticSecurityScanner::new();
        let mut package = make_test_package("", "content", "hash");
        package.spec.metadata.version = String::new();

        let result = scanner.scan(&package).await.expect("test: scan should succeed");
        // Should have vulnerabilities for empty name and version
        let metadata_issues: Vec<_> = result
            .vulnerabilities
            .iter()
            .filter(|v| v.component == "metadata")
            .collect();
        assert!(
            metadata_issues.len() >= 2,
            "should detect missing name and version"
        );
    }

    #[tokio::test]
    async fn test_scanner_type_specific_json_validation() {
        let scanner = StaticSecurityScanner::new();
        let mut package = make_test_package("json-pkg", "not valid json {{{", "hash");
        package.spec.spec.asset_type = "json".to_string();

        let result = scanner.scan(&package).await.expect("test: scan should succeed");
        let type_issues: Vec<_> = result
            .vulnerabilities
            .iter()
            .filter(|v| v.component == "type-validation")
            .collect();
        assert!(
            !type_issues.is_empty(),
            "should detect invalid JSON content"
        );
    }
}
