//! Data Sharing Configuration
//!
//! Configuration for data sharing policies, approval workflows, and anonymization preferences.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::assets::core::{AssetResult, AssetError};

/// Data sharing minimization settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingMinimizationSettings {
    /// Default sharing policy
    pub default_policy: SharingPolicy,
    
    /// Per-recipient sharing rules
    pub per_recipient_rules: HashMap<String, SharingPolicy>,
    
    /// Data category sharing preferences
    pub category_preferences: HashMap<String, SharingPreference>,
    
    /// Sharing approval workflows
    pub approval_workflows: Vec<SharingApprovalWorkflow>,
}

/// Data sharing policies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SharingPolicy {
    NoSharing,
    MinimalSharing,
    ContextualSharing,
    StandardSharing,
    MaximalSharing,
}

/// Sharing preference for data categories
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingPreference {
    /// Allow sharing
    pub allow_sharing: bool,
    
    /// Anonymization requirements
    pub anonymization_required: bool,
    
    /// Purpose limitations
    pub purpose_limitations: Vec<String>,
    
    /// Retention limitations
    pub retention_limitations: Duration,
    
    /// Geographic limitations
    pub geographic_limitations: Vec<String>,
}

/// Sharing approval workflow
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingApprovalWorkflow {
    /// Workflow name
    pub name: String,
    
    /// Trigger conditions
    pub triggers: Vec<SharingTrigger>,
    
    /// Approval steps
    pub approval_steps: Vec<ApprovalStep>,
    
    /// Default action if no response
    pub default_action: SharingAction,
    
    /// Workflow timeout
    pub timeout: Duration,
}

/// Sharing approval triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingTrigger {
    /// Trigger type
    pub trigger_type: SharingTriggerType,
    
    /// Trigger conditions
    pub conditions: HashMap<String, String>,
    
    /// Priority level
    pub priority: u32,
}

/// Types of sharing triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SharingTriggerType {
    RecipientType,
    DataSensitivity,
    Purpose,
    Geographic,
    Temporal,
}

/// Approval step in workflow
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApprovalStep {
    /// Step name
    pub name: String,
    
    /// Approver requirements
    pub approvers: Vec<ApproverRequirement>,
    
    /// Step timeout
    pub timeout: Duration,
    
    /// Required consensus level
    pub consensus_level: ConsensusLevel,
    
    /// Step conditions
    pub conditions: Vec<String>,
}

/// Approver requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApproverRequirement {
    /// Approver type
    pub approver_type: ApproverType,
    
    /// Required qualifications
    pub qualifications: Vec<String>,
    
    /// Alternative approvers
    pub alternatives: Vec<String>,
    
    /// Escalation procedures
    pub escalation: Vec<String>,
}

/// Types of approvers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ApproverType {
    DataOwner,
    PrivacyOfficer,
    Administrator,
    LegalCounsel,
    ComplianceOfficer,
    ExternalAuditor,
}

/// Consensus levels for approval
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsensusLevel {
    Unanimous,
    Majority,
    Plurality,
    SingleApprover,
}

/// Sharing actions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SharingAction {
    Allow,
    Deny,
    Conditional,
    Escalate,
    Defer,
}

/// Anonymization preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnonymizationPreferences {
    /// Preferred anonymization techniques
    pub preferred_techniques: Vec<AnonymizationTechnique>,
    
    /// Anonymization strength preferences
    pub strength_preferences: AnonymizationStrengthPreferences,
    
    /// Re-identification risk tolerance
    pub risk_tolerance: ReidentificationRiskTolerance,
    
    /// Utility preservation requirements
    pub utility_requirements: UtilityPreservationRequirements,
}

/// Anonymization techniques
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnonymizationTechnique {
    KAnonymity,
    LDiversity,
    TCloseness,
    DifferentialPrivacy,
    DataMasking,
    Pseudonymization,
    Generalization,
    Suppression,
}

/// Anonymization strength preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnonymizationStrengthPreferences {
    /// K-anonymity level preference
    pub k_anonymity_level: Option<u32>,
    
    /// L-diversity requirements
    pub l_diversity_requirements: Option<u32>,
    
    /// T-closeness requirements
    pub t_closeness_requirements: Option<f32>,
    
    /// Differential privacy parameters
    pub differential_privacy: Option<DifferentialPrivacyPreferences>,
}

