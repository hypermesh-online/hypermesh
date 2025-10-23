//! User Contribution Platform - Interface for hardware sharing and asset contribution
//!
//! This module provides the user-facing interface for contributing hardware resources
//! to the HyperMesh network, managing rewards through Caesar integration, and
//! configuring privacy settings for resource sharing.

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, Mutex};
use uuid::Uuid;

use crate::assets::core::{
    AssetManager, AssetId, AssetType, AssetAllocation, PrivacyLevel,
    AssetStatus, AssetState, ConsensusProof,
};
use crate::assets::adapters::{
    CpuAssetAdapter, GpuAssetAdapter, MemoryAssetAdapter, StorageAssetAdapter,
};
use crate::catalog::vm::{PrivacyConfig, ResourceSharingConfig};

/// User contribution platform for hardware sharing
pub struct UserContributionPlatform {
    /// Asset management system
    asset_manager: Arc<AssetManager>,
    /// User profiles and configurations
    user_profiles: Arc<RwLock<HashMap<UserId, UserProfile>>>,
    /// Active contributions tracking
    active_contributions: Arc<RwLock<HashMap<ContributionId, ContributionSession>>>,
    /// Reward calculation engine
    reward_engine: Arc<RewardEngine>,
    /// Platform metrics
    metrics: Arc<Mutex<PlatformMetrics>>,
    /// Configuration
    config: PlatformConfig,
}

/// User identifier
pub type UserId = String;

/// Contribution session identifier
pub type ContributionId = String;

/// User profile with hardware and sharing preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// User identifier
    pub user_id: UserId,
    /// User display name
    pub display_name: String,
    /// Email address
    pub email: String,
    /// Hardware configuration
    pub hardware_config: HardwareConfiguration,
    /// Sharing preferences
    pub sharing_preferences: SharingPreferences,
    /// Reputation score
    pub reputation_score: f64,
    /// Total earnings
    pub total_earnings: u64,
    /// Account status
    pub account_status: AccountStatus,
    /// Registration timestamp
    pub registered_at: SystemTime,
    /// Last active timestamp
    pub last_active: SystemTime,
}

/// Hardware configuration detected/configured by user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfiguration {
    /// CPU information
    pub cpu_info: CpuInfo,
    /// GPU information
    pub gpu_info: Vec<GpuInfo>,
    /// Memory information
    pub memory_info: MemoryInfo,
    /// Storage information
    pub storage_info: Vec<StorageInfo>,
    /// Network information
    pub network_info: NetworkInfo,
    /// Hardware verification status
    pub verification_status: VerificationStatus,
}

/// CPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model: String,
    pub cores: u32,
    pub threads: u32,
    pub base_frequency: u64, // MHz
    pub max_frequency: u64,  // MHz
    pub cache_l1: u64,       // KB
    pub cache_l2: u64,       // KB
    pub cache_l3: u64,       // KB
    pub architecture: String,
    pub instruction_sets: Vec<String>,
}

/// GPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub model: String,
    pub vendor: String,
    pub memory: u64,         // Bytes
    pub compute_units: u32,
    pub base_clock: u64,     // MHz
    pub memory_clock: u64,   // MHz
    pub memory_bus_width: u32, // Bits
    pub compute_capability: Option<String>,
    pub supported_apis: Vec<String>,
}

/// Memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_capacity: u64, // Bytes
    pub available_capacity: u64, // Bytes
    pub memory_type: String, // DDR4, DDR5, etc.
    pub speed: u64,          // MHz
    pub modules: Vec<MemoryModule>,
}

/// Memory module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryModule {
    pub size: u64,           // Bytes
    pub speed: u64,          // MHz
    pub latency: String,     // CAS latency
    pub manufacturer: String,
}

/// Storage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub device_type: StorageType,
    pub capacity: u64,       // Bytes
    pub available: u64,      // Bytes
    pub interface: String,   // SATA, NVMe, etc.
    pub read_speed: u64,     // MB/s
    pub write_speed: u64,    // MB/s
    pub manufacturer: String,
    pub model: String,
}

/// Storage device types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    HDD,
    SSD,
    NVMe,
    Optane,
    Network,
}

/// Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
    pub bandwidth_upload: u64,   // Mbps
    pub bandwidth_download: u64, // Mbps
    pub latency: u64,           // ms
    pub is_metered: bool,
    pub location: NetworkLocation,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub interface_type: String, // Ethernet, WiFi, etc.
    pub speed: u64,             // Mbps
    pub mac_address: String,
    pub ip_addresses: Vec<String>,
}

