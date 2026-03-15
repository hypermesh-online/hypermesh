// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Revocation List (CRL) Generator and Distributor
//!
//! Generates and distributes CRLs signed with FALCON-1024 for quantum-resistant
//! certificate revocation verification. The CRL follows a simplified model
//! suitable for STOQ transport rather than full ASN.1 DER encoding.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::certificate_store::CertificateStore;
use super::stoq_ca_client::RevocationReason;
use super::CertificateStatus;
use crate::crypto::falcon::FalconCrypto;
use crate::crypto::{FalconPrivateKey, FalconSignature};
use crate::errors::Result as TrustChainResult;
use crate::errors::TrustChainError;

/// A single entry in the Certificate Revocation List.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevokedCertEntry {
    /// Serial number of the revoked certificate.
    pub serial_number: String,
    /// Time at which the certificate was revoked.
    pub revocation_date: SystemTime,
    /// Reason for revocation.
    pub reason: RevocationReason,
}

/// A signed Certificate Revocation List.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertificateRevocationList {
    /// CRL version (v2 = 1).
    pub version: u8,
    /// Issuer distinguished name.
    pub issuer: String,
    /// Time at which this CRL was generated.
    pub this_update: SystemTime,
    /// Time at which the next CRL will be published.
    pub next_update: SystemTime,
    /// List of revoked certificate entries.
    pub revoked_certificates: Vec<RevokedCertEntry>,
    /// FALCON-1024 signature over the CRL contents.
    pub signature: FalconSignature,
    /// Monotonically increasing CRL sequence number.
    pub crl_number: u64,
}

/// Generates CRLs from the certificate store.
pub struct CrlGenerator {
    /// Certificate store to scan for revoked certificates.
    store: Arc<CertificateStore>,
    /// FALCON-1024 private key for CRL signing.
    signing_key: FalconPrivateKey,
    /// FALCON crypto engine.
    falcon: FalconCrypto,
    /// Issuer name included in every CRL.
    issuer_name: String,
    /// Validity period for each CRL.
    validity_period: Duration,
    /// Atomic CRL number counter.
    crl_number: AtomicU64,
}

impl CrlGenerator {
    /// Create a new CRL generator.
    ///
    /// * `store` - Certificate store to scan for revoked certificates.
    /// * `signing_key` - FALCON-1024 private key for CRL signing.
    /// * `issuer_name` - Distinguished name of the CRL issuer.
    /// * `validity_period` - How long the CRL is considered current (default: 24h).
    pub fn new(
        store: Arc<CertificateStore>,
        signing_key: FalconPrivateKey,
        issuer_name: String,
        validity_period: Option<Duration>,
    ) -> TrustChainResult<Self> {
        let falcon = FalconCrypto::new()
            .map_err(|e| TrustChainError::CryptoError {
                reason: format!("Failed to initialize FALCON-1024 for CRL: {}", e),
            })?;

        info!(
            "CRL generator initialized: issuer={}, validity={}s",
            issuer_name,
            validity_period
                .unwrap_or_else(|| Duration::from_secs(86400))
                .as_secs()
        );

        Ok(Self {
            store,
            signing_key,
            falcon,
            issuer_name,
            validity_period: validity_period.unwrap_or_else(|| Duration::from_secs(86400)),
            crl_number: AtomicU64::new(0),
        })
    }

    /// Generate a new CRL containing all currently revoked certificates.
    ///
    /// Scans the certificate store, builds the revoked entries list,
    /// increments the CRL number, and signs the result with FALCON-1024.
    pub async fn generate_crl(&self) -> TrustChainResult<CertificateRevocationList> {
        let now = SystemTime::now();
        let next_update = now + self.validity_period;
        let crl_number = self.crl_number.fetch_add(1, Ordering::SeqCst);

        // Collect all revoked certificates from the store
        let revoked_certs = self.store.get_revoked_certificates();

        let revoked_entries: Vec<RevokedCertEntry> = revoked_certs
            .iter()
            .filter_map(|cert| {
                if let CertificateStatus::Revoked { reason, revoked_at } = &cert.status {
                    Some(RevokedCertEntry {
                        serial_number: cert.serial_number.clone(),
                        revocation_date: *revoked_at,
                        reason: parse_revocation_reason(reason),
                    })
                } else {
                    None
                }
            })
            .collect();

        debug!(
            "CRL generation: {} revoked entries, crl_number={}",
            revoked_entries.len(),
            crl_number
        );

        // Build signing payload
        let payload = self.build_signing_payload(
            &revoked_entries,
            now,
            next_update,
            crl_number,
        );

        // Sign with FALCON-1024
        let signature = self
            .falcon
            .sign(&payload, &self.signing_key)
            .await
            .map_err(|e| TrustChainError::CryptoError {
                reason: format!("CRL signing failed: {}", e),
            })?;

        info!(
            "CRL generated: issuer={}, entries={}, crl_number={}",
            self.issuer_name,
            revoked_entries.len(),
            crl_number
        );

        Ok(CertificateRevocationList {
            version: 1, // v2
            issuer: self.issuer_name.clone(),
            this_update: now,
            next_update,
            revoked_certificates: revoked_entries,
            signature,
            crl_number,
        })
    }

