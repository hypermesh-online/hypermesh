// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Field Device Bootstrap with Intermittent Connectivity
//!
//! Supports devices that have unreliable network access during initial
//! certificate enrollment. The bootstrap flow is:
//!
//! 1. **Provisional**: Device generates a self-signed cert with short validity.
//! 2. **Enrolling**: When connectivity returns, device submits a CA enrollment.
//! 3. **Enrolled**: CA issues a signed cert; provisional cert is replaced.
//!
//! If connectivity is lost during enrollment, the device retains its
//! provisional cert and retries enrollment on the next connection.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::errors::{Result as TrustChainResult, TrustChainError};

/// Bootstrap state for a field device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootstrapState {
    /// Device has a locally generated provisional certificate.
    Provisional,
    /// Enrollment request submitted to CA, awaiting response.
    Enrolling,
    /// CA-signed certificate received and installed.
    Enrolled,
}

impl std::fmt::Display for BootstrapState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provisional => write!(f, "Provisional"),
            Self::Enrolling => write!(f, "Enrolling"),
            Self::Enrolled => write!(f, "Enrolled"),
        }
    }
}

/// Configuration for field bootstrap.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldBootstrapConfig {
    /// Validity duration for provisional self-signed certificates.
    pub provisional_validity: Duration,
    /// Maximum number of enrollment retries before requiring manual intervention.
    pub max_enrollment_retries: u32,
}

impl Default for FieldBootstrapConfig {
    fn default() -> Self {
        Self {
            provisional_validity: Duration::from_secs(24 * 60 * 60), // 24 hours
            max_enrollment_retries: 5,
        }
    }
}

/// A provisional certificate generated locally by a field device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProvisionalCertificate {
    /// Unique device identifier.
    pub device_id: String,
    /// Self-signed certificate data (opaque bytes).
    pub certificate_data: Vec<u8>,
    /// When the provisional cert was created.
    pub created_at: SystemTime,
    /// When the provisional cert expires.
    pub expires_at: SystemTime,
}

/// An enrollment request queued for CA submission.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnrollmentRequest {
    /// Unique enrollment request identifier.
    pub request_id: String,
    /// Device identifier requesting enrollment.
    pub device_id: String,
    /// Common name for the certificate.
    pub common_name: String,
    /// When the request was created.
    pub created_at: SystemTime,
    /// Number of submission attempts.
    pub retry_count: u32,
}

/// Record for a single device's bootstrap lifecycle.
#[derive(Clone, Debug)]
struct DeviceRecord {
    state: BootstrapState,
    provisional: Option<ProvisionalCertificate>,
    enrollment: Option<EnrollmentRequest>,
    signed_cert_data: Option<Vec<u8>>,
    retry_count: u32,
}

/// Manages the bootstrap lifecycle for field devices.
pub struct FieldBootstrap {
    config: FieldBootstrapConfig,
    /// Device bootstrap records keyed by device_id.
    devices: Arc<RwLock<HashMap<String, DeviceRecord>>>,
}

