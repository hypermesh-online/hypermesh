// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Scheduling dispatch: priority calculation, strategy selection, constraint creation.

use std::sync::Arc;
use std::time::{SystemTime, Duration};
use anyhow::Result;

use crate::catalog::vm::consensus::ConsensusOperation;
use super::super::context::ExecutionContext;
use super::{
    ExecutionScheduler, ExecutionPlan, ScheduledExecution,
    ExecutionPriority, OptimizationStrategy,
    ExecutionConstraints, ConsensusValidationLevel, AssetIsolationLevel,
    QueueStatus,
};

impl ExecutionScheduler {
    /// Schedule execution with consensus validation
    pub async fn schedule_execution(
        &self,
        consensus_operation: &ConsensusOperation,
        context: &Arc<ExecutionContext>,
        code: &str,
    ) -> Result<ExecutionPlan> {
        let execution_id = uuid::Uuid::new_v4().to_string();

        let required_assets = self.analyze_resource_requirements(code, &context.language).await?;

        let estimated_duration = self.estimate_execution_duration(
            code,
            &context.language,
            &required_assets,
        ).await?;

        let optimization_strategy = self.determine_optimization_strategy(context).await?;
        let constraints = self.create_execution_constraints(context).await?;
        let priority = self.determine_execution_priority(context).await?;
        let priority_score = self.calculate_priority_score(&priority, context).await?;

        let execution_plan = ExecutionPlan {
            execution_id: execution_id.clone(),
            code: code.to_string(),
            language: context.language.clone(),
            required_assets,
            consensus_operation: consensus_operation.clone(),
            priority,
            estimated_duration,
            optimization_strategy,
            constraints,
        };

        let scheduled_execution = ScheduledExecution {
            plan: execution_plan.clone(),
            context: Arc::clone(context),
            scheduled_at: SystemTime::now(),
            deadline: self.calculate_deadline(context).await?,
            priority_score,
        };

        {
            let mut queue = self.pending_queue.write().await;
            queue.push(scheduled_execution);
        }

        {
            let mut metrics = self.metrics.lock().await;
            metrics.total_scheduled += 1;
            metrics.queue_length = {
                let queue = self.pending_queue.read().await;
                queue.len()
            };
        }

        self.trigger_scheduling_cycle().await?;

        Ok(execution_plan)
    }

    /// Determine optimization strategy based on context
    pub(super) async fn determine_optimization_strategy(
        &self,
        _context: &ExecutionContext,
    ) -> Result<OptimizationStrategy> {
        Ok(OptimizationStrategy::Balanced)
    }

    /// Create execution constraints based on context
    pub(super) async fn create_execution_constraints(
        &self,
        _context: &ExecutionContext,
    ) -> Result<ExecutionConstraints> {
        Ok(ExecutionConstraints {
            max_execution_time: Some(Duration::from_secs(300)),
            max_memory_usage: Some(4 * 1024 * 1024 * 1024),
            max_cpu_cores: Some(8),
            min_consensus_level: ConsensusValidationLevel::Complete,
            asset_isolation: AssetIsolationLevel::Process,
        })
    }

    /// Determine execution priority based on context
    pub(super) async fn determine_execution_priority(
        &self,
        _context: &ExecutionContext,
    ) -> Result<ExecutionPriority> {
        Ok(ExecutionPriority::Normal)
    }

    /// Calculate priority score for queue ordering
    pub(super) async fn calculate_priority_score(
        &self,
        priority: &ExecutionPriority,
        _context: &ExecutionContext,
    ) -> Result<f64> {
        let mut score = *priority as u8 as f64 * 100.0;
        score += 10.0;
        score += 5.0;
        Ok(score)
    }

    /// Calculate execution deadline
    pub(super) async fn calculate_deadline(
        &self,
        _context: &ExecutionContext,
    ) -> Result<Option<SystemTime>> {
        Ok(Some(SystemTime::now() + Duration::from_secs(3600)))
    }

    /// Trigger scheduling cycle to process pending executions
    pub(super) async fn trigger_scheduling_cycle(&self) -> Result<()> {
        tracing::debug!("Scheduling cycle triggered");
        Ok(())
    }

    /// Get pending queue status
    pub async fn get_queue_status(&self) -> QueueStatus {
        let queue = self.pending_queue.read().await;
        let running = self.running_executions.read().await;

        QueueStatus {
            pending_count: queue.len(),
            running_count: running.len(),
            total_capacity: self.config.max_concurrent_executions,
            next_execution_eta: queue.peek().map(|exec| exec.scheduled_at),
        }
    }

    /// Update resource availability
    pub async fn update_resource_availability(
        &self,
        available_cpu: u32,
        available_memory: u64,
        available_gpu: u32,
        available_storage: u64,
        available_bandwidth: u64,
    ) -> Result<()> {
        let mut tracker = self.resource_tracker.write().await;
        tracker.available_cpu_cores = available_cpu;
        tracker.available_memory = available_memory;
        tracker.available_gpu_cores = available_gpu;
        tracker.available_storage = available_storage;
        tracker.available_bandwidth = available_bandwidth;
        Ok(())
    }
}
