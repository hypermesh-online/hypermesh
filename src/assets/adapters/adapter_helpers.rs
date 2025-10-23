//! Helper functions for asset adapters
//!
//! Common utilities and helper functions for creating consistent
//! asset allocations and status across all adapter implementations.

use std::collections::HashMap;
use std::time::SystemTime;

use crate::assets::core::{
    AssetId, AssetType, AssetAllocationRequest, AssetResult,
    PrivacyLevel, AssetAllocation, AssetStatus, AssetState,
    ResourceUsage, ProxyAddress,
};
use crate::assets::core::privacy::{AllocationConfig, AccessConfig, ResourceAllocationConfig, ConcurrencyLimits, DurationConfig, ConsensusRequirements, AccessPermissions, RateLimits, AuthRequirements};
use crate::assets::core::status::{AssetHealthStatus, AssetPerformanceMetrics};

/// Create a standard AssetAllocation for adapter responses
pub fn create_asset_allocation(
    asset_id: AssetId,
    request: &AssetAllocationRequest,
    proxy_address: Option<ProxyAddress>,
    metadata: HashMap<String, String>,
) -> AssetAllocation {
    // Create asset status
    let mut status = AssetStatus::new(
        asset_id.clone(),
        request.certificate_fingerprint.clone(),
        request.privacy_level.clone(),
    );
    
    // Set state to allocated
    status.state = AssetState::Allocated;
    
    // Add metadata
    for (key, value) in metadata {
        status.add_metadata(key, value);
    }
    
    // Set proxy address if provided
    if let Some(proxy_addr) = proxy_address {
        status.set_proxy_address(proxy_addr);
    }
    
    // Add consensus proof
    status.add_consensus_proof(request.consensus_proof.clone());
    
    // Create allocation config
    let allocation_config = AllocationConfig {
        privacy_level: request.privacy_level.clone(),
        resource_allocation: ResourceAllocationConfig::default(),
        concurrency_limits: ConcurrencyLimits::default(),
        duration_config: DurationConfig::default(),
        consensus_requirements: ConsensusRequirements::default(),
    };
    
    // Create access config
    let access_config = AccessConfig {
        allowed_certificates: vec![request.certificate_fingerprint.clone()],
        allowed_networks: Vec::new(),
        permissions: AccessPermissions::default(),
        rate_limits: RateLimits::default(),
        auth_requirements: AuthRequirements::default(),
    };
    
    let mut allocation = AssetAllocation {
        asset_id,
        status,
        allocation_config,
        access_config,
        allocated_at: SystemTime::now(),
        expires_at: request.duration_limit.map(|d| SystemTime::now() + d),
    };
    
    // Add allowed certificates from tags if present
    for (key, value) in &request.tags {
        if key == "allowed_certificates" {
            allocation.access_config.allowed_certificates.push(value.clone());
        }
    }
    
    allocation
}

/// Create a standard AssetStatus for adapter responses
pub fn create_asset_status(
    asset_id: AssetId,
    state: AssetState,
    privacy_level: PrivacyLevel,
    certificate_fingerprint: String,
    resource_usage: Option<ResourceUsage>,
    proxy_address: Option<ProxyAddress>,
    metadata: HashMap<String, String>,
) -> AssetStatus {
    let now = SystemTime::now();
    
    let mut status = AssetStatus {
        asset_id,
        state,
        allocated_at: now,
        last_accessed: now,
        resource_usage: resource_usage.unwrap_or_else(|| ResourceUsage {
            cpu_usage: None,
            gpu_usage: None,
            memory_usage: None,
            storage_usage: None,
            network_usage: None,
            measurement_timestamp: now,
        }),
        privacy_level,
        proxy_address,
        consensus_proofs: Vec::new(),
        owner_certificate_fingerprint: certificate_fingerprint,
        metadata,
        health_status: AssetHealthStatus::default(),
        performance_metrics: AssetPerformanceMetrics::default(),
    };
    
    status
}

/// Get supported privacy levels for all adapters
pub fn get_supported_privacy_levels() -> Vec<PrivacyLevel> {
    vec![
        PrivacyLevel::Private,
        PrivacyLevel::PrivateNetwork,
        PrivacyLevel::P2P,
        PrivacyLevel::PublicNetwork,
        PrivacyLevel::FullPublic,
    ]
}