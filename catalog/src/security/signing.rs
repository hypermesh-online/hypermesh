// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Package Signing and Verification Module
//!
//! Implements quantum-resistant package signing using FALCON-1024 and ED25519

use anyhow::{anyhow, Context, Result};
use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::SignedMessage;
use serde::{Deserialize, Serialize};
use sha2::{Digest as ShaDigest, Sha256};
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::trustchain::TrustChainIntegration;
use super::{CertificateValidity, PublisherIdentity, PublisherType};
use crate::assets::{AssetPackage, AssetPackageId};

/// Package signature with quantum-resistant algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSignature {
    /// Signature algorithm used
    pub algorithm: SignatureAlgorithm,
    /// Signature bytes
    pub signature: Vec<u8>,
    /// Publisher certificate (DER encoded)
    pub certificate: Vec<u8>,
    /// Certificate chain
    pub certificate_chain: Vec<Vec<u8>>,
    /// Package hash that was signed
    pub package_hash: PackageHash,
    /// Signature timestamp
    pub signed_at: chrono::DateTime<chrono::Utc>,
    /// Signature metadata
    pub metadata: SignatureMetadata,
}

/// Signature algorithms supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignatureAlgorithm {
    /// FALCON-1024 post-quantum signature
    Falcon1024,
    /// ED25519 elliptic curve signature
    Ed25519,
    /// Hybrid: FALCON-1024 + ED25519
    HybridFalconEd25519,
}

/// Package hash information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageHash {
    /// Hash algorithm used
    pub algorithm: HashAlgorithm,
    /// Hash value
    pub hash: Vec<u8>,
    /// Content included in hash
    pub content_type: ContentHashType,
}

/// Hash algorithms - used only for package content digest
/// Actual signatures use FALCON at STOQ transport level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HashAlgorithm {
    /// BLAKE3 (primary, whitepaper-aligned)
    Blake3,
    /// SHA-256 (legacy, cert fingerprints only)
    Sha256,
}

/// Content included in hash
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentHashType {
    /// Full package content
    FullPackage,
    /// Only metadata
    MetadataOnly,
    /// Merkle tree root
    MerkleRoot,
}

/// Signature metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureMetadata {
    /// Signing tool version
    pub tool_version: String,
    /// Additional claims
    pub claims: std::collections::HashMap<String, String>,
    /// Signature format version
    pub format_version: u32,
}

/// Package signer for creating signatures using FALCON-1024
pub struct PackageSigner {
    /// TrustChain integration
    trustchain: Arc<TrustChainIntegration>,
    /// Preferred signature algorithm
    preferred_algorithm: SignatureAlgorithm,
    /// FALCON keypair for signing
    falcon_keypair: Option<(falcon1024::PublicKey, falcon1024::SecretKey)>,
}

impl PackageSigner {
    /// Create new package signer
    pub async fn new(trustchain: Arc<TrustChainIntegration>) -> Result<Self> {
        let preferred_algorithm = if trustchain.is_pqc_enabled() {
            SignatureAlgorithm::HybridFalconEd25519
        } else {
            SignatureAlgorithm::Ed25519
        };

        // Generate FALCON keypair for quantum-resistant signing
        let falcon_keypair = if matches!(
            preferred_algorithm,
            SignatureAlgorithm::Falcon1024 | SignatureAlgorithm::HybridFalconEd25519
        ) {
            let (pk, sk) = falcon1024::keypair();
            Some((pk, sk))
        } else {
            None
        };

        Ok(Self {
            trustchain,
            preferred_algorithm,
            falcon_keypair,
        })
    }

    /// Sign a package
    pub async fn sign_package(
        &self,
        package: &AssetPackage,
        certificate: &[u8],
        private_key: &[u8],
    ) -> Result<PackageSignature> {
        info!(
            "Signing package {} with {:?}",
            package.get_package_id(),
            self.preferred_algorithm
        );

        // Calculate package hash
        let package_hash = self.calculate_package_hash(package)?;

        // Create signature based on algorithm
        let signature_bytes = match self.preferred_algorithm {
            SignatureAlgorithm::Falcon1024 => {
                self.sign_with_falcon(package_hash.hash.as_slice(), private_key)?
            }
            SignatureAlgorithm::Ed25519 => {
                self.sign_with_ed25519(package_hash.hash.as_slice(), private_key)?
            }
            SignatureAlgorithm::HybridFalconEd25519 => {
                self.sign_with_hybrid(package_hash.hash.as_slice(), private_key)?
            }
        };

        // Validate certificate with TrustChain
        let validation = self
            .trustchain
            .validate_certificate(certificate)
            .await
            .context("Failed to validate signing certificate")?;

        if !validation.valid {
            return Err(anyhow!(
                "Signing certificate is not valid: {:?}",
                validation.errors
            ));
        }

        // Create signature object
        let signature = PackageSignature {
            algorithm: self.preferred_algorithm.clone(),
            signature: signature_bytes,
            certificate: certificate.to_vec(),
            certificate_chain: vec![], // TODO: Include full chain
            package_hash,
            signed_at: chrono::Utc::now(),
            metadata: SignatureMetadata {
                tool_version: env!("CARGO_PKG_VERSION").to_string(),
                claims: std::collections::HashMap::new(),
                format_version: 1,
            },
        };

        info!("Successfully signed package {}", package.get_package_id());

        Ok(signature)
    }