impl FieldBootstrap {
    /// Create a new field bootstrap manager.
    pub fn new() -> Self {
        Self::with_config(FieldBootstrapConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: FieldBootstrapConfig) -> Self {
        Self {
            config,
            devices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a provisional self-signed certificate for a device.
    ///
    /// This is the first step: the device creates a temporary identity that
    /// allows basic local operations before CA enrollment.
    pub async fn generate_provisional(
        &self,
        device_id: &str,
        common_name: &str,
    ) -> TrustChainResult<ProvisionalCertificate> {
        let now = SystemTime::now();
        let expires_at = now + self.config.provisional_validity;

        // Build a simple self-signed cert representation
        // In production this would use rcgen, but we keep it opaque here
        let cert_data = Self::build_provisional_cert(device_id, common_name, now, expires_at);

        let provisional = ProvisionalCertificate {
            device_id: device_id.to_string(),
            certificate_data: cert_data,
            created_at: now,
            expires_at,
        };

        let mut devices = self.devices.write().await;
        devices.insert(
            device_id.to_string(),
            DeviceRecord {
                state: BootstrapState::Provisional,
                provisional: Some(provisional.clone()),
                enrollment: None,
                signed_cert_data: None,
                retry_count: 0,
            },
        );

        info!(
            "Generated provisional certificate for device '{}', valid for {}s",
            device_id,
            self.config.provisional_validity.as_secs()
        );

        Ok(provisional)
    }

    /// Queue a CA enrollment request for when connectivity is available.
    ///
    /// Transitions the device from Provisional to Enrolling.
    pub async fn queue_enrollment(
        &self,
        device_id: &str,
        common_name: &str,
    ) -> TrustChainResult<EnrollmentRequest> {
        let mut devices = self.devices.write().await;
        let record = devices.get_mut(device_id).ok_or_else(|| {
            TrustChainError::InvalidRequest {
                reason: format!("Device '{}' not registered for bootstrap", device_id),
            }
        })?;

        if record.state == BootstrapState::Enrolled {
            return Err(TrustChainError::InvalidRequest {
                reason: format!("Device '{}' is already enrolled", device_id),
            });
        }

        let request = EnrollmentRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            device_id: device_id.to_string(),
            common_name: common_name.to_string(),
            created_at: SystemTime::now(),
            retry_count: record.retry_count,
        };

        record.state = BootstrapState::Enrolling;
        record.enrollment = Some(request.clone());

        info!(
            "Queued enrollment for device '{}' (attempt #{})",
            device_id, record.retry_count
        );

        Ok(request)
    }

    /// Complete enrollment with a CA-signed certificate.
    ///
    /// Transitions from Enrolling to Enrolled and replaces the provisional cert.
    pub async fn complete_enrollment(
        &self,
        device_id: &str,
        signed_certificate: Vec<u8>,
    ) -> TrustChainResult<()> {
        let mut devices = self.devices.write().await;
        let record = devices.get_mut(device_id).ok_or_else(|| {
            TrustChainError::InvalidRequest {
                reason: format!("Device '{}' not registered for bootstrap", device_id),
            }
        })?;

        if record.state != BootstrapState::Enrolling {
            return Err(TrustChainError::InvalidRequest {
                reason: format!(
                    "Device '{}' is in state '{}', expected 'Enrolling'",
                    device_id, record.state
                ),
            });
        }

        record.state = BootstrapState::Enrolled;
        record.signed_cert_data = Some(signed_certificate);
        record.provisional = None; // Clear provisional

        info!("Device '{}' enrollment complete", device_id);
        Ok(())
    }

    /// Handle a connectivity interruption during enrollment.
    ///
    /// Reverts from Enrolling back to Provisional so the device can retry.
    /// Increments the retry counter.
    pub async fn handle_connectivity_loss(
        &self,
        device_id: &str,
    ) -> TrustChainResult<()> {
        let mut devices = self.devices.write().await;
        let record = devices.get_mut(device_id).ok_or_else(|| {
            TrustChainError::InvalidRequest {
                reason: format!("Device '{}' not registered", device_id),
            }
        })?;

        if record.state != BootstrapState::Enrolling {
            return Err(TrustChainError::InvalidRequest {
                reason: format!(
                    "Device '{}' is not enrolling (state: {})",
                    device_id, record.state
                ),
            });
        }

        record.retry_count += 1;
        if record.retry_count > self.config.max_enrollment_retries {
            return Err(TrustChainError::InvalidRequest {
                reason: format!(
                    "Device '{}' exceeded max enrollment retries ({})",
                    device_id, self.config.max_enrollment_retries
                ),
            });
        }

        record.state = BootstrapState::Provisional;
        record.enrollment = None;

        warn!(
            "Device '{}' lost connectivity during enrollment, reverting to provisional (retry #{})",
            device_id, record.retry_count
        );

        Ok(())
    }

    /// Get the current bootstrap state for a device.
    pub async fn get_state(&self, device_id: &str) -> Option<BootstrapState> {
        self.devices
            .read()
            .await
            .get(device_id)
            .map(|r| r.state.clone())
    }

    /// Get the number of enrollment retries for a device.
    pub async fn get_retry_count(&self, device_id: &str) -> u32 {
        self.devices
            .read()
            .await
            .get(device_id)
            .map(|r| r.retry_count)
            .unwrap_or(0)
    }

    /// Build a simple provisional certificate representation.
    fn build_provisional_cert(
        device_id: &str,
        common_name: &str,
        created_at: SystemTime,
        expires_at: SystemTime,
    ) -> Vec<u8> {
        // In production, this would generate a proper self-signed X.509 cert.
        // Here we build a deterministic hash-based token.
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"PROVISIONAL:");
        hasher.update(device_id.as_bytes());
        hasher.update(b":");
        hasher.update(common_name.as_bytes());
        hasher.update(b":");
        let created_secs = created_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let expires_secs = expires_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        hasher.update(&created_secs.to_le_bytes());
        hasher.update(&expires_secs.to_le_bytes());
        hasher.finalize().as_bytes().to_vec()
    }
}

impl Default for FieldBootstrap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_full_bootstrap_flow() {
        let bootstrap = FieldBootstrap::new();

