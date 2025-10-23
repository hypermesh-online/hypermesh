//! Monitoring and Dashboard Configuration
//!
//! Configuration for privacy monitoring, dashboards, and user interfaces.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::assets::core::{AssetResult, AssetError};

/// Dashboard preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardPreferences {
    /// Preferred dashboard view
    pub default_view: DashboardView,
    
    /// Dashboard widgets
    pub widgets: Vec<DashboardWidget>,
    
    /// Refresh settings
    pub refresh_settings: DashboardRefreshSettings,
    
    /// Notification preferences
    pub notification_preferences: DashboardNotificationPreferences,
    
    /// Accessibility settings
    pub accessibility: DashboardAccessibilitySettings,
}

/// Dashboard view options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DashboardView {
    Overview,
    Detailed,
    Compact,
    Custom,
}

/// Dashboard widget configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardWidget {
    /// Widget identifier
    pub id: String,
    
    /// Widget type
    pub widget_type: DashboardWidgetType,
    
    /// Widget position
    pub position: WidgetPosition,
    
    /// Widget size
    pub size: WidgetSize,
    
    /// Widget visibility
    pub visible: bool,
    
    /// Widget configuration
    pub config: HashMap<String, String>,
}

/// Dashboard widget types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DashboardWidgetType {
    PrivacyOverview,
    ConsentStatus,
    DataUsage,
    SecurityAlerts,
    RiskMetrics,
    ActivityLog,
}

/// Widget position
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WidgetPosition {
    pub x: i32,
    pub y: i32,
}

/// Widget size
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WidgetSize {
    pub width: u32,
    pub height: u32,
}

/// Dashboard refresh settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardRefreshSettings {
    /// Auto-refresh enabled
    pub auto_refresh: bool,
    
    /// Refresh interval
    pub refresh_interval: Duration,
    
    /// Real-time updates enabled
    pub realtime_updates: bool,
    
    /// Background refresh enabled
    pub background_refresh: bool,
}

/// Dashboard notification preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardNotificationPreferences {
    /// Enable notifications
    pub enabled: bool,
    
    /// Notification persistence
    pub persistence: NotificationPersistence,
    
    /// Notification grouping
    pub grouping: NotificationGrouping,
    
    /// Sound notifications
    pub sound_enabled: bool,
    
    /// Visual notifications
    pub visual_enabled: bool,
}

/// Notification persistence options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NotificationPersistence {
    Temporary,
    Persistent,
    UntilDismissed,
}

/// Notification grouping
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationGrouping {
    /// Enable grouping
    pub enabled: bool,
    
    /// Grouping criteria
    pub criteria: Vec<GroupingCriterion>,
    
    /// Group collapse settings
    pub collapse_settings: GroupCollapseSettings,
    
    /// Maximum group size
    pub max_group_size: u32,
}

/// Grouping criteria
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GroupingCriterion {
    ByType,
    BySeverity,
    BySource,
    ByTimeWindow,
}

/// Group collapse settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupCollapseSettings {
    /// Auto-collapse groups
    pub auto_collapse: bool,
    
    /// Collapse threshold
    pub collapse_threshold: u32,
    
    /// Show count in collapsed groups
    pub show_count: bool,
    
    /// Allow manual expand/collapse
    pub manual_control: bool,
}

/// Dashboard accessibility settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DashboardAccessibilitySettings {
    /// High contrast mode
    pub high_contrast: bool,
    
    /// Large text mode
    pub large_text: bool,
    
    /// Screen reader support
    pub screen_reader_support: bool,
    
    /// Keyboard navigation
    pub keyboard_navigation: bool,
    
    /// Color blind support
    pub color_blind_support: ColorBlindSupport,
}

/// Color blind support settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorBlindSupport {
    /// Enable color blind support
    pub enabled: bool,
    
    /// Color blind type
    pub color_blind_type: ColorBlindType,
    
    /// Alternative color schemes
    pub alternative_schemes: Vec<String>,
    
    /// Pattern-based indicators
    pub pattern_indicators: bool,
}

/// Color blind types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ColorBlindType {
    None,
    Protanopia,
    Deuteranopia,
    Tritanopia,
    Monochromacy,
}

impl Default for DashboardPreferences {
    fn default() -> Self {
        Self {
            default_view: DashboardView::Overview,
            widgets: Vec::new(),
            refresh_settings: DashboardRefreshSettings::default(),
            notification_preferences: DashboardNotificationPreferences::default(),
            accessibility: DashboardAccessibilitySettings::default(),
        }
    }
}

impl Default for DashboardRefreshSettings {
    fn default() -> Self {
        Self {
            auto_refresh: true,
            refresh_interval: Duration::from_secs(30),
            realtime_updates: true,
            background_refresh: false,
        }
    }
}

impl Default for DashboardNotificationPreferences {
    fn default() -> Self {
        Self {
            enabled: true,
            persistence: NotificationPersistence::UntilDismissed,
            grouping: NotificationGrouping::default(),
            sound_enabled: false,
            visual_enabled: true,
        }
    }
}

impl Default for NotificationGrouping {
    fn default() -> Self {
        Self {
            enabled: true,
            criteria: vec![GroupingCriterion::ByType],
            collapse_settings: GroupCollapseSettings::default(),
            max_group_size: 10,
        }
    }
}

impl Default for GroupCollapseSettings {
    fn default() -> Self {
        Self {
            auto_collapse: false,
            collapse_threshold: 5,
            show_count: true,
            manual_control: true,
        }
    }
}

impl Default for DashboardAccessibilitySettings {
    fn default() -> Self {
        Self {
            high_contrast: false,
            large_text: false,
            screen_reader_support: true,
            keyboard_navigation: true,
            color_blind_support: ColorBlindSupport::default(),
        }
    }
}

impl Default for ColorBlindSupport {
    fn default() -> Self {
        Self {
            enabled: false,
            color_blind_type: ColorBlindType::None,
            alternative_schemes: Vec::new(),
            pattern_indicators: false,
        }
    }
}