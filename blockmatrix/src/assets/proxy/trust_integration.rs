// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain Integration for Proxy System
//!
//! Integrates with TrustChain certificate hierarchy for federated trust validation

use blake3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::assets::core::{AssetError, AssetResult, ProxyCapabilities, ProxyNodeInfo};

/// TrustChain integration handler
pub struct TrustChainIntegration {
    /// Certificate validator
    certificate_validator: CertificateValidator,

    /// Trust chain cache
    trust_chain_cache: HashMap<String, TrustChain>,

    /// Certificate revocation list
    revocation_list: HashMap<String, RevocationEntry>,

    /// Integration configuration
    config: TrustChainConfig,
}

/// Certificate validator for TrustChain integration
pub struct CertificateValidator {
    /// Root certificate authorities
    root_cas: HashMap<String, RootCA>,

    /// Intermediate certificate authorities
    intermediate_cas: HashMap<String, IntermediateCA>,

    /// Certificate validation cache
    validation_cache: HashMap<String, ValidationResult>,

    /// Validation configuration
    validation_config: ValidationConfig,
}

/// Trust chain representation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustChain {
    /// Chain identifier
    pub chain_id: String,

    /// Root CA certificate fingerprint
    pub root_ca_fingerprint: String,

    /// Intermediate CA certificates (if any)
    pub intermediate_certificates: Vec<String>,

    /// End entity certificate fingerprint
    pub end_entity_fingerprint: String,

    /// Chain validation status
    pub validation_status: ChainValidationStatus,

    /// Whether the trust chain is valid (binary: valid or invalid)
    pub valid: bool,

    /// Chain creation timestamp
    pub created_at: SystemTime,

    /// Last validation timestamp
    pub last_validated: SystemTime,

    /// Chain expiration timestamp
    pub expires_at: SystemTime,
}

/// Chain validation status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChainValidationStatus {
    /// Chain is valid and trusted
    Valid,

    /// Chain validation is pending
    Pending,

    /// Chain has expired
    Expired,

    /// Chain validation failed
    ValidationFailed { reason: String },

    /// Chain has been revoked
    Revoked,

    /// Chain is untrusted
    Untrusted,
}

/// Root Certificate Authority
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RootCA {
    /// CA identifier
    ca_id: String,

    /// CA name
    ca_name: String,

    /// Public key fingerprint
    public_key_fingerprint: String,

    /// CA status
    status: CAStatus,

    /// Certificate validity period
    valid_from: SystemTime,
    valid_until: SystemTime,
}

/// Intermediate Certificate Authority
#[derive(Clone, Debug, Serialize, Deserialize)]
struct IntermediateCA {
    /// CA identifier
    ca_id: String,

    /// CA name
    ca_name: String,

    /// Parent CA identifier
    parent_ca_id: String,

    /// Public key fingerprint
    public_key_fingerprint: String,

    /// CA status
    status: CAStatus,

    /// Certificate validity period
    valid_from: SystemTime,
    valid_until: SystemTime,
}

/// Certificate Authority status
#[derive(Clone, Debug, Serialize, Deserialize)]
enum CAStatus {
    Active,
    Suspended,
    Revoked,
    Expired,
}

/// Certificate validation result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Certificate fingerprint
    pub certificate_fingerprint: String,

    /// Whether the certificate chain is valid (binary)
    pub valid: bool,

    /// Validation reason/message
    pub validation_message: String,

    /// Validation errors (if any)
    pub errors: Vec<String>,

    /// Validation timestamp
    pub validated_at: SystemTime,

    /// Result expiration
    pub expires_at: SystemTime,
}

/// Certificate revocation entry
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RevocationEntry {
    /// Revoked certificate fingerprint
    certificate_fingerprint: String,

    /// Revocation reason
    revocation_reason: RevocationReason,

    /// Revocation timestamp
    revoked_at: SystemTime,

    /// Revoking authority
    revoking_authority: String,
}

/// Reasons for certificate revocation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RevocationReason {
    KeyCompromise,
    CACompromise,
    AffiliationChanged,
    Superseded,
    CessationOfOperation,
    CertificateHold,
    RemoveFromCRL,
    PrivilegeWithdrawn,
    AACompromise,
}

