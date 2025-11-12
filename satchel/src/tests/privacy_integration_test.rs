//! Privacy System Integration Tests
//!
//! Tests the complete privacy management system including:
//! - Privacy manager and allocation
//! - NKrypt allocation types  
//! - CAESAR reward calculation
//! - Privacy enforcement
//! - User configuration

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use hypermesh_assets::core::{
    AssetId, AssetType, PrivacyLevel,
    ConsensusProof, SpaceProof, StakeProof, WorkProof, TimeProof,
    WorkloadType, WorkState
};

use hypermesh_assets::privacy::{
    PrivacyManager, PrivacyManagerConfig, 
    PrivacyAllocationType, UserPrivacyConfiguration,
    CaesarRewardCalculator, PrivacyEnforcer,
    UserPrivacyConfig, PrivacySettings, ResourcePrivacySettings,
    PrivacyConstraints, PrivacyValidationRules,
    EnforcementStrictness, AuditLoggingConfig,
    CaesarRewardConfig, RewardDistributionConfig, PayoutFrequency
};

/// Test the complete privacy allocation workflow
#[tokio::test]
async fn test_privacy_allocation_workflow() {
    // Create privacy manager configuration
    let manager_config = PrivacyManagerConfig {
        default_privacy_level: PrivacyLevel::P2P,
        default_resource_allocation: super::ResourceAllocationConfig::default(),
        global_consensus_requirements: super::ConsensusRequirementConfig::default(),
        base_reward_config: CaesarRewardConfig {
            base_reward_rate: 10.0,
            privacy_multiplier: 1.0,
            utilization_multiplier: 1.0,
            consensus_bonus: 0.1,
            max_reward_cap: 1000.0,
            distribution_config: RewardDistributionConfig {
                immediate_payout: true,
                immediate_percentage: 1.0,
                auto_stake_remainder: false,
                minimum_payout_threshold: 1.0,
                payout_frequency: PayoutFrequency::Immediate,
            },
        },
        proxy_integration_enabled: true,
        enforcement_strictness: EnforcementStrictness::Moderate,
        audit_logging: AuditLoggingConfig {
            enabled: true,
            log_all_events: true,
            log_violations_only: false,
            retention_period: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            anonymize_logs: false,
        },
    };
    
    // Create privacy manager
    let privacy_manager = PrivacyManager::new(manager_config, None).await.unwrap();
    
    // Create user privacy configuration
    let user_config = UserPrivacyConfiguration {
        user_id: "test-user-123".to_string(),
        preferred_privacy_level: PrivacyLevel::PublicNetwork,
        resource_privacy_settings: HashMap::new(),
        consensus_requirements: super::ConsensusRequirementConfig::default(),
        reward_preferences: super::manager::CaesarRewardPreferences {
            enabled: true,
            minimum_reward_rate: 5.0,
            payout_frequency: PayoutFrequency::Immediate,
            auto_stake_percentage: 0.2, // 20% auto-stake
            optimization_preferences: super::manager::RewardOptimizationPreferences {
                optimize_for_maximum_rewards: true,
                balance_rewards_privacy: false,
                reward_privacy_ratio: 0.8, // Prefer rewards
                accept_dynamic_adjustments: true,
            },
        },
        proxy_preferences: super::manager::ProxyPreferences {
            enabled: true,
            preferred_proxy_types: vec!["http".to_string(), "socks5".to_string()],
            geographic_preferences: vec!["us-west".to_string()],
            trust_requirements: super::TrustRequirements::default(),
            performance_requirements: super::manager::ProxyPerformanceRequirements {
                max_latency_ms: 100,
                min_bandwidth_mbps: 100,
                min_uptime_percentage: 0.99,
                max_connection_time_ms: 5000,
            },
        },
        allocation_constraints: super::manager::AllocationConstraints {
            max_total_allocations: 10,
            max_per_resource_type: HashMap::from([
                ("cpu".to_string(), 4),
                ("memory".to_string(), 8),
                ("gpu".to_string(), 2),
            ]),
            max_allocation_duration: Duration::from_secs(24 * 60 * 60), // 24 hours
            allocation_cooldown: Duration::from_secs(5 * 60), // 5 minutes
            budget_constraints: super::manager::BudgetConstraints {
                max_tokens_per_allocation: 100.0,
                max_tokens_per_period: 1000.0,
                budget_period: Duration::from_secs(24 * 60 * 60), // Daily budget
                auto_renewal_budget: 500.0,
            },
        },
        privacy_history: super::manager::PrivacyHistory {
            total_allocations: 0,
            privacy_level_usage: HashMap::new(),
            resource_usage: HashMap::new(),
            violations: Vec::new(),
            preference_evolution: Vec::new(),
        },
    };
    
    // Register user configuration
    privacy_manager.register_user_config("test-user-123".to_string(), user_config).await.unwrap();
    
    // Create test asset
    let asset_id = AssetId::new(AssetType::Cpu);
    
    // Create consensus proof (all four proofs required)
    let space_proof = SpaceProof {
        node_id: "test-node".to_string(),
        storage_path: "/test/storage".to_string(),
        allocated_size: 1_000_000_000, // 1GB
        proof_hash: vec![1, 2, 3, 4, 5, 6, 7, 8],
        timestamp: SystemTime::now(),
    };
    
    let stake_proof = StakeProof {
        stake_holder: "test-user-123".to_string(),
        stake_holder_id: "user-123-id".to_string(),
        stake_amount: 1500, // Above minimum
        stake_timestamp: SystemTime::now(),
    };
    
    let work_proof = WorkProof {
        worker_id: "test-worker".to_string(),
        workload_id: "cpu-allocation-work".to_string(),
        process_id: 12345,
        computational_power: 200, // Above minimum
        workload_type: WorkloadType::Compute,
        work_state: WorkState::Completed,
    };
    
    let time_proof = TimeProof {
        network_time_offset: Duration::from_secs(10), // Within tolerance
        time_verification_timestamp: SystemTime::now(),
        nonce: 42,
        proof_hash: vec![9, 10, 11, 12, 13, 14, 15, 16],
    };
    
    let consensus_proof = ConsensusProof::new(space_proof, stake_proof, work_proof, time_proof);
    
    // Request privacy-controlled asset allocation
    let allocation_result = privacy_manager.allocate_privacy_controlled_access(
        "test-user-123",
        &asset_id,
        Some(PrivacyLevel::PublicNetwork), // Override default
        Some(consensus_proof),
    ).await.unwrap();
    
    // Verify allocation result
    assert_eq!(allocation_result.asset_id, asset_id);
    assert_eq!(allocation_result.privacy_level, PrivacyLevel::PublicNetwork);
    assert!(matches!(allocation_result.allocation_type, PrivacyAllocationType::Public));
    
    // Verify resource configuration
    assert!(allocation_result.resource_config.cpu_percentage > 0.0);
    assert!(allocation_result.resource_config.max_concurrent_users > 0);
    
    // Verify consensus requirements
    assert!(allocation_result.consensus_requirements.require_proof_of_space);
    assert!(allocation_result.consensus_requirements.require_proof_of_stake);
    assert!(allocation_result.consensus_requirements.require_proof_of_work);
    assert!(allocation_result.consensus_requirements.require_proof_of_time);
    
    // Verify CAESAR reward configuration
    assert!(allocation_result.reward_config.base_reward_rate > 0.0);
    assert!(allocation_result.reward_config.privacy_multiplier > 0.0);
    
    // Test access validation
    let access_allowed = privacy_manager.validate_access(
        &allocation_result.allocation_id,
        "test-user-123",
        "read",
    ).await.unwrap();
    
    assert!(access_allowed, "User should have access to their own allocation");
    
    // Test access denial for unauthorized user
    let access_denied = privacy_manager.validate_access(
        &allocation_result.allocation_id,
        "unauthorized-user",
        "read",
    ).await.unwrap();
    
    assert!(!access_denied, "Unauthorized user should be denied access");
    
    println!("Privacy allocation workflow test completed successfully");
}

