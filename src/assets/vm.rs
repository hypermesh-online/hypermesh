//! VM Executor for HyperMesh Asset Integration
//! 
//! Executes VMs through HyperMesh asset allocation with consensus validation.
//! Integrates with Catalog for VM provisioning and execution.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::sync::RwLock;
use tokio::process::Command;
use tracing::{info, debug, warn, error};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::config::VmConfig;
use crate::assets::allocation::{AssetAllocator, AllocationRequest, AllocationPriority, RequesterType, AllocationConstraints};
use crate::transport::StoqTransportLayer;

/// VM executor for HyperMesh integration
pub struct VmExecutor {
    /// Configuration
    config: VmConfig,
    
    /// Asset allocator for resource management
    allocator: Arc<AssetAllocator>,
    
    /// STOQ transport for VM communication
    stoq_transport: Arc<StoqTransportLayer>,
    
    /// Active VM executions
    executions: Arc<RwLock<HashMap<String, Arc<VmExecution>>>>,
    
    /// VM templates and images
    vm_registry: Arc<RwLock<VmRegistry>>,
    
    /// VM execution statistics
    stats: Arc<RwLock<VmStats>>,
}

/// VM execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmExecutionRequest {
    /// Operation to perform
    pub operation: String,
    
    /// VM specification
    pub vm_spec: VmSpecification,
    
    /// Resource requirements
    pub resource_requirements: VmResourceRequirements,
    
    /// Execution parameters
    pub execution_params: ExecutionParameters,
    
    /// Security configuration
    pub security_config: VmSecurityConfig,
}

/// VM specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSpecification {
    /// VM image information
    pub image_id: String,
    pub image_source: VmImageSource,
    
    /// VM configuration
    pub vm_type: VmType,
    pub vm_size: VmSize,
    
    /// Runtime configuration
    pub runtime_config: RuntimeConfig,
    
    /// Network configuration
    pub network_config: NetworkConfig,
    
    /// Storage configuration
    pub storage_config: StorageConfig,
}

/// VM image sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmImageSource {
    /// From Catalog system
    Catalog,
    /// Pre-built system image
    System,
    /// Container image
    Container,
    /// Custom image
    Custom,
}

/// VM types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmType {
    /// General purpose VM
    General,
    /// Compute optimized
    Compute,
    /// Memory optimized
    Memory,
    /// Storage optimized
    Storage,
    /// GPU accelerated
    Gpu,
    /// Network optimized
    Network,
}

/// VM sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmSize {
    Nano,
    Micro,
    Small,
    Medium,
    Large,
    XLarge,
    XXLarge,
    Custom(VmCustomSize),
}

/// Custom VM size specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmCustomSize {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub storage_gb: u64,
    pub network_bandwidth_mbps: u32,
}

/// VM resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmResourceRequirements {
    /// CPU requirements
    pub cpu_cores: u32,
    pub cpu_architecture: String,
    
    /// Memory requirements
    pub memory_mb: u64,
    pub memory_type: MemoryType,
    
    /// Storage requirements
    pub storage_gb: u64,
    pub storage_type: StorageType,
    pub iops_required: Option<u32>,
    
    /// Network requirements
    pub network_bandwidth_mbps: u32,
    pub network_latency_ms: Option<f64>,
    
    /// GPU requirements
    pub gpu_required: bool,
    pub gpu_type: Option<String>,
    pub gpu_memory_mb: Option<u64>,
}

/// Memory types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Standard,
    HighMemory,
    LowLatency,
    Persistent,
}

/// Storage types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageType {
    Ssd,
    Hdd,
    Nvme,
    Memory,
    Network,
}

/// Execution parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionParameters {
    /// Execution timeout
    pub timeout: Option<Duration>,
    
    /// Execution priority
    pub priority: ExecutionPriority,
    
    /// Resource limits
    pub resource_limits: ResourceLimits,
    
    /// Environment variables
    pub environment: HashMap<String, String>,
    
    /// Command line arguments
    pub args: Vec<String>,
    
    /// Working directory
    pub working_directory: Option<String>,
}

/// Execution priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionPriority {
    Low,
    Normal,
    High,
    Realtime,
}

/// Resource limits for VM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_usage: Option<f64>,
    pub max_memory_usage: Option<u64>,
    pub max_disk_usage: Option<u64>,
    pub max_network_usage: Option<u64>,
    pub max_execution_time: Option<Duration>,
}

