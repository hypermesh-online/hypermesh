// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Common test utilities for catalog tests

use catalog::assets::*;
use chrono::Utc;
use std::collections::HashMap;

/// Create a test AssetPackage with proper structure
#[allow(dead_code)]
pub fn create_test_package(name: &str, version: &str) -> AssetPackage {
    AssetPackage {
        spec: AssetSpec {
            api_version: "catalog.v1".to_string(),
            kind: "Asset".to_string(),
            metadata: PackageSpecMetadata {
                name: name.to_string(),
                version: version.to_string(),
                tags: vec!["test".to_string()],
                description: Some(format!("Test package {name}")),
                author: Some("Test Author".to_string()),
                license: Some("MIT".to_string()),
                homepage: None,
                repository: None,
                download_count: 0,
                featured: false,
                keywords: vec![],
                created: None,
                updated: None,
            },
            spec: AssetSpecification {
                asset_type: "lua-script".to_string(),
                content: AssetContent {
                    main: "main.lua".to_string(),
                    files: vec![],
                    inline: Some("print('test')".to_string()),
                    binary: vec![],
                    templates: vec![],
                },
                security: AssetSecurity {
                    state_proof_required: false,
                    certificate_pinning: false,
                    hash_validation: "blake3".to_string(),
                    sandbox_level: "standard".to_string(),
                    allowed_syscalls: vec![],
                    network_access: NetworkAccess {
                        enabled: false,
                        allowed_domains: vec![],
                        allowed_ports: vec![],
                        require_tls: true,
                    },
                    file_access: FileAccess {
                        level: "read_only".to_string(),
                        allowed_paths: vec![],
                        denied_paths: vec![],
                        allow_temp: false,
                    },
                    permissions: vec![],
                },
                resources: AssetResources {
                    cpu_limit: "1000m".to_string(),
                    memory_limit: "256Mi".to_string(),
                    execution_timeout: "30s".to_string(),
                    storage_required: None,
                    network_bandwidth: None,
                    gpu_required: false,
                    hardware_requirements: vec![],
                },
                execution: AssetExecution {
                    delegation_strategy: "nearest_node".to_string(),
                    minimum_state_proof: 1,
                    retry_policy: "none".to_string(),
                    max_concurrent: None,
                    priority: "normal".to_string(),
                    timeout_config: TimeoutConfig {
                        execution: "30s".to_string(),
                        network: "10s".to_string(),
                        io: "5s".to_string(),
                        compilation: None,
                    },
                    scheduling: SchedulingConfig {
                        timing: "immediate".to_string(),
                        allocation_strategy: "best_fit".to_string(),
                        node_affinity: vec![],
                        anti_affinity: vec![],
                    },
                },
                dependencies: vec![],
                environment: HashMap::new(),
                config_schema: None,
            },
        },
        content: AssetContentResolved {
            main_content: "print('test')".to_string(),
            file_contents: HashMap::new(),
            binary_contents: HashMap::new(),
            template_content: HashMap::new(),
            resolved_dependencies: vec![],
        },
        validation: AssetValidationStatus {
            is_valid: true,
            validated_at: Utc::now(),
            errors: vec![],
            warnings: vec![],
            security_results: SecurityScanResults {
                security_score: 100,
                vulnerabilities: vec![],
                recommendations: vec![],
                scanned_at: Utc::now(),
            },
            dependency_results: DependencyValidationResults {
                dependencies_valid: true,
                total_dependencies: 0,
                valid_dependencies: 0,
                invalid_dependencies: vec![],
                conflicts: vec![],
                validated_at: Utc::now(),
            },
        },
        package_hash: String::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        signature: None,
    }
}