/// Network location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLocation {
    pub country: String,
    pub region: String,
    pub city: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: String,
}

/// Hardware verification status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Failed(String),
    Expired,
}

/// Sharing preferences configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingPreferences {
    /// Resource sharing settings by type
    pub resource_settings: HashMap<AssetType, ResourceSharingSettings>,
    /// Global privacy level
    pub privacy_level: PrivacyLevel,
    /// Operating hours for sharing
    pub operating_hours: OperatingHours,
    /// Performance preferences
    pub performance_preferences: PerformancePreferences,
    /// Pricing configuration
    pub pricing_config: PricingConfiguration,
    /// Notification preferences
    pub notification_preferences: NotificationPreferences,
}

/// Resource sharing settings for specific asset type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSharingSettings {
    /// Enable sharing for this resource
    pub enabled: bool,
    /// Percentage of resource to share (0-100)
    pub share_percentage: f64,
    /// Privacy level for this resource
    pub privacy_level: PrivacyLevel,
    /// Maximum concurrent users
    pub max_concurrent_users: u32,
    /// Maximum session duration
    pub max_session_duration: Duration,
    /// Minimum price per unit
    pub min_price_per_unit: f64,
    /// Resource-specific constraints
    pub constraints: ResourceConstraints,
}

/// Resource-specific constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    /// CPU constraints
    pub cpu_constraints: Option<CpuConstraints>,
    /// GPU constraints
    pub gpu_constraints: Option<GpuConstraints>,
    /// Memory constraints
    pub memory_constraints: Option<MemoryConstraints>,
    /// Storage constraints
    pub storage_constraints: Option<StorageConstraints>,
}

/// CPU sharing constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuConstraints {
    pub max_threads_per_user: u32,
    pub allowed_instruction_sets: Vec<String>,
    pub priority_level: CpuPriority,
    pub thermal_limit: Option<u32>, // Celsius
}

/// CPU priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CpuPriority {
    Low,
    Normal,
    High,
    Realtime,
}

/// GPU sharing constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConstraints {
    pub max_memory_per_user: u64,
    pub allowed_compute_types: Vec<ComputeType>,
    pub max_concurrent_kernels: u32,
    pub power_limit: Option<u32>, // Watts
}

/// GPU compute types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputeType {
    Graphics,
    GeneralPurpose,
    MachineLearning,
    Cryptocurrency,
    Scientific,
}

/// Memory sharing constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConstraints {
    pub max_allocation_per_user: u64,
    pub memory_protection_level: MemoryProtectionLevel,
    pub allowed_access_patterns: Vec<AccessPattern>,
}

/// Memory protection levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryProtectionLevel {
    Basic,
    Isolated,
    Encrypted,
    SecureEnclave,
}

/// Memory access patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessPattern {
    Sequential,
    Random,
    Streaming,
    Cached,
}

/// Storage sharing constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConstraints {
    pub max_storage_per_user: u64,
    pub allowed_file_types: Vec<String>,
    pub encryption_required: bool,
    pub backup_required: bool,
    pub retention_policy: RetentionPolicy,
}

/// Data retention policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    DeleteAfterSession,
    RetainFor(Duration),
    UserControlled,
    Permanent,
}

/// Operating hours configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingHours {
    /// Weekly schedule
    pub schedule: HashMap<Weekday, TimeRange>,
    /// Timezone
    pub timezone: String,
    /// Enable 24/7 sharing
    pub always_on: bool,
    /// Emergency override settings
    pub emergency_override: EmergencyOverride,
}

/// Days of the week
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// Time range for operating hours
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: String, // HH:MM format
    pub end: String,   // HH:MM format
}

/// Emergency override settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyOverride {
    pub enabled: bool,
    pub max_override_duration: Duration,
    pub override_price_multiplier: f64,
    pub authorized_users: Vec<UserId>,
}

/// Performance preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePreferences {
    /// Preferred resource utilization target (0-100%)
    pub target_utilization: f64,
    /// Maximum temperature limits
    pub temperature_limits: TemperatureLimits,
    /// Power consumption preferences
    pub power_preferences: PowerPreferences,
    /// Noise level preferences
    pub noise_preferences: NoisePreferences,
}

