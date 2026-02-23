// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Validation types - results, reports, traits, and performance metrics

use std::time::SystemTime;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

/// Validation result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationResult {
    /// Validation passed
    Passed {
        message: String,
        duration_ms: u64,
    },

    /// Validation failed
    Failed {
        reason: String,
        details: Vec<String>,
    },

    /// Validation skipped
    Skipped {
        reason: String,
    },
}

impl ValidationResult {
    /// Check if validation passed
    pub fn is_passed(&self) -> bool {
        matches!(self, ValidationResult::Passed { .. })
    }

    /// Check if validation failed
    pub fn is_failed(&self) -> bool {
        matches!(self, ValidationResult::Failed { .. })
    }
}

/// Validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Report ID
    pub id: String,

    /// Timestamp
    pub timestamp: SystemTime,

    /// Total validations run
    pub total_validations: usize,

    /// Passed validations
    pub passed: usize,

    /// Failed validations
    pub failed: usize,

    /// Skipped validations
    pub skipped: usize,

    /// Individual validation results
    pub results: HashMap<String, ValidationResult>,

    /// Component status
    pub component_status: HashMap<String, bool>,

    /// Performance metrics
    pub performance: PerformanceValidation,

    /// Overall success
    pub success: bool,
}

impl ValidationReport {
    /// Check if all validations passed
    pub fn all_healthy(&self) -> bool {
        self.failed == 0 && self.passed > 0
    }

    /// Get failure reasons
    pub fn get_failures(&self) -> Vec<(String, String)> {
        self.results
            .iter()
            .filter_map(|(name, result)| {
                if let ValidationResult::Failed { reason, .. } = result {
                    Some((name.clone(), reason.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Performance validation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceValidation {
    /// Asset processing time (ms)
    pub avg_processing_time_ms: u64,

    /// Asset retrieval time (ms)
    pub avg_retrieval_time_ms: u64,

    /// Deduplication rate
    pub deduplication_rate: f64,

    /// Network latency (ms)
    pub avg_network_latency_ms: u64,

    /// Storage efficiency
    pub storage_efficiency: f64,

    /// Meets performance targets
    pub meets_targets: bool,
}

/// Component validation trait
#[async_trait]
pub trait ComponentValidation {
    /// Validate component health
    async fn validate_health(&self) -> ValidationResult;

    /// Validate component configuration
    async fn validate_config(&self) -> ValidationResult;

    /// Validate component connectivity
    async fn validate_connectivity(&self) -> ValidationResult;
}

/// End-to-end validation trait
#[async_trait]
pub trait E2EValidation {
    /// Validate end-to-end workflow
    async fn validate_e2e(&self) -> ValidationResult;

    /// Validate performance targets
    async fn validate_performance(&self) -> ValidationResult;

    /// Validate data integrity
    async fn validate_integrity(&self) -> ValidationResult;
}
