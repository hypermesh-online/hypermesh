//! Language-Specific Adapters for Multi-Language VM Support
//!
//! This module implements dedicated adapters for each supported language,
//! translating consensus constructs into language-native operations while
//! maintaining full ConsensusProof validation (PoSp+PoSt+PoWk+PoTm).
//!
//! Each adapter follows Proof of State patterns for consensus integration:
//! - Native Julia VM integration (direct runtime access)
//! - PyCall for Python execution through Julia
//! - RCall for R execution through Julia  
//! - JavaScriptCall for JavaScript execution
//! - Clang/Cxx for C/C++ compilation and execution
//! - RustCall for Rust compilation and execution

pub mod julia;
pub mod python;
pub mod r;
pub mod javascript;
pub mod c;
pub mod cpp;
pub mod rust;

// Re-export adapters
pub use julia::JuliaAdapter;
pub use python::PythonAdapter;
pub use r::RAdapter;
pub use javascript::JavaScriptAdapter;
pub use c::CAdapter;
pub use cpp::CppAdapter;
pub use rust::RustAdapter;

// Import needed types from parent module
use super::{ConsensusConstructType, SourceLocation};

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::consensus::proof::ConsensusProof;
use super::super::consensus::ConsensusVM;
use super::super::execution::{ExecutionContext, ExecutionResult};
use super::{
    LanguageRuntime, LanguageSpecificConfig, ConsensusRequirements,
    ConsensusConstruct, AssetRequirements, TranslatedError, ConsensusBridge
};

/// Base adapter functionality shared across all language adapters
pub struct BaseAdapter {
    /// Language identifier
    language_id: String,
    /// Adapter type (PyCall, RCall, etc.)
    adapter_type: String,
    /// Consensus VM for validation
    consensus_vm: Arc<ConsensusVM>,
    /// Language-specific configuration
    config: AdapterConfig,
    /// Consensus bridge for translation
    consensus_bridge: Arc<ConsensusBridge>,
}

/// Adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Runtime timeout (microseconds)
    pub timeout_micros: u64,
    /// Memory limit (bytes)
    pub memory_limit_bytes: u64,
    /// Consensus requirements
    pub consensus_requirements: ConsensusRequirements,
    /// Allowed asset types
    pub allowed_asset_types: Vec<String>,
    /// Runtime-specific settings
    pub runtime_settings: RuntimeSettings,
}

/// Runtime-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSettings {
    /// Environment variables
    pub env_vars: std::collections::HashMap<String, String>,
    /// Runtime flags
    pub runtime_flags: Vec<String>,
    /// Library paths
    pub library_paths: Vec<String>,
    /// Additional configuration
    pub additional_config: std::collections::HashMap<String, serde_json::Value>,
}

impl BaseAdapter {
    /// Create new base adapter
    pub fn new(
        language_id: String,
        adapter_type: String,
        consensus_vm: Arc<ConsensusVM>,
        consensus_bridge: Arc<ConsensusBridge>,
        config: Option<&LanguageSpecificConfig>,
    ) -> Self {
        let adapter_config = AdapterConfig::from_language_config(config);
        
        Self {
            language_id,
            adapter_type,
            consensus_vm,
            config: adapter_config,
            consensus_bridge,
        }
    }
    
    /// Get language identifier
    pub fn language_id(&self) -> &str {
        &self.language_id
    }
    
    /// Get adapter type
    pub fn adapter_type(&self) -> &str {
        &self.adapter_type
    }
    
    /// Get consensus VM
    pub fn consensus_vm(&self) -> Arc<ConsensusVM> {
        Arc::clone(&self.consensus_vm)
    }
    
    /// Get configuration
    pub fn config(&self) -> &AdapterConfig {
        &self.config
    }
    
    /// Validate consensus proof against adapter requirements
    pub async fn validate_adapter_consensus_proof(
        &self,
        proof: &ConsensusProof,
    ) -> Result<bool> {
        // First validate against global VM requirements
        if !self.consensus_vm.validate_consensus_proof(proof).await? {
            return Ok(false);
        }
        
        // Then validate against adapter-specific requirements
        self.validate_adapter_specific_requirements(proof).await
    }
    
