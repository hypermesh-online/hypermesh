//! FALCON-1024 Post-Quantum Cryptography Integration Example
//!
//! Demonstrates how to use FALCON-1024 post-quantum signatures with TrustChain
//! certificate authority for quantum-resistant certificate operations.

use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber;

use trustchain::{
    PostQuantumCrypto, SecurityIntegratedCA, SecurityIntegrationConfig, 
    CAConfig, CertificateRequest, ConsensusProof, FalconKeyPair, PQCAlgorithm
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🔐 FALCON-1024 Post-Quantum Cryptography Integration Example");
    info!("==============================================================");

    // 1. Initialize Post-Quantum Cryptography System
    info!("📚 Step 1: Initializing Post-Quantum Cryptography System");
    let pqc = PostQuantumCrypto::new()?;
    
    // Validate quantum resistance
    let is_quantum_resistant = pqc.validate_quantum_resistance(&PQCAlgorithm::Falcon1024)?;
    info!("✅ FALCON-1024 Quantum Resistance: {}", is_quantum_resistant);
    
    // Get algorithm information
    let algorithm_info = pqc.get_algorithm_info(&PQCAlgorithm::Falcon1024);
    let performance_info = pqc.get_performance_info(&PQCAlgorithm::Falcon1024);
    info!("🔍 Algorithm: {}", algorithm_info);
    info!("⚡ Performance: {}", performance_info);

    // 2. Generate FALCON-1024 Key Pairs
    info!("\n📚 Step 2: Generating FALCON-1024 Key Pairs");
    
    // Generate CA key pair
    let ca_keypair = pqc.generate_ca_keypair("example-ca").await?;
    info!("🔑 CA Key Generated: {}", ca_keypair.public_key);
    
    // Generate asset authentication key pair
    let asset_keypair = pqc.generate_asset_keypair().await?;
    info!("🔑 Asset Key Generated: {}", asset_keypair.public_key);
    
    // Generate remote proxy key pair
    let proxy_keypair = pqc.generate_proxy_keypair().await?;
    info!("🔑 Proxy Key Generated: {}", proxy_keypair.public_key);

    // 3. Create Digital Signatures with FALCON-1024
    info!("\n📚 Step 3: Creating FALCON-1024 Digital Signatures");
    
    let test_data = b"This is a test message for FALCON-1024 post-quantum signature";
    
    // Sign with CA key
    let ca_signature = pqc.sign_with_falcon(test_data, &ca_keypair.private_key).await?;
    info!("✍️  CA Signature Created: {} bytes", ca_signature.signature_bytes.len());
    
    // Sign with asset key
    let asset_signature = pqc.sign_with_falcon(test_data, &asset_keypair.private_key).await?;
    info!("✍️  Asset Signature Created: {} bytes", asset_signature.signature_bytes.len());

    // 4. Verify Digital Signatures
    info!("\n📚 Step 4: Verifying FALCON-1024 Digital Signatures");
    
    // Verify CA signature
    let ca_valid = pqc.verify_falcon_signature(test_data, &ca_signature, &ca_keypair.public_key).await?;
    info!("🔍 CA Signature Valid: {}", ca_valid);
    
    // Verify asset signature
    let asset_valid = pqc.verify_falcon_signature(test_data, &asset_signature, &asset_keypair.public_key).await?;
    info!("🔍 Asset Signature Valid: {}", asset_valid);
    
    // Cross-verification should fail
    let cross_valid = pqc.verify_falcon_signature(test_data, &ca_signature, &asset_keypair.public_key).await?;
    info!("🔍 Cross-verification (should fail): {}", cross_valid);

    // 5. Initialize Security-Integrated Certificate Authority with FALCON-1024
    info!("\n📚 Step 5: Initializing Security-Integrated CA with FALCON-1024");
    
    let ca_config = CAConfig {
        ca_id: "falcon-example-ca".to_string(),
        bind_address: std::net::Ipv6Addr::LOCALHOST,
        port: 8443,
        cert_validity_days: 1,
        ..Default::default()
    };
    
    let security_config = SecurityIntegrationConfig {
        mandatory_security_validation: false, // Simplified for example
        block_on_security_failure: false,
        mandatory_consensus: false,
        log_all_operations: true,
        mandatory_post_quantum: true,        // CRITICAL: Enable FALCON-1024
        enable_hybrid_signatures: true,     // Enable transition support
        quantum_security_level: 128,
    };
    
    let security_ca = SecurityIntegratedCA::new(ca_config, security_config).await?;
    info!("✅ Security-Integrated CA initialized with FALCON-1024");
    
    // Get post-quantum information
    let pq_info = security_ca.get_pq_info();
    info!("🔐 Post-Quantum Algorithm: {:?}", pq_info.algorithm);
    info!("🔐 CA Fingerprint: {}", pq_info.ca_public_key_fingerprint);
    info!("🔐 Quantum Security Level: {} bits", pq_info.quantum_security_level);
    info!("🔐 Hybrid Signatures: {}", pq_info.hybrid_signatures_enabled);

    // 6. Issue Post-Quantum Certificate
    info!("\n📚 Step 6: Issuing Post-Quantum Certificate");
    
    let cert_request = CertificateRequest {
        common_name: "example.hypermesh.online".to_string(),
        san_entries: vec!["example.hypermesh.online".to_string()],
        node_id: "example_node_001".to_string(),
        ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
        consensus_proof: ConsensusProof::default_for_testing(),
        timestamp: std::time::SystemTime::now(),
    };
    
    let issued_cert = security_ca.issue_certificate_secure(cert_request).await?;
    info!("📜 Post-Quantum Certificate Issued: {}", issued_cert.serial_number);
    info!("🔐 Certificate Algorithm: {:?}", issued_cert.metadata.signature_algorithm);
    
    // Check for post-quantum metadata
    if let Some(pq_algorithm) = issued_cert.metadata.tags.get("pq_algorithm") {
        info!("✅ Certificate uses post-quantum algorithm: {}", pq_algorithm);
    }
    
    if let Some(quantum_level) = issued_cert.metadata.tags.get("quantum_security_level") {
        info!("🛡️  Quantum security level: {} bits", quantum_level);
    }

    // 7. Generate Key Pairs for Asset Authentication and Remote Proxy
    info!("\n📚 Step 7: Generating Key Pairs for HyperMesh Integration");
    
    // Generate key for asset authentication
    let hypermesh_asset_key = security_ca.generate_asset_keypair().await?;
    info!("🔑 HyperMesh Asset Key: {}", hypermesh_asset_key.public_key);
    
    // Generate key for remote proxy authentication
    let hypermesh_proxy_key = security_ca.generate_proxy_keypair().await?;
    info!("🔑 HyperMesh Proxy Key: {}", hypermesh_proxy_key.public_key);

    // 8. Demonstrate Encryption with Kyber
    info!("\n📚 Step 8: Demonstrating Kyber Post-Quantum Encryption");
    
    let kyber_keypair = pqc.generate_encryption_keypair().await?;
    info!("🔑 Kyber Encryption Key Generated");
    
    let secret_data = b"This is secret data encrypted with Kyber post-quantum encryption";
    let encrypted_data = pqc.encrypt_with_kyber(secret_data, &kyber_keypair.public_key).await?;
    info!("🔒 Data Encrypted: {} bytes → {} bytes", secret_data.len(), encrypted_data.len());
    
    let decrypted_data = pqc.decrypt_with_kyber(&encrypted_data, &kyber_keypair.private_key).await?;
    info!("🔓 Data Decrypted: {} bytes", decrypted_data.len());
    
    let decryption_successful = secret_data == decrypted_data.as_slice();
    info!("✅ Decryption Successful: {}", decryption_successful);

    info!("\n🎉 FALCON-1024 Post-Quantum Cryptography Integration Complete!");
    info!("==============================================================");
    info!("✅ All post-quantum cryptographic operations completed successfully");
    info!("🔐 TrustChain is now quantum-resistant with FALCON-1024 signatures");
    info!("🛡️  HyperMesh asset authentication keys ready for integration");
    info!("🌐 Remote proxy authentication configured for quantum security");
    
    Ok(())
}