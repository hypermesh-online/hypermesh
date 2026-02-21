// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog VM Matrix Chain Integration
//!
//! Integration layer for the Catalog VM system with the new matrix chain architecture.
//! Each entity has its own blockchain, and VM operations can:
//! 1. Execute on specific entity chains
//! 2. Validate compute assets across entity chains
//! 3. Respect entity privacy policies during execution
//! 4. Support multi-entity workflows spanning multiple chains
//! 5. Request asset allocation from specific entity blockchains
//!
//! Based on Proof of State patterns adapted for the HyperMesh matrix architecture.

pub mod types;
pub mod coordinator;
pub mod operations;

// Re-export all public types
pub use types::*;

// Re-export the VM and coordinator
pub use operations::MatrixAwareVM;
pub use coordinator::EntityAssetCoordinator;

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{VMConfig, ConsensusProofVM};
    use crate::assets::matrix_blockchain::{
        MatrixBlockchainManager, EntityType, PrivacyPolicyConfig,
    };
    use std::sync::Arc;
    use std::collections::HashMap;
    use super::super::PrivacyMode;

    #[tokio::test]
    async fn test_matrix_aware_vm_creation() {
        let vm_config = VMConfig::default();
        let base_vm = Arc::new(ConsensusProofVM::new(vm_config).await.expect("test: VM creation"));
        let matrix_manager = Arc::new(MatrixBlockchainManager::new());

        let matrix_vm = MatrixAwareVM::new(base_vm, matrix_manager).await;
        assert!(matrix_vm.is_ok());
    }

    #[test]
    fn test_cross_entity_validation_creation() {
        use crate::assets::core::{AssetRegistration, AssetData, NetworkScope, AssetCategory, BaseSystemType};

        let data = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };
        let asset_id = AssetRegistration::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Container),
        );

        let validation = CrossEntityValidation {
            entity_domain: "honda.hypermesh.online".to_string(),
            asset_id,
            validation_fields: vec!["vin".to_string(), "model".to_string()],
            validation_type: ValidationRequirementType::AssetExists,
            privacy_level: PrivacyMode::PRIVATE,
        };

        assert_eq!(validation.entity_domain, "honda.hypermesh.online");
        assert_eq!(validation.validation_fields.len(), 2);
    }

    #[test]
    fn test_entity_asset_request_creation() {
        let request = EntityAssetRequest {
            entity_domain: "dealer.hypermesh.online".to_string(),
            asset_type: "cpu".to_string(),
            requested_amount: 2,
            duration_seconds: 3600,
            compensation_tokens: 100,
            priority: AssetRequestPriority::Normal,
        };

        assert_eq!(request.entity_domain, "dealer.hypermesh.online");
        assert_eq!(request.requested_amount, 2);
    }

    #[test]
    fn test_multi_entity_workflow_creation() {
        let workflow = MultiEntityWorkflow {
            entity_sequence: vec![
                "honda.hypermesh.online".to_string(),
                "dealer.hypermesh.online".to_string(),
                "bank.hypermesh.online".to_string(),
            ],
            data_flow: HashMap::new(),
            sync_requirements: vec![
                EntitySyncRequirement {
                    source_entity: "honda.hypermesh.online".to_string(),
                    target_entity: "dealer.hypermesh.online".to_string(),
                    sync_type: SyncType::Sequential,
                    max_delay_micros: 1000000,
                }
            ],
            workflow_privacy: WorkflowPrivacyPolicy {
                intermediate_privacy: PrivacyMode::PRIVATE,
                final_privacy: PrivacyMode::PUBLIC,
                intermediate_access: vec!["dealer.hypermesh.online".to_string()],
                data_sharing_rules: HashMap::new(),
            },
        };

        assert_eq!(workflow.entity_sequence.len(), 3);
        assert_eq!(workflow.sync_requirements.len(), 1);
    }

    #[tokio::test]
    async fn test_entity_asset_coordinator() {
        let coordinator = EntityAssetCoordinator::new();

        let config = EntityVMConfig {
            entity_domain: "test.hypermesh.online".to_string(),
            entity_type: EntityType::Organization("Test".to_string()),
            vm_config: VMConfig::default(),
            privacy_policies: PrivacyPolicyConfig {
                public_fields: vec![],
                federated_fields: HashMap::new(),
                zk_proof_fields: vec![],
                default_privacy_level: PrivacyMode::PRIVATE,
            },
            trusted_partners: vec![],
            max_external_allocation: {
                let mut alloc = HashMap::new();
                alloc.insert("cpu".to_string(), 4);
                alloc.insert("memory".to_string(), 8192);
                alloc
            },
        };

        assert!(coordinator.update_entity_pool("test.hypermesh.online", &config).is_ok());

        let request = EntityAssetRequest {
            entity_domain: "test.hypermesh.online".to_string(),
            asset_type: "cpu".to_string(),
            requested_amount: 2,
            duration_seconds: 3600,
            compensation_tokens: 100,
            priority: AssetRequestPriority::Normal,
        };

        let allocation = coordinator.allocate_asset_from_entity(&request).await;
        assert!(allocation.is_ok());

        if let Ok(allocation) = allocation {
            assert_eq!(allocation.allocated_capacity, 2);
            assert!(coordinator.release_allocation(&allocation.allocation_id).await.is_ok());
        }
    }
}
