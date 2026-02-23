// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network Asset Adapter with bandwidth allocation and traffic management
//!
//! Features:
//! - Network interface management
//! - Bandwidth allocation and QoS
//! - Traffic shaping and prioritization
//! - IPv6-only networking support
//! - Network security and isolation
//! - Latency and packet loss monitoring

pub mod types;
mod adapter;

pub use types::*;
pub use adapter::NetworkAssetAdapter;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};
    use super::*;
    use crate::assets::core::{
        AssetAdapter, AssetType, AssetAllocationRequest,
        ConsensusProof, PrivacyMode, NetworkRequirements,
        SpaceProof, StakeProof, WorkProof, TimeProof,
        WorkloadType, WorkState, AssetCategory, BaseSystemType,
    };

    async fn create_test_network_request() -> AssetAllocationRequest {
        AssetAllocationRequest {
            asset_type: AssetType::Network,
            requested_resources: crate::assets::core::ResourceRequirements {
                network_usage: Some(NetworkRequirements {
                    bandwidth_mbps: 1000,
                    max_latency_us: Some(1000),
                    max_packet_loss_percent: Some(0.1),
                    protocols: vec!["TCP".to_string(), "UDP".to_string()],
                }),
                ..Default::default()
            },
            privacy_level: PrivacyMode::PRIVATE,
            consensus_proof: ConsensusProof::new(
                StakeProof {
                    stake_holder: "test-holder".to_string(),
                    stake_holder_id: "test-holder-id".to_string(),
                    stake_amount: 50,
                    stake_timestamp: SystemTime::now(),
                },
                TimeProof {
                    network_time_offset: Duration::from_millis(500),
                    time_verification_timestamp: SystemTime::now(),
                    nonce: 42,
                    proof_hash: vec![5, 6, 7, 8],
                },
                SpaceProof {
                    node_id: "test-node".to_string(),
                    storage_path: "/test/network".to_string(),
                    total_size: 1000,
                    total_storage: 2000,
                    file_hash: "test_network_hash".to_string(),
                    proof_timestamp: SystemTime::now(),
                },
                WorkProof {
                    owner_id: "test-worker".to_string(),
                    workload_id: "test-workload".to_string(),
                    pid: 12345,
                    computational_power: 20,
                    workload_type: WorkloadType::Network,
                    work_state: WorkState::Completed,
                    work_challenges: vec!["network_challenge".to_string()],
                    proof_timestamp: SystemTime::now(),
                },
            ),
            certificate_fingerprint: "test-cert".to_string(),
            duration_limit: Some(Duration::from_secs(3600)),
            tags: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_network_adapter_creation() {
        let adapter = NetworkAssetAdapter::new().await;
        assert_eq!(adapter.asset_type(), AssetType::Network);
    }

    #[tokio::test]
    async fn test_network_allocation() {
        let adapter = NetworkAssetAdapter::new().await;
        let request = create_test_network_request().await;

        let allocation = adapter.allocate_asset(&request).await.unwrap();
        assert!(matches!(allocation.asset_id.category, AssetCategory::BaseSystem(BaseSystemType::Network)));

        adapter.deallocate_asset(&allocation.asset_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_network_health_check() {
        let adapter = NetworkAssetAdapter::new().await;
        let health = adapter.health_check().await.unwrap();

        assert!(health.healthy);
        assert!(health.performance_metrics.contains_key("total_bandwidth_gbps"));
        assert!(health.performance_metrics.contains_key("total_interfaces"));
    }

    #[tokio::test]
    async fn test_network_capabilities() {
        let adapter = NetworkAssetAdapter::new().await;
        let capabilities = adapter.get_capabilities();

        assert_eq!(capabilities.asset_type, AssetType::Network);
        assert!(capabilities.supports_proxy_addressing);
        assert!(capabilities.features.contains(&"ipv6_only".to_string()));
        assert!(capabilities.features.contains(&"qos_management".to_string()));
    }
}
