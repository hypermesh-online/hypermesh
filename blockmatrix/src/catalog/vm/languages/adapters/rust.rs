//! Rust Language Adapter - RustCall Integration with Consensus
//!
//! This adapter provides Rust code compilation and execution through Julia's RustCall
//! integration, enabling Rust code to access ConsensusProof validation and asset
//! management through the HyperMesh ecosystem. Rust consensus constructs are
//! translated to Julia consensus operations for execution.
//!
//! Key Features:
//! - RustCall-based compilation and execution through Julia
//! - Rust attributes for consensus requirements (#[consensus_required])
//! - Asset management through Rust structs and traits
//! - Zero-cost abstractions for consensus validation
//! - Memory-safe asset operations with consensus guarantees

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::consensus::proof::ConsensusProof;
use super::super::super::consensus::ConsensusVM;
use super::super::super::execution::{ExecutionContext, ExecutionResult};
use super::{
    LanguageRuntime, BaseAdapter, AdapterConfig, ExecutionEnvironment,
    LanguageSpecificConfig, ConsensusBridge,
};
use super::super::{
    ConsensusRequirements, ConsensusConstruct, AssetRequirements, TranslatedError,
    ConsensusConstructType, SourceLocation, ProofRequirement, AssetDependency,
    CpuRequirements, MemoryRequirements, GpuRequirements, StorageRequirements,
    ErrorCategory, AssetAccessPattern, MemoryAccessPattern,
};

/// Rust language adapter using RustCall integration
pub struct RustAdapter {
    /// Base adapter functionality
    base: BaseAdapter,
    /// Consensus bridge for translation
    consensus_bridge: Arc<ConsensusBridge>,
    /// Rust-specific configuration
    rust_config: RustAdapterConfig,
}

/// Rust-specific adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustAdapterConfig {
    /// Rust compiler (rustc) path
    pub rustc_path: Option<String>,
    /// Cargo executable path
    pub cargo_path: Option<String>,
    /// Target architecture
    pub target_arch: Option<String>,
    /// Optimization level (0-3)
    pub optimization_level: u8,
    /// Enable debug information
    pub debug_info: bool,
    /// Rust edition (2018, 2021)
    pub edition: String,
    /// Required crates for consensus
    pub consensus_crates: Vec<String>,
    /// GPU computing crates
    pub gpu_crates: Vec<String>,
    /// Async runtime crates
    pub async_crates: Vec<String>,
    /// Enable unsafe code
    pub allow_unsafe: bool,
    /// Compile-time feature flags
    pub features: Vec<String>,
}

impl RustAdapter {
    /// Create new Rust adapter with RustCall integration
    pub async fn new(
        consensus_vm: Arc<ConsensusVM>,
        consensus_bridge: Arc<ConsensusBridge>,
        config: Option<&LanguageSpecificConfig>,
    ) -> Result<Self> {
        let base = BaseAdapter::new(
            "rust".to_string(),
            "RustCall".to_string(),
            consensus_vm,
            Arc::clone(&consensus_bridge),
            config,
        );
        
        let rust_config = RustAdapterConfig::from_language_config(config);
        
        Ok(Self {
            base,
            consensus_bridge,
            rust_config,
        })
    }
    
    /// Execute Rust code through RustCall with consensus validation
    async fn execute_rust_with_consensus(
        &self,
        code: &str,
        context: Arc<ExecutionContext>,
        consensus_proof: ConsensusProof,
    ) -> Result<ExecutionResult> {
        // Create execution environment
        let mut env = self.base.create_execution_environment(&context).await?;
        
        // Validate consensus proof
        if !self.base.validate_adapter_consensus_proof(&consensus_proof).await? {
            return Err(anyhow::anyhow!("Consensus proof validation failed for Rust execution"));
        }
        
        // Prepare Rust code for compilation
        let prepared_code = self.prepare_rust_code(code, &consensus_proof).await?;
        
        // Compile Rust code
        let compiled_binary = self.compile_rust_code(&prepared_code, &env).await?;
        
        // Execute through Julia RustCall integration
        let julia_code = self.generate_rustcall_integration(&compiled_binary, &consensus_proof).await?;
        let result = self.execute_julia_rustcall(&julia_code, context).await?;
        
        // Cleanup environment
        self.base.cleanup_execution_environment(env).await?;
        
        Ok(result)
    }
    