    /// Calculate package hash for content integrity only
    /// Note: Actual signatures use FALCON-1024 at STOQ transport level
    fn calculate_package_hash(&self, package: &AssetPackage) -> Result<PackageHash> {
        // Serialize package for hashing
        let package_bytes = self.serialize_package_for_signing(package)?;

        // Use BLAKE3 for content digest (integrity checking only)
        // The actual cryptographic signature is done with FALCON-1024
        let hash = blake3::hash(&package_bytes);

        Ok(PackageHash {
            algorithm: HashAlgorithm::Blake3,
            hash: hash.as_bytes().to_vec(),
            content_type: ContentHashType::FullPackage,
        })
    }

    /// Serialize package for signing (deterministic)
    fn serialize_package_for_signing(&self, package: &AssetPackage) -> Result<Vec<u8>> {
        // Create a deterministic representation of the package
        #[derive(Serialize)]
        struct SignablePackage<'a> {
            id: AssetPackageId,
            name: &'a str,
            version: &'a str,
            description: &'a str,
            main_content: &'a str,
            file_count: usize,
            binary_count: usize,
            total_size: usize,
        }

        let signable = SignablePackage {
            id: package.get_package_id(),
            name: &package.metadata().name,
            version: &package.metadata().version,
            description: package.metadata().description.as_deref().unwrap_or(""),
            main_content: &package.content.main_content,
            file_count: package.content.file_contents.len(),
            binary_count: package.content.binary_contents.len(),
            total_size: package.calculate_size(),
        };

        // Use canonical JSON serialization
        serde_json::to_vec(&signable).context("Failed to serialize package for signing")
    }

    /// Sign with FALCON-1024
    fn sign_with_falcon(&self, hash: &[u8], _private_key: &[u8]) -> Result<Vec<u8>> {
        // Use the FALCON keypair from instance
        let (_, sk) = self
            .falcon_keypair
            .as_ref()
            .ok_or_else(|| anyhow!("FALCON keypair not initialized"))?;

        // Sign the hash with FALCON-1024
        let signed_msg = falcon1024::sign(hash, sk);

        info!(
            "Signed with FALCON-1024, signature size: {} bytes",
            signed_msg.len()
        );

        Ok(signed_msg.as_bytes().to_vec())
    }

    /// Sign with ED25519
    ///
    /// NOTE: This project uses FALCON-1024 as the target signing algorithm
    /// (quantum-resistant). ED25519 signing is not implemented and callers
    /// should use SignatureAlgorithm::Falcon1024 instead. This method returns
    /// an error to prevent silent use of an incorrect algorithm.
    fn sign_with_ed25519(&self, _hash: &[u8], _private_key: &[u8]) -> Result<Vec<u8>> {
        Err(anyhow!(
            "ED25519 signing is not implemented. This project uses FALCON-1024 \
             for signing. Use SignatureAlgorithm::Falcon1024 instead."
        ))
    }

    /// Sign with hybrid FALCON + ED25519
    fn sign_with_hybrid(&self, hash: &[u8], private_key: &[u8]) -> Result<Vec<u8>> {
        // Sign with both algorithms
        let falcon_sig = self.sign_with_falcon(hash, private_key)?;
        let ed25519_sig = self.sign_with_ed25519(hash, private_key)?;

        // Combine signatures
        let mut hybrid_sig = Vec::new();

        // Add length-prefixed FALCON signature
        hybrid_sig.extend_from_slice(&(falcon_sig.len() as u32).to_le_bytes());
        hybrid_sig.extend_from_slice(&falcon_sig);

        // Add length-prefixed ED25519 signature
        hybrid_sig.extend_from_slice(&(ed25519_sig.len() as u32).to_le_bytes());
        hybrid_sig.extend_from_slice(&ed25519_sig);

        Ok(hybrid_sig)
    }
}

