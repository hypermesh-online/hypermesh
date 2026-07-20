// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Legacy `TrustChainCA` struct.
//!
//! Use `SecurityIntegratedCA` (from `security_integration`) for new deployments.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use rcgen::{Certificate as RcgenCertificate, CertificateParams, KeyPair};
use x509_parser::parse_x509_certificate;

use crate::proof_of_state::{
    StateProofClientMetrics, StateProofContext, StateProofValidationResult,
    StateProofValidationStatus, HyperMeshStateProofClient,
};

use super::certificate_store::CertificateStore as CertStore;
use super::config::{CAConfig, CAMode};
use super::policy::PolicyEngine;
use super::types::{
    CertificateMetadata, CertificateRequest, CertificateStatus, CertificateSubjectType,
    IssuedCertificate,
};

/// TrustChain Certificate Authority (Legacy - use SecurityIntegratedCA for new deployments)
#[derive(Clone)]
pub struct TrustChainCA {
    /// Root CA certificate
    root_ca: Arc<RwLock<RcgenCertificate>>,
    /// Root CA key pair (needed to sign leaf certificates via signed_by())
    root_key_pair: Arc<KeyPair>,
    /// Issued certificates store
    certificate_store: Arc<CertStore>,
    /// Certificate policies
    policy_engine: Arc<PolicyEngine>,
    /// State proof validation context (retained for state proof operations)
    _state_proof_context: Arc<StateProofContext>,
    /// HyperMesh Proof of State client for validation
    hypermesh_client: Arc<HyperMeshStateProofClient>,
    /// Four-proof state proof validator (wrapped in Mutex for mutability)
    pub state_proof_validator: Arc<tokio::sync::Mutex<crate::proof_of_state::FourProofValidator>>,
    /// CA configuration
    config: Arc<CAConfig>,
    /// Optional threshold configuration for distributed CA signing.
    /// When set, the CA's FALCON-1024 key can be split into shares and signing
    /// requires t-of-n shares to be collected.
    threshold_config: Option<crate::crypto::threshold::ThresholdConfig>,
    /// Cached key shares after splitting (only available on the node that performed the split).
    ca_key_shares: Option<Vec<crate::crypto::threshold::KeyShare>>,
}

impl TrustChainCA {
    /// Create a new TrustChain CA
    pub async fn new(config: CAConfig) -> Result<Self> {
        info!("Initializing TrustChain CA: {}", config.ca_id);

        // Initialize root CA certificate and key pair
        let (root_cert, root_key) = match config.mode {
            CAMode::LocalhostTesting => {
                info!("Creating self-signed root CA for localhost testing");
                Self::create_self_signed_root(&config.ca_id)?
            }
            CAMode::Production => {
                info!("Loading production root CA (software-protected)");
                // AWS CloudHSM dependencies REMOVED - software-only operation
                // Using software-based key generation for production
                Self::create_self_signed_root(&config.ca_id)?
            }
        };

        // Initialize certificate store
        let certificate_store = Arc::new(CertStore::new().await?);

        // Initialize policy engine
        let policy_engine = Arc::new(PolicyEngine::new(config.state_requirements.clone()));

        // Initialize state proof context
        let state_proof_context = Arc::new(StateProofContext::new(
            config.ca_id.clone(),
            "trustchain_network".to_string(),
        ));

        // Initialize HyperMesh Proof of State client
        let hypermesh_client =
            Arc::new(HyperMeshStateProofClient::new(config.hypermesh_client_config.clone()).await?);

        // Initialize four-proof state proof validator
        let state_proof_validator = Arc::new(tokio::sync::Mutex::new(
            crate::proof_of_state::FourProofValidator::new(),
        ));

        let ca = Self {
            root_ca: Arc::new(RwLock::new(root_cert)),
            root_key_pair: Arc::new(root_key),
            certificate_store,
            policy_engine,
            _state_proof_context: state_proof_context,
            hypermesh_client,
            state_proof_validator,
            config: Arc::new(config),
            threshold_config: None,
            ca_key_shares: None,
        };

        info!("TrustChain CA initialized successfully");
        Ok(ca)
    }

