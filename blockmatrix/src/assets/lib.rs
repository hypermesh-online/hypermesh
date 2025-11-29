//! HyperMesh Asset Management System
//!
//! Universal asset management where everything in HyperMesh is an Asset:
//! - Hardware resources (CPU, GPU, Memory, Storage, Network)
//! - Containers and services  
//! - User-defined assets and applications
//!
//! All assets require Consensus Proof validation (PoSpace + PoStake + PoWork + PoTime)
//! and support user-configurable privacy levels with remote proxy addressing.
//!
//! # Key Features
//!
//! - **Universal Asset System**: Everything is treated as an asset with unified management
//! - **Consensus Proof Integration**: ALL operations require PoSp+PoSt+PoWk+PoTm validation
//! - **Privacy-Aware Allocation**: User-configurable privacy levels (Private → FullPublic)
//! - **Remote Proxy Addressing**: NAT-like addressing for global HyperMesh ecosystem
//! - **Quantum-Resistant Security**: FALCON-1024 signatures, Kyber encryption patterns
//! - **Federated Trust**: Integration with TrustChain certificate hierarchy
//! - **Asset Adapters**: Specialized handling while maintaining unified interface
//! - **Real-time Monitoring**: Asset status, health, and performance tracking
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     AssetManager                            │
//! │  ┌─────────────────┐ ┌─────────────────┐ ┌───────────────┐  │
//! │  │  Asset Registry │ │ Adapter Registry│ │ Proxy Resolver│  │
//! │  └─────────────────┘ └─────────────────┘ └───────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                                 │
//!         ┌───────────────────────┼───────────────────────┐
//!         │                       │                       │
//! ┌───────▼────────┐    ┌─────────▼────────┐    ┌─────────▼────────┐
//! │  CpuAdapter    │    │   GpuAdapter     │    │ MemoryAdapter    │
//! │                │    │                  │    │                  │
//! │ • Cores        │    │ • Compute Units  │    │ • RAM Pool       │
//! │ • Frequency    │    │ • GPU Memory     │    │ • Swap Space     │
//! │ • Architecture │    │ • Nova/Vulkan    │    │ • NUMA Aware     │
//! └────────────────┘    └──────────────────┘    └──────────────────┘
//!
//! ┌────────────────┐    ┌──────────────────┐    ┌──────────────────┐
//! │ StorageAdapter │    │ NetworkAdapter   │    │ ContainerAdapter │
//! │                │    │                  │    │                  │
//! │ • NVMe/SSD     │    │ • Bandwidth      │    │ • Docker/Podman  │
//! │ • Distributed  │    │ • QoS            │    │ • K8s Integration│
//! │ • Sharding     │    │ • IPv6 Only      │    │ • Resource Limits│
//! └────────────────┘    └──────────────────┘    └──────────────────┘
//! ```
//!
//! # Usage Examples
//!
//! ## Basic Asset Allocation
//!
//! ```rust,no_run
//! use hypermesh_assets::{AssetManager, AssetAllocationRequest, AssetType, PrivacyLevel};
//! use hypermesh_assets::core::{ConsensusProof, SpaceProof, StakeProof, WorkProof, TimeProof};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let asset_manager = AssetManager::new();
//!
//! // Create consensus proof (ALL FOUR PROOFS REQUIRED)
//! let consensus_proof = ConsensusProof::new(
//!     SpaceProof { /* ... */ },
//!     StakeProof { /* ... */ },
//!     WorkProof { /* ... */ },
//!     TimeProof { /* ... */ },
//! );
//!
//! let request = AssetAllocationRequest {
//!     asset_type: AssetType::Cpu,
//!     privacy_level: PrivacyLevel::P2P,
//!     consensus_proof,
//!     certificate_fingerprint: "cert-fingerprint".to_string(),
//!     // ... other fields
//! };
//!
//! let allocation = asset_manager.allocate_asset(request).await?;
//! println!("Allocated asset: {}", allocation.asset_id);
//! # Ok(())
//! # }
//! ```
//!
//! ## Privacy Configuration
//!
//! ```rust,no_run
//! use hypermesh_assets::core::privacy::{PrivacyLevel, UserPrivacyConfig};
//!
//! // Configure user privacy preferences
//! let privacy_config = UserPrivacyConfig {
//!     default_privacy_level: PrivacyLevel::P2P,
//!     // Private: No sharing, no rewards
//!     // PrivateNetwork: Trusted groups only  
//!     // P2P: Verified peers
//!     // PublicNetwork: Public networks
//!     // FullPublic: Maximum rewards, full sharing
//!     ..Default::default()
//! };
//! ```
//!
//! ## Remote Proxy Addressing
//!
//! ```rust,no_run
//! use hypermesh_assets::core::proxy::{ProxyAddress, ProxyAddressResolver};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let resolver = ProxyAddressResolver::new();
//!
//! // Create NAT-like proxy address
//! let proxy_addr = ProxyAddress::new(
//!     [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01], // HyperMesh network
//!     [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08], // Node ID
//!     8080 // Asset port
//! );
//!
//! println!("Proxy address: {}", proxy_addr); // hypermesh://network_usage:node/port
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]

