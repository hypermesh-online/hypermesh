// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Julia language runtime implementation for the consensus VM.

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use std::process::{Command, Stdio};
use anyhow::{Result, anyhow};

use crate::assets::core::AssetManager;
use super::{
    LanguageRuntime, LanguageExecutionResult, LanguageCapabilities,
    ResourceUsage, MemoryModel, ExecutionModel, SandboxConfig,
    AssetId, AssetHandle,
};
use super::super::context::ExecutionContext;
use super::super::MemoryUsagePattern;

/// Julia language runtime implementation
pub struct JuliaLanguageRuntime {
    _asset_manager: Arc<AssetManager>,
    sandbox_config: SandboxConfig,
    julia_binary_path: String,
}

impl JuliaLanguageRuntime {
    pub fn new(
        asset_manager: Arc<AssetManager>,
        sandbox_config: SandboxConfig,
    ) -> Result<Self> {
        let julia_binary_path = which::which("julia")
            .map_err(|_| anyhow!("Julia binary not found in PATH"))?
            .to_string_lossy()
            .to_string();

        Ok(Self {
            _asset_manager: asset_manager,
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
        _context: &ExecutionContext,
        _asset_allocations: &HashMap<AssetId, AssetHandle>,
    ) -> Result<LanguageExecutionResult> {
        let start_time = Instant::now();

        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path().to_string_lossy().to_string();
        std::fs::write(&temp_path, code)?;

        let mut cmd = Command::new(&self.julia_binary_path);
        cmd.arg(&temp_path)
           .stdin(Stdio::null())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        if self.sandbox_config.enable_network_isolation {
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
            resource_usage: ResourceUsage::default(),
            memory_pattern: MemoryUsagePattern {
                peak_usage: 1024 * 1024 * 10,
                average_usage: 1024 * 1024 * 5,
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
            version: "1.9.0".to_string(),
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
        Ok(())
    }

    async fn get_resource_usage(&self) -> Result<ResourceUsage> {
        Ok(ResourceUsage::default())
    }
}
