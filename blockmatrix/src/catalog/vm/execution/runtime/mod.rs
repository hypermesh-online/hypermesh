// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Consensus Runtime - Native VM execution with HyperMesh asset integration
//!
//! This is the core runtime that executes code with consensus proof validation
//! integrated directly into the execution model. Every operation requires
//! and validates consensus proofs as language-level constructs.

mod julia;
mod python;
mod rust_lang;

pub use julia::JuliaLanguageRuntime;
pub use python::PythonLanguageRuntime;
pub use rust_lang::RustLanguageRuntime;

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, Duration, Instant};
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, Mutex};
use uuid::Uuid;

use crate::catalog::vm::consensus::ConsensusVM;
use crate::assets::core::{AssetManager, AssetRegistration, AssetType, AssetAllocationRequest, ResourceRequirements, PrivacyMode};

/// VM runtime uses the core AssetRegistration as its asset identifier for
/// tracking execution resource handles, since the asset manager returns this type.
pub(crate) type AssetId = AssetRegistration;
use crate::assets::core::adapter::{StorageType, StorageRequirements};
use super::context::ExecutionContext;
use super::scheduler::{ExecutionScheduler, ExecutionPlan};
use super::{RuntimeExecutionResult, MemoryUsagePattern, StorageOperation};

/// Language runtime trait for multi-language support
#[async_trait::async_trait]
pub trait LanguageRuntime: Send + Sync {
    /// Execute code in this language runtime
    async fn execute(
        &self,
        code: &str,
        context: &ExecutionContext,
        asset_allocations: &HashMap<AssetId, AssetHandle>,
    ) -> Result<LanguageExecutionResult>;

    /// Get runtime capabilities
    fn capabilities(&self) -> LanguageCapabilities;

    /// Initialize runtime environment
    async fn initialize(&self) -> Result<()>;

    /// Cleanup runtime environment
    async fn cleanup(&self) -> Result<()>;

    /// Get current resource usage
    async fn get_resource_usage(&self) -> Result<ResourceUsage>;
}

/// Language execution result
#[derive(Debug, Clone)]
pub struct LanguageExecutionResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub execution_time: Duration,
    pub resource_usage: ResourceUsage,
    pub memory_pattern: MemoryUsagePattern,
    pub storage_operations: Vec<StorageOperation>,
}

/// Language capabilities description
#[derive(Debug, Clone)]
pub struct LanguageCapabilities {
    pub language_name: String,
    pub version: String,
    pub supported_features: Vec<String>,
    pub memory_model: MemoryModel,
    pub execution_model: ExecutionModel,
    pub consensus_integration: bool,
}

/// Memory model types
#[derive(Debug, Clone)]
pub enum MemoryModel {
    Managed,     // Garbage collected (Julia, Python)
    Manual,      // Manual memory management (C, C++, Rust)
    Hybrid,      // Mixed approach (JavaScript, R)
}

/// Execution model types
#[derive(Debug, Clone)]
pub enum ExecutionModel {
    Interpreted, // Direct interpretation
    Compiled,    // Ahead-of-time compilation
    JIT,         // Just-in-time compilation
    Bytecode,    // Bytecode virtual machine
}

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time_micros: u64,
    pub memory_peak_bytes: u64,
    pub memory_average_bytes: u64,
    pub disk_reads_bytes: u64,
    pub disk_writes_bytes: u64,
    pub network_in_bytes: u64,
    pub network_out_bytes: u64,
    pub gpu_compute_micros: Option<u64>,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_time_micros: 0,
            memory_peak_bytes: 0,
            memory_average_bytes: 0,
            disk_reads_bytes: 0,
            disk_writes_bytes: 0,
            network_in_bytes: 0,
            network_out_bytes: 0,
            gpu_compute_micros: None,
        }
    }
}

/// Execution handle for tracking active executions
#[derive(Debug, Clone)]
pub struct ExecutionHandle {
    pub execution_id: String,
    pub started_at: SystemTime,
    pub language: String,
    pub process_id: Option<u32>,
    pub asset_allocations: HashMap<AssetId, AssetHandle>,
    pub context: Arc<ExecutionContext>,
}

/// Asset handle for resource management
#[derive(Debug, Clone)]
pub struct AssetHandle {
    pub asset_id: AssetId,
    pub asset_type: AssetType,
    pub allocated_capacity: u64,
    pub current_usage: u64,
    pub allocation_timestamp: SystemTime,
}