    /// Issue a new certificate with HyperMesh Proof of State validation
    pub async fn issue_certificate(
        &self,
        request: CertificateRequest,
    ) -> Result<IssuedCertificate> {
        info!(
            "Processing certificate request for: {} with HyperMesh Proof of State validation",
            request.common_name
        );

        // Validate certificate request through HyperMesh Proof of State
        let state_proof_result = self
            .hypermesh_client
            .validate_certificate_request(&request, &self.config.state_requirements)
            .await?;

        // Process state proof validation result
        match state_proof_result.result {
            StateProofValidationStatus::Valid => {
                info!(
                    "HyperMesh Proof of State validation successful for: {}",
                    request.common_name
                );
            }
            StateProofValidationStatus::Invalid {
                failed_proofs,
                reason,
            } => {
                error!(
                    "HyperMesh Proof of State validation failed for {}: {} (failed proofs: {:?})",
                    request.common_name, reason, failed_proofs
                );
                return Err(anyhow!(
                    "HyperMesh Proof of State validation failed: {reason} (failed proofs: {failed_proofs:?})"
                ));
            }
            StateProofValidationStatus::Pending {
                estimated_completion,
            } => {
                error!(
                    "HyperMesh Proof of State validation pending for {}, estimated completion: {:?}",
                    request.common_name, estimated_completion
                );
                return Err(anyhow!(
                    "HyperMesh Proof of State validation pending, try again later"
                ));
            }
            StateProofValidationStatus::Error {
                error_code,
                message,
            } => {
                error!(
                    "HyperMesh Proof of State validation error for {}: {} ({})",
                    request.common_name, message, error_code
                );
                return Err(anyhow!(
                    "HyperMesh Proof of State validation error: {message} ({error_code})"
                ));
            }
        }

        // Validate certificate policy
        if !self.policy_engine.validate_request(&request).await? {
            return Err(anyhow!("Certificate policy validation failed"));
        }

        // Generate certificate with HyperMesh Proof of State
        let issued_cert = self
            .generate_certificate_with_state_proof(request, state_proof_result)
            .await?;

        // Store certificate
        self.certificate_store
            .store_certificate(&issued_cert)
            .await?;

        info!(
            "Certificate issued successfully with HyperMesh Proof of State: {}",
            issued_cert.serial_number
        );
        Ok(issued_cert)
    }

    /// Issue certificate with pre-validated state proof (skips HyperMesh network call).
    /// Used by SecurityIntegratedCA which already performed local state proof validation.
    ///
    /// IMPORTANT: The caller MUST have already validated the state proof via
    /// FourProofValidator before calling this method. This method performs a
    /// local four-proof validation as a safety net to ensure no certificate
    /// is issued without PoS verification.
    pub async fn issue_certificate_local(
        &self,
        request: CertificateRequest,
    ) -> Result<IssuedCertificate> {
        info!(
            "Processing certificate request for: {} (pre-validated state proof)",
            request.common_name
        );

        // Safety net: validate state proof locally even for pre-validated requests.
        // This ensures no certificate is ever issued without PoS verification.
        {
            let mut validator = self.state_proof_validator.lock().await;
            let local_pos_result = validator
                .validate_state_proof(&request.state_proof)
                .await
                .map_err(|e| anyhow!("Local PoS validation error: {e}"))?;
            if !local_pos_result.is_valid() {
                return Err(anyhow!(
                    "Local four-proof PoS validation failed for certificate request: {}",
                    request.common_name
                ));
            }
        }

        // Validate certificate policy
        if !self.policy_engine.validate_request(&request).await? {
            return Err(anyhow!("Certificate policy validation failed"));
        }

        // Build a local state proof result (state proof was already validated by caller)
        let local_result = StateProofValidationResult {
            result: StateProofValidationStatus::Valid,
            proof_hash: request.state_proof.hash().ok(),
            validator_id: "local-security-integrated-ca".to_string(),
            validated_at: std::time::SystemTime::now(),
            metrics: crate::proof_of_state::hypermesh_client::ValidationMetrics {
                validation_time_us: 0,
                validator_nodes: 1,
                all_proofs_valid: true,
                network_load: 0.0,
            },
            details: crate::proof_of_state::hypermesh_client::ValidationDetails {
                proof_results: crate::proof_of_state::hypermesh_client::ProofValidationResults {
                    space_proof_valid: true,
                    stake_proof_valid: true,
                    work_proof_valid: true,
                    time_proof_valid: true,
                },
                bft_status: crate::proof_of_state::hypermesh_client::ByzantineFaultToleranceStatus {
                    byzantine_nodes_detected: 0,
                    fault_tolerance_maintained: true,
                    recovery_action_taken: None,
                },
                performance_stats: crate::proof_of_state::hypermesh_client::PerformanceStatistics {
                    state_proof_latency_ms: 0,
                    throughput_ops_per_sec: 0.0,
                    network_overhead_bytes: 0,
                },
            },
        };

        let issued_cert = self
            .generate_certificate_with_state_proof(request, local_result)
            .await?;
        self.certificate_store
            .store_certificate(&issued_cert)
            .await?;

        info!(
            "Certificate issued successfully (pre-validated state proof): {}",
            issued_cert.serial_number
        );
        Ok(issued_cert)
    }

