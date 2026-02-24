// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog-HyperMesh Integration Bridge
//!
//! This module bridges the Catalog system with HyperMesh's container runtime,
//! enabling Catalog assets to be deployed as containers with full consensus
//! validation and resource management. Code execution is delegated to remote
//! HyperMesh nodes via STOQ protocol.

pub mod types;
pub mod config;
pub mod monitoring;
pub mod operations;

// Re-export all public types
pub use types::*;

// Re-export the bridge
pub use operations::CatalogHyperMeshBridge;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    #[test]
    fn test_catalog_asset_types() {
    }

    #[test]
    fn test_deployment_strategies() {
        let vm_config = VMDeploymentConfig {
            language_runtime: "julia".to_string(),
            execution_timeout: Duration::from_secs(300),
            memory_limit: 1024 * 1024 * 1024,
            cpu_limit: 2,
            enable_gpu: false,
            environment_variables: HashMap::new(),
        };

        let strategy = DeploymentStrategy::VMExecution { vm_config };
        assert!(matches!(strategy, DeploymentStrategy::VMExecution { .. }));
    }

    #[tokio::test]
    async fn test_bridge_creation() {
        let config = BridgeConfiguration::default();
        assert!(config.enable_vm_deployments);
        assert!(config.enable_container_deployments);
        assert_eq!(config.max_concurrent_deployments, 50);
    }
}
