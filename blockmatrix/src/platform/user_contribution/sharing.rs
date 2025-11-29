//! Resource sharing preferences and configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::assets::core::{AssetType, PrivacyLevel};
use super::hardware::HardwareConfiguration;
use super::pricing::PricingConfiguration;
use super::notifications::NotificationPreferences;

/// Sharing preferences configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingPreferences {
    pub resource_settings: HashMap<AssetType, ResourceSharingSettings>,
    pub privacy_level: PrivacyLevel,
    pub operating_hours: OperatingHours,
    pub performance_preferences: PerformancePreferences,
    pub pricing_config: PricingConfiguration,
    pub notification_preferences: NotificationPreferences,
}

/// Resource sharing settings for specific asset type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSharingSettings {
    pub enabled: bool,
    pub share_percentage: f64,
    pub privacy_level: PrivacyLevel,
    pub max_concurrent_users: u32,
    pub max_session_duration: Duration,
    pub min_price_per_unit: f64,
    pub constraints: ResourceConstraints,
}

/// Resource-specific constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    pub cpu_constraints: Option<CpuConstraints>,
    pub gpu_constraints: Option<GpuConstraints>,
    pub memory_constraints: Option<MemoryConstraints>,
    pub storage_constraints: Option<StorageConstraints>,
}

/// CPU sharing constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuConstraints {
    pub max_threads_per_user: u32,
    pub allowed_instruction_sets: Vec<String>,
    pub priority_level: CpuPriority,
    pub thermal_limit: Option<u32>,
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
    pub power_limit: Option<u32>,
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
    pub schedule: HashMap<Weekday, TimeRange>,
    pub timezone: String,
    pub always_on: bool,
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
    pub start: String,
    pub end: String,
}

/// Emergency override settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyOverride {
    pub enabled: bool,
    pub max_override_duration: Duration,
    pub override_price_multiplier: f64,
    pub authorized_users: Vec<String>,
}

/// Performance preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePreferences {
    pub target_utilization: f64,
    pub temperature_limits: TemperatureLimits,
    pub power_preferences: PowerPreferences,
    pub noise_preferences: NoisePreferences,
}

/// Temperature limits for hardware protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureLimits {
    pub cpu_max_temp: u32,
    pub gpu_max_temp: u32,
    pub warning_threshold: u32,
    pub emergency_shutdown: u32,
}

/// Power consumption preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerPreferences {
    pub max_power_consumption: Option<u32>,
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
    pub cpu_scaling: f64,
    pub gpu_power_limit: f64,
    pub memory_speed: f64,
    pub storage_power: f64,
}

/// Power limit during peak hours
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerLimit {
    pub limit_watts: u32,
    pub peak_start: String,
    pub peak_end: String,
}

/// Noise level preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoisePreferences {
    pub max_fan_speed: u32,
    pub quiet_hours: Vec<TimeRange>,
    pub noise_tolerance: NoiseLevel,
}

/// Noise tolerance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoiseLevel {
    Silent,
    Quiet,
    Normal,
    Loud,
    NoLimit,
}

impl SharingPreferences {
    /// Create default sharing preferences based on hardware configuration
    pub fn default_for_hardware(hardware_config: &HardwareConfiguration) -> Self {
        let mut resource_settings = HashMap::new();

        resource_settings.insert(AssetType::Cpu, ResourceSharingSettings {
            enabled: true,
            share_percentage: 25.0,
            privacy_level: PrivacyLevel::P2P,
            max_concurrent_users: 2,
            max_session_duration: Duration::from_secs(3600),
            min_price_per_unit: 0.10,
            constraints: ResourceConstraints {
                cpu_constraints: Some(CpuConstraints {
                    max_threads_per_user: hardware_config.cpu_info.threads / 4,
                    allowed_instruction_sets: hardware_config.cpu_info.instruction_sets.clone(),
                    priority_level: CpuPriority::Normal,
                    thermal_limit: Some(80),
                }),
                gpu_constraints: None,
                memory_constraints: None,
                storage_constraints: None,
            },
        });

        resource_settings.insert(AssetType::Memory, ResourceSharingSettings {
            enabled: false,
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

        if !hardware_config.gpu_info.is_empty() {
            resource_settings.insert(AssetType::Gpu, ResourceSharingSettings {
                enabled: false,
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
                        power_limit: Some(300),
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
                max_power_consumption: Some(500),
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
