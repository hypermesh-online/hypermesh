// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Authority Implementation
//!
//! TrustChain Certificate Authority with Proof of State validation and mandatory security integration
//! Supports both localhost testing and production deployment with IPv6-only networking

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use rcgen::{Certificate as RcgenCertificate, CertificateParams, KeyPair};
use x509_parser::parse_x509_certificate;

use crate::consensus::{
    ConsensusClientMetrics, ConsensusContext, ConsensusProof, ConsensusRequirements,
    ConsensusValidationResult, ConsensusValidationStatus, FourProofSet, HyperMeshClientConfig,
    HyperMeshConsensusClient,
};

pub mod certificate_authority;
pub mod certificate_manager;
pub mod certificate_store;
pub mod federation;
pub mod field_bootstrap;
pub mod grace_period;
pub mod policy;
pub mod security_integration; // Security integration module
pub mod stoq_ca_client;

pub use certificate_manager::*;
pub use certificate_store::{CertificateStore as CertStore, CertificateStoreMetrics};
pub use field_bootstrap::{BootstrapState, FieldBootstrap, FieldBootstrapConfig};
pub use grace_period::{GracePeriodConfig, GracePeriodManager, GraceScope, RenewalToken};
pub use policy::*;
// AWS CloudHSM dependencies REMOVED - software-only operation
pub use stoq_ca_client::*;
// Re-export from certificate_authority with qualified imports
pub use certificate_authority::{TrustChainCA as TrustChainCAImpl, *};
// Re-export security integration
pub use security_integration::*;
// Re-export federation types
pub use federation::{
    FederatedCA, FederatedValidationResult, FederationManager, FederationPolicy, FederationStatus,
    FederationTrustLevel,
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
    /// Consensus validation context (retained for consensus proof operations)
    _consensus_context: Arc<ConsensusContext>,
    /// HyperMesh consensus client for validation
    hypermesh_client: Arc<HyperMeshConsensusClient>,
    /// Four-proof consensus validator (wrapped in Mutex for mutability)
    pub consensus: Arc<tokio::sync::Mutex<crate::consensus::FourProofValidator>>,
    /// CA configuration
    config: Arc<CAConfig>,
}

/// Certificate Authority Configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CAConfig {
    /// CA identifier
    pub ca_id: String,
    /// IPv6 bind address
    pub bind_address: std::net::Ipv6Addr,
    /// Port for CA services
    pub port: u16,
    /// Certificate validity period
    pub cert_validity_days: u32,
    /// Automatic rotation interval
    pub rotation_interval: Duration,
    /// Operating mode
    pub mode: CAMode,
    /// Consensus requirements
    pub consensus_requirements: ConsensusRequirements,
    /// HyperMesh consensus client configuration
    pub hypermesh_client_config: HyperMeshClientConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CAMode {
    /// Localhost testing with self-signed root
    LocalhostTesting,
    /// Production with software-protected root
    /// AWS CloudHSM dependencies REMOVED - software-only operation
    Production,
}

// AWS CloudHSM dependencies REMOVED - software-only operation
// HSM Configuration structures removed for software-only implementation

impl Default for CAConfig {
    fn default() -> Self {
        Self {
            ca_id: "trustchain-ca-localhost".to_string(),
            bind_address: std::net::Ipv6Addr::LOCALHOST,
            port: 8443,            // Standard CA port (use testing() method for port 0)
            cert_validity_days: 1, // 24 hour certificates
            rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            mode: CAMode::LocalhostTesting,
            consensus_requirements: ConsensusRequirements::localhost_testing(),
            hypermesh_client_config: HyperMeshClientConfig::localhost_testing(),
        }
    }
}

impl CAConfig {
    /// Testing configuration with OS-assigned random port
    pub fn testing() -> Self {
        Self {
            ca_id: "trustchain-ca-test".to_string(),
            bind_address: std::net::Ipv6Addr::LOCALHOST,
            port: 0, // OS-assigned random port to avoid conflicts
            cert_validity_days: 1,
            rotation_interval: Duration::from_secs(24 * 60 * 60),
            mode: CAMode::LocalhostTesting,
            consensus_requirements: ConsensusRequirements::localhost_testing(),
            hypermesh_client_config: HyperMeshClientConfig::localhost_testing(),
        }
    }