    /// Validate adapter-specific consensus requirements
    async fn validate_adapter_specific_requirements(
        &self,
        proof: &ConsensusProof,
    ) -> Result<bool> {
        let requirements = &self.config.consensus_requirements;
        
        // Validate space proof if required
        if requirements.require_space_proof {
            if let Some(min_space) = requirements.minimum_proof_values.get("space") {
                if proof.proof_of_space.committed_space < *min_space {
                    return Ok(false);
                }
            }
        }
        
        // Validate stake proof if required
        if requirements.require_stake_proof {
            if let Some(min_stake) = requirements.minimum_proof_values.get("stake") {
                if proof.proof_of_stake.authority_level < *min_stake {
                    return Ok(false);
                }
            }
        }
        
        // Validate work proof if required
        if requirements.require_work_proof {
            if let Some(min_difficulty) = requirements.minimum_proof_values.get("work") {
                if (proof.proof_of_work.difficulty as u64) < *min_difficulty {
                    return Ok(false);
                }
            }
        }
        
        // Time validation handled by VM layer
        
        Ok(true)
    }
    
    /// Create execution environment for language
    pub async fn create_execution_environment(
        &self,
        context: &ExecutionContext,
    ) -> Result<ExecutionEnvironment> {
        ExecutionEnvironment::new(
            &self.language_id,
            &self.config,
            context,
        ).await
    }
    
    /// Cleanup execution environment
    pub async fn cleanup_execution_environment(
        &self,
        env: ExecutionEnvironment,
    ) -> Result<()> {
        env.cleanup().await
    }
}

/// Execution environment for language adapter
pub struct ExecutionEnvironment {
    /// Language identifier
    language_id: String,
    /// Process ID if applicable
    process_id: Option<u32>,
    /// Working directory
    working_directory: String,
    /// Environment variables
    env_vars: std::collections::HashMap<String, String>,
    /// Resource allocations
    resource_allocations: ResourceAllocations,
    /// Temporary files created
    temp_files: Vec<String>,
}

/// Resource allocations for execution
#[derive(Debug, Clone)]
pub struct ResourceAllocations {
    /// Allocated CPU cores
    pub cpu_cores: Vec<u32>,
    /// Allocated memory (bytes)
    pub memory_bytes: u64,
    /// GPU allocation if any
    pub gpu_allocation: Option<GpuAllocation>,
    /// Storage allocation
    pub storage_bytes: u64,
}

/// GPU allocation details
#[derive(Debug, Clone)]
pub struct GpuAllocation {
    /// GPU device ID
    pub device_id: u32,
    /// Allocated memory (bytes)
    pub memory_bytes: u64,
    /// Compute units allocated
    pub compute_units: u32,
}