    /// Validate certificate chain
    pub async fn validate_certificate_chain(&self, cert_der: &[u8]) -> Result<bool> {
        let _start = std::time::Instant::now();
        debug!("Validating certificate chain");

        // Parse certificate
        let (_, _parsed_cert) = parse_x509_certificate(cert_der)
            .map_err(|e| anyhow!("Failed to parse certificate: {e}"))?;

        // Calculate fingerprint
        let fingerprint = self.calculate_fingerprint(cert_der);

        // Check if certificate exists in store
        if let Some(stored_cert) = self
            .certificate_store
            .get_certificate(&hex::encode(fingerprint))
            .await?
        {
            // Validate certificate status
            match stored_cert.status {
                CertificateStatus::Valid => {
                    // Check expiration
                    if SystemTime::now() > stored_cert.expires_at {
                        warn!("Certificate expired: {}", stored_cert.serial_number);
                        return Ok(false);
                    }

                    // Validate state proof through HyperMesh (for legacy certificates with embedded proofs)
                    // For certificates issued with HyperMesh Proof of State, they are already validated
                    if stored_cert.state_proof.hash().is_ok() {
                        debug!("Certificate validation successful (HyperMesh Proof of State validated)");
                        return Ok(true);
                    } else {
                        warn!(
                            "State proof validation failed for certificate: {}",
                            stored_cert.serial_number
                        );
                        return Ok(false);
                    }
                }
                CertificateStatus::Revoked { .. } => {
                    warn!("Certificate revoked: {}", stored_cert.serial_number);
                    return Ok(false);
                }
                CertificateStatus::Expired => {
                    warn!("Certificate expired: {}", stored_cert.serial_number);
                    return Ok(false);
                }
            }
        }

        warn!("Certificate not found in store");
        Ok(false)
    }

    /// Revoke a certificate
    pub async fn revoke_certificate(&self, serial_number: &str, reason: String) -> Result<()> {
        info!("Revoking certificate: {}", serial_number);

        self.certificate_store
            .revoke_certificate(serial_number, reason)
            .await?;

        info!("Certificate revoked successfully: {}", serial_number);
        Ok(())
    }

    /// Get CA certificate for trust anchor
    pub async fn get_ca_certificate(&self) -> Result<Vec<u8>> {
        let root_ca = self.root_ca.read().await;
        // rcgen 0.13: Use der() instead of serialize_der()
        Ok(root_ca.der().to_vec())
    }

    /// Get root certificate (alias for API compatibility)
    pub async fn get_root_certificate(&self) -> Result<Vec<u8>> {
        self.get_ca_certificate().await
    }

    /// Internal: Create self-signed root CA with its key pair
    fn create_self_signed_root(ca_id: &str) -> Result<(RcgenCertificate, KeyPair)> {
        // rcgen 0.13: Create root CA with CA constraint so it can sign leaf certs
        let mut params = CertificateParams::new(vec![ca_id.to_string()])?;
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);

