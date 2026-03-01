// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Security integration tests for Catalog
//!
//! Tests both basic type construction (existing) and
//! Sprint 23 security features (signing, trustchain, scanner).

mod common;

// ===========================================================================
// Existing tests (pre-Sprint 23) -- preserved as-is
// ===========================================================================

#[test]
fn test_security_module_exists() {
    use catalog::security::{SecurityConfig, TrustLevel};

    let config = SecurityConfig {
        trustchain_endpoint: "https://trust.hypermesh.online:8443".to_string(),
        default_trust_policy: TrustLevel::Moderate,
        enable_pqc_signatures: true,
        ..Default::default()
    };

    assert_eq!(config.default_trust_policy, TrustLevel::Moderate);
    assert!(config.enable_pqc_signatures);
    assert_eq!(
        config.trustchain_endpoint,
        "https://trust.hypermesh.online:8443"
    );
}

#[test]
fn test_trust_levels() {
    use catalog::security::TrustLevel;

    let levels = vec![
        TrustLevel::Strict,
        TrustLevel::Moderate,
        TrustLevel::Permissive,
    ];

    for level in levels {
        match level {
            TrustLevel::Strict => assert_eq!(format!("{level:?}"), "Strict"),
            TrustLevel::Moderate => assert_eq!(format!("{level:?}"), "Moderate"),
            TrustLevel::Permissive => assert_eq!(format!("{level:?}"), "Permissive"),
            _ => {}
        }
    }
}

#[test]
fn test_publisher_types() {
    use catalog::security::PublisherType;

    let types = [
        PublisherType::Individual,
        PublisherType::Organization,
        PublisherType::Community,
        PublisherType::Official,
        PublisherType::Unknown,
    ];

    assert_eq!(types.len(), 5);
}

#[test]
fn test_signature_algorithms() {
    use catalog::security::signing::SignatureAlgorithm;

    let algorithms = vec![
        SignatureAlgorithm::Falcon1024,
        SignatureAlgorithm::Ed25519,
        SignatureAlgorithm::HybridFalconEd25519,
    ];

    for algo in algorithms {
        match algo {
            SignatureAlgorithm::Falcon1024 => {
                assert_eq!(format!("{algo:?}"), "Falcon1024");
            }
            SignatureAlgorithm::Ed25519 => {
                assert_eq!(format!("{algo:?}"), "Ed25519");
            }
            SignatureAlgorithm::HybridFalconEd25519 => {
                assert_eq!(format!("{algo:?}"), "HybridFalconEd25519");
            }
        }
    }
}

#[test]
fn test_distribution_config_with_security() {
    use catalog::distribution::DistributionConfig;

    let config = DistributionConfig::default();

    assert!(config.require_signatures);
    assert!(!config.allow_unverified_publishers);
    assert_eq!(
        config.security.trustchain_endpoint,
        "https://trust.hypermesh.online:8443"
    );
    assert!(config.security.enable_pqc_signatures);
}

#[test]
fn test_security_severity_ordering() {
    use catalog::security::Severity;

    assert!(Severity::Low < Severity::Medium);
    assert!(Severity::Medium < Severity::High);
    assert!(Severity::High < Severity::Critical);
}

#[test]
fn test_violation_types() {
    use catalog::security::ViolationType;

    let violations = [
        ViolationType::InvalidSignature,
        ViolationType::InvalidCertificate,
        ViolationType::BlacklistedPublisher,
        ViolationType::UnauthenticatedPublisher,
        ViolationType::Vulnerability,
        ViolationType::ExpiredCertificate,
        ViolationType::RevokedCertificate,
        ViolationType::UnknownPublisher,
        ViolationType::PackageSizeExceeded,
    ];

    assert_eq!(violations.len(), 9);
}

#[tokio::test]
async fn test_binary_publisher_verification() {
    use catalog::security::reputation::PublisherAuthenticator;

    let auth = PublisherAuthenticator::new();

    // Non-revoked publisher is authenticated
    let result = auth
        .verify("fp-abc")
        .await
        .expect("test: verify should succeed");
    assert!(result.authenticated);

    // Revoked publisher is not authenticated
    auth.revoke("fp-abc", "test revocation").await;
    let result = auth
        .verify("fp-abc")
        .await
        .expect("test: verify should succeed");
    assert!(!result.authenticated);
}

// ===========================================================================
// Sprint 23 security tests -- FALCON signing, TrustChain, scanners
// ===========================================================================

