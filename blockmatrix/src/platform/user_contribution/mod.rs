//! User Contribution Platform - Interface for hardware sharing and asset contribution
//!
//! This module provides the user-facing interface for contributing hardware resources
//! to the HyperMesh network, managing rewards through Caesar integration, and
//! configuring privacy settings for resource sharing.

pub mod hardware;
pub mod sharing;
pub mod pricing;
pub mod notifications;
pub mod session;
pub mod rewards;
pub mod platform;

// Re-export main types
pub use hardware::{
    HardwareConfiguration, CpuInfo, GpuInfo, MemoryInfo, MemoryModule,
    StorageInfo, StorageType, NetworkInfo, NetworkInterface, NetworkLocation,
    VerificationStatus,
};

pub use sharing::{
    SharingPreferences, ResourceSharingSettings, ResourceConstraints,
    CpuConstraints, CpuPriority, GpuConstraints, ComputeType,
    MemoryConstraints, MemoryProtectionLevel, AccessPattern,
    StorageConstraints, RetentionPolicy, OperatingHours, Weekday,
    TimeRange, EmergencyOverride, PerformancePreferences,
    TemperatureLimits, PowerPreferences, PowerSavingMode,
    PowerSavingConfig, PowerLimit, NoisePreferences, NoiseLevel,
};

pub use pricing::{
    PricingConfiguration, PriceModel, Currency, VolumeTier,
    DynamicPricingConfig, PaymentPreferences, PaymentFrequency,
    TaxReporting, DiscountSettings, LoyaltyDiscount, ReferralBonus,
    SeasonalPromotion,
};

pub use notifications::{
    NotificationPreferences, EmailNotifications, PushNotifications,
    SmsNotifications, InAppNotifications, NotificationFrequency,
};

pub use session::{
    UserId, ContributionId, ContributionSession, UsageMetrics,
    SessionEarnings, PayoutStatus, SessionPerformance, SessionStatus,
    AccountStatus,
};

pub use rewards::{
    RewardEngine, PerformanceMultipliers, ReputationSystem, PlatformMetrics,
};

pub use platform::{
    UserContributionPlatform, UserProfile, PlatformConfig,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::assets::core::AssetManager;

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
                total_capacity: 32 * 1024 * 1024 * 1024,
                available_capacity: 28 * 1024 * 1024 * 1024,
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
        assert!(preferences.resource_settings.contains_key(&crate::assets::core::AssetType::Cpu));
        assert!(preferences.resource_settings.contains_key(&crate::assets::core::AssetType::Memory));

        let cpu_settings = &preferences.resource_settings[&crate::assets::core::AssetType::Cpu];
        assert!(cpu_settings.enabled);
        assert_eq!(cpu_settings.share_percentage, 25.0);
    }
}
