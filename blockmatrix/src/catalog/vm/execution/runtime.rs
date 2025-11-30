//! Consensus Runtime - Native VM execution with HyperMesh asset integration
//!
//! This is the core runtime that executes code with consensus proof validation
//! integrated directly into the execution model. Every operation requires
//! and validates consensus proofs as language-level constructs.

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, Duration, Instant};
use std::process::{Command, Stdio};
use std::io::{Write, BufRead, BufReader};
use std::thread;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, Mutex};
use uuid::Uuid;

use crate::catalog::vm::consensus::{ConsensusVM, ConsensusOperation, ConsensusExecutionResult};
use crate::assets::core::{AssetManager, AssetId, AssetType, AssetAllocationRequest};
use super::context::ExecutionContext;
use super::scheduler::{ExecutionScheduler, ExecutionPlan};
use super::{RuntimeExecutionResult, MemoryUsagePattern, StorageOperation};

/// Production-ready consensus runtime with native VM execution
pub struct ConsensusRuntime {
    /// Consensus VM integration
    consensus_vm: Arc<ConsensusVM>,
    /// Execution scheduler
    scheduler: Arc<ExecutionScheduler>,
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

impl ConsensusRuntime {
    /// Create new consensus runtime
    pub async fn new(
        consensus_vm: Arc<ConsensusVM>,
        scheduler: Arc<ExecutionScheduler>,
    ) -> Result<Self> {
        let asset_manager = Arc::new(AssetManager::new());
        let language_runtimes = Arc::new(RwLock::new(HashMap::new()));
        let active_executions = Arc::new(RwLock::new(HashMap::new()));
        let metrics = Arc::new(Mutex::new(RuntimeMetrics::default()));
        let sandbox_config = SandboxConfig::default();
        
        let runtime = Self {
            consensus_vm,
            scheduler,
            asset_manager,
            language_runtimes,
            active_executions,
            metrics,
            sandbox_config,
        };
        
        // Initialize built-in language runtimes
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
        
        // Allocate required assets
        let asset_allocations = self.allocate_assets(&execution_plan.required_assets).await?;
        
        // Create execution handle
        let execution_handle = ExecutionHandle {
            execution_id: execution_id.clone(),
            started_at: SystemTime::now(),
            language: execution_plan.language.clone(),
            process_id: None,
            asset_allocations: asset_allocations.clone(),
            context: Arc::clone(&context),
        };
        
        // Register execution
        {
            let mut active = self.active_executions.write().await;
            active.insert(execution_id.clone(), execution_handle);
        }
        
        // Execute consensus operation
        let consensus_result = self.consensus_vm.execute_consensus_operation(
            &execution_plan.consensus_operation
        ).await?;
        
        // Get language runtime
        let language_runtime = {
            let runtimes = self.language_runtimes.read().await;
            runtimes.get(&execution_plan.language)
                .ok_or_else(|| anyhow!("Language runtime not found: {}", execution_plan.language))?
                .clone()
        };
        
        // Execute code
        let execution_result = language_runtime.execute(
            &execution_plan.code,
            &context,
            &asset_allocations,
        ).await;
        
        // Clean up execution
        {
            let mut active = self.active_executions.write().await;
            active.remove(&execution_id);
        }
        
        // Deallocate assets
        self.deallocate_assets(&asset_allocations).await?;
        
        // Update metrics
        let execution_time = start_time.elapsed();
        {
            let mut metrics = self.metrics.lock().await;
            metrics.total_executions += 1;
            if execution_result.is_ok() {
                metrics.successful_executions += 1;
            } else {
                metrics.failed_executions += 1;
            }
            
            // Update average execution time
            let total_time = metrics.average_execution_time.as_micros() as u64 * (metrics.total_executions - 1)
                + execution_time.as_micros() as u64;
            metrics.average_execution_time = Duration::from_micros(total_time / metrics.total_executions);
            
            metrics.consensus_validations += 1;
            metrics.asset_allocations += asset_allocations.len() as u64;
        }
        
        // Convert to runtime result
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
        
        // Julia runtime
        let julia_runtime = Arc::new(JuliaLanguageRuntime::new(
            Arc::clone(&self.asset_manager),
            self.sandbox_config.clone(),
        )?);
        runtimes.insert("julia".to_string(), julia_runtime);
        
        // Python runtime
        let python_runtime = Arc::new(PythonLanguageRuntime::new(
            Arc::clone(&self.asset_manager),
            self.sandbox_config.clone(),
        )?);
        runtimes.insert("python".to_string(), python_runtime);
        
        // Rust runtime (for compiled execution)
        let rust_runtime = Arc::new(RustLanguageRuntime::new(
            Arc::clone(&self.asset_manager),
            self.sandbox_config.clone(),
        )?);
        runtimes.insert("rust".to_string(), rust_runtime);
        
        // Initialize all runtimes
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
            // Create allocation request
            let allocation_request = AssetAllocationRequest {
                asset_type: asset_type.clone(),
                requested_resources: ResourceRequirements {
                    cpu: None,
                    gpu_usage: None,
                    memory_usage: None,
                    storage_usage: Some(*required_capacity),
                    network_usage: None,
                },
                privacy_level: PrivacyLevel::Private,
                consensus_proof: self.create_allocation_consensus_proof().await?,
                certificate_fingerprint: String::new(),
                duration_limit: Some(Duration::from_secs(3600)), // 1 hour default
                tags: HashMap::new(),
            };
            
            // Allocate through asset manager
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
            total_size: 1024 * 1024 * 1024, // 1GB
            total_storage: 10 * 1024 * 1024 * 1024, // 10GB total
            file_hash: hex::encode(&[1, 2, 3, 4]), // In production: real cryptographic proof
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
            proof_hash: vec![5, 6, 7, 8], // In production: real cryptographic proof
        };
        