        // Step 1: Generate provisional
        let prov = bootstrap
            .generate_provisional("device-001", "field.local")
            .await
            .expect("test: provisional");
        assert_eq!(prov.device_id, "device-001");
        assert!(!prov.certificate_data.is_empty());
        assert_eq!(
            bootstrap.get_state("device-001").await,
            Some(BootstrapState::Provisional)
        );

        // Step 2: Queue enrollment
        let enrollment = bootstrap
            .queue_enrollment("device-001", "field.local")
            .await
            .expect("test: enroll");
        assert_eq!(enrollment.device_id, "device-001");
        assert_eq!(
            bootstrap.get_state("device-001").await,
            Some(BootstrapState::Enrolling)
        );

        // Step 3: Complete enrollment
        let signed_cert = vec![0xCA; 64];
        bootstrap
            .complete_enrollment("device-001", signed_cert)
            .await
            .expect("test: complete");
        assert_eq!(
            bootstrap.get_state("device-001").await,
            Some(BootstrapState::Enrolled)
        );
    }

    #[tokio::test]
    async fn test_connectivity_interruption_and_retry() {
        let bootstrap = FieldBootstrap::new();

        // Generate provisional
        bootstrap
            .generate_provisional("device-002", "sensor.local")
            .await
            .expect("test: provisional");

        // Start enrollment
        bootstrap
            .queue_enrollment("device-002", "sensor.local")
            .await
            .expect("test: enroll");
        assert_eq!(
            bootstrap.get_state("device-002").await,
            Some(BootstrapState::Enrolling)
        );

        // Connectivity lost
        bootstrap
            .handle_connectivity_loss("device-002")
            .await
            .expect("test: connectivity loss");
        assert_eq!(
            bootstrap.get_state("device-002").await,
            Some(BootstrapState::Provisional)
        );
        assert_eq!(bootstrap.get_retry_count("device-002").await, 1);

        // Retry enrollment
        bootstrap
            .queue_enrollment("device-002", "sensor.local")
            .await
            .expect("test: re-enroll");
        assert_eq!(
            bootstrap.get_state("device-002").await,
            Some(BootstrapState::Enrolling)
        );

        // This time it succeeds
        bootstrap
            .complete_enrollment("device-002", vec![0xBB; 32])
            .await
            .expect("test: complete");
        assert_eq!(
            bootstrap.get_state("device-002").await,
            Some(BootstrapState::Enrolled)
        );
    }

    #[tokio::test]
    async fn test_max_retries_enforced() {
        let config = FieldBootstrapConfig {
            max_enrollment_retries: 2,
            ..Default::default()
        };
        let bootstrap = FieldBootstrap::with_config(config);

        bootstrap
            .generate_provisional("device-retry", "test.local")
            .await
            .expect("test: provisional");

        // Two connectivity losses allowed
        for _ in 0..2 {
            bootstrap
                .queue_enrollment("device-retry", "test.local")
                .await
                .expect("test: enroll");
            bootstrap
                .handle_connectivity_loss("device-retry")
                .await
                .expect("test: loss");
        }

        // Third enrollment attempt
        bootstrap
            .queue_enrollment("device-retry", "test.local")
            .await
            .expect("test: enroll 3");

        // Third connectivity loss exceeds limit
        let err = bootstrap
            .handle_connectivity_loss("device-retry")
            .await;
        assert!(err.is_err(), "Should exceed max retries");
    }

    #[tokio::test]
    async fn test_unregistered_device_rejected() {
        let bootstrap = FieldBootstrap::new();

        let err = bootstrap
            .queue_enrollment("unknown-device", "test.local")
            .await;
        assert!(err.is_err(), "Unregistered device should be rejected");
    }

    #[tokio::test]
    async fn test_already_enrolled_rejected() {
        let bootstrap = FieldBootstrap::new();

        bootstrap
            .generate_provisional("enrolled-dev", "test.local")
            .await
            .expect("test: provisional");
        bootstrap
            .queue_enrollment("enrolled-dev", "test.local")
            .await
            .expect("test: enroll");
        bootstrap
            .complete_enrollment("enrolled-dev", vec![0xAA; 16])
            .await
            .expect("test: complete");

        // Re-enrollment should fail
        let err = bootstrap
            .queue_enrollment("enrolled-dev", "test.local")
            .await;
        assert!(err.is_err(), "Already enrolled device should be rejected");
    }
}
