// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Contribution session tracking and management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use super::pricing::Currency;
use crate::assets::core::{AssetAllocation, AssetType};

pub type UserId = String;
pub type ContributionId = String;

/// Contribution session tracking
#[derive(Debug, Clone)]
pub struct ContributionSession {
    pub session_id: ContributionId,
    pub user_id: UserId,
    pub asset_allocations: HashMap<AssetType, AssetAllocation>,
    pub started_at: SystemTime,
    pub expected_end: SystemTime,
    pub actual_usage: HashMap<AssetType, UsageMetrics>,
    pub earnings: SessionEarnings,
    pub performance_metrics: SessionPerformance,
    pub status: SessionStatus,
}

/// Usage metrics for a contribution session
#[derive(Debug, Clone, Default)]
pub struct UsageMetrics {
    pub total_usage_time: Duration,
    pub average_utilization: f64,
    pub peak_utilization: f64,
    pub efficiency_score: f64,
    pub user_satisfaction: Option<f64>,
}

/// Session earnings tracking
#[derive(Debug, Clone, Default)]
pub struct SessionEarnings {
    pub base_earnings: f64,
    pub performance_bonus: f64,
    pub reputation_bonus: f64,
    pub total_earnings: f64,
    pub currency: Currency,
    pub payout_status: PayoutStatus,
}

/// Payout status
#[derive(Debug, Clone, Default)]
pub enum PayoutStatus {
    #[default]
    Pending,
    Processing,
    Completed,
    Failed(String),
}

/// Session performance metrics
#[derive(Debug, Clone, Default)]
pub struct SessionPerformance {
    pub uptime_percentage: f64,
    pub response_time_avg: Duration,
    pub error_rate: f64,
    pub resource_efficiency: f64,
    pub user_rating: Option<f64>,
}

/// Session status
#[derive(Debug, Clone)]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Terminated(String),
    Error(String),
}

/// Account status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountStatus {
    Active,
    Suspended,
    Banned,
    PendingVerification,
    Closed,
}