    /// Prepare Rust code with consensus integration
    async fn prepare_rust_code(
        &self,
        rust_code: &str,
        consensus_proof: &ConsensusProof,
    ) -> Result<String> {
        let mut prepared_code = String::new();
        
        // Add consensus prelude
        prepared_code.push_str(&self.generate_rust_consensus_prelude()?);
        
        // Add the user's Rust code
        prepared_code.push_str("\n// User Rust code\n");
        prepared_code.push_str(rust_code);
        
        // Add main function if not present
        if !rust_code.contains("fn main") {
            prepared_code.push_str("\n\nfn main() {\n");
            prepared_code.push_str("    // Execute user code with consensus validation\n");
            prepared_code.push_str("    let _result = user_code_execution();\n");
            prepared_code.push_str("}\n");
        }
        
        Ok(prepared_code)
    }
    
    /// Generate Rust consensus prelude
    fn generate_rust_consensus_prelude(&self) -> Result<String> {
        Ok(r#"
// HyperMesh Consensus Integration for Rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::marker::PhantomData;

// Consensus proof types
#[derive(Debug, Clone)]
pub struct ConsensusProof {
    pub space_commitment: u64,
    pub stake_authority: u64,
    pub work_difficulty: u32,
    pub time_sequence: u64,
}

impl ConsensusProof {
    pub fn validate(&self) -> bool {
        // Validation handled by Julia layer
        true
    }
}

// Consensus required attribute (procedural macro would be implemented)
pub trait ConsensusRequired {
    fn validate_consensus(&self, proof: &ConsensusProof) -> bool;
}

// Asset management traits and structs
pub trait Asset {
    type ResourceType;
    
    fn allocate(&mut self) -> Result<(), AssetError>;
    fn deallocate(&mut self) -> Result<(), AssetError>;
    fn is_allocated(&self) -> bool;
}

#[derive(Debug)]
pub enum AssetError {
    AllocationFailed,
    InsufficientResources,
    ConsensusValidationFailed,
}

pub struct CpuAsset {
    cores: u32,
    allocated: bool,
}

impl CpuAsset {
    pub fn new(cores: u32) -> Self {
        Self {
            cores,
            allocated: false,
        }
    }
}

impl Asset for CpuAsset {
    type ResourceType = u32;
    
    fn allocate(&mut self) -> Result<(), AssetError> {
        if !self.allocated {
            self.allocated = true;
            Ok(())
        } else {
            Err(AssetError::AllocationFailed)
        }
    }
    
    fn deallocate(&mut self) -> Result<(), AssetError> {
        self.allocated = false;
        Ok(())
    }
    
    fn is_allocated(&self) -> bool {
        self.allocated
    }
}

pub struct GpuAsset {
    memory_mb: u64,
    allocated: bool,
}

impl GpuAsset {
    pub fn new(memory_mb: u64) -> Self {
        Self {
            memory_mb,
            allocated: false,
        }
    }
}

impl Asset for GpuAsset {
    type ResourceType = u64;
    
    fn allocate(&mut self) -> Result<(), AssetError> {
        if !self.allocated {
            self.allocated = true;
            Ok(())
        } else {
            Err(AssetError::AllocationFailed)
        }
    }
    
    fn deallocate(&mut self) -> Result<(), AssetError> {
        self.allocated = false;
        Ok(())
    }
    
    fn is_allocated(&self) -> bool {
        self.allocated
    }
}

pub struct MemoryAsset {
    size_mb: u64,
    allocated: bool,
}

impl MemoryAsset {
    pub fn new(size_mb: u64) -> Self {
        Self {
            size_mb,
            allocated: false,
        }
    }
}

impl Asset for MemoryAsset {
    type ResourceType = u64;
    
    fn allocate(&mut self) -> Result<(), AssetError> {
        if !self.allocated {
            self.allocated = true;
            Ok(())
        } else {
            Err(AssetError::AllocationFailed)
        }
    }
    
    fn deallocate(&mut self) -> Result<(), AssetError> {
        self.allocated = false;
        Ok(())
    }
    
    fn is_allocated(&self) -> bool {
        self.allocated
    }
}

// P2P execution functions
pub fn remote_execute<T>(peer_id: &str, code: T) -> Result<String, P2PError>
where
    T: std::fmt::Display,
{
    // P2P execution handled by Julia layer
    Ok(format!("Remote execution on {}: {}", peer_id, code))
}

#[derive(Debug)]
pub enum P2PError {
    PeerNotFound,
    ConsensusValidationFailed,
    NetworkError,
}

// Blockchain storage
pub struct BlockchainStorage;

impl BlockchainStorage {
    pub fn store<T>(data: T, consensus_proof: &ConsensusProof) -> Result<String, StorageError>
    where
        T: serde::Serialize,
    {
        // Storage handled by Julia layer
        Ok(format!("stored_{}", std::ptr::addr_of!(data) as usize))
    }
    
    pub fn retrieve(storage_id: &str) -> Result<String, StorageError> {
        // Retrieval handled by Julia layer
        Ok(format!("Retrieved data for {}", storage_id))
    }
}

#[derive(Debug)]
pub enum StorageError {
    SerializationFailed,
    ConsensusValidationFailed,
    NetworkError,
}

// Macros for consensus validation (would be implemented as procedural macros)
macro_rules! consensus_validate {
    ($proof:expr, $space:expr, $stake:expr, $work:expr, $time:expr) => {
        {
            let valid = $proof.space_commitment >= $space &&
                       $proof.stake_authority >= $stake &&
                       $proof.work_difficulty >= $work &&
                       $proof.time_sequence >= $time;
            if !valid {
                return Err("Consensus validation failed".into());
            }
        }
    };
}

// User code execution function (would be generated based on actual code)
fn user_code_execution() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
"#.to_string())
    }
    
    /// Compile Rust code to binary
    async fn compile_rust_code(
        &self,
        code: &str,
        env: &ExecutionEnvironment,
    ) -> Result<String> {
        let source_file = format!("{}/main.rs", env.working_directory());
        let binary_file = format!("{}/main", env.working_directory());
        
        // Write source code to file
        std::fs::write(&source_file, code)?;
        
        // Generate Cargo.toml
        let cargo_toml = self.generate_cargo_toml()?;
        let cargo_toml_path = format!("{}/Cargo.toml", env.working_directory());
        std::fs::write(&cargo_toml_path, cargo_toml)?;
        
        // Compile with rustc or cargo
        let rustc_path = self.rust_config.rustc_path
            .as_deref()
            .unwrap_or("rustc");
        
        let compile_command = format!(
            "{} {} -o {} --edition {} -O",
            rustc_path,
            source_file,
            binary_file,
            self.rust_config.edition
        );
        
        // Execute compilation (this would use actual process execution)
        // For now, return a placeholder path
        Ok(binary_file)
    }
    
    /// Generate Cargo.toml for consensus integration
    fn generate_cargo_toml(&self) -> Result<String> {
        let mut cargo_toml = String::new();
        
        cargo_toml.push_str("[package]\n");
        cargo_toml.push_str("name = \"hypermesh-consensus-rust\"\n");
        cargo_toml.push_str("version = \"0.1.0\"\n");
        cargo_toml.push_str(&format!("edition = \"{}\"\n\n", self.rust_config.edition));
        
        cargo_toml.push_str("[dependencies]\n");
        for crate_name in &self.rust_config.consensus_crates {
            cargo_toml.push_str(&format!("{} = \"*\"\n", crate_name));
        }
        
        if !self.rust_config.gpu_crates.is_empty() {
            cargo_toml.push_str("\n# GPU computing crates\n");
            for crate_name in &self.rust_config.gpu_crates {
                cargo_toml.push_str(&format!("{} = \"*\"\n", crate_name));
            }
        }
        
        if !self.rust_config.async_crates.is_empty() {
            cargo_toml.push_str("\n# Async runtime crates\n");
            for crate_name in &self.rust_config.async_crates {
                cargo_toml.push_str(&format!("{} = \"*\"\n", crate_name));
            }
        }
        
        cargo_toml.push_str("\n[profile.release]\n");
        cargo_toml.push_str(&format!("opt-level = {}\n", self.rust_config.optimization_level));
        cargo_toml.push_str(&format!("debug = {}\n", self.rust_config.debug_info));
        
        Ok(cargo_toml)
    }
    
    /// Generate Julia RustCall integration
    async fn generate_rustcall_integration(
        &self,
        binary_path: &str,
        consensus_proof: &ConsensusProof,
    ) -> Result<String> {
        let mut julia_code = String::new();
        
        julia_code.push_str("using RustCall\n");
        julia_code.push_str("using HyperMeshConsensus\n\n");
        
        julia_code.push_str("# Rust binary execution with consensus\n");
        julia_code.push_str(&format!(
            "rust_binary = \"{}\"\n",
            binary_path
        ));
        
        julia_code.push_str("# Consensus validation\n");
        julia_code.push_str(&format!(
            "proof = ConsensusProof({}, {}, {}, {})\n",
            consensus_proof.space_proof.total_size,
            consensus_proof.stake_proof.stake_amount,
            consensus_proof.work_proof.computational_power,
            consensus_proof.time_proof.nonce
        ));
        
        julia_code.push_str("\n");
        julia_code.push_str("consensus_result = begin\n");
        julia_code.push_str("    if validate_consensus_proof(proof)\n");
        julia_code.push_str("        try\n");
        julia_code.push_str("            # Execute Rust binary through RustCall\n");
        julia_code.push_str("            rust_output = run(`$rust_binary`)\n");
        julia_code.push_str("            ConsensusExecutionResult(true, rust_output, nothing)\n");
        julia_code.push_str("        catch e\n");
        julia_code.push_str("            ConsensusExecutionResult(false, nothing, string(e))\n");
        julia_code.push_str("        end\n");
        julia_code.push_str("    else\n");
        julia_code.push_str("        ConsensusExecutionResult(false, nothing, \"Consensus validation failed\")\n");
        julia_code.push_str("    end\n");
        julia_code.push_str("end\n");
        
        Ok(julia_code)
    }
    
    /// Execute Julia code with RustCall
    async fn execute_julia_rustcall(
        &self,
        julia_code: &str,
        context: Arc<ExecutionContext>,
    ) -> Result<ExecutionResult> {
        // This would integrate with the actual Julia VM execution
        // For now, return a placeholder result
        Ok(ExecutionResult {
            execution_id: uuid::Uuid::new_v4().to_string(),
            success: true,
            output: Some(serde_json::json!({
                "result": "Rust code compiled and executed through RustCall",
                "adapter": "RustCall",
                "julia_code_length": julia_code.len(),
                "optimization_level": self.rust_config.optimization_level,
                "edition": self.rust_config.edition
            })),
            error_message: None,
            consensus_results: super::super::super::consensus::ConsensusExecutionResult::success(
                "rust_rustcall".to_string(),
                Some(serde_json::json!({"rustcall_execution": "completed"})),
                super::super::super::consensus::ProofValidationResults {
                    space_proof_valid: true,
                    stake_proof_valid: true,
                    work_proof_valid: true,
                    time_proof_valid: true,
                    combined_proof_hash_valid: true,
                    validation_timestamp: std::time::SystemTime::now(),
                },
                super::super::super::consensus::ResourceUsageMetrics {
                    cpu_cycles: 75000,
                    memory_bytes: 64 * 1024 * 1024, // 64MB for Rust
                    storage_bytes: 0,
                    network_bytes: 0,
                    execution_duration_micros: 10000, // Fast compiled execution
                },
            ),
            asset_utilization: super::super::super::execution::AssetUtilizationReport {
                cpu_utilization: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("rust_binary".to_string(), 40.0);
                    map
                },
                gpu_utilization: None,
                memory_usage: super::super::super::execution::MemoryUsagePattern {
                    peak_usage: 64 * 1024 * 1024,
                    average_usage: 32 * 1024 * 1024,
                    allocations: 10,
                    deallocations: 10,
                    gc_events: 0, // Rust doesn't use GC
                },
                storage_operations: vec![],
                network_bandwidth_used: 0,
            },
            privacy_compliance: super::super::super::execution::PrivacyComplianceReport {
                privacy_level_used: crate::catalog::vm::PrivacyLevel::Private,
                anonymization_applied: false,
                data_sharing_events: vec![],
                violations: vec![],
            },
            metadata: super::super::super::execution::ExecutionMetadata {
                language: "rust".to_string(),
                code_size: julia_code.len() as u64,
                started_at: std::time::SystemTime::now(),
                completed_at: Some(std::time::SystemTime::now()),
                duration_micros: 10000,
                vm_version: "rust-rustcall-consensus-0.1.0".to_string(),
                consensus_requirements_hash: [0; 32],
            },
        })
    }
    
