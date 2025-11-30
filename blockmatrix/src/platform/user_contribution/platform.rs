//! User contribution platform implementation

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, Mutex};
use uuid::Uuid;

use crate::assets::core::{AssetManager, AssetType, AssetAllocation, ConsensusProof};
use super::hardware::{HardwareConfiguration, CpuInfo, GpuInfo, MemoryInfo, MemoryModule, StorageInfo, StorageType, NetworkInfo, NetworkInterface, NetworkLocation, VerificationStatus};
use super::sharing::SharingPreferences;
use super::session::{UserId, ContributionId, ContributionSession, SessionEarnings, SessionPerformance, SessionStatus, AccountStatus};
use super::rewards::{RewardEngine, PlatformMetrics};
use super::pricing::PaymentFrequency;

/// User contribution platform for hardware sharing
pub struct UserContributionPlatform {
    asset_manager: Arc<AssetManager>,
    user_profiles: Arc<RwLock<HashMap<UserId, UserProfile>>>,
    active_contributions: Arc<RwLock<HashMap<ContributionId, ContributionSession>>>,
    reward_engine: Arc<RewardEngine>,
    metrics: Arc<Mutex<PlatformMetrics>>,
    config: PlatformConfig,
}

/// User profile with hardware and sharing preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: UserId,
    pub display_name: String,
    pub email: String,
    pub hardware_config: HardwareConfiguration,
    pub sharing_preferences: SharingPreferences,
    pub reputation_score: f64,
    pub total_earnings: u64,
    pub account_status: AccountStatus,
    pub registered_at: SystemTime,
    pub last_active: SystemTime,
}

/// Platform configuration
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    pub max_users_per_node: u32,
    pub verification_timeout: Duration,
    pub session_timeout: Duration,
    pub payout_frequency: PaymentFrequency,
    pub minimum_reputation: f64,
    pub maximum_resource_allocation: f64,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            max_users_per_node: 1000,
            verification_timeout: Duration::from_secs(3600),
            session_timeout: Duration::from_secs(86400),
            payout_frequency: PaymentFrequency::Daily,
            minimum_reputation: 0.5,
            maximum_resource_allocation: 0.8,
        }
    }
}

impl UserContributionPlatform {
    /// Create new user contribution platform
    pub async fn new(
        asset_manager: Arc<AssetManager>,
        config: PlatformConfig,
    ) -> Result<Self> {
        let reward_engine = Arc::new(RewardEngine::new());

        Ok(Self {
            asset_manager,
            user_profiles: Arc::new(RwLock::new(HashMap::new())),
            active_contributions: Arc::new(RwLock::new(HashMap::new())),
            reward_engine,
            metrics: Arc::new(Mutex::new(PlatformMetrics::default())),
            config,
        })
    }

    /// Register new user with hardware detection
    pub async fn register_user(
        &self,
        user_id: UserId,
        display_name: String,
        email: String,
    ) -> Result<UserProfile> {
        let hardware_config = self.detect_hardware_configuration().await?;
        let sharing_preferences = SharingPreferences::default_for_hardware(&hardware_config);

        let user_profile = UserProfile {
            user_id: user_id.clone(),
            display_name,
            email,
            hardware_config,
            sharing_preferences,
            reputation_score: 1.0,
            total_earnings: 0,
            account_status: AccountStatus::PendingVerification,
            registered_at: SystemTime::now(),
            last_active: SystemTime::now(),
        };

        {
            let mut profiles = self.user_profiles.write().await;
            profiles.insert(user_id, user_profile.clone());
        }

        {
            let mut metrics = self.metrics.lock().await;
            metrics.total_users += 1;
        }

        Ok(user_profile)
    }

    /// Start contribution session for user
    pub async fn start_contribution_session(
        &self,
        user_id: &UserId,
        resource_types: Vec<AssetType>,
        consensus_proof: ConsensusProof,
    ) -> Result<ContributionSession> {
        let user_profile = {
            let profiles = self.user_profiles.read().await;
            profiles.get(user_id)
                .ok_or_else(|| anyhow!("User not found"))?
                .clone()
        };

        self.verify_user_eligibility(&user_profile).await?;

        let mut asset_allocations = HashMap::new();
        for asset_type in resource_types {
            let allocation = self.allocate_user_resource(
                &user_profile,
                asset_type,
                &consensus_proof,
            ).await?;
            asset_allocations.insert(asset_type, allocation);
        }

        let session_id = Uuid::new_v4().to_string();
        let session = ContributionSession {
            session_id: session_id.clone(),
            user_id: user_id.clone(),
            asset_allocations,
            started_at: SystemTime::now(),
            expected_end: SystemTime::now() + Duration::from_secs(3600),
            actual_usage: HashMap::new(),
            earnings: SessionEarnings::default(),
            performance_metrics: SessionPerformance::default(),
            status: SessionStatus::Active,
        };

        {
            let mut sessions = self.active_contributions.write().await;
            sessions.insert(session_id, session.clone());
        }

        {
            let mut metrics = self.metrics.lock().await;
            metrics.active_users += 1;
        }

        Ok(session)
    }

