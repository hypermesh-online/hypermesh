// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Monitoring and observability type definitions for catalog deployments

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfiguration {
    pub enable_metrics: bool,
    pub enable_logging: bool,
    pub enable_tracing: bool,
    pub metrics_config: MetricsConfig,
    pub logging_config: LoggingConfig,
    pub alerting_config: AlertingConfig,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub collection_interval: Duration,
    pub retention_period: Duration,
    pub custom_metrics: Vec<CustomMetric>,
}

/// Custom metric definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetric {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub labels: Vec<String>,
}

/// Metric types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub log_level: LogLevel,
    pub log_format: LogFormat,
    pub retention_days: u32,
    pub log_destinations: Vec<LogDestination>,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Log formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Text,
    Structured,
}

/// Log destinations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogDestination {
    Console,
    File(String),
    Network(String),
    Database(String),
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    pub enable_alerts: bool,
    pub alert_rules: Vec<AlertRule>,
    pub notification_channels: Vec<NotificationChannel>,
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub rule_id: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub message: String,
    pub channels: Vec<String>,
}

/// Alert conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    MetricThreshold {
        metric: String,
        threshold: f64,
        operator: ComparisonOperator,
    },
    ErrorRate {
        threshold: f64,
        window: Duration,
    },
    ResourceUsage {
        resource: String,
        threshold: f64,
    },
    Custom(String, serde_json::Value),
}

/// Comparison operators for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Notification channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email {
        addresses: Vec<String>,
    },
    Slack {
        webhook_url: String,
        channel: String,
    },
    Webhook {
        url: String,
        headers: HashMap<String, String>,
    },
    SMS {
        phone_numbers: Vec<String>,
    },
}