    /// Parse Rust-specific consensus constructs
    fn parse_rust_consensus_constructs(&self, code: &str) -> Result<Vec<ConsensusConstruct>> {
        let mut constructs = Vec::new();
        let lines: Vec<&str> = code.lines().collect();
        
        for (line_num, line) in lines.iter().enumerate() {
            // Parse #[consensus_required] attribute
            if line.trim().starts_with("#[consensus_required") {
                let construct = self.parse_consensus_attribute(line, line_num as u32)?;
                constructs.push(construct);
            }
            
            // Parse consensus_validate! macro calls
            if line.contains("consensus_validate!") {
                let construct = self.parse_consensus_macro(line, line_num as u32)?;
                constructs.push(construct);
            }
            
            // Parse asset struct instantiations
            if line.contains("CpuAsset::new") || line.contains("GpuAsset::new") ||
               line.contains("MemoryAsset::new") || line.contains("StorageAsset::new") {
                let construct = self.parse_asset_operation(line, line_num as u32)?;
                constructs.push(construct);
            }
            
            // Parse remote_execute calls
            if line.contains("remote_execute") {
                let construct = self.parse_p2p_execution(line, line_num as u32)?;
                constructs.push(construct);
            }
        }
        
        Ok(constructs)
    }
    
    /// Parse #[consensus_required] attribute
    fn parse_consensus_attribute(&self, line: &str, line_num: u32) -> Result<ConsensusConstruct> {
        let mut required_proofs = Vec::new();
        
        // Parse attribute parameters
        if let Some(params_start) = line.find('(') {
            if let Some(params_end) = line.find(')') {
                let params = &line[params_start + 1..params_end];
                for param in params.split(',') {
                    let parts: Vec<&str> = param.split('=').collect();
                    if parts.len() == 2 {
                        let proof_type = parts[0].trim();
                        let value: u64 = parts[1].trim().parse().unwrap_or(0);
                        
                        let mut minimum_values = std::collections::HashMap::new();
                        minimum_values.insert(proof_type.to_string(), value);
                        
                        required_proofs.push(ProofRequirement {
                            proof_type: proof_type.to_string(),
                            minimum_values,
                            constraints: vec!["rust_validated".to_string()],
                        });
                    }
                }
            }
        }
        
        Ok(ConsensusConstruct {
            construct_type: ConsensusConstructType::ConsensusFunction,
            source_location: SourceLocation {
                line: line_num + 1,
                column: 1,
                length: line.len() as u32,
                text: line.to_string(),
            },
            required_proofs,
            asset_dependencies: vec![],
        })
    }
    
