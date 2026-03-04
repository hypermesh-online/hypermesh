// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Container asset adapter with resource orchestration.

pub mod adapter;
pub mod types;

// Re-export all public types
pub use adapter::ContainerAssetAdapter;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::{
        AssetAdapter, AssetAllocationRequest, AssetCategory, AssetType, BaseSystemType,
        StateProof, ContainerRequirements, PortMapping, PrivacyMode, SpaceProof, StakeProof,
        TimeProof, VolumeMount, WorkProof, WorkState, WorkloadType,
    };
    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};

    async fn create_test_container_request() -> AssetAllocationRequest {
        AssetAllocationRequest {
            asset_type: AssetType::Container,
            requested_resources: crate::assets::core::ResourceRequirements {
                container: Some(ContainerRequirements {
                    image: "nginx:latest".to_string(),
                    cpu_limit: 1.0,
                    memory_limit_bytes: 512 * 1024 * 1024,
                    environment: {
                        let mut env = HashMap::new();
                        env.insert("ENV".to_string(), "production".to_string());
                        env
                    },
                    volumes: vec![VolumeMount {
                        source: "/host/data".to_string(),
                        target: "/container/data".to_string(),
                        read_only: false,
                    }],
                    ports: vec![PortMapping {
                        container_port: 80,
                        host_port: None,
                        protocol: "TCP".to_string(),
                    }],
                }),
                ..Default::default()
            },
            privacy_level: PrivacyMode::PRIVATE,
            state_proof: StateProof::new(
                StakeProof {
                    stake_holder: "test-holder".to_string(),
                    stake_holder_id: "test-holder-id".to_string(),
                    stake_amount: 75,
                    stake_timestamp: SystemTime::now(),
                },
                TimeProof {
                    network_time_offset: Duration::from_secs(10),
                    time_verification_timestamp: SystemTime::now(),
                    nonce: 42,
                    proof_hash: vec![5, 6, 7, 8],
                },
                SpaceProof {
                    node_id: "test-node".to_string(),
                    storage_path: "/test/container".to_string(),
                    total_size: 512 * 1024 * 1024,
                    total_storage: 1024 * 1024 * 1024,
                    file_hash: "test_container_hash".to_string(),
                    proof_timestamp: SystemTime::now(),
                },
                WorkProof {
                    owner_id: "test-worker".to_string(),
                    workload_id: "test-workload".to_string(),
                    pid: 12345,
                    computational_power: 50,
                    workload_type: WorkloadType::Compute,
                    work_state: WorkState::Completed,
                    work_challenges: vec!["container_challenge".to_string()],
                    proof_timestamp: SystemTime::now(),
                },
            ),
            certificate_fingerprint: "test-cert".to_string(),
            duration_limit: Some(Duration::from_secs(3600)),
            tags: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_container_adapter_creation() {
        let adapter = ContainerAssetAdapter::new().await;
        assert_eq!(adapter.asset_type(), AssetType::Container);
    }

    #[tokio::test]
    async fn test_container_allocation() {
        let adapter = ContainerAssetAdapter::new().await;
        let request = create_test_container_request().await;

        let allocation = adapter.allocate_asset(&request).await.expect("test");
        assert!(matches!(
            allocation.asset_id.category,
            AssetCategory::BaseSystem(BaseSystemType::Container)
        ));

        adapter
            .deallocate_asset(&allocation.asset_id)
            .await
            .expect("test");
    }

    #[tokio::test]
    async fn test_container_health_check() {
        let adapter = ContainerAssetAdapter::new().await;
        let health = adapter.health_check().await.expect("test");

        assert!(health.healthy);
        assert!(health.performance_metrics.contains_key("active_containers"));
    }

    #[tokio::test]
    async fn test_container_capabilities() {
        let adapter = ContainerAssetAdapter::new().await;
        let capabilities = adapter.get_capabilities();

        assert_eq!(capabilities.asset_type, AssetType::Container);
        assert!(capabilities.supports_proxy_addressing);
        assert!(capabilities
            .features
            .contains(&"container_orchestration".to_string()));
        assert!(capabilities
            .features
            .contains(&"security_controls".to_string()));
    }
}