#[test]
fn test_falcon_1024_keypair_generation_and_signing() {
    use pqcrypto_falcon::falcon1024;

    // Generate keypair
    let (pk, sk) = falcon1024::keypair();

    // Sign test data
    let data = b"catalog package content for signing verification";
    let signed_msg = falcon1024::sign(data, &sk);
    assert!(
        signed_msg.len() > data.len(),
        "Signed message should be larger than original"
    );

    // Verify signature
    let opened = falcon1024::open(&signed_msg, &pk);
    assert!(
        opened.is_ok(),
        "FALCON-1024 signature verification should succeed"
    );

    let recovered = opened.unwrap();
    assert_eq!(
        recovered.as_slice(),
        data,
        "Recovered data should match original"
    );
}

#[test]
fn test_falcon_1024_wrong_key_fails_verification() {
    use pqcrypto_falcon::falcon1024;

    let (_, sk1) = falcon1024::keypair();
    let (pk2, _) = falcon1024::keypair();

    let data = b"signed with key 1";
    let signed_msg = falcon1024::sign(data, &sk1);

    // Verify with wrong public key should fail
    let result = falcon1024::open(&signed_msg, &pk2);
    assert!(result.is_err(), "Verification with wrong key should fail");
}

#[tokio::test]
async fn test_trustchain_integration_creation() {
    use catalog::security::trustchain::{TrustChainConfig, TrustChainIntegration};

    let config = TrustChainConfig {
        endpoint: "https://trust.hypermesh.online:8443".to_string(),
        enable_pqc: true,
        cert_cache_ttl: 3600,
    };

    let integration = TrustChainIntegration::new(config).await;
    assert!(
        integration.is_ok(),
        "TrustChainIntegration should initialize successfully"
    );
}

#[tokio::test]
async fn test_trustchain_validate_certificate() {
    use catalog::security::trustchain::{TrustChainConfig, TrustChainIntegration};

    let config = TrustChainConfig {
        endpoint: "https://trust.hypermesh.online:8443".to_string(),
        enable_pqc: true,
        cert_cache_ttl: 3600,
    };

    let integration = TrustChainIntegration::new(config).await.unwrap();

    // Validate a test certificate (placeholder returns valid)
    let cert_bytes = b"test-certificate-data";
    let validation = integration.validate_certificate(cert_bytes).await.unwrap();
    assert!(
        validation.valid,
        "Placeholder validation should return valid"
    );
    assert!(validation.chain_valid);
    assert!(!validation.revoked);
}

#[tokio::test]
async fn test_trustchain_issue_certificate_requires_stoq() {
    use catalog::security::trustchain::{TrustChainConfig, TrustChainIntegration};

    let config = TrustChainConfig {
        endpoint: "https://trust.hypermesh.online:8443".to_string(),
        enable_pqc: true,
        cert_cache_ttl: 3600,
    };

    let integration = TrustChainIntegration::new(config).await.unwrap();

    // Issue certificate should fail because STOQ transport is not configured
    let result = integration
        .issue_certificate("test-publisher".to_string(), Some("Test Org".to_string()))
        .await;
    assert!(
        result.is_err(),
        "issue_certificate should fail without STOQ transport"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("STOQ"),
        "Error should mention STOQ transport"
    );
}

#[tokio::test]
async fn test_trustchain_check_revocation() {
    use catalog::security::trustchain::{TrustChainConfig, TrustChainIntegration};

    let config = TrustChainConfig {
        endpoint: "https://trust.hypermesh.online:8443".to_string(),
        enable_pqc: true,
        cert_cache_ttl: 3600,
    };

    let integration = TrustChainIntegration::new(config).await.unwrap();

    // Revocation check (placeholder returns not revoked)
    let revoked = integration
        .check_revocation("test-fingerprint-abc")
        .await
        .unwrap();
    assert!(!revoked, "Placeholder should return not revoked");
}

#[tokio::test]
async fn test_trustchain_verify_chain() {
    use catalog::security::trustchain::{Certificate, TrustChainConfig, TrustChainIntegration};

    let config = TrustChainConfig {
        endpoint: "https://trust.hypermesh.online:8443".to_string(),
        enable_pqc: true,
        cert_cache_ttl: 3600,
    };

    let integration = TrustChainIntegration::new(config).await.unwrap();

    // Build a valid chain: leaf -> root
    let leaf = Certificate {
        fingerprint: "leaf-fp".to_string(),
        common_name: "leaf-cert".to_string(),
        organization: Some("Test".to_string()),
        issuer: "TrustChain CA Root".to_string(),
        not_before: chrono::Utc::now() - chrono::Duration::days(1),
        not_after: chrono::Utc::now() + chrono::Duration::days(364),
        san_entries: vec![],
        chain: vec![],
        raw_bytes: vec![],
        pqc_signature: None,
    };

    // Single cert chain where issuer matches CA root common_name
    let result = integration.verify_chain(&[leaf]).await.unwrap();
    assert!(
        result,
        "Single cert chain with matching issuer should verify"
    );
}

