//! Security and Trust Module for Catalog
//!
//! Integrates TrustChain certificate-based package verification
//! using quantum-resistant FALCON-1024 signatures

pub mod trustchain;
pub mod signing;
pub mod reputation;
pub mod policies;

use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub use trustchain::{TrustChainIntegration, TrustChainConfig};
pub use signing::{PackageSigner, SignatureVerifier, PackageSignature};
pub use reputation::{PublisherReputation, ReputationSystem};
pub use policies::{TrustPolicy, PolicyEngine, TrustLevel};

use crate::assets::{AssetPackage, AssetPackageId};

/// Security manager for package verification and trust
pub struct SecurityManager {
    /// TrustChain integration for certificate validation
    trustchain: Arc<TrustChainIntegration>,
    /// Package signing system
    signer: Arc<PackageSigner>,
    /// Signature verification system
    verifier: Arc<SignatureVerifier>,
    /// Publisher reputation system
    reputation: Arc<ReputationSystem>,
    /// Trust policy engine
    policy_engine: Arc<PolicyEngine>,
    /// Security configuration
    config: SecurityConfig,
    /// Security metrics
    metrics: Arc<SecurityMetrics>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// TrustChain endpoint
    pub trustchain_endpoint: String,
    /// Default trust policy
    pub default_trust_policy: TrustLevel,
    /// Enable automatic security updates
    pub auto_security_updates: bool,
    /// Enable vulnerability scanning
    pub vulnerability_scanning: bool,
    /// Maximum package size for verification (bytes)
    pub max_package_size: u64,
    /// Certificate cache TTL (seconds)
    pub cert_cache_ttl: u64,
    /// Blacklisted publishers
    pub blacklisted_publishers: Vec<String>,
    /// Whitelisted publishers (bypass some checks)
    pub whitelisted_publishers: Vec<String>,
    /// Enable quantum-resistant signatures (FALCON-1024)
    pub enable_pqc_signatures: bool,
    /// Enable certificate pinning
    pub enable_cert_pinning: bool,
    /// Pinned certificates (publisher -> cert fingerprint)
    pub pinned_certificates: HashMap<String, String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            trustchain_endpoint: "https://trust.hypermesh.online:8443".to_string(),
            default_trust_policy: TrustLevel::Moderate,
            auto_security_updates: true,
            vulnerability_scanning: true,
            max_package_size: 100 * 1024 * 1024, // 100MB
            cert_cache_ttl: 3600, // 1 hour
            blacklisted_publishers: vec![],
            whitelisted_publishers: vec![],
            enable_pqc_signatures: true,
            enable_cert_pinning: false,
            pinned_certificates: HashMap::new(),
        }
    }
}

/// Security metrics for monitoring
#[derive(Debug, Default)]
pub struct SecurityMetrics {
    /// Total packages verified
    pub packages_verified: Arc<std::sync::atomic::AtomicU64>,
    /// Failed verifications
    pub verification_failures: Arc<std::sync::atomic::AtomicU64>,
    /// Certificate validations
    pub cert_validations: Arc<std::sync::atomic::AtomicU64>,
    /// Invalid certificates encountered
    pub invalid_certs: Arc<std::sync::atomic::AtomicU64>,
    /// Blacklisted packages blocked
    pub blacklisted_blocked: Arc<std::sync::atomic::AtomicU64>,
    /// Vulnerability scans performed
    pub vulnerability_scans: Arc<std::sync::atomic::AtomicU64>,
    /// Vulnerabilities detected
    pub vulnerabilities_found: Arc<std::sync::atomic::AtomicU64>,
}

/// Package verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the package is verified
    pub verified: bool,
    /// Verification timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Signature verification details
    pub signature_valid: bool,
    /// Certificate validation details
    pub certificate_valid: bool,
    /// Publisher identity
    pub publisher: Option<PublisherIdentity>,
    /// Publisher reputation score
    pub reputation_score: Option<f64>,
    /// Trust policy evaluation
    pub policy_result: PolicyResult,
    /// Detected vulnerabilities
    pub vulnerabilities: Vec<Vulnerability>,
    /// Warning messages
    pub warnings: Vec<String>,
    /// Error messages
    pub errors: Vec<String>,
}

/// Publisher identity from certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherIdentity {
    /// Publisher common name
    pub common_name: String,
    /// Publisher organization
    pub organization: Option<String>,
    /// Certificate fingerprint
    pub cert_fingerprint: String,
    /// Certificate issuer
    pub cert_issuer: String,
    /// Certificate validity period
    pub cert_validity: CertificateValidity,
    /// Publisher's TrustChain ID
    pub trustchain_id: String,
    /// Publisher type
    pub publisher_type: PublisherType,
}

/// Certificate validity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidity {
    /// Not valid before
    pub not_before: chrono::DateTime<chrono::Utc>,
    /// Not valid after
    pub not_after: chrono::DateTime<chrono::Utc>,
    /// Is currently valid
    pub is_valid: bool,
    /// Days until expiration
    pub days_until_expiry: Option<i64>,
}

