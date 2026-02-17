// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Type definitions for container migration.

use crate::{NodeId, ContainerId};
use super::super::ResourceRequirements;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Migration decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationDecision {
    pub migration_id: String,
    pub container_id: ContainerId,
    pub source_node: NodeId,
    pub target_node: NodeId,
    pub reason: MigrationReason,
    pub strategy: MigrationStrategy,
    pub plan: MigrationPlan,
    pub expected_duration: Duration,
    pub confidence: f64,
    pub timestamp: SystemTime,
}

/// Migration reasons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationReason {
    NodeMaintenance { maintenance_window: Duration },
    LoadBalancing { current_load: f64, target_load: f64 },
    ResourceOptimization { resource_type: String, improvement: f64 },
    PerformanceOptimization { expected_improvement: f64 },
    NodeFailure { failure_type: String },
    CostOptimization { cost_savings: f64 },
    SecurityCompliance { policy_violation: String },
    Manual { reason: String },
}

/// Migration strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStrategy {
    LiveMigration,
    StopAndRestart,
    BlueGreen,
    Rolling,
    SnapshotRestore,
}

/// Migration plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub phases: Vec<MigrationPhase>,
    pub pre_migration_checks: Vec<PreMigrationCheck>,
    pub post_migration_validation: Vec<PostMigrationValidation>,
    pub rollback_plan: RollbackPlan,
    pub resource_requirements: MigrationResourceRequirements,
    pub network_considerations: NetworkMigrationPlan,
}

/// Migration phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPhase {
    pub phase_name: String,
    pub description: String,
    pub estimated_duration: Duration,
    pub operations: Vec<MigrationOperation>,
    pub success_criteria: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Migration operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationOperation {
    PrepareTarget { resource_allocation: ResourceRequirements },
    BeginMigration { migration_type: String },
    SyncState { sync_type: StateSync },
    UpdateNetworking { routing_changes: Vec<String> },
    SwitchTraffic { traffic_percentage: f64 },
    CleanupSource,
    Validate { validation_type: String },
}

/// State synchronization types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateSync { MemorySync, StorageSync, NetworkSync, ApplicationSync }

/// Pre-migration check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreMigrationCheck {
    pub check_name: String,
    pub description: String,
    pub check_type: CheckType,
    pub required: bool,
    pub timeout: Duration,
}

/// Check types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckType {
    ResourceAvailability,
    NetworkConnectivity,
    StorageAccessibility,
    SecurityPolicy,
    ApplicationHealth,
    Custom { check_command: String },
}

/// Post-migration validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMigrationValidation {
    pub validation_name: String,
    pub validation_type: ValidationType,
    pub expected_result: String,
    pub timeout: Duration,
}

/// Validation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    Performance { metrics: Vec<String> },
    Functional { test_suite: String },
    ResourceUsage,
    Network,
    Security,
}

/// Rollback plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    pub strategy: RollbackStrategy,
    pub triggers: Vec<RollbackTrigger>,
    pub steps: Vec<RollbackStep>,
    pub max_rollback_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackStrategy { Automatic, Manual, None }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackTrigger {
    MigrationTimeout,
    HealthCheckFailure { check_name: String },
    PerformanceDegradation { threshold: f64 },
    ResourceExhaustion { resource: String },
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackStep {
    pub step_name: String,
    pub operation: RollbackOperation,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackOperation { RestoreOriginal, RevertNetworking, CleanupTarget, RestoreTraffic }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResourceRequirements {
    pub additional_cpu: f64,
    pub additional_memory: u64,
    pub additional_storage: u64,
    pub network_bandwidth: u64,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMigrationPlan {
    pub dns_updates: Vec<DnsUpdate>,
    pub load_balancer_updates: Vec<LoadBalancerUpdate>,
    pub firewall_changes: Vec<FirewallChange>,
    pub latency_impact: LatencyImpact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsUpdate { pub record_name: String, pub old_value: String, pub new_value: String, pub ttl: Duration }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerUpdate { pub lb_id: String, pub update_type: LoadBalancerUpdateType, pub endpoints: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancerUpdateType { AddEndpoint, RemoveEndpoint, UpdateWeight { weight: f64 } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallChange { pub rule_id: String, pub change_type: FirewallChangeType, pub rule_spec: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FirewallChangeType { AddRule, RemoveRule, ModifyRule }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyImpact { pub expected_increase_ms: f64, pub impact_duration: Duration, pub mitigations: Vec<String> }

/// Migration execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationExecution {
    pub decision: MigrationDecision,
    pub status: MigrationStatus,
    pub current_phase: usize,
    pub started_at: SystemTime,
    pub progress: f64,
    pub execution_log: Vec<MigrationLogEntry>,
    pub performance_metrics: MigrationPerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationStatus {
    Planned, PreChecks, InProgress, Validating, Completed,
    Failed { error: String }, Cancelled { reason: String }, RollingBack, RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationLogEntry {
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub phase: String,
    pub message: String,
    pub data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel { Info, Warning, Error, Debug }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPerformanceMetrics {
    pub migration_start: SystemTime,
    pub phase_durations: HashMap<String, Duration>,
    pub downtime_duration: Option<Duration>,
    pub data_transfer: DataTransferMetrics,
    pub resource_usage: ResourceUsageDuringMigration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTransferMetrics { pub total_bytes: u64, pub transfer_rate: u64, pub transfer_duration: Duration }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageDuringMigration { pub peak_cpu_usage: f64, pub peak_memory_usage: f64, pub network_bandwidth_used: u64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord { pub execution: MigrationExecution, pub outcome: MigrationOutcome, pub lessons_learned: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationOutcome {
    Success { total_duration: Duration, downtime: Duration, performance_improvement: f64 },
    Failure { reason: String, failed_phase: String, recovery_action: String },
    Cancelled { reason: String, cleanup_completed: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationMetrics {
    pub total_migrations: u64,
    pub successful_migrations: u64,
    pub failed_migrations: u64,
    pub avg_migration_duration: Duration,
    pub avg_downtime: Duration,
    pub live_migration_success_rate: f64,
    pub efficiency_improvement: f64,
}
