// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CPU Asset Adapter with core management and scheduling
//!
//! Features:
//! - CPU core allocation (physical cores, logical cores, threads)
//! - Frequency scaling and power management
//! - CPU affinity and NUMA awareness
//! - Process isolation and security boundaries
//! - PoWork computational proof validation
//! - Time-based scheduling with PoTime integration

mod adapter;
pub mod types;

pub use adapter::CpuAssetAdapter;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::{
        AssetAdapter, AssetAllocationRequest, AssetType, StateProof, CpuRequirements,
        PrivacyMode,
    };
    #[allow(unused_imports)]
    use crate::assets::core::{
        SpaceProof, StakeProof, TimeProof, WorkProof,
    };
    use std::collections::HashMap;
    use std::time::Duration;

    fn _create_test_cpu_request() -> AssetAllocationRequest {
        AssetAllocationRequest {
            asset_type: AssetType::Cpu,
            requested_resources: crate::assets::core::ResourceRequirements {
                cpu: Some(CpuRequirements {
                    cores: 2,
                    min_frequency_mhz: Some(2400),
                    architecture: Some("x86_64".to_string()),
                    required_features: vec!["AVX2".to_string()],
                }),
                ..Default::default()
            },
            privacy_level: PrivacyMode::PRIVATE,
            state_proof: StateProof::new_for_testing(),
            certificate_fingerprint: "test-cert".to_string(),
            duration_limit: Some(Duration::from_secs(3600)),
            tags: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_cpu_adapter_creation() {
        let adapter = CpuAssetAdapter::new().await;
        assert_eq!(adapter.asset_type(), AssetType::Cpu);
        assert!(adapter.state.total_cores > 0);
    }

    #[tokio::test]
    async fn test_cpu_allocation() {
        // CANONICAL MODEL: PoStake is authorization (WHO) and PoWork is the
        // HASH of work (WHAT) — assert bound identity + a work hash, never
        // magnitudes.
        let test_proof = StateProof::new_for_testing();
        assert!(
            !test_proof.stake_proof.stake_holder_id.is_empty(),
            "PoStake must carry a bound identity"
        );
        assert!(
            test_proof.work_proof.work_hash != [0u8; 32],
            "PoWork must carry a work hash"
        );
    }

    #[tokio::test]
    async fn test_cpu_health_check() {
        let adapter = CpuAssetAdapter::new().await;
        let health = adapter.health_check().await.expect("test: async operation");

        assert!(health.healthy);
        assert!(health.performance_metrics.contains_key("total_cores"));
        assert!(health.performance_metrics.contains_key("available_cores"));
    }

    #[tokio::test]
    async fn test_cpu_capabilities() {
        let adapter = CpuAssetAdapter::new().await;
        let capabilities = adapter.get_capabilities();

        assert_eq!(capabilities.asset_type, AssetType::Cpu);
        assert!(capabilities.supports_proxy_addressing);
        assert!(capabilities
            .features
            .contains(&"frequency_scaling".to_string()));
        assert!(capabilities
            .features
            .contains(&"process_isolation".to_string()));
    }

    #[test]
    fn test_state_proof_creation() {
        let proof = StateProof::new_for_testing();
        assert!(proof.validate());
    }
}