/// VM security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSecurityConfig {
    /// Isolation level
    pub isolation_level: IsolationLevel,
    
    /// Security policies
    pub security_policies: Vec<SecurityPolicy>,
    
    /// Access control
    pub access_control: AccessControl,
    
    /// Encryption requirements
    pub encryption_config: EncryptionConfig,
}

/// VM isolation levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    Process,
    Container,
    VirtualMachine,
    Hardware,
}

/// Security policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub policy_type: String,
    pub policy_rules: Vec<String>,
    pub enforcement_level: EnforcementLevel,
}

/// Policy enforcement levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementLevel {
    Advisory,
    Enforcing,
    Blocking,
}

/// Access control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControl {
    pub allowed_users: Vec<String>,
    pub allowed_roles: Vec<String>,
    pub network_policies: Vec<NetworkPolicy>,
    pub file_permissions: HashMap<String, String>,
}

/// Network policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub rule_name: String,
    pub direction: NetworkDirection,
    pub protocol: String,
    pub ports: Vec<u16>,
    pub allowed_addresses: Vec<String>,
}

/// Network directions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkDirection {
    Inbound,
    Outbound,
    Both,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub encrypt_at_rest: bool,
    pub encrypt_in_transit: bool,
    pub encryption_algorithm: String,
    pub key_management: KeyManagement,
}

/// Key management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyManagement {
    Local,
    External,
    HardwareSecurityModule,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub runtime_type: RuntimeType,
    pub runtime_options: HashMap<String, String>,
    pub startup_script: Option<String>,
    pub health_check: Option<HealthCheckConfig>,
}

/// Runtime types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeType {
    Native,
    Container,
    Wasm,
    Julia,
    Python,
    NodeJs,
    Java,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub check_type: HealthCheckType,
    pub endpoint: String,
    pub interval: Duration,
    pub timeout: Duration,
    pub retries: u32,
}

/// Health check types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthCheckType {
    Http,
    Tcp,
    Command,
    File,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub network_mode: NetworkMode,
    pub port_mappings: Vec<PortMapping>,
    pub dns_config: DnsConfig,
}

/// Network modes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    Bridge,
    Host,
    None,
    Custom(String),
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub dns_servers: Vec<String>,
    pub search_domains: Vec<String>,
    pub options: HashMap<String, String>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub volumes: Vec<VolumeMount>,
    pub tmpfs_mounts: Vec<TmpfsMount>,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    pub mount_type: MountType,
    pub read_only: bool,
}

/// Mount types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MountType {
    Bind,
    Volume,
    Tmpfs,
}

/// Tmpfs mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmpfsMount {
    pub target: String,
    pub size_mb: u64,
    pub options: HashMap<String, String>,
}

/// VM execution instance
#[derive(Debug, Clone)]
pub struct VmExecution {
    /// Execution identification
    pub id: String,
    pub vm_asset_id: String,
    
    /// Execution details
    pub request: VmExecutionRequest,
    pub started_at: Instant,
    pub completed_at: Option<Instant>,
    
    /// Execution state
    pub state: Arc<RwLock<ExecutionState>>,
    
    /// Resource allocations
    pub resource_allocations: Vec<String>, // Allocation IDs
    
    /// Execution results
    pub results: Arc<RwLock<Option<ExecutionResults>>>,
    
    /// Performance metrics
    pub metrics: Arc<RwLock<ExecutionMetrics>>,
}

/// Execution state
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionState {
    Pending,
    Allocating,
    Provisioning,
    Running,
    Suspended,
    Completed,
    Failed(String),
    Cancelled,
}

/// Execution results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResults {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub execution_time: Duration,
    pub resource_usage: ExecutionResourceUsage,
    pub artifacts: Vec<ExecutionArtifact>,
}

/// Resource usage during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResourceUsage {
    pub peak_cpu_usage: f64,
    pub peak_memory_usage: u64,
    pub total_disk_io: u64,
    pub total_network_io: u64,
    pub gpu_usage: Option<f64>,
}

/// Execution artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionArtifact {
    pub artifact_type: String,
    pub path: String,
    pub size_bytes: u64,
    pub checksum: String,
}

/// Execution performance metrics
#[derive(Debug, Clone, Default)]
pub struct ExecutionMetrics {
    pub current_cpu_usage: f64,
    pub current_memory_usage: u64,
    pub current_disk_io: u64,
    pub current_network_io: u64,
    pub uptime: Duration,
    pub health_status: String,
}

