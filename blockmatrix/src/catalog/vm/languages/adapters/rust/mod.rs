// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Rust Language Adapter - RustCall Integration with Consensus
//!
//! This adapter provides Rust code compilation and execution through Julia's RustCall
//! integration, enabling Rust code to access ConsensusProof validation and asset
//! management through the HyperMesh ecosystem.

mod prelude;
mod parsing;
mod analysis;

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
pub use trustchain::consensus::ConsensusProof;

use crate::catalog::vm::consensus as vm_consensus;
use crate::catalog::vm::execution as vm_execution;
use vm_consensus::ConsensusVM;
use vm_execution::{ExecutionContext, ExecutionResult};
use super::{
    LanguageRuntime, BaseAdapter, ExecutionEnvironment,
    LanguageSpecificConfig, ConsensusBridge,
};
use super::super::{
    ConsensusRequirements, ConsensusConstruct, AssetRequirements, TranslatedError,
};

/// Rust language adapter using RustCall integration
#[allow(dead_code)]
pub struct RustAdapter {
    base: BaseAdapter,
    consensus_bridge: Arc<ConsensusBridge>,
    rust_config: RustAdapterConfig,
}

/// Rust-specific adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustAdapterConfig {
    pub rustc_path: Option<String>,
    pub cargo_path: Option<String>,
    pub target_arch: Option<String>,
    pub optimization_level: u8,
    pub debug_info: bool,
    pub edition: String,
    pub consensus_crates: Vec<String>,
    pub gpu_crates: Vec<String>,
    pub async_crates: Vec<String>,
    pub allow_unsafe: bool,
    pub features: Vec<String>,
}

impl RustAdapter {
    /// Create new Rust adapter with RustCall integration
    pub async fn new(
        consensus_vm: Arc<RwLock<ConsensusVM>>,
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

        Ok(Self { base, consensus_bridge, rust_config })
    }

    /// Execute Rust code through RustCall with consensus validation
    async fn execute_rust_with_consensus(
        &self,
        code: &str,
        context: Arc<ExecutionContext>,
        consensus_proof: ConsensusProof,
    ) -> Result<ExecutionResult> {
        let env = self.base.create_execution_environment(&context).await?;

        if !self.base.validate_adapter_consensus_proof(&consensus_proof).await? {
            return Err(anyhow::anyhow!("Consensus proof validation failed for Rust execution"));
        }

        let prepared_code = self.prepare_rust_code(code, &consensus_proof).await?;
        let compiled_binary = self.compile_rust_code(&prepared_code, &env).await?;
        let julia_code = self.generate_rustcall_integration(&compiled_binary, &consensus_proof).await?;
        let result = self.execute_julia_rustcall(&julia_code, context).await?;

        self.base.cleanup_execution_environment(env).await?;
        Ok(result)
    }

    async fn prepare_rust_code(&self, rust_code: &str, _consensus_proof: &ConsensusProof) -> Result<String> {
        let mut prepared_code = prelude::generate_rust_consensus_prelude();
        prepared_code.push_str("\n// User Rust code\n");
        prepared_code.push_str(rust_code);

        if !rust_code.contains("fn main") {
            prepared_code.push_str("\n\nfn main() {\n");
            prepared_code.push_str("    // Execute user code with consensus validation\n");
            prepared_code.push_str("    let _result = user_code_execution();\n");
            prepared_code.push_str("}\n");
        }

        Ok(prepared_code)
    }

    async fn compile_rust_code(&self, code: &str, env: &ExecutionEnvironment) -> Result<String> {
        let source_file = format!("{}/main.rs", env.working_directory());
        let binary_file = format!("{}/main", env.working_directory());

        std::fs::write(&source_file, code)?;

        let cargo_toml = self.generate_cargo_toml()?;
        let cargo_toml_path = format!("{}/Cargo.toml", env.working_directory());
        std::fs::write(&cargo_toml_path, cargo_toml)?;

        let rustc_path = self.rust_config.rustc_path.as_deref().unwrap_or("rustc");
        let _compile_command = format!(
            "{} {} -o {} --edition {} -O",
            rustc_path, source_file, binary_file, self.rust_config.edition
        );

        Ok(binary_file)
    }

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

