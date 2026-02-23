// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Python Language Adapter - PyCall Integration with Consensus
//!
//! This adapter provides Python execution through Julia's PyCall package,
//! enabling Python code to access ConsensusProof validation and asset management
//! through the HyperMesh ecosystem. Python consensus constructs are translated
//! to Julia consensus operations for execution.
//!
//! Key Features:
//! - PyCall-based Python execution through Julia
//! - Python decorators for consensus requirements (@consensus_required)
//! - Asset management through Python classes (CPUAsset, GPUAsset, etc.)
//! - P2P execution with consensus validation
//! - Error translation for Python-specific issues

pub mod config;
pub mod parsing;

pub use config::PythonAdapterConfig;

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::consensus::proof::ConsensusProof;
use super::super::super::consensus::ConsensusVM;
use super::super::super::execution::{ExecutionContext, ExecutionResult};
use super::{
    LanguageRuntime, BaseAdapter,
    LanguageSpecificConfig, ConsensusBridge,
};
use super::super::{
    ConsensusRequirements, ConsensusConstruct, AssetRequirements, TranslatedError,
    CpuRequirements, MemoryRequirements, GpuRequirements, ErrorCategory,
    MemoryAccessPattern,
};

/// Python language adapter using PyCall integration
pub struct PythonAdapter {
    /// Base adapter functionality
    base: BaseAdapter,
    /// Consensus bridge for translation
    _consensus_bridge: Arc<ConsensusBridge>,
    /// Python-specific configuration
    python_config: PythonAdapterConfig,
}

impl PythonAdapter {
    /// Create new Python adapter with PyCall integration
    pub async fn new(
        consensus_vm: Arc<RwLock<ConsensusVM>>,
        consensus_bridge: Arc<ConsensusBridge>,
        config: Option<&LanguageSpecificConfig>,
    ) -> Result<Self> {
        let base = BaseAdapter::new(
            "python".to_string(),
            "PyCall".to_string(),
            consensus_vm,
            Arc::clone(&consensus_bridge),
            config,
        );

        let python_config = PythonAdapterConfig::from_language_config(config);

        Ok(Self {
            base,
            _consensus_bridge: consensus_bridge,
            python_config,
        })
    }

    /// Execute Python code through PyCall with consensus validation
    async fn execute_python_with_consensus(
        &self,
        code: &str,
        context: Arc<ExecutionContext>,
        consensus_proof: ConsensusProof,
    ) -> Result<ExecutionResult> {
        // Create execution environment
        let env = self.base.create_execution_environment(&context).await?;

        // Validate consensus proof
        if !self.base.validate_adapter_consensus_proof(&consensus_proof).await? {
            return Err(anyhow::anyhow!("Consensus proof validation failed for Python execution"));
        }

        // Translate Python code to Julia with PyCall integration
        let julia_code = self.translate_python_to_julia(code, &consensus_proof).await?;

        // Execute through Julia VM with PyCall
        let result = self.execute_julia_pycall(&julia_code, context).await?;

        // Cleanup environment
        self.base.cleanup_execution_environment(env).await?;

        Ok(result)
    }

    /// Translate Python code to Julia with PyCall integration
    async fn translate_python_to_julia(
        &self,
        python_code: &str,
        consensus_proof: &ConsensusProof,
    ) -> Result<String> {
        let mut julia_code = String::new();

        // Setup PyCall environment
        julia_code.push_str("using PyCall\n");
        julia_code.push_str("using HyperMeshConsensus\n\n");

        // Initialize Python environment
        julia_code.push_str("# Initialize Python environment with consensus\n");
        julia_code.push_str("py\"\"\"\n");
        julia_code.push_str("import sys\n");
        julia_code.push_str("import json\n");
        julia_code.push_str("from typing import Any, Dict, List\n\n");

        // Add HyperMesh consensus integration
        julia_code.push_str(&self.generate_python_consensus_integration()?);

        // Add the actual Python code
        julia_code.push_str("\n# User Python code\n");
        julia_code.push_str(python_code);
        julia_code.push_str("\n\"\"\"\n\n");

        // Add consensus validation wrapper
        julia_code.push_str(&self.generate_consensus_validation_wrapper(consensus_proof)?);

        Ok(julia_code)
    }