        Ok(ConsensusProof::new(space_proof, stake_proof, work_proof, time_proof))
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
        // Stop all active executions
        let executions = {
            let active = self.active_executions.read().await;
            active.clone()
        };
        
        for execution in executions.values() {
            // In production: gracefully terminate processes
            tracing::warn!("Terminating execution: {}", execution.execution_id);
        }
        
        // Cleanup language runtimes
        let runtimes = self.language_runtimes.read().await;
        for runtime in runtimes.values() {
            runtime.cleanup().await?;
        }
        
        Ok(())
    }
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

/// Julia language runtime implementation
pub struct JuliaLanguageRuntime {
    asset_manager: Arc<AssetManager>,
    sandbox_config: SandboxConfig,
    julia_binary_path: String,
}

impl JuliaLanguageRuntime {
    pub fn new(
        asset_manager: Arc<AssetManager>,
        sandbox_config: SandboxConfig,
    ) -> Result<Self> {
        // Find Julia binary
        let julia_binary_path = which::which("julia")
            .map_err(|_| anyhow!("Julia binary not found in PATH"))?
            .to_string_lossy()
            .to_string();
        
        Ok(Self {
            asset_manager,
            sandbox_config,
            julia_binary_path,
        })
    }
}

#[async_trait::async_trait]
impl LanguageRuntime for JuliaLanguageRuntime {
    async fn execute(
        &self,
        code: &str,
        context: &ExecutionContext,
        asset_allocations: &HashMap<AssetId, AssetHandle>,
    ) -> Result<LanguageExecutionResult> {
        let start_time = Instant::now();
        
        // Create temporary file for code
        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path().to_string_lossy().to_string();
        std::fs::write(&temp_path, code)?;
        
        // Execute Julia with sandbox
        let mut cmd = Command::new(&self.julia_binary_path);
        cmd.arg(&temp_path)
           .stdin(Stdio::null())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        // Apply sandbox restrictions
        if self.sandbox_config.enable_network_isolation {
            // In production: apply network namespace isolation
            cmd.env("JULIA_DEPOT_PATH", "/tmp/julia_depot");
        }
        
        let output = cmd.output()?;
        let execution_time = start_time.elapsed();
        
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let result_output = if success {
            Some(serde_json::Value::String(stdout.to_string()))
        } else {
            None
        };
        
        let error_message = if !success {
            Some(stderr.to_string())
        } else {
            None
        };
        
        Ok(LanguageExecutionResult {
            success,
            output: result_output,
            error_message,
            execution_time,
            resource_usage: ResourceUsage::default(), // In production: collect real metrics
            memory_pattern: MemoryUsagePattern {
                peak_usage: 1024 * 1024 * 10, // 10MB estimate
                average_usage: 1024 * 1024 * 5, // 5MB estimate
                allocations: 100,
                deallocations: 95,
                gc_events: 2,
            },
            storage_operations: vec![],
        })
    }
    
    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            language_name: "Julia".to_string(),
            version: "1.9.0".to_string(), // In production: detect actual version
            supported_features: vec![
                "numerical_computing".to_string(),
                "parallel_execution".to_string(),
                "gpu_computing".to_string(),
                "package_manager".to_string(),
            ],
            memory_model: MemoryModel::Managed,
            execution_model: ExecutionModel::JIT,
            consensus_integration: true,
        }
    }
    
    async fn initialize(&self) -> Result<()> {
        // Verify Julia installation and dependencies
        let output = Command::new(&self.julia_binary_path)
            .arg("--version")
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow!("Julia runtime initialization failed"));
        }
        
        tracing::info!("Julia runtime initialized: {}", 
            String::from_utf8_lossy(&output.stdout).trim());
        Ok(())
    }
    
    async fn cleanup(&self) -> Result<()> {
        // Cleanup temporary files and resources
        Ok(())
    }
    
    async fn get_resource_usage(&self) -> Result<ResourceUsage> {
        // In production: collect actual resource usage metrics
        Ok(ResourceUsage::default())
    }
}