impl ExecutionEnvironment {
    /// Create new execution environment
    pub async fn new(
        language_id: &str,
        config: &AdapterConfig,
        context: &ExecutionContext,
    ) -> Result<Self> {
        let working_directory = format!("/tmp/hypermesh_vm_{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&working_directory)?;
        
        // Allocate resources based on configuration and availability
        let resource_allocations = ResourceAllocations {
            cpu_cores: Self::allocate_cpu_cores(config, context).await?,
            memory_bytes: Self::allocate_memory(config, context).await?,
            gpu_allocation: Self::allocate_gpu(config, context).await?,
            storage_bytes: Self::allocate_storage(config, context).await?,
        };
        
        // Set up environment variables
        let mut env_vars = config.runtime_settings.env_vars.clone();
        env_vars.insert("HYPERMESH_LANGUAGE".to_string(), language_id.to_string());
        env_vars.insert("HYPERMESH_WORKDIR".to_string(), working_directory.clone());
        
        Ok(Self {
            language_id: language_id.to_string(),
            process_id: None,
            working_directory,
            env_vars,
            resource_allocations,
            temp_files: Vec::new(),
        })
    }
    
    /// Allocate CPU cores for execution
    async fn allocate_cpu_cores(
        config: &AdapterConfig,
        _context: &ExecutionContext,
    ) -> Result<Vec<u32>> {
        // Simplified CPU allocation - would integrate with actual resource manager
        let available_cores = std::thread::available_parallelism()?.get() as u32;
        let requested_cores = std::cmp::min(2, available_cores); // Conservative default
        
        Ok((0..requested_cores).collect())
    }
    
    /// Allocate memory for execution
    async fn allocate_memory(
        config: &AdapterConfig,
        _context: &ExecutionContext,
    ) -> Result<u64> {
        // Use configured memory limit or conservative default
        Ok(std::cmp::min(config.memory_limit_bytes, 2 * 1024 * 1024 * 1024)) // 2GB max
    }
    
    /// Allocate GPU if available and requested
    async fn allocate_gpu(
        _config: &AdapterConfig,
        _context: &ExecutionContext,
    ) -> Result<Option<GpuAllocation>> {
        // GPU allocation would integrate with actual GPU resource manager
        Ok(None) // Placeholder - no GPU allocation for now
    }
    
    /// Allocate storage space
    async fn allocate_storage(
        _config: &AdapterConfig,
        _context: &ExecutionContext,
    ) -> Result<u64> {
        // Conservative storage allocation
        Ok(1024 * 1024 * 1024) // 1GB
    }
    
    /// Get working directory
    pub fn working_directory(&self) -> &str {
        &self.working_directory
    }
    
    /// Get environment variables
    pub fn env_vars(&self) -> &std::collections::HashMap<String, String> {
        &self.env_vars
    }
    
    /// Get resource allocations
    pub fn resource_allocations(&self) -> &ResourceAllocations {
        &self.resource_allocations
    }
    
    /// Add temporary file for cleanup
    pub fn add_temp_file(&mut self, file_path: String) {
        self.temp_files.push(file_path);
    }
    
    /// Cleanup execution environment
    pub async fn cleanup(self) -> Result<()> {
        // Clean up temporary files
        for file_path in &self.temp_files {
            if std::path::Path::new(file_path).exists() {
                std::fs::remove_file(file_path)?;
            }
        }
        
        // Clean up working directory
        if std::path::Path::new(&self.working_directory).exists() {
            std::fs::remove_dir_all(&self.working_directory)?;
        }
        
        // TODO: Deallocate resources (CPU, memory, GPU, storage)
        // This would integrate with the actual resource manager
        
        Ok(())
    }
}

impl AdapterConfig {
    /// Create adapter config from language-specific config
    pub fn from_language_config(config: Option<&LanguageSpecificConfig>) -> Self {
        let default_config = Self::default();
        
        if let Some(lang_config) = config {
            Self {
                timeout_micros: lang_config.timeout_micros.unwrap_or(default_config.timeout_micros),
                memory_limit_bytes: lang_config.memory_limit_bytes.unwrap_or(default_config.memory_limit_bytes),
                consensus_requirements: lang_config.consensus_requirements.clone()
                    .unwrap_or(default_config.consensus_requirements),
                allowed_asset_types: lang_config.allowed_asset_types.clone(),
                runtime_settings: default_config.runtime_settings,
            }
        } else {
            default_config
        }
    }
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            timeout_micros: 30_000_000, // 30 seconds
            memory_limit_bytes: 2 * 1024 * 1024 * 1024, // 2GB
            consensus_requirements: ConsensusRequirements::default(),
            allowed_asset_types: vec![
                "cpu".to_string(),
                "memory".to_string(),
                "storage".to_string(),
            ],
            runtime_settings: RuntimeSettings::default(),
        }
    }
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("HYPERMESH_ENABLED".to_string(), "1".to_string());
        
        Self {
            env_vars,
            runtime_flags: Vec::new(),
            library_paths: Vec::new(),
            additional_config: std::collections::HashMap::new(),
        }
    }
}

/// Utility functions for adapter implementations
pub mod utils {
    use super::*;
    
    /// Parse consensus annotations from source code
    pub fn parse_consensus_annotations(code: &str, language: &str) -> Result<Vec<ConsensusConstruct>> {
        let mut constructs = Vec::new();
        
        // Language-specific parsing logic would go here
        match language {
            "julia" => parse_julia_consensus_annotations(code, &mut constructs)?,
            "python" => parse_python_consensus_annotations(code, &mut constructs)?,
            "r" => parse_r_consensus_annotations(code, &mut constructs)?,
            "javascript" => parse_javascript_consensus_annotations(code, &mut constructs)?,
            "c" | "cpp" => parse_c_consensus_annotations(code, &mut constructs)?,
            "rust" => parse_rust_consensus_annotations(code, &mut constructs)?,
            _ => return Err(anyhow::anyhow!("Unsupported language for consensus parsing: {}", language)),
        }
        
        Ok(constructs)
    }
    
