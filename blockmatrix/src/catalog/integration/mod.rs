//! Catalog Integration Module - Bridge between Catalog and HyperMesh systems
//!
//! This module provides the integration layer between the Catalog system
//! and HyperMesh's VM and container runtime, enabling seamless deployment
//! of Catalog assets across the distributed computing platform.

pub mod hypermesh_bridge;

pub use hypermesh_bridge::{
    CatalogHyperMeshBridge, CatalogDeploymentSpec, CatalogDeploymentResult,
    CatalogAssetType, DeploymentStrategy, BridgeConfiguration,
    VMDeploymentConfig, ContainerDeploymentConfig, DeploymentInfo,
};

/// Integration utilities and helpers
pub mod utils {
    use super::*;
    use anyhow::Result;
    
    /// Convert Catalog asset to appropriate deployment strategy
    pub fn recommend_deployment_strategy(
        asset: &CatalogAssetType,
    ) -> DeploymentStrategy {
        match asset {
            CatalogAssetType::JuliaScript { .. } => {
                DeploymentStrategy::VMExecution {
                    vm_config: VMDeploymentConfig {
                        language_runtime: "julia".to_string(),
                        execution_timeout: std::time::Duration::from_secs(3600),
                        memory_limit: 2 * 1024 * 1024 * 1024, // 2GB
                        cpu_limit: 4,
                        enable_gpu: false,
                        environment_variables: std::collections::HashMap::new(),
                    },
                }
            },
            CatalogAssetType::PythonApp { .. } => {
                DeploymentStrategy::VMExecution {
                    vm_config: VMDeploymentConfig {
                        language_runtime: "python".to_string(),
                        execution_timeout: std::time::Duration::from_secs(1800),
                        memory_limit: 1024 * 1024 * 1024, // 1GB
                        cpu_limit: 2,
                        enable_gpu: false,
                        environment_variables: std::collections::HashMap::new(),
                    },
                }
            },
            CatalogAssetType::ContainerImage { .. } => {
                DeploymentStrategy::Container {
                    container_config: ContainerDeploymentConfig {
                        base_image: "ubuntu:20.04".to_string(),
                        ports: vec![],
                        volumes: vec![],
                        environment_variables: std::collections::HashMap::new(),
                        command: vec![],
                        args: vec![],
                    },
                }
            },
            CatalogAssetType::DataPipeline { .. } => {
                DeploymentStrategy::Hybrid {
                    vm_config: VMDeploymentConfig {
                        language_runtime: "python".to_string(),
                        execution_timeout: std::time::Duration::from_secs(7200),
                        memory_limit: 4 * 1024 * 1024 * 1024, // 4GB
                        cpu_limit: 8,
                        enable_gpu: false,
                        environment_variables: std::collections::HashMap::new(),
                    },
                    container_config: ContainerDeploymentConfig {
                        base_image: "python:3.9".to_string(),
                        ports: vec![],
                        volumes: vec![],
                        environment_variables: std::collections::HashMap::new(),
                        command: vec!["python".to_string()],
                        args: vec![],
                    },
                }
            },
            _ => {
                DeploymentStrategy::VMExecution {
                    vm_config: VMDeploymentConfig {
                        language_runtime: "python".to_string(),
                        execution_timeout: std::time::Duration::from_secs(1800),
                        memory_limit: 1024 * 1024 * 1024,
                        cpu_limit: 2,
                        enable_gpu: false,
                        environment_variables: std::collections::HashMap::new(),
                    },
                }
            }
        }
    }
    