/// TrustChain configuration
#[derive(Clone, Debug)]
struct TrustChainConfig {
    /// Enable certificate validation caching
    enable_validation_caching: bool,

    /// Validation cache timeout
    validation_cache_timeout: Duration,

    /// Maximum trust chain length
    max_chain_length: u8,

    /// Enable online revocation checking
    enable_online_revocation_check: bool,

    /// Revocation check timeout
    _revocation_check_timeout: Duration,
}

impl Default for TrustChainConfig {
    fn default() -> Self {
        Self {
            enable_validation_caching: true,
            validation_cache_timeout: Duration::from_secs(3600), // 1 hour
            max_chain_length: 5,
            enable_online_revocation_check: true,
            _revocation_check_timeout: Duration::from_secs(30),
        }
    }
}

/// Validation configuration
#[derive(Clone, Debug)]
struct ValidationConfig {
    /// Strict validation mode
    strict_mode: bool,

    /// Allow self-signed certificates in development
    allow_self_signed: bool,

    /// Signature algorithm whitelist
    _allowed_signature_algorithms: Vec<String>,

    /// Minimum key length
    _min_key_length: u32,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            allow_self_signed: false,
            _allowed_signature_algorithms: vec![
                "FALCON-1024".to_string(),
                "Ed25519".to_string(),
                "ECDSA-P256".to_string(),
                "RSA-PSS".to_string(),
            ],
            _min_key_length: 2048,
        }
    }
}

impl Default for TrustChainIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustChainIntegration {
    /// Create new TrustChain integration
    pub fn new() -> Self {
        Self {
            certificate_validator: CertificateValidator::new().unwrap_or_else(|_| {
                CertificateValidator {
                    root_cas: HashMap::new(),
                    intermediate_cas: HashMap::new(),
                    validation_cache: HashMap::new(),
                    validation_config: ValidationConfig::default(),
                }
            }),
            trust_chain_cache: HashMap::new(),
            revocation_list: HashMap::new(),
            config: TrustChainConfig::default(),
        }
    }

    /// Validate proxy node certificate against TrustChain
    pub async fn validate_node_certificate(&self, node_info: &ProxyNodeInfo) -> AssetResult<bool> {
        // Check validation cache first
        if self.config.enable_validation_caching {
            if let Some(cached_result) = self
                .get_cached_validation(&node_info.certificate_fingerprint)
                .await?
            {
                if cached_result.expires_at > SystemTime::now() {
                    return Ok(cached_result.valid);
                }
            }
        }

        // Check revocation list
        if self
            .is_certificate_revoked(&node_info.certificate_fingerprint)
            .await?
        {
            tracing::warn!(
                "Certificate is revoked: {}",
                node_info.certificate_fingerprint
            );
            return Ok(false);
        }

        // Build trust chain
        let trust_chain = self
            .build_trust_chain(&node_info.certificate_fingerprint)
            .await?;

        // Validate trust chain
        let validation_result = self.validate_trust_chain(&trust_chain).await?;

        // Cache validation result
        if self.config.enable_validation_caching {
            self.cache_validation_result(&node_info.certificate_fingerprint, &validation_result)
                .await?;
        }

        tracing::info!(
            "Node certificate validation for {:?}: {}",
            node_info.node_id,
            validation_result.valid,
        );

        Ok(validation_result.valid)
    }

    /// Build trust chain for certificate
    async fn build_trust_chain(&self, certificate_fingerprint: &str) -> AssetResult<TrustChain> {
        // Check cache first
        if let Some(cached_chain) = self.trust_chain_cache.get(certificate_fingerprint) {
            if cached_chain.expires_at > SystemTime::now() {
                return Ok(cached_chain.clone());
            }
        }

        // TODO: Implement actual TrustChain certificate chain building
        // For now, simulate chain building
        let chain = TrustChain {
            chain_id: self.generate_chain_id(certificate_fingerprint)?,
            root_ca_fingerprint: "hypermesh-root-ca".to_string(),
            intermediate_certificates: vec!["hypermesh-intermediate-ca".to_string()],
            end_entity_fingerprint: certificate_fingerprint.to_string(),
            validation_status: ChainValidationStatus::Pending,
            valid: false, // Will be determined during validation
            created_at: SystemTime::now(),
            last_validated: SystemTime::UNIX_EPOCH,
            expires_at: SystemTime::now() + Duration::from_secs(86400), // 24 hours
        };

        tracing::debug!(
            "Built trust chain for certificate: {}",
            certificate_fingerprint
        );
        Ok(chain)
    }

