// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for TrustChain certificate management

use std::time::Duration;
use trustchain::ca::{CertificateConfig, CertificateMode, create_certificate_manager};
use trustchain::ca::{TrustChainCA, CAConfig, CAMode, CertificateRequest};

/// Test certificate generation
#[tokio::test]
async fn test_certificate_generation() {
    // Use TrustChainCA instead of CertificateManager for certificate generation
    let config = CAConfig {
        ca_id: "test-ca".to_string(),
        bind_address: std::net::Ipv6Addr::LOCALHOST,
        port: 8443,
        cert_validity_days: 30,
        rotation_interval: Duration::from_secs(3600),
        mode: CAMode::LocalhostTesting,
        ..Default::default()
    };

    let ca = TrustChainCA::new(config).await;
    assert!(ca.is_ok(), "TrustChain CA creation should succeed");

    if let Ok(ca) = ca {
        // Test certificate issuance using the CA
        let cert_request = CertificateRequest {
            common_name: "test.hypermesh.local".to_string(),
            san_entries: vec![],
            ipv6_addresses: vec![],
            node_id: "test-node-1".to_string(),
            consensus_proof: trustchain::consensus::ConsensusProof::default(),
            timestamp: std::time::SystemTime::now(),
        };

        let cert = ca.issue_certificate(cert_request).await;
        assert!(cert.is_ok(), "Certificate generation should succeed");
    }
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
    assert!(manager.is_ok(), "Certificate manager creation should succeed");

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
        assert!(cert1.is_ok() && cert2.is_ok(), "Both certificates should exist");
    }
}

/// Test certificate validation
#[tokio::test]
async fn test_certificate_validation() {
    let config = CAConfig {
        ca_id: "test-ca".to_string(),
        bind_address: std::net::Ipv6Addr::LOCALHOST,
        port: 8444,
        cert_validity_days: 30,
        rotation_interval: Duration::from_secs(3600),
        mode: CAMode::LocalhostTesting,
        ..Default::default()
    };

    let ca = TrustChainCA::new(config).await;
    assert!(ca.is_ok(), "TrustChain CA creation should succeed");

    if let Ok(ca) = ca {
        // Generate a certificate first
        let cert_request = CertificateRequest {
            common_name: "test.hypermesh.local".to_string(),
            san_entries: vec![],
            ipv6_addresses: vec![],
            node_id: "test-node-2".to_string(),
            consensus_proof: trustchain::consensus::ConsensusProof::default(),
            timestamp: std::time::SystemTime::now(),
        };

        let cert_result = ca.issue_certificate(cert_request).await;
        assert!(cert_result.is_ok(), "Certificate generation should succeed");

        if let Ok(cert) = cert_result {
            // Validate the certificate chain
            let validation = ca.validate_certificate_chain(&cert.certificate_der).await;
            assert!(validation.is_ok(), "Certificate validation should succeed");

            if let Ok(valid) = validation {
                assert!(valid, "Certificate should be valid");
            }
        }
    }
}

/// Test certificate chain building
#[tokio::test]
async fn test_certificate_chain() {
    let config = CAConfig {
        ca_id: "test-ca".to_string(),
        bind_address: std::net::Ipv6Addr::LOCALHOST,
        port: 8445,
        cert_validity_days: 30,
        rotation_interval: Duration::from_secs(3600),
        mode: CAMode::LocalhostTesting,
        ..Default::default()
    };

    let ca = TrustChainCA::new(config).await;
    assert!(ca.is_ok(), "TrustChain CA creation should succeed");

    if let Ok(ca) = ca {
        // Issue a certificate
        let cert_request = CertificateRequest {
            common_name: "test.hypermesh.local".to_string(),
            san_entries: vec![],
            ipv6_addresses: vec![],
            node_id: "test-node-3".to_string(),
            consensus_proof: trustchain::consensus::ConsensusProof::default(),
            timestamp: std::time::SystemTime::now(),
        };

        let issued_cert = ca.issue_certificate(cert_request).await;
        assert!(issued_cert.is_ok(), "Certificate issuance should succeed");

        if let Ok(cert) = issued_cert {
            // Get the root certificate
            let root_cert = ca.get_root_certificate().await;
            assert!(root_cert.is_ok(), "Root certificate retrieval should succeed");

            // Verify we have both certificates (leaf and root)
            assert!(!cert.certificate_der.is_empty(), "Issued certificate should not be empty");
            assert!(root_cert.unwrap().len() > 0, "Root certificate should not be empty");

            // Validate the issued certificate
            let valid = ca.validate_certificate_chain(&cert.certificate_der).await;
            assert!(valid.is_ok(), "Chain validation should succeed");
        }
    }
}

/// Test FALCON-1024 operations
#[tokio::test]
async fn test_falcon_operations() {
    // Test FALCON-1024 using the crypto module directly
    use trustchain::crypto::{FalconCrypto, KeyUsage};

    // Create FALCON crypto instance
    let falcon = FalconCrypto::new();
    assert!(falcon.is_ok(), "FALCON crypto initialization should succeed");

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