/// Temperature limits for hardware protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureLimits {
    pub cpu_max_temp: u32,    // Celsius
    pub gpu_max_temp: u32,    // Celsius
    pub warning_threshold: u32, // Celsius
    pub emergency_shutdown: u32, // Celsius
}

/// Power consumption preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerPreferences {
    pub max_power_consumption: Option<u32>, // Watts
    pub power_saving_mode: PowerSavingMode,
    pub peak_hours_limit: Option<PowerLimit>,
}

/// Power saving modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PowerSavingMode {
    Disabled,
    Balanced,
    Aggressive,
    Custom(PowerSavingConfig),
}

/// Custom power saving configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSavingConfig {
    pub cpu_scaling: f64,      // 0.1 - 1.0
    pub gpu_power_limit: f64,  // 0.1 - 1.0
    pub memory_speed: f64,     // 0.1 - 1.0
    pub storage_power: f64,    // 0.1 - 1.0
}

/// Power limit during peak hours
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerLimit {
    pub limit_watts: u32,
    pub peak_start: String, // HH:MM
    pub peak_end: String,   // HH:MM
}

/// Noise level preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoisePreferences {
    pub max_fan_speed: u32,     // Percentage
    pub quiet_hours: Vec<TimeRange>,
    pub noise_tolerance: NoiseLevel,
}

/// Noise tolerance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoiseLevel {
    Silent,     // <20 dB
    Quiet,      // <30 dB
    Normal,     // <40 dB
    Loud,       // <50 dB
    NoLimit,    // No restriction
}

/// Pricing configuration for resource sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfiguration {
    /// Base prices per resource type
    pub base_prices: HashMap<AssetType, PriceModel>,
    /// Dynamic pricing settings
    pub dynamic_pricing: DynamicPricingConfig,
    /// Payment preferences
    pub payment_preferences: PaymentPreferences,
    /// Discount settings
    pub discount_settings: DiscountSettings,
}

/// Price model for resource type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceModel {
    /// Base price per unit per hour
    pub base_price: f64,
    /// Price currency
    pub currency: Currency,
    /// Minimum session price
    pub minimum_price: f64,
    /// Peak hour multiplier
    pub peak_multiplier: f64,
    /// Volume discount tiers
    pub volume_tiers: Vec<VolumeTier>,
}

/// Supported currencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Currency {
    USD,
    EUR,
    CaesarTokens,
    HyperMeshCredits,
    Custom(String),
}

/// Volume discount tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeTier {
    pub min_hours: u64,
    pub discount_percentage: f64,
}

/// Dynamic pricing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicPricingConfig {
    pub enabled: bool,
    pub demand_multiplier: f64,    // Increase price based on demand
    pub supply_multiplier: f64,    // Decrease price based on supply
    pub reputation_bonus: f64,     // Price bonus for high reputation
    pub performance_bonus: f64,    // Price bonus for high performance
    pub update_frequency: Duration,
}

/// Payment preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPreferences {
    pub preferred_currency: Currency,
    pub payment_frequency: PaymentFrequency,
    pub minimum_payout: f64,
    pub auto_reinvest_percentage: f64,
    pub tax_reporting: TaxReporting,
}

/// Payment frequencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentFrequency {
    Immediate,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
}

/// Tax reporting settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxReporting {
    pub enabled: bool,
    pub tax_jurisdiction: String,
    pub tax_rate: f64,
    pub reporting_format: String,
}

/// Discount settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountSettings {
    pub loyalty_discounts: Vec<LoyaltyDiscount>,
    pub referral_bonuses: ReferralBonus,
    pub seasonal_promotions: Vec<SeasonalPromotion>,
}

/// Loyalty discount tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoyaltyDiscount {
    pub min_reputation: f64,
    pub discount_percentage: f64,
    pub additional_benefits: Vec<String>,
}

/// Referral bonus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralBonus {
    pub enabled: bool,
    pub bonus_amount: f64,
    pub bonus_currency: Currency,
    pub max_referrals: Option<u32>,
}

/// Seasonal promotion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPromotion {
    pub name: String,
    pub start_date: SystemTime,
    pub end_date: SystemTime,
    pub discount_percentage: f64,
    pub applicable_resources: Vec<AssetType>,
}

/// Notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub email_notifications: EmailNotifications,
    pub push_notifications: PushNotifications,
    pub sms_notifications: SmsNotifications,
    pub in_app_notifications: InAppNotifications,
}