    /// Get the current CRL number (next CRL will have this number).
    pub fn current_crl_number(&self) -> u64 {
        self.crl_number.load(Ordering::SeqCst)
    }

    /// Build a deterministic byte payload for CRL signing.
    fn build_signing_payload(
        &self,
        entries: &[RevokedCertEntry],
        this_update: SystemTime,
        next_update: SystemTime,
        crl_number: u64,
    ) -> Vec<u8> {
        let this_secs = this_update
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let next_secs = next_update
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut payload = Vec::with_capacity(256);
        payload.extend_from_slice(b"CRL:");
        payload.extend_from_slice(self.issuer_name.as_bytes());
        payload.extend_from_slice(b":");
        payload.extend_from_slice(&crl_number.to_le_bytes());
        payload.extend_from_slice(b":");
        payload.extend_from_slice(&this_secs.to_le_bytes());
        payload.extend_from_slice(b":");
        payload.extend_from_slice(&next_secs.to_le_bytes());

        for entry in entries {
            payload.extend_from_slice(b"|");
            payload.extend_from_slice(entry.serial_number.as_bytes());
        }

        payload
    }
}

/// Distributes the latest CRL to consumers.
///
/// Stores the most recently published CRL and provides query access
/// for revocation checking without hitting the certificate store directly.
///
/// Transport-agnostic: CRL data is serializable via serde and can be
/// distributed over STOQ streams, gossip protocol, or any other transport.
/// No HTTP-specific assumptions exist in this layer.
pub struct CrlDistributor {
    /// The latest published CRL.
    latest_crl: Arc<RwLock<Option<CertificateRevocationList>>>,
}

impl CrlDistributor {
    /// Create a new CRL distributor with no initial CRL.
    pub fn new() -> Self {
        Self {
            latest_crl: Arc::new(RwLock::new(None)),
        }
    }

    /// Publish a new CRL, replacing the previous one.
    pub async fn publish_crl(&self, crl: CertificateRevocationList) {
        info!(
            "Publishing CRL: crl_number={}, entries={}",
            crl.crl_number,
            crl.revoked_certificates.len()
        );
        let mut guard = self.latest_crl.write().await;
        *guard = Some(crl);
    }

    /// Get the currently published CRL, if any.
    pub async fn get_current_crl(&self) -> Option<CertificateRevocationList> {
        let guard = self.latest_crl.read().await;
        guard.clone()
    }

    /// Check whether a certificate serial number appears in the current CRL.
    pub async fn is_certificate_revoked(&self, serial: &str) -> bool {
        let guard = self.latest_crl.read().await;
        match guard.as_ref() {
            Some(crl) => crl
                .revoked_certificates
                .iter()
                .any(|entry| entry.serial_number == serial),
            None => {
                warn!("CRL revocation check: no CRL published yet");
                false
            }
        }
    }
}

