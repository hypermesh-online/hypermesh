// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! User Contribution Platform - Interface for hardware sharing and asset contribution
//!
//! This module provides the user-facing interface for contributing hardware resources
//! to the HyperMesh network, managing rewards through Caesar integration, and
//! configuring privacy settings for resource sharing.

pub mod hardware;
pub mod notifications;
pub mod platform;
pub mod pricing;
pub mod rewards;
pub mod session;
pub mod sharing;

// Re-export main types
pub use hardware::{
    CpuInfo, GpuInfo, HardwareConfiguration, MemoryInfo, MemoryModule, NetworkInfo,
    NetworkInterface, NetworkLocation, StorageInfo, StorageType, VerificationStatus,
};

pub use sharing::{
    AccessPattern, ComputeType, CpuConstraints, CpuPriority, EmergencyOverride, GpuConstraints,
    MemoryConstraints, MemoryProtectionLevel, NoiseLevel, NoisePreferences, OperatingHours,
    PerformancePreferences, PowerLimit, PowerPreferences, PowerSavingConfig, PowerSavingMode,
    ResourceConstraints, ResourceSharingSettings, RetentionPolicy, SharingPreferences,
    StorageConstraints, TemperatureLimits, TimeRange, Weekday,
};

pub use pricing::{
    Currency, DiscountSettings, DynamicPricingConfig, LoyaltyDiscount, PaymentFrequency,
    PaymentPreferences, PriceModel, PricingConfiguration, ReferralBonus, SeasonalPromotion,
    TaxReporting, VolumeTier,
};

pub use notifications::{
    EmailNotifications, InAppNotifications, NotificationFrequency, NotificationPreferences,
    PushNotifications, SmsNotifications,
};

pub use session::{
    AccountStatus, ContributionId, ContributionSession, PayoutStatus, SessionEarnings,
    SessionPerformance, SessionStatus, UsageMetrics, UserId,
};

pub use rewards::{AuthenticationConfig, PerformanceMultipliers, PlatformMetrics, RewardEngine};

pub use platform::{PlatformConfig, UserContributionPlatform, UserProfile};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::AssetManager;
    use std::sync::Arc;

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
        let platform = UserContributionPlatform::new(asset_manager, config)
            .await
            .expect("test: expected success");

        let profile = platform
            .register_user(
                "user123".to_string(),
                "Test User".to_string(),
                "test@example.com".to_string(),
            )
            .await;

        assert!(profile.is_ok());
        let profile = profile.expect("test: profile operation");
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
        assert!(preferences
            .resource_settings
            .contains_key(&crate::assets::core::AssetType::Cpu));
        assert!(preferences
            .resource_settings
            .contains_key(&crate::assets::core::AssetType::Memory));

        let cpu_settings = &preferences.resource_settings[&crate::assets::core::AssetType::Cpu];
        assert!(cpu_settings.enabled);
        assert_eq!(cpu_settings.share_percentage, 25.0);
    }
}
