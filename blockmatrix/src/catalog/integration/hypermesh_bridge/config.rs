// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Execution and deployment configuration types for catalog deployments

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfiguration {
    pub max_execution_time: Duration,
    pub retry_policy: RetryPolicy,
    pub scaling_policy: ScalingPolicy,
    pub failure_handling: FailureHandling,
    pub checkpoint_policy: CheckpointPolicy,
}

/// Retry policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub backoff_strategy: BackoffStrategy,
    pub retry_conditions: Vec<RetryCondition>,
}

/// Backoff strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential,
    Custom(f64),
}

/// Retry conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    TransientError,
    ResourceUnavailable,
    TimeoutError,
    Custom(String),
}

/// Scaling policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPolicy {
    pub min_instances: u32,
    pub max_instances: u32,
    pub target_utilization: f64,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub scale_up_delay: Duration,
    pub scale_down_delay: Duration,
}

/// Failure handling strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureHandling {
    Ignore,
    Restart,
    Failover,
    Alert,
    Custom(String),
}

/// Checkpoint policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointPolicy {
    pub enable_checkpoints: bool,
    pub checkpoint_interval: Duration,
    pub max_checkpoints: u32,
    pub checkpoint_storage: CheckpointStorage,
}

/// Checkpoint storage options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointStorage {
    Local,
    Distributed,
    Cloud,
    Custom(String),
}