/// Differential privacy preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DifferentialPrivacyPreferences {
    /// Preferred epsilon value
    pub epsilon: f32,
    
    /// Preferred delta value
    pub delta: f32,
    
    /// Sensitivity tolerance
    pub sensitivity_tolerance: f32,
    
    /// Noise distribution preference
    pub noise_distribution: NoiseDistribution,
}

/// Noise distribution options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NoiseDistribution {
    Laplacian,
    Gaussian,
    Exponential,
    Custom { parameters: HashMap<String, f32> },
}

/// Re-identification risk tolerance
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReidentificationRiskTolerance {
    /// Maximum acceptable risk level
    pub max_risk_level: f32,
    
    /// Risk assessment frequency
    pub assessment_frequency: Duration,
    
    /// Risk mitigation preferences
    pub mitigation_preferences: Vec<RiskMitigationStrategy>,
    
    /// Monitoring requirements
    pub monitoring_requirements: RiskMonitoringRequirements,
}

/// Risk mitigation strategies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RiskMitigationStrategy {
    IncreaseAnonymization,
    ReduceDataSharing,
    ImproveAccessControls,
    EnhanceMonitoring,
    SeekExpertReview,
}

/// Risk monitoring requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskMonitoringRequirements {
    /// Continuous monitoring
    pub continuous_monitoring: bool,
    
    /// Alert thresholds
    pub alert_thresholds: HashMap<String, f32>,
    
    /// Automated responses
    pub automated_responses: Vec<AutomatedRiskResponse>,
    
    /// Reporting requirements
    pub reporting_requirements: RiskReportingRequirements,
}

/// Automated risk responses
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutomatedRiskResponse {
    /// Response trigger
    pub trigger: RiskResponseTrigger,
    
    /// Response action
    pub action: RiskResponseAction,
    
    /// Response delay
    pub delay: Duration,
    
    /// Confirmation requirements
    pub confirmation_required: bool,
}

/// Risk response triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RiskResponseTrigger {
    ThresholdExceeded { metric: String, threshold: f32 },
    TrendDetected { trend_type: String },
    AnomalyDetected { confidence: f32 },
    ExternalThreat { threat_level: String },
}

/// Risk response actions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RiskResponseAction {
    IncreasePrivacy,
    RestrictAccess,
    NotifyUser,
    EscalateToAdmin,
    ActivateContingency,
}

/// Risk reporting requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskReportingRequirements {
    /// Reporting frequency
    pub frequency: ReportingFrequency,
    
    /// Report recipients
    pub recipients: Vec<ReportRecipient>,
    
    /// Report detail level
    pub detail_level: ReportDetailLevel,
    
    /// Report filtering preferences
    pub filtering_preferences: ReportFilteringPreferences,
    
    /// Report delivery preferences
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
    /// Recipient identifier
    pub id: String,
    
    /// Recipient type
    pub recipient_type: RecipientType,
    
    /// Contact information
    pub contact_info: HashMap<String, String>,
    
    /// Delivery preferences
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
    /// Include only specific risk levels
    pub risk_level_filter: Vec<String>,
    
    /// Time range filters
    pub time_range_filters: Vec<TimeRangeFilter>,
    
    /// Category filters
    pub category_filters: Vec<String>,
    
    /// Custom filters
    pub custom_filters: Vec<CustomFilter>,
}

/// Time range filter
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeRangeFilter {
    /// Filter name
    pub name: String,
    
    /// Time specification
    pub time_spec: TimeSpecification,
    
    /// Recurring pattern
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
    /// Filter name
    pub name: String,
    
    /// Filter criteria
    pub criteria: HashMap<String, String>,
    
    /// Filter operation
    pub operation: String,
    
    /// Include or exclude
    pub include: bool,
}

/// Report delivery preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryPreferences {
    /// Preferred delivery method
    pub method: DeliveryMethod,
    
    /// Delivery scheduling
    pub scheduling: DeliveryScheduling,
    
    /// Retry settings
    pub retry_settings: RetrySettings,
    
    /// Batch delivery settings
    pub batch_settings: BatchDeliverySettings,
    
    /// Format preferences
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
    /// Immediate delivery
    pub immediate: bool,
    
    /// Scheduled deliveries
    pub scheduled_deliveries: Vec<ScheduledDelivery>,
    
    /// Delivery time zone
    pub timezone: String,
}