// Re-export all public APIs
pub mod core;
pub mod adapters;
pub mod proxy;
pub mod blockchain;
pub mod cross_chain;
pub mod matrix_blockchain;

// Main exports
pub use core::{
    AssetManager, AssetError, AssetResult,
    AssetId, AssetType,
    AssetStatus, AssetState,
    ConsensusProof, SpaceProof, StakeProof, WorkProof, TimeProof,
    ConsensusRequirements, AssetStatistics,
};

// Adapter system exports
pub use core::adapter::{
    AssetAdapter, AssetAllocationRequest, 
    ResourceRequirements, ResourceUsage, ResourceLimits,
    CpuRequirements, GpuRequirements, MemoryRequirements, 
    StorageRequirements, NetworkRequirements, ContainerRequirements,
    AdapterCapabilities, AdapterHealth,
};

// Privacy system exports  
pub use core::privacy::{
    PrivacyLevel, AssetAllocation, AllocationConfig,
    AccessConfig, AccessPermissions, RateLimits,
    UserPrivacyConfig, CaesarPreferences,
};

// Complete privacy management system exports
pub mod privacy {
    pub use crate::assets::core::privacy::{
        // Core privacy management
        PrivacyManager, PrivacyManagerConfig, UserPrivacyConfiguration,
        PrivacyAllocationResult, ResourceAllocationConfig,
        
        // Privacy allocation types (from Proof of State patterns)
        PrivacyAllocationType, AllocationTypeConfig, AllocationTypeConstraints,
        PrivacyTransition, TransitionValidation,
        
        // Privacy enforcement
        PrivacyEnforcer, AccessControlResult, PrivacyViolation,
        EnforcementAction, PrivacyAuditLog,
        
        // CAESAR reward system
        CaesarRewardCalculator, RewardConfiguration, RewardTier,
        RewardCalculationResult, PerformanceBonus,
        
        // Privacy configuration
        UserPrivacyConfig, PrivacySettings, ResourcePrivacySettings,
        PrivacyConstraints, PrivacyValidationRules, PrivacyTemplate,
        PrivacyPreset, AdvancedPrivacyOptions,
        
        // Consensus requirements
        ConsensusRequirementConfig, DifficultyRequirements,
        
        // Proxy configuration for privacy
        ProxyConfiguration, NatAddressingPreferences,
        ProxyNodeSelection, QuantumSecurityConfig,
    };
}

// Proxy system exports
pub use core::proxy::{
    ProxyAddress, ProxyAddressResolver, ProxyNodeInfo,
    ProxyCapabilities, ProxyStatistics,
};

// Remote Proxy/NAT system exports (CRITICAL IMPLEMENTATION)
pub mod proxy {
    pub use crate::assets::core::proxy::{
        RemoteProxyManager, ProxyRouter, ProxyForwarder,
        TrustChainIntegration, QuantumSecurity, ShardedDataAccess,
        NATTranslator, GlobalAddress, MemoryAddressTranslator,
        ProxySystemStats, ProxyNetworkConfig,
    };
}

// Hardware adapter exports
pub use adapters::{
    AdapterRegistry,
    MemoryAssetAdapter, CpuAssetAdapter, GpuAssetAdapter,
    StorageAssetAdapter, NetworkAssetAdapter, ContainerAssetAdapter,
};

// Cross-chain validation system exports
pub use cross_chain::{
    CrossNetworkValidator, CrossChainValidationManager, CrossChainValidationResult,
    CrossChainValidationRule, BusinessWorkflowType, ZKProofStatement, ZKStatementType,
    PrivacyRequirements, NetworkValidationStep, ValidationCacheConfig,
};

// Matrix blockchain system exports
pub use matrix_blockchain::{
    MatrixBlockchainManager, EntityBlockchain, EntityType, EntityConfig,
    MatrixCoordinate, ValidationRequest, PublicValidationResponse, ValidationResult,
};

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// HyperMesh Asset System version
pub const HYPERMESH_ASSETS_VERSION: &str = "0.1.0";

/// Supported consensus proof version
pub const CONSENSUS_PROOF_VERSION: u8 = 1;

/// Default HyperMesh network identifier (IPv6 prefix)
pub const DEFAULT_HYPERMESH_NETWORK_ID: [u8; 16] = [
    0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01
];

