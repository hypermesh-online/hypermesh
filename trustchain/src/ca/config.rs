// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Authority configuration and runtime metrics.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::proof_of_state::{HyperMeshClientConfig, StateRequirements};

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
    /// State proof requirements
    pub state_requirements: StateRequirements,
    /// HyperMesh Proof of State client configuration
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
            state_requirements: StateRequirements::localhost_testing(),
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
            state_requirements: StateRequirements::localhost_testing(),
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
            state_requirements: StateRequirements::production(),
            hypermesh_client_config: HyperMeshClientConfig::production(
                "https://hypermesh.hypermesh.online:8080".to_string(),
            ),
        }
    }
}

/// CA metrics for monitoring (Item 2.8: real certificate operation metrics)
#[derive(Default)]
pub struct CAMetrics {
    pub certificates_issued: std::sync::atomic::AtomicU64,
    /// Certificates revoked (Item 2.8)
    pub certificates_revoked: std::sync::atomic::AtomicU64,
    pub state_validations: std::sync::atomic::AtomicU64,
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
            state_validations: std::sync::atomic::AtomicU64::new(
                self.state_validations.load(Relaxed),
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
            .field("state_validations", &self.state_validations.load(Relaxed))
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
        state.serialize_field("state_validations", &self.state_validations.load(Relaxed))?;
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
            state_validations: u64,
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
            state_validations: std::sync::atomic::AtomicU64::new(data.state_validations),
            ct_log_entries: std::sync::atomic::AtomicU64::new(data.ct_log_entries),
            average_issuance_time_ms: std::sync::atomic::AtomicU64::new(data.average_issuance_time_ms),
            validation_latency_ms: std::sync::atomic::AtomicU64::new(data.validation_latency_ms),
            performance_violations: std::sync::atomic::AtomicU64::new(data.performance_violations),
        })
    }
}