/// Email notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNotifications {
    pub enabled: bool,
    pub resource_allocation: bool,
    pub payment_received: bool,
    pub system_alerts: bool,
    pub performance_reports: bool,
    pub frequency: NotificationFrequency,
}

/// Push notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotifications {
    pub enabled: bool,
    pub urgent_alerts: bool,
    pub resource_requests: bool,
    pub earning_milestones: bool,
    pub quiet_hours: Vec<TimeRange>,
}

/// SMS notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsNotifications {
    pub enabled: bool,
    pub emergency_only: bool,
    pub phone_number: Option<String>,
    pub verification_status: VerificationStatus,
}

/// In-app notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InAppNotifications {
    pub enabled: bool,
    pub show_badges: bool,
    pub sound_alerts: bool,
    pub vibration_alerts: bool,
}

/// Notification frequencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationFrequency {
    Immediate,
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

/// Account status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountStatus {
    Active,
    Suspended,
    Banned,
    PendingVerification,
    Closed,
}

/// Contribution session tracking
#[derive(Debug, Clone)]
pub struct ContributionSession {
    pub session_id: ContributionId,
    pub user_id: UserId,
    pub asset_allocations: HashMap<AssetType, AssetAllocation>,
    pub started_at: SystemTime,
    pub expected_end: SystemTime,
    pub actual_usage: HashMap<AssetType, UsageMetrics>,
    pub earnings: SessionEarnings,
    pub performance_metrics: SessionPerformance,
    pub status: SessionStatus,
}

/// Usage metrics for a contribution session
#[derive(Debug, Clone, Default)]
pub struct UsageMetrics {
    pub total_usage_time: Duration,
    pub average_utilization: f64,
    pub peak_utilization: f64,
    pub efficiency_score: f64,
    pub user_satisfaction: Option<f64>,
}

/// Session earnings tracking
#[derive(Debug, Clone, Default)]
pub struct SessionEarnings {
    pub base_earnings: f64,
    pub performance_bonus: f64,
    pub reputation_bonus: f64,
    pub total_earnings: f64,
    pub currency: Currency,
    pub payout_status: PayoutStatus,
}

/// Payout status
#[derive(Debug, Clone)]
pub enum PayoutStatus {
    Pending,
    Processing,
    Completed,
    Failed(String),
}

impl Default for PayoutStatus {
    fn default() -> Self {
        PayoutStatus::Pending
    }
}

/// Session performance metrics
#[derive(Debug, Clone, Default)]
pub struct SessionPerformance {
    pub uptime_percentage: f64,
    pub response_time_avg: Duration,
    pub error_rate: f64,
    pub resource_efficiency: f64,
    pub user_rating: Option<f64>,
}

/// Session status
#[derive(Debug, Clone)]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Terminated(String),
    Error(String),
}

/// Reward calculation engine
pub struct RewardEngine {
    /// Base reward rates
    base_rates: HashMap<AssetType, f64>,
    /// Performance multipliers
    performance_multipliers: PerformanceMultipliers,
    /// Reputation system
    reputation_system: ReputationSystem,
}

/// Performance multipliers for reward calculation
#[derive(Debug, Clone)]
pub struct PerformanceMultipliers {
    pub uptime_multiplier: f64,
    pub efficiency_multiplier: f64,
    pub user_satisfaction_multiplier: f64,
    pub response_time_multiplier: f64,
}

/// Reputation system for users
#[derive(Debug, Clone)]
pub struct ReputationSystem {
    pub base_reputation: f64,
    pub uptime_weight: f64,
    pub performance_weight: f64,
    pub user_feedback_weight: f64,
    pub tenure_weight: f64,
}

/// Platform metrics
#[derive(Debug, Default)]
pub struct PlatformMetrics {
    pub total_users: u64,
    pub active_users: u64,
    pub total_resources_shared: HashMap<AssetType, u64>,
    pub total_earnings_distributed: f64,
    pub average_user_rating: f64,
    pub platform_efficiency: f64,
    pub network_utilization: f64,
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
        // Detect hardware configuration
        let hardware_config = self.detect_hardware_configuration().await?;
        
        // Create default sharing preferences
        let sharing_preferences = SharingPreferences::default_for_hardware(&hardware_config);
        
        let user_profile = UserProfile {
            user_id: user_id.clone(),
            display_name,
            email,
            hardware_config,
            sharing_preferences,
            reputation_score: 1.0, // Start with neutral reputation
            total_earnings: 0,
            account_status: AccountStatus::PendingVerification,
            registered_at: SystemTime::now(),
            last_active: SystemTime::now(),
        };
        