/// Test NKrypt allocation types and transitions
#[tokio::test]
async fn test_allocation_types_and_transitions() {
    // Test all four NKrypt allocation types
    let private = PrivacyAllocationType::Private;
    let public = PrivacyAllocationType::Public;
    let anonymous = PrivacyAllocationType::Anonymous;
    let verified = PrivacyAllocationType::Verified;
    
    // Test descriptions
    assert!(private.description().contains("Internal use only"));
    assert!(public.description().contains("Cross-network accessible"));
    assert!(anonymous.description().contains("No identity tracking"));
    assert!(verified.description().contains("Full consensus validation"));
    
    // Test capabilities
    assert!(!private.supports_remote_access());
    assert!(public.supports_remote_access());
    assert!(anonymous.supports_remote_access());
    assert!(verified.supports_remote_access());
    
    assert!(private.supports_identity_tracking());
    assert!(public.supports_identity_tracking());
    assert!(!anonymous.supports_identity_tracking()); // Key difference
    assert!(verified.supports_identity_tracking());
    
    // Test consensus proof requirements
    assert!(!private.requires_consensus_proof());
    assert!(!public.requires_consensus_proof());
    assert!(!anonymous.requires_consensus_proof());
    assert!(verified.requires_consensus_proof()); // Only verified requires full consensus
    
    // Test privacy level mappings
    assert_eq!(private.minimum_privacy_level(), PrivacyLevel::Private);
    assert_eq!(public.minimum_privacy_level(), PrivacyLevel::PublicNetwork);
    assert_eq!(anonymous.minimum_privacy_level(), PrivacyLevel::P2P);
    assert_eq!(verified.minimum_privacy_level(), PrivacyLevel::FullPublic);
    
    // Test CAESAR reward multipliers
    assert_eq!(private.base_reward_multiplier(), 0.0); // No rewards
    assert_eq!(public.base_reward_multiplier(), 0.75);
    assert_eq!(anonymous.base_reward_multiplier(), 0.5); // Lower for privacy
    assert_eq!(verified.base_reward_multiplier(), 1.0); // Maximum rewards
    
    // Test valid transitions
    assert!(private.can_transition_to(&public));
    assert!(private.can_transition_to(&verified));
    assert!(!private.can_transition_to(&anonymous)); // Can't go from identified to anonymous
    
    assert!(public.can_transition_to(&private));
    assert!(public.can_transition_to(&verified));
    assert!(!public.can_transition_to(&anonymous)); // Can't go from public to anonymous
    
    assert!(anonymous.can_transition_to(&private));
    assert!(anonymous.can_transition_to(&verified));
    assert!(!anonymous.can_transition_to(&public)); // Can't expose identity
    
    assert!(verified.can_transition_to(&private));
    assert!(verified.can_transition_to(&public));
    assert!(verified.can_transition_to(&anonymous));
    
    println!("Allocation types and transitions test completed successfully");
}

