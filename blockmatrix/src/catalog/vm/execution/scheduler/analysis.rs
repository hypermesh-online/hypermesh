// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Resource analysis, code complexity, and execution duration estimation.

use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;

use crate::assets::core::AssetType;
use super::{ExecutionScheduler, CodeComplexity};

impl ExecutionScheduler {
    /// Analyze resource requirements for code execution
    pub(super) async fn analyze_resource_requirements(
        &self,
        code: &str,
        language: &str,
    ) -> Result<HashMap<AssetType, u64>> {
        let mut requirements = HashMap::new();

        match language.to_lowercase().as_str() {
            "julia" => {
                requirements.insert(AssetType::Cpu, 2);
                requirements.insert(AssetType::Memory, 1024 * 1024 * 1024);

                if code.contains("Nova") || code.contains("GPU") {
                    requirements.insert(AssetType::Gpu, 1);
                }
            },
            "python" => {
                requirements.insert(AssetType::Cpu, 1);
                requirements.insert(AssetType::Memory, 512 * 1024 * 1024);

                if code.contains("torch") || code.contains("tensorflow") {
                    requirements.insert(AssetType::Gpu, 1);
                    requirements.insert(AssetType::Memory, 2 * 1024 * 1024 * 1024);
                }
            },
            "rust" => {
                requirements.insert(AssetType::Cpu, 1);
                requirements.insert(AssetType::Memory, 256 * 1024 * 1024);
            },
            _ => {
                requirements.insert(AssetType::Cpu, 1);
                requirements.insert(AssetType::Memory, 512 * 1024 * 1024);
            }
        }

        let code_size = code.len();
        if code_size > 10000 {
            if let Some(memory) = requirements.get_mut(&AssetType::Memory) {
                *memory = (*memory as f64 * 1.5) as u64;
            }
        }

        if code.contains("file") || code.contains("read") || code.contains("write") {
            requirements.insert(AssetType::Storage, 100 * 1024 * 1024);
        }

        Ok(requirements)
    }

    /// Estimate execution duration based on code and resource requirements
    pub(super) async fn estimate_execution_duration(
        &self,
        code: &str,
        language: &str,
        required_assets: &HashMap<AssetType, u64>,
    ) -> Result<Duration> {
        let code_lines = code.lines().count();
        let code_complexity = self.analyze_code_complexity(code, language).await?;

        let base_duration = match language.to_lowercase().as_str() {
            "julia" => Duration::from_millis(100 + code_lines as u64 * 10),
            "python" => Duration::from_millis(200 + code_lines as u64 * 20),
            "rust" => Duration::from_millis(50 + code_lines as u64 * 5),
            _ => Duration::from_millis(150 + code_lines as u64 * 15),
        };

        let complexity_factor = match code_complexity {
            CodeComplexity::Low => 1.0,
            CodeComplexity::Medium => 2.0,
            CodeComplexity::High => 5.0,
            CodeComplexity::VeryHigh => 10.0,
        };

        let resource_factor = self.calculate_resource_availability_factor(required_assets).await?;

        Ok(Duration::from_nanos(
            (base_duration.as_nanos() as f64 * complexity_factor * resource_factor) as u64
        ))
    }

    /// Analyze code complexity
    pub(super) async fn analyze_code_complexity(
        &self,
        code: &str,
        language: &str,
    ) -> Result<CodeComplexity> {
        let mut complexity_score = 0;

        complexity_score += code.matches("for").count() * 2;
        complexity_score += code.matches("while").count() * 3;

        complexity_score += code.matches("if").count();
        complexity_score += code.matches("match").count() * 2;

        complexity_score += code.matches("function").count();
        complexity_score += code.matches("def ").count();
        complexity_score += code.matches("fn ").count();

        complexity_score += code.matches("sort").count() * 3;
        complexity_score += code.matches("search").count() * 2;
        complexity_score += code.matches("parallel").count() * 5;

        match language.to_lowercase().as_str() {
            "julia" => {
                complexity_score += code.matches("@parallel").count() * 10;
                complexity_score += code.matches("@distributed").count() * 15;
            },
            "python" => {
                complexity_score += code.matches("multiprocessing").count() * 8;
                complexity_score += code.matches("threading").count() * 6;
            },
            "rust" => {
                complexity_score += code.matches("async").count() * 4;
                complexity_score += code.matches("await").count() * 2;
            },
            _ => {}
        }

        Ok(match complexity_score {
            0..=5 => CodeComplexity::Low,
            6..=15 => CodeComplexity::Medium,
            16..=50 => CodeComplexity::High,
            _ => CodeComplexity::VeryHigh,
        })
    }

    /// Calculate resource availability factor for execution time estimation
    pub(super) async fn calculate_resource_availability_factor(
        &self,
        required_assets: &HashMap<AssetType, u64>,
    ) -> Result<f64> {
        let resource_tracker = self.resource_tracker.read().await;
        let mut availability_factor = 1.0;

        for (asset_type, required_amount) in required_assets {
            let available_amount = match asset_type {
                AssetType::Cpu => resource_tracker.available_cpu_cores as u64,
                AssetType::Memory => resource_tracker.available_memory,
                AssetType::Gpu => resource_tracker.available_gpu_cores as u64,
                AssetType::Storage => resource_tracker.available_storage,
                AssetType::Network => resource_tracker.available_bandwidth,
                AssetType::Container => 100,
                AssetType::Economic => 1000,
                AssetType::VirtualMachine => 10,
                AssetType::Library => 1000,
            };

            if available_amount > 0 {
                let utilization = *required_amount as f64 / available_amount as f64;
                if utilization > 0.8 {
                    availability_factor *= 1.0 + (utilization - 0.8) * 3.0;
                }
            } else {
                availability_factor *= 5.0;
            }
        }

        Ok(availability_factor)
    }
}
