//! ConsensusProof-native JuliaVM System with Matrix Chain Integration
//!
//! A virtual machine implementation where consensus proofs (PoSp+PoSt+PoWk+PoTm) are 
//! language-level constructs, not just validation layers. Every VM operation requires
//! consensus validation as part of the execution model.
//!
//! Key Features:
//! - Language-level consensus constructs embedded directly into VM execution
//! - Asset-aware execution treating all resources as HyperMesh Assets
//! - Multi-language support (Julia, Python, R, JavaScript, C/C++, Rust)
//! - Blockchain-native compute with direct storage integration
//! - P2P resource sharing with user-configurable privacy levels
//! - **Matrix Chain Integration**: Entity-aware execution across blockchain networks
//! - **Cross-Entity Validation**: Validate compute assets across entity chains
//! - **Multi-Entity Workflows**: Support workflows spanning multiple entity blockchains
//! - **Privacy-Preserving Execution**: Respect entity privacy policies during execution
//! - **Federated Asset Allocation**: Request compute resources from specific entities
//!
//! ## Matrix Chain Architecture
//!
//! The VM now supports execution across the matrix chain architecture where each entity
//! (DMV, Dealer, Insurance, Bank, Manufacturer, etc.) operates their own blockchain:
//!
//! ```rust,no_run
//! use hypermesh_vm::{ConsensusProofVM, MatrixExecutionContext, CrossEntityValidation};
//! use crate::consensus::ConsensusProof;
//! use std::sync::Arc;
//!
//! async fn matrix_execution_example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create base VM
//!     let vm = Arc::new(ConsensusProofVM::new(Default::default()).await?);
//!     let matrix_manager = Arc::new(MatrixBlockchainManager::new());
//!     
//!     // Create matrix execution context for cross-entity workflow
//!     let matrix_context = MatrixExecutionContext {
//!         target_entity: Some("honda.hypermesh.online".to_string()),
//!         cross_entity_validations: vec![
//!             CrossEntityValidation {
//!                 entity_domain: "dmv.hypermesh.online".to_string(),
//!                 asset_id: vehicle_vin,
//!                 validation_fields: vec!["registration_status".to_string()],
//!                 validation_type: ValidationRequirementType::AssetExists,
//!                 privacy_level: PrivacyLevel::P2P,
//!             }
//!         ],
//!         entity_asset_requests: vec![
//!             EntityAssetRequest {
//!                 entity_domain: "dealer.hypermesh.online".to_string(),
//!                 asset_type: "cpu".to_string(),
//!                 requested_amount: 4,
//!                 duration_seconds: 3600,
//!                 compensation_tokens: 100,
//!                 priority: AssetRequestPriority::High,
//!             }
//!         ],
//!         // ... other context fields
//!     };
//!     
//!     // Execute across multiple entity chains
//!     let result = vm.execute_matrix_aware(
//!         "vehicle_validation_code",
//!         "julia",
//!         matrix_context,
//!         matrix_manager,
//!     ).await?;
//!     
//!     // Access cross-entity validation results
//!     for (entity, validation_result) in result.cross_entity_validations {
//!         println!("Validation from {}: {:?}", entity, validation_result);
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! Based on Proof of State four-proof consensus patterns adapted for HyperMesh ecosystem.

pub mod consensus;
pub mod execution;  
pub mod integration;
pub mod julia;
pub mod languages;
pub mod matrix_integration;
pub mod examples;

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

// Re-export core types
pub use crate::consensus::{ConsensusProof, SpaceProof, StakeProof, WorkProof, TimeProof};
pub use consensus::{ConsensusVM, VMConsensusContext, ConsensusOperation};
pub use execution::{VMExecutor, ExecutionContext, ExecutionResult};
pub use crate::assets::AssetAdapter;
pub use crate::integration::{BlockchainIntegration, P2PRouter};
pub use julia::{JuliaVM, JuliaConsensusRuntime};
pub use languages::{MultiLanguageSupport, LanguageRuntime};
pub use matrix_integration::{
    MatrixAwareVM, MatrixExecutionContext, MatrixExecutionResult,
    ValidationRequirementType, AssetRequestPriority, EntitySyncRequirement,
    SyncType, WorkflowPrivacyPolicy, ValidationConstraint,
    CrossEntityValidation, EntityAssetRequest, MultiEntityWorkflow
};

