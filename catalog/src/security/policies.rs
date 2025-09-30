//! Trust Policies and Policy Engine
//!
//! Configurable trust policies for package installation and verification

use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{info, debug, warn};

use super::{VerificationResult, PolicyResult, PolicyViolation, ViolationType, Severity};

/// Trust level for package verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrustLevel {
    /// Strict: Only allow fully verified packages from trusted publishers
    Strict,
    /// Moderate: Allow verified packages, warn on issues
    Moderate,
    /// Permissive: Allow most packages, only block critical issues
    Permissive,
    /// Custom: User-defined policy rules
    Custom(String),
}

/// Trust policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPolicy {
    /// Policy name
    pub name: String,
    /// Trust level
    pub level: TrustLevel,
    /// Policy rules
    pub rules: Vec<PolicyRule>,
    /// Required checks
    pub required_checks: RequiredChecks,
    /// Allowed publisher types
    pub allowed_publisher_types: Vec<super::PublisherType>,
    /// Minimum reputation score
    pub min_reputation_score: Option<f64>,
    /// Maximum vulnerability severity allowed
    pub max_vulnerability_severity: Option<Severity>,
    /// Allow unsigned packages
    pub allow_unsigned: bool,
    /// Allow expired certificates
    pub allow_expired_certs: bool,
    /// Require certificate pinning
    pub require_cert_pinning: bool,
    /// Custom validation hooks
    pub custom_validators: Vec<String>,
}

/// Policy rule for custom policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule name
    pub name: String,
    /// Rule condition
    pub condition: RuleCondition,
    /// Action to take
    pub action: RuleAction,
    /// Rule priority (lower = higher priority)
    pub priority: u32,
}

/// Rule condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    /// Package name matches pattern
    PackageNamePattern(String),
    /// Publisher matches pattern
    PublisherPattern(String),
    /// Package size exceeds limit
    PackageSizeExceeds(u64),
    /// Has specific dependency
    HasDependency(String),
    /// Certificate issuer matches
    CertificateIssuer(String),
    /// Reputation below threshold
    ReputationBelow(f64),
    /// Contains file pattern
    ContainsFilePattern(String),
    /// Custom condition
    Custom(String),
}

/// Rule actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    /// Block installation
    Block(String), // reason
    /// Warn user
    Warn(String), // message
    /// Require confirmation
    RequireConfirmation(String), // message
    /// Allow installation
    Allow,
    /// Run custom validator
    RunValidator(String),
}

/// Required security checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredChecks {
    /// Require valid signature
    pub signature: bool,
    /// Require valid certificate
    pub certificate: bool,
    /// Require certificate chain validation
    pub certificate_chain: bool,
    /// Require revocation check
    pub revocation_check: bool,
    /// Require vulnerability scan
    pub vulnerability_scan: bool,
    /// Require reputation check
    pub reputation_check: bool,
    /// Require consensus validation
    pub consensus_validation: bool,
    /// Require post-quantum signatures
    pub pqc_signatures: bool,
}

impl Default for RequiredChecks {
    fn default() -> Self {
        Self {
            signature: true,
            certificate: true,
            certificate_chain: true,
            revocation_check: true,
            vulnerability_scan: true,
            reputation_check: true,
            consensus_validation: false,
            pqc_signatures: false,
        }
    }
}

/// Policy engine for evaluating trust policies
pub struct PolicyEngine {
    /// Active policies by name
    policies: Arc<RwLock<HashMap<String, TrustPolicy>>>,
    /// Default trust level
    default_level: TrustLevel,
    /// Policy templates
    templates: Arc<RwLock<HashMap<String, PolicyTemplate>>>,
}

/// Policy template for common scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyTemplate {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Base policy
    pub policy: TrustPolicy,
}

impl PolicyEngine {
    /// Create new policy engine
    pub fn new(default_level: TrustLevel) -> Self {
        let engine = Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            default_level,
            templates: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize default policies
        futures::executor::block_on(engine.initialize_default_policies());

        engine
    }