/// Test CAESAR reward calculation system
#[tokio::test]
async fn test_caesar_reward_calculation() {
    let base_config = CaesarRewardConfig {
        base_reward_rate: 10.0,
        privacy_multiplier: 1.0,
        utilization_multiplier: 1.0,
        consensus_bonus: 0.1,
        max_reward_cap: 1000.0,
        distribution_config: RewardDistributionConfig {
            immediate_payout: false,
            immediate_percentage: 0.8, // 80% immediate
            auto_stake_remainder: true, // 20% staked
            minimum_payout_threshold: 1.0,
            payout_frequency: PayoutFrequency::Daily,
        },
    };
    
    let reward_calculator = CaesarRewardCalculator::new(&base_config).await.unwrap();
    
    // Test reward calculation for different privacy levels
    let resource_config = super::ResourceAllocationConfig::default();
    let user_preferences = super::manager::CaesarRewardPreferences {
        enabled: true,
        minimum_reward_rate: 5.0,
        payout_frequency: PayoutFrequency::Daily,
        auto_stake_percentage: 0.2,
        optimization_preferences: super::manager::RewardOptimizationPreferences {
            optimize_for_maximum_rewards: true,
            balance_rewards_privacy: false,
            reward_privacy_ratio: 0.8,
            accept_dynamic_adjustments: true,
        },
    };
    
    // Test Full Public (maximum rewards)
    let full_public_config = reward_calculator.calculate_reward_config(
        &PrivacyLevel::FullPublic,
        &resource_config,
        &user_preferences,
    ).await.unwrap();
    
    assert_eq!(full_public_config.privacy_multiplier, 1.0);
    assert!(full_public_config.base_reward_rate > 0.0);
    
    // Test Private (no rewards)
    let private_config = reward_calculator.calculate_reward_config(
        &PrivacyLevel::Private,
        &resource_config,
        &user_preferences,
    ).await.unwrap();
    
    assert_eq!(private_config.privacy_multiplier, 0.0);
    
    // Test P2P (intermediate rewards)
    let p2p_config = reward_calculator.calculate_reward_config(
        &PrivacyLevel::P2P,
        &resource_config,
        &user_preferences,
    ).await.unwrap();
    
    assert!(p2p_config.privacy_multiplier > 0.0);
    assert!(p2p_config.privacy_multiplier < 1.0);
    
    // Test actual reward calculation
    let allocation_duration = Duration::from_secs(2 * 60 * 60); // 2 hours
    let resource_utilization = HashMap::from([
        ("cpu".to_string(), 0.8_f32),
        ("memory".to_string(), 0.6_f32),
        ("storage".to_string(), 0.4_f32),
    ]);
    let performance_metrics = HashMap::from([
        ("Uptime".to_string(), 0.99_f32),
        ("ResponseTime".to_string(), 50.0_f32), // 50ms average
    ]);
    
    let reward_result = reward_calculator.calculate_actual_rewards(
        allocation_duration,
        &resource_utilization,
        &PrivacyLevel::FullPublic,
        &performance_metrics,
        "Silver", // User tier
    ).await.unwrap();
    
    assert!(reward_result.final_reward > 0.0);
    assert!(reward_result.final_reward >= reward_result.base_reward);
    assert!(reward_result.breakdown.privacy_multiplier > 0.0);
    
    println!("Reward calculation: Base: {:.2}, Final: {:.2}", 
             reward_result.base_reward, reward_result.final_reward);
    
    println!("CAESAR reward calculation test completed successfully");
}

