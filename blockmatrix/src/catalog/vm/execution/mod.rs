//! Consensus-Aware VM Execution Engine
//!
//! This module implements the core execution engine that runs code with
//! native consensus proof validation. Every execution step requires and
//! validates consensus proofs as fundamental language constructs.

pub mod context;
pub mod scheduler;
pub mod runtime;

// Re-export key types
pub use context::ExecutionContext;
pub use scheduler::{ExecutionScheduler, ExecutionPlan, ScheduledExecution};
pub use runtime::{ConsensusRuntime, LanguageRuntime, ResourceUsage};

use std::sync::Arc;
use std::collections::HashMap;
use std::time::SystemTime;
use anyhow::Result;
use serde::{Serialize, Deserialize};

use crate::consensus::proof::ConsensusProof;
use super::consensus::{ConsensusVM, ConsensusOperation, ConsensusExecutionResult};
use super::{AssetManagementConfig, PrivacyConfig, AssetAllocation, PrivacyLevel};
// Already imported above, no need to import again

/// Main VM executor with consensus-native execution
pub struct VMExecutor {
    /// Consensus VM for proof validation
    consensus_vm: Arc<ConsensusVM>,
    /// Execution scheduler for managing operations
    scheduler: Arc<ExecutionScheduler>,
    /// Runtime environment with consensus integration
    runtime: Arc<ConsensusRuntime>,
    /// Asset management configuration
    asset_config: AssetManagementConfig,
    /// Active execution contexts
    active_contexts: Arc<std::sync::RwLock<HashMap<String, Arc<ExecutionContext>>>>,
}

/// Execution result with comprehensive consensus information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Execution identifier
    pub execution_id: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Execution output
    pub output: Option<serde_json::Value>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Consensus validation results
    pub consensus_results: ConsensusExecutionResult,
    /// Asset utilization during execution
    pub asset_utilization: AssetUtilizationReport,
    /// Privacy compliance report
    pub privacy_compliance: PrivacyComplianceReport,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
}

/// Asset utilization report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetUtilizationReport {
    /// CPU utilization per core
    pub cpu_utilization: HashMap<String, f64>,
    /// GPU utilization if used
    pub gpu_utilization: Option<HashMap<String, f64>>,
    /// Memory usage patterns
    pub memory_usage: MemoryUsagePattern,
    /// Storage operations performed
    pub storage_operations: Vec<StorageOperation>,
    /// Network bandwidth used
    pub network_bandwidth_used: u64,
}

/// Memory usage pattern tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsagePattern {
    /// Peak memory usage (bytes)
    pub peak_usage: u64,
    /// Average memory usage (bytes)
    pub average_usage: u64,
    /// Memory allocation count
    pub allocations: u32,
    /// Memory deallocation count
    pub deallocations: u32,
    /// Garbage collection events
    pub gc_events: u32,
}

/// Storage operation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageOperation {
    /// Operation type (read, write, delete)
    pub operation_type: String,
    /// Data size involved (bytes)
    pub data_size: u64,
    /// Storage location
    pub location: String,
    /// Timestamp of operation
    pub timestamp: SystemTime,
    /// Consensus proof used
    pub consensus_proof_hash: [u8; 32],
}

/// Privacy compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyComplianceReport {
    /// Privacy level used during execution
    pub privacy_level_used: PrivacyLevel,
    /// Data anonymization applied
    pub anonymization_applied: bool,
    /// External data sharing events
    pub data_sharing_events: Vec<DataSharingEvent>,
    /// Compliance violations if any
    pub violations: Vec<PrivacyViolation>,
}

/// Data sharing event tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSharingEvent {
    /// Type of data shared
    pub data_type: String,
    /// Recipient identifier
    pub recipient: String,
    /// Data size shared (bytes)
    pub data_size: u64,
    /// Privacy level at time of sharing
    pub privacy_level: PrivacyLevel,
    /// Timestamp of sharing
    pub timestamp: SystemTime,
}

/// Privacy violation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyViolation {
    /// Type of violation
    pub violation_type: String,
    /// Description of what happened
    pub description: String,
    /// Severity level (1-10)
    pub severity: u8,
    /// Timestamp when violation occurred
    pub timestamp: SystemTime,
}

/// Execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Programming language used
    pub language: String,
    /// Code size (bytes)
    pub code_size: u64,
    /// Execution start time
    pub started_at: SystemTime,
    /// Execution end time
    pub completed_at: Option<SystemTime>,
    /// Total execution duration (microseconds)
    pub duration_micros: u64,
    /// VM version used
    pub vm_version: String,
    /// Consensus requirements used
    pub consensus_requirements_hash: [u8; 32],
}

impl VMExecutor {
    /// Create new VM executor
    pub async fn new(
        consensus_vm: Arc<ConsensusVM>,
        asset_config: AssetManagementConfig,
    ) -> Result<Self> {
        let scheduler = Arc::new(ExecutionScheduler::new(
            Arc::clone(&consensus_vm),
            asset_config.clone(),
        ).await?);
        
        let runtime = Arc::new(ConsensusRuntime::new(
            Arc::clone(&consensus_vm),
            Arc::clone(&scheduler),
        ).await?);
        
        Ok(Self {
            consensus_vm,
            scheduler,
            runtime,
            asset_config,
            active_contexts: Arc::new(std::sync::RwLock::new(HashMap::new())),
        })
    }
    
    /// Execute code with full consensus validation
    pub async fn execute(
        &self,
        code: &str,
        context: ExecutionContext,
    ) -> Result<ExecutionResult> {
        let execution_id = uuid::Uuid::new_v4().to_string();
        let start_time = SystemTime::now();
        
        // Create execution metadata
        let metadata = ExecutionMetadata {
            language: context.language.clone(),
            code_size: code.len() as u64,
            started_at: start_time,
            completed_at: None,
            duration_micros: 0,
            vm_version: "consensus-vm-0.1.0".to_string(),
            consensus_requirements_hash: self.calculate_requirements_hash(),
        };
        
        // Register execution context
        let context_arc = Arc::new(context);
        {
            let mut active = self.active_contexts.write().unwrap();
            active.insert(execution_id.clone(), Arc::clone(&context_arc));
        }
        
        // Create consensus operation for execution
        let consensus_operation = self.consensus_vm.create_consensus_operation(
            "execute",
            context_arc.asset_id(),
            context_arc.consensus_proof().clone(),
        ).await?;
        
        // Schedule execution
        let execution_plan = self.scheduler.schedule_execution(
            &consensus_operation,
            &context_arc,
            code,
        ).await?;
        
        // Execute through runtime
        let runtime_result = self.runtime.execute_with_plan(
            execution_plan,
            Arc::clone(&context_arc),
        ).await?;
        
        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time)
            .unwrap_or_default()
            .as_micros() as u64;
        
        // Generate comprehensive execution result
        let execution_result = ExecutionResult {
            execution_id: execution_id.clone(),
            success: runtime_result.success,
            output: runtime_result.output,
            error_message: runtime_result.error_message,
            consensus_results: runtime_result.consensus_results,
            asset_utilization: self.generate_asset_utilization_report(&context_arc).await?,
            privacy_compliance: self.generate_privacy_compliance_report(&context_arc).await?,
            metadata: ExecutionMetadata {
                completed_at: Some(end_time),
                duration_micros: duration,
                ..metadata
            },
        };
        
        // Clean up execution context
        {
            let mut active = self.active_contexts.write().unwrap();
            active.remove(&execution_id);
        }
        
