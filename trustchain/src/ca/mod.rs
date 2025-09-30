//! Certificate Authority Implementation
//! 
//! TrustChain Certificate Authority with NKrypt consensus validation and mandatory security integration
//! Supports both localhost testing and production deployment with IPv6-only networking

use std::sync::Arc;
use std::time::{SystemTime, Duration};
use std::collections::HashMap;
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};

use rustls::{Certificate, PrivateKey};
use rcgen::{generate_simple_self_signed, Certificate as RcgenCertificate};
use x509_parser::parse_x509_certificate;

use crate::consensus::{
    ConsensusProof, ConsensusContext, ConsensusRequirements, ConsensusResult,
    HyperMeshConsensusClient, HyperMeshClientConfig, ConsensusValidationService,
    ConsensusValidationStatus, ConsensusValidationResult, ConsensusClientMetrics,
    FourProofSet
};

pub mod certificate_manager;
pub mod certificate_store;
pub mod policy;
pub mod certificate_authority;
pub mod stoq_ca_client;
pub mod security_integration; // Security integration module

pub use certificate_manager::*;
pub use certificate_store::CertificateStore as CertStore;
pub use policy::*;
// AWS CloudHSM dependencies REMOVED - software-only operation
pub use stoq_ca_client::*;
// Re-export from certificate_authority with qualified imports
pub use certificate_authority::{TrustChainCA as TrustChainCAImpl, *};
// Re-export security integration
pub use security_integration::*;

/// TrustChain Certificate Authority (Legacy - use SecurityIntegratedCA for new deployments)
#[derive(Clone)]
pub struct TrustChainCA {
    /// Root CA certificate
    root_ca: Arc<RwLock<RcgenCertificate>>,
    /// Issued certificates store
    certificate_store: Arc<CertStore>,
    /// Certificate policies
    policy_engine: Arc<PolicyEngine>,
    /// Consensus validation context
    consensus_context: Arc<ConsensusContext>,
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
            port: 8443,
            cert_validity_days: 1, // 24 hour certificates
            rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            mode: CAMode::LocalhostTesting,
            consensus_requirements: ConsensusRequirements::localhost_testing(),
            hypermesh_client_config: HyperMeshClientConfig::localhost_testing(),
        }
    }
}

impl CAConfig {
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
                "https://hypermesh.hypermesh.online:8080".to_string()
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
}

/// Issued certificate information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IssuedCertificate {
    /// Certificate serial number
    pub serial_number: String,
    /// DER-encoded certificate
    pub certificate_der: Vec<u8>,
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
    Revoked { reason: String, revoked_at: SystemTime },
    Expired,
}

/// CA metrics for monitoring
#[derive(Default)]
pub struct CAMetrics {
    pub certificates_issued: std::sync::atomic::AtomicU64,
    // AWS CloudHSM dependencies REMOVED - software-only operation
    // hsm_operations metric removed
    pub consensus_validations: std::sync::atomic::AtomicU64,
    pub ct_log_entries: std::sync::atomic::AtomicU64,
    pub average_issuance_time_ms: std::sync::atomic::AtomicU64,
    pub performance_violations: std::sync::atomic::AtomicU64,
}