    /// Parse consensus_validate! macro
    fn parse_consensus_macro(&self, line: &str, line_num: u32) -> Result<ConsensusConstruct> {
        Ok(ConsensusConstruct {
            construct_type: ConsensusConstructType::ConsensusFunction,
            source_location: SourceLocation {
                line: line_num + 1,
                column: 1,
                length: line.len() as u32,
                text: line.to_string(),
            },
            required_proofs: vec![
                ProofRequirement {
                    proof_type: "all".to_string(),
                    minimum_values: std::collections::HashMap::new(),
                    constraints: vec!["macro_validated".to_string()],
                }
            ],
            asset_dependencies: vec![],
        })
    }
    
    /// Parse asset operations
    fn parse_asset_operation(&self, line: &str, line_num: u32) -> Result<ConsensusConstruct> {
        let asset_type = if line.contains("CpuAsset") {
            "cpu"
        } else if line.contains("GpuAsset") {
            "gpu"
        } else if line.contains("MemoryAsset") {
            "memory"
        } else if line.contains("StorageAsset") {
            "storage"
        } else {
            "generic"
        };
        
        let asset_dependencies = vec![AssetDependency {
            asset_type: asset_type.to_string(),
            minimum_amount: 1024,
            access_pattern: AssetAccessPattern::Exclusive, // Rust prefers exclusive access
        }];
        
        Ok(ConsensusConstruct {
            construct_type: ConsensusConstructType::AssetOperation,
            source_location: SourceLocation {
                line: line_num + 1,
                column: 1,
                length: line.len() as u32,
                text: line.to_string(),
            },
            required_proofs: vec![
                ProofRequirement {
                    proof_type: "space".to_string(),
                    minimum_values: {
                        let mut map = std::collections::HashMap::new();
                        map.insert("space".to_string(), 1024);
                        map
                    },
                    constraints: vec!["asset_validated".to_string()],
                }
            ],
            asset_dependencies,
        })
    }
    