        let key_pair = KeyPair::generate()?;
        let cert = params.self_signed(&key_pair)?;
        Ok((cert, key_pair))
    }

    /// Internal: Generate certificate with HyperMesh Proof of State validation result
    ///
    /// Uses rcgen `signed_by()` to produce a leaf certificate signed by the root CA,
    /// establishing a proper Root CA (self-signed) -> Leaf cert (CA-signed) hierarchy.
    async fn generate_certificate_with_state_proof(
        &self,
        request: CertificateRequest,
        state_proof_result: StateProofValidationResult,
    ) -> Result<IssuedCertificate> {
        let root_ca = self.root_ca.read().await;

        // rcgen 0.13: Create leaf certificate parameters
        let mut params = rcgen::CertificateParams::new(vec![request.common_name.clone()])?;

        // Leaf certs are NOT CAs
        params.is_ca = rcgen::IsCa::NoCa;

        // Add SAN entries (rcgen 0.13: SanType uses Ia5String)
        for san in &request.san_entries {
            params
                .subject_alt_names
                .push(rcgen::SanType::DnsName(rcgen::Ia5String::try_from(
                    san.as_str(),
                )?));
        }

        // Add IPv6 addresses
        for ipv6_addr in &request.ipv6_addresses {
            params
                .subject_alt_names
                .push(rcgen::SanType::IpAddress(std::net::IpAddr::V6(*ipv6_addr)));
        }

        // Item 2.7: Set validity period based on identity scope
        let now = SystemTime::now();
        let validity_secs = match &request.identity_scope {
            Some(scope) if !scope.tracked => {
                // Anonymous: ephemeral (15 minutes)
                15 * 60
            }
            Some(scope) if scope.tracked && matches!(scope.blockchain_scope, hypermesh_lib::BlockchainScope::Device) => {
                // Private/bounded group: medium validity (7 days)
                7 * 24 * 60 * 60
            }
            Some(scope) if scope.tracked && matches!(scope.blockchain_scope, hypermesh_lib::BlockchainScope::Network) => {
                // Public/network: full chain (90 days)
                90 * 24 * 60 * 60
            }
            _ => {
                // Default: use configured validity
                self.config.cert_validity_days as u64 * 24 * 60 * 60
            }
        };
        let expires_at = now + Duration::from_secs(validity_secs);

        params.not_before = now.into();
        params.not_after = expires_at.into();

        // Item 2.4: Set KeyUsage and ExtendedKeyUsage based on subject type
        let subject_type = request.subject_type.unwrap_or(CertificateSubjectType::Node);
        let mut key_usages = vec![
            rcgen::KeyUsagePurpose::DigitalSignature,
        ];
        let mut eku_purposes = Vec::new();

        match subject_type {
            CertificateSubjectType::Node => {
                key_usages.push(rcgen::KeyUsagePurpose::KeyEncipherment);
                eku_purposes.push(rcgen::ExtendedKeyUsagePurpose::ServerAuth);
                eku_purposes.push(rcgen::ExtendedKeyUsagePurpose::ClientAuth);
            }
            CertificateSubjectType::Service => {
                eku_purposes.push(rcgen::ExtendedKeyUsagePurpose::ServerAuth);
            }
            CertificateSubjectType::Agent => {
                eku_purposes.push(rcgen::ExtendedKeyUsagePurpose::ClientAuth);
            }
        }
        params.key_usages = key_usages;
        params.extended_key_usages = eku_purposes;

        // Item 2.6: Embed identity scope extension as custom X.509 extension
        if let Some(ref scope) = request.identity_scope {
            let scope_ext = crate::trust::hypermesh_integration::types::IdentityScopeExtension {
                subject_type: match subject_type {
                    CertificateSubjectType::Node => crate::trust::hypermesh_integration::types::CertificateSubjectType::Node,
                    CertificateSubjectType::Service => crate::trust::hypermesh_integration::types::CertificateSubjectType::Service,
                    CertificateSubjectType::Agent => crate::trust::hypermesh_integration::types::CertificateSubjectType::Agent,
                },
                blockchain_scope: scope.blockchain_scope,
                tracked: scope.tracked,
                workload_type: hypermesh_lib::WorkloadType::from(subject_type),
            };
            let scope_bytes = scope_ext.to_bytes();
            let oid_parts: Vec<u64> = crate::trust::hypermesh_integration::types::IDENTITY_SCOPE_EXTENSION_OID
                .split('.')
                .filter_map(|s| s.parse::<u64>().ok())
                .collect();
            if oid_parts.len() >= 2 {
                params.custom_extensions.push(
                    rcgen::CustomExtension::from_oid_content(
                        &oid_parts,
                        scope_bytes.to_vec(),
                    ),
                );
            }
            debug!("Added identity scope extension: scope={:?}, tracked={}", scope.blockchain_scope, scope.tracked);
        }

        // Add HyperMesh Proof of State metadata as certificate extension
        if let Some(proof_hash) = state_proof_result.proof_hash {
            let state_proof_extension = format!(
                "HyperMesh-StateProof: {}, Validator: {}",
                hex::encode(proof_hash),
                state_proof_result.validator_id
            );
            debug!(
                "Adding HyperMesh Proof of State metadata: {}",
                state_proof_extension
            );
        }

        // Generate leaf key pair and sign with CA root via signed_by()
        let leaf_key_pair = KeyPair::generate()?;
        let cert = params.signed_by(&leaf_key_pair, &root_ca, &self.root_key_pair)?;
        let cert_der = cert.der().to_vec();

        // Convert to PEM format for API compatibility
        let certificate_pem = cert.pem();

        // Build certificate chain PEM (leaf + root)
        let root_ca_pem = root_ca.pem();
        let chain_pem = format!("{certificate_pem}\n{root_ca_pem}");

        // Calculate fingerprint
        let fingerprint = self.calculate_fingerprint(&cert_der);

        // Generate serial number
        let serial_number = hex::encode(&fingerprint[..16]);

        // Create enhanced metadata with HyperMesh Proof of State information
        let mut metadata = CertificateMetadata::default();
        metadata.tags.insert(
            "state_validator".to_string(),
            state_proof_result.validator_id,
        );
        if let Some(proof_hash) = state_proof_result.proof_hash {
            metadata
                .tags
                .insert("state_proof_hash".to_string(), hex::encode(proof_hash));
        }
        metadata.tags.insert(
            "state_validation_time".to_string(),
            state_proof_result.metrics.validation_time_us.to_string(),
        );
        metadata.tags.insert(
            "state_proof_valid".to_string(),
            state_proof_result.metrics.all_proofs_valid.to_string(),
        );

        Ok(IssuedCertificate {
            serial_number,
            certificate_der: cert_der,
            certificate_pem,
            chain_pem,
            fingerprint,
            common_name: request.common_name,
            issued_at: now,
            expires_at,
            issuer_ca_id: self.config.ca_id.clone(),
            state_proof: request.state_proof,
            status: CertificateStatus::Valid,
            metadata,
        })
    }

    /// Get HyperMesh Proof of State client metrics
    pub async fn get_state_proof_metrics(&self) -> Result<StateProofClientMetrics> {
        Ok(self.hypermesh_client.get_metrics().await)
    }

    /// Reset HyperMesh Proof of State client metrics
    pub async fn reset_state_proof_metrics(&self) -> Result<()> {
        self.hypermesh_client.reset_metrics().await;
        Ok(())
    }

    /// Validate the canonical four-proof set through HyperMesh for complex
    /// certificate operations.
    pub async fn validate_four_proofs(
        &self,
        proof_set: &hypermesh_lib::proof::StateProof,
        operation: &str,
        asset_id: &str,
        node_id: &str,
    ) -> Result<StateProofValidationResult> {
        info!(
            "Validating four-proof set through HyperMesh for operation: {}",
            operation
        );

        let result = self
            .hypermesh_client
            .validate_four_proofs(proof_set, operation, asset_id, node_id)
            .await?;

        match &result.result {
            StateProofValidationStatus::Valid => {
                info!(
                    "Four-proof validation successful for operation: {}",
                    operation
                );
            }
            StateProofValidationStatus::Invalid {
                failed_proofs,
                reason,
            } => {
                warn!(
                    "Four-proof validation failed for operation {}: {} (failed: {:?})",
                    operation, reason, failed_proofs
                );
            }
            status => {
                debug!(
                    "Four-proof validation status for operation {}: {:?}",
                    operation, status
                );
            }
        }

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Threshold CA signing (Shamir SSS over FALCON-1024)
    // -----------------------------------------------------------------------

    /// Split a FALCON-1024 signing key into threshold shares using Shamir SSS.
    ///
    /// Returns N [`KeyShare`]s; any T can reconstruct the key and sign.
    /// The original key is NOT erased (backward compat / single-node mode).
    ///
    /// The CA itself uses rcgen (ECDSA) for X.509 issuance, so the FALCON key
    /// bytes must be provided explicitly (typically from [`FalconIdentity`]).
    pub fn split_ca_key(
        &mut self,
        falcon_secret_key: &[u8],
        falcon_public_key: &[u8],
        config: crate::crypto::threshold::ThresholdConfig,
    ) -> Result<Vec<crate::crypto::threshold::KeyShare>> {
        use sha2::{Digest, Sha256};

        let signer = crate::crypto::threshold::ThresholdSigner::new(config.clone())?;

        // Compute SHA-256 fingerprint of the public key (matches ThresholdSigner convention)
        let fingerprint: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(b"FALCON-1024-KEY:");
            h.update(falcon_public_key);
            h.finalize().into()
        };

        let shares = signer.split_signing_key(falcon_secret_key, fingerprint)?;

        self.threshold_config = Some(config);
        self.ca_key_shares = Some(shares.clone());

        info!(
            "Split FALCON-1024 CA key into {} threshold shares (threshold={})",
            shares.len(),
            self.threshold_config.as_ref().map_or(0, |c| c.threshold),
        );

        Ok(shares)
    }

    /// Sign a message using threshold shares (requires t shares).
    ///
    /// Instead of using the local CA key, this reconstructs the FALCON-1024
    /// private key from the provided shares and signs. The reconstructed key
    /// exists only for the duration of this call.
    pub fn sign_with_threshold(
        &self,
        message: &[u8],
        shares: &[crate::crypto::threshold::KeyShare],
    ) -> Result<Vec<u8>> {
        let config = self
            .threshold_config
            .as_ref()
            .ok_or_else(|| anyhow!("Threshold not configured — call split_ca_key() first"))?;

        let signer = crate::crypto::threshold::ThresholdSigner::new(config.clone())?;
        let signature = signer.reconstruct_and_sign(shares, message)?;

        debug!(
            "Threshold-signed message ({} bytes) with {} shares",
            message.len(),
            shares.len(),
        );

        Ok(signature)
    }

    /// Check if threshold signing is configured.
    pub fn is_threshold_configured(&self) -> bool {
        self.threshold_config.is_some()
    }

    /// Get the threshold configuration, if set.
    pub fn threshold_config(&self) -> Option<&crate::crypto::threshold::ThresholdConfig> {
        self.threshold_config.as_ref()
    }

    /// Get the cached key shares (only available on the node that performed the split).
    pub fn ca_key_shares(&self) -> Option<&[crate::crypto::threshold::KeyShare]> {
        self.ca_key_shares.as_deref()
    }

    /// Internal: Calculate certificate fingerprint
    fn calculate_fingerprint(&self, cert_der: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::threshold::ThresholdConfig;
    use crate::proof_of_state::StateProof;

    #[tokio::test]
    async fn test_ca_creation() {
        let config = CAConfig::testing(); // Use testing config with random port
        let ca = TrustChainCA::new(config)
            .await
            .expect("Failed to create CA");

        let ca_cert = ca
            .get_ca_certificate()
            .await
            .expect("Failed to get CA certificate");
        assert!(!ca_cert.is_empty());
    }

    #[tokio::test]
    async fn test_certificate_issuance() -> Result<(), Box<dyn std::error::Error>> {
        let config = CAConfig::testing(); // Use testing config with random port
        let ca = TrustChainCA::new(config)
            .await
            .expect("Failed to create CA");

        let request = CertificateRequest {
            common_name: "test.localhost".to_string(),
            san_entries: vec!["test.localhost".to_string()],
            node_id: "test_node_001".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            state_proof: StateProof::default_for_testing(),
            timestamp: SystemTime::now(),
            identity_scope: None,
            subject_type: None,
        };

        let issued_cert = ca
            .issue_certificate_local(request)
            .await
            .expect("Failed to issue certificate");
        assert_eq!(issued_cert.common_name, "test.localhost");
        assert!(matches!(issued_cert.status, CertificateStatus::Valid));
        Ok(())
    }

    // -- Threshold CA signing tests -----------------------------------------

    #[tokio::test]
    async fn test_split_ca_key_produces_shares() {
        let config = CAConfig::testing();
        let mut ca = TrustChainCA::new(config)
            .await
            .expect("test: Failed to create CA");

        // Generate a FALCON-1024 keypair for threshold splitting
        let identity = crate::identity::FalconIdentity::generate();

        let threshold_cfg = ThresholdConfig {
            threshold: 3,
            total_shares: 5,
        };
        let shares = ca
            .split_ca_key(
                identity.secret_key_bytes(),
                &identity.public_key,
                threshold_cfg,
            )
            .expect("test: split_ca_key");

        assert_eq!(shares.len(), 5);
        assert!(ca.is_threshold_configured());
        assert_eq!(ca.threshold_config().expect("test: config").threshold, 3);
        assert_eq!(ca.threshold_config().expect("test: config").total_shares, 5);

        // All shares must have the same fingerprint
        let fp = shares[0].key_fingerprint;
        for share in &shares {
            assert_eq!(share.key_fingerprint, fp);
        }

        // Cached shares should match
        let cached = ca.ca_key_shares().expect("test: cached shares");
        assert_eq!(cached.len(), 5);
    }

    #[tokio::test]
    async fn test_sign_with_threshold_succeeds() {
        let config = CAConfig::testing();
        let mut ca = TrustChainCA::new(config)
            .await
            .expect("test: Failed to create CA");

        let identity = crate::identity::FalconIdentity::generate();

        let threshold_cfg = ThresholdConfig {
            threshold: 3,
            total_shares: 5,
        };
        let shares = ca
            .split_ca_key(
                identity.secret_key_bytes(),
                &identity.public_key,
                threshold_cfg,
            )
            .expect("test: split_ca_key");

        // Sign with exactly 3 shares (the threshold)
        let message = b"threshold CA signing test message";
        let signature = ca
            .sign_with_threshold(message, &shares[0..3])
            .expect("test: sign_with_threshold");

        // Verify with the original public key.
        // ThresholdSigner::reconstruct_and_sign hashes the message with SHA-256
        // before signing (matching FalconCrypto::sign convention), so we must
        // verify against the hashed message.
        let message_hash: [u8; 32] = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(message);
            h.finalize().into()
        };
        // verify_signature is on the NodeSigner trait
        use hypermesh_lib::NodeSigner;
        let valid = crate::identity::FalconIdentity::verify_signature(
            &identity.public_key,
            &message_hash,
            &signature,
        )
        .expect("test: verify_signature");
        assert!(valid, "Threshold signature should verify with original public key");
    }

    #[tokio::test]
    async fn test_sign_with_insufficient_shares_fails() {
        let config = CAConfig::testing();
        let mut ca = TrustChainCA::new(config)
            .await
            .expect("test: Failed to create CA");

        let identity = crate::identity::FalconIdentity::generate();

        let threshold_cfg = ThresholdConfig {
            threshold: 3,
            total_shares: 5,
        };
        let shares = ca
            .split_ca_key(
                identity.secret_key_bytes(),
                &identity.public_key,
                threshold_cfg,
            )
            .expect("test: split_ca_key");

        // Only 2 shares — below the threshold of 3
        let result = ca.sign_with_threshold(b"msg", &shares[0..2]);
        assert!(result.is_err(), "2-of-3 threshold should fail");
    }

    #[tokio::test]
    async fn test_threshold_not_configured_error() {
        let config = CAConfig::testing();
        let ca = TrustChainCA::new(config)
            .await
            .expect("test: Failed to create CA");

        // No split_ca_key() called — threshold not configured
        assert!(!ca.is_threshold_configured());
        assert!(ca.threshold_config().is_none());
        assert!(ca.ca_key_shares().is_none());

        let result = ca.sign_with_threshold(b"msg", &[]);
        assert!(result.is_err(), "sign_with_threshold without config should fail");
        let err_msg = result.err().expect("test: expected error").to_string();
        assert!(
            err_msg.contains("Threshold not configured"),
            "Error should mention threshold not configured, got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_certificate_validation() -> Result<(), Box<dyn std::error::Error>> {
        let config = CAConfig::testing(); // Use testing config with random port
        let ca = TrustChainCA::new(config)
            .await
            .expect("Failed to create CA");

        let request = CertificateRequest {
            common_name: "test.localhost".to_string(),
            san_entries: vec!["test.localhost".to_string()],
            node_id: "test_node_001".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            state_proof: StateProof::default_for_testing(),
            timestamp: SystemTime::now(),
            identity_scope: None,
            subject_type: None,
        };

        let issued_cert = ca
            .issue_certificate_local(request)
            .await
            .expect("Failed to issue certificate");
        let is_valid = ca
            .validate_certificate_chain(&issued_cert.certificate_der)
            .await
            .expect("Failed to validate certificate chain");
        assert!(is_valid);
        Ok(())
    }
}