/// Test privacy enforcement system
#[tokio::test]
async fn test_privacy_enforcement() {
    let manager_config = PrivacyManagerConfig {
        default_privacy_level: PrivacyLevel::P2P,
        default_resource_allocation: super::ResourceAllocationConfig::default(),
        global_consensus_requirements: super::ConsensusRequirementConfig::default(),
        base_reward_config: CaesarRewardConfig::default(),
        proxy_integration_enabled: false,
        enforcement_strictness: EnforcementStrictness::Strict,
        audit_logging: AuditLoggingConfig {
            enabled: true,
            log_all_events: true,
            log_violations_only: false,
            retention_period: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            anonymize_logs: false,
        },
    };
    
    let enforcer = PrivacyEnforcer::new(&manager_config).await.unwrap();
    
    // Create test allocation
    let asset_id = AssetId::new(AssetType::Memory);
    let allocation_result = super::PrivacyAllocationResult {
        asset_id: asset_id.clone(),
        allocation_type: PrivacyAllocationType::Public,
        privacy_level: PrivacyLevel::PublicNetwork,
        resource_config: super::ResourceAllocationConfig::default(),
        consensus_requirements: super::ConsensusRequirementConfig::default(),
        reward_config: CaesarRewardConfig::default(),
        proxy_config: super::ProxyConfiguration::default(),
        allocated_at: SystemTime::now(),
        expires_at: Some(SystemTime::now() + Duration::from_secs(60 * 60)), // 1 hour
        allocation_id: "test-allocation-123".to_string(),
    };
    
    // Test valid access
    let access_result = enforcer.validate_access(
        &allocation_result,
        "authorized-user",
        "read",
    ).await.unwrap();
    
    assert!(access_result.allowed, "Access should be allowed for valid request");
    assert!(access_result.risk_assessment.is_some());
    
    // Test privacy violation handling
    let violation = super::enforcement::PrivacyViolation {
        violation_id: "violation-123".to_string(),
        timestamp: SystemTime::now(),
        violation_type: super::enforcement::PrivacyViolationType::UnauthorizedAccess,
        severity: super::enforcement::ViolationSeverity::Medium,
        user_id: "violating-user".to_string(),
        resource_id: asset_id.to_string(),
        details: super::enforcement::ViolationDetails {
            description: "Unauthorized access attempt".to_string(),
            evidence: vec![],
            impact: super::enforcement::ViolationImpact {
                privacy_impact: super::enforcement::ImpactLevel::Medium,
                security_impact: super::enforcement::ImpactLevel::Low,
                users_affected: 1,
                data_types_exposed: vec!["memory_data".to_string()],
                potential_consequences: vec!["Data exposure".to_string()],
            },
            root_cause: Some("Weak access controls".to_string()),
            metadata: HashMap::new(),
        },
        response: None,
        resolution_status: super::enforcement::ResolutionStatus::Open,
    };
    
    enforcer.record_violation(violation).await.unwrap();
    
    println!("Privacy enforcement test completed successfully");
}