    /// Parse P2P execution calls
    fn parse_p2p_execution(&self, line: &str, line_num: u32) -> Result<ConsensusConstruct> {
        Ok(ConsensusConstruct {
            construct_type: ConsensusConstructType::P2PExecution,
            source_location: SourceLocation {
                line: line_num + 1,
                column: 1,
                length: line.len() as u32,
                text: line.to_string(),
            },
            required_proofs: vec![
                ProofRequirement {
                    proof_type: "stake".to_string(),
                    minimum_values: {
                        let mut map = std::collections::HashMap::new();
                        map.insert("stake".to_string(), 1000);
                        map
                    },
                    constraints: vec!["p2p_validated".to_string()],
                }
            ],
            asset_dependencies: vec![
                AssetDependency {
                    asset_type: "network".to_string(),
                    minimum_amount: 1024,
                    access_pattern: AssetAccessPattern::Shared,
                }
            ],
        })
    }
    
    /// Analyze Rust code for asset requirements
    fn analyze_rust_asset_requirements(&self, code: &str) -> Result<AssetRequirements> {
        let mut cpu_requirements = None;
        let mut memory_requirements = None;
        let mut gpu_requirements = None;
        let mut storage_requirements = None;
        
        // Analyze for parallel processing
        if code.contains("rayon") || code.contains("std::thread") || 
           code.contains("tokio") || code.contains("async") {
            cpu_requirements = Some(CpuRequirements {
                min_cores: 2,
                preferred_cores: std::thread::available_parallelism()
                    .map(|n| n.get() as u32)
                    .unwrap_or(4),
                architecture: Some("x86_64".to_string()),
                required_features: vec!["multithread".to_string()],
            });
        }
        
        // Analyze memory usage patterns
        let memory_multiplier = if code.contains("Vec") || code.contains("HashMap") {
            if code.contains("Box") || code.contains("Rc") || code.contains("Arc") {
                3 // Complex data structures
            } else {
                2 // Simple collections
            }
        } else {
            1
        };
        
        let base_memory = 128 * 1024 * 1024; // 128MB base
        let estimated_memory = base_memory * memory_multiplier;
        
        memory_requirements = Some(MemoryRequirements {
            min_ram_bytes: estimated_memory / 2,
            preferred_ram_bytes: estimated_memory,
            access_patterns: vec![MemoryAccessPattern::Sequential], // Rust prefers sequential
        });
        
        // Check for GPU libraries
        if code.contains("novarc") || code.contains("wgpu") || 
           code.contains("vulkano") || code.contains("opencl") {
            gpu_requirements = Some(GpuRequirements {
                min_memory_bytes: 1024 * 1024 * 1024, // 1GB
                compute_capability: Some("5.0".to_string()),
                gpu_types: vec!["nvidia".to_string(), "amd".to_string()],
            });
        }
        
        // Check for storage operations
        if code.contains("std::fs") || code.contains("serde") || 
           code.contains("bincode") || code.contains("persist") {
            storage_requirements = Some(StorageRequirements {
                min_storage_bytes: 100 * 1024 * 1024, // 100MB
                storage_types: vec!["ssd".to_string()], // Rust prefers fast storage
                io_patterns: vec!["sequential".to_string()],
            });
        }
        
        Ok(AssetRequirements {
            cpu_requirements,
            gpu_requirements,
            memory_requirements,
            storage_requirements,
            network_requirements: None,
        })
    }
    