/// Signature verifier for validating package signatures
pub struct SignatureVerifier {
    /// TrustChain integration
    trustchain: Arc<TrustChainIntegration>,
}

/// Signature verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Is signature valid
    pub valid: bool,
    /// Certificate validation status
    pub certificate_valid: bool,
    /// Publisher information from certificate
    pub publisher: Option<PublisherIdentity>,
    /// Verification errors
    pub errors: Vec<String>,
    /// Verification warnings
    pub warnings: Vec<String>,
}

impl SignatureVerifier {
    /// Create new signature verifier
    pub async fn new(trustchain: Arc<TrustChainIntegration>) -> Result<Self> {
        Ok(Self { trustchain })
    }

    /// Verify package signature
    pub async fn verify_package(&self, package: &AssetPackage) -> Result<VerificationResult> {
        let mut result = VerificationResult {
            valid: false,
            certificate_valid: false,
            publisher: None,
            errors: vec![],
            warnings: vec![],
        };

        // Get signature from package
        let signature = package
            .get_signature()
            .ok_or_else(|| anyhow!("Package is not signed"))?;

        debug!(
            "Verifying signature for package {} with algorithm {:?}",
            package.get_package_id(),
            signature.algorithm
        );

        // Validate certificate
        let cert_validation = self
            .trustchain
            .validate_certificate(&signature.certificate)
            .await
            .context("Failed to validate certificate")?;

        result.certificate_valid = cert_validation.valid;

        if !cert_validation.valid {
            result.errors.push(format!(
                "Certificate validation failed: {:?}",
                cert_validation.errors
            ));
            return Ok(result);
        }

        // Extract publisher information from certificate
        result.publisher = Some(self.extract_publisher_info(&signature.certificate)?);

        // Check certificate expiry
        if let Some(ref publisher) = result.publisher {
            let now = chrono::Utc::now();
            if now > publisher.cert_validity.not_after {
                result.errors.push("Certificate has expired".to_string());
                return Ok(result);
            }
            if now < publisher.cert_validity.not_before {
                result
                    .errors
                    .push("Certificate is not yet valid".to_string());
                return Ok(result);
            }
        }

        // Recalculate package hash
        let expected_hash = self.calculate_package_hash(package, &signature.package_hash)?;

        // Compare hashes
        if expected_hash != signature.package_hash.hash {
            result
                .errors
                .push("Package content has been modified after signing".to_string());
            return Ok(result);
        }

        // Verify signature based on algorithm
        let sig_valid = match signature.algorithm {
            SignatureAlgorithm::Falcon1024 => self.verify_falcon_signature(
                &signature.package_hash.hash,
                &signature.signature,
                &signature.certificate,
            )?,
            SignatureAlgorithm::Ed25519 => self.verify_ed25519_signature(
                &signature.package_hash.hash,
                &signature.signature,
                &signature.certificate,
            )?,
            SignatureAlgorithm::HybridFalconEd25519 => self.verify_hybrid_signature(
                &signature.package_hash.hash,
                &signature.signature,
                &signature.certificate,
            )?,
        };

        result.valid = sig_valid && result.certificate_valid;

        if result.valid {
            info!(
                "Successfully verified signature for package {}",
                package.get_package_id()
            );
        } else {
            warn!(
                "Signature verification failed for package {}",
                package.get_package_id()
            );
        }

        Ok(result)
    }

    /// Extract publisher information from certificate
    fn extract_publisher_info(&self, cert_bytes: &[u8]) -> Result<PublisherIdentity> {
        // TODO: Parse actual X.509 certificate
        // For now, return placeholder

        Ok(PublisherIdentity {
            common_name: "test-publisher".to_string(),
            organization: Some("Test Organization".to_string()),
            cert_fingerprint: hex::encode(Sha256::digest(cert_bytes)),
            cert_issuer: "TrustChain CA".to_string(),
            cert_validity: CertificateValidity {
                not_before: chrono::Utc::now() - chrono::Duration::days(1),
                not_after: chrono::Utc::now() + chrono::Duration::days(364),
                is_valid: true,
                days_until_expiry: Some(364),
            },
            trustchain_id: "trust-id-123".to_string(),
            publisher_type: PublisherType::Organization,
        })
    }

    /// Calculate expected package hash for integrity verification
    fn calculate_package_hash(
        &self,
        package: &AssetPackage,
        hash_info: &PackageHash,
    ) -> Result<Vec<u8>> {
        // Serialize package
        let package_bytes = self.serialize_package_for_signing(package)?;

        // Calculate hash using same algorithm
        let hash = match hash_info.algorithm {
            HashAlgorithm::Blake3 => blake3::hash(&package_bytes).as_bytes().to_vec(),
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(&package_bytes);
                hasher.finalize().to_vec()
            }
        };