/// Asset identifier for VM operations
pub type AssetId = Uuid;

/// VM configuration with consensus requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMConfig {
    /// Required consensus proofs for all operations
    pub consensus_requirements: ConsensusRequirements,
    /// Asset management configuration
    pub asset_config: AssetManagementConfig,
    /// P2P networking configuration
    pub p2p_config: P2PConfig,
    /// Blockchain integration settings
    pub blockchain_config: BlockchainConfig,
    /// Privacy and resource sharing settings
    pub privacy_config: PrivacyConfig,
}

/// Consensus requirements for VM operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusRequirements {
    /// Require Proof of Space for all operations
    pub require_proof_of_space: bool,
    /// Require Proof of Stake for all operations
    pub require_proof_of_stake: bool,
    /// Require Proof of Work for all operations
    pub require_proof_of_work: bool,
    /// Require Proof of Time for all operations
    pub require_proof_of_time: bool,
    /// Minimum difficulty for work proofs
    pub min_work_difficulty: u32,
    /// Minimum space commitment (bytes)
    pub min_space_commitment: u64,
    /// Minimum stake authority level
    pub min_stake_authority: u64,
    /// Maximum time drift allowed (microseconds)
    pub max_time_drift: u64,
}

impl Default for ConsensusRequirements {
    fn default() -> Self {
        Self {
            require_proof_of_space: true,
            require_proof_of_stake: true, 
            require_proof_of_work: true,
            require_proof_of_time: true,
            min_work_difficulty: 16,
            min_space_commitment: 1024 * 1024 * 1024, // 1GB
            min_stake_authority: 1000,
            max_time_drift: 1_000_000, // 1 second
        }
    }
}

/// Asset management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManagementConfig {
    /// Enable CPU as managed asset
    pub manage_cpu: bool,
    /// Enable GPU as managed asset
    pub manage_gpu: bool,
    /// Enable memory as managed asset
    pub manage_memory: bool,
    /// Enable storage as managed asset
    pub manage_storage: bool,
    /// Asset allocation percentages per resource type
    pub resource_allocation: HashMap<String, f64>,
    /// Maximum concurrent asset operations
    pub max_concurrent_operations: u32,
}

impl Default for AssetManagementConfig {
    fn default() -> Self {
        let mut resource_allocation = HashMap::new();
        resource_allocation.insert("cpu".to_string(), 50.0);
        resource_allocation.insert("gpu".to_string(), 25.0);
        resource_allocation.insert("memory".to_string(), 60.0);
        resource_allocation.insert("storage".to_string(), 30.0);
        
        Self {
            manage_cpu: true,
            manage_gpu: false,
            manage_memory: true,
            manage_storage: true,
            resource_allocation,
            max_concurrent_operations: 10,
        }
    }
}

/// P2P networking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PConfig {
    /// Enable P2P resource sharing
    pub enable_p2p_sharing: bool,
    /// Maximum peer connections
    pub max_peers: u32,
    /// Peer discovery mechanisms
    pub discovery_methods: Vec<String>,
    /// Trust score thresholds
    pub trust_thresholds: HashMap<String, f64>,
}

impl Default for P2PConfig {
    fn default() -> Self {
        let mut trust_thresholds = HashMap::new();
        trust_thresholds.insert("min_execution_trust".to_string(), 0.7);
        trust_thresholds.insert("min_storage_trust".to_string(), 0.8);
        
        Self {
            enable_p2p_sharing: true,
            max_peers: 50,
            discovery_methods: vec!["dht".to_string(), "bootstrap".to_string()],
            trust_thresholds,
        }
    }
}