/// Scheduled delivery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledDelivery {
    /// Schedule name
    pub name: String,
    
    /// Delivery time
    pub time: String,
    
    /// Recurrence pattern
    pub recurrence: RecurringPattern,
}

/// Retry settings for delivery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetrySettings {
    /// Enable retries
    pub enabled: bool,
    
    /// Maximum retry attempts
    pub max_attempts: u32,
    
    /// Retry delay
    pub retry_delay: Duration,
    
    /// Backoff strategy
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
    /// Enable batching
    pub enabled: bool,
    
    /// Batch size limit
    pub batch_size: u32,
    
    /// Batch timeout
    pub batch_timeout: Duration,
    
    /// Force delivery threshold
    pub force_delivery_threshold: u32,
}

/// Format preferences for reports
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormatPreferences {
    /// Preferred format
    pub format: ReportFormat,
    
    /// Compression preferences
    pub compression_enabled: bool,
    
    /// Encryption preferences
    pub encryption_enabled: bool,
    
    /// Localization preferences
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
    /// Preferred language
    pub language: String,
    
    /// Date format preferences
    pub date_format: String,
    
    /// Number format preferences
    pub number_format: String,
    
    /// Currency preferences
    pub currency: String,
}

/// Utility preservation requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UtilityPreservationRequirements {
    /// Minimum utility threshold
    pub min_utility_threshold: f32,
    
    /// Utility metrics to preserve
    pub utility_metrics: Vec<UtilityMetric>,
    
    /// Trade-off preferences
    pub tradeoff_preferences: UtilityTradeoffPreferences,
    
    /// Quality assessment requirements
    pub quality_assessment: QualityAssessmentRequirements,
}

/// Utility metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UtilityMetric {
    /// Metric name
    pub name: String,
    
    /// Metric type
    pub metric_type: UtilityMetricType,
    
    /// Minimum acceptable value
    pub min_value: f32,
    
    /// Target value
    pub target_value: f32,
    
    /// Measurement frequency
    pub measurement_frequency: Duration,
}

/// Types of utility metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UtilityMetricType {
    Accuracy,
    Precision,
    Recall,
    F1Score,
    AUC,
    InformationLoss,
    CustomMetric { definition: String },
}

/// Utility trade-off preferences
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UtilityTradeoffPreferences {
    /// Privacy vs utility weighting
    pub privacy_utility_weight: f32,
    
    /// Acceptable utility loss
    pub acceptable_utility_loss: f32,
    
    /// Adaptive adjustment settings
    pub adaptive_adjustment: AdaptiveAdjustmentSettings,
}

/// Adaptive adjustment settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptiveAdjustmentSettings {
    /// Enable adaptive adjustments
    pub enabled: bool,
    
    /// Adjustment triggers
    pub triggers: Vec<AdjustmentTrigger>,
    
    /// Adjustment limits
    pub limits: AdjustmentLimits,
    
    /// Learning parameters
    pub learning_parameters: HashMap<String, f32>,
}

/// Adjustment triggers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdjustmentTrigger {
    /// Trigger condition
    pub condition: AdjustmentCondition,
    
    /// Threshold value
    pub threshold: f32,
    
    /// Adjustment direction
    pub direction: AdjustmentDirection,
    
    /// Adjustment magnitude
    pub magnitude: f32,
}

/// Adjustment conditions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdjustmentCondition {
    UtilityBelowThreshold,
    PrivacyAboveThreshold,
    RiskExceedsLimit,
    UserFeedback,
    PerformanceMetric,
}

/// Adjustment directions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdjustmentDirection {
    IncreasePrivacy,
    DecreasePrivacy,
    IncreaseUtility,
    DecreaseUtility,
    Balanced,
}

/// Adjustment limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdjustmentLimits {
    /// Maximum privacy adjustment
    pub max_privacy_adjustment: f32,
    
    /// Maximum utility adjustment
    pub max_utility_adjustment: f32,
    
    /// Adjustment frequency limits
    pub frequency_limits: HashMap<String, Duration>,
}

/// Quality assessment requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityAssessmentRequirements {
    /// Quality metrics
    pub metrics: Vec<QualityMetric>,
    
    /// Assessment frequency
    pub assessment_frequency: Duration,
    
    /// Quality thresholds
    pub thresholds: QualityThresholds,
    
    /// Reporting requirements
    pub reporting: QualityReportingRequirements,
}

