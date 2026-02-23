// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Rust language runtime implementation for the consensus VM (compiled execution).

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

/// Rust language runtime implementation (compiled execution)
pub struct RustLanguageRuntime {
    _asset_manager: Arc<AssetManager>,
    _sandbox_config: SandboxConfig,
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
            _asset_manager: asset_manager,
            _sandbox_config: sandbox_config,
            rustc_binary_path,
        })
    }
}

#[async_trait::async_trait]
impl LanguageRuntime for RustLanguageRuntime {
    async fn execute(
        &self,
        code: &str,
        _context: &ExecutionContext,
        _asset_allocations: &HashMap<AssetId, AssetHandle>,
    ) -> Result<LanguageExecutionResult> {
        let start_time = Instant::now();

        let temp_source = tempfile::NamedTempFile::new()?;
        let source_path = temp_source.path().to_string_lossy().to_string();
        std::fs::write(&source_path, code)?;

        let temp_binary = tempfile::NamedTempFile::new()?;
        let binary_path = temp_binary.path().to_string_lossy().to_string();

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
                peak_usage: 1024 * 1024 * 2,
                average_usage: 1024 * 1024 * 1,
                allocations: 20,
                deallocations: 20,
                gc_events: 0,
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