/// Publisher type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PublisherType {
    /// Individual developer
    Individual,
    /// Organization with verified identity
    Organization,
    /// Community project
    Community,
    /// Official HyperMesh package
    Official,
    /// Unknown/unverified
    Unknown,
}

/// Policy evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    /// Whether policy allows installation
    pub allowed: bool,
    /// Applied trust level
    pub trust_level: TrustLevel,
    /// Policy violations
    pub violations: Vec<PolicyViolation>,
    /// Policy recommendations
    pub recommendations: Vec<String>,
}

/// Policy violation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    /// Violation type
    pub violation_type: ViolationType,
    /// Severity level
    pub severity: Severity,
    /// Description of violation
    pub description: String,
    /// Remediation steps
    pub remediation: Option<String>,
}

/// Types of policy violations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViolationType {
    /// Invalid or missing signature
    InvalidSignature,
    /// Certificate validation failure
    InvalidCertificate,
    /// Publisher is blacklisted
    BlacklistedPublisher,
    /// Low reputation score
    LowReputation,
    /// Package contains vulnerabilities
    Vulnerability,
    /// Certificate expired
    ExpiredCertificate,
    /// Certificate revoked
    RevokedCertificate,
    /// Unknown publisher
    UnknownPublisher,
    /// Package too large
    PackageSizeExceeded,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Ord, PartialOrd, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Detected vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    /// CVE identifier
    pub cve_id: Option<String>,
    /// Vulnerability title
    pub title: String,
    /// Description
    pub description: String,
    /// Severity level
    pub severity: Severity,
    /// Affected component
    pub component: String,
    /// Fixed version
    pub fixed_version: Option<String>,
    /// Exploit available
    pub exploit_available: bool,
}

impl SecurityManager {
    /// Create a new security manager
    pub async fn new(config: SecurityConfig) -> Result<Self> {
        // Initialize TrustChain integration
        let trustchain_config = TrustChainConfig {
            endpoint: config.trustchain_endpoint.clone(),
            enable_pqc: config.enable_pqc_signatures,
            cert_cache_ttl: config.cert_cache_ttl,
        };
        let trustchain = Arc::new(
            TrustChainIntegration::new(trustchain_config)
                .await
                .context("Failed to initialize TrustChain integration")?
        );

        // Initialize signing and verification systems
        let signer = Arc::new(PackageSigner::new(trustchain.clone()).await?);
        let verifier = Arc::new(SignatureVerifier::new(trustchain.clone()).await?);

        // Initialize reputation system
        let reputation = Arc::new(ReputationSystem::new().await?);

        // Initialize policy engine
        let policy_engine = Arc::new(PolicyEngine::new(config.default_trust_policy));

        Ok(Self {
            trustchain,
            signer,
            verifier,
            reputation,
            policy_engine,
            config,
            metrics: Arc::new(SecurityMetrics::default()),
        })
    }

    /// Sign a package before publishing
    pub async fn sign_package(
        &self,
        package: &mut AssetPackage,
        publisher_cert: &[u8],
        private_key: &[u8],
    ) -> Result<PackageSignature> {
        // Increment metrics
        self.metrics.packages_verified.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Sign the package
        let signature = self.signer
            .sign_package(package, publisher_cert, private_key)
            .await
            .context("Failed to sign package")?;

        // Attach signature to package
        package.attach_signature(signature.clone());

        Ok(signature)
    }

