//! HyperMesh Platform - User-facing platform for distributed computing
//!
//! This module provides the complete user experience for participating in
//! the HyperMesh distributed computing network, including hardware contribution,
//! resource sharing, and earnings management.

pub mod user_contribution;

pub use user_contribution::{
    UserContributionPlatform, UserProfile, ContributionSession,
    HardwareConfiguration, SharingPreferences, PlatformMetrics,
    UserId, ContributionId,
};

/// Platform-wide configuration and constants
pub mod config {
    use std::time::Duration;
    
    /// Maximum number of concurrent contribution sessions per user
    pub const MAX_CONCURRENT_SESSIONS: u32 = 10;
    
    /// Default session timeout
    pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(86400); // 24 hours
    
    /// Minimum reputation score for platform participation
    pub const MIN_REPUTATION_SCORE: f64 = 0.5;
    
    /// Default hardware verification timeout
    pub const HARDWARE_VERIFICATION_TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour
    
    /// Platform version
    pub const PLATFORM_VERSION: &str = "1.0.0";
}

/// Platform initialization and management
pub struct HyperMeshPlatform {
    contribution_platform: user_contribution::UserContributionPlatform,
}

impl HyperMeshPlatform {
    /// Initialize the complete HyperMesh platform
    pub async fn initialize() -> anyhow::Result<Self> {
        let asset_manager = std::sync::Arc::new(
            crate::assets::core::AssetManager::new()
        );
        
        let platform_config = user_contribution::PlatformConfig::default();
        
        let contribution_platform = user_contribution::UserContributionPlatform::new(
            asset_manager,
            platform_config,
        ).await?;
        
        Ok(Self {
            contribution_platform,
        })
    }
    
    /// Get the user contribution platform
    pub fn contribution_platform(&self) -> &user_contribution::UserContributionPlatform {
        &self.contribution_platform
    }
}