        // Store user profile
        {
            let mut profiles = self.user_profiles.write().await;
            profiles.insert(user_id, user_profile.clone());
        }
        
        // Update metrics
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
        // Get user profile
        let user_profile = {
            let profiles = self.user_profiles.read().await;
            profiles.get(user_id)
                .ok_or_else(|| anyhow!("User not found"))?
                .clone()
        };
        
        // Verify user is eligible
        self.verify_user_eligibility(&user_profile).await?;
        
        // Allocate resources based on sharing preferences
        let mut asset_allocations = HashMap::new();
        for asset_type in resource_types {
            let allocation = self.allocate_user_resource(
                &user_profile,
                asset_type,
                &consensus_proof,
            ).await?;
            asset_allocations.insert(asset_type, allocation);
        }
        
        // Create contribution session
        let session_id = Uuid::new_v4().to_string();
        let session = ContributionSession {
            session_id: session_id.clone(),
            user_id: user_id.clone(),
            asset_allocations,
            started_at: SystemTime::now(),
            expected_end: SystemTime::now() + Duration::from_secs(3600), // Default 1 hour
            actual_usage: HashMap::new(),
            earnings: SessionEarnings::default(),
            performance_metrics: SessionPerformance::default(),
            status: SessionStatus::Active,
        };
        
        // Register session
        {
            let mut sessions = self.active_contributions.write().await;
            sessions.insert(session_id, session.clone());
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.lock().await;
            metrics.active_users += 1;
        }
        
        Ok(session)
    }
    
    /// Detect hardware configuration on this system
    async fn detect_hardware_configuration(&self) -> Result<HardwareConfiguration> {
        // This would use system APIs to detect actual hardware
        // For now, return a mock configuration
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
                    memory: 24 * 1024 * 1024 * 1024, // 24GB
                    compute_units: 128,
                    base_clock: 2520,
                    memory_clock: 10500,
                    memory_bus_width: 384,
                    compute_capability: Some("8.9".to_string()),
                    supported_apis: vec!["Nova".to_string(), "Vulkan".to_string(), "OpenCL".to_string()],
                }
            ],
            memory_info: MemoryInfo {
                total_capacity: 128 * 1024 * 1024 * 1024, // 128GB
                available_capacity: 100 * 1024 * 1024 * 1024, // 100GB
                memory_type: "DDR5".to_string(),
                speed: 5600,
                modules: vec![
                    MemoryModule {
                        size: 32 * 1024 * 1024 * 1024, // 32GB
                        speed: 5600,
                        latency: "CL36".to_string(),
                        manufacturer: "G.Skill".to_string(),
                    }
                ],
            },
            storage_info: vec![
                StorageInfo {
                    device_type: StorageType::NVMe,
                    capacity: 2 * 1024 * 1024 * 1024 * 1024, // 2TB
                    available: 1024 * 1024 * 1024 * 1024, // 1TB
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
                        speed: 10000, // 10 Gbps
                        mac_address: "00:11:22:33:44:55".to_string(),
                        ip_addresses: vec!["192.168.1.100".to_string()],
                    }
                ],
                bandwidth_upload: 1000,   // 1 Gbps
                bandwidth_download: 1000, // 1 Gbps
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
    
    /// Verify user eligibility for contribution
    async fn verify_user_eligibility(&self, user_profile: &UserProfile) -> Result<()> {
        // Check account status
        match user_profile.account_status {
            AccountStatus::Active => {},
            AccountStatus::Suspended => return Err(anyhow!("Account suspended")),
            AccountStatus::Banned => return Err(anyhow!("Account banned")),
            AccountStatus::PendingVerification => return Err(anyhow!("Account pending verification")),
            AccountStatus::Closed => return Err(anyhow!("Account closed")),
        }
        
        // Check reputation threshold
        if user_profile.reputation_score < self.config.minimum_reputation {
            return Err(anyhow!("Reputation score too low"));
        }
        
        // Check hardware verification
        match user_profile.hardware_config.verification_status {
            VerificationStatus::Verified => {},
            _ => return Err(anyhow!("Hardware not verified")),
        }
        
        Ok(())
    }
    
    /// Allocate user resource for sharing
    async fn allocate_user_resource(
        &self,
        user_profile: &UserProfile,
        asset_type: AssetType,
        consensus_proof: &ConsensusProof,
    ) -> Result<AssetAllocation> {
        // Get sharing settings for this resource type
        let resource_settings = user_profile.sharing_preferences.resource_settings
            .get(&asset_type)
            .ok_or_else(|| anyhow!("Resource type not configured for sharing"))?;
        
        if !resource_settings.enabled {
            return Err(anyhow!("Resource sharing disabled for this type"));
        }
        
        // Calculate available capacity based on share percentage
        let total_capacity = self.get_total_capacity_for_asset_type(&user_profile.hardware_config, &asset_type)?;
        let shared_capacity = (total_capacity as f64 * resource_settings.share_percentage / 100.0) as u64;
        
        // Create asset allocation request
        let allocation_request = crate::assets::core::AssetAllocationRequest {
            asset_type,
            required_capacity: shared_capacity,
            priority: crate::assets::core::AssetPriority::Normal,
            duration: resource_settings.max_session_duration,
            consensus_proof: consensus_proof.clone(),
        };
        
        // Allocate through asset manager
        self.asset_manager.allocate_asset(allocation_request).await
    }
    
    /// Get total capacity for asset type from hardware configuration
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
    
    /// Get platform metrics
    pub async fn get_metrics(&self) -> PlatformMetrics {
        let metrics = self.metrics.lock().await;
        metrics.clone()
    }
    
    /// List user's contribution sessions
    pub async fn list_user_sessions(&self, user_id: &UserId) -> Vec<ContributionSession> {
        let sessions = self.active_contributions.read().await;
        sessions.values()
            .filter(|session| &session.user_id == user_id)
            .cloned()
            .collect()
    }
}