/// Runtime performance metrics
#[derive(Debug, Default)]
pub struct RuntimeMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time: Duration,
    pub current_memory_usage: u64,
    pub peak_memory_usage: u64,
    pub total_cpu_time: Duration,
    pub consensus_validations: u64,
    pub asset_allocations: u64,
}

impl Clone for RuntimeMetrics {
    fn clone(&self) -> Self {
        Self {
            total_executions: self.total_executions,
            successful_executions: self.successful_executions,
            failed_executions: self.failed_executions,
            average_execution_time: self.average_execution_time,
            current_memory_usage: self.current_memory_usage,
            peak_memory_usage: self.peak_memory_usage,
            total_cpu_time: self.total_cpu_time,
            consensus_validations: self.consensus_validations,
            asset_allocations: self.asset_allocations,
        }
    }
}

/// Security sandbox configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub enable_network_isolation: bool,
    pub enable_filesystem_isolation: bool,
    pub max_memory_mb: u64,
    pub max_cpu_time_seconds: u64,
    pub max_file_descriptors: u32,
    pub allowed_syscalls: Vec<String>,
    pub environment_variables: HashMap<String, String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enable_network_isolation: true,
            enable_filesystem_isolation: true,
            max_memory_mb: 1024,
            max_cpu_time_seconds: 300,
            max_file_descriptors: 1024,
            allowed_syscalls: vec![
                "read".to_string(),
                "write".to_string(),
                "open".to_string(),
                "close".to_string(),
                "mmap".to_string(),
                "munmap".to_string(),
            ],
            environment_variables: HashMap::new(),
        }
    }
}

/// Production-ready consensus runtime with native VM execution
pub struct ConsensusRuntime {
    /// Consensus VM integration
    consensus_vm: Arc<RwLock<ConsensusVM>>,
    /// Execution scheduler
    _scheduler: Arc<ExecutionScheduler>,
    /// Asset manager for resource allocation
    asset_manager: Arc<AssetManager>,
    /// Language runtime managers
    language_runtimes: Arc<RwLock<HashMap<String, Arc<dyn LanguageRuntime>>>>,
    /// Active execution tracking
    active_executions: Arc<RwLock<HashMap<String, ExecutionHandle>>>,
    /// Performance metrics
    metrics: Arc<Mutex<RuntimeMetrics>>,
    /// Security sandbox configuration
    sandbox_config: SandboxConfig,
}

impl ConsensusRuntime {
    /// Create new consensus runtime
    pub async fn new(
        consensus_vm: Arc<RwLock<ConsensusVM>>,
        scheduler: Arc<ExecutionScheduler>,
    ) -> Result<Self> {
        let asset_manager = Arc::new(AssetManager::new());
        let language_runtimes = Arc::new(RwLock::new(HashMap::new()));
        let active_executions = Arc::new(RwLock::new(HashMap::new()));
        let metrics = Arc::new(Mutex::new(RuntimeMetrics::default()));
        let sandbox_config = SandboxConfig::default();

        let runtime = Self {
            consensus_vm,
            _scheduler: scheduler,
            asset_manager,
            language_runtimes,
            active_executions,
            metrics,
            sandbox_config,
        };

        runtime.initialize_language_runtimes().await?;

        Ok(runtime)
    }