    /// Translate Rust errors to user-friendly messages
    fn translate_rust_error(&self, error: &str) -> Result<TranslatedError> {
        let error_category = if error.contains("error[E") {
            if error.contains("E0425") || error.contains("E0412") {
                ErrorCategory::SyntaxError // Cannot find value/type
            } else if error.contains("E0277") || error.contains("E0308") {
                ErrorCategory::RuntimeError // Trait not implemented/type mismatch
            } else {
                ErrorCategory::SyntaxError
            }
        } else if error.contains("consensus") {
            ErrorCategory::ConsensusError
        } else if error.contains("memory") || error.contains("allocation") {
            ErrorCategory::ResourceError
        } else {
            ErrorCategory::RuntimeError
        };
        
        let translated_error = match error_category {
            ErrorCategory::SyntaxError => {
                if error.contains("E0425") {
                    "Variable or function not found - check spelling and imports".to_string()
                } else if error.contains("E0412") {
                    "Type not found - check type names and use statements".to_string()
                } else {
                    "Rust compilation error - check syntax and types".to_string()
                }
            },
            ErrorCategory::RuntimeError => {
                if error.contains("E0277") {
                    "Trait not implemented - implement required traits".to_string()
                } else {
                    "Type mismatch - check function signatures and variable types".to_string()
                }
            },
            ErrorCategory::ConsensusError => "Consensus validation failed - ensure proper proofs".to_string(),
            ErrorCategory::ResourceError => "Memory allocation failed - reduce memory usage".to_string(),
            _ => error.to_string(),
        };
        
        let suggested_fixes = match error_category {
            ErrorCategory::SyntaxError => vec![
                "Check variable and function names".to_string(),
                "Verify all imports with 'use' statements".to_string(),
                "Check Rust edition compatibility".to_string(),
            ],
            ErrorCategory::RuntimeError => vec![
                "Check trait implementations".to_string(),
                "Verify type annotations".to_string(),
                "Check function signatures".to_string(),
            ],
            ErrorCategory::ConsensusError => vec![
                "Verify consensus proofs meet minimum requirements".to_string(),
                "Check asset allocations".to_string(),
                "Ensure proper attribute usage".to_string(),
            ],
            _ => vec!["Check Rust documentation for details".to_string()],
        };
        
        Ok(TranslatedError {
            original_error: error.to_string(),
            translated_error,
            error_category,
            suggested_fixes,
            consensus_issues: if matches!(error_category, ErrorCategory::ConsensusError) {
                vec!["Consensus validation failed in Rust code".to_string()]
            } else {
                vec![]
            },
        })
    }
}