    /// Estimate resource requirements for Catalog asset
    pub fn estimate_resource_requirements(
        asset: &CatalogAssetType,
    ) -> hypermesh_bridge::CatalogResourceRequirements {
        match asset {
            CatalogAssetType::JuliaScript { code, .. } => {
                // Estimate based on code complexity
                let complexity_factor = estimate_code_complexity(code);
                hypermesh_bridge::CatalogResourceRequirements {
                    cpu_cores: Some((2.0 * complexity_factor) as u32),
                    memory_mb: Some((1024.0 * complexity_factor) as u64),
                    storage_gb: Some(1),
                    gpu_count: if code.contains("GPU") || code.contains("Nova") { Some(1) } else { None },
                    network_bandwidth_mbps: Some(10),
                    custom_resources: std::collections::HashMap::new(),
                }
            },
            CatalogAssetType::PythonApp { code, .. } => {
                let complexity_factor = estimate_code_complexity(code);
                hypermesh_bridge::CatalogResourceRequirements {
                    cpu_cores: Some((1.0 * complexity_factor) as u32),
                    memory_mb: Some((512.0 * complexity_factor) as u64),
                    storage_gb: Some(1),
                    gpu_count: if code.contains("torch") || code.contains("tensorflow") { Some(1) } else { None },
                    network_bandwidth_mbps: Some(5),
                    custom_resources: std::collections::HashMap::new(),
                }
            },
            CatalogAssetType::ContainerImage { .. } => {
                hypermesh_bridge::CatalogResourceRequirements {
                    cpu_cores: Some(2),
                    memory_mb: Some(1024),
                    storage_gb: Some(5),
                    gpu_count: None,
                    network_bandwidth_mbps: Some(10),
                    custom_resources: std::collections::HashMap::new(),
                }
            },
            CatalogAssetType::DataPipeline { stages, .. } => {
                // Scale based on pipeline complexity
                let stage_count = stages.len() as f64;
                hypermesh_bridge::CatalogResourceRequirements {
                    cpu_cores: Some((4.0 * stage_count.sqrt()) as u32),
                    memory_mb: Some((2048.0 * stage_count.sqrt()) as u64),
                    storage_gb: Some((10.0 * stage_count) as u64),
                    gpu_count: None,
                    network_bandwidth_mbps: Some((50.0 * stage_count.sqrt()) as u64),
                    custom_resources: std::collections::HashMap::new(),
                }
            },
            _ => {
                // Default conservative estimates
                hypermesh_bridge::CatalogResourceRequirements {
                    cpu_cores: Some(2),
                    memory_mb: Some(1024),
                    storage_gb: Some(1),
                    gpu_count: None,
                    network_bandwidth_mbps: Some(10),
                    custom_resources: std::collections::HashMap::new(),
                }
            }
        }
    }
    
    /// Estimate code complexity factor (1.0 = simple, 5.0 = very complex)
    fn estimate_code_complexity(code: &str) -> f64 {
        let mut complexity = 1.0;
        
        // Count complexity indicators
        let lines = code.lines().count();
        complexity += (lines as f64 / 100.0).min(2.0); // Max +2.0 for lines
        
        // Count loops and conditionals
        let loops = code.matches("for ").count() + code.matches("while ").count();
        complexity += (loops as f64 * 0.2).min(1.0); // Max +1.0 for loops
        
        // Count function definitions
        let functions = code.matches("function ").count() + code.matches("def ").count();
        complexity += (functions as f64 * 0.1).min(0.5); // Max +0.5 for functions
        
        // Check for parallel/concurrent code
        if code.contains("parallel") || code.contains("concurrent") || code.contains("async") {
            complexity += 1.0;
        }
        
        // Check for heavy computational libraries
        if code.contains("numpy") || code.contains("scipy") || code.contains("pandas") ||
           code.contains("LinearAlgebra") || code.contains("Statistics") {
            complexity += 0.5;
        }
        
        complexity.min(5.0) // Cap at 5.0
    }
}

/// Catalog integration tests
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_deployment_strategy_recommendation() {
        let julia_script = CatalogAssetType::JuliaScript {
            code: "println(\"Hello, World!\")".to_string(),
            dependencies: vec![],
            entry_point: "main".to_string(),
        };
        
        let strategy = utils::recommend_deployment_strategy(&julia_script);
        assert!(matches!(strategy, DeploymentStrategy::VMExecution { .. }));
    }
    
    #[test]
    fn test_resource_estimation() {
        let python_app = CatalogAssetType::PythonApp {
            code: "import torch\nprint('AI model')".to_string(),
            requirements_txt: "torch>=1.0".to_string(),
            entry_point: "main.py".to_string(),
        };
        
        let requirements = utils::estimate_resource_requirements(&python_app);
        assert!(requirements.gpu_count.is_some()); // Should detect GPU need from torch
        assert!(requirements.cpu_cores.is_some());
        assert!(requirements.memory_mb.is_some());
    }
    
    #[test]
    fn test_code_complexity_estimation() {
        let simple_code = "print('hello')";
        let complex_code = r#"
            import numpy as np
            import torch
            
            def complex_function():
                for i in range(1000):
                    for j in range(1000):
                        if i > j:
                            result = np.parallel.compute()
                return result
        "#;
        
        let simple_complexity = utils::estimate_code_complexity(simple_code);
        let complex_complexity = utils::estimate_code_complexity(complex_code);
        
        assert!(complex_complexity > simple_complexity);
        assert!(simple_complexity >= 1.0);
        assert!(complex_complexity <= 5.0);
    }
}