/// Test user privacy configuration validation
#[test]
fn test_user_privacy_config_validation() {
    let mut config = UserPrivacyConfig::default();
    config.user_id = "test-user".to_string();
    
    // Test basic validation
    let warnings = config.validate().unwrap();
    assert!(warnings.is_empty(), "Default config should have no validation warnings");
    
    // Test conflicting settings
    config.privacy_settings.privacy_mode = super::config::PrivacyMode::MaximumPrivacy;
    config.privacy_settings.default_privacy_level = PrivacyLevel::FullPublic;
    
    let warnings = config.validate().unwrap();
    assert!(!warnings.is_empty(), "Conflicting settings should produce warnings");
    assert!(warnings[0].contains("Maximum privacy mode conflicts"));
    
    // Test resource allocation validation
    let mut resource_settings = super::config::ResourceTypePrivacySettings::default("cpu".to_string());
    resource_settings.default_allocation = 1.5; // 150% - invalid
    
    config.resource_settings.per_resource_settings.insert("cpu".to_string(), resource_settings);
    
    let warnings = config.validate().unwrap();
    let has_allocation_warning = warnings.iter().any(|w| w.contains("allocation exceeds 100%"));
    assert!(has_allocation_warning, "Should warn about allocation exceeding 100%");
    
    println!("User privacy configuration validation test completed successfully");
}

impl Default for super::ResourceAllocationConfig {
    fn default() -> Self {
        Self {
            cpu_percentage: 1.0,
            gpu_percentage: 1.0,
            memory_percentage: 1.0,
            storage_percentage: 1.0,
            network_percentage: 1.0,
            max_concurrent_users: 10,
            max_concurrent_processes: 100,
            duration_config: super::DurationLimits::default(),
        }
    }
}

impl Default for super::ConsensusRequirementConfig {
    fn default() -> Self {
        Self {
            require_proof_of_space: true,
            require_proof_of_stake: true,
            require_proof_of_work: true,
            require_proof_of_time: true,
            minimum_stake: 1000,
            max_time_offset: Duration::from_secs(30),
            validation_frequency: Duration::from_secs(5 * 60), // 5 minutes
            difficulty_requirements: super::DifficultyRequirements::default(),
        }
    }
}

impl Default for super::DifficultyRequirements {
    fn default() -> Self {
        Self {
            work_difficulty: 16,
            space_commitment: 1_000_000_000, // 1GB
            stake_multiplier: 1.0,
            time_precision_ms: 100,
        }
    }
}

impl Default for super::DurationLimits {
    fn default() -> Self {
        Self {
            max_total_duration: Some(Duration::from_secs(24 * 60 * 60)), // 24 hours
            max_session_duration: Some(Duration::from_secs(4 * 60 * 60)), // 4 hours
            cooldown_duration: Duration::from_secs(5 * 60), // 5 minutes
            grace_period: Duration::from_secs(5 * 60), // 5 minutes
            auto_renewal: Some(super::AutoRenewalConfig::default()),
        }
    }
}