    /// Initialize default policies
    async fn initialize_default_policies(&self) {
        let mut policies = self.policies.write().await;

        // Strict policy
        policies.insert("strict".to_string(), TrustPolicy {
            name: "strict".to_string(),
            level: TrustLevel::Strict,
            rules: vec![],
            required_checks: RequiredChecks {
                signature: true,
                certificate: true,
                certificate_chain: true,
                revocation_check: true,
                vulnerability_scan: true,
                reputation_check: true,
                consensus_validation: true,
                pqc_signatures: true,
            },
            allowed_publisher_types: vec![
                super::PublisherType::Official,
                super::PublisherType::Organization,
            ],
            min_reputation_score: Some(0.7),
            max_vulnerability_severity: Some(Severity::Low),
            allow_unsigned: false,
            allow_expired_certs: false,
            require_cert_pinning: true,
            custom_validators: vec![],
        });

        // Moderate policy
        policies.insert("moderate".to_string(), TrustPolicy {
            name: "moderate".to_string(),
            level: TrustLevel::Moderate,
            rules: vec![],
            required_checks: RequiredChecks::default(),
            allowed_publisher_types: vec![
                super::PublisherType::Official,
                super::PublisherType::Organization,
                super::PublisherType::Community,
                super::PublisherType::Individual,
            ],
            min_reputation_score: Some(0.3),
            max_vulnerability_severity: Some(Severity::High),
            allow_unsigned: false,
            allow_expired_certs: false,
            require_cert_pinning: false,
            custom_validators: vec![],
        });

        // Permissive policy
        policies.insert("permissive".to_string(), TrustPolicy {
            name: "permissive".to_string(),
            level: TrustLevel::Permissive,
            rules: vec![],
            required_checks: RequiredChecks {
                signature: false,
                certificate: false,
                certificate_chain: false,
                revocation_check: false,
                vulnerability_scan: true,
                reputation_check: false,
                consensus_validation: false,
                pqc_signatures: false,
            },
            allowed_publisher_types: vec![
                super::PublisherType::Official,
                super::PublisherType::Organization,
                super::PublisherType::Community,
                super::PublisherType::Individual,
                super::PublisherType::Unknown,
            ],
            min_reputation_score: None,
            max_vulnerability_severity: None,
            allow_unsigned: true,
            allow_expired_certs: true,
            require_cert_pinning: false,
            custom_validators: vec![],
        });
    }