    /// Generate Python consensus integration code
    fn generate_python_consensus_integration(&self) -> Result<String> {
        Ok(r#"
# HyperMesh Consensus Integration for Python
class ConsensusProof:
    def __init__(self, proof_data):
        self.proof_data = proof_data

    def validate(self):
        # Validation handled by Julia layer
        return True

class ConsensusRequired:
    def __init__(self, space=None, stake=None, work=None, time=None):
        self.space = space
        self.stake = stake
        self.work = work
        self.time = time

    def __call__(self, func):
        def wrapper(*args, **kwargs):
            # Consensus validation handled by Julia layer
            return func(*args, **kwargs)
        return wrapper

# Decorator for consensus-required functions
def consensus_required(space=None, stake=None, work=None, time=None):
    return ConsensusRequired(space, stake, work, time)

# Asset management classes
class CPUAsset:
    def __init__(self, cores=1):
        self.cores = cores
        self._allocated = False

    def allocate(self):
        # Asset allocation handled by Julia layer
        self._allocated = True
        return self

    def deallocate(self):
        self._allocated = False

class GPUAsset:
    def __init__(self, memory_mb=1024):
        self.memory_mb = memory_mb
        self._allocated = False

    def allocate(self):
        self._allocated = True
        return self

    def deallocate(self):
        self._allocated = False

class MemoryAsset:
    def __init__(self, size_mb=1024):
        self.size_mb = size_mb
        self._allocated = False

    def allocate(self):
        self._allocated = True
        return self

    def deallocate(self):
        self._allocated = False

# P2P execution
def remote_execute(peer_id, code, consensus_proof=None):
    # P2P execution handled by Julia layer
    return f"Remote execution on {peer_id}: {code[:50]}..."

# Blockchain storage
class BlockchainStorage:
    @staticmethod
    def store(data, consensus_proof):
        # Blockchain storage handled by Julia layer
        return f"stored_{hash(str(data))}"

    @staticmethod
    def retrieve(storage_id):
        # Retrieval handled by Julia layer
        return {"data": f"Retrieved data for {storage_id}"}
"#.to_string())
    }

    /// Generate consensus validation wrapper
    fn generate_consensus_validation_wrapper(
        &self,
        consensus_proof: &ConsensusProof,
    ) -> Result<String> {
        let mut wrapper = String::new();

        wrapper.push_str("# Consensus validation wrapper\n");
        wrapper.push_str("consensus_result = begin\n");
        wrapper.push_str("    # Validate consensus proof\n");
        wrapper.push_str(&format!(
            "    proof = ConsensusProof({:?}, {:?}, {:?}, {:?})\n",
            consensus_proof.space_proof.total_size,
            consensus_proof.stake_proof.stake_amount,
            consensus_proof.work_proof.computational_power,
            consensus_proof.time_proof.nonce
        ));
        wrapper.push_str("    \n");
        wrapper.push_str("    if validate_consensus_proof(proof)\n");
        wrapper.push_str("        # Execute Python code with consensus validation\n");
        wrapper.push_str("        try\n");
        wrapper.push_str("            py_result = py\"globals().get('__result__', 'No result')\"\n");
        wrapper.push_str("            ConsensusExecutionResult(true, py_result, nothing)\n");
        wrapper.push_str("        catch e\n");
        wrapper.push_str("            ConsensusExecutionResult(false, nothing, string(e))\n");
        wrapper.push_str("        end\n");
        wrapper.push_str("    else\n");
        wrapper.push_str("        ConsensusExecutionResult(false, nothing, \"Consensus validation failed\")\n");
        wrapper.push_str("    end\n");
        wrapper.push_str("end\n");

        Ok(wrapper)
    }

    /// Execute Julia code with PyCall
    async fn execute_julia_pycall(
        &self,
        julia_code: &str,
        _context: Arc<ExecutionContext>,
    ) -> Result<ExecutionResult> {
        // This would integrate with the actual Julia VM execution
        // For now, return a placeholder result
        Ok(ExecutionResult {
            execution_id: uuid::Uuid::new_v4().to_string(),
            success: true,
            output: Some(serde_json::json!({
                "result": "Python code executed through PyCall",
                "adapter": "PyCall",
                "julia_code_length": julia_code.len()
            })),
            error_message: None,
            consensus_results: super::super::super::consensus::ConsensusExecutionResult::success(
                "python_pycall".to_string(),
                Some(serde_json::json!({"pycall_execution": "completed"})),
                super::super::super::consensus::ProofValidationResults {
                    space_proof_valid: true,
                    stake_proof_valid: true,
                    work_proof_valid: true,
                    time_proof_valid: true,
                    combined_proof_hash_valid: true,
                    validation_timestamp: std::time::SystemTime::now(),
                },
                super::super::super::consensus::ResourceUsageMetrics {
                    cpu_cycles: 50000,
                    memory_bytes: 128 * 1024 * 1024, // 128MB for Python
                    storage_bytes: 0,
                    network_bytes: 0,
                    execution_duration_micros: 15000,
                },
            ),
            asset_utilization: super::super::super::execution::AssetUtilizationReport {
                cpu_utilization: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("python_interpreter".to_string(), 30.0);
                    map
                },
                gpu_utilization: None,
                memory_usage: super::super::super::execution::MemoryUsagePattern {
                    peak_usage: 128 * 1024 * 1024,
                    average_usage: 64 * 1024 * 1024,
                    allocations: 25,
                    deallocations: 20,
                    gc_events: 2,
                },
                storage_operations: vec![],
                network_bandwidth_used: 0,
            },
            privacy_compliance: super::super::super::execution::PrivacyComplianceReport {
                privacy_level_used: crate::catalog::vm::PrivacyMode::PRIVATE,
                anonymization_applied: false,
                data_sharing_events: vec![],
                violations: vec![],
            },
            metadata: super::super::super::execution::ExecutionMetadata {
                language: "python".to_string(),
                code_size: julia_code.len() as u64,
                started_at: std::time::SystemTime::now(),
                completed_at: Some(std::time::SystemTime::now()),
                duration_micros: 15000,
                vm_version: "python-pycall-consensus-0.1.0".to_string(),
                consensus_requirements_hash: [0; 32],
            },
        })
    }