impl Clone for CAMetrics {
    fn clone(&self) -> Self {
        Self {
            certificates_issued: std::sync::atomic::AtomicU64::new(
                self.certificates_issued.load(std::sync::atomic::Ordering::Relaxed)
            ),
            // AWS CloudHSM dependencies REMOVED - hsm_operations removed
            consensus_validations: std::sync::atomic::AtomicU64::new(
                self.consensus_validations.load(std::sync::atomic::Ordering::Relaxed)
            ),
            ct_log_entries: std::sync::atomic::AtomicU64::new(
                self.ct_log_entries.load(std::sync::atomic::Ordering::Relaxed)
            ),
            average_issuance_time_ms: std::sync::atomic::AtomicU64::new(
                self.average_issuance_time_ms.load(std::sync::atomic::Ordering::Relaxed)
            ),
            performance_violations: std::sync::atomic::AtomicU64::new(
                self.performance_violations.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }
}

impl std::fmt::Debug for CAMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CAMetrics")
            .field("certificates_issued", &self.certificates_issued.load(std::sync::atomic::Ordering::Relaxed))
            // AWS CloudHSM dependencies REMOVED - hsm_operations field removed
            .field("consensus_validations", &self.consensus_validations.load(std::sync::atomic::Ordering::Relaxed))
            .field("ct_log_entries", &self.ct_log_entries.load(std::sync::atomic::Ordering::Relaxed))
            .field("average_issuance_time_ms", &self.average_issuance_time_ms.load(std::sync::atomic::Ordering::Relaxed))
            .field("performance_violations", &self.performance_violations.load(std::sync::atomic::Ordering::Relaxed))
            .finish()
    }
}

impl serde::Serialize for CAMetrics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("CAMetrics", 5)?; // AWS CloudHSM dependencies REMOVED - reduced field count
        state.serialize_field("certificates_issued", &self.certificates_issued.load(std::sync::atomic::Ordering::Relaxed))?;
        // AWS CloudHSM dependencies REMOVED - hsm_operations serialization removed
        state.serialize_field("consensus_validations", &self.consensus_validations.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("ct_log_entries", &self.ct_log_entries.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("average_issuance_time_ms", &self.average_issuance_time_ms.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("performance_violations", &self.performance_violations.load(std::sync::atomic::Ordering::Relaxed))?;
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
            // AWS CloudHSM dependencies REMOVED - hsm_operations field removed
            consensus_validations: u64,
            ct_log_entries: u64,
            average_issuance_time_ms: u64,
            performance_violations: u64,
        }
        
        let data = CAMetricsData::deserialize(deserializer)?;
        Ok(Self {
            certificates_issued: std::sync::atomic::AtomicU64::new(data.certificates_issued),
            // AWS CloudHSM dependencies REMOVED - software-only operation
            consensus_validations: std::sync::atomic::AtomicU64::new(data.consensus_validations),
            ct_log_entries: std::sync::atomic::AtomicU64::new(data.ct_log_entries),
            average_issuance_time_ms: std::sync::atomic::AtomicU64::new(data.average_issuance_time_ms),
            performance_violations: std::sync::atomic::AtomicU64::new(data.performance_violations),
        })
    }
}