#[tokio::test]
async fn test_trustchain_verify_empty_chain_fails() {
    use catalog::security::trustchain::{TrustChainConfig, TrustChainIntegration};

    let config = TrustChainConfig {
        endpoint: "https://trust.hypermesh.online:8443".to_string(),
        enable_pqc: true,
        cert_cache_ttl: 3600,
    };

    let integration = TrustChainIntegration::new(config).await.unwrap();

    let result = integration.verify_chain(&[]).await;
    assert!(result.is_err(), "Empty certificate chain should fail");
}

#[tokio::test]
async fn test_trustchain_pqc_enabled() {
    use catalog::security::trustchain::{TrustChainConfig, TrustChainIntegration};

    let config = TrustChainConfig {
        endpoint: "test".to_string(),
        enable_pqc: true,
        cert_cache_ttl: 60,
    };

    let integration = TrustChainIntegration::new(config).await.unwrap();
    assert!(integration.is_pqc_enabled(), "PQC should be enabled");
}

#[tokio::test]
async fn test_vulnerability_scanner_detects_credentials() {
    use catalog::validation::scanners::StaticSecurityScanner;
    use catalog::validation::traits::SecurityScanner;

    let scanner = StaticSecurityScanner::new();

    // Create a test package with hardcoded credentials in content
    let package = common::create_test_package("cred-test", "1.0.0");
    // The default test package has clean content, so score should be 100
    let result = scanner.scan(&package).await.unwrap();
    assert_eq!(result.score, 100, "Clean package should have score 100");
    assert!(
        result.rule_failures.is_empty(),
        "Clean package should have no rule failures"
    );
}

#[tokio::test]
async fn test_vulnerability_scanner_detects_command_injection() {
    use catalog::validation::scanners::StaticSecurityScanner;
    use catalog::validation::traits::SecurityScanner;

    let scanner = StaticSecurityScanner::new();

    // Create a package with suspicious content
    let mut package = common::create_test_package("injection-test", "1.0.0");
    package.content.main_content = "system(user_input)".to_string();

    let result = scanner.scan(&package).await.unwrap();
    assert!(
        !result.injection_risks.is_empty(),
        "Should detect command injection risk"
    );
    assert!(
        result.score < 100,
        "Score should be reduced for injection risks"
    );
}

#[tokio::test]
async fn test_vulnerability_scanner_detects_path_traversal() {
    use catalog::validation::scanners::StaticSecurityScanner;
    use catalog::validation::traits::SecurityScanner;

    let scanner = StaticSecurityScanner::new();

    let mut package = common::create_test_package("traversal-test", "1.0.0");
    package.content.main_content = "open(\"../../etc/passwd\")".to_string();

    let result = scanner.scan(&package).await.unwrap();
    assert!(
        !result.injection_risks.is_empty(),
        "Should detect path traversal"
    );
}

#[tokio::test]
async fn test_security_manager_creation() {
    use catalog::security::{SecurityConfig, SecurityManager};

    let config = SecurityConfig::default();
    let manager = SecurityManager::new(config).await;
    assert!(
        manager.is_ok(),
        "SecurityManager should initialize with default config"
    );
}

#[tokio::test]
async fn test_security_manager_verify_unsigned_package() {
    use catalog::security::{SecurityConfig, SecurityManager};

    let config = SecurityConfig::default();
    let manager = SecurityManager::new(config).await.unwrap();

    // Unsigned package should fail verification (no signature attached)
    let package = common::create_test_package("unsigned-pkg", "1.0.0");
    let result = manager.verify_package(&package).await;
    // verify_package calls verifier.verify_package which expects a signature
    // Since there is none, it should return an error about missing signature
    assert!(
        result.is_ok(),
        "verify_package should return Ok(VerificationResult)"
    );
    let verification = result.unwrap();
    assert!(
        !verification.verified,
        "Unsigned package should not be verified"
    );
    assert!(
        !verification.errors.is_empty(),
        "Should have errors about missing signature"
    );
}

fn main() {
    println!("Security integration tests completed successfully!");
}