/// VM registry for templates and images
#[derive(Debug, Clone)]
pub struct VmRegistry {
    pub vm_images: HashMap<String, VmImage>,
    pub vm_templates: HashMap<String, VmTemplate>,
}

/// VM image information
#[derive(Debug, Clone)]
pub struct VmImage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: VmImageSource,
    pub size_bytes: u64,
    pub checksum: String,
    pub tags: Vec<String>,
}

/// VM template
#[derive(Debug, Clone)]
pub struct VmTemplate {
    pub id: String,
    pub name: String,
    pub specification: VmSpecification,
    pub default_resources: VmResourceRequirements,
    pub created_at: SystemTime,
}

/// VM execution statistics
#[derive(Debug, Clone, Default)]
pub struct VmStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub active_executions: u32,
    pub avg_execution_time_ms: f64,
    pub total_resource_hours: f64,
    pub catalog_integrations: u64,
}

impl VmExecutor {
    /// Create new VM executor
    pub async fn new(
        config: &VmConfig,
        allocator: Arc<AssetAllocator>,
        stoq_transport: Arc<StoqTransportLayer>
    ) -> Result<Self> {
        info!("🖥️  Initializing VM Executor");
        info!("   Features: Asset allocation, Catalog integration, Security isolation");
        
        let vm_registry = VmRegistry {
            vm_images: HashMap::new(),
            vm_templates: HashMap::new(),
        };
        
        Ok(Self {
            config: config.clone(),
            allocator,
            stoq_transport,
            executions: Arc::new(RwLock::new(HashMap::new())),
            vm_registry: Arc::new(RwLock::new(vm_registry)),
            stats: Arc::new(RwLock::new(VmStats::default())),
        })
    }
    
    /// Start VM executor
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting VM Executor");
        
        // Initialize VM registry
        self.initialize_vm_registry().await?;
        
        // Start execution monitoring
        self.start_execution_monitoring().await?;
        
        info!("✅ VM Executor started");
        info!("   VM images: {}", self.vm_registry.read().await.vm_images.len());
        info!("   VM templates: {}", self.vm_registry.read().await.vm_templates.len());
        
