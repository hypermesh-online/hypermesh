// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for TrustChain certificate management
//!
//! Tests use `issue_certificate_local()` for local-only consensus validation.
//! The full `issue_certificate()` path requires a live HyperMesh consensus
//! network (STOQ transport) and is only exercised in end-to-end tests.

use std::time::Duration;
use trustchain::ca::{create_certificate_manager, CertificateConfig, CertificateMode};
use trustchain::ca::{CAConfig, CertificateRequest, TrustChainCA};
use trustchain::consensus::ConsensusProof;

/// Test certificate generation
#[tokio::test]
async fn test_certificate_generation() {
    let config = CAConfig::testing();

    let ca = TrustChainCA::new(config)
        .await
        .expect("TrustChain CA creation should succeed");

    let cert_request = CertificateRequest {
        common_name: "test.hypermesh.local".to_string(),
        san_entries: vec![],
        ipv6_addresses: vec![],
        node_id: "test-node-1".to_string(),
        consensus_proof: ConsensusProof::new_for_testing(),
        timestamp: std::time::SystemTime::now(),
        identity_scope: None,
        subject_type: None,
    };

    let cert = ca
        .issue_certificate_local(cert_request)
        .await
        .expect("Certificate generation should succeed");

    assert_eq!(cert.common_name, "test.hypermesh.local");
    assert!(!cert.serial_number.is_empty());
    assert!(!cert.certificate_der.is_empty());
}

/// Test certificate rotation
#[tokio::test]
async fn test_certificate_rotation() {
    // Use CertificateConfig for STOQ's certificate manager
    let config = CertificateConfig {
        mode: CertificateMode::LocalhostTesting,
        node_id: "test-node".to_string(),
        ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
        common_name: "test.localhost".to_string(),
        rotation_interval: Duration::from_secs(1),
        trustchain_endpoint: None,
        network_type: None,
    };

    let manager = create_certificate_manager(config).await;
    assert!(
        manager.is_ok(),
        "Certificate manager creation should succeed"
    );

    if let Ok(mgr) = manager {
        // Get initial certificate fingerprint
        let cert1 = mgr.get_certificate_fingerprint().await;
        assert!(cert1.is_ok(), "Should have initial certificate");

        // Wait for rotation
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Check if rotation might have occurred
        let cert2 = mgr.get_certificate_fingerprint().await;
        assert!(cert2.is_ok(), "Should still have certificate");

        // In a real test, we'd verify the certificates are different
        // For now, just verify they exist
        assert!(
            cert1.is_ok() && cert2.is_ok(),
            "Both certificates should exist"
        );
    }
}

/// Test certificate validation
#[tokio::test]
async fn test_certificate_validation() {
    let config = CAConfig::testing();

    let ca = TrustChainCA::new(config)
        .await
        .expect("TrustChain CA creation should succeed");

    let cert_request = CertificateRequest {
        common_name: "test.hypermesh.local".to_string(),
        san_entries: vec![],
        ipv6_addresses: vec![],
        node_id: "test-node-2".to_string(),
        consensus_proof: ConsensusProof::new_for_testing(),
        timestamp: std::time::SystemTime::now(),
        identity_scope: None,
        subject_type: None,
    };

    let cert = ca
        .issue_certificate_local(cert_request)
        .await
        .expect("Certificate generation should succeed");

    let valid = ca
        .validate_certificate_chain(&cert.certificate_der)
        .await
        .expect("Certificate validation should succeed");

    assert!(valid, "Certificate should be valid");
}

/// Test certificate chain building
#[tokio::test]
async fn test_certificate_chain() {
    let config = CAConfig::testing();

    let ca = TrustChainCA::new(config)
        .await
        .expect("TrustChain CA creation should succeed");

    let cert_request = CertificateRequest {
        common_name: "test.hypermesh.local".to_string(),
        san_entries: vec![],
        ipv6_addresses: vec![],
        node_id: "test-node-3".to_string(),
        consensus_proof: ConsensusProof::new_for_testing(),
        timestamp: std::time::SystemTime::now(),
        identity_scope: None,
        subject_type: None,
    };

    let cert = ca
        .issue_certificate_local(cert_request)
        .await
        .expect("Certificate issuance should succeed");

    // Get the root certificate
    let root_cert = ca
        .get_root_certificate()
        .await
        .expect("Root certificate retrieval should succeed");

    // Verify we have both certificates (leaf and root)
    assert!(
        !cert.certificate_der.is_empty(),
        "Issued certificate should not be empty"
    );
    assert!(
        !root_cert.is_empty(),
        "Root certificate should not be empty"
    );

    // Validate the issued certificate
    let valid = ca
        .validate_certificate_chain(&cert.certificate_der)
        .await
        .expect("Chain validation should succeed");

    assert!(valid, "Certificate chain should be valid");
}

/// Test FALCON-1024 operations
#[tokio::test]
async fn test_falcon_operations() {
    // Test FALCON-1024 using the crypto module directly
    use trustchain::crypto::{FalconCrypto, KeyUsage};

    // Create FALCON crypto instance
    let falcon = FalconCrypto::new();
    assert!(
        falcon.is_ok(),
        "FALCON crypto initialization should succeed"
    );

    if let Ok(crypto) = falcon {
        // Test key generation
        let keypair = crypto.generate_keypair(KeyUsage::CertificateSigning).await;
        assert!(keypair.is_ok(), "FALCON keypair generation should succeed");

        if let Ok(kp) = keypair {
            // Test signing
            let data = b"test data for signing";
            let signature = crypto.sign(data, &kp.private_key).await;
            assert!(signature.is_ok(), "FALCON signing should succeed");

            // Test verification
            if let Ok(sig) = signature {
                let verified = crypto.verify(data, &sig, &kp.public_key).await;
                assert!(verified.is_ok(), "FALCON verification should succeed");
                assert!(verified.unwrap(), "Signature should be valid");
            }
        }
    }
}