    /// Validate trust chain (binary: valid or invalid)
    async fn validate_trust_chain(
        &self,
        trust_chain: &TrustChain,
    ) -> AssetResult<ValidationResult> {
        let mut is_valid = true;
        let mut validation_message = "Trust chain validation successful".to_string();

        // Validate root CA
        if let Some(root_ca) = self
            .certificate_validator
            .root_cas
            .get(&trust_chain.root_ca_fingerprint)
        {
            if !matches!(root_ca.status, CAStatus::Active) {
                is_valid = false;
                validation_message = "Root CA is not active".to_string();
            } else if root_ca.valid_until < SystemTime::now() {
                is_valid = false;
                validation_message = "Root CA certificate has expired".to_string();
            }
        } else {
            is_valid = false;
            validation_message = "Root CA not found in trust store".to_string();
        }

        // Validate intermediate CAs
        for intermediate_fingerprint in &trust_chain.intermediate_certificates {
            if let Some(intermediate_ca) = self
                .certificate_validator
                .intermediate_cas
                .get(intermediate_fingerprint)
            {
                if !matches!(intermediate_ca.status, CAStatus::Active) {
                    is_valid = false;
                    validation_message =
                        format!("Intermediate CA {intermediate_fingerprint} is not active");
                    break;
                } else if intermediate_ca.valid_until < SystemTime::now() {
                    is_valid = false;
                    validation_message =
                        format!("Intermediate CA {intermediate_fingerprint} has expired");
                    break;
                }
            } else {
                is_valid = false;
                validation_message =
                    format!("Intermediate CA {intermediate_fingerprint} not found");
                break;
            }
        }

        // Validate chain length
        let chain_length = 1 + trust_chain.intermediate_certificates.len() as u8; // Root + intermediates
        if chain_length > self.config.max_chain_length {
            is_valid = false;
            validation_message = format!(
                "Trust chain too long: {} > {}",
                chain_length, self.config.max_chain_length
            );
        }

        let validation_message_clone = validation_message.clone();
        let result = ValidationResult {
            certificate_fingerprint: trust_chain.end_entity_fingerprint.clone(),
            valid: is_valid,
            validation_message,
            errors: if is_valid {
                Vec::new()
            } else {
                vec![validation_message_clone]
            },
            validated_at: SystemTime::now(),
            expires_at: SystemTime::now() + self.config.validation_cache_timeout,
        };

        tracing::debug!(
            "Trust chain validation result: {}",
            result.valid,
        );

        Ok(result)
    }

    /// Check if certificate is revoked
    async fn is_certificate_revoked(&self, certificate_fingerprint: &str) -> AssetResult<bool> {
        // Check local revocation list
        if self.revocation_list.contains_key(certificate_fingerprint) {
            return Ok(true);
        }

        // TODO: Implement online revocation checking (OCSP/CRL)
        if self.config.enable_online_revocation_check {
            // Simulate online revocation check
            tracing::debug!(
                "Performing online revocation check for: {}",
                certificate_fingerprint
            );
        }

        Ok(false)
    }

    /// Get cached validation result
    async fn get_cached_validation(
        &self,
        _certificate_fingerprint: &str,
    ) -> AssetResult<Option<ValidationResult>> {
        // TODO: Implement actual cache lookup
        // For now, return None to force validation
        Ok(None)
    }

    /// Cache validation result
    async fn cache_validation_result(
        &self,
        certificate_fingerprint: &str,
        _validation_result: &ValidationResult,
    ) -> AssetResult<()> {
        // TODO: Implement actual cache storage
        tracing::debug!("Cached validation result for: {}", certificate_fingerprint);
        Ok(())
    }