    /// Evaluate package against policy
    pub async fn evaluate_package(&self, verification: &VerificationResult) -> Result<PolicyResult> {
        let policy = self.get_active_policy().await;

        let mut result = PolicyResult {
            allowed: true,
            trust_level: policy.level.clone(),
            violations: vec![],
            recommendations: vec![],
        };

        // Check required checks
        if policy.required_checks.signature && !verification.signature_valid {
            result.violations.push(PolicyViolation {
                violation_type: ViolationType::InvalidSignature,
                severity: Severity::Critical,
                description: "Package signature is invalid or missing".to_string(),
                remediation: Some("Ensure package is properly signed".to_string()),
            });
            result.allowed = false;
        }

        if policy.required_checks.certificate && !verification.certificate_valid {
            result.violations.push(PolicyViolation {
                violation_type: ViolationType::InvalidCertificate,
                severity: Severity::Critical,
                description: "Publisher certificate is invalid".to_string(),
                remediation: Some("Publisher needs valid TrustChain certificate".to_string()),
            });
            result.allowed = false;
        }

        // Check publisher type
        if let Some(ref publisher) = verification.publisher {
            if !policy.allowed_publisher_types.contains(&publisher.publisher_type) {
                result.violations.push(PolicyViolation {
                    violation_type: ViolationType::UnknownPublisher,
                    severity: Severity::High,
                    description: format!("Publisher type {:?} not allowed by policy",
                                       publisher.publisher_type),
                    remediation: Some("Use packages from allowed publisher types".to_string()),
                });
                result.allowed = false;
            }

            // Check certificate expiry
            if !policy.allow_expired_certs && !publisher.cert_validity.is_valid {
                result.violations.push(PolicyViolation {
                    violation_type: ViolationType::ExpiredCertificate,
                    severity: Severity::High,
                    description: "Publisher certificate has expired".to_string(),
                    remediation: Some("Publisher needs to renew certificate".to_string()),
                });
                result.allowed = false;
            }
        }

        // Check reputation score
        if let Some(min_score) = policy.min_reputation_score {
            if let Some(score) = verification.reputation_score {
                if score < min_score {
                    result.violations.push(PolicyViolation {
                        violation_type: ViolationType::LowReputation,
                        severity: Severity::Medium,
                        description: format!("Publisher reputation {:.2} below required {:.2}",
                                           score, min_score),
                        remediation: Some("Wait for publisher to build reputation".to_string()),
                    });
                    if policy.level == TrustLevel::Strict {
                        result.allowed = false;
                    } else {
                        result.recommendations.push(
                            "Warning: Publisher has low reputation score".to_string()
                        );
                    }
                }
            }
        }

        // Check vulnerabilities
        if !verification.vulnerabilities.is_empty() {
            let max_severity = verification.vulnerabilities
                .iter()
                .map(|v| &v.severity)
                .max()
                .cloned();

            if let Some(max_allowed) = &policy.max_vulnerability_severity {
                if let Some(found_severity) = max_severity {
                    if found_severity > *max_allowed {
                        result.violations.push(PolicyViolation {
                            violation_type: ViolationType::Vulnerability,
                            severity: found_severity.clone(),
                            description: format!("Package contains {:?} severity vulnerabilities",
                                               found_severity),
                            remediation: Some("Update to version without vulnerabilities".to_string()),
                        });
                        result.allowed = false;
                    }
                }
            }
        }

        // Apply custom rules
        for rule in &policy.rules {
            if self.evaluate_rule(rule, verification) {
                match &rule.action {
                    RuleAction::Block(reason) => {
                        result.violations.push(PolicyViolation {
                            violation_type: ViolationType::InvalidSignature, // Generic violation
                            severity: Severity::High,
                            description: reason.clone(),
                            remediation: None,
                        });
                        result.allowed = false;
                    }
                    RuleAction::Warn(message) => {
                        result.recommendations.push(message.clone());
                    }
                    RuleAction::RequireConfirmation(message) => {
                        result.recommendations.push(format!("⚠️  {}", message));
                    }
                    RuleAction::Allow => {
                        // No action needed
                    }
                    RuleAction::RunValidator(validator) => {
                        result.recommendations.push(
                            format!("Custom validator '{}' needs to be run", validator)
                        );
                    }
                }
            }
        }

        // Add general recommendations
        if result.allowed {
            if verification.warnings.len() > 0 {
                result.recommendations.push(
                    format!("Package has {} warnings", verification.warnings.len())
                );
            }

            if policy.require_cert_pinning && verification.publisher.is_some() {
                result.recommendations.push(
                    "Consider pinning this publisher's certificate for enhanced security".to_string()
                );
            }
        }

        Ok(result)
    }

    /// Get active policy based on trust level
    async fn get_active_policy(&self) -> TrustPolicy {
        let policies = self.policies.read().await;

        match &self.default_level {
            TrustLevel::Strict => {
                policies.get("strict").cloned().unwrap_or_else(|| self.default_policy())
            }
            TrustLevel::Moderate => {
                policies.get("moderate").cloned().unwrap_or_else(|| self.default_policy())
            }
            TrustLevel::Permissive => {
                policies.get("permissive").cloned().unwrap_or_else(|| self.default_policy())
            }
            TrustLevel::Custom(name) => {
                policies.get(name).cloned().unwrap_or_else(|| self.default_policy())
            }
        }
    }

