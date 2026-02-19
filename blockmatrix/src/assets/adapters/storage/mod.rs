// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Storage Asset Adapter with distributed sharding and encryption
//!
//! Features:
//! - Block device management (NVMe, SSD, HDD)
//! - Distributed storage pools with replication
//! - Content-aware sharding and deduplication
//! - Encryption at rest with Kyber quantum-resistant crypto
//! - Storage health monitoring and predictive maintenance
//! - PoSpace proof validation for storage commitment

// Module declarations
mod adapter;
mod allocation;
mod devices;
mod sharding;
mod encryption;
mod distribution;

// Re-exports for public API
pub use self::adapter::StorageAssetAdapter;
pub use self::allocation::{StorageAllocation, StoragePool, PoolHealthStatus, StorageUsageStats};
pub use self::devices::{StorageDevice, StorageStatus, StorageHealthMetrics, SmartData};
pub use self::sharding::{ShardingConfig, ShardingAlgorithm};
pub use self::encryption::create_kyber_encryption_key;
pub use self::distribution::generate_proxy_address;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;
    use crate::assets::core::{
        AssetType, AssetAllocationRequest, PrivacyMode, StorageRequirements, StorageType,
        SpaceProof, StakeProof, WorkProof, TimeProof, WorkloadType, WorkState, ConsensusProof,
    };
    use std::time::{Duration, SystemTime};

    async fn create_test_storage_request() -> AssetAllocationRequest {
        use sha2::{Sha256, Digest};

        // Create TimeProof with valid hash
        let network_time_offset = Duration::from_secs(10);
        let time_verification_timestamp = SystemTime::now();
        let nonce = 42u64;

        let mut hasher = Sha256::new();
        hasher.update(&network_time_offset.as_micros().to_le_bytes());
        let timestamp_micros = time_verification_timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        hasher.update(&timestamp_micros.to_le_bytes());
        hasher.update(&nonce.to_le_bytes());
        let proof_hash = hasher.finalize().to_vec();

        AssetAllocationRequest {
            asset_type: AssetType::Storage,
            requested_resources: crate::assets::core::ResourceRequirements {
                storage_usage: Some(StorageRequirements {
                    size_bytes: 100 * 1024 * 1024 * 1024, // 100GB
                    storage_type: StorageType::Ssd,
                    min_iops: Some(1000),
                    min_bandwidth_mbps: Some(100),
                    durability_replicas: 2,
                }),
                ..Default::default()
            },
            privacy_level: PrivacyMode::PRIVATE,
            // ConsensusProof::new expects: (stake, time, space, work)
            consensus_proof: ConsensusProof::new(
                StakeProof {
                    stake_holder: "test-holder".to_string(),
                    stake_holder_id: "test-holder-id".to_string(),
                    stake_amount: 1000,
                    stake_timestamp: SystemTime::now(),
                },
                TimeProof {
                    network_time_offset,
                    time_verification_timestamp,
                    nonce,
                    proof_hash,
                },
                SpaceProof {
                    node_id: "test-node".to_string(),
                    storage_path: "/test/storage".to_string(),
                    total_size: 100 * 1024 * 1024 * 1024,
                    total_storage: 200 * 1024 * 1024 * 1024,
                    file_hash: "test_storage_hash".to_string(),
                    proof_timestamp: SystemTime::now(),
                },
                WorkProof {
                    owner_id: "test-worker".to_string(),
                    workload_id: "test-workload".to_string(),
                    pid: 12345,
                    computational_power: 100,
                    workload_type: WorkloadType::Storage,
                    work_state: WorkState::Completed,
                    work_challenges: vec!["storage_challenge".to_string()],
                    proof_timestamp: SystemTime::now(),
                },
            ),
            certificate_fingerprint: "test-cert".to_string(),
            duration_limit: Some(Duration::from_secs(3600)),
            tags: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_storage_adapter_creation() {
        let adapter = StorageAssetAdapter::new().await;
        use crate::assets::core::AssetAdapter;
        assert_eq!(adapter.asset_type(), AssetType::Storage);
    }

    #[tokio::test]
    async fn test_storage_allocation() {
        use crate::assets::core::AssetAdapter;
        use crate::assets::core::{AssetCategory, BaseSystemType};
        let adapter = StorageAssetAdapter::new().await;
        let request = create_test_storage_request().await;

        let allocation = adapter.allocate_asset(&request).await.unwrap();
        assert!(matches!(allocation.asset_id.category, AssetCategory::BaseSystem(BaseSystemType::Storage)));

        // Test deallocation
        adapter.deallocate_asset(&allocation.asset_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_storage_health_check() {
        use crate::assets::core::AssetAdapter;
        let adapter = StorageAssetAdapter::new().await;
        let health = adapter.health_check().await.unwrap();

        assert!(health.healthy);
        assert!(health.performance_metrics.contains_key("total_capacity_gb"));
        assert!(health.performance_metrics.contains_key("average_health_percent"));
    }

    #[tokio::test]
    async fn test_storage_capabilities() {
        use crate::assets::core::AssetAdapter;
        let adapter = StorageAssetAdapter::new().await;
        let capabilities = adapter.get_capabilities();

        assert_eq!(capabilities.asset_type, AssetType::Storage);
        assert!(capabilities.supports_proxy_addressing);
        assert!(capabilities.features.contains(&"distributed_storage".to_string()));
        assert!(capabilities.features.contains(&"kyber_encryption".to_string()));
    }
}