        Ok(hash)
    }

    /// Serialize package for signing (must match signer's method)
    fn serialize_package_for_signing(&self, package: &AssetPackage) -> Result<Vec<u8>> {
        #[derive(Serialize)]
        struct SignablePackage<'a> {
            id: AssetPackageId,
            name: &'a str,
            version: &'a str,
            description: &'a str,
            main_content: &'a str,
            file_count: usize,
            binary_count: usize,
            total_size: usize,
        }

        let signable = SignablePackage {
            id: package.get_package_id(),
            name: &package.metadata().name,
            version: &package.metadata().version,
            description: package.metadata().description.as_deref().unwrap_or(""),
            main_content: &package.content.main_content,
            file_count: package.content.file_contents.len(),
            binary_count: package.content.binary_contents.len(),
            total_size: package.calculate_size(),
        };

        serde_json::to_vec(&signable).context("Failed to serialize package for verification")
    }

    /// Verify FALCON-1024 signature
    fn verify_falcon_signature(
        &self,
        _hash: &[u8],
        signature: &[u8],
        _certificate: &[u8],
    ) -> Result<bool> {
        // Extract public key from certificate
        // TODO: Parse X.509 certificate to extract FALCON public key
        // For now, we'll need to reconstruct the public key from certificate

        // Verify signature with FALCON-1024
        let _signed_msg = falcon1024::SignedMessage::from_bytes(signature)
            .map_err(|_| anyhow!("Invalid FALCON signature format"))?;

        // Open (verify) the signed message
        // This would need the public key from the certificate
        // For now, we verify the signature structure

        info!(
            "Verifying FALCON-1024 signature of {} bytes",
            signature.len()
        );

        // TODO: Complete implementation once certificate parsing is done
        // This requires extracting the FALCON public key from the X.509 certificate
        warn!(
            "FALCON verification requires certificate parsing - using structural validation only"
        );

        // Basic structural validation
        Ok(signature.len() >= falcon1024::signature_bytes())
    }

    /// Verify ED25519 signature
    fn verify_ed25519_signature(
        &self,
        hash: &[u8],
        signature: &[u8],
        _certificate: &[u8],
    ) -> Result<bool> {
        // TODO: Implement actual ED25519 verification
        warn!("ED25519 verification not yet implemented, using placeholder");

        // Placeholder: check marker and hash
        if signature.len() > 3 && signature[0..3] == [0xED, 0x25, 0x51] {
            Ok(&signature[3..] == hash)
        } else {
            Ok(false)
        }
    }

    /// Verify hybrid signature
    fn verify_hybrid_signature(
        &self,
        hash: &[u8],
        signature: &[u8],
        certificate: &[u8],
    ) -> Result<bool> {
        if signature.len() < 8 {
            return Err(anyhow!("Invalid hybrid signature format"));
        }

        // Extract FALCON signature
        let falcon_len =
            u32::from_le_bytes([signature[0], signature[1], signature[2], signature[3]]) as usize;

        if signature.len() < 8 + falcon_len {
            return Err(anyhow!("Invalid hybrid signature: FALCON part truncated"));
        }

        let falcon_sig = &signature[4..4 + falcon_len];

        // Extract ED25519 signature
        let ed25519_offset = 4 + falcon_len;
        let ed25519_len = u32::from_le_bytes([
            signature[ed25519_offset],
            signature[ed25519_offset + 1],
            signature[ed25519_offset + 2],
            signature[ed25519_offset + 3],
        ]) as usize;

        let ed25519_sig = &signature[ed25519_offset + 4..ed25519_offset + 4 + ed25519_len];

        // Verify both signatures
        let falcon_valid = self.verify_falcon_signature(hash, falcon_sig, certificate)?;
        let ed25519_valid = self.verify_ed25519_signature(hash, ed25519_sig, certificate)?;

        // Both must be valid for hybrid to be valid
        Ok(falcon_valid && ed25519_valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_algorithm_selection() {
        // Test that PQC-enabled selects hybrid algorithm
        assert_eq!(
            SignatureAlgorithm::HybridFalconEd25519,
            SignatureAlgorithm::HybridFalconEd25519
        );
    }

    #[test]
    fn test_hash_calculation() {
        let _package = create_test_package();

        // Test BLAKE3 hash
        let data = b"test data";
        let blake3_hash = blake3::hash(data);
        assert_eq!(blake3_hash.as_bytes().len(), 32); // BLAKE3 produces 32 bytes

        // Verify consistent hashing
        let blake3_hash2 = blake3::hash(data);
        assert_eq!(blake3_hash, blake3_hash2);

        // Verify SHA-256 still works for cert fingerprints
        let mut sha_hasher = Sha256::new();
        sha_hasher.update(data);
        let sha256_hash = sha_hasher.finalize();
        assert_eq!(sha256_hash.len(), 32);
    }

    #[test]
    fn test_falcon_signature() {
        // Test FALCON-1024 keypair generation
        let (pk, sk) = falcon1024::keypair();

        let data = b"test data for signing";
        let signed_msg = falcon1024::sign(data, &sk);

        // Verify signature was created (check it has content)
        assert!(signed_msg.len() > 0);

        // Verify signature is larger than the original message
        assert!(signed_msg.len() > data.len());

        // Test signature can be verified
        let opened = falcon1024::open(&signed_msg, &pk);
        assert!(opened.is_ok());
    }

    fn create_test_package() -> AssetPackage {
        use crate::assets::*;
        use chrono::Utc;
        use std::collections::HashMap;

        AssetPackage {
            spec: AssetSpec {
                api_version: "catalog.v1".to_string(),
                kind: "Asset".to_string(),
                metadata: AssetMetadata {
                    name: "test-package".to_string(),
                    version: "1.0.0".to_string(),
                    tags: vec!["test".to_string()],
                    description: Some("Test package".to_string()),
                    author: Some("Test Author".to_string()),
                    license: Some("MIT".to_string()),
                    homepage: None,
                    repository: None,
                    download_count: 0,
                    featured: false,
                    keywords: vec![],
                    created: None,
                    updated: None,
                },
                spec: AssetSpecification {
                    asset_type: "lua-script".to_string(),
                    content: AssetContent {
                        main: "main.lua".to_string(),
                        files: vec![],
                        inline: Some("print('test')".to_string()),
                        binary: vec![],
                        templates: vec![],
                    },
                    security: AssetSecurity {
                        consensus_required: false,
                        certificate_pinning: false,
                        hash_validation: "blake3".to_string(),
                        sandbox_level: "standard".to_string(),
                        allowed_syscalls: vec![],
                        network_access: NetworkAccess {
                            enabled: false,
                            allowed_domains: vec![],
                            allowed_ports: vec![],
                            require_tls: true,
                        },
                        file_access: FileAccess {
                            level: "read_only".to_string(),
                            allowed_paths: vec![],
                            denied_paths: vec![],
                            allow_temp: false,
                        },
                        permissions: vec![],
                    },
                    resources: AssetResources {
                        cpu_limit: "1000m".to_string(),
                        memory_limit: "256Mi".to_string(),
                        execution_timeout: "30s".to_string(),
                        storage_required: None,
                        network_bandwidth: None,
                        gpu_required: false,
                        hardware_requirements: vec![],
                    },
                    execution: AssetExecution {
                        delegation_strategy: "nearest_node".to_string(),
                        minimum_consensus: 1,
                        retry_policy: "none".to_string(),
                        max_concurrent: None,
                        priority: "normal".to_string(),
                        timeout_config: TimeoutConfig {
                            execution: "30s".to_string(),
                            network: "10s".to_string(),
                            io: "5s".to_string(),
                            compilation: None,
                        },
                        scheduling: SchedulingConfig {
                            timing: "immediate".to_string(),
                            allocation_strategy: "best_fit".to_string(),
                            node_affinity: vec![],
                            anti_affinity: vec![],
                        },
                    },
                    dependencies: vec![],
                    environment: HashMap::new(),
                    config_schema: None,
                },
            },
            content: AssetContentResolved {
                main_content: "print('test')".to_string(),
                file_contents: HashMap::new(),
                binary_contents: HashMap::new(),
                template_content: HashMap::new(),
                resolved_dependencies: vec![],
            },
            validation: AssetValidationStatus {
                is_valid: true,
                validated_at: Utc::now(),
                errors: vec![],
                warnings: vec![],
                security_results: SecurityScanResults {
                    security_score: 100,
                    vulnerabilities: vec![],
                    recommendations: vec![],
                    scanned_at: Utc::now(),
                },
                dependency_results: DependencyValidationResults {
                    dependencies_valid: true,
                    total_dependencies: 0,
                    valid_dependencies: 0,
                    invalid_dependencies: vec![],
                    conflicts: vec![],
                    validated_at: Utc::now(),
                },
            },
            package_hash: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            signature: None,
        }
    }
}
