// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Helper functions for asset adapters
//!
//! Common utilities and helper functions for creating consistent
//! asset allocations and status across all adapter implementations.

use std::collections::HashMap;
use std::time::SystemTime;

use crate::assets::core::privacy::{
    AccessConfig, AccessPermissions, AllocationConfig, AuthRequirements, ConcurrencyLimits,
    StateRequirements, DurationConfig, RateLimits, ResourceAllocationConfig,
};
use crate::assets::core::status::{AssetHealthStatus, AssetPerformanceMetrics};
use crate::assets::core::{
    AssetAllocation, AssetAllocationRequest, AssetRegistration, AssetState, AssetStatus,
    PrivacyMode, ProxyAddress, ResourceUsage,
};

/// Create a standard AssetAllocation for adapter responses
pub fn _create_asset_allocation(
    asset_id: AssetRegistration,
    request: &AssetAllocationRequest,
    proxy_address: Option<ProxyAddress>,
    metadata: HashMap<String, String>,
) -> AssetAllocation {
    // Create asset status
    let mut status = AssetStatus::new(
        asset_id.clone(),
        request.certificate_fingerprint.clone(),
        request.privacy_level,
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

    // Add state proof
    status.add_state_proof(request.state_proof.clone());

    // Create allocation config
    let allocation_config = AllocationConfig {
        privacy_level: request.privacy_level,
        resource_allocation: ResourceAllocationConfig::default(),
        concurrency_limits: ConcurrencyLimits::default(),
        duration_config: DurationConfig::default(),
        state_requirements: StateRequirements::default(),
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
            allocation
                .access_config
                .allowed_certificates
                .push(value.clone());
        }
    }

    allocation
}

/// Create a standard AssetStatus for adapter responses
pub fn _create_asset_status(
    asset_id: AssetRegistration,
    state: AssetState,
    privacy_level: PrivacyMode,
    certificate_fingerprint: String,
    resource_usage: Option<ResourceUsage>,
    proxy_address: Option<ProxyAddress>,
    metadata: HashMap<String, String>,
) -> AssetStatus {
    let now = SystemTime::now();

    AssetStatus {
        asset_id,
        state,
        allocated_at: now,
        last_accessed: now,
        resource_usage: resource_usage.unwrap_or(ResourceUsage {
            cpu_usage: None,
            gpu_usage: None,
            memory_usage: None,
            storage_usage: None,
            network_usage: None,
            measurement_timestamp: now,
        }),
        privacy_level,
        proxy_address,
        state_proofs: Vec::new(),
        owner_certificate_fingerprint: certificate_fingerprint,
        metadata,
        health_status: AssetHealthStatus::default(),
        performance_metrics: AssetPerformanceMetrics::default(),
    }
}

/// Get supported privacy levels for all adapters
pub fn _get_supported_privacy_levels() -> Vec<PrivacyMode> {
    vec![
        PrivacyMode::PRIVATE,
        PrivacyMode::PRIVATE,
        PrivacyMode::PRIVATE,
        PrivacyMode::PUBLIC,
        PrivacyMode::PUBLIC,
    ]
}
