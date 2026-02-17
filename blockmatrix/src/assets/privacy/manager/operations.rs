// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Privacy manager operations - allocation, validation, and enforcement logic.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::super::{
    PrivacyAllocationResult, ResourceAllocationConfig, ConsensusRequirementConfig,
    ProxyConfiguration, allocation_types::PrivacyAllocationType,
};
use crate::assets::core::{AssetId, AssetResult, AssetError, PrivacyLevel};
use crate::consensus::proof::ConsensusProof;
use crate::assets::proxy::RemoteProxyManager;

use super::types::*;

impl PrivacyManager {
    /// Create new privacy manager
    pub async fn new(
        config: PrivacyManagerConfig,
        proxy_manager: Option<Arc<RemoteProxyManager>>,
    ) -> AssetResult<Self> {
        let enforcer = Arc::new(super::super::PrivacyEnforcer::new(&config).await?);
        let reward_calculator = Arc::new(super::super::CaesarRewardCalculator::new(&config.base_reward_config).await?);

        Ok(Self {
            config,
            user_configs: Arc::new(RwLock::new(HashMap::new())),
            active_allocations: Arc::new(RwLock::new(HashMap::new())),
            proxy_manager,
            enforcer,
            reward_calculator,
            audit_logger: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Register user privacy configuration
    pub async fn register_user_config(
        &self,
        user_id: String,
        config: UserPrivacyConfiguration,
    ) -> AssetResult<()> {
        let mut user_configs = self.user_configs.write().await;
        user_configs.insert(user_id.clone(), config);

        self.log_privacy_event(
            PrivacyEventType::ConfigurationChanged,
            Some(user_id),
            None,
            HashMap::from([
                ("action".to_string(), "user_config_registered".to_string())
            ]),
            LogLevel::Info,
        ).await?;

        Ok(())
    }

    /// Allocate privacy-controlled asset access
    pub async fn allocate_privacy_controlled_access(
        &self,
        user_id: &str,
        asset_id: &AssetId,
        requested_privacy_level: Option<PrivacyLevel>,
        consensus_proof: Option<ConsensusProof>,
    ) -> AssetResult<PrivacyAllocationResult> {
        // Get user configuration
        let user_config = {
            let configs = self.user_configs.read().await;
            configs.get(user_id)
                .ok_or_else(|| AssetError::AdapterError {
                    message: format!("No privacy configuration found for user: {}", user_id)
                })?
                .clone()
        };

        // Determine privacy level
        let privacy_level = requested_privacy_level
            .unwrap_or(user_config.preferred_privacy_level.clone());

        // Validate consensus proof if required
        if let Some(proof) = &consensus_proof {
            if !proof.validate() {
                return Err(AssetError::AdapterError {
                    message: "Invalid consensus proof provided".to_string()
                });
            }
        }

        // Determine allocation type based on privacy level and user history
        let allocation_type = self.determine_allocation_type(
            &privacy_level,
            &user_config.privacy_history,
        ).await?;

        // Create resource allocation configuration
        let resource_config = self.create_resource_config(
            &user_config,
            &privacy_level,
            asset_id,
        ).await?;

        // Create consensus requirements
        let consensus_requirements = self.merge_consensus_requirements(
            &user_config.consensus_requirements,
            &privacy_level,
        ).await?;

        // Calculate CAESAR rewards
        let reward_config = self.reward_calculator.calculate_reward_config(
            &privacy_level,
            &resource_config,
            &user_config.reward_preferences,
        ).await?;

        // Configure proxy settings if enabled
        let proxy_config = if user_config.proxy_preferences.enabled {
            self.create_proxy_config(
                &user_config.proxy_preferences,
                &privacy_level,
                asset_id,
            ).await?
        } else {
            ProxyConfiguration::default()
        };

        // Generate allocation ID
        let allocation_id = Uuid::new_v4().to_string();

        // Create allocation result
        let allocation_result = PrivacyAllocationResult {
            asset_id: asset_id.clone(),
            allocation_type,
            privacy_level: privacy_level.clone(),
            resource_config,
            consensus_requirements,
            reward_config,
            proxy_config,
            allocated_at: SystemTime::now(),
            expires_at: Some(SystemTime::now() + Duration::from_secs(3600)), // 1 hour default
            allocation_id: allocation_id.clone(),
        };

        // Store allocation
        {
            let mut allocations = self.active_allocations.write().await;
            allocations.insert(allocation_id.clone(), allocation_result.clone());
        }

        // Log allocation event
        self.log_privacy_event(
            PrivacyEventType::AllocationCreated,
            Some(user_id.to_string()),
            Some(allocation_id.clone()),
            HashMap::from([
                ("privacy_level".to_string(), format!("{:?}", privacy_level)),
                ("asset_id".to_string(), asset_id.to_string()),
            ]),
            LogLevel::Info,
        ).await?;

        Ok(allocation_result)
    }

    /// Validate access to privacy-controlled resource
    pub async fn validate_access(
        &self,
        allocation_id: &str,
        requester_id: &str,
        access_type: &str,
    ) -> AssetResult<bool> {
        // Get allocation
        let allocation = {
            let allocations = self.active_allocations.read().await;
            allocations.get(allocation_id)
                .ok_or_else(|| AssetError::AdapterError {
                    message: format!("Allocation not found: {}", allocation_id)
                })?
                .clone()
        };

        // Check expiry
        if let Some(expires_at) = allocation.expires_at {
            if SystemTime::now() >= expires_at {
                self.log_privacy_event(
                    PrivacyEventType::AccessDenied,
                    Some(requester_id.to_string()),
                    Some(allocation_id.to_string()),
                    HashMap::from([
                        ("reason".to_string(), "allocation_expired".to_string())
                    ]),
                    LogLevel::Warn,
                ).await?;

                return Ok(false);
            }
        }

        // Validate with enforcer
        let validation_result = self.enforcer.validate_access(
            &allocation,
            requester_id,
            access_type,
        ).await?;

        if validation_result.allowed {
            self.log_privacy_event(
                PrivacyEventType::AccessGranted,
                Some(requester_id.to_string()),
                Some(allocation_id.to_string()),
                HashMap::from([
                    ("access_type".to_string(), access_type.to_string())
                ]),
                LogLevel::Info,
            ).await?;
        } else {
            self.log_privacy_event(
                PrivacyEventType::AccessDenied,
                Some(requester_id.to_string()),
                Some(allocation_id.to_string()),
                HashMap::from([
                    ("access_type".to_string(), access_type.to_string()),
                    ("reason".to_string(), validation_result.reason.unwrap_or_default()),
                ]),
                LogLevel::Warn,
            ).await?;
        }

        Ok(validation_result.allowed)
    }

    // Helper methods (implementation details)
    async fn determine_allocation_type(
        &self,
        privacy_level: &PrivacyLevel,
        privacy_history: &PrivacyHistory,
    ) -> AssetResult<PrivacyAllocationType> {
        match privacy_level {
            PrivacyLevel::Private => Ok(PrivacyAllocationType::Private),
            PrivacyLevel::FullPublic => {
                if privacy_history.violations.is_empty() {
                    Ok(PrivacyAllocationType::Verified)
                } else {
                    Ok(PrivacyAllocationType::Public)
                }
            },
            _ => Ok(PrivacyAllocationType::Public),
        }
    }

    async fn create_resource_config(
        &self,
        user_config: &UserPrivacyConfiguration,
        _privacy_level: &PrivacyLevel,
        asset_id: &AssetId,
    ) -> AssetResult<ResourceAllocationConfig> {
        let asset_type = asset_id.asset_type()
            .map(|at| format!("{:?}", at).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());

        let resource_privacy = user_config.resource_privacy_settings
            .get(&asset_type)
            .cloned()
            .unwrap_or_else(|| ResourcePrivacyConfig::default_for_type(&asset_type));

        Ok(ResourceAllocationConfig {
            cpu_percentage: resource_privacy.allocation_percentage,
            gpu_percentage: resource_privacy.allocation_percentage,
            memory_percentage: resource_privacy.allocation_percentage,
            storage_percentage: resource_privacy.allocation_percentage,
            network_percentage: resource_privacy.allocation_percentage,
            max_concurrent_users: resource_privacy.max_concurrent_access,
            max_concurrent_processes: resource_privacy.max_concurrent_access * 10,
            duration_config: resource_privacy.duration_limits,
        })
    }

    async fn merge_consensus_requirements(
        &self,
        user_requirements: &ConsensusRequirementConfig,
        privacy_level: &PrivacyLevel,
    ) -> AssetResult<ConsensusRequirementConfig> {
        let mut merged = user_requirements.clone();

        match privacy_level {
            PrivacyLevel::Private => {
                merged.require_proof_of_work = false;
                merged.minimum_stake = 0;
            },
            PrivacyLevel::FullPublic => {
                merged.require_proof_of_space = true;
                merged.require_proof_of_stake = true;
                merged.require_proof_of_work = true;
                merged.require_proof_of_time = true;
                merged.minimum_stake = merged.minimum_stake.max(1000);
            },
            _ => {
                merged.minimum_stake = merged.minimum_stake.max(100);
            }
        }

        Ok(merged)
    }

    async fn create_proxy_config(
        &self,
        _proxy_preferences: &ProxyPreferences,
        _privacy_level: &PrivacyLevel,
        _asset_id: &AssetId,
    ) -> AssetResult<ProxyConfiguration> {
        Ok(ProxyConfiguration::default())
    }

    async fn log_privacy_event(
        &self,
        event_type: PrivacyEventType,
        user_id: Option<String>,
        allocation_id: Option<String>,
        details: HashMap<String, String>,
        severity: LogLevel,
    ) -> AssetResult<()> {
        if !self.config.audit_logging.enabled {
            return Ok(());
        }

        let entry = PrivacyAuditEntry {
            timestamp: SystemTime::now(),
            user_id: if self.config.audit_logging.anonymize_logs {
                None
            } else {
                user_id
            },
            event_type,
            details,
            severity,
            allocation_id,
        };

        let mut logger = self.audit_logger.write().await;
        logger.push(entry);

        Ok(())
    }
}

impl ResourcePrivacyConfig {
    fn default_for_type(resource_type: &str) -> Self {
        Self {
            resource_type: resource_type.to_string(),
            privacy_level: PrivacyLevel::P2P,
            allocation_percentage: 0.5, // 50% default allocation
            max_concurrent_access: 5,
            duration_limits: super::super::DurationLimits::default(),
            access_rules: Vec::new(),
        }
    }
}

impl Default for ProxyConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            nat_preferences: super::super::NatAddressingPreferences::default(),
            node_selection: super::super::ProxyNodeSelection::default(),
            quantum_security: super::super::QuantumSecurityConfig::default(),
            trust_requirements: super::super::TrustRequirements::default(),
        }
    }
}