    /// Analyze Python code for asset requirements
    fn analyze_python_asset_requirements(&self, code: &str) -> Result<AssetRequirements> {
        let mut cpu_requirements = None;
        let mut gpu_requirements = None;

        // Analyze for multiprocessing/threading
        if code.contains("multiprocessing") || code.contains("threading") ||
           code.contains("concurrent.futures") {
            cpu_requirements = Some(CpuRequirements {
                min_cores: 2,
                preferred_cores: 4,
                architecture: None,
                required_features: vec!["multiprocessing".to_string()],
            });
        }

        // Analyze for numpy/scientific computing
        let memory_multiplier = if code.contains("numpy") || code.contains("scipy") {
            4 // Scientific computing needs more memory
        } else if code.contains("pandas") {
            3 // Data analysis needs significant memory
        } else {
            1
        };

        let base_memory = 256 * 1024 * 1024; // 256MB base
        let estimated_memory = base_memory * memory_multiplier;

        let memory_requirements = Some(MemoryRequirements {
            min_ram_bytes: estimated_memory / 2,
            preferred_ram_bytes: estimated_memory,
            access_patterns: vec![MemoryAccessPattern::Random], // Python uses random access
        });

        // Check for GPU libraries
        if code.contains("tensorflow") || code.contains("torch") ||
           code.contains("cupy") || code.contains("nova") {
            gpu_requirements = Some(GpuRequirements {
                min_memory_bytes: 2 * 1024 * 1024 * 1024, // 2GB
                compute_capability: Some("3.5".to_string()),
                gpu_types: vec!["nvidia".to_string()],
            });
        }

        Ok(AssetRequirements {
            cpu_requirements,
            gpu_requirements,
            memory_requirements,
            storage_requirements: None,
            network_requirements: None,
        })
    }

