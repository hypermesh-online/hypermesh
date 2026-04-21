// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Node Authentication for HyperMesh Transport

use blake3;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tracing::debug;
// X509 parsing removed for MVP simplicity

use super::config::AuthenticationConfig;
use super::error::Result;

/// Certificate validation result
#[derive(Debug, Clone)]
pub struct CertificateValidation {
    pub is_valid: bool,
    pub node_id: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub fingerprint: String,
    pub validation_errors: Vec<String>,
}

/// Cached certificate information
#[derive(Debug, Clone)]
struct CachedCertInfo {
    _node_id: String,
    _fingerprint: String,
    _expires_at: chrono::DateTime<chrono::Utc>,
    cached_at: Instant,
    validation_result: bool,
}

/// Node authenticator for certificate-based authentication
pub struct NodeAuthenticator {
    config: AuthenticationConfig,
    cert_cache: Arc<RwLock<HashMap<String, CachedCertInfo>>>,
    _trust_store: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

/// Auth manager (alias for NodeAuthenticator for compatibility)
pub type AuthManager = NodeAuthenticator;

impl NodeAuthenticator {
    /// Create a new node authenticator
    pub fn new(config: AuthenticationConfig) -> Self {
        let cert_cache = Arc::new(RwLock::new(HashMap::new()));
        let trust_store = Arc::new(RwLock::new(HashMap::new()));

        Self {
            config,
            cert_cache,
            _trust_store: trust_store,
        }
    }

    /// Authenticate a node by ID
    pub async fn authenticate_node(&self, node_id: &str) -> Result<bool> {
        debug!("Authenticating node {}", node_id);

        if !self.config.enable_cert_validation {
            debug!("Certificate validation disabled, auto-accepting");
            return Ok(true);
        }

        // Check cache first
        {
            let cache = self.cert_cache.read();
            if let Some(cached) = cache.get(node_id) {
                if cached.cached_at.elapsed() < Duration::from_secs(300) {
                    debug!("Using cached validation result for node {}", node_id);
                    return Ok(cached.validation_result);
                }
            }
        }

        // For MVP, perform basic node ID validation
        let is_valid = self.validate_node_id(node_id);

        // Cache the result
        {
            let mut cache = self.cert_cache.write();
            cache.insert(
                node_id.to_string(),
                CachedCertInfo {
                    _node_id: node_id.to_string(),
                    _fingerprint: self.generate_fingerprint(node_id),
                    _expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
                    cached_at: Instant::now(),
                    validation_result: is_valid,
                },
            );
        }

        Ok(is_valid)
    }

    /// Validate a node ID format
    fn validate_node_id(&self, node_id: &str) -> bool {
        // Basic validation: node ID should be non-empty and follow a pattern
        !node_id.is_empty()
            && (node_id.starts_with("node-")
                || node_id.starts_with("hypermesh-")
                || node_id.starts_with("hm-"))
    }

    /// Generate a fingerprint for a node ID
    fn generate_fingerprint(&self, node_id: &str) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(node_id.as_bytes());
        hasher.update(b"hypermesh-transport");
        let hash = hasher.finalize();
        hex::encode(hash.as_bytes())
    }

    /// Validate an X.509 certificate: parse subject CN, check expiry, compute fingerprint
    pub async fn validate_certificate(&self, cert_data: &[u8]) -> Result<CertificateValidation> {
        let (_, cert) = x509_parser::parse_x509_certificate(cert_data).map_err(|e| {
            super::error::TransportError::CertificateValidation(format!("Invalid X.509: {e}"))
        })?;

        let node_id = cert
            .tbs_certificate
            .subject
            .iter_common_name()
            .next()
            .and_then(|cn| cn.as_str().ok())
            .unwrap_or("unknown-node")
            .to_string();

        let now = chrono::Utc::now();
        let not_after = chrono::DateTime::from_timestamp(
            cert.tbs_certificate.validity.not_after.timestamp(),
            0,
        )
        .ok_or_else(|| {
            super::error::TransportError::CertificateValidation(
                "Invalid not_after timestamp".to_string(),
            )
        })?;

        let node_id_valid = self.validate_node_id(&node_id);
        let not_expired = now < not_after;
        let is_valid = not_expired && node_id_valid;

        let mut validation_errors = Vec::new();
        if !not_expired {
            validation_errors.push("certificate expired".to_string());
        }
        if !node_id_valid {
            validation_errors.push(format!("invalid node_id format: {node_id}"));
        }

        let fingerprint = hex::encode(blake3::hash(cert_data).as_bytes());

        Ok(CertificateValidation {
            is_valid,
            node_id,
            expires_at: not_after,
            fingerprint,
            validation_errors,
        })
    }

    /// Clear the certificate cache
    pub fn clear_cache(&self) {
        let mut cache = self.cert_cache.write();
        cache.clear();
        debug!("Certificate cache cleared");
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cert_cache.read();
        let total = cache.len();
        let valid = cache.values().filter(|c| c.validation_result).count();
        (total, valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_authentication() {
        let config = AuthenticationConfig::default();
        let auth = NodeAuthenticator::new(config);

        // Test valid node IDs
        assert!(auth.authenticate_node("node-123").await.expect("test: async operation"));
        assert!(auth.authenticate_node("hypermesh-abc").await.expect("test: async operation"));
        assert!(auth.authenticate_node("hm-xyz").await.expect("test: async operation"));

        // Test invalid node ID
        assert!(!auth.authenticate_node("invalid").await.expect("test: async operation"));
    }

    #[test]
    fn test_fingerprint_generation() {
        let config = AuthenticationConfig::default();
        let auth = NodeAuthenticator::new(config);

        let fp1 = auth.generate_fingerprint("node-123");
        let fp2 = auth.generate_fingerprint("node-123");
        let fp3 = auth.generate_fingerprint("node-456");

        // Same input should produce same fingerprint
        assert_eq!(fp1, fp2);
        // Different input should produce different fingerprint
        assert_ne!(fp1, fp3);
    }
}