impl RewardEngine {
    pub fn new() -> Self {
        let mut base_rates = HashMap::new();
        base_rates.insert(AssetType::Cpu, 0.10);    // $0.10 per core-hour
        base_rates.insert(AssetType::Memory, 0.01); // $0.01 per GB-hour
        base_rates.insert(AssetType::Gpu, 1.00);    // $1.00 per GPU-hour
        base_rates.insert(AssetType::Storage, 0.001); // $0.001 per GB-hour
        base_rates.insert(AssetType::Network, 0.05); // $0.05 per Mbps-hour
        
        Self {
            base_rates,
            performance_multipliers: PerformanceMultipliers {
                uptime_multiplier: 1.2,
                efficiency_multiplier: 1.1,
                user_satisfaction_multiplier: 1.15,
                response_time_multiplier: 1.05,
            },
            reputation_system: ReputationSystem {
                base_reputation: 1.0,
                uptime_weight: 0.3,
                performance_weight: 0.3,
                user_feedback_weight: 0.25,
                tenure_weight: 0.15,
            },
        }
    }
}

impl SharingPreferences {
    /// Create default sharing preferences based on hardware configuration
    pub fn default_for_hardware(hardware_config: &HardwareConfiguration) -> Self {
        let mut resource_settings = HashMap::new();
        
        // CPU sharing - conservative defaults
        resource_settings.insert(AssetType::Cpu, ResourceSharingSettings {
            enabled: true,
            share_percentage: 25.0, // Share 25% of CPU
            privacy_level: PrivacyLevel::P2P,
            max_concurrent_users: 2,
            max_session_duration: Duration::from_secs(3600),
            min_price_per_unit: 0.10,
            constraints: ResourceConstraints {
                cpu_constraints: Some(CpuConstraints {
                    max_threads_per_user: hardware_config.cpu_info.threads / 4,
                    allowed_instruction_sets: hardware_config.cpu_info.instruction_sets.clone(),
                    priority_level: CpuPriority::Normal,
                    thermal_limit: Some(80), // 80°C
                }),
                gpu_constraints: None,
                memory_constraints: None,
                storage_constraints: None,
            },
        });
        
        // Memory sharing - very conservative
        resource_settings.insert(AssetType::Memory, ResourceSharingSettings {
            enabled: false, // Disabled by default for security
            share_percentage: 10.0,
            privacy_level: PrivacyLevel::Private,
            max_concurrent_users: 1,
            max_session_duration: Duration::from_secs(1800),
            min_price_per_unit: 0.01,
            constraints: ResourceConstraints {
                cpu_constraints: None,
                gpu_constraints: None,
                memory_constraints: Some(MemoryConstraints {
                    max_allocation_per_user: hardware_config.memory_info.total_capacity / 10,
                    memory_protection_level: MemoryProtectionLevel::Isolated,
                    allowed_access_patterns: vec![AccessPattern::Sequential, AccessPattern::Random],
                }),
                storage_constraints: None,
            },
        });
        
        // GPU sharing - high value, moderate risk
        if !hardware_config.gpu_info.is_empty() {
            resource_settings.insert(AssetType::Gpu, ResourceSharingSettings {
                enabled: false, // Disabled by default, user must explicitly enable
                share_percentage: 50.0,
                privacy_level: PrivacyLevel::P2P,
                max_concurrent_users: 1,
                max_session_duration: Duration::from_secs(7200),
                min_price_per_unit: 1.00,
                constraints: ResourceConstraints {
                    cpu_constraints: None,
                    gpu_constraints: Some(GpuConstraints {
                        max_memory_per_user: hardware_config.gpu_info[0].memory / 2,
                        allowed_compute_types: vec![ComputeType::GeneralPurpose, ComputeType::MachineLearning],
                        max_concurrent_kernels: 10,
                        power_limit: Some(300), // 300W
                    }),
                    memory_constraints: None,
                    storage_constraints: None,
                },
            });
        }
        
        Self {
            resource_settings,
            privacy_level: PrivacyLevel::Private,
            operating_hours: OperatingHours::default_24_7(),
            performance_preferences: PerformancePreferences::conservative(),
            pricing_config: PricingConfiguration::default(),
            notification_preferences: NotificationPreferences::default(),
        }
    }
}