/// Blockchain integration configuration  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    /// Enable blockchain-native storage
    pub enable_blockchain_storage: bool,
    /// Blockchain endpoint
    pub blockchain_endpoint: Option<String>,
    /// Block confirmation requirements
    pub required_confirmations: u32,
    /// Gas limits for operations
    pub gas_limits: HashMap<String, u64>,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        let mut gas_limits = HashMap::new();
        gas_limits.insert("storage".to_string(), 100_000);
        gas_limits.insert("compute".to_string(), 200_000);
        
        Self {
            enable_blockchain_storage: true,
            blockchain_endpoint: None,
            required_confirmations: 3,
            gas_limits,
        }
    }
}

/// Privacy and resource sharing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Default privacy level for resources
    pub default_privacy_level: PrivacyLevel,
    /// User-configurable resource sharing settings
    pub resource_sharing: HashMap<String, ResourceSharingConfig>,
    /// Anonymization settings
    pub anonymization_enabled: bool,
}

/// Privacy levels for resource sharing (from Proof of State analysis)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// No public access
    Private,
    /// Specific networks/groups only
    PrivateNetwork,
    /// Trusted peer sharing
    P2P,
    /// Specific public networks
    PublicNetwork,
    /// Maximum CAESAR rewards, full HyperMesh participation
    FullPublic,
}

/// Resource sharing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSharingConfig {
    /// Privacy level for this resource
    pub privacy_level: PrivacyLevel,
    /// Percentage of resource to share (0-100)
    pub share_percentage: f64,
    /// Maximum concurrent usage
    pub max_concurrent_usage: u32,
    /// Rewards configuration
    pub rewards_enabled: bool,
    /// Duration limits (seconds)
    pub max_duration: u64,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        let mut resource_sharing = HashMap::new();
        
        // Conservative defaults following Proof of State patterns
        resource_sharing.insert("cpu".to_string(), ResourceSharingConfig {
            privacy_level: PrivacyLevel::P2P,
            share_percentage: 25.0,
            max_concurrent_usage: 2,
            rewards_enabled: true,
            max_duration: 3600, // 1 hour
        });
        
        resource_sharing.insert("memory".to_string(), ResourceSharingConfig {
            privacy_level: PrivacyLevel::Private,
            share_percentage: 10.0,
            max_concurrent_usage: 1,
            rewards_enabled: false,
            max_duration: 1800, // 30 minutes
        });
        
        Self {
            default_privacy_level: PrivacyLevel::Private,
            resource_sharing,
            anonymization_enabled: true,
        }
    }
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            consensus_requirements: ConsensusRequirements::default(),
            asset_config: AssetManagementConfig::default(),
            p2p_config: P2PConfig::default(),
            blockchain_config: BlockchainConfig::default(),
            privacy_config: PrivacyConfig::default(),
        }
    }
}

/// Main ConsensusProof VM instance
pub struct ConsensusProofVM {
    /// VM configuration
    config: Arc<VMConfig>,
    /// Consensus VM core
    consensus_vm: Arc<ConsensusVM>,
    /// Execution engine
    executor: Arc<VMExecutor>,
    /// Julia runtime
    julia_runtime: Arc<JuliaVM>,
    /// Multi-language support
    language_support: Arc<MultiLanguageSupport>,
    /// Asset adapters
    asset_adapters: HashMap<String, Arc<dyn AssetAdapter>>,
    /// Blockchain integration
    blockchain: Arc<dyn BlockchainIntegration>,
    /// P2P router
    p2p_router: Arc<P2PRouter>,
}

impl ConsensusProofVM {
    /// Create new VM instance with consensus-native execution
    pub async fn new(config: VMConfig) -> Result<Self> {
        let config = Arc::new(config);
        
        // Initialize consensus VM with 4-proof requirements
        let consensus_vm = Arc::new(ConsensusVM::new(
            config.consensus_requirements.clone()
        )?);
        
        // Initialize execution engine
        let executor = Arc::new(VMExecutor::new(
            Arc::clone(&consensus_vm),
            config.asset_config.clone(),
        ).await?);
        
        // Initialize Julia runtime with consensus integration
        let julia_runtime = Arc::new(JuliaVM::new(
            Arc::clone(&consensus_vm)
        ).await?);
        
        // Initialize multi-language support
        let language_support = Arc::new(MultiLanguageSupport::new(
            Arc::clone(&consensus_vm)
        ).await?);
        
        // Initialize asset adapters (CPU, GPU, Memory, Storage)
        let asset_adapters = Self::initialize_asset_adapters(&config).await?;
        
        // Initialize blockchain integration
        let blockchain = Arc::new(integration::HyperMeshBlockchain::new(
            config.blockchain_config.clone()
        ).await?);
        
        // Initialize P2P router
        let p2p_router = Arc::new(P2PRouter::new(
            config.p2p_config.clone(),
            Arc::clone(&blockchain),
        ).await?);
        
        Ok(Self {
            config,
            consensus_vm,
            executor,
            julia_runtime,
            language_support,
            asset_adapters,
            blockchain,
            p2p_router,
        })
    }
    