    /// Get default policy
    fn default_policy(&self) -> TrustPolicy {
        TrustPolicy {
            name: "default".to_string(),
            level: TrustLevel::Moderate,
            rules: vec![],
            required_checks: RequiredChecks::default(),
            allowed_publisher_types: vec![
                super::PublisherType::Official,
                super::PublisherType::Organization,
                super::PublisherType::Community,
            ],
            min_reputation_score: Some(0.5),
            max_vulnerability_severity: Some(Severity::Medium),
            allow_unsigned: false,
            allow_expired_certs: false,
            require_cert_pinning: false,
            custom_validators: vec![],
        }
    }

    /// Evaluate a single rule
    fn evaluate_rule(&self, rule: &PolicyRule, verification: &VerificationResult) -> bool {
        match &rule.condition {
            RuleCondition::ReputationBelow(threshold) => {
                verification.reputation_score
                    .map(|score| score < *threshold)
                    .unwrap_or(true)
            }
            RuleCondition::PublisherPattern(pattern) => {
                verification.publisher.as_ref()
                    .map(|p| p.common_name.contains(pattern))
                    .unwrap_or(false)
            }
            // TODO: Implement other conditions
            _ => false,
        }
    }

    /// Add custom policy
    pub async fn add_policy(&self, policy: TrustPolicy) -> Result<()> {
        let mut policies = self.policies.write().await;
        policies.insert(policy.name.clone(), policy);
        Ok(())
    }

    /// Remove custom policy
    pub async fn remove_policy(&self, name: &str) -> Result<()> {
        let mut policies = self.policies.write().await;
        policies.remove(name);
        Ok(())
    }

    /// List available policies
    pub async fn list_policies(&self) -> Vec<String> {
        let policies = self.policies.read().await;
        policies.keys().cloned().collect()
    }

    /// Get policy by name
    pub async fn get_policy(&self, name: &str) -> Option<TrustPolicy> {
        let policies = self.policies.read().await;
        policies.get(name).cloned()
    }

    /// Set default trust level
    pub async fn set_default_level(&mut self, level: TrustLevel) {
        self.default_level = level;
    }

    /// Export policies
    pub async fn export_policies(&self) -> Result<Vec<TrustPolicy>> {
        let policies = self.policies.read().await;
        Ok(policies.values().cloned().collect())
    }

    /// Import policies
    pub async fn import_policies(&self, policies: Vec<TrustPolicy>) -> Result<()> {
        let mut policy_map = self.policies.write().await;
        for policy in policies {
            policy_map.insert(policy.name.clone(), policy);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_engine_creation() {
        let engine = PolicyEngine::new(TrustLevel::Moderate);
        assert_eq!(engine.default_level, TrustLevel::Moderate);
    }

    #[tokio::test]
    async fn test_default_policies() {
        let engine = PolicyEngine::new(TrustLevel::Strict);

        let policies = engine.list_policies().await;
        assert!(policies.contains(&"strict".to_string()));
        assert!(policies.contains(&"moderate".to_string()));
        assert!(policies.contains(&"permissive".to_string()));
    }

    #[tokio::test]
    async fn test_custom_policy() {
        let engine = PolicyEngine::new(TrustLevel::Moderate);

        let custom_policy = TrustPolicy {
            name: "custom-test".to_string(),
            level: TrustLevel::Custom("custom-test".to_string()),
            rules: vec![],
            required_checks: RequiredChecks::default(),
            allowed_publisher_types: vec![super::PublisherType::Official],
            min_reputation_score: Some(0.9),
            max_vulnerability_severity: None,
            allow_unsigned: false,
            allow_expired_certs: false,
            require_cert_pinning: true,
            custom_validators: vec![],
        };

        engine.add_policy(custom_policy).await.unwrap();

        let policies = engine.list_policies().await;
        assert!(policies.contains(&"custom-test".to_string()));
    }
}