    /// Generate language-specific consensus validation code
    pub fn generate_consensus_validation_code(
        language: &str,
        constructs: &[ConsensusConstruct],
    ) -> Result<String> {
        match language {
            "julia" => generate_julia_consensus_code(constructs),
            "python" => generate_python_consensus_code(constructs),
            "r" => generate_r_consensus_code(constructs),
            "javascript" => generate_javascript_consensus_code(constructs),
            "c" => generate_c_consensus_code(constructs),
            "cpp" => generate_cpp_consensus_code(constructs),
            "rust" => generate_rust_consensus_code(constructs),
            _ => Err(anyhow::anyhow!("Unsupported language for consensus code generation: {}", language)),
        }
    }
    
    // Language-specific parsing implementations (placeholders)
    fn parse_julia_consensus_annotations(code: &str, constructs: &mut Vec<ConsensusConstruct>) -> Result<()> {
        // Parse @consensus macros and similar Julia constructs
        // This would implement actual Julia AST parsing
        if code.contains("@consensus") {
            constructs.push(ConsensusConstruct {
                construct_type: super::ConsensusConstructType::ConsensusFunction,
                source_location: super::SourceLocation {
                    line: 1,
                    column: 1,
                    length: code.len() as u32,
                    text: code.to_string(),
                },
                required_proofs: vec![],
                asset_dependencies: vec![],
            });
        }
        Ok(())
    }
    
    fn parse_python_consensus_annotations(code: &str, constructs: &mut Vec<ConsensusConstruct>) -> Result<()> {
        // Parse Python consensus decorators and functions
        if code.contains("@consensus_required") || code.contains("consensus.validate") {
            constructs.push(ConsensusConstruct {
                construct_type: super::ConsensusConstructType::ConsensusFunction,
                source_location: super::SourceLocation {
                    line: 1,
                    column: 1,
                    length: code.len() as u32,
                    text: code.to_string(),
                },
                required_proofs: vec![],
                asset_dependencies: vec![],
            });
        }
        Ok(())
    }
    
    fn parse_r_consensus_annotations(code: &str, constructs: &mut Vec<ConsensusConstruct>) -> Result<()> {
        // Parse R consensus function calls
        if code.contains("consensus.required") || code.contains("consensus$validate") {
            constructs.push(ConsensusConstruct {
                construct_type: super::ConsensusConstructType::ConsensusFunction,
                source_location: super::SourceLocation {
                    line: 1,
                    column: 1,
                    length: code.len() as u32,
                    text: code.to_string(),
                },
                required_proofs: vec![],
                asset_dependencies: vec![],
            });
        }
        Ok(())
    }
    
    fn parse_javascript_consensus_annotations(code: &str, constructs: &mut Vec<ConsensusConstruct>) -> Result<()> {
        // Parse JavaScript consensus function calls and decorators
        if code.contains("consensusRequired") || code.contains("consensus.validate") {
            constructs.push(ConsensusConstruct {
                construct_type: super::ConsensusConstructType::ConsensusFunction,
                source_location: super::SourceLocation {
                    line: 1,
                    column: 1,
                    length: code.len() as u32,
                    text: code.to_string(),
                },
                required_proofs: vec![],
                asset_dependencies: vec![],
            });
        }
        Ok(())
    }
    
    fn parse_c_consensus_annotations(code: &str, constructs: &mut Vec<ConsensusConstruct>) -> Result<()> {
        // Parse C/C++ consensus macros and function calls
        if code.contains("CONSENSUS_REQUIRED") || code.contains("consensus_validate") {
            constructs.push(ConsensusConstruct {
                construct_type: super::ConsensusConstructType::ConsensusFunction,
                source_location: super::SourceLocation {
                    line: 1,
                    column: 1,
                    length: code.len() as u32,
                    text: code.to_string(),
                },
                required_proofs: vec![],
                asset_dependencies: vec![],
            });
        }
        Ok(())
    }
    
    fn parse_rust_consensus_annotations(code: &str, constructs: &mut Vec<ConsensusConstruct>) -> Result<()> {
        // Parse Rust consensus attributes and function calls
        if code.contains("#[consensus_required]") || code.contains("consensus_validate!") {
            constructs.push(ConsensusConstruct {
                construct_type: super::ConsensusConstructType::ConsensusFunction,
                source_location: super::SourceLocation {
                    line: 1,
                    column: 1,
                    length: code.len() as u32,
                    text: code.to_string(),
                },
                required_proofs: vec![],
                asset_dependencies: vec![],
            });
        }
        Ok(())
    }
    
