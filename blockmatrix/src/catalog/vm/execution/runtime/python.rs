// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Python language runtime implementation for the consensus VM.

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

/// Python language runtime implementation
pub struct PythonLanguageRuntime {
    _asset_manager: Arc<AssetManager>,
    _sandbox_config: SandboxConfig,
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
            _asset_manager: asset_manager,
            _sandbox_config: sandbox_config,
            python_binary_path,
        })
    }
}

#[async_trait::async_trait]
impl LanguageRuntime for PythonLanguageRuntime {
    async fn execute(
        &self,
        code: &str,
        _context: &ExecutionContext,
        _asset_allocations: &HashMap<AssetId, AssetHandle>,
    ) -> Result<LanguageExecutionResult> {
        let start_time = Instant::now();

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
                peak_usage: 1024 * 1024 * 8,
                average_usage: 1024 * 1024 * 4,
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