impl Default for super::AutoRenewalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_renewals: 3,
            renewal_threshold: Duration::from_secs(10 * 60), // 10 minutes
            require_confirmation: true,
        }
    }
}

impl Default for CaesarRewardConfig {
    fn default() -> Self {
        Self {
            base_reward_rate: 10.0,
            privacy_multiplier: 1.0,
            utilization_multiplier: 1.0,
            consensus_bonus: 0.1,
            max_reward_cap: 1000.0,
            distribution_config: RewardDistributionConfig::default(),
        }
    }
}

impl Default for RewardDistributionConfig {
    fn default() -> Self {
        Self {
            immediate_payout: true,
            immediate_percentage: 1.0,
            auto_stake_remainder: false,
            minimum_payout_threshold: 1.0,
            payout_frequency: PayoutFrequency::Immediate,
        }
    }
}

impl Default for super::ProxyConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            nat_preferences: super::NatAddressingPreferences::default(),
            node_selection: super::ProxyNodeSelection::default(),
            quantum_security: super::QuantumSecurityConfig::default(),
            trust_requirements: super::TrustRequirements::default(),
        }
    }
}

impl Default for super::TrustRequirements {
    fn default() -> Self {
        Self {
            min_trust_score: 0.7,
            require_certificate_validation: true,
            require_consensus_validation: false,
            trust_decay: super::TrustDecayConfig::default(),
            reputation_requirements: super::ReputationRequirements::default(),
        }
    }
}

impl Default for super::TrustDecayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            decay_rate_per_day: 0.01, // 1% per day
            minimum_trust_floor: 0.1, // 10% minimum
            refresh_requirements: super::TrustRefreshRequirements::default(),
        }
    }
}

impl Default for super::TrustRefreshRequirements {
    fn default() -> Self {
        Self {
            refresh_frequency: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            refresh_actions: vec!["successful_operation".to_string()],
            success_boost: 0.05, // 5% boost
            failure_penalty: 0.1, // 10% penalty
        }
    }
}

impl Default for super::ReputationRequirements {
    fn default() -> Self {
        Self {
            min_reputation_score: 0.8,
            min_successful_operations: 10,
            max_failure_rate: 0.05, // 5% failure rate max
            recent_performance_weight: 0.7, // 70% weight on recent performance
        }
    }
}

impl Default for super::NatAddressingPreferences {
    fn default() -> Self {
        Self {
            preferred_networks: vec!["192.168.0.0/16".to_string()],
            port_preferences: super::PortAllocationPreferences::default(),
            prefer_ipv6: true,
            enable_upnp: false,
            persistence_config: super::ConnectionPersistenceConfig::default(),
        }
    }
}

impl Default for super::PortAllocationPreferences {
    fn default() -> Self {
        Self {
            preferred_ranges: vec![super::PortRange { start: 8000, end: 9000 }],
            avoid_well_known: true,
            use_random_allocation: false,
            binding_timeout: Duration::from_secs(30),
        }
    }
}

impl Default for super::ConnectionPersistenceConfig {
    fn default() -> Self {
        Self {
            keep_alive: true,
            connection_timeout: Duration::from_secs(30),
            max_idle_time: Duration::from_secs(5 * 60), // 5 minutes
            max_reconnect_attempts: 3,
        }
    }
}

impl Default for super::ProxyNodeSelection {
    fn default() -> Self {
        Self {
            min_trust_score: 0.8,
            required_capabilities: vec!["http_proxy".to_string()],
            geographic_preferences: Vec::new(),
            min_bandwidth_mbps: 100,
            max_latency_ms: 100,
            load_balancing: super::LoadBalancingPreferences::default(),
        }
    }
}

impl Default for super::LoadBalancingPreferences {
    fn default() -> Self {
        Self {
            strategy: super::LoadBalancingStrategy::LeastConnections,
            max_utilization_threshold: 0.8,
            enable_failover: true,
            health_check_frequency: Duration::from_secs(30),
        }
    }
}

impl Default for super::QuantumSecurityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            use_falcon_signatures: false,
            use_kyber_encryption: false,
            qkd_enabled: false,
            security_level: super::QuantumSecurityLevel::Basic,
        }
    }
}