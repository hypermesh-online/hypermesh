// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Default implementations for sharing configuration types.

use std::collections::HashMap;
use std::time::Duration;

use super::reporting::*;
use super::types::*;

impl Default for SharingMinimizationSettings {
    fn default() -> Self {
        Self {
            default_policy: SharingPolicy::MinimalSharing,
            per_recipient_rules: HashMap::new(),
            category_preferences: HashMap::new(),
            approval_workflows: Vec::new(),
        }
    }
}

impl Default for AnonymizationPreferences {
    fn default() -> Self {
        Self {
            preferred_techniques: vec![AnonymizationTechnique::KAnonymity],
            strength_preferences: AnonymizationStrengthPreferences::default(),
            risk_tolerance: ReidentificationRiskTolerance::default(),
            utility_requirements: UtilityPreservationRequirements::default(),
        }
    }
}

impl Default for AnonymizationStrengthPreferences {
    fn default() -> Self {
        Self {
            k_anonymity_level: Some(5),
            l_diversity_requirements: Some(2),
            t_closeness_requirements: Some(0.2),
            differential_privacy: None,
        }
    }
}

impl Default for ReidentificationRiskTolerance {
    fn default() -> Self {
        Self {
            max_risk_level: 0.1,
            assessment_frequency: Duration::from_secs(24 * 3600), // Daily
            mitigation_preferences: vec![RiskMitigationStrategy::IncreaseAnonymization],
            monitoring_requirements: RiskMonitoringRequirements::default(),
        }
    }
}

impl Default for RiskMonitoringRequirements {
    fn default() -> Self {
        Self {
            continuous_monitoring: true,
            alert_thresholds: HashMap::new(),
            automated_responses: Vec::new(),
            reporting_requirements: RiskReportingRequirements::default(),
        }
    }
}

impl Default for RiskReportingRequirements {
    fn default() -> Self {
        Self {
            frequency: ReportingFrequency::Daily,
            recipients: Vec::new(),
            detail_level: ReportDetailLevel::Standard,
            filtering_preferences: ReportFilteringPreferences::default(),
            delivery_preferences: DeliveryPreferences::default(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for ReportFilteringPreferences {
    fn default() -> Self {
        Self {
            risk_level_filter: Vec::new(),
            time_range_filters: Vec::new(),
            category_filters: Vec::new(),
            custom_filters: Vec::new(),
        }
    }
}

impl Default for DeliveryPreferences {
    fn default() -> Self {
        Self {
            method: DeliveryMethod::Dashboard,
            scheduling: DeliveryScheduling::default(),
            retry_settings: RetrySettings::default(),
            batch_settings: BatchDeliverySettings::default(),
            format_preferences: FormatPreferences::default(),
        }
    }
}

impl Default for DeliveryScheduling {
    fn default() -> Self {
        Self {
            immediate: true,
            scheduled_deliveries: Vec::new(),
            timezone: "UTC".to_string(),
        }
    }
}

impl Default for RetrySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            retry_delay: Duration::from_secs(300), // 5 minutes
            backoff_strategy: BackoffStrategy::Exponential,
        }
    }
}

impl Default for BatchDeliverySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            batch_size: 100,
            batch_timeout: Duration::from_secs(3600), // 1 hour
            force_delivery_threshold: 1000,
        }
    }
}

impl Default for FormatPreferences {
    fn default() -> Self {
        Self {
            format: ReportFormat::JSON,
            compression_enabled: false,
            encryption_enabled: true,
            localization: LocalizationPreferences::default(),
        }
    }
}

impl Default for LocalizationPreferences {
    fn default() -> Self {
        Self {
            language: "en-US".to_string(),
            date_format: "YYYY-MM-DD".to_string(),
            number_format: "1,234.56".to_string(),
            currency: "USD".to_string(),
        }
    }
}

impl Default for UtilityPreservationRequirements {
    fn default() -> Self {
        Self {
            min_utility_threshold: 0.8,
            utility_metrics: Vec::new(),
            tradeoff_preferences: UtilityTradeoffPreferences::default(),
            quality_assessment: QualityAssessmentRequirements::default(),
        }
    }
}

impl Default for UtilityTradeoffPreferences {
    fn default() -> Self {
        Self {
            privacy_utility_weight: 0.5,
            acceptable_utility_loss: 0.2,
            adaptive_adjustment: AdaptiveAdjustmentSettings::default(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for AdaptiveAdjustmentSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            triggers: Vec::new(),
            limits: AdjustmentLimits::default(),
            learning_parameters: HashMap::new(),
        }
    }
}

impl Default for AdjustmentLimits {
    fn default() -> Self {
        Self {
            max_privacy_adjustment: 0.1,
            max_utility_adjustment: 0.1,
            frequency_limits: HashMap::new(),
        }
    }
}

impl Default for QualityAssessmentRequirements {
    fn default() -> Self {
        Self {
            metrics: Vec::new(),
            assessment_frequency: Duration::from_secs(7 * 24 * 3600), // Weekly
            thresholds: QualityThresholds::default(),
            reporting: QualityReportingRequirements::default(),
        }
    }
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            minimum_quality: 0.7,
            target_quality: 0.9,
            degradation_tolerance: 0.1,
            assessment_methods: vec![QualityAssessmentMethod::Statistical],
        }
    }
}

impl Default for QualityReportingRequirements {
    fn default() -> Self {
        Self {
            frequency: ReportingFrequency::Weekly,
            recipients: Vec::new(),
            detail_level: ReportDetailLevel::Standard,
            alert_conditions: Vec::new(),
        }
    }
}
