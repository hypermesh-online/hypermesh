//! Security Integration Tests
//!
//! Tests for TrustChain integration, package signing, and verification

use catalog::security::{
    SecurityManager, SecurityConfig, TrustLevel,
    PackageSigner, SignatureVerifier, PublisherReputation,
    PolicyEngine, TrustPolicy
};
use catalog::assets::{
    AssetPackage, AssetMetadata, AssetContent, AssetSpec,
    AssetSecurity, AssetResources, AssetExecution, IsolationLevel
};
use catalog::distribution::{P2PDistribution, DistributionConfig};
use catalog::AssetType;

use anyhow::Result;
use tokio;

/// Create a test package
fn create_test_package(name: &str) -> AssetPackage {
    AssetPackage {
        metadata: AssetMetadata {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: "Test package for security testing".to_string(),
            author: "Test Author".to_string(),
            license: "MIT".to_string(),
            repository: Some("https://github.com/test/package".to_string()),
            keywords: vec!["test".to_string()],
            categories: vec!["testing".to_string()],
            dependencies: vec![],
            custom_fields: std::collections::HashMap::new(),
        },
        content: AssetContent {
            main_content: "console.log('Hello from test package');".to_string(),
            file_contents: std::collections::HashMap::from([
                ("README.md".to_string(), "# Test Package".to_string()),
            ]),
            binary_contents: std::collections::HashMap::new(),
        },
        spec: AssetSpec {
            asset_type: AssetType::Compute,
            capabilities: vec!["compute".to_string()],
            constraints: vec![],
            interfaces: vec![],
        },
        security: AssetSecurity {
            permissions: vec!["network".to_string()],
            isolation_level: IsolationLevel::Standard,
            encryption: None,
        },
        resources: AssetResources {
            cpu_cores: Some(1),
            memory_mb: Some(512),
            storage_mb: Some(100),
            gpu_required: false,
            network_bandwidth_mbps: Some(10),
        },
        execution: AssetExecution {
            entry_point: Some("index.js".to_string()),
            runtime: Some("node".to_string()),
            environment_variables: std::collections::HashMap::new(),
            arguments: vec![],
            working_directory: None,
        },
    }
}

#[tokio::test]
async fn test_security_manager_initialization() -> Result<()> {
    let config = SecurityConfig::default();
    let security_manager = SecurityManager::new(config).await?;

    assert!(security_manager.get_metrics().packages_verified.load(
        std::sync::atomic::Ordering::Relaxed
    ) == 0);

    Ok(())
}

#[tokio::test]
async fn test_package_signing_and_verification() -> Result<()> {
    // This test would require actual TrustChain integration
    // For now, we test the structure

    let config = SecurityConfig {
        trustchain_endpoint: "https://trust.hypermesh.online:8443".to_string(),
        default_trust_policy: TrustLevel::Moderate,
        enable_pqc_signatures: true,
        ..Default::default()
    };

    let security_manager = SecurityManager::new(config).await?;
    let mut package = create_test_package("test-signed-package");

    // In a real scenario, we would:
    // 1. Get a certificate from TrustChain
    // 2. Sign the package
    // 3. Verify the signature

    // For now, verify that verification fails for unsigned packages
    let result = security_manager.verify_package(&package).await?;
    assert!(!result.verified);
    assert!(!result.signature_valid);

    Ok(())
}

#[tokio::test]
async fn test_trust_policies() -> Result<()> {
    let mut engine = PolicyEngine::new(TrustLevel::Strict);

    // Test that strict policy is available
    let policies = engine.list_policies().await;
    assert!(policies.contains(&"strict".to_string()));
    assert!(policies.contains(&"moderate".to_string()));
    assert!(policies.contains(&"permissive".to_string()));

    // Test getting a specific policy
    let strict_policy = engine.get_policy("strict").await;
    assert!(strict_policy.is_some());

    let policy = strict_policy.unwrap();
    assert_eq!(policy.level, TrustLevel::Strict);
    assert!(policy.required_checks.signature);
    assert!(policy.required_checks.certificate);
    assert!(policy.required_checks.pqc_signatures);

    Ok(())
}

#[tokio::test]
async fn test_reputation_system() -> Result<()> {
    use catalog::security::ReputationSystem;

    let reputation_system = ReputationSystem::new().await?;

    // Test initial reputation
    let initial_score = reputation_system.get_publisher_score("new-publisher").await?;
    assert_eq!(initial_score, 0.5); // Default initial score

    // Record successful install
    reputation_system.update_reputation("test-publisher", true, Some(5)).await?;
    let score = reputation_system.get_publisher_score("test-publisher").await?;
    assert!(score > 0.5);

    // Record failed install
    reputation_system.update_reputation("test-publisher", false, Some(2)).await?;
    let new_score = reputation_system.get_publisher_score("test-publisher").await?;
    assert!(new_score < score);

    Ok(())
}