    /// Verify a package before installation
    pub async fn verify_package(
        &self,
        package: &AssetPackage,
    ) -> Result<VerificationResult> {
        let mut result = VerificationResult {
            verified: false,
            timestamp: chrono::Utc::now(),
            signature_valid: false,
            certificate_valid: false,
            publisher: None,
            reputation_score: None,
            policy_result: PolicyResult {
                allowed: false,
                trust_level: self.config.default_trust_policy,
                violations: vec![],
                recommendations: vec![],
            },
            vulnerabilities: vec![],
            warnings: vec![],
            errors: vec![],
        };

        // Check package size
        if package.calculate_size() as u64 > self.config.max_package_size {
            result.policy_result.violations.push(PolicyViolation {
                violation_type: ViolationType::PackageSizeExceeded,
                severity: Severity::Medium,
                description: format!("Package size exceeds maximum allowed size of {} bytes",
                    self.config.max_package_size),
                remediation: Some("Consider splitting the package into smaller components".to_string()),
            });
            return Ok(result);
        }

        // Verify package signature
        match self.verifier.verify_package(package).await {
            Ok(sig_result) => {
                result.signature_valid = sig_result.valid;
                result.certificate_valid = sig_result.certificate_valid;

                if let Some(publisher_info) = sig_result.publisher {
                    // Check blacklist
                    if self.config.blacklisted_publishers.contains(&publisher_info.common_name) {
                        result.policy_result.violations.push(PolicyViolation {
                            violation_type: ViolationType::BlacklistedPublisher,
                            severity: Severity::Critical,
                            description: format!("Publisher '{}' is blacklisted",
                                publisher_info.common_name),
                            remediation: None,
                        });
                        self.metrics.blacklisted_blocked.fetch_add(1,
                            std::sync::atomic::Ordering::Relaxed);
                        return Ok(result);
                    }

                    // Get publisher reputation
                    let reputation_score = self.reputation
                        .get_publisher_score(&publisher_info.trustchain_id)
                        .await?;

                    result.publisher = Some(publisher_info.clone());
                    result.reputation_score = Some(reputation_score);

                    // Check reputation against policy
                    if reputation_score < 0.5 &&
                       self.config.default_trust_policy == TrustLevel::Strict {
                        result.policy_result.violations.push(PolicyViolation {
                            violation_type: ViolationType::LowReputation,
                            severity: Severity::High,
                            description: format!("Publisher reputation score {} is below threshold",
                                reputation_score),
                            remediation: Some("Wait for publisher to build reputation or adjust trust policy".to_string()),
                        });
                    }
                }
            }
            Err(e) => {
                result.errors.push(format!("Signature verification failed: {}", e));
                result.policy_result.violations.push(PolicyViolation {
                    violation_type: ViolationType::InvalidSignature,
                    severity: Severity::Critical,
                    description: "Package signature verification failed".to_string(),
                    remediation: Some("Ensure package is signed with a valid TrustChain certificate".to_string()),
                });
                self.metrics.verification_failures.fetch_add(1,
                    std::sync::atomic::Ordering::Relaxed);
                return Ok(result);
            }
        }

        // Scan for vulnerabilities if enabled
        if self.config.vulnerability_scanning {
            self.metrics.vulnerability_scans.fetch_add(1,
                std::sync::atomic::Ordering::Relaxed);

            let vulnerabilities = self.scan_vulnerabilities(package).await?;
            if !vulnerabilities.is_empty() {
                self.metrics.vulnerabilities_found.fetch_add(vulnerabilities.len() as u64,
                    std::sync::atomic::Ordering::Relaxed);

                for vuln in &vulnerabilities {
                    if vuln.severity >= Severity::High {
                        result.policy_result.violations.push(PolicyViolation {
                            violation_type: ViolationType::Vulnerability,
                            severity: vuln.severity.clone(),
                            description: format!("Critical vulnerability detected: {}", vuln.title),
                            remediation: vuln.fixed_version.as_ref()
                                .map(|v| format!("Update to version {}", v)),
                        });
                    }
                }
                result.vulnerabilities = vulnerabilities;
            }
        }

        // Apply trust policy
        let policy_evaluation = self.policy_engine
            .evaluate_package(&result)
            .await?;

        result.policy_result = policy_evaluation;
        result.verified = result.signature_valid &&
                         result.certificate_valid &&
                         result.policy_result.allowed;

        Ok(result)
    }

    /// Scan package for vulnerabilities
    async fn scan_vulnerabilities(&self, package: &AssetPackage) -> Result<Vec<Vulnerability>> {
        // TODO: Integrate with vulnerability database
        // For now, return empty vec
        Ok(vec![])
    }

    /// Update publisher reputation after installation
    pub async fn update_reputation(
        &self,
        publisher_id: &str,
        success: bool,
        user_rating: Option<u8>,
    ) -> Result<()> {
        self.reputation
            .update_reputation(publisher_id, success, user_rating)
            .await
    }

    /// Get security metrics
    pub fn get_metrics(&self) -> &SecurityMetrics {
        &self.metrics
    }

    /// Clear certificate cache
    pub async fn clear_cert_cache(&self) -> Result<()> {
        self.trustchain.clear_cache().await
    }

    /// Add publisher to blacklist
    pub async fn blacklist_publisher(&mut self, publisher: String) {
        if !self.config.blacklisted_publishers.contains(&publisher) {
            self.config.blacklisted_publishers.push(publisher);
        }
    }

    /// Add publisher to whitelist
    pub async fn whitelist_publisher(&mut self, publisher: String) {
        if !self.config.whitelisted_publishers.contains(&publisher) {
            self.config.whitelisted_publishers.push(publisher);
        }
    }

    /// Pin certificate for a publisher
    pub async fn pin_certificate(&mut self, publisher: String, cert_fingerprint: String) {
        self.config.pinned_certificates.insert(publisher, cert_fingerprint);
    }
}

// Extension trait for AssetPackage
impl AssetPackage {
    /// Attach signature to package
    pub fn attach_signature(&mut self, signature: PackageSignature) {
        self.metadata.custom_fields.insert(
            "signature".to_string(),
            serde_json::to_value(signature).unwrap(),
        );
    }

    /// Get package signature
    pub fn get_signature(&self) -> Option<PackageSignature> {
        self.metadata.custom_fields.get("signature")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_manager_creation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[test]
    fn test_default_config() {
        let config = SecurityConfig::default();
        assert_eq!(config.default_trust_policy, TrustLevel::Moderate);
        assert!(config.enable_pqc_signatures);
        assert!(config.auto_security_updates);
    }
}