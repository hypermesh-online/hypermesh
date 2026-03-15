// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Store
//!
//! Canonical storage backend for issued certificates.
//! Uses DashMap for lock-free concurrent access.
//! Revocation list is persisted to disk as JSON.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{error, info, warn};

use super::{CertificateStatus, IssuedCertificate};
use crate::errors::Result as TrustChainResult;

/// Default directory for trustchain persistent data.
const DEFAULT_DATA_DIR: &str = "./trustchain-data";

/// Default file name for the revocation list.
const REVOCATIONS_FILE: &str = "revocations.json";

/// Metrics for certificate store operations
#[derive(Default)]
pub struct CertificateStoreMetrics {
    pub total_certificates: std::sync::atomic::AtomicU64,
    pub revoked_certificates: std::sync::atomic::AtomicU64,
    pub expired_certificates: std::sync::atomic::AtomicU64,
}

/// A single revocation entry persisted to disk.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RevocationEntry {
    serial_number: String,
    reason: String,
    revoked_at: SystemTime,
}

/// Certificate storage backend using DashMap for concurrent access
#[derive(Clone)]
pub struct CertificateStore {
    /// Certificates indexed by serial number
    certificates: Arc<DashMap<String, IssuedCertificate>>,
    /// In-memory revocation list (serial_number -> entry)
    revocations: Arc<DashMap<String, RevocationEntry>>,
    /// Path to the revocation persistence file
    revocations_path: PathBuf,
    /// Operation metrics
    metrics: Arc<CertificateStoreMetrics>,
}

impl CertificateStore {
    /// Create new certificate store with default revocation persistence path.
    pub async fn new() -> TrustChainResult<Self> {
        Self::with_data_dir(DEFAULT_DATA_DIR).await
    }

    /// Create certificate store with a custom data directory.
    pub async fn with_data_dir(data_dir: &str) -> TrustChainResult<Self> {
        let revocations_path = Path::new(data_dir).join(REVOCATIONS_FILE);

        let store = Self {
            certificates: Arc::new(DashMap::new()),
            revocations: Arc::new(DashMap::new()),
            revocations_path,
            metrics: Arc::new(CertificateStoreMetrics::default()),
        };

        // Load persisted revocations from disk
        store.load_revocations();

        Ok(store)
    }

    /// Store certificate (indexed by serial number)
    pub async fn store_certificate(&self, certificate: &IssuedCertificate) -> TrustChainResult<()> {
        self.certificates
            .insert(certificate.serial_number.clone(), certificate.clone());
        self.metrics
            .total_certificates
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Get certificate by serial number
    pub async fn get_certificate_by_serial(
        &self,
        serial_number: &str,
    ) -> TrustChainResult<Option<IssuedCertificate>> {
        Ok(self
            .certificates
            .get(serial_number)
            .map(|cert| cert.clone()))
    }

    /// Get certificate by fingerprint (hex-encoded)
    pub async fn get_certificate(
        &self,
        fingerprint: &str,
    ) -> TrustChainResult<Option<IssuedCertificate>> {
        let cert = self
            .certificates
            .iter()
            .find(|entry| hex::encode(entry.value().fingerprint) == fingerprint)
            .map(|entry| entry.value().clone());
        Ok(cert)
    }

    /// Revoke certificate by serial number.
    ///
    /// Updates the in-memory store and flushes the revocation list to disk.
    pub async fn revoke_certificate(
        &self,
        serial_number: &str,
        reason: String,
    ) -> TrustChainResult<()> {
        let revoked_at = SystemTime::now();

        if let Some(mut cert) = self.certificates.get_mut(serial_number) {
            cert.status = CertificateStatus::Revoked {
                reason: reason.clone(),
                revoked_at,
            };
        }

        // Record in revocation list (persisted separately for restart survival)
        let entry = RevocationEntry {
            serial_number: serial_number.to_string(),
            reason,
            revoked_at,
        };
        self.revocations
            .insert(serial_number.to_string(), entry);

        self.metrics
            .revoked_certificates
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Flush to disk
        self.flush_revocations();

        info!("Certificate revoked: {}", serial_number);
        Ok(())
    }

    /// Check whether a serial number has been revoked.
    pub fn is_revoked(&self, serial_number: &str) -> bool {
        self.revocations.contains_key(serial_number)
    }

    /// Get store metrics
    pub fn metrics(&self) -> &CertificateStoreMetrics {
        &self.metrics
    }

    /// Get total certificate count
    pub fn count(&self) -> usize {
        self.certificates.len()
    }

    /// Iterate over all stored certificates (snapshot of current values).
    pub fn iter_certificates(&self) -> impl Iterator<Item = IssuedCertificate> + '_ {
        self.certificates.iter().map(|entry| entry.value().clone())
    }