    /// Execute code with full consensus validation
    pub async fn execute_with_consensus(
        &self,
        code: &str,
        language: &str,
        consensus_proof: ConsensusProof,
    ) -> Result<ExecutionResult> {
        // Validate consensus proof meets requirements
        if !self.consensus_vm.validate_consensus_proof(&consensus_proof).await? {
            return Err(anyhow::anyhow!("Invalid consensus proof"));
        }
        
        // Create execution context with asset awareness
        let context = ExecutionContext {
            consensus_proof,
            language: language.to_string(),
            asset_allocations: self.calculate_asset_allocations().await?,
            privacy_settings: self.config.privacy_config.clone(),
            blockchain_context: self.blockchain.get_context().await?,
            p2p_context: self.p2p_router.get_routing_context().await?,
        };
        
        // Execute through consensus VM
        self.executor.execute(code, context).await
    }
    
    /// Initialize asset adapters for all resource types
    async fn initialize_asset_adapters(
        config: &VMConfig,
    ) -> Result<HashMap<String, Arc<dyn AssetAdapter>>> {
        let mut adapters: HashMap<String, Arc<dyn AssetAdapter>> = HashMap::new();
        
        if config.asset_config.manage_cpu {
            adapters.insert("cpu".to_string(), Arc::new(
                crate::assets::CpuAssetAdapter::new(
                    config.consensus_requirements.clone()
                ).await?
            ));
        }
        
        if config.asset_config.manage_gpu {
            adapters.insert("gpu".to_string(), Arc::new(
                crate::assets::GpuAssetAdapter::new(
                    config.consensus_requirements.clone()
                ).await?
            ));
        }
        
        if config.asset_config.manage_memory {
            adapters.insert("memory".to_string(), Arc::new(
                crate::assets::MemoryAssetAdapter::new(
                    config.consensus_requirements.clone()
                ).await?
            ));
        }
        
        if config.asset_config.manage_storage {
            adapters.insert("storage".to_string(), Arc::new(
                crate::assets::StorageAssetAdapter::new(
                    config.consensus_requirements.clone()
                ).await?
            ));
        }
        
        Ok(adapters)
    }
    
    /// Calculate current asset allocations based on availability and sharing settings
    async fn calculate_asset_allocations(&self) -> Result<HashMap<String, AssetAllocation>> {
        let mut allocations = HashMap::new();
        
        for (asset_type, adapter) in &self.asset_adapters {
            let availability = adapter.get_availability().await?;
            let sharing_config = self.config.privacy_config.resource_sharing
                .get(asset_type)
                .cloned()
                .unwrap_or_default();
            
            allocations.insert(asset_type.clone(), AssetAllocation {
                total_capacity: availability.total_capacity,
                available_capacity: availability.available_capacity,
                shared_capacity: (availability.total_capacity as f64 * 
                    sharing_config.share_percentage / 100.0) as u64,
                privacy_level: sharing_config.privacy_level,
                max_concurrent_usage: sharing_config.max_concurrent_usage,
            });
        }
        
        Ok(allocations)
    }
    
    /// Get VM configuration
    pub fn config(&self) -> Arc<VMConfig> {
        Arc::clone(&self.config)
    }
    
    /// Get consensus VM instance
    pub fn consensus_vm(&self) -> Arc<ConsensusVM> {
        Arc::clone(&self.consensus_vm)
    }
    
    /// Get Julia runtime
    pub fn julia_runtime(&self) -> Arc<JuliaVM> {
        Arc::clone(&self.julia_runtime)
    }
    