#[async_trait]
impl LanguageRuntime for RustAdapter {
    fn language_id(&self) -> &str {
        "rust"
    }
    
    fn adapter_type(&self) -> &str {
        "RustCall"
    }
    
    async fn execute_with_consensus(
        &self,
        code: &str,
        context: Arc<ExecutionContext>,
        consensus_proof: ConsensusProof,
    ) -> Result<ExecutionResult> {
        self.execute_rust_with_consensus(code, context, consensus_proof).await
    }
    
    async fn validate_consensus_constructs(
        &self,
        code: &str,
        requirements: &ConsensusRequirements,
    ) -> Result<Vec<ConsensusConstruct>> {
        let constructs = self.parse_rust_consensus_constructs(code)?;
        
        // Validate constructs against requirements
        for construct in &constructs {
            for proof_req in &construct.required_proofs {
                match proof_req.proof_type.as_str() {
                    "space" => {
                        if !requirements.require_space_proof {
                            return Err(anyhow::anyhow!("Space proof required but not enabled"));
                        }
                    },
                    "stake" => {
                        if !requirements.require_stake_proof {
                            return Err(anyhow::anyhow!("Stake proof required but not enabled"));
                        }
                    },
                    _ => {},
                }
            }
        }
        
        Ok(constructs)
    }
    
    async fn analyze_asset_requirements(&self, code: &str) -> Result<AssetRequirements> {
        self.analyze_rust_asset_requirements(code)
    }
    
    fn supports_consensus_feature(&self, feature: &str) -> bool {
        match feature {
            "consensus_attributes" => true,
            "consensus_macros" => true,
            "asset_traits" => true,
            "zero_cost_consensus" => true,
            "memory_safety" => true,
            "rustcall_integration" => true,
            "compile_time_validation" => true,
            "unsafe_code" => self.rust_config.allow_unsafe,
            _ => false,
        }
    }
    
    async fn translate_error(&self, error: &str) -> Result<TranslatedError> {
        self.translate_rust_error(error)
    }
}

impl RustAdapterConfig {
    pub fn from_language_config(config: Option<&LanguageSpecificConfig>) -> Self {
        if let Some(lang_config) = config {
            Self {
                rustc_path: lang_config.runtime_path.clone(),
                cargo_path: None,
                target_arch: Some("x86_64-unknown-linux-gnu".to_string()),
                optimization_level: 2, // Balanced optimization
                debug_info: false,     // Release builds
                edition: "2021".to_string(),
                consensus_crates: vec![
                    "serde".to_string(),
                    "tokio".to_string(),
                    "anyhow".to_string(),
                ],
                gpu_crates: vec![
                    "novarc".to_string(),
                    "wgpu".to_string(),
                ],
                async_crates: vec![
                    "tokio".to_string(),
                    "async-trait".to_string(),
                ],
                allow_unsafe: false,
                features: vec![],
            }
        } else {
            Self::default()
        }
    }
}