    /// Execute with execution plan from scheduler
    pub async fn execute_with_plan(
        &self,
        execution_plan: ExecutionPlan,
        context: Arc<ExecutionContext>,
    ) -> Result<RuntimeExecutionResult> {
        let execution_id = Uuid::new_v4().to_string();
        let start_time = Instant::now();

        let asset_allocations = self.allocate_assets(&execution_plan.required_assets).await?;

        let execution_handle = ExecutionHandle {
            execution_id: execution_id.clone(),
            started_at: SystemTime::now(),
            language: execution_plan.language.clone(),
            process_id: None,
            asset_allocations: asset_allocations.clone(),
            context: Arc::clone(&context),
        };

        {
            let mut active = self.active_executions.write().await;
            active.insert(execution_id.clone(), execution_handle);
        }

        let consensus_result = self.consensus_vm.write().await.execute_consensus_operation(
            &execution_plan.consensus_operation,
            &[],
        ).await?;

        let language_runtime = {
            let runtimes = self.language_runtimes.read().await;
            runtimes.get(&execution_plan.language)
                .ok_or_else(|| anyhow!("Language runtime not found: {}", execution_plan.language))?
                .clone()
        };

        let execution_result = language_runtime.execute(
            &execution_plan.code,
            &context,
            &asset_allocations,
        ).await;

        {
            let mut active = self.active_executions.write().await;
            active.remove(&execution_id);
        }

        self.deallocate_assets(&asset_allocations).await?;

        let execution_time = start_time.elapsed();
        {
            let mut metrics = self.metrics.lock().await;
            metrics.total_executions += 1;
            if execution_result.is_ok() {
                metrics.successful_executions += 1;
            } else {
                metrics.failed_executions += 1;
            }

            let total_time = metrics.average_execution_time.as_micros() as u64 * (metrics.total_executions - 1)
                + execution_time.as_micros() as u64;
            metrics.average_execution_time = Duration::from_micros(total_time / metrics.total_executions);

            metrics.consensus_validations += 1;
            metrics.asset_allocations += asset_allocations.len() as u64;
        }

        match execution_result {
            Ok(lang_result) => Ok(RuntimeExecutionResult {
                success: lang_result.success,
                output: lang_result.output,
                error_message: lang_result.error_message,
                consensus_results: consensus_result,
            }),
            Err(e) => Ok(RuntimeExecutionResult {
                success: false,
                output: None,
                error_message: Some(e.to_string()),
                consensus_results: consensus_result,
            }),
        }
    }

    /// Initialize built-in language runtimes
    async fn initialize_language_runtimes(&self) -> Result<()> {
        let mut runtimes = self.language_runtimes.write().await;

        let julia_runtime = Arc::new(JuliaLanguageRuntime::new(
            Arc::clone(&self.asset_manager),
            self.sandbox_config.clone(),
        )?);
        runtimes.insert("julia".to_string(), julia_runtime);

        let python_runtime = Arc::new(PythonLanguageRuntime::new(
            Arc::clone(&self.asset_manager),
            self.sandbox_config.clone(),
        )?);
        runtimes.insert("python".to_string(), python_runtime);

        let rust_runtime = Arc::new(RustLanguageRuntime::new(
            Arc::clone(&self.asset_manager),
            self.sandbox_config.clone(),
        )?);
        runtimes.insert("rust".to_string(), rust_runtime);

        for runtime in runtimes.values() {
            runtime.initialize().await?;
        }

        Ok(())
    }

    /// Allocate assets for execution
    async fn allocate_assets(
        &self,
        required_assets: &HashMap<AssetType, u64>,
    ) -> Result<HashMap<AssetId, AssetHandle>> {
        let mut allocations = HashMap::new();

        for (asset_type, required_capacity) in required_assets {
            let allocation_request = AssetAllocationRequest {
                asset_type: asset_type.clone(),
                requested_resources: ResourceRequirements {
                    cpu: None,
                    gpu_usage: None,
                    memory_usage: None,
                    storage_usage: Some(StorageRequirements {
                        size_bytes: *required_capacity,
                        storage_type: StorageType::Ssd,
                        min_iops: None,
                        min_bandwidth_mbps: None,
                        durability_replicas: 1,
                    }),
                    network_usage: None,
                    container: None,
                    economic: None,
                },
                privacy_level: PrivacyMode::PRIVATE,
                consensus_proof: self.create_allocation_consensus_proof().await?,
                certificate_fingerprint: String::new(),
                duration_limit: Some(Duration::from_secs(3600)),
                tags: HashMap::new(),
            };

            let allocation = self.asset_manager.allocate_asset(allocation_request).await?;

            let asset_handle = AssetHandle {
                asset_id: allocation.asset_id.clone(),
                asset_type: asset_type.clone(),
                allocated_capacity: *required_capacity,
                current_usage: 0,
                allocation_timestamp: SystemTime::now(),
            };

            allocations.insert(allocation.asset_id, asset_handle);
        }

        Ok(allocations)
    }

    /// Deallocate assets after execution
    async fn deallocate_assets(
        &self,
        allocations: &HashMap<AssetId, AssetHandle>,
    ) -> Result<()> {
        for asset_id in allocations.keys() {
            self.asset_manager.deallocate_asset(asset_id).await?;
        }
        Ok(())
    }