    /// Get language support
    pub fn language_support(&self) -> Arc<MultiLanguageSupport> {
        Arc::clone(&self.language_support)
    }
    
    /// Create matrix-aware VM for entity blockchain operations
    pub async fn create_matrix_aware_vm(
        self: Arc<Self>,
        matrix_manager: Arc<crate::assets::matrix_blockchain::MatrixBlockchainManager>,
    ) -> Result<matrix_integration::MatrixAwareVM> {
        matrix_integration::MatrixAwareVM::new(
            self,
            matrix_manager,
        ).await
    }
    
    /// Execute with matrix chain awareness
    pub async fn execute_matrix_aware(
        self: Arc<Self>,
        code: &str,
        language: &str,
        matrix_context: matrix_integration::MatrixExecutionContext,
        matrix_manager: Arc<crate::assets::matrix_blockchain::MatrixBlockchainManager>,
    ) -> Result<matrix_integration::MatrixExecutionResult> {
        let matrix_vm = self.create_matrix_aware_vm(matrix_manager).await?;
        matrix_vm.execute_matrix_aware(code, language, matrix_context).await
    }
}

/// Asset allocation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAllocation {
    /// Total capacity of the asset
    pub total_capacity: u64,
    /// Currently available capacity
    pub available_capacity: u64,
    /// Capacity available for sharing
    pub shared_capacity: u64,
    /// Privacy level for sharing
    pub privacy_level: PrivacyLevel,
    /// Maximum concurrent usage
    pub max_concurrent_usage: u32,
}

/// Asset availability information
#[derive(Debug, Clone)]
pub struct AssetAvailability {
    /// Total capacity
    pub total_capacity: u64,
    /// Available capacity
    pub available_capacity: u64,
    /// Current utilization percentage
    pub utilization_percentage: f64,
}