    /// Return all certificates that are currently in Revoked status.
    ///
    /// Used by the CRL generator to build the revocation list.
    pub fn get_revoked_certificates(&self) -> Vec<IssuedCertificate> {
        self.certificates
            .iter()
            .filter(|entry| matches!(entry.value().status, CertificateStatus::Revoked { .. }))
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Find the first certificate matching a given common name.
    /// Returns `None` if no certificate with that CN exists.
    pub async fn find_by_common_name(&self, common_name: &str) -> Option<IssuedCertificate> {
        self.certificates
            .iter()
            .find(|entry| entry.value().common_name == common_name)
            .map(|entry| entry.value().clone())
    }

    // -- Persistence helpers --------------------------------------------------

    /// Load revocations from disk. Failures are logged but not propagated so
    /// the CA can still start even if the file is missing or corrupt.
    fn load_revocations(&self) {
        let path = &self.revocations_path;
        if !path.exists() {
            info!(
                "No revocation file found at {}, starting with empty list",
                path.display()
            );
            return;
        }

        match std::fs::read_to_string(path) {
            Ok(contents) => match serde_json::from_str::<Vec<RevocationEntry>>(&contents) {
                Ok(entries) => {
                    let count = entries.len();
                    for entry in entries {
                        self.revocations
                            .insert(entry.serial_number.clone(), entry);
                    }
                    info!(
                        "Loaded {} revocation entries from {}",
                        count,
                        path.display()
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to parse revocation file {}: {}",
                        path.display(),
                        e
                    );
                }
            },
            Err(e) => {
                warn!(
                    "Failed to read revocation file {}: {}",
                    path.display(),
                    e
                );
            }
        }
    }

    /// Flush the in-memory revocation list to disk as JSON.
    fn flush_revocations(&self) {
        let entries: Vec<RevocationEntry> = self
            .revocations
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        // Ensure the parent directory exists
        if let Some(parent) = self.revocations_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                error!(
                    "Failed to create revocation data directory {}: {}",
                    parent.display(),
                    e
                );
                return;
            }
        }

        match serde_json::to_string_pretty(&entries) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.revocations_path, json) {
                    error!(
                        "Failed to write revocation file {}: {}",
                        self.revocations_path.display(),
                        e
                    );
                }
            }
            Err(e) => {
                error!("Failed to serialize revocation list: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_revocation_persistence_roundtrip() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "trustchain-test-revocations-{}",
            std::process::id()
        ));
        let dir_str = tmp_dir.to_str().expect("test: temp dir as str");

        // Create store, revoke something, drop it
        {
            let store = CertificateStore::with_data_dir(dir_str)
                .await
                .expect("test: create store");
            store
                .revoke_certificate("serial-001", "test revocation".to_string())
                .await
                .expect("test: revoke");
            assert!(store.is_revoked("serial-001"));
        }

        // Create a new store from the same directory — should load from disk
        {
            let store = CertificateStore::with_data_dir(dir_str)
                .await
                .expect("test: create store 2");
            assert!(
                store.is_revoked("serial-001"),
                "Revocation should survive restart"
            );
            assert!(
                !store.is_revoked("serial-999"),
                "Non-revoked serial should not appear"
            );
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