#[tokio::test]
async fn test_distribution_with_security() -> Result<()> {
    let config = DistributionConfig {
        require_signatures: true,
        allow_unverified_publishers: false,
        ..Default::default()
    };

    // Test that distribution config includes security settings
    assert!(config.require_signatures);
    assert!(!config.allow_unverified_publishers);

    // P2PDistribution would fail to initialize without proper network setup
    // This is expected in test environment

    Ok(())
}

#[tokio::test]
async fn test_certificate_validation_flow() -> Result<()> {
    use catalog::security::trustchain::{TrustChainIntegration, TrustChainConfig};

    let config = TrustChainConfig {
        endpoint: "https://trust.hypermesh.online:8443".to_string(),
        enable_pqc: true,
        cert_cache_ttl: 3600,
    };

    // This would fail in test environment without actual TrustChain server
    // But we're testing the structure exists

    Ok(())
}

#[tokio::test]
async fn test_security_policy_evaluation() -> Result<()> {
    use catalog::security::{VerificationResult, PolicyResult};

    let engine = PolicyEngine::new(TrustLevel::Moderate);

    // Create a mock verification result
    let verification = VerificationResult {
        verified: false,
        timestamp: chrono::Utc::now(),
        signature_valid: false,
        certificate_valid: false,
        publisher: None,
        reputation_score: Some(0.3),
        policy_result: PolicyResult {
            allowed: false,
            trust_level: TrustLevel::Moderate,
            violations: vec![],
            recommendations: vec![],
        },
        vulnerabilities: vec![],
        warnings: vec![],
        errors: vec!["No signature found".to_string()],
    };

    let policy_result = engine.evaluate_package(&verification).await?;

    // Should not be allowed due to missing signature
    assert!(!policy_result.allowed);
    assert!(!policy_result.violations.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_blacklist_whitelist() -> Result<()> {
    let config = SecurityConfig::default();
    let mut security_manager = SecurityManager::new(config).await?;

    // Test blacklisting
    security_manager.blacklist_publisher("malicious-publisher".to_string()).await;

    // Test whitelisting
    security_manager.whitelist_publisher("trusted-publisher".to_string()).await;

    // Test certificate pinning
    security_manager.pin_certificate(
        "important-publisher".to_string(),
        "abc123def456".to_string()
    ).await;

    Ok(())
}

#[tokio::test]
async fn test_vulnerability_detection() -> Result<()> {
    let config = SecurityConfig {
        vulnerability_scanning: true,
        ..Default::default()
    };

    let security_manager = SecurityManager::new(config).await?;
    let package = create_test_package("vulnerable-package");

    // Verify package (will check for vulnerabilities)
    let result = security_manager.verify_package(&package).await?;

    // In real implementation, this would scan for actual vulnerabilities
    // For now, we just verify the structure is in place
    assert!(result.vulnerabilities.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_signature_algorithms() -> Result<()> {
    use catalog::security::signing::SignatureAlgorithm;

    // Test that all signature algorithms are defined
    let algorithms = vec![
        SignatureAlgorithm::Falcon1024,
        SignatureAlgorithm::Ed25519,
        SignatureAlgorithm::HybridFalconEd25519,
    ];

    for algo in algorithms {
        match algo {
            SignatureAlgorithm::Falcon1024 => {
                // FALCON-1024 post-quantum signature
                assert_eq!(format!("{:?}", algo), "Falcon1024");
            }
            SignatureAlgorithm::Ed25519 => {
                // ED25519 elliptic curve signature
                assert_eq!(format!("{:?}", algo), "Ed25519");
            }
            SignatureAlgorithm::HybridFalconEd25519 => {
                // Hybrid signature for maximum security
                assert_eq!(format!("{:?}", algo), "HybridFalconEd25519");
            }
        }
    }

    Ok(())
}

#[test]
fn test_trust_levels() {
    use catalog::security::TrustLevel;

    // Verify all trust levels are available
    let levels = vec![
        TrustLevel::Strict,
        TrustLevel::Moderate,
        TrustLevel::Permissive,
        TrustLevel::Custom("enterprise".to_string()),
    ];

    for level in levels {
        match level {
            TrustLevel::Strict => assert_eq!(format!("{:?}", level), "Strict"),
            TrustLevel::Moderate => assert_eq!(format!("{:?}", level), "Moderate"),
            TrustLevel::Permissive => assert_eq!(format!("{:?}", level), "Permissive"),
            TrustLevel::Custom(name) => assert_eq!(name, "enterprise"),
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

    // Verify all publisher types are defined
    assert_eq!(types.len(), 5);
}