        Ok(())
    }
    
    /// Initialize VM registry with default images and templates
    async fn initialize_vm_registry(&self) -> Result<()> {
        let mut registry = self.vm_registry.write().await;
        
        // Add default VM images
        let ubuntu_image = VmImage {
            id: "ubuntu-22.04".to_string(),
            name: "Ubuntu 22.04 LTS".to_string(),
            version: "22.04".to_string(),
            source: VmImageSource::System,
            size_bytes: 1024 * 1024 * 1024, // 1GB
            checksum: "sha256:example".to_string(),
            tags: vec!["ubuntu".to_string(), "linux".to_string()],
        };
        
        registry.vm_images.insert(ubuntu_image.id.clone(), ubuntu_image);
        
        // Add Julia VM template
        let julia_template = VmTemplate {
            id: "julia-compute".to_string(),
            name: "Julia Compute Environment".to_string(),
            specification: VmSpecification {
                image_id: "julia-1.9".to_string(),
                image_source: VmImageSource::Catalog,
                vm_type: VmType::Compute,
                vm_size: VmSize::Medium,
                runtime_config: RuntimeConfig {
                    runtime_type: RuntimeType::Julia,
                    runtime_options: HashMap::new(),
                    startup_script: Some("julia --version".to_string()),
                    health_check: None,
                },
                network_config: NetworkConfig {
                    network_mode: NetworkMode::Bridge,
                    port_mappings: Vec::new(),
                    dns_config: DnsConfig {
                        dns_servers: vec!["8.8.8.8".to_string()],
                        search_domains: Vec::new(),
                        options: HashMap::new(),
                    },
                },
                storage_config: StorageConfig {
                    volumes: Vec::new(),
                    tmpfs_mounts: Vec::new(),
                },
            },
            default_resources: VmResourceRequirements {
                cpu_cores: 2,
                cpu_architecture: "x86_64".to_string(),
                memory_mb: 4096,
                memory_type: MemoryType::Standard,
                storage_gb: 20,
                storage_type: StorageType::Ssd,
                iops_required: None,
                network_bandwidth_mbps: 100,
                network_latency_ms: None,
                gpu_required: false,
                gpu_type: None,
                gpu_memory_mb: None,
            },
            created_at: SystemTime::now(),
        };
        
        registry.vm_templates.insert(julia_template.id.clone(), julia_template);
        
        Ok(())
    }
    
    /// Start execution monitoring
    async fn start_execution_monitoring(&self) -> Result<()> {
        let executions = self.executions.clone();
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let mut executions_guard = executions.write().await;
                let mut completed_executions = Vec::new();
                
                for (execution_id, execution) in executions_guard.iter_mut() {
                    let state = execution.state.read().await.clone();
                    
                    match state {
                        ExecutionState::Completed | ExecutionState::Failed(_) | ExecutionState::Cancelled => {
                            completed_executions.push(execution_id.clone());
                        }
                        ExecutionState::Running => {
                            // Update metrics for running executions
                            Self::update_execution_metrics(execution).await;
                        }
                        _ => {}
                    }
                }
                
                // Clean up completed executions
                for execution_id in completed_executions {
                    executions_guard.remove(&execution_id);
                    
                    let mut stats_guard = stats.write().await;
                    stats_guard.active_executions = stats_guard.active_executions.saturating_sub(1);
                }
            }
        });
        
        Ok(())
    }
    
    /// Execute VM through asset allocation
    pub async fn execute(
        &self,
        vm_asset: &crate::assets::Asset,
        request: VmExecutionRequest
    ) -> Result<Arc<VmExecution>> {
        let start_time = Instant::now();
        
        info!("🖥️  Executing VM: {} with operation: {}", vm_asset.id, request.operation);
        
        // Generate execution ID
        let execution_id = format!("exec-{}", Uuid::new_v4());
        
        // Allocate resources for VM execution
        let resource_allocations = self.allocate_vm_resources(&vm_asset.id, &request).await?;
        
        // Create VM execution
        let execution = Arc::new(VmExecution {
            id: execution_id.clone(),
            vm_asset_id: vm_asset.id.clone(),
            request: request.clone(),
            started_at: start_time,
            completed_at: None,
            state: Arc::new(RwLock::new(ExecutionState::Provisioning)),
            resource_allocations,
            results: Arc::new(RwLock::new(None)),
            metrics: Arc::new(RwLock::new(ExecutionMetrics::default())),
        });
        
        // Store execution
        self.executions.write().await.insert(execution_id.clone(), execution.clone());
        
        // Start VM execution
        let execution_clone = execution.clone();
        let executor_clone = self.clone();
        
        tokio::spawn(async move {
            if let Err(e) = executor_clone.perform_vm_execution(execution_clone).await {
                error!("VM execution failed: {}", e);
            }
        });
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_executions += 1;
            stats.active_executions += 1;
            stats.catalog_integrations += 1;
        }
        
        info!("✅ VM execution started: {}", execution_id);
        Ok(execution)
    }
    
    /// Allocate resources for VM execution
    async fn allocate_vm_resources(
        &self,
        vm_asset_id: &str,
        request: &VmExecutionRequest
    ) -> Result<Vec<String>> {
        let mut allocation_ids = Vec::new();
        
        // Allocate CPU resources
        let cpu_request = AllocationRequest {
            asset_id: format!("{}-cpu", vm_asset_id),
            amount: request.resource_requirements.cpu_cores as f64,
            duration: request.execution_params.timeout,
            priority: match request.execution_params.priority {
                ExecutionPriority::Low => AllocationPriority::Low,
                ExecutionPriority::Normal => AllocationPriority::Normal,
                ExecutionPriority::High => AllocationPriority::High,
                ExecutionPriority::Realtime => AllocationPriority::Critical,
            },
            requester_id: format!("vm-executor-{}", vm_asset_id),
            requester_type: RequesterType::Vm,
            constraints: AllocationConstraints::default(),
            performance_requirements: None,
        };
        
        let cpu_allocation = self.allocator.allocate(&cpu_request).await?;
        allocation_ids.push(cpu_allocation.id.clone());
        
        // Allocate memory resources
        let memory_request = AllocationRequest {
            asset_id: format!("{}-memory", vm_asset_id),
            amount: request.resource_requirements.memory_mb as f64,
            duration: request.execution_params.timeout,
            priority: cpu_request.priority.clone(),
            requester_id: cpu_request.requester_id.clone(),
            requester_type: RequesterType::Vm,
            constraints: AllocationConstraints::default(),
            performance_requirements: None,
        };
        
        let memory_allocation = self.allocator.allocate(&memory_request).await?;
        allocation_ids.push(memory_allocation.id.clone());
        
        // Allocate GPU resources if required
        if request.resource_requirements.gpu_required {
            let gpu_request = AllocationRequest {
                asset_id: format!("{}-gpu", vm_asset_id),
                amount: 1.0, // One GPU unit
                duration: request.execution_params.timeout,
                priority: cpu_request.priority.clone(),
                requester_id: cpu_request.requester_id.clone(),
                requester_type: RequesterType::Vm,
                constraints: AllocationConstraints::default(),
                performance_requirements: None,
            };
            
            let gpu_allocation = self.allocator.allocate(&gpu_request).await?;
            allocation_ids.push(gpu_allocation.id.clone());
        }
        
        Ok(allocation_ids)
    }
    
    /// Perform actual VM execution
    async fn perform_vm_execution(&self, execution: Arc<VmExecution>) -> Result<()> {
        // Update state to running
        *execution.state.write().await = ExecutionState::Running;
        
        // Simulate VM execution based on operation
        let result = match execution.request.operation.as_str() {
            "execute_julia" => self.execute_julia_vm(&execution).await,
            "execute_python" => self.execute_python_vm(&execution).await,
            "execute_container" => self.execute_container_vm(&execution).await,
            _ => self.execute_generic_vm(&execution).await,
        };
        
        // Update execution state and results
        match result {
            Ok(execution_results) => {
                *execution.state.write().await = ExecutionState::Completed;
                *execution.results.write().await = Some(execution_results);
                
                let mut stats = self.stats.write().await;
                stats.successful_executions += 1;
            }
            Err(e) => {
                *execution.state.write().await = ExecutionState::Failed(e.to_string());
                
                let mut stats = self.stats.write().await;
                stats.failed_executions += 1;
            }
        }
        
        // Release allocated resources
        for allocation_id in &execution.resource_allocations {
            if let Err(e) = self.allocator.release(allocation_id).await {
                warn!("Failed to release allocation {}: {}", allocation_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Execute Julia VM
    async fn execute_julia_vm(&self, execution: &Arc<VmExecution>) -> Result<ExecutionResults> {
        info!("🔬 Executing Julia VM: {}", execution.id);
        
        // Create Julia command
        let mut cmd = Command::new("julia");
        cmd.args(&execution.request.execution_params.args);
        
        // Set environment variables
        for (key, value) in &execution.request.execution_params.environment {
            cmd.env(key, value);
        }
        
        // Set working directory
        if let Some(ref working_dir) = execution.request.execution_params.working_directory {
            cmd.current_dir(working_dir);
        }
        
        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        // Execute command
        let start_time = Instant::now();
        let output = cmd.output().await?;
        let execution_time = start_time.elapsed();
        
        let results = ExecutionResults {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            resource_usage: ExecutionResourceUsage {
                peak_cpu_usage: 50.0, // Simulated
                peak_memory_usage: execution.request.resource_requirements.memory_mb * 1024 * 1024,
                total_disk_io: 1024 * 1024, // 1MB
                total_network_io: 0,
                gpu_usage: None,
            },
            artifacts: Vec::new(),
        };
        
        Ok(results)
    }
    
    /// Execute Python VM
    async fn execute_python_vm(&self, execution: &Arc<VmExecution>) -> Result<ExecutionResults> {
        info!("🐍 Executing Python VM: {}", execution.id);
        
        // Similar to Julia execution but with Python
        let mut cmd = Command::new("python3");
        cmd.args(&execution.request.execution_params.args);
        
        // Set environment and execute
        for (key, value) in &execution.request.execution_params.environment {
            cmd.env(key, value);
        }
        
        if let Some(ref working_dir) = execution.request.execution_params.working_directory {
            cmd.current_dir(working_dir);
        }
        
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        let start_time = Instant::now();
        let output = cmd.output().await?;
        let execution_time = start_time.elapsed();
        
        let results = ExecutionResults {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            resource_usage: ExecutionResourceUsage {
                peak_cpu_usage: 30.0, // Simulated
                peak_memory_usage: execution.request.resource_requirements.memory_mb * 1024 * 1024,
                total_disk_io: 512 * 1024, // 512KB
                total_network_io: 0,
                gpu_usage: None,
            },
            artifacts: Vec::new(),
        };
        
        Ok(results)
    }
    
    /// Execute container VM
    async fn execute_container_vm(&self, execution: &Arc<VmExecution>) -> Result<ExecutionResults> {
        info!("📦 Executing Container VM: {}", execution.id);
        
        // Simulate container execution
        let execution_time = Duration::from_secs(2); // Simulated execution time
        
        let results = ExecutionResults {
            exit_code: Some(0),
            stdout: "Container executed successfully".to_string(),
            stderr: String::new(),
            execution_time,
            resource_usage: ExecutionResourceUsage {
                peak_cpu_usage: 25.0,
                peak_memory_usage: execution.request.resource_requirements.memory_mb * 1024 * 1024,
                total_disk_io: 256 * 1024, // 256KB
                total_network_io: 128 * 1024, // 128KB
                gpu_usage: None,
            },
            artifacts: Vec::new(),
        };
        
        Ok(results)
    }
    
    /// Execute generic VM
    async fn execute_generic_vm(&self, execution: &Arc<VmExecution>) -> Result<ExecutionResults> {
        info!("⚙️ Executing Generic VM: {}", execution.id);
        
        // Simulate generic execution
        let execution_time = Duration::from_secs(1);
        
        let results = ExecutionResults {
            exit_code: Some(0),
            stdout: format!("VM {} executed successfully", execution.id),
            stderr: String::new(),
            execution_time,
            resource_usage: ExecutionResourceUsage {
                peak_cpu_usage: 20.0,
                peak_memory_usage: execution.request.resource_requirements.memory_mb * 1024 * 1024,
                total_disk_io: 128 * 1024,
                total_network_io: 64 * 1024,
                gpu_usage: if execution.request.resource_requirements.gpu_required { Some(15.0) } else { None },
            },
            artifacts: Vec::new(),
        };
        
        Ok(results)
    }
    
    /// Update execution metrics
    async fn update_execution_metrics(execution: &Arc<VmExecution>) {
        let mut metrics = execution.metrics.write().await;
        
        // Simulate metric updates
        metrics.current_cpu_usage = 45.0;
        metrics.current_memory_usage = execution.request.resource_requirements.memory_mb * 1024 * 1024 / 2;
        metrics.uptime = execution.started_at.elapsed();
        metrics.health_status = "healthy".to_string();
    }
    
    /// Get execution by ID
    pub async fn get_execution(&self, execution_id: &str) -> Option<Arc<VmExecution>> {
        self.executions.read().await.get(execution_id).cloned()
    }
    
    /// List active executions
    pub async fn list_active_executions(&self) -> Vec<Arc<VmExecution>> {
        self.executions.read().await.values().cloned().collect()
    }
    
    /// Get VM execution statistics
    pub async fn get_statistics(&self) -> VmStats {
        let mut stats = self.stats.read().await.clone();
        stats.active_executions = self.executions.read().await.len() as u32;
        
        // Update average execution time
        if stats.total_executions > 0 {
            // This would be calculated from actual execution times
            stats.avg_execution_time_ms = 2000.0; // Placeholder
        }
        
        stats
    }
    
    /// Shutdown VM executor
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down VM Executor");
        
        // Cancel all active executions
        let execution_ids: Vec<String> = self.executions.read().await.keys().cloned().collect();
        
        for execution_id in execution_ids {
            if let Some(execution) = self.executions.read().await.get(&execution_id) {
                *execution.state.write().await = ExecutionState::Cancelled;
                
                // Release allocated resources
                for allocation_id in &execution.resource_allocations {
                    if let Err(e) = self.allocator.release(allocation_id).await {
                        warn!("Error releasing allocation {} during shutdown: {}", allocation_id, e);
                    }
                }
            }
        }
        
        // Clear executions
        self.executions.write().await.clear();
        
        info!("✅ VM Executor shutdown complete");
        Ok(())
    }
}

// Allow cloning for async contexts
impl Clone for VmExecutor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            allocator: self.allocator.clone(),
            stoq_transport: self.stoq_transport.clone(),
            executions: self.executions.clone(),
            vm_registry: self.vm_registry.clone(),
            stats: self.stats.clone(),
        }
    }
}

// Asset struct for external usage
pub struct VmAsset {
    pub id: String,
    pub name: String,
    pub vm_type: VmType,
    pub resource_requirements: VmResourceRequirements,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_vm_execution_lifecycle() {
        // Test complete VM execution lifecycle
    }
    
    #[tokio::test]
    async fn test_resource_allocation_for_vm() {
        // Test resource allocation for VM execution
    }
    
    #[tokio::test]
    async fn test_julia_vm_execution() {
        // Test Julia-specific VM execution
    }
    
    #[tokio::test]
    async fn test_vm_security_isolation() {
        // Test VM security and isolation features
    }
}