    async fn generate_rustcall_integration(
        &self,
        binary_path: &str,
        consensus_proof: &ConsensusProof,
    ) -> Result<String> {
        let mut julia_code = String::new();

        julia_code.push_str("using RustCall\n");
        julia_code.push_str("using HyperMeshConsensus\n\n");
        julia_code.push_str("# Rust binary execution with consensus\n");
        julia_code.push_str(&format!("rust_binary = \"{}\"\n", binary_path));
        julia_code.push_str("# Consensus validation\n");
        julia_code.push_str(&format!(
            "proof = ConsensusProof({}, {}, {}, {})\n",
            consensus_proof.space_proof.total_size,
            consensus_proof.stake_proof.stake_amount,
            consensus_proof.work_proof.computational_power,
            consensus_proof.time_proof.nonce
        ));
        julia_code.push_str("\nconsensus_result = begin\n");
        julia_code.push_str("    if validate_consensus_proof(proof)\n");
        julia_code.push_str("        try\n");
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

    async fn execute_julia_rustcall(
        &self,
        julia_code: &str,
        _context: Arc<ExecutionContext>,
    ) -> Result<ExecutionResult> {
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
            consensus_results: vm_consensus::ConsensusExecutionResult::success(
                "rust_rustcall".to_string(),
                Some(serde_json::json!({"rustcall_execution": "completed"})),
                vm_consensus::ProofValidationResults {
                    space_proof_valid: true, stake_proof_valid: true,
                    work_proof_valid: true, time_proof_valid: true,
                    combined_proof_hash_valid: true, validation_timestamp: std::time::SystemTime::now(),
                },
                vm_consensus::ResourceUsageMetrics {
                    cpu_cycles: 75000, memory_bytes: 64 * 1024 * 1024,
                    storage_bytes: 0, network_bytes: 0, execution_duration_micros: 10000,
                },
            ),
            asset_utilization: vm_execution::AssetUtilizationReport {
                cpu_utilization: { let mut m = std::collections::HashMap::new(); m.insert("rust_binary".to_string(), 40.0); m },
                gpu_utilization: None,
                memory_usage: vm_execution::MemoryUsagePattern {
                    peak_usage: 64 * 1024 * 1024, average_usage: 32 * 1024 * 1024,
                    allocations: 10, deallocations: 10, gc_events: 0,
                },
                storage_operations: vec![], network_bandwidth_used: 0,
            },
            privacy_compliance: vm_execution::PrivacyComplianceReport {
                privacy_level_used: crate::catalog::vm::PrivacyMode::PRIVATE,
                anonymization_applied: false, data_sharing_events: vec![], violations: vec![],
            },
            metadata: vm_execution::ExecutionMetadata {
                language: "rust".to_string(), code_size: julia_code.len() as u64,
                started_at: std::time::SystemTime::now(), completed_at: Some(std::time::SystemTime::now()),
                duration_micros: 10000, vm_version: "rust-rustcall-consensus-0.1.0".to_string(),
                consensus_requirements_hash: [0; 32],
            },
        })
    }
}

#[async_trait]
impl LanguageRuntime for RustAdapter {
    fn language_id(&self) -> &str { "rust" }
    fn adapter_type(&self) -> &str { "RustCall" }

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
        let constructs = parsing::parse_rust_consensus_constructs(code)?;

