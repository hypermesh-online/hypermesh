// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Reporting and delivery configuration types for sharing module.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Risk reporting requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskReportingRequirements {
    pub frequency: ReportingFrequency,
    pub recipients: Vec<ReportRecipient>,
    pub detail_level: ReportDetailLevel,
    pub filtering_preferences: ReportFilteringPreferences,
    pub delivery_preferences: DeliveryPreferences,
}

/// Reporting frequency options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReportingFrequency {
    RealTime,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    OnDemand,
    EventTriggered,
}

/// Report recipients
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportRecipient {
    pub id: String,
    pub recipient_type: RecipientType,
    pub contact_info: HashMap<String, String>,
    pub delivery_preferences: HashMap<String, String>,
}

/// Types of report recipients
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecipientType {
    User,
    Administrator,
    PrivacyOfficer,
    ComplianceOfficer,
    ExternalAuditor,
    RegulatoryBody,
}

/// Report detail levels
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReportDetailLevel {
    Summary,
    Standard,
    Detailed,
    Comprehensive,
}

/// Report filtering preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportFilteringPreferences {
    pub risk_level_filter: Vec<String>,
    pub time_range_filters: Vec<TimeRangeFilter>,
    pub category_filters: Vec<String>,
    pub custom_filters: Vec<CustomFilter>,
}

/// Time range filter
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeRangeFilter {
    pub name: String,
    pub time_spec: TimeSpecification,
    pub pattern: Option<RecurringPattern>,
}

/// Time specification options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TimeSpecification {
    LastNHours(u32),
    LastNDays(u32),
    LastNWeeks(u32),
    LastNMonths(u32),
    SpecificTimeRange { start: String, end: String },
}

/// Recurring patterns
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecurringPattern {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Custom { pattern: String },
}

/// Custom filter definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomFilter {
    pub name: String,
    pub criteria: HashMap<String, String>,
    pub operation: String,
    pub include: bool,
}

/// Report delivery preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryPreferences {
    pub method: DeliveryMethod,
    pub scheduling: DeliveryScheduling,
    pub retry_settings: RetrySettings,
    pub batch_settings: BatchDeliverySettings,
    pub format_preferences: FormatPreferences,
}

/// Delivery method options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeliveryMethod {
    Email,
    SMS,
    Push,
    Webhook,
    API,
    Dashboard,
    File,
}

/// Delivery scheduling
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryScheduling {
    pub immediate: bool,
    pub scheduled_deliveries: Vec<ScheduledDelivery>,
    pub timezone: String,
}

/// Scheduled delivery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledDelivery {
    pub name: String,
    pub time: String,
    pub recurrence: RecurringPattern,
}

/// Retry settings for delivery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetrySettings {
    pub enabled: bool,
    pub max_attempts: u32,
    pub retry_delay: Duration,
    pub backoff_strategy: BackoffStrategy,
}

/// Backoff strategies for retries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Linear,
    Exponential,
    Fixed,
    Custom { parameters: HashMap<String, f32> },
}

/// Batch delivery settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchDeliverySettings {
    pub enabled: bool,
    pub batch_size: u32,
    pub batch_timeout: Duration,
    pub force_delivery_threshold: u32,
}

/// Format preferences for reports
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormatPreferences {
    pub format: ReportFormat,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
    pub localization: LocalizationPreferences,
}

/// Report format options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReportFormat {
    JSON,
    XML,
    CSV,
    PDF,
    HTML,
    Custom { format_spec: String },
}

/// Localization preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalizationPreferences {
    pub language: String,
    pub date_format: String,
    pub number_format: String,
    pub currency: String,
}