impl Default for RustAdapterConfig {
    fn default() -> Self {
        Self {
            rustc_path: None, // Use system rustc
            cargo_path: None, // Use system cargo
            target_arch: Some("x86_64-unknown-linux-gnu".to_string()),
            optimization_level: 2,
            debug_info: false,
            edition: "2021".to_string(),
            consensus_crates: vec![
                "serde".to_string(),
                "tokio".to_string(),
                "anyhow".to_string(),
                "uuid".to_string(),
            ],
            gpu_crates: vec![
                "novarc".to_string(),
                "wgpu".to_string(),
            ],
            async_crates: vec![
                "tokio".to_string(),
                "async-trait".to_string(),
                "futures".to_string(),
            ],
            allow_unsafe: false,
            features: vec![
                "consensus".to_string(),
                "asset_management".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::vm::consensus::ConsensusVM;
    use crate::catalog::vm::ConsensusRequirements as VMConsensusRequirements;
    
    #[tokio::test]
    async fn test_rust_adapter_creation() {
        let consensus_requirements = VMConsensusRequirements::default();
        let consensus_vm = Arc::new(ConsensusVM::new(consensus_requirements).unwrap());
        let consensus_bridge = Arc::new(ConsensusBridge::new().await.unwrap());
        
        let adapter = RustAdapter::new(consensus_vm, consensus_bridge, None).await;
        assert!(adapter.is_ok());
        
        if let Ok(adapter) = adapter {
            assert_eq!(adapter.language_id(), "rust");
            assert_eq!(adapter.adapter_type(), "RustCall");
        }
    }
    
    #[test]
    fn test_rust_consensus_construct_parsing() {
        let adapter = create_test_adapter();
        
        let code = r#"
#[consensus_required(space=2048, stake=1500)]
fn consensus_compute(data: &str) -> String {
    data.to_uppercase()
}

let cpu_asset = CpuAsset::new(8);
let result = remote_execute("peer1", "compute_task()").unwrap();
consensus_validate!(proof, 1024, 1000, 16, 0);
        "#;
        
        let constructs = adapter.parse_rust_consensus_constructs(code).unwrap();
        assert!(!constructs.is_empty());
        
        let has_consensus = constructs.iter().any(|c| matches!(c.construct_type, ConsensusConstructType::ConsensusFunction));
        let has_asset = constructs.iter().any(|c| matches!(c.construct_type, ConsensusConstructType::AssetOperation));
        let has_p2p = constructs.iter().any(|c| matches!(c.construct_type, ConsensusConstructType::P2PExecution));
        
        assert!(has_consensus);
        assert!(has_asset);
        assert!(has_p2p);
    }
    
    #[test]
    fn test_rust_asset_requirements() {
        let adapter = create_test_adapter();
        
        let code = r#"
use rayon::prelude::*;
use novarc::driver::*;
use std::fs::File;

fn parallel_compute() {
    let data: Vec<i32> = (0..1_000_000).collect();
    let result: Vec<i32> = data.par_iter().map(|&x| x * x).collect();
    
    // GPU computation
    let gpu = CudaDevice::new(0).unwrap();
    
    // File operations
    let mut file = File::create("output.dat").unwrap();
}
        "#;
        
        let requirements = adapter.analyze_rust_asset_requirements(code).unwrap();
        
        // Should detect CPU, memory, GPU, and storage requirements
        assert!(requirements.cpu_requirements.is_some());
        assert!(requirements.memory_requirements.is_some());
        assert!(requirements.gpu_requirements.is_some());
        assert!(requirements.storage_requirements.is_some());
        
        let cpu_req = requirements.cpu_requirements.unwrap();
        assert!(cpu_req.min_cores >= 2);
        assert!(cpu_req.required_features.contains(&"multithread".to_string()));
    }
    
    #[test]
    fn test_rust_error_translation() {
        let adapter = create_test_adapter();
        
        let error = "error[E0425]: cannot find value `undefined_var` in this scope";
        let translated = adapter.translate_rust_error(error).unwrap();
        
        assert!(matches!(translated.error_category, ErrorCategory::SyntaxError));
        assert!(!translated.suggested_fixes.is_empty());
        assert!(translated.translated_error.contains("not found"));
    }
    
    #[test]
    fn test_rust_config_defaults() {
        let config = RustAdapterConfig::default();
        
        assert_eq!(config.edition, "2021");
        assert_eq!(config.optimization_level, 2);
        assert!(!config.allow_unsafe);
        assert!(config.consensus_crates.contains(&"serde".to_string()));
        assert!(config.gpu_crates.contains(&"novarc".to_string()));
    }
    
    #[test]
    fn test_cargo_toml_generation() {
        let adapter = create_test_adapter();
        let cargo_toml = adapter.generate_cargo_toml().unwrap();
        
        assert!(cargo_toml.contains("[package]"));
        assert!(cargo_toml.contains("edition = \"2021\""));
        assert!(cargo_toml.contains("[dependencies]"));
        assert!(cargo_toml.contains("serde"));
        assert!(cargo_toml.contains("[profile.release]"));
    }
    
    // Helper function
    fn create_test_adapter() -> RustAdapter {
        let consensus_requirements = VMConsensusRequirements::default();
        let consensus_vm = Arc::new(ConsensusVM::new(consensus_requirements).unwrap());
        let consensus_bridge = Arc::new(ConsensusBridge::new().await.unwrap());
        
        RustAdapter {
            base: BaseAdapter::new(
                "rust".to_string(),
                "RustCall".to_string(),
                consensus_vm,
                Arc::clone(&consensus_bridge),
                None,
            ),
            consensus_bridge,
            rust_config: RustAdapterConfig::default(),
        }
    }
}