    // Language-specific consensus code generation (placeholders)
    fn generate_julia_consensus_code(constructs: &[ConsensusConstruct]) -> Result<String> {
        let mut code = String::new();
        for construct in constructs {
            code.push_str(&format!(
                "# Consensus validation for: {}\n",
                construct.source_location.text
            ));
        }
        Ok(code)
    }
    
    fn generate_python_consensus_code(constructs: &[ConsensusConstruct]) -> Result<String> {
        let mut code = String::new();
        for construct in constructs {
            code.push_str(&format!(
                "# Consensus validation for: {}\n",
                construct.source_location.text
            ));
        }
        Ok(code)
    }
    
    fn generate_r_consensus_code(constructs: &[ConsensusConstruct]) -> Result<String> {
        let mut code = String::new();
        for construct in constructs {
            code.push_str(&format!(
                "# Consensus validation for: {}\n",
                construct.source_location.text
            ));
        }
        Ok(code)
    }
    
    fn generate_javascript_consensus_code(constructs: &[ConsensusConstruct]) -> Result<String> {
        let mut code = String::new();
        for construct in constructs {
            code.push_str(&format!(
                "// Consensus validation for: {}\n",
                construct.source_location.text
            ));
        }
        Ok(code)
    }
    
    fn generate_c_consensus_code(constructs: &[ConsensusConstruct]) -> Result<String> {
        let mut code = String::new();
        for construct in constructs {
            code.push_str(&format!(
                "/* Consensus validation for: {} */\n",
                construct.source_location.text
            ));
        }
        Ok(code)
    }
    
    fn generate_cpp_consensus_code(constructs: &[ConsensusConstruct]) -> Result<String> {
        let mut code = String::new();
        for construct in constructs {
            code.push_str(&format!(
                "// Consensus validation for: {}\n",
                construct.source_location.text
            ));
        }
        Ok(code)
    }
    
    fn generate_rust_consensus_code(constructs: &[ConsensusConstruct]) -> Result<String> {
        let mut code = String::new();
        for construct in constructs {
            code.push_str(&format!(
                "// Consensus validation for: {}\n",
                construct.source_location.text
            ));
        }
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::vm::consensus::ConsensusVM;
    use crate::catalog::vm::ConsensusRequirements as VMConsensusRequirements;
    
    #[tokio::test]
    async fn test_base_adapter_creation() {
        let consensus_requirements = VMConsensusRequirements::default();
        let consensus_vm = Arc::new(ConsensusVM::new(consensus_requirements).unwrap());
        let consensus_bridge = Arc::new(ConsensusBridge::new().await.unwrap());
        
        let adapter = BaseAdapter::new(
            "test".to_string(),
            "TestAdapter".to_string(),
            consensus_vm,
            consensus_bridge,
            None,
        );
        
        assert_eq!(adapter.language_id(), "test");
        assert_eq!(adapter.adapter_type(), "TestAdapter");
    }
    
    #[test]
    fn test_adapter_config_defaults() {
        let config = AdapterConfig::default();
        
        assert_eq!(config.timeout_micros, 30_000_000);
        assert_eq!(config.memory_limit_bytes, 2 * 1024 * 1024 * 1024);
        assert!(config.allowed_asset_types.contains(&"cpu".to_string()));
        assert!(config.allowed_asset_types.contains(&"memory".to_string()));
    }
    
    #[tokio::test]
    async fn test_execution_environment_creation() {
        let config = AdapterConfig::default();
        let context = ExecutionContext::default(); // Would need actual implementation
        
        // Note: This test might fail due to missing ExecutionContext implementation
        // but demonstrates the expected structure
    }
    
    #[test]
    fn test_consensus_annotation_parsing() {
        let julia_code = "@consensus function test() end";
        let constructs = utils::parse_consensus_annotations(julia_code, "julia").unwrap();
        assert_eq!(constructs.len(), 1);
        
        let python_code = "@consensus_required\ndef test(): pass";
        let constructs = utils::parse_consensus_annotations(python_code, "python").unwrap();
        assert_eq!(constructs.len(), 1);
    }
}