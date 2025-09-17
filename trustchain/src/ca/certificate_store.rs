//! Certificate Store
//! 
//! Storage backend for issued certificates.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::Result;

use super::{IssuedCertificate, CertificateStatus};

/// Certificate storage backend
#[derive(Clone)]
pub struct CertificateStore {
    /// Certificates by serial number
    certificates: Arc<RwLock<HashMap<String, IssuedCertificate>>>,
}

impl CertificateStore {
    /// Create new certificate store
    pub async fn new() -> Result<Self> {
        Ok(Self {
            certificates: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Store certificate
    pub async fn store_certificate(&self, certificate: &IssuedCertificate) -> Result<()> {
        let mut certs = self.certificates.write().await;
        certs.insert(certificate.serial_number.clone(), certificate.clone());
        Ok(())
    }

    /// Get certificate by fingerprint
    pub async fn get_certificate(&self, fingerprint: &str) -> Result<Option<IssuedCertificate>> {
        let certs = self.certificates.read().await;
        // Find by fingerprint hex representation
        let cert = certs.values()
            .find(|c| hex::encode(c.fingerprint) == fingerprint)
            .cloned();
        Ok(cert)
    }

    /// Revoke certificate
    pub async fn revoke_certificate(&self, serial_number: &str, reason: String) -> Result<()> {
        let mut certs = self.certificates.write().await;
        if let Some(cert) = certs.get_mut(serial_number) {
            cert.status = CertificateStatus::Revoked {
                reason,
                revoked_at: std::time::SystemTime::now(),
            };
        }
        Ok(())
    }
}