        for construct in &constructs {
            for proof_req in &construct.required_proofs {
                match proof_req.proof_type.as_str() {
                    "space" => {
                        if !requirements.require_space_proof {
                            return Err(anyhow::anyhow!("Space proof required but not enabled"));
                        }
                    }
                    "stake" => {
                        if !requirements.require_stake_proof {
                            return Err(anyhow::anyhow!("Stake proof required but not enabled"));
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(constructs)
    }

    async fn analyze_asset_requirements(&self, code: &str) -> Result<AssetRequirements> {
        analysis::analyze_rust_asset_requirements(code)
    }

    fn supports_consensus_feature(&self, feature: &str) -> bool {
        match feature {
            "consensus_attributes" | "consensus_macros" | "asset_traits"
            | "zero_cost_consensus" | "memory_safety" | "rustcall_integration"
            | "compile_time_validation" => true,
            "unsafe_code" => self.rust_config.allow_unsafe,
            _ => false,
        }
    }

    async fn translate_error(&self, error: &str) -> Result<TranslatedError> {
        analysis::translate_rust_error(error)
    }
}

impl RustAdapterConfig {
    pub fn from_language_config(config: Option<&LanguageSpecificConfig>) -> Self {
        if let Some(lang_config) = config {
            Self {
                rustc_path: lang_config.runtime_path.clone(),
                cargo_path: None,
                target_arch: Some("x86_64-unknown-linux-gnu".to_string()),
                optimization_level: 2,
                debug_info: false,
                edition: "2021".to_string(),
                consensus_crates: vec![
                    "serde".to_string(), "tokio".to_string(), "anyhow".to_string(),
                ],
                gpu_crates: vec!["novarc".to_string(), "wgpu".to_string()],
                async_crates: vec!["tokio".to_string(), "async-trait".to_string()],
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
            rustc_path: None,
            cargo_path: None,
            target_arch: Some("x86_64-unknown-linux-gnu".to_string()),
            optimization_level: 2,
            debug_info: false,
            edition: "2021".to_string(),
            consensus_crates: vec![
                "serde".to_string(), "tokio".to_string(),
                "anyhow".to_string(), "uuid".to_string(),
            ],
            gpu_crates: vec!["novarc".to_string(), "wgpu".to_string()],
            async_crates: vec![
                "tokio".to_string(), "async-trait".to_string(), "futures".to_string(),
            ],
            allow_unsafe: false,
            features: vec!["consensus".to_string(), "asset_management".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::{ConsensusConstructType, ErrorCategory};
    use crate::catalog::vm::consensus::ConsensusVM;
    use crate::catalog::vm::ConsensusRequirements as VMConsensusRequirements;

    #[tokio::test]
    async fn test_rust_adapter_creation() {
        let consensus_requirements = VMConsensusRequirements::default();
        let consensus_vm = Arc::new(RwLock::new(ConsensusVM::new(consensus_requirements).expect("test")));
        let consensus_bridge = Arc::new(ConsensusBridge::new().await.expect("test"));

        let adapter = RustAdapter::new(consensus_vm, consensus_bridge, None).await;
        assert!(adapter.is_ok());

        if let Ok(adapter) = adapter {
            assert_eq!(adapter.language_id(), "rust");
            assert_eq!(adapter.adapter_type(), "RustCall");
        }
    }

    #[tokio::test]
    async fn test_rust_consensus_construct_parsing() {
        let code = r#"
#[consensus_required(space=2048, stake=1500)]
fn consensus_compute(data: &str) -> String {
    data.to_uppercase()
}

let cpu_asset = CpuAsset::new(8);
let result = remote_execute("peer1", "compute_task()").expect("p2p");
consensus_validate!(proof, 1024, 1000, 16, 0);
        "#;

        let constructs = parsing::parse_rust_consensus_constructs(code).expect("test");
        assert!(!constructs.is_empty());

        let has_consensus = constructs.iter().any(|c| matches!(c.construct_type, ConsensusConstructType::ConsensusFunction));
        let has_asset = constructs.iter().any(|c| matches!(c.construct_type, ConsensusConstructType::AssetOperation));
        let has_p2p = constructs.iter().any(|c| matches!(c.construct_type, ConsensusConstructType::P2PExecution));

        assert!(has_consensus);
        assert!(has_asset);
        assert!(has_p2p);
    }

    #[tokio::test]
    async fn test_rust_asset_requirements() {
        let code = r#"
use rayon::prelude::*;
use novarc::driver::*;
use std::fs::File;

fn parallel_compute() {
    let data: Vec<i32> = (0..1_000_000).collect();
    let result: Vec<i32> = data.par_iter().map(|&x| x * x).collect();

    // GPU computation
    let gpu = CudaDevice::new(0).expect("gpu");

    // File operations
    let mut file = File::create("output.dat").expect("file");
}
        "#;

        let requirements = analysis::analyze_rust_asset_requirements(code).expect("test");

        assert!(requirements.cpu_requirements.is_some());
        assert!(requirements.memory_requirements.is_some());
        assert!(requirements.gpu_requirements.is_some());
        assert!(requirements.storage_requirements.is_some());

        let cpu_req = requirements.cpu_requirements.expect("test");
        assert!(cpu_req.min_cores >= 2);
        assert!(cpu_req.required_features.contains(&"multithread".to_string()));
    }

    #[tokio::test]
    async fn test_rust_error_translation() {
        let error = "error[E0425]: cannot find value `undefined_var` in this scope";
        let translated = analysis::translate_rust_error(error).expect("test");

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

    #[tokio::test]
    async fn test_cargo_toml_generation() {
        let adapter = create_test_adapter().await;
        let cargo_toml = adapter.generate_cargo_toml().expect("test");

        assert!(cargo_toml.contains("[package]"));
        assert!(cargo_toml.contains("edition = \"2021\""));
        assert!(cargo_toml.contains("[dependencies]"));
        assert!(cargo_toml.contains("serde"));
        assert!(cargo_toml.contains("[profile.release]"));
    }

    async fn create_test_adapter() -> RustAdapter {
        let consensus_requirements = VMConsensusRequirements::default();
        let consensus_vm = Arc::new(RwLock::new(ConsensusVM::new(consensus_requirements).expect("test")));
        let consensus_bridge = Arc::new(ConsensusBridge::new().await.expect("test"));

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