impl Default for ResourceSharingConfig {
    fn default() -> Self {
        Self {
            privacy_level: PrivacyLevel::Private,
            share_percentage: 0.0,
            max_concurrent_usage: 1,
            rewards_enabled: false,
            max_duration: 3600,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_consensus_proof_vm_creation() {
        let config = VMConfig::default();
        let vm = ConsensusProofVM::new(config).await;
        // Note: This will fail until all dependencies are implemented
        // but tests the basic structure
        assert!(vm.is_err()); // Expected to fail with missing implementations
    }
    
    #[test]
    fn test_vm_config_defaults() {
        let config = VMConfig::default();
        
        // Test consensus requirements
        assert!(config.consensus_requirements.require_proof_of_space);
        assert!(config.consensus_requirements.require_proof_of_stake);
        assert!(config.consensus_requirements.require_proof_of_work);
        assert!(config.consensus_requirements.require_proof_of_time);
        
        // Test asset management
        assert!(config.asset_config.manage_cpu);
        assert!(config.asset_config.manage_memory);
        assert!(config.asset_config.manage_storage);
        
        // Test privacy settings
        assert!(matches!(
            config.privacy_config.default_privacy_level,
            PrivacyLevel::Private
        ));
    }
    
    #[tokio::test]
    async fn test_matrix_integration_types() {
        use crate::assets::matrix_blockchain::{MatrixBlockchainManager, EntityType};
        use uuid::Uuid;
        
        // Test CrossEntityValidation creation
        let validation = CrossEntityValidation {
            entity_domain: "honda.hypermesh.online".to_string(),
            asset_id: Uuid::new_v4(),
            validation_fields: vec!["vin".to_string(), "model".to_string()],
            validation_type: ValidationRequirementType::AssetExists,
            privacy_level: PrivacyLevel::P2P,
        };
        
        assert_eq!(validation.entity_domain, "honda.hypermesh.online");
        assert_eq!(validation.validation_fields.len(), 2);
        assert!(matches!(validation.validation_type, ValidationRequirementType::AssetExists));
        
        // Test EntityAssetRequest creation
        let request = EntityAssetRequest {
            entity_domain: "dealer.hypermesh.online".to_string(),
            asset_type: "cpu".to_string(),
            requested_amount: 4,
            duration_seconds: 3600,
            compensation_tokens: 100,
            priority: AssetRequestPriority::High,
        };
        
        assert_eq!(request.entity_domain, "dealer.hypermesh.online");
        assert_eq!(request.requested_amount, 4);
        assert!(matches!(request.priority, AssetRequestPriority::High));
        
        // Test MultiEntityWorkflow creation
        let workflow = MultiEntityWorkflow {
            entity_sequence: vec![
                "honda.hypermesh.online".to_string(),
                "dealer.hypermesh.online".to_string(),
                "bank.hypermesh.online".to_string(),
            ],
            data_flow: std::collections::HashMap::new(),
            sync_requirements: vec![],
            workflow_privacy: WorkflowPrivacyPolicy {
                intermediate_privacy: PrivacyLevel::P2P,
                final_privacy: PrivacyLevel::PublicNetwork,
                intermediate_access: vec!["dealer.hypermesh.online".to_string()],
                data_sharing_rules: std::collections::HashMap::new(),
            },
        };
        
        assert_eq!(workflow.entity_sequence.len(), 3);
        assert!(matches!(workflow.workflow_privacy.intermediate_privacy, PrivacyLevel::P2P));
    }
    
    #[tokio::test]
    async fn test_matrix_execution_context_creation() {
        use crate::catalog::vm::execution::ExecutionContext;
        use crate::consensus::ConsensusProof;
        use uuid::Uuid;
        
        // Create base execution context
        let base_context = ExecutionContext {
            consensus_proof: ConsensusProof::new(
                crate::consensus::proof::SpaceProof::new(
                    "/test".to_string(),
                    crate::consensus::proof::NetworkPosition {
                        address: "test.hypermesh.online".to_string(),
                        zone: "test".to_string(),
                        distance_metric: 0,
                    },
                    0,
                ),
                crate::consensus::proof::StakeProof::new(
                    "test.hypermesh.online".to_string(),
                    "test-node".to_string(),
                    1000,
                    crate::consensus::proof::AccessPermissions {
                        read_level: crate::consensus::proof::AccessLevel::Public,
                        write_level: crate::consensus::proof::AccessLevel::Public,
                        admin_level: crate::consensus::proof::AccessLevel::Verified,
                        allocation_rights: vec!["test".to_string()],
                    },
                    vec!["test-allowance".to_string()],
                ),
                crate::consensus::proof::WorkProof::new(
                    b"test-challenge",
                    4,
                    "test".to_string(),
                ).unwrap(),
                crate::consensus::proof::TimeProof::new(0, None, 0),
            ),
            language: "julia".to_string(),
            asset_allocations: std::collections::HashMap::new(),
            privacy_settings: PrivacyConfig::default(),
            blockchain_context: serde_json::Value::Null,
            p2p_context: serde_json::Value::Null,
        };
        
        // Create matrix execution context
        let matrix_context = MatrixExecutionContext {
            base_context,
            target_entity: Some("honda.hypermesh.online".to_string()),
            cross_entity_validations: vec![
                CrossEntityValidation {
                    entity_domain: "dmv.hypermesh.online".to_string(),
                    asset_id: Uuid::new_v4(),
                    validation_fields: vec!["registration_status".to_string()],
                    validation_type: ValidationRequirementType::AssetExists,
                    privacy_level: PrivacyLevel::P2P,
                }
            ],
            entity_privacy_policies: std::collections::HashMap::new(),
            workflow_config: None,
            entity_asset_requests: vec![
                EntityAssetRequest {
                    entity_domain: "dealer.hypermesh.online".to_string(),
                    asset_type: "cpu".to_string(),
                    requested_amount: 2,
                    duration_seconds: 1800,
                    compensation_tokens: 50,
                    priority: AssetRequestPriority::Normal,
                }
            ],
        };
        
        assert_eq!(matrix_context.target_entity, Some("honda.hypermesh.online".to_string()));
        assert_eq!(matrix_context.cross_entity_validations.len(), 1);
        assert_eq!(matrix_context.entity_asset_requests.len(), 1);
        assert_eq!(matrix_context.base_context.language, "julia");
    }
}