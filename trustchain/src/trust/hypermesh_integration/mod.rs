// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Trust Integration with Byzantine Fault Detection
//!
//! Integrates TrustChain certificate authority with HyperMesh asset system,
//! providing trust validation, Byzantine fault detection, and remote proxy management.

pub mod types;
pub mod operations;

// Re-export all public types for backward compatibility
pub use types::*;
pub use operations::*;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use std::time::SystemTime;

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
    fn test_trust_score_components() {
        let trust_score = TrustScore {
            overall_score: 0.85,
            confidence: 0.9,
            components: TrustComponents {
                consensus_score: 0.9,
                reputation_score: 0.8,
                verification_score: 0.95,
                performance_score: 0.75,
                availability_score: 0.85,
            },
            last_updated: SystemTime::now(),
            expiry: SystemTime::now() + std::time::Duration::from_secs(3600),
        };
        assert!(trust_score.overall_score > 0.8);
        assert!(trust_score.confidence > 0.8);
    }

    #[test]
    fn test_byzantine_fault_types() {
        let fault_types = vec![
            ByzantineFaultType::DoubleSigning,
            ByzantineFaultType::EquivocationAttack,
            ByzantineFaultType::NothingAtStake,
            ByzantineFaultType::LongRangeAttack,
        ];
        assert_eq!(fault_types.len(), 4);
    }

    #[test]
    fn test_proxy_connection_types() {
        let proxy_types = vec![
            ProxyType::Direct,
            ProxyType::Encrypted,
            ProxyType::Federated,
            ProxyType::Anonymous,
        ];
        assert_eq!(proxy_types.len(), 4);
    }

    #[test]
    fn test_trust_levels() {
        let levels = vec![
            TrustLevel::Untrusted,
            TrustLevel::Low,
            TrustLevel::Medium,
            TrustLevel::High,
            TrustLevel::Verified,
        ];
        assert_eq!(levels.len(), 5);
    }

    #[test]
    fn test_alert_thresholds() {
        let thresholds = AlertThresholds {
            byzantine_confidence: 0.8,
            trust_score_degradation: 0.3,
            performance_degradation: 0.5,
            availability_threshold: 0.95,
        };
        assert!(thresholds.byzantine_confidence > 0.7);
        assert!(thresholds.availability_threshold > 0.9);
    }
}