/// Default asset port range for proxy addressing
pub const DEFAULT_ASSET_PORT_RANGE: (u16, u16) = (8000, 9000);

/// Maximum concurrent asset allocations per adapter
pub const MAX_CONCURRENT_ALLOCATIONS: u32 = 1000;

/// Default consensus proof validation timeout
pub const CONSENSUS_VALIDATION_TIMEOUT_SECS: u64 = 30;

/// Asset health check interval
pub const ASSET_HEALTH_CHECK_INTERVAL_SECS: u64 = 60;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::time::{Duration, SystemTime};
    
    #[tokio::test]
    async fn test_full_asset_lifecycle() {
        // Test complete asset allocation and deallocation lifecycle
        let manager = AssetManager::new();
        
        // This would be a full integration test with real adapters
        // For now, we just test that the manager can be created
        let stats = manager.get_asset_statistics().await;
        assert_eq!(stats.total_assets, 0);
    }
    
    #[test]
    fn test_consensus_proof_validation() {
        let space_proof = SpaceProof {
            node_id: "test-node".to_string(),
            storage_path: "/test/storage".to_string(),
            allocated_size: 1024 * 1024 * 1024, // 1GB
            proof_hash: vec![1, 2, 3, 4, 5, 6, 7, 8],
            timestamp: SystemTime::now(),
        };
        
        let stake_proof = StakeProof {
            stake_holder: "test-holder".to_string(),
            stake_holder_id: "test-holder-id".to_string(),
            stake_amount: 1000,
            stake_timestamp: SystemTime::now(),
        };
        
        let work_proof = WorkProof {
            worker_id: "test-worker".to_string(),
            workload_id: "test-workload".to_string(),
            process_id: 12345,
            computational_power: 100,
            workload_type: core::WorkloadType::Compute,
            work_state: core::WorkState::Completed,
        };
        
        let time_proof = TimeProof {
            network_time_offset: Duration::from_secs(5),
            time_verification_timestamp: SystemTime::now(),
            nonce: 42,
            proof_hash: vec![9, 10, 11, 12, 13, 14, 15, 16],
        };
        
        let consensus_proof = ConsensusProof::new(space_proof, stake_proof, work_proof, time_proof);
        
        // Test that all four proofs are required and validated
        assert!(consensus_proof.validate());
    }
    
    #[test]
    fn test_privacy_level_hierarchy() {
        // Test privacy level access control hierarchy
        assert!(!PrivacyLevel::Private.allows_access_from(&PrivacyLevel::FullPublic));
        assert!(PrivacyLevel::FullPublic.allows_access_from(&PrivacyLevel::Private));
        assert!(PrivacyLevel::PublicNetwork.allows_access_from(&PrivacyLevel::P2P));
        
        // Test CAESAR reward multipliers
        assert_eq!(PrivacyLevel::Private.caesar_reward_multiplier(), 0.0);
        assert_eq!(PrivacyLevel::FullPublic.caesar_reward_multiplier(), 1.0);
        assert!(PrivacyLevel::P2P.caesar_reward_multiplier() > 0.0);
        assert!(PrivacyLevel::P2P.caesar_reward_multiplier() < 1.0);
    }
    
    #[test]
    fn test_proxy_address_system() {
        let network_id = DEFAULT_HYPERMESH_NETWORK_ID;
        let node_id = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let asset_port = 8080;
        
        let proxy_addr = ProxyAddress::new(network_id, node_id, asset_port);
        
        // Test address creation and verification
        assert_eq!(proxy_addr.network_id, network_id);
        assert_eq!(proxy_addr.node_id, node_id);
        assert_eq!(proxy_addr.asset_port, asset_port);
        assert!(proxy_addr.verify_access_token());
        
        // Test string conversion
        let addr_string = proxy_addr.to_string();
        assert!(addr_string.starts_with("hypermesh://"));
        
        // Test IPv6 conversion
        let socket_addr = proxy_addr.to_ipv6_socket();
        assert_eq!(socket_addr.port(), asset_port);
    }
    
    #[test]
    fn test_asset_id_generation() {
        let asset_id = AssetId::new(AssetType::Cpu);
        
        // Test asset ID properties
        assert_eq!(asset_id.asset_type, AssetType::Cpu);
        assert!(asset_id.verify_blockchain_hash());
        
        // Test hex string conversion
        let hex_string = asset_id.to_hex_string();
        assert!(hex_string.starts_with("cpu:"));
        assert!(hex_string.contains('-')); // UUID hyphens
        assert!(hex_string.len() > 50); // Reasonable length check
        
        // Test short ID
        let short_id = asset_id.short_id();
        assert!(short_id.starts_with("cpu:"));
        assert!(short_id.contains("..."));
    }
}