impl OperatingHours {
    /// Create 24/7 operating hours
    pub fn default_24_7() -> Self {
        let mut schedule = HashMap::new();
        let all_day = TimeRange {
            start: "00:00".to_string(),
            end: "23:59".to_string(),
        };
        
        schedule.insert(Weekday::Monday, all_day.clone());
        schedule.insert(Weekday::Tuesday, all_day.clone());
        schedule.insert(Weekday::Wednesday, all_day.clone());
        schedule.insert(Weekday::Thursday, all_day.clone());
        schedule.insert(Weekday::Friday, all_day.clone());
        schedule.insert(Weekday::Saturday, all_day.clone());
        schedule.insert(Weekday::Sunday, all_day);
        
        Self {
            schedule,
            timezone: "UTC".to_string(),
            always_on: true,
            emergency_override: EmergencyOverride {
                enabled: false,
                max_override_duration: Duration::from_secs(3600),
                override_price_multiplier: 2.0,
                authorized_users: vec![],
            },
        }
    }
}

impl PerformancePreferences {
    /// Create conservative performance preferences
    pub fn conservative() -> Self {
        Self {
            target_utilization: 70.0,
            temperature_limits: TemperatureLimits {
                cpu_max_temp: 80,
                gpu_max_temp: 85,
                warning_threshold: 75,
                emergency_shutdown: 95,
            },
            power_preferences: PowerPreferences {
                max_power_consumption: Some(500), // 500W
                power_saving_mode: PowerSavingMode::Balanced,
                peak_hours_limit: Some(PowerLimit {
                    limit_watts: 300,
                    peak_start: "18:00".to_string(),
                    peak_end: "22:00".to_string(),
                }),
            },
            noise_preferences: NoisePreferences {
                max_fan_speed: 70,
                quiet_hours: vec![
                    TimeRange {
                        start: "22:00".to_string(),
                        end: "08:00".to_string(),
                    }
                ],
                noise_tolerance: NoiseLevel::Normal,
            },
        }
    }
}