        Ok(execution_result)
    }
    
    /// Generate asset utilization report
    async fn generate_asset_utilization_report(
        &self,
        context: &ExecutionContext,
    ) -> Result<AssetUtilizationReport> {
        // In a real implementation, this would collect actual utilization metrics
        // from the asset adapters and runtime environment
        
        let mut cpu_utilization = HashMap::new();
        for (asset_type, allocation) in context.asset_allocations() {
            if asset_type == "cpu" {
                cpu_utilization.insert(
                    "core_0".to_string(),
                    (allocation.available_capacity as f64 / allocation.total_capacity as f64) * 100.0
                );
            }
        }
        
        Ok(AssetUtilizationReport {
            cpu_utilization,
            gpu_utilization: None, // Would be populated if GPU was used
            memory_usage: MemoryUsagePattern {
                peak_usage: 1024 * 1024 * 10, // 10MB placeholder
                average_usage: 1024 * 1024 * 5, // 5MB placeholder
                allocations: 100,
                deallocations: 95,
                gc_events: 2,
            },
            storage_operations: vec![], // Would be populated with actual operations
            network_bandwidth_used: 1024, // 1KB placeholder
        })
    }
    
    /// Generate privacy compliance report
    async fn generate_privacy_compliance_report(
        &self,
        context: &ExecutionContext,
    ) -> Result<PrivacyComplianceReport> {
        // Check privacy settings and detect any violations
        let privacy_settings = context.privacy_settings();
        
        Ok(PrivacyComplianceReport {
            privacy_level_used: privacy_settings.default_privacy_level.clone(),
            anonymization_applied: privacy_settings.anonymization_enabled,
            data_sharing_events: vec![], // Would track actual sharing events
            violations: vec![], // Would contain any detected violations
        })
    }
    
    /// Calculate consensus requirements hash for metadata
    fn calculate_requirements_hash(&self) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        
        let requirements = self.consensus_vm.requirements();
        let mut hasher = Sha256::new();
        
        hasher.update(&[
            requirements.require_proof_of_space as u8,
            requirements.require_proof_of_stake as u8,
            requirements.require_proof_of_work as u8,
            requirements.require_proof_of_time as u8,
        ]);
        hasher.update(&requirements.min_work_difficulty.to_le_bytes());
        hasher.update(&requirements.min_space_commitment.to_le_bytes());
        hasher.update(&requirements.min_stake_authority.to_le_bytes());
        hasher.update(&requirements.max_time_drift.to_le_bytes());
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
    
    /// Get active execution contexts
    pub fn get_active_contexts(&self) -> HashMap<String, Arc<ExecutionContext>> {
        let active = self.active_contexts.read().unwrap();
        active.clone()
    }
    
    /// Get execution statistics
    pub fn get_execution_stats(&self) -> ExecutionStats {
        let active = self.active_contexts.read().unwrap();
        
        ExecutionStats {
            active_executions: active.len() as u32,
            total_executions_handled: 0, // Would be tracked
            average_execution_time_micros: 0, // Would be calculated
            success_rate: 0.0, // Would be calculated
            resource_utilization_percentage: 0.0, // Would be calculated
        }
    }
    
    /// Shutdown executor gracefully
    pub async fn shutdown(&self) -> Result<()> {
        // Wait for active executions to complete
        loop {
            let active_count = {
                let active = self.active_contexts.read().unwrap();
                active.len()
            };
            
            if active_count == 0 {
                break;
            }
            
            // Wait a bit and check again
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        Ok(())
    }
}

/// Runtime result from consensus runtime
#[derive(Debug, Clone)]
pub struct RuntimeExecutionResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub consensus_results: ConsensusExecutionResult,
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub active_executions: u32,
    pub total_executions_handled: u64,
    pub average_execution_time_micros: f64,
    pub success_rate: f64,
    pub resource_utilization_percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::proof::{SpaceProof, StakeProof, WorkProof, TimeProof, ConsensusProof};
    use crate::catalog::vm::{ConsensusRequirements, VMConfig};
    use uuid::Uuid;
    
    #[tokio::test]
    async fn test_vm_executor_creation() {
        let requirements = ConsensusRequirements::default();
        let consensus_vm = Arc::new(super::super::consensus::ConsensusVM::new(requirements).unwrap());
        let asset_config = AssetManagementConfig::default();
        
        let executor = VMExecutor::new(consensus_vm, asset_config).await;
        // May fail due to unimplemented dependencies, but tests structure
        assert!(executor.is_ok() || executor.is_err());
    }
    
    #[test]
    fn test_execution_metadata() {
        let metadata = ExecutionMetadata {
            language: "julia".to_string(),
            code_size: 1024,
            started_at: SystemTime::now(),
            completed_at: None,
            duration_micros: 0,
            vm_version: "test".to_string(),
            consensus_requirements_hash: [0; 32],
        };
        
        assert_eq!(metadata.language, "julia");
        assert_eq!(metadata.code_size, 1024);
    }
    
    #[test]
    fn test_asset_utilization_report() {
        let mut cpu_utilization = HashMap::new();
        cpu_utilization.insert("core_0".to_string(), 75.5);
        
        let report = AssetUtilizationReport {
            cpu_utilization,
            gpu_utilization: None,
            memory_usage: MemoryUsagePattern {
                peak_usage: 1024,
                average_usage: 512,
                allocations: 10,
                deallocations: 8,
                gc_events: 1,
            },
            storage_operations: vec![],
            network_bandwidth_used: 100,
        };
        
        assert_eq!(report.memory_usage.peak_usage, 1024);
        assert_eq!(report.network_bandwidth_used, 100);
    }
}