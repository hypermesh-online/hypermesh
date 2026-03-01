// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Trust Integration with Byzantine Fault Detection
//!
//! Integrates TrustChain certificate authority with HyperMesh asset system,
//! providing binary authentication, Byzantine fault detection, and remote proxy management.

pub mod operations;
pub mod types;

// Re-export all public types for backward compatibility
pub use operations::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_trust_validator_creation() {
        let config = TrustValidatorConfig::default();
        let _validator = HyperMeshTrustValidator::new(config).await;
    }

    #[test]
    fn test_asset_id_creation() {
        let asset_id = AssetId {
            uuid: Uuid::new_v4(),
            asset_type: AssetType::Cpu,
            network_id: "test-network".to_string(),
        };
        assert_eq!(asset_id.asset_type, AssetType::Cpu);
        assert_eq!(asset_id.network_id, "test-network");
    }

    #[test]
    fn test_authentication_status() {
        let status = AuthenticationStatus {
            authenticated: true,
            certificate_valid: true,
            consensus_verified: true,
            last_checked: SystemTime::now(),
            expiry: SystemTime::now() + std::time::Duration::from_secs(3600),
        };
        assert!(status.authenticated);
        assert!(status.certificate_valid);
        assert!(status.consensus_verified);
    }

    #[test]
    fn test_byzantine_fault_types() {
        let fault_types = [
            ByzantineFaultType::DoubleSigning,
            ByzantineFaultType::EquivocationAttack,
            ByzantineFaultType::NothingAtStake,
            ByzantineFaultType::LongRangeAttack,
        ];
        assert_eq!(fault_types.len(), 4);
    }

    #[test]
    fn test_proxy_connection_types() {
        let proxy_types = [
            ProxyType::Direct,
            ProxyType::Encrypted,
            ProxyType::Federated,
            ProxyType::Anonymous,
        ];
        assert_eq!(proxy_types.len(), 4);
    }

    #[test]
    fn test_alert_thresholds() {
        let thresholds = AlertThresholds {
            byzantine_confidence: 0.8,
            performance_degradation: 0.5,
            availability_threshold: 0.95,
        };
        assert!(thresholds.byzantine_confidence > 0.7);
        assert!(thresholds.availability_threshold > 0.9);
    }
}