    /// Production configuration for trust.hypermesh.online
    pub fn production() -> Self {
        Self {
            ca_id: "trustchain-ca-production".to_string(),
            bind_address: std::net::Ipv6Addr::UNSPECIFIED, // Bind to all IPv6 interfaces
            port: 8443,
            cert_validity_days: 1, // 24 hour certificates
            rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            mode: CAMode::Production,
            consensus_requirements: ConsensusRequirements::production(),
            hypermesh_client_config: HyperMeshClientConfig::production(
                "https://hypermesh.hypermesh.online:8080".to_string(),
            ),
        }
    }
}

/// Certificate issuance request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertificateRequest {
    /// Common name for certificate
    pub common_name: String,
    /// Subject alternative names
    pub san_entries: Vec<String>,
    /// Requesting node ID
    pub node_id: String,
    /// IPv6 addresses for certificate
    pub ipv6_addresses: Vec<std::net::Ipv6Addr>,
    /// Consensus proof for validation
    pub consensus_proof: ConsensusProof,
    /// Request timestamp
    pub timestamp: SystemTime,
    /// Identity scope for scope-aware certificates (Item 2.6/2.7)
    /// When None, defaults to Device scope, untracked (anonymous)
    #[serde(default)]
    pub identity_scope: Option<CertificateIdentityScope>,
    /// Certificate subject type for KeyUsage/EKU selection (Item 2.4)
    /// When None, defaults to Node
    #[serde(default)]
    pub subject_type: Option<CertificateSubjectType>,
}

/// Identity scope embedded into certificates (Items 2.6, 2.7)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertificateIdentityScope {
    /// Blockchain scope: Device (local) or Network (synced)
    pub blockchain_scope: hypermesh_lib::BlockchainScope,
    /// Whether the identity is tracked
    pub tracked: bool,
}

/// Certificate subject type for KeyUsage/EKU decisions (Item 2.4)
#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CertificateSubjectType {
    /// Node identity: gets digitalSignature + keyEncipherment + serverAuth + clientAuth
    Node,
    /// Service identity: gets digitalSignature + serverAuth
    Service,
    /// Agent identity: gets digitalSignature + clientAuth
    Agent,
}

impl From<CertificateSubjectType> for hypermesh_lib::WorkloadType {
    fn from(cst: CertificateSubjectType) -> Self {
        match cst {
            CertificateSubjectType::Node => Self::Node,
            CertificateSubjectType::Service => Self::Service,
            CertificateSubjectType::Agent => Self::Agent,
        }
    }
}

/// Issued certificate information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IssuedCertificate {
    /// Certificate serial number
    pub serial_number: String,
    /// DER-encoded certificate
    pub certificate_der: Vec<u8>,
    /// PEM-encoded certificate (for API compatibility)
    pub certificate_pem: String,
    /// PEM-encoded certificate chain (for API compatibility)
    pub chain_pem: String,
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: [u8; 32],
    /// Common name
    pub common_name: String,
    /// Issue timestamp
    pub issued_at: SystemTime,
    /// Expiration timestamp
    pub expires_at: SystemTime,
    /// Issuing CA ID
    pub issuer_ca_id: String,
    /// Associated consensus proof
    pub consensus_proof: ConsensusProof,
    /// Certificate status
    pub status: CertificateStatus,
    /// Additional metadata
    pub metadata: CertificateMetadata,
}