impl Default for PricingConfiguration {
    fn default() -> Self {
        let mut base_prices = HashMap::new();
        base_prices.insert(AssetType::Cpu, PriceModel {
            base_price: 0.10,
            currency: Currency::CaesarTokens,
            minimum_price: 0.01,
            peak_multiplier: 1.5,
            volume_tiers: vec![
                VolumeTier { min_hours: 10, discount_percentage: 5.0 },
                VolumeTier { min_hours: 100, discount_percentage: 10.0 },
                VolumeTier { min_hours: 1000, discount_percentage: 15.0 },
            ],
        });
        
        Self {
            base_prices,
            dynamic_pricing: DynamicPricingConfig {
                enabled: true,
                demand_multiplier: 1.2,
                supply_multiplier: 0.8,
                reputation_bonus: 1.1,
                performance_bonus: 1.05,
                update_frequency: Duration::from_secs(300),
            },
            payment_preferences: PaymentPreferences {
                preferred_currency: Currency::CaesarTokens,
                payment_frequency: PaymentFrequency::Daily,
                minimum_payout: 1.0,
                auto_reinvest_percentage: 0.0,
                tax_reporting: TaxReporting {
                    enabled: false,
                    tax_jurisdiction: "".to_string(),
                    tax_rate: 0.0,
                    reporting_format: "".to_string(),
                },
            },
            discount_settings: DiscountSettings {
                loyalty_discounts: vec![],
                referral_bonuses: ReferralBonus {
                    enabled: true,
                    bonus_amount: 10.0,
                    bonus_currency: Currency::CaesarTokens,
                    max_referrals: Some(10),
                },
                seasonal_promotions: vec![],
            },
        }
    }
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            email_notifications: EmailNotifications {
                enabled: true,
                resource_allocation: true,
                payment_received: true,
                system_alerts: true,
                performance_reports: false,
                frequency: NotificationFrequency::Daily,
            },
            push_notifications: PushNotifications {
                enabled: true,
                urgent_alerts: true,
                resource_requests: false,
                earning_milestones: true,
                quiet_hours: vec![
                    TimeRange {
                        start: "22:00".to_string(),
                        end: "08:00".to_string(),
                    }
                ],
            },
            sms_notifications: SmsNotifications {
                enabled: false,
                emergency_only: true,
                phone_number: None,
                verification_status: VerificationStatus::Pending,
            },
            in_app_notifications: InAppNotifications {
                enabled: true,
                show_badges: true,
                sound_alerts: false,
                vibration_alerts: false,
            },
        }
    }
}

impl Clone for PlatformMetrics {
    fn clone(&self) -> Self {
        Self {
            total_users: self.total_users,
            active_users: self.active_users,
            total_resources_shared: self.total_resources_shared.clone(),
            total_earnings_distributed: self.total_earnings_distributed,
            average_user_rating: self.average_user_rating,
            platform_efficiency: self.platform_efficiency,
            network_utilization: self.network_utilization,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_platform_creation() {
        let asset_manager = Arc::new(AssetManager::new());
        let config = PlatformConfig::default();
        
        let platform = UserContributionPlatform::new(asset_manager, config).await;
        assert!(platform.is_ok());
    }
    
    #[tokio::test]
    async fn test_user_registration() {
        let asset_manager = Arc::new(AssetManager::new());
        let config = PlatformConfig::default();
        let platform = UserContributionPlatform::new(asset_manager, config).await.unwrap();
        
        let profile = platform.register_user(
            "user123".to_string(),
            "Test User".to_string(),
            "test@example.com".to_string(),
        ).await;
        
        assert!(profile.is_ok());
        let profile = profile.unwrap();
        assert_eq!(profile.user_id, "user123");
        assert_eq!(profile.display_name, "Test User");
    }
    
    #[test]
    fn test_sharing_preferences_creation() {
        let hardware_config = HardwareConfiguration {
            cpu_info: CpuInfo {
                model: "Test CPU".to_string(),
                cores: 8,
                threads: 16,
                base_frequency: 3000,
                max_frequency: 4000,
                cache_l1: 512,
                cache_l2: 4096,
                cache_l3: 32768,
                architecture: "x86_64".to_string(),
                instruction_sets: vec!["AVX2".to_string()],
            },
            gpu_info: vec![],
            memory_info: MemoryInfo {
                total_capacity: 32 * 1024 * 1024 * 1024, // 32GB
                available_capacity: 28 * 1024 * 1024 * 1024, // 28GB
                memory_type: "DDR4".to_string(),
                speed: 3200,
                modules: vec![],
            },
            storage_info: vec![],
            network_info: NetworkInfo {
                interfaces: vec![],
                bandwidth_upload: 100,
                bandwidth_download: 100,
                latency: 10,
                is_metered: false,
                location: NetworkLocation {
                    country: "US".to_string(),
                    region: "CA".to_string(),
                    city: "SF".to_string(),
                    latitude: None,
                    longitude: None,
                    timezone: "UTC".to_string(),
                },
            },
            verification_status: VerificationStatus::Verified,
        };
        
        let preferences = SharingPreferences::default_for_hardware(&hardware_config);
        assert!(preferences.resource_settings.contains_key(&AssetType::Cpu));
        assert!(preferences.resource_settings.contains_key(&AssetType::Memory));
        
        let cpu_settings = &preferences.resource_settings[&AssetType::Cpu];
        assert!(cpu_settings.enabled);
        assert_eq!(cpu_settings.share_percentage, 25.0);
    }
}