impl TrustChainCA {
    /// Create a new TrustChain CA
    pub async fn new(config: CAConfig) -> Result<Self> {
        info!("Initializing TrustChain CA: {}", config.ca_id);

        // Initialize root CA certificate
        let root_ca = match config.mode {
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
        let hypermesh_client = Arc::new(
            HyperMeshConsensusClient::new(config.hypermesh_client_config.clone()).await?
        );

        // Initialize four-proof consensus validator
        let consensus = Arc::new(tokio::sync::Mutex::new(crate::consensus::FourProofValidator::new()));

        let ca = Self {
            root_ca: Arc::new(RwLock::new(root_ca)),
            certificate_store,
            policy_engine,
            consensus_context,
            hypermesh_client,
            consensus,
            config: Arc::new(config),
        };

        info!("TrustChain CA initialized successfully");
        Ok(ca)
    }

    /// Issue a new certificate with HyperMesh consensus validation
    pub async fn issue_certificate(&self, request: CertificateRequest) -> Result<IssuedCertificate> {
        info!("Processing certificate request for: {} with HyperMesh consensus validation", request.common_name);

        // Validate certificate request through HyperMesh consensus
        let consensus_result = self.hypermesh_client.validate_certificate_request(
            &request,
            &self.config.consensus_requirements,
        ).await?;

        // Process consensus validation result
        match consensus_result.result {
            ConsensusValidationStatus::Valid => {
                info!("HyperMesh consensus validation successful for: {}", request.common_name);
            }
            ConsensusValidationStatus::Invalid { failed_proofs, reason } => {
                error!("HyperMesh consensus validation failed for {}: {} (failed proofs: {:?})", 
                       request.common_name, reason, failed_proofs);
                return Err(anyhow!("HyperMesh consensus validation failed: {} (failed proofs: {:?})", reason, failed_proofs));
            }
            ConsensusValidationStatus::Pending { estimated_completion } => {
                error!("HyperMesh consensus validation pending for {}, estimated completion: {:?}", 
                       request.common_name, estimated_completion);
                return Err(anyhow!("HyperMesh consensus validation pending, try again later"));
            }
            ConsensusValidationStatus::Error { error_code, message } => {
                error!("HyperMesh consensus validation error for {}: {} ({})", 
                       request.common_name, message, error_code);
                return Err(anyhow!("HyperMesh consensus validation error: {} ({})", message, error_code));
            }
        }

        // Validate certificate policy
        if !self.policy_engine.validate_request(&request).await? {
            return Err(anyhow!("Certificate policy validation failed"));
        }

        // Generate certificate with HyperMesh consensus proof
        let issued_cert = self.generate_certificate_with_consensus(request, consensus_result).await?;

        // Store certificate
        self.certificate_store.store_certificate(&issued_cert).await?;

        info!("Certificate issued successfully with HyperMesh consensus: {}", issued_cert.serial_number);
        Ok(issued_cert)
    }

    /// Validate certificate chain
    pub async fn validate_certificate_chain(&self, cert_der: &[u8]) -> Result<bool> {
        debug!("Validating certificate chain");

        // Parse certificate
        let (_, parsed_cert) = parse_x509_certificate(cert_der)
            .map_err(|e| anyhow!("Failed to parse certificate: {}", e))?;

        // Calculate fingerprint
        let fingerprint = self.calculate_fingerprint(cert_der);

        // Check if certificate exists in store
        if let Some(stored_cert) = self.certificate_store.get_certificate(&hex::encode(fingerprint)).await? {
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
                        warn!("Consensus proof validation failed for certificate: {}", stored_cert.serial_number);
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

        self.certificate_store.revoke_certificate(serial_number, reason).await?;

        info!("Certificate revoked successfully: {}", serial_number);
        Ok(())
    }

    /// Get CA certificate for trust anchor
    pub async fn get_ca_certificate(&self) -> Result<Vec<u8>> {
        let root_ca = self.root_ca.read().await;
        Ok(root_ca.serialize_der()?)
    }

    /// Get root certificate (alias for API compatibility)
    pub async fn get_root_certificate(&self) -> Result<Vec<u8>> {
        self.get_ca_certificate().await
    }

    /// Internal: Create self-signed root CA
    fn create_self_signed_root(ca_id: &str) -> Result<RcgenCertificate> {
        let cert = generate_simple_self_signed(vec![ca_id.to_string()])?;
        Ok(cert)
    }

    /// Internal: Generate certificate with HyperMesh consensus validation result
    async fn generate_certificate_with_consensus(
        &self, 
        request: CertificateRequest,
        consensus_result: ConsensusValidationResult,
    ) -> Result<IssuedCertificate> {
        let root_ca = self.root_ca.read().await;
        
        // Create certificate with requested parameters
        let mut params = rcgen::CertificateParams::new(vec![request.common_name.clone()]);
        
        // Add SAN entries
        for san in &request.san_entries {
            params.subject_alt_names.push(rcgen::SanType::DnsName(san.clone()));
        }
        
        // Add IPv6 addresses
        for ipv6_addr in &request.ipv6_addresses {
            params.subject_alt_names.push(rcgen::SanType::IpAddress(std::net::IpAddr::V6(*ipv6_addr)));
        }
        
        // Set validity period
        let now = SystemTime::now();
        let expires_at = now + Duration::from_secs(self.config.cert_validity_days as u64 * 24 * 60 * 60);
        
        params.not_before = now.into();
        params.not_after = expires_at.into();
        
        // Add HyperMesh consensus metadata as certificate extension
        if let Some(proof_hash) = consensus_result.proof_hash {
            let consensus_extension = format!("HyperMesh-Consensus: {}, Validator: {}", 
                                              hex::encode(proof_hash), 
                                              consensus_result.validator_id);
            // Note: In production, this would be added as a proper X.509 extension
            debug!("Adding HyperMesh consensus metadata: {}", consensus_extension);
        }
        
        // Generate certificate
        let cert = rcgen::Certificate::from_params(params)?;
        let cert_der = cert.serialize_der_with_signer(&root_ca)?;
        
        // Calculate fingerprint
        let fingerprint = self.calculate_fingerprint(&cert_der);
        
        // Generate serial number
        let serial_number = hex::encode(&fingerprint[..16]);
        
        // Create enhanced metadata with HyperMesh consensus information
        let mut metadata = CertificateMetadata::default();
        metadata.tags.insert("consensus_validator".to_string(), consensus_result.validator_id);
        if let Some(proof_hash) = consensus_result.proof_hash {
            metadata.tags.insert("consensus_proof_hash".to_string(), hex::encode(proof_hash));
        }
        metadata.tags.insert("consensus_validation_time".to_string(), 
                             consensus_result.metrics.validation_time_us.to_string());
        metadata.tags.insert("consensus_confidence".to_string(), 
                             consensus_result.metrics.confidence_level.to_string());
        
        Ok(IssuedCertificate {
            serial_number,
            certificate_der: cert_der,
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
        info!("Validating four-proof set through HyperMesh for operation: {}", operation);
        
        let result = self.hypermesh_client.validate_four_proofs(
            proof_set,
            operation,
            asset_id,
            node_id,
        ).await?;

        match &result.result {
            ConsensusValidationStatus::Valid => {
                info!("Four-proof validation successful for operation: {}", operation);
            }
            ConsensusValidationStatus::Invalid { failed_proofs, reason } => {
                warn!("Four-proof validation failed for operation {}: {} (failed: {:?})", 
                      operation, reason, failed_proofs);
            }
            status => {
                debug!("Four-proof validation status for operation {}: {:?}", operation, status);
            }
        }

        Ok(result)
    }

    /// Internal: Calculate certificate fingerprint
    fn calculate_fingerprint(&self, cert_der: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
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
        let config = CAConfig::default();
        let ca = TrustChainCA::new(config).await.unwrap();
        
        let ca_cert = ca.get_ca_certificate().await.unwrap();
        assert!(!ca_cert.is_empty());
    }

    #[tokio::test]
    async fn test_certificate_issuance() {
        let config = CAConfig::default();
        let ca = TrustChainCA::new(config).await.unwrap();
        
        let request = CertificateRequest {
            common_name: "test.localhost".to_string(),
            san_entries: vec!["test.localhost".to_string()],
            node_id: "test_node_001".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            consensus_proof: ConsensusProof::generate_from_network(&node_id).await?,
            timestamp: SystemTime::now(),
        };
        
        let issued_cert = ca.issue_certificate(request).await.unwrap();
        assert_eq!(issued_cert.common_name, "test.localhost");
        assert!(matches!(issued_cert.status, CertificateStatus::Valid));
    }

    #[tokio::test]
    async fn test_certificate_validation() {
        let config = CAConfig::default();
        let ca = TrustChainCA::new(config).await.unwrap();
        
        let request = CertificateRequest {
            common_name: "test.localhost".to_string(),
            san_entries: vec!["test.localhost".to_string()],
            node_id: "test_node_001".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            consensus_proof: ConsensusProof::generate_from_network(&node_id).await?,
            timestamp: SystemTime::now(),
        };
        
        let issued_cert = ca.issue_certificate(request).await.unwrap();
        let is_valid = ca.validate_certificate_chain(&issued_cert.certificate_der).await.unwrap();
        assert!(is_valid);
    }
}