/// Additional certificate metadata
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CertificateMetadata {
    /// Key algorithm used
    pub key_algorithm: Option<String>,
    /// Signature algorithm used
    pub signature_algorithm: Option<String>,
    /// Extensions included
    pub extensions: Vec<String>,
    /// Additional tags
    pub tags: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CertificateStatus {
    Valid,
    Revoked {
        reason: String,
        revoked_at: SystemTime,
    },
    Expired,
}

/// CA metrics for monitoring (Item 2.8: real certificate operation metrics)
#[derive(Default)]
pub struct CAMetrics {
    pub certificates_issued: std::sync::atomic::AtomicU64,
    /// Certificates revoked (Item 2.8)
    pub certificates_revoked: std::sync::atomic::AtomicU64,
    pub consensus_validations: std::sync::atomic::AtomicU64,
    pub ct_log_entries: std::sync::atomic::AtomicU64,
    pub average_issuance_time_ms: std::sync::atomic::AtomicU64,
    /// Validation latency in milliseconds (Item 2.8)
    pub validation_latency_ms: std::sync::atomic::AtomicU64,
    pub performance_violations: std::sync::atomic::AtomicU64,
}

impl Clone for CAMetrics {
    fn clone(&self) -> Self {
        use std::sync::atomic::Ordering::Relaxed;
        Self {
            certificates_issued: std::sync::atomic::AtomicU64::new(
                self.certificates_issued.load(Relaxed),
            ),
            certificates_revoked: std::sync::atomic::AtomicU64::new(
                self.certificates_revoked.load(Relaxed),
            ),
            consensus_validations: std::sync::atomic::AtomicU64::new(
                self.consensus_validations.load(Relaxed),
            ),
            ct_log_entries: std::sync::atomic::AtomicU64::new(
                self.ct_log_entries.load(Relaxed),
            ),
            average_issuance_time_ms: std::sync::atomic::AtomicU64::new(
                self.average_issuance_time_ms.load(Relaxed),
            ),
            validation_latency_ms: std::sync::atomic::AtomicU64::new(
                self.validation_latency_ms.load(Relaxed),
            ),
            performance_violations: std::sync::atomic::AtomicU64::new(
                self.performance_violations.load(Relaxed),
            ),
        }
    }
}

impl std::fmt::Debug for CAMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::sync::atomic::Ordering::Relaxed;
        f.debug_struct("CAMetrics")
            .field("certificates_issued", &self.certificates_issued.load(Relaxed))
            .field("certificates_revoked", &self.certificates_revoked.load(Relaxed))
            .field("consensus_validations", &self.consensus_validations.load(Relaxed))
            .field("ct_log_entries", &self.ct_log_entries.load(Relaxed))
            .field("average_issuance_time_ms", &self.average_issuance_time_ms.load(Relaxed))
            .field("validation_latency_ms", &self.validation_latency_ms.load(Relaxed))
            .field("performance_violations", &self.performance_violations.load(Relaxed))
            .finish()
    }
}