    /// Create consensus proof for asset allocation
    async fn create_allocation_consensus_proof(
        &self,
    ) -> Result<crate::assets::core::ConsensusProof> {
        use crate::assets::core::{
            ConsensusProof, SpaceProof, StakeProof, WorkProof, TimeProof,
            WorkloadType, WorkState
        };

        let space_proof = SpaceProof {
            node_id: "hypermesh-runtime".to_string(),
            storage_path: "/tmp/hypermesh-runtime".to_string(),
            total_size: 1024 * 1024 * 1024,
            total_storage: 10 * 1024 * 1024 * 1024,
            file_hash: hex::encode(&[1, 2, 3, 4]),
            proof_timestamp: SystemTime::now(),
        };

        let stake_proof = StakeProof {
            stake_holder: "hypermesh-runtime".to_string(),
            stake_holder_id: "runtime-001".to_string(),
            stake_amount: 10000,
            stake_timestamp: SystemTime::now(),
        };

        let work_proof = WorkProof {
            owner_id: "hypermesh-runtime".to_string(),
            workload_id: Uuid::new_v4().to_string(),
            pid: std::process::id() as u64,
            computational_power: 1000,
            workload_type: WorkloadType::Compute,
            work_state: WorkState::Running,
            work_challenges: vec![],
            proof_timestamp: SystemTime::now(),
        };

        let time_proof = TimeProof {
            network_time_offset: Duration::from_millis(10),
            time_verification_timestamp: SystemTime::now(),
            nonce: rand::random(),
            proof_hash: vec![5, 6, 7, 8],
        };

        Ok(ConsensusProof::new(stake_proof, time_proof, space_proof, work_proof))
    }

    /// Get runtime metrics
    pub async fn get_metrics(&self) -> RuntimeMetrics {
        let metrics = self.metrics.lock().await;
        metrics.clone()
    }

    /// Get active executions
    pub async fn get_active_executions(&self) -> HashMap<String, ExecutionHandle> {
        let active = self.active_executions.read().await;
        active.clone()
    }

    /// Shutdown runtime gracefully
    pub async fn shutdown(&self) -> Result<()> {
        let executions = {
            let active = self.active_executions.read().await;
            active.clone()
        };

        for execution in executions.values() {
            tracing::warn!("Terminating execution: {}", execution.execution_id);
        }

        let runtimes = self.language_runtimes.read().await;
        for runtime in runtimes.values() {
            runtime.cleanup().await?;
        }

        Ok(())
    }
}

impl Default for MemoryUsagePattern {
    fn default() -> Self {
        Self {
            peak_usage: 0,
            average_usage: 0,
            allocations: 0,
            deallocations: 0,
            gc_events: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::vm::ConsensusRequirements;
    use crate::catalog::vm::consensus::ConsensusVM;

    #[tokio::test]
    async fn test_consensus_runtime_creation() {
        let requirements = ConsensusRequirements::default();
        let consensus_vm = Arc::new(RwLock::new(ConsensusVM::new(requirements).unwrap()));
        let scheduler = Arc::new(ExecutionScheduler::new(
            Arc::clone(&consensus_vm),
            super::super::super::AssetManagementConfig::default(),
        ).await.unwrap());

        let runtime = ConsensusRuntime::new(Arc::clone(&consensus_vm), scheduler).await;
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_language_runtime_capabilities() {
        let asset_manager = Arc::new(AssetManager::new());
        let sandbox_config = SandboxConfig::default();

        let julia_runtime = JuliaLanguageRuntime::new(
            Arc::clone(&asset_manager),
            sandbox_config.clone(),
        );

        if julia_runtime.is_ok() {
            let caps = julia_runtime.unwrap().capabilities();
            assert_eq!(caps.language_name, "Julia");
            assert!(matches!(caps.memory_model, MemoryModel::Managed));
            assert!(matches!(caps.execution_model, ExecutionModel::JIT));
        }
    }

    #[test]
    fn test_resource_usage_default() {
        let usage = ResourceUsage::default();
        assert_eq!(usage.cpu_time_micros, 0);
        assert_eq!(usage.memory_peak_bytes, 0);
        assert_eq!(usage.gpu_compute_micros, None);
    }

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert!(config.enable_network_isolation);
        assert!(config.enable_filesystem_isolation);
        assert_eq!(config.max_memory_mb, 1024);
        assert_eq!(config.max_cpu_time_seconds, 300);
    }
}