impl Default for CrlDistributor {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a reason string into a `RevocationReason`.
fn parse_revocation_reason(reason: &str) -> RevocationReason {
    match reason.to_lowercase().as_str() {
        "keycompromise" | "key_compromise" | "key compromise" => RevocationReason::KeyCompromise,
        "cacompromise" | "ca_compromise" | "ca compromise" => RevocationReason::CaCompromise,
        "affiliationchanged" | "affiliation_changed" | "affiliation changed" => {
            RevocationReason::AffiliationChanged
        }
        "superseded" => RevocationReason::Superseded,
        "cessationofoperation" | "cessation_of_operation" | "cessation of operation" => {
            RevocationReason::CessationOfOperation
        }
        "privilegewithdrawn" | "privilege_withdrawn" | "privilege withdrawn" => {
            RevocationReason::PrivilegeWithdrawn
        }
        _ => RevocationReason::Unspecified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyUsage;

    /// Helper: create store, generator, and distributor for testing.
    async fn setup() -> (Arc<CertificateStore>, CrlGenerator, CrlDistributor) {
        let store = Arc::new(
            CertificateStore::new()
                .await
                .expect("test: failed to create store"),
        );

        let falcon = FalconCrypto::new().expect("test: failed to init FALCON");
        let keypair = falcon
            .generate_keypair(KeyUsage::CertificateAuthority)
            .await
            .expect("test: failed to generate keypair");

        let generator = CrlGenerator::new(
            Arc::clone(&store),
            keypair.private_key,
            "CN=Test CA".to_string(),
            Some(Duration::from_secs(3600)),
        )
        .expect("test: failed to create CRL generator");

        let distributor = CrlDistributor::new();

        (store, generator, distributor)
    }

    fn create_test_certificate(
        serial: &str,
        status: CertificateStatus,
    ) -> super::super::IssuedCertificate {
        use crate::proof_of_state::StateProof;

        super::super::IssuedCertificate {
            serial_number: serial.to_string(),
            certificate_der: vec![0u8; 32],
            certificate_pem: String::new(),
            chain_pem: String::new(),
            fingerprint: [0u8; 32],
            common_name: format!("test-{}", serial),
            issued_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(86400),
            issuer_ca_id: "test-ca".to_string(),
            state_proof: StateProof::default_for_testing(),
            status,
            metadata: super::super::CertificateMetadata::default(),
        }
    }

    #[tokio::test]
    async fn test_crl_generation_empty() {
        let (_store, generator, _distributor) = setup().await;

        let crl = generator
            .generate_crl()
            .await
            .expect("test: CRL generation failed");

        assert_eq!(crl.version, 1);
        assert_eq!(crl.issuer, "CN=Test CA");
        assert_eq!(crl.crl_number, 0);
        assert!(crl.revoked_certificates.is_empty());
        assert!(!crl.signature.signature_bytes.is_empty());
        assert!(crl.next_update > crl.this_update);
    }

    #[tokio::test]
    async fn test_crl_generation_with_revoked_certs() {
        let (store, generator, _distributor) = setup().await;

        // Add some certificates: 2 revoked, 1 valid
        let valid = create_test_certificate("serial-valid", CertificateStatus::Valid);
        let revoked1 = create_test_certificate(
            "serial-rev-1",
            CertificateStatus::Revoked {
                reason: "KeyCompromise".to_string(),
                revoked_at: SystemTime::now(),
            },
        );
        let revoked2 = create_test_certificate(
            "serial-rev-2",
            CertificateStatus::Revoked {
                reason: "Superseded".to_string(),
                revoked_at: SystemTime::now(),
            },
        );

        store.store_certificate(&valid).await.expect("test: store failed");
        store.store_certificate(&revoked1).await.expect("test: store failed");
        store.store_certificate(&revoked2).await.expect("test: store failed");

        let crl = generator
            .generate_crl()
            .await
            .expect("test: CRL generation failed");

        assert_eq!(crl.revoked_certificates.len(), 2);

        let serials: Vec<&str> = crl
            .revoked_certificates
            .iter()
            .map(|e| e.serial_number.as_str())
            .collect();
        assert!(serials.contains(&"serial-rev-1"));
        assert!(serials.contains(&"serial-rev-2"));
        assert!(!serials.contains(&"serial-valid"));
    }

    #[tokio::test]
    async fn test_crl_number_increments() {
        let (_store, generator, _distributor) = setup().await;

        let crl1 = generator.generate_crl().await.expect("test: CRL gen failed");
        let crl2 = generator.generate_crl().await.expect("test: CRL gen failed");
        let crl3 = generator.generate_crl().await.expect("test: CRL gen failed");

        assert_eq!(crl1.crl_number, 0);
        assert_eq!(crl2.crl_number, 1);
        assert_eq!(crl3.crl_number, 2);
        assert_eq!(generator.current_crl_number(), 3);
    }

    #[tokio::test]
    async fn test_distributor_publish_and_get() {
        let (_store, generator, distributor) = setup().await;

        // No CRL published yet
        assert!(distributor.get_current_crl().await.is_none());

        let crl = generator.generate_crl().await.expect("test: CRL gen failed");
        distributor.publish_crl(crl.clone()).await;

        let retrieved = distributor
            .get_current_crl()
            .await
            .expect("test: expected CRL after publish");

        assert_eq!(retrieved.crl_number, crl.crl_number);
        assert_eq!(retrieved.issuer, crl.issuer);
    }

    #[tokio::test]
    async fn test_distributor_revocation_check() {
        let (store, generator, distributor) = setup().await;

        let revoked = create_test_certificate(
            "serial-check",
            CertificateStatus::Revoked {
                reason: "Unspecified".to_string(),
                revoked_at: SystemTime::now(),
            },
        );
        store.store_certificate(&revoked).await.expect("test: store failed");

        // Before CRL is published, check returns false
        assert!(!distributor.is_certificate_revoked("serial-check").await);

        // Generate and publish CRL
        let crl = generator.generate_crl().await.expect("test: CRL gen failed");
        distributor.publish_crl(crl).await;

        // Now check succeeds
        assert!(distributor.is_certificate_revoked("serial-check").await);
        assert!(!distributor.is_certificate_revoked("serial-not-revoked").await);
    }

    #[tokio::test]
    async fn test_crl_signature_present() {
        let (store, generator, _distributor) = setup().await;

        let revoked = create_test_certificate(
            "serial-sig",
            CertificateStatus::Revoked {
                reason: "CaCompromise".to_string(),
                revoked_at: SystemTime::now(),
            },
        );
        store.store_certificate(&revoked).await.expect("test: store failed");

        let crl = generator.generate_crl().await.expect("test: CRL gen failed");

        assert!(!crl.signature.signature_bytes.is_empty());
        assert_eq!(crl.signature.algorithm, "FALCON-1024");
    }
}