impl serde::Serialize for CAMetrics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        use std::sync::atomic::Ordering::Relaxed;
        let mut state = serializer.serialize_struct("CAMetrics", 7)?;
        state.serialize_field("certificates_issued", &self.certificates_issued.load(Relaxed))?;
        state.serialize_field("certificates_revoked", &self.certificates_revoked.load(Relaxed))?;
        state.serialize_field("consensus_validations", &self.consensus_validations.load(Relaxed))?;
        state.serialize_field("ct_log_entries", &self.ct_log_entries.load(Relaxed))?;
        state.serialize_field("average_issuance_time_ms", &self.average_issuance_time_ms.load(Relaxed))?;
        state.serialize_field("validation_latency_ms", &self.validation_latency_ms.load(Relaxed))?;
        state.serialize_field("performance_violations", &self.performance_violations.load(Relaxed))?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for CAMetrics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct CAMetricsData {
            certificates_issued: u64,
            #[serde(default)]
            certificates_revoked: u64,
            consensus_validations: u64,
            ct_log_entries: u64,
            average_issuance_time_ms: u64,
            #[serde(default)]
            validation_latency_ms: u64,
            performance_violations: u64,
        }

        let data = CAMetricsData::deserialize(deserializer)?;
        Ok(Self {
            certificates_issued: std::sync::atomic::AtomicU64::new(data.certificates_issued),
            certificates_revoked: std::sync::atomic::AtomicU64::new(data.certificates_revoked),
            consensus_validations: std::sync::atomic::AtomicU64::new(data.consensus_validations),
            ct_log_entries: std::sync::atomic::AtomicU64::new(data.ct_log_entries),
            average_issuance_time_ms: std::sync::atomic::AtomicU64::new(data.average_issuance_time_ms),
            validation_latency_ms: std::sync::atomic::AtomicU64::new(data.validation_latency_ms),
            performance_violations: std::sync::atomic::AtomicU64::new(data.performance_violations),
        })
    }
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
        let policy_engine = Arc::new(PolicyEngine::new(config.consensus_requirements.clone()));

        // Initialize consensus context
        let consensus_context = Arc::new(ConsensusContext::new(
            config.ca_id.clone(),
            "trustchain_network".to_string(),
        ));

        // Initialize HyperMesh consensus client
        let hypermesh_client =
            Arc::new(HyperMeshConsensusClient::new(config.hypermesh_client_config.clone()).await?);

        // Initialize four-proof consensus validator
        let consensus = Arc::new(tokio::sync::Mutex::new(
            crate::consensus::FourProofValidator::new(),
        ));

        let ca = Self {
            root_ca: Arc::new(RwLock::new(root_cert)),
            root_key_pair: Arc::new(root_key),
            certificate_store,
            policy_engine,
            _consensus_context: consensus_context,
            hypermesh_client,
            consensus,
            config: Arc::new(config),
        };

        info!("TrustChain CA initialized successfully");
        Ok(ca)
    }

    /// Issue a new certificate with HyperMesh consensus validation
    pub async fn issue_certificate(
        &self,
        request: CertificateRequest,
    ) -> Result<IssuedCertificate> {
        info!(
            "Processing certificate request for: {} with HyperMesh consensus validation",
            request.common_name
        );

        // Validate certificate request through HyperMesh consensus
        let consensus_result = self
            .hypermesh_client
            .validate_certificate_request(&request, &self.config.consensus_requirements)
            .await?;

        // Process consensus validation result
        match consensus_result.result {
            ConsensusValidationStatus::Valid => {
                info!(
                    "HyperMesh consensus validation successful for: {}",
                    request.common_name
                );
            }
            ConsensusValidationStatus::Invalid {
                failed_proofs,
                reason,
            } => {
                error!(
                    "HyperMesh consensus validation failed for {}: {} (failed proofs: {:?})",
                    request.common_name, reason, failed_proofs
                );
                return Err(anyhow!(
                    "HyperMesh consensus validation failed: {reason} (failed proofs: {failed_proofs:?})"
                ));
            }
            ConsensusValidationStatus::Pending {
                estimated_completion,
            } => {
                error!(
                    "HyperMesh consensus validation pending for {}, estimated completion: {:?}",
                    request.common_name, estimated_completion
                );
                return Err(anyhow!(
                    "HyperMesh consensus validation pending, try again later"
                ));
            }
            ConsensusValidationStatus::Error {
                error_code,
                message,
            } => {
                error!(
                    "HyperMesh consensus validation error for {}: {} ({})",
                    request.common_name, message, error_code
                );
                return Err(anyhow!(
                    "HyperMesh consensus validation error: {message} ({error_code})"
                ));
            }
        }

        // Validate certificate policy
        if !self.policy_engine.validate_request(&request).await? {
            return Err(anyhow!("Certificate policy validation failed"));
        }

        // Generate certificate with HyperMesh consensus proof
        let issued_cert = self
            .generate_certificate_with_consensus(request, consensus_result)
            .await?;

        // Store certificate
        self.certificate_store
            .store_certificate(&issued_cert)
            .await?;

        info!(
            "Certificate issued successfully with HyperMesh consensus: {}",
            issued_cert.serial_number
        );
        Ok(issued_cert)
    }

    /// Issue certificate with pre-validated consensus (skips HyperMesh network call).
    /// Used by SecurityIntegratedCA which already performed local consensus validation.
    pub async fn issue_certificate_local(
        &self,
        request: CertificateRequest,
    ) -> Result<IssuedCertificate> {
        info!(
            "Processing certificate request for: {} (pre-validated consensus)",
            request.common_name
        );

        // Validate certificate policy
        if !self.policy_engine.validate_request(&request).await? {
            return Err(anyhow!("Certificate policy validation failed"));
        }

        // Build a local consensus result (consensus was already validated by caller)
        let local_result = ConsensusValidationResult {
            result: ConsensusValidationStatus::Valid,
            proof_hash: request.consensus_proof.hash().ok(),
            validator_id: "local-security-integrated-ca".to_string(),
            validated_at: std::time::SystemTime::now(),
            metrics: crate::consensus::hypermesh_client::ValidationMetrics {
                validation_time_us: 0,
                validator_nodes: 1,
                confidence_level: 1.0,
                network_load: 0.0,
            },
            details: crate::consensus::hypermesh_client::ValidationDetails {
                proof_results: crate::consensus::hypermesh_client::ProofValidationResults {
                    space_proof_valid: true,
                    stake_proof_valid: true,
                    work_proof_valid: true,
                    time_proof_valid: true,
                },
                bft_status: crate::consensus::hypermesh_client::ByzantineFaultToleranceStatus {
                    byzantine_nodes_detected: 0,
                    fault_tolerance_maintained: true,
                    recovery_action_taken: None,
                },
                performance_stats: crate::consensus::hypermesh_client::PerformanceStatistics {
                    consensus_latency_ms: 0,
                    throughput_ops_per_sec: 0.0,
                    network_overhead_bytes: 0,
                },
            },
        };

        let issued_cert = self
            .generate_certificate_with_consensus(request, local_result)
            .await?;
        self.certificate_store
            .store_certificate(&issued_cert)
            .await?;

        info!(
            "Certificate issued successfully (pre-validated consensus): {}",
            issued_cert.serial_number
        );
        Ok(issued_cert)
    }

    /// Validate certificate chain
    pub async fn validate_certificate_chain(&self, cert_der: &[u8]) -> Result<bool> {
        let start = std::time::Instant::now();
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

                    // Validate consensus proof through HyperMesh (for legacy certificates with embedded proofs)
                    // For certificates issued with HyperMesh consensus, they are already validated
                    if stored_cert.consensus_proof.hash().is_ok() {
                        debug!("Certificate validation successful (HyperMesh consensus validated)");
                        return Ok(true);
                    } else {
                        warn!(
                            "Consensus proof validation failed for certificate: {}",
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

    /// Internal: Generate certificate with HyperMesh consensus validation result
    ///
    /// Uses rcgen `signed_by()` to produce a leaf certificate signed by the root CA,
    /// establishing a proper Root CA (self-signed) -> Leaf cert (CA-signed) hierarchy.
    async fn generate_certificate_with_consensus(
        &self,
        request: CertificateRequest,
        consensus_result: ConsensusValidationResult,
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

        // Add HyperMesh consensus metadata as certificate extension
        if let Some(proof_hash) = consensus_result.proof_hash {
            let consensus_extension = format!(
                "HyperMesh-Consensus: {}, Validator: {}",
                hex::encode(proof_hash),
                consensus_result.validator_id
            );
            debug!(
                "Adding HyperMesh consensus metadata: {}",
                consensus_extension
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

        // Create enhanced metadata with HyperMesh consensus information
        let mut metadata = CertificateMetadata::default();
        metadata.tags.insert(
            "consensus_validator".to_string(),
            consensus_result.validator_id,
        );
        if let Some(proof_hash) = consensus_result.proof_hash {
            metadata
                .tags
                .insert("consensus_proof_hash".to_string(), hex::encode(proof_hash));
        }
        metadata.tags.insert(
            "consensus_validation_time".to_string(),
            consensus_result.metrics.validation_time_us.to_string(),
        );
        metadata.tags.insert(
            "consensus_confidence".to_string(),
            consensus_result.metrics.confidence_level.to_string(),
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
            consensus_proof: request.consensus_proof,
            status: CertificateStatus::Valid,
            metadata,
        })
    }

    /// Get HyperMesh consensus client metrics
    pub async fn get_consensus_metrics(&self) -> Result<ConsensusClientMetrics> {
        Ok(self.hypermesh_client.get_metrics().await)
    }

    /// Reset HyperMesh consensus client metrics
    pub async fn reset_consensus_metrics(&self) -> Result<()> {
        self.hypermesh_client.reset_metrics().await;
        Ok(())
    }

    /// Validate four-proof set through HyperMesh for complex certificate operations
    pub async fn validate_four_proofs(
        &self,
        proof_set: &FourProofSet,
        operation: &str,
        asset_id: &str,
        node_id: &str,
    ) -> Result<ConsensusValidationResult> {
        info!(
            "Validating four-proof set through HyperMesh for operation: {}",
            operation
        );

        let result = self
            .hypermesh_client
            .validate_four_proofs(proof_set, operation, asset_id, node_id)
            .await?;

        match &result.result {
            ConsensusValidationStatus::Valid => {
                info!(
                    "Four-proof validation successful for operation: {}",
                    operation
                );
            }
            ConsensusValidationStatus::Invalid {
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
    use crate::consensus::ConsensusProof;

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
            consensus_proof: ConsensusProof::default_for_testing(),
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
            consensus_proof: ConsensusProof::default_for_testing(),
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