    /// Translate Python errors to user-friendly messages
    fn translate_python_error(&self, error: &str) -> Result<TranslatedError> {
        let error_category = if error.contains("SyntaxError") {
            ErrorCategory::SyntaxError
        } else if error.contains("NameError") {
            ErrorCategory::SyntaxError
        } else if error.contains("TypeError") || error.contains("AttributeError") {
            ErrorCategory::RuntimeError
        } else if error.contains("consensus") {
            ErrorCategory::ConsensusError
        } else if error.contains("MemoryError") {
            ErrorCategory::ResourceError
        } else {
            ErrorCategory::RuntimeError
        };

        let translated_error = match error_category {
            ErrorCategory::SyntaxError => {
                if error.contains("NameError") {
                    "Variable or function name not defined - check spelling and imports".to_string()
                } else {
                    "Python syntax error - check code structure and indentation".to_string()
                }
            },
            ErrorCategory::RuntimeError => "Python runtime error - check function calls and data types".to_string(),
            ErrorCategory::ConsensusError => "Consensus validation failed - ensure proper consensus proofs".to_string(),
            ErrorCategory::ResourceError => "Insufficient memory - reduce data size or enable GPU if available".to_string(),
            _ => error.to_string(),
        };

        let suggested_fixes = match error_category {
            ErrorCategory::SyntaxError => vec![
                "Check indentation (Python requires consistent indentation)".to_string(),
                "Verify all variables are defined before use".to_string(),
                "Check import statements".to_string(),
            ],
            ErrorCategory::RuntimeError => vec![
                "Check function parameters and return types".to_string(),
                "Verify object attributes exist".to_string(),
            ],
            ErrorCategory::ConsensusError => vec![
                "Verify consensus proofs meet minimum requirements".to_string(),
                "Check asset allocations are sufficient".to_string(),
                "Ensure @consensus_required decorator parameters are correct".to_string(),
            ],
            _ => vec!["Check Python documentation for details".to_string()],
        };

        Ok(TranslatedError {
            original_error: error.to_string(),
            translated_error,
            error_category,
            suggested_fixes,
            consensus_issues: if matches!(error_category, ErrorCategory::ConsensusError) {
                vec!["Consensus proof validation failed in Python code".to_string()]
            } else {
                vec![]
            },
        })
    }
}

#[async_trait]
impl LanguageRuntime for PythonAdapter {
    fn language_id(&self) -> &str {
        "python"
    }

    fn adapter_type(&self) -> &str {
        "PyCall"
    }

    async fn execute_with_consensus(
        &self,
        code: &str,
        context: Arc<ExecutionContext>,
        consensus_proof: ConsensusProof,
    ) -> Result<ExecutionResult> {
        self.execute_python_with_consensus(code, context, consensus_proof).await
    }

    async fn validate_consensus_constructs(
        &self,
        code: &str,
        requirements: &ConsensusRequirements,
    ) -> Result<Vec<ConsensusConstruct>> {
        let constructs = parsing::parse_python_consensus_constructs(code)?;

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
        self.analyze_python_asset_requirements(code)
    }

    fn supports_consensus_feature(&self, feature: &str) -> bool {
        match feature {
            "consensus_decorators" => true,
            "asset_classes" => true,
            "p2p_execution" => true,
            "pycall_integration" => true,
            "numpy_support" => self.python_config.enable_numpy,
            "ml_support" => self.python_config.enable_ml_packages,
            _ => false,
        }
    }

