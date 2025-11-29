//! Notification preferences and configuration

use serde::{Deserialize, Serialize};
use super::hardware::VerificationStatus;
use super::sharing::TimeRange;

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
