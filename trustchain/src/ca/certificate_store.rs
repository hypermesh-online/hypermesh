// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Store
//!
//! Canonical storage backend for issued certificates.
//! Uses DashMap for lock-free concurrent access.

use std::sync::Arc;
use dashmap::DashMap;
use tracing::info;

use crate::errors::Result as TrustChainResult;
use super::{IssuedCertificate, CertificateStatus};

/// Metrics for certificate store operations
#[derive(Default)]
pub struct CertificateStoreMetrics {
    pub total_certificates: std::sync::atomic::AtomicU64,
    pub revoked_certificates: std::sync::atomic::AtomicU64,
    pub expired_certificates: std::sync::atomic::AtomicU64,
}

/// Certificate storage backend using DashMap for concurrent access
#[derive(Clone)]
pub struct CertificateStore {
    /// Certificates indexed by serial number
    certificates: Arc<DashMap<String, IssuedCertificate>>,
    /// Operation metrics
    metrics: Arc<CertificateStoreMetrics>,
}

impl CertificateStore {
    /// Create new certificate store
    pub async fn new() -> TrustChainResult<Self> {
        Ok(Self {
            certificates: Arc::new(DashMap::new()),
            metrics: Arc::new(CertificateStoreMetrics::default()),
        })
    }

    /// Store certificate (indexed by serial number)
    pub async fn store_certificate(&self, certificate: &IssuedCertificate) -> TrustChainResult<()> {
        self.certificates.insert(certificate.serial_number.clone(), certificate.clone());
        self.metrics.total_certificates.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Get certificate by serial number
    pub async fn get_certificate_by_serial(&self, serial_number: &str) -> TrustChainResult<Option<IssuedCertificate>> {
        Ok(self.certificates.get(serial_number).map(|cert| cert.clone()))
    }

    /// Get certificate by fingerprint (hex-encoded)
    pub async fn get_certificate(&self, fingerprint: &str) -> TrustChainResult<Option<IssuedCertificate>> {
        let cert = self.certificates.iter()
            .find(|entry| hex::encode(entry.value().fingerprint) == fingerprint)
            .map(|entry| entry.value().clone());
        Ok(cert)
    }

    /// Revoke certificate by serial number
    pub async fn revoke_certificate(&self, serial_number: &str, reason: String) -> TrustChainResult<()> {
        if let Some(mut cert) = self.certificates.get_mut(serial_number) {
            cert.status = CertificateStatus::Revoked {
                reason,
                revoked_at: std::time::SystemTime::now(),
            };
            self.metrics.revoked_certificates.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            info!("Certificate revoked: {}", serial_number);
        }
        Ok(())
    }

    /// Get store metrics
    pub fn metrics(&self) -> &CertificateStoreMetrics {
        &self.metrics
    }

    /// Get total certificate count
    pub fn count(&self) -> usize {
        self.certificates.len()
    }
}