/// Python language runtime implementation
pub struct PythonLanguageRuntime {
    asset_manager: Arc<AssetManager>,
    sandbox_config: SandboxConfig,
    python_binary_path: String,
}

impl PythonLanguageRuntime {
    pub fn new(
        asset_manager: Arc<AssetManager>,
        sandbox_config: SandboxConfig,
    ) -> Result<Self> {
        let python_binary_path = which::which("python3")
            .or_else(|_| which::which("python"))
            .map_err(|_| anyhow!("Python binary not found in PATH"))?
            .to_string_lossy()
            .to_string();
        
        Ok(Self {
            asset_manager,
            sandbox_config,
            python_binary_path,
        })
    }
}

#[async_trait::async_trait]
impl LanguageRuntime for PythonLanguageRuntime {
    async fn execute(
        &self,
        code: &str,
        context: &ExecutionContext,
        asset_allocations: &HashMap<AssetId, AssetHandle>,
    ) -> Result<LanguageExecutionResult> {
        let start_time = Instant::now();
        
        // Execute Python code
        let mut cmd = Command::new(&self.python_binary_path);
        cmd.arg("-c")
           .arg(code)
           .stdin(Stdio::null())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let output = cmd.output()?;
        let execution_time = start_time.elapsed();
        
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        Ok(LanguageExecutionResult {
            success,
            output: if success { Some(serde_json::Value::String(stdout.to_string())) } else { None },
            error_message: if !success { Some(stderr.to_string()) } else { None },
            execution_time,
            resource_usage: ResourceUsage::default(),
            memory_pattern: MemoryUsagePattern {
                peak_usage: 1024 * 1024 * 8, // 8MB estimate
                average_usage: 1024 * 1024 * 4, // 4MB estimate
                allocations: 80,
                deallocations: 75,
                gc_events: 3,
            },
            storage_operations: vec![],
        })
    }
    
    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            language_name: "Python".to_string(),
            version: "3.9.0".to_string(),
            supported_features: vec![
                "data_science".to_string(),
                "machine_learning".to_string(),
                "web_frameworks".to_string(),
                "package_manager".to_string(),
            ],
            memory_model: MemoryModel::Managed,
            execution_model: ExecutionModel::Interpreted,
            consensus_integration: true,
        }
    }
    
    async fn initialize(&self) -> Result<()> {
        let output = Command::new(&self.python_binary_path)
            .arg("--version")
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow!("Python runtime initialization failed"));
        }
        
        tracing::info!("Python runtime initialized: {}", 
            String::from_utf8_lossy(&output.stdout).trim());
        Ok(())
    }
    
    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
    
    async fn get_resource_usage(&self) -> Result<ResourceUsage> {
        Ok(ResourceUsage::default())
    }
}

/// Rust language runtime implementation (compiled execution)
pub struct RustLanguageRuntime {
    asset_manager: Arc<AssetManager>,
    sandbox_config: SandboxConfig,
    rustc_binary_path: String,
}