    async fn detect_hardware_configuration(&self) -> Result<HardwareConfiguration> {
        Ok(HardwareConfiguration {
            cpu_info: CpuInfo {
                model: "AMD Ryzen 9 5950X".to_string(),
                cores: 16,
                threads: 32,
                base_frequency: 3400,
                max_frequency: 4900,
                cache_l1: 1024,
                cache_l2: 8192,
                cache_l3: 65536,
                architecture: "x86_64".to_string(),
                instruction_sets: vec!["AVX2".to_string(), "AVX512".to_string()],
            },
            gpu_info: vec![
                GpuInfo {
                    model: "NVIDIA RTX 4090".to_string(),
                    vendor: "NVIDIA".to_string(),
                    memory_usage: 24 * 1024 * 1024 * 1024,
                    compute_units: 128,
                    base_clock: 2520,
                    memory_clock: 10500,
                    memory_bus_width: 384,
                    compute_capability: Some("8.9".to_string()),
                    supported_apis: vec!["Nova".to_string(), "Vulkan".to_string(), "OpenCL".to_string()],
                }
            ],
            memory_info: MemoryInfo {
                total_capacity: 128 * 1024 * 1024 * 1024,
                available_capacity: 100 * 1024 * 1024 * 1024,
                memory_type: "DDR5".to_string(),
                speed: 5600,
                modules: vec![
                    MemoryModule {
                        size: 32 * 1024 * 1024 * 1024,
                        speed: 5600,
                        latency: "CL36".to_string(),
                        manufacturer: "G.Skill".to_string(),
                    }
                ],
            },
            storage_info: vec![
                StorageInfo {
                    device_type: StorageType::NVMe,
                    capacity: 2 * 1024 * 1024 * 1024 * 1024,
                    available: 1024 * 1024 * 1024 * 1024,
                    interface: "NVMe 4.0".to_string(),
                    read_speed: 7000,
                    write_speed: 6500,
                    manufacturer: "Samsung".to_string(),
                    model: "980 PRO".to_string(),
                }
            ],
            network_info: NetworkInfo {
                interfaces: vec![
                    NetworkInterface {
                        name: "eth0".to_string(),
                        interface_type: "Ethernet".to_string(),
                        speed: 10000,
                        mac_address: "00:11:22:33:44:55".to_string(),
                        ip_addresses: vec!["192.168.1.100".to_string()],
                    }
                ],
                bandwidth_upload: 1000,
                bandwidth_download: 1000,
                latency: 5,
                is_metered: false,
                location: NetworkLocation {
                    country: "United States".to_string(),
                    region: "California".to_string(),
                    city: "San Francisco".to_string(),
                    latitude: Some(37.7749),
                    longitude: Some(-122.4194),
                    timezone: "America/Los_Angeles".to_string(),
                },
            },
            verification_status: VerificationStatus::Pending,
        })
    }

    async fn verify_user_eligibility(&self, user_profile: &UserProfile) -> Result<()> {
        match user_profile.account_status {
            AccountStatus::Active => {},
            AccountStatus::Suspended => return Err(anyhow!("Account suspended")),
            AccountStatus::Banned => return Err(anyhow!("Account banned")),
            AccountStatus::PendingVerification => return Err(anyhow!("Account pending verification")),
            AccountStatus::Closed => return Err(anyhow!("Account closed")),
        }

        if user_profile.reputation_score < self.config.minimum_reputation {
            return Err(anyhow!("Reputation score too low"));
        }

        match user_profile.hardware_config.verification_status {
            VerificationStatus::Verified => {},
            _ => return Err(anyhow!("Hardware not verified")),
        }

        Ok(())
    }

    async fn allocate_user_resource(
        &self,
        user_profile: &UserProfile,
        asset_type: AssetType,
        consensus_proof: &ConsensusProof,
    ) -> Result<AssetAllocation> {
        let resource_settings = user_profile.sharing_preferences.resource_settings
            .get(&asset_type)
            .ok_or_else(|| anyhow!("Resource type not configured for sharing"))?;

        if !resource_settings.enabled {
            return Err(anyhow!("Resource sharing disabled for this type"));
        }

        let total_capacity = self.get_total_capacity_for_asset_type(&user_profile.hardware_config, &asset_type)?;
        let shared_capacity = (total_capacity as f64 * resource_settings.share_percentage / 100.0) as u64;

        let allocation_request = crate::assets::core::AssetAllocationRequest {
            asset_type,
            requested_resources: crate::assets::core::ResourceRequirements {
                cpu: None,
                gpu_usage: None,
                memory_usage: None,
                storage_usage: Some(shared_capacity),
                network_usage: None,
                container: None,
            },
            privacy_level: crate::assets::core::PrivacyLevel::Private,
            consensus_proof: consensus_proof.clone(),
            certificate_fingerprint: String::new(),
            duration_limit: Some(resource_settings.max_session_duration),
            tags: HashMap::new(),
        };

        self.asset_manager.allocate_asset(allocation_request).await
    }

    fn get_total_capacity_for_asset_type(
        &self,
        hardware_config: &HardwareConfiguration,
        asset_type: &AssetType,
    ) -> Result<u64> {
        match asset_type {
            AssetType::Cpu => Ok(hardware_config.cpu_info.threads as u64),
            AssetType::Memory => Ok(hardware_config.memory_info.total_capacity),
            AssetType::Gpu => Ok(hardware_config.gpu_info.len() as u64),
            AssetType::Storage => Ok(hardware_config.storage_info.iter()
                .map(|s| s.capacity)
                .sum()),
            AssetType::Network => Ok(hardware_config.network_info.bandwidth_upload),
            _ => Err(anyhow!("Unsupported asset type")),
        }
    }

    pub async fn get_metrics(&self) -> PlatformMetrics {
        let metrics = self.metrics.lock().await;
        metrics.clone()
    }

    pub async fn list_user_sessions(&self, user_id: &UserId) -> Vec<ContributionSession> {
        let sessions = self.active_contributions.read().await;
        sessions.values()
            .filter(|session| &session.user_id == user_id)
            .cloned()
            .collect()
    }
}