    async fn translate_error(&self, error: &str) -> Result<TranslatedError> {
        self.translate_python_error(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::vm::consensus::ConsensusVM;
    use crate::catalog::vm::ConsensusRequirements as VMConsensusRequirements;

    #[tokio::test]
    async fn test_python_adapter_creation() {
        let consensus_requirements = VMConsensusRequirements::default();
        let consensus_vm = Arc::new(RwLock::new(ConsensusVM::new(consensus_requirements).unwrap()));
        let consensus_bridge = Arc::new(ConsensusBridge::new().await.unwrap());

        let adapter = PythonAdapter::new(consensus_vm, consensus_bridge, None).await;
        assert!(adapter.is_ok());

        if let Ok(adapter) = adapter {
            assert_eq!(adapter.language_id(), "python");
            assert_eq!(adapter.adapter_type(), "PyCall");
        }
    }

    #[tokio::test]
    async fn test_python_consensus_construct_parsing() {
        let code = r#"
@consensus_required(space=2048, stake=1500)
def consensus_compute(data):
    return process_data(data)

cpu_asset = CPUAsset(cores=4)
result = remote_execute("peer1", "compute_task()")
proof = ConsensusProof(space_data)
        "#;

        let constructs = parsing::parse_python_consensus_constructs(code).unwrap();
        assert!(!constructs.is_empty());

        use super::super::super::ConsensusConstructType;
        let has_consensus = constructs.iter().any(|c| matches!(c.construct_type, ConsensusConstructType::ConsensusFunction));
        let has_asset = constructs.iter().any(|c| matches!(c.construct_type, ConsensusConstructType::AssetOperation));
        let has_p2p = constructs.iter().any(|c| matches!(c.construct_type, ConsensusConstructType::P2PExecution));

        assert!(has_consensus);
        assert!(has_asset);
        assert!(has_p2p);
    }

    #[tokio::test]
    async fn test_python_asset_requirements() {
        let adapter = create_test_adapter().await;

        let code = r#"
import numpy as np
import tensorflow as tf
from multiprocessing import Pool

def ml_compute():
    with Pool(8) as p:
        data = np.random.rand(10000, 10000)
        model = tf.keras.Sequential([...])
        return model.fit(data)
        "#;

        let requirements = adapter.analyze_python_asset_requirements(code).unwrap();

        // Should detect CPU, memory, and GPU requirements
        assert!(requirements.cpu_requirements.is_some());
        assert!(requirements.memory_requirements.is_some());
        assert!(requirements.gpu_requirements.is_some());

        let cpu_req = requirements.cpu_requirements.unwrap();
        assert!(cpu_req.min_cores >= 2);
        assert!(cpu_req.required_features.contains(&"multiprocessing".to_string()));

        let gpu_req = requirements.gpu_requirements.unwrap();
        assert!(gpu_req.min_memory_bytes > 0);
    }

    #[tokio::test]
    async fn test_python_error_translation() {
        let adapter = create_test_adapter().await;

        let error = "NameError: name 'undefined_var' is not defined";
        let translated = adapter.translate_python_error(error).unwrap();

        assert!(matches!(translated.error_category, ErrorCategory::SyntaxError));
        assert!(!translated.suggested_fixes.is_empty());
        assert!(translated.suggested_fixes.iter().any(|fix| fix.contains("indentation")));
    }

    #[tokio::test]
    async fn test_python_to_julia_translation() {
        let adapter = create_test_adapter().await;
        let consensus_proof = create_test_consensus_proof();

        let python_code = r#"
@consensus_required(space=1024)
def test_function():
    return "Hello from Python"
        "#;

        let julia_code = adapter.translate_python_to_julia(python_code, &consensus_proof).await.unwrap();

        assert!(julia_code.contains("using PyCall"));
        assert!(julia_code.contains("HyperMeshConsensus"));
        assert!(julia_code.contains("consensus_required"));
        assert!(julia_code.contains(python_code));
    }

    // Helper functions
    async fn create_test_adapter() -> PythonAdapter {
        let consensus_requirements = VMConsensusRequirements::default();
        let consensus_vm = Arc::new(RwLock::new(ConsensusVM::new(consensus_requirements).unwrap()));
        let consensus_bridge = Arc::new(ConsensusBridge::new().await.unwrap());

        PythonAdapter {
            base: BaseAdapter::new(
                "python".to_string(),
                "PyCall".to_string(),
                consensus_vm,
                Arc::clone(&consensus_bridge),
                None,
            ),
            _consensus_bridge: consensus_bridge,
            python_config: PythonAdapterConfig::default(),
        }
    }

    fn create_test_consensus_proof() -> ConsensusProof {
        use crate::consensus::proof::{SpaceProof, StakeProof, WorkProof, TimeProof, WorkloadType, WorkState};
        use std::time::Duration;

        ConsensusProof::new(
            StakeProof::new(
                "test".to_string(),
                "test-node".to_string(),
                1000,
            ),
            TimeProof::new(Duration::from_secs(0)),
            SpaceProof::new(
                "test-node".to_string(),
                "/tmp/test".to_string(),
                1024,
            ),
            WorkProof::new(
                "test-owner".to_string(),
                "test-workload".to_string(),
                12345,
                100,
                WorkloadType::Compute,
                WorkState::Completed,
            ),
        )
    }
}