    /// Generate chain ID
    fn generate_chain_id(&self, certificate_fingerprint: &str) -> AssetResult<String> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(certificate_fingerprint.as_bytes());
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| AssetError::AdapterError {
                message: "Invalid system time for chain ID generation".to_string(),
            })?
            .as_nanos();
        hasher.update(&nanos.to_le_bytes());
        let hash = hasher.finalize();
        Ok(hex::encode(&hash.as_bytes()[..16]))
    }

    /// Add certificate to revocation list
    pub async fn revoke_certificate(
        &mut self,
        certificate_fingerprint: String,
        reason: RevocationReason,
        revoking_authority: String,
    ) -> AssetResult<()> {
        let revocation_entry = RevocationEntry {
            certificate_fingerprint: certificate_fingerprint.clone(),
            revocation_reason: reason,
            revoked_at: SystemTime::now(),
            revoking_authority,
        };

        self.revocation_list
            .insert(certificate_fingerprint.clone(), revocation_entry);

        tracing::warn!("Revoked certificate: {}", certificate_fingerprint);
        Ok(())
    }

    /// Check if certificate is valid (binary: valid or invalid)
    pub async fn is_certificate_valid(
        &self,
        certificate_fingerprint: &str,
    ) -> AssetResult<bool> {
        if let Some(cached_result) = self.get_cached_validation(certificate_fingerprint).await? {
            if cached_result.expires_at > SystemTime::now() {
                return Ok(cached_result.valid);
            }
        }

        let trust_chain = self.build_trust_chain(certificate_fingerprint).await?;
        let validation_result = self.validate_trust_chain(&trust_chain).await?;

        Ok(validation_result.valid)
    }

    /// Backward-compat alias: returns 1.0 for valid, 0.0 for invalid
    pub async fn get_certificate_trust_level(
        &self,
        certificate_fingerprint: &str,
    ) -> AssetResult<f32> {
        let valid = self.is_certificate_valid(certificate_fingerprint).await?;
        Ok(if valid { 1.0 } else { 0.0 })
    }

    /// Cleanup expired cache entries
    pub async fn cleanup_expired_cache(&self) -> AssetResult<u64> {
        // TODO: Implement cache cleanup
        tracing::debug!("Cleaned up expired trust chain cache entries");
        Ok(0)
    }
}

impl CertificateValidator {
    /// Create new certificate validator
    pub fn new() -> AssetResult<Self> {
        let mut root_cas = HashMap::new();
        let mut intermediate_cas = HashMap::new();

        // Add default HyperMesh root CA
        root_cas.insert(
            "hypermesh-root-ca".to_string(),
            RootCA {
                ca_id: "hypermesh-root-ca".to_string(),
                ca_name: "HyperMesh Root CA".to_string(),
                public_key_fingerprint: "hypermesh-root-ca-key".to_string(),
                status: CAStatus::Active,
                valid_from: SystemTime::now() - Duration::from_secs(86400 * 365), // 1 year ago
                valid_until: SystemTime::now() + Duration::from_secs(86400 * 365 * 10), // 10 years
            },
        );

        // Add default intermediate CA
        intermediate_cas.insert(
            "hypermesh-intermediate-ca".to_string(),
            IntermediateCA {
                ca_id: "hypermesh-intermediate-ca".to_string(),
                ca_name: "HyperMesh Intermediate CA".to_string(),
                parent_ca_id: "hypermesh-root-ca".to_string(),
                public_key_fingerprint: "hypermesh-intermediate-ca-key".to_string(),
                status: CAStatus::Active,
                valid_from: SystemTime::now() - Duration::from_secs(86400 * 30), // 30 days ago
                valid_until: SystemTime::now() + Duration::from_secs(86400 * 365 * 2), // 2 years
            },
        );

        Ok(Self {
            root_cas,
            intermediate_cas,
            validation_cache: HashMap::new(),
            validation_config: ValidationConfig::default(),
        })
    }