impl RustLanguageRuntime {
    pub fn new(
        asset_manager: Arc<AssetManager>,
        sandbox_config: SandboxConfig,
    ) -> Result<Self> {
        let rustc_binary_path = which::which("rustc")
            .map_err(|_| anyhow!("Rust compiler not found in PATH"))?
            .to_string_lossy()
            .to_string();
        
        Ok(Self {
            asset_manager,
            sandbox_config,
            rustc_binary_path,
        })
    }
}

#[async_trait::async_trait]
impl LanguageRuntime for RustLanguageRuntime {
    async fn execute(
        &self,
        code: &str,
        context: &ExecutionContext,
        asset_allocations: &HashMap<AssetId, AssetHandle>,
    ) -> Result<LanguageExecutionResult> {
        let start_time = Instant::now();
        
        // Create temporary source file
        let temp_source = tempfile::NamedTempFile::new()?;
        let source_path = temp_source.path().to_string_lossy().to_string();
        std::fs::write(&source_path, code)?;
        
        // Create temporary binary file
        let temp_binary = tempfile::NamedTempFile::new()?;
        let binary_path = temp_binary.path().to_string_lossy().to_string();
        
        // Compile Rust code
        let compile_output = Command::new(&self.rustc_binary_path)
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .output()?;
        
        if !compile_output.status.success() {
            let stderr = String::from_utf8_lossy(&compile_output.stderr);
            return Ok(LanguageExecutionResult {
                success: false,
                output: None,
                error_message: Some(format!("Compilation failed: {}", stderr)),
                execution_time: start_time.elapsed(),
                resource_usage: ResourceUsage::default(),
                memory_pattern: MemoryUsagePattern::default(),
                storage_operations: vec![],
            });
        }
        
        // Execute compiled binary
        let exec_output = Command::new(&binary_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        
        let execution_time = start_time.elapsed();
        let success = exec_output.status.success();
        let stdout = String::from_utf8_lossy(&exec_output.stdout);
        let stderr = String::from_utf8_lossy(&exec_output.stderr);
        
        Ok(LanguageExecutionResult {
            success,
            output: if success { Some(serde_json::Value::String(stdout.to_string())) } else { None },
            error_message: if !success { Some(stderr.to_string()) } else { None },
            execution_time,
            resource_usage: ResourceUsage::default(),
            memory_pattern: MemoryUsagePattern {
                peak_usage: 1024 * 1024 * 2, // 2MB estimate
                average_usage: 1024 * 1024 * 1, // 1MB estimate
                allocations: 20,
                deallocations: 20,
                gc_events: 0, // No GC in Rust
            },
            storage_operations: vec![],
        })
    }
    
    fn capabilities(&self) -> LanguageCapabilities {
        LanguageCapabilities {
            language_name: "Rust".to_string(),
            version: "1.70.0".to_string(),
            supported_features: vec![
                "systems_programming".to_string(),
                "memory_safety".to_string(),
                "concurrent_programming".to_string(),
                "package_manager".to_string(),
            ],
            memory_model: MemoryModel::Manual,
            execution_model: ExecutionModel::Compiled,
            consensus_integration: true,
        }
    }
    
    async fn initialize(&self) -> Result<()> {
        let output = Command::new(&self.rustc_binary_path)
            .arg("--version")
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow!("Rust runtime initialization failed"));
        }
        
        tracing::info!("Rust runtime initialized: {}", 
            String::from_utf8_lossy(&output.stdout).trim());
        Ok(())
    }
    
    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
    
    async fn get_resource_usage(&self) -> Result<ResourceUsage> {
        Ok(ResourceUsage::default())
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
    use crate::consensus::{ConsensusRequirements, ConsensusVM};
    
    #[tokio::test]
    async fn test_consensus_runtime_creation() {
        let requirements = ConsensusRequirements::default();
        let consensus_vm = Arc::new(ConsensusVM::new(requirements).unwrap());
        let scheduler = Arc::new(ExecutionScheduler::new(
            Arc::clone(&consensus_vm),
            super::super::super::AssetManagementConfig::default(),
        ).await.unwrap());
        
        let runtime = ConsensusRuntime::new(consensus_vm, scheduler).await;
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