/// Quality metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityMetric {
    /// Metric name
    pub name: String,
    
    /// Measurement method
    pub measurement_method: QualityMeasurementMethod,
    
    /// Expected range
    pub expected_range: (f32, f32),
    
    /// Critical threshold
    pub critical_threshold: f32,
}

/// Quality measurement methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QualityMeasurementMethod {
    Automated,
    ManualReview,
    UserFeedback,
    ExpertEvaluation,
    BenchmarkComparison,
}

/// Quality thresholds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityThresholds {
    /// Minimum acceptable quality
    pub minimum_quality: f32,
    
    /// Target quality level
    pub target_quality: f32,
    
    /// Quality degradation tolerance
    pub degradation_tolerance: f32,
    
    /// Assessment methods
    pub assessment_methods: Vec<QualityAssessmentMethod>,
}

/// Quality assessment methods
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QualityAssessmentMethod {
    Statistical,
    MachineLearning,
    UserStudy,
    ExpertReview,
    BenchmarkTesting,
}

/// Quality reporting requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityReportingRequirements {
    /// Reporting frequency
    pub frequency: ReportingFrequency,
    
    /// Report recipients
    pub recipients: Vec<String>,
    
    /// Report detail level
    pub detail_level: ReportDetailLevel,
    
    /// Alert conditions
    pub alert_conditions: Vec<String>,
}

// Validation implementations
impl SharingMinimizationSettings {
    pub fn validate(&self) -> AssetResult<()> {
        for workflow in &self.approval_workflows {
            workflow.validate()?;
        }
        Ok(())
    }
}

impl SharingApprovalWorkflow {
    pub fn validate(&self) -> AssetResult<()> {
        if self.name.trim().is_empty() {
            return Err(AssetError::ValidationError { message: "Workflow name cannot be empty".to_string() });
        }
        
        if self.approval_steps.is_empty() {
            return Err(AssetError::ValidationError { message: "Workflow must have at least one approval step".to_string() });
        }
        
        for step in &self.approval_steps {
            step.validate()?;
        }
        
        Ok(())
    }
}

impl ApprovalStep {
    pub fn validate(&self) -> AssetResult<()> {
        if self.name.trim().is_empty() {
            return Err(AssetError::ValidationError { message: "Approval step name cannot be empty".to_string() });
        }
        
        if self.approvers.is_empty() {
            return Err(AssetError::ValidationError { message: "Approval step must have at least one approver".to_string() });
        }
        
        Ok(())
    }
}

impl AnonymizationPreferences {
    pub fn validate(&self) -> AssetResult<()> {
        if self.preferred_techniques.is_empty() {
            return Err(AssetError::ValidationError { message: "At least one anonymization technique must be specified".to_string() });
        }
        
        self.risk_tolerance.validate()?;
        self.utility_requirements.validate()?;
        
        Ok(())
    }
}

impl ReidentificationRiskTolerance {
    pub fn validate(&self) -> AssetResult<()> {
        if self.max_risk_level < 0.0 || self.max_risk_level > 1.0 {
            return Err(AssetError::ValidationError { message: "Risk level must be between 0.0 and 1.0".to_string() });
        }
        
        self.monitoring_requirements.validate()?;
        
        Ok(())
    }
}

impl RiskMonitoringRequirements {
    pub fn validate(&self) -> AssetResult<()> {
        for response in &self.automated_responses {
            response.validate()?;
        }
        Ok(())
    }
}

impl AutomatedRiskResponse {
    pub fn validate(&self) -> AssetResult<()> {
        if self.delay.as_secs() == 0 {
            return Err(AssetError::ValidationError { message: "Response delay cannot be zero".to_string() });
        }
        Ok(())
    }
}

impl UtilityPreservationRequirements {
    pub fn validate(&self) -> AssetResult<()> {
        if self.min_utility_threshold < 0.0 || self.min_utility_threshold > 1.0 {
            return Err(AssetError::ValidationError { message: "Utility threshold must be between 0.0 and 1.0".to_string() });
        }
        
        self.quality_assessment.validate()?;
        
        Ok(())
    }
}

impl QualityAssessmentRequirements {
    pub fn validate(&self) -> AssetResult<()> {
        if self.metrics.is_empty() {
            return Err(AssetError::ValidationError { message: "At least one quality metric must be specified".to_string() });
        }
        Ok(())
    }
}

// Default implementations
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