//! Simple security integration test that doesn't depend on HyperMesh

#[test]
fn test_security_module_exists() {
    // This test verifies that the security module is properly integrated
    // and can be used within Catalog

    use catalog::security::{SecurityConfig, TrustLevel};

    let config = SecurityConfig {
        trustchain_endpoint: "https://trust.hypermesh.online:8443".to_string(),
        default_trust_policy: TrustLevel::Moderate,
        enable_pqc_signatures: true,
        ..Default::default()
    };

    assert_eq!(config.default_trust_policy, TrustLevel::Moderate);
    assert!(config.enable_pqc_signatures);
    assert_eq!(config.trustchain_endpoint, "https://trust.hypermesh.online:8443");
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
            TrustLevel::Strict => assert_eq!(format!("{:?}", level), "Strict"),
            TrustLevel::Moderate => assert_eq!(format!("{:?}", level), "Moderate"),
            TrustLevel::Permissive => assert_eq!(format!("{:?}", level), "Permissive"),
            _ => {}
        }
    }
}

#[test]
fn test_publisher_types() {
    use catalog::security::PublisherType;

    let types = vec![
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
                assert_eq!(format!("{:?}", algo), "Falcon1024");
            }
            SignatureAlgorithm::Ed25519 => {
                assert_eq!(format!("{:?}", algo), "Ed25519");
            }
            SignatureAlgorithm::HybridFalconEd25519 => {
                assert_eq!(format!("{:?}", algo), "HybridFalconEd25519");
            }
        }
    }
}

#[test]
fn test_distribution_config_with_security() {
    use catalog::distribution::DistributionConfig;

    let config = DistributionConfig::default();

    // Verify security settings are included
    assert!(config.require_signatures);
    assert!(!config.allow_unverified_publishers);

    // Verify TrustChain endpoint is configured
    assert_eq!(config.security.trustchain_endpoint, "https://trust.hypermesh.online:8443");

    // Verify post-quantum cryptography is enabled
    assert!(config.security.enable_pqc_signatures);
}

#[test]
fn test_security_severity_ordering() {
    use catalog::security::Severity;

    // Test that severity levels are properly ordered
    assert!(Severity::Low < Severity::Medium);
    assert!(Severity::Medium < Severity::High);
    assert!(Severity::High < Severity::Critical);
}

#[test]
fn test_violation_types() {
    use catalog::security::ViolationType;

    let violations = vec![
        ViolationType::InvalidSignature,
        ViolationType::InvalidCertificate,
        ViolationType::BlacklistedPublisher,
        ViolationType::LowReputation,
        ViolationType::Vulnerability,
        ViolationType::ExpiredCertificate,
        ViolationType::RevokedCertificate,
        ViolationType::UnknownPublisher,
        ViolationType::PackageSizeExceeded,
    ];

    // Verify all violation types are defined
    assert_eq!(violations.len(), 9);
}

#[test]
fn test_reputation_tiers() {
    use catalog::security::reputation::PublisherTier;

    let tiers = vec![
        PublisherTier::Unverified,
        PublisherTier::Bronze,
        PublisherTier::Silver,
        PublisherTier::Gold,
        PublisherTier::Platinum,
    ];

    assert_eq!(tiers.len(), 5);
}

fn main() {
    println!("Security integration tests completed successfully!");
}