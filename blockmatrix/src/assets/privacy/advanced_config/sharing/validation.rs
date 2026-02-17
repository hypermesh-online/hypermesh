// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Validation implementations for sharing configuration types.

use crate::assets::core::{AssetResult, AssetError};

use super::types::*;

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