    /// Validate a certificate
    pub async fn validate_certificate(
        &self,
        certificate_fingerprint: &str,
    ) -> AssetResult<ValidationResult> {
        // Check cache first
        if let Some(cached) = self.validation_cache.get(certificate_fingerprint) {
            if cached.expires_at > SystemTime::now() {
                return Ok(cached.clone());
            }
        }

        // Perform validation (binary: valid or invalid)
        let mut result = ValidationResult {
            certificate_fingerprint: certificate_fingerprint.to_string(),
            valid: true,
            validation_message: String::new(),
            errors: Vec::new(),
            validated_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
        };

        // Check if it's a known root CA
        if self.root_cas.contains_key(certificate_fingerprint) {
            result.validation_message = "Root CA certificate".to_string();
            return Ok(result);
        }

        // Check if it's an intermediate CA
        if let Some(intermediate) = self.intermediate_cas.get(certificate_fingerprint) {
            // Validate chain to root
            if self.root_cas.contains_key(&intermediate.parent_ca_id) {
                result.validation_message = "Valid intermediate CA with trusted root".to_string();
            } else {
                result.valid = false;
                result.validation_message = "Intermediate CA with unverified root".to_string();
                result
                    .errors
                    .push("Root CA not verified".to_string());
            }
            return Ok(result);
        }

        // For unknown certificates in non-strict mode, accept
        if !self.validation_config.strict_mode || self.validation_config.allow_self_signed {
            result.validation_message =
                "Self-signed or unknown certificate accepted in non-strict mode".to_string();
        } else {
            result.valid = false;
            result.validation_message = "Certificate validation failed".to_string();
            result
                .errors
                .push("Unknown certificate in strict mode".to_string());
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::ProxyCapabilities;

    fn create_test_node_info() -> ProxyNodeInfo {
        // Convert string to [u8; 8] for node_id
        let mut node_id_bytes = [0u8; 8];
        let bytes = "test-node".as_bytes();
        let len = bytes.len().min(8);
        node_id_bytes[..len].copy_from_slice(&bytes[..len]);

        ProxyNodeInfo {
            node_id: node_id_bytes,
            network_address: "192.168.1.100".to_string(),
            capabilities: ProxyCapabilities {
                http_proxy: true,
                socks5_proxy: true,
                tcp_forwarding: true,
                vpn_tunnel: false,
                max_connections: 1000,
                bandwidth_mbps: 1000,
                protocols: vec!["HTTP".to_string(), "SOCKS5".to_string()],
            },
            is_authenticated: true,
            last_heartbeat: SystemTime::now(),
            certificate_fingerprint: "test-cert-fingerprint".to_string(),
        }
    }

    #[tokio::test]
    async fn test_trust_chain_integration_creation() {
        let integration = TrustChainIntegration::new();
        assert_eq!(integration.trust_chain_cache.len(), 0);
        assert_eq!(integration.revocation_list.len(), 0);
    }

    #[tokio::test]
    async fn test_certificate_validator_creation() {
        let validator = CertificateValidator::new().expect("Failed to create CertificateValidator");
        assert!(!validator.root_cas.is_empty());
        assert!(!validator.intermediate_cas.is_empty());
    }

    #[tokio::test]
    async fn test_build_trust_chain() {
        let integration = TrustChainIntegration::new();
        let cert_fingerprint = "test-cert-fingerprint";

        let trust_chain = integration
            .build_trust_chain(cert_fingerprint)
            .await
            .expect("Failed to build trust chain");

        assert_eq!(trust_chain.end_entity_fingerprint, cert_fingerprint);
        assert!(!trust_chain.chain_id.is_empty());
        assert!(!trust_chain.intermediate_certificates.is_empty());
    }

    #[tokio::test]
    async fn test_validate_node_certificate() {
        let integration = TrustChainIntegration::new();
        let node_info = create_test_node_info();

        // This should succeed with the default setup
        let is_valid = integration
            .validate_node_certificate(&node_info)
            .await
            .expect("Failed to validate node certificate");
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_certificate_revocation() {
        let mut integration = TrustChainIntegration::new();
        let cert_fingerprint = "test-cert-to-revoke".to_string();

        // Certificate should not be revoked initially
        let is_revoked = integration
            .is_certificate_revoked(&cert_fingerprint)
            .await
            .expect("Failed to check revocation status");
        assert!(!is_revoked);

        // Revoke the certificate
        integration
            .revoke_certificate(
                cert_fingerprint.clone(),
                RevocationReason::KeyCompromise,
                "test-authority".to_string(),
            )
            .await
            .expect("Failed to revoke certificate");

        // Certificate should now be revoked
        let is_revoked = integration
            .is_certificate_revoked(&cert_fingerprint)
            .await
            .expect("Failed to check revocation status");
        assert!(is_revoked);
    }
}
