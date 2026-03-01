// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Library types - conversion and validation between LibraryAssetPackage and registry types

use std::sync::Arc;

use super::metadata::*;
use crate::registry::AssetTypeDefinition;

// Conversion implementations for Asset Registry integration
impl LibraryAssetPackage {
    /// Convert to AssetTypeDefinition (new registry architecture)
    pub fn to_asset_type_definition(&self) -> Result<AssetTypeDefinition, anyhow::Error> {
        use blockmatrix::assets::ConsensusProof;
        use blockmatrix::consensus::proof_of_state_integration::{
            SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
        };
        use serde_json::json;
        use std::time::Duration;

        let stake_proof = StakeProof::new(
            self.author().unwrap_or("unknown").to_string(),
            self.id.to_string(),
            1000,
        );

        let space_proof = SpaceProof::new(
            "catalog-node".to_string(),
            format!("/catalog/{}", self.id),
            self.size,
        );

        let work_proof = WorkProof::new(
            self.author().unwrap_or("unknown").to_string(),
            format!("package-{}", self.id),
            chrono::Utc::now().timestamp() as u64,
            100,
            WorkloadType::Compute,
            WorkState::Completed,
        );

        let time_proof = TimeProof::new(Duration::from_secs(
            self.metadata
                .as_ref()
                .map(|m| (m.modified - m.created) as u64)
                .unwrap_or(0),
        ));

        let consensus_proof = ConsensusProof::new(stake_proof, time_proof, space_proof, work_proof);

        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "version": { "type": "string" },
                "asset_type": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["name", "version", "asset_type"]
        });

        let mut type_def = AssetTypeDefinition::new(self.name.clone(), schema, consensus_proof);

        type_def.metadata.version = self.version.clone();
        type_def.metadata.author = self.author().map(|s| s.to_string());
        type_def.metadata.description = self.description.clone();
        type_def.metadata.tags = self.tags();
        type_def.metadata.license = self.license().map(|s| s.to_string());

        if let Some(metadata) = &self.metadata {
            type_def.metadata.created_at = chrono::DateTime::from_timestamp(metadata.created, 0)
                .unwrap_or_else(chrono::Utc::now);
            type_def.metadata.updated_at = chrono::DateTime::from_timestamp(metadata.modified, 0)
                .unwrap_or_else(chrono::Utc::now);
        }

        for dep in self.dependencies() {
            type_def.add_dependency(dep.name.to_string());
        }

        Ok(type_def)
    }

    /// Create LibraryAssetPackage from AssetTypeDefinition (new registry architecture)
    pub fn from_asset_type_definition(type_def: &AssetTypeDefinition) -> Self {
        Self {
            id: Arc::from(type_def.asset_id.to_string().as_str()),
            name: type_def.type_name.clone(),
            version: type_def.metadata.version.clone(),
            description: type_def.metadata.description.clone(),
            asset_type: "library".to_string(),
            size: 0,
            hash: type_def.asset_id.to_string(),
            content: type_def.schema.to_string(),
            metadata: Some(PackageMetadata {
                name: Arc::from(type_def.type_name.as_str()),
                version: Arc::from(type_def.metadata.version.as_str()),
                description: type_def
                    .metadata
                    .description
                    .as_ref()
                    .map(|d| Arc::from(d.as_str())),
                author: type_def
                    .metadata
                    .author
                    .as_ref()
                    .map(|a| Arc::from(a.as_str())),
                license: type_def
                    .metadata
                    .license
                    .as_ref()
                    .map(|l| Arc::from(l.as_str())),
                tags: type_def
                    .metadata
                    .tags
                    .iter()
                    .map(|t| Arc::from(t.as_str()))
                    .collect(),
                keywords: Arc::new([]),
                created: type_def.metadata.created_at.timestamp(),
                modified: type_def.metadata.updated_at.timestamp(),
            }),
            spec: None,
            content_refs: None,
            validation: None,
        }
    }

    /// Convert to Catalog AssetPackage for operations requiring full package structure
    pub fn to_asset_package(&self) -> crate::assets::AssetPackage {
        use crate::assets::*;
        use chrono::Utc;

        AssetPackage {
            spec: AssetSpec {
                api_version: "v1".to_string(),
                kind: self.asset_type.clone(),
                metadata: AssetMetadata {
                    name: self.name.clone(),
                    version: self.version.clone(),
                    tags: self.tags(),
                    description: self.description.clone(),
                    author: self.author().map(|s| s.to_string()),
                    license: self.license().map(|s| s.to_string()),
                    homepage: None,
                    repository: None,
                    download_count: 0,
                    featured: false,
                    keywords: vec![],
                    created: None,
                    updated: None,
                },
                spec: AssetSpecification {
                    asset_type: self.asset_type.clone(),
                    content: AssetContent {
                        main: self.content.clone(),
                        files: vec![],
                        inline: None,
                        binary: vec![],
                        templates: vec![],
                    },
                    security: AssetSecurity {
                        consensus_required: self
                            .spec
                            .as_ref()
                            .map(|s| s.security.consensus_required)
                            .unwrap_or(false),
                        certificate_pinning: false,
                        hash_validation: "blake3".to_string(),
                        sandbox_level: self
                            .spec
                            .as_ref()
                            .map(|s| match s.security.sandbox_level {
                                SandboxLevel::None => "none".to_string(),
                                SandboxLevel::Standard => "standard".to_string(),
                                SandboxLevel::Strict => "strict".to_string(),
                            })
                            .unwrap_or_else(|| "standard".to_string()),
                        allowed_syscalls: vec![],
                        network_access: NetworkAccess {
                            enabled: self
                                .spec
                                .as_ref()
                                .map(|s| s.security.network_access)
                                .unwrap_or(false),
                            allowed_domains: vec![],
                            allowed_ports: vec![],
                            require_tls: true,
                        },
                        file_access: FileAccess {
                            level: self
                                .spec
                                .as_ref()
                                .map(|s| match s.security.filesystem_access {
                                    FilesystemAccess::None => "none".to_string(),
                                    FilesystemAccess::ReadOnly => "read_only".to_string(),
                                    FilesystemAccess::ReadWrite => "read_write".to_string(),
                                })
                                .unwrap_or_else(|| "none".to_string()),
                            allowed_paths: vec![],
                            denied_paths: vec![],
                            allow_temp: false,
                        },
                        permissions: vec![],
                    },
                    resources: AssetResources {
                        cpu_limit: self
                            .spec
                            .as_ref()
                            .map(|s| format!("{}m", s.resources.cpu_millicores))
                            .unwrap_or_else(|| "100m".to_string()),
                        memory_limit: self
                            .spec
                            .as_ref()
                            .map(|s| format!("{}Mi", s.resources.memory_mb))
                            .unwrap_or_else(|| "128Mi".to_string()),
                        execution_timeout: self
                            .spec
                            .as_ref()
                            .map(|s| format!("{}s", s.resources.timeout_seconds))
                            .unwrap_or_else(|| "30s".to_string()),
                        storage_required: self
                            .spec
                            .as_ref()
                            .and_then(|s| s.resources.storage_mb.map(|mb| format!("{mb}Mi"))),
                        network_bandwidth: self.spec.as_ref().and_then(|s| {
                            s.resources.network_mbps.map(|mbps| format!("{mbps}Mbps"))
                        }),
                        gpu_required: self
                            .spec
                            .as_ref()
                            .map(|s| s.resources.gpu_required)
                            .unwrap_or(false),
                        hardware_requirements: vec![],
                    },
                    execution: AssetExecution {
                        delegation_strategy: self
                            .spec
                            .as_ref()
                            .map(|s| match s.execution.strategy {
                                ExecutionStrategy::NearestNode => "nearest".to_string(),
                                ExecutionStrategy::RandomNode => "random".to_string(),
                                ExecutionStrategy::SpecificNode => "specific".to_string(),
                                ExecutionStrategy::LoadBalanced => "loadbalanced".to_string(),
                            })
                            .unwrap_or_else(|| "loadbalanced".to_string()),
                        minimum_consensus: self
                            .spec
                            .as_ref()
                            .map(|s| s.execution.min_consensus)
                            .unwrap_or(1),
                        retry_policy: self
                            .spec
                            .as_ref()
                            .map(|s| {
                                if s.execution.retry_policy.exponential_backoff {
                                    format!(
                                        "exponential:{}:{}",
                                        s.execution.retry_policy.max_attempts,
                                        s.execution.retry_policy.base_delay_ms
                                    )
                                } else {
                                    format!(
                                        "fixed:{}:{}",
                                        s.execution.retry_policy.max_attempts,
                                        s.execution.retry_policy.base_delay_ms
                                    )
                                }
                            })
                            .unwrap_or_else(|| "exponential:3:1000".to_string()),
                        max_concurrent: self.spec.as_ref().and_then(|s| s.execution.max_concurrent),
                        priority: self
                            .spec
                            .as_ref()
                            .map(|s| match s.execution.priority {
                                ExecutionPriority::Low => "low".to_string(),
                                ExecutionPriority::Normal => "normal".to_string(),
                                ExecutionPriority::High => "high".to_string(),
                                ExecutionPriority::Critical => "critical".to_string(),
                            })
                            .unwrap_or_else(|| "normal".to_string()),
                        timeout_config: TimeoutConfig {
                            execution: self
                                .spec
                                .as_ref()
                                .map(|s| format!("{}s", s.resources.timeout_seconds))
                                .unwrap_or_else(|| "30s".to_string()),
                            network: "10s".to_string(),
                            io: "5s".to_string(),
                            compilation: None,
                        },
                        scheduling: SchedulingConfig {
                            timing: "immediate".to_string(),
                            allocation_strategy: "balanced".to_string(),
                            node_affinity: vec![],
                            anti_affinity: vec![],
                        },
                    },
                    dependencies: self
                        .dependencies()
                        .iter()
                        .map(|d| AssetDependency {
                            name: d.name.to_string(),
                            version: d.version_constraint.to_string(),
                            optional: d.optional,
                            source: DependencySource::Registry {
                                registry: "default".to_string(),
                                namespace: None,
                            },
                            features: vec![],
                            platform: d.platform.as_ref().map(|p| p.to_string()),
                        })
                        .collect(),
                    environment: self
                        .spec
                        .as_ref()
                        .map(|s| {
                            s.environment
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    config_schema: None,
                },
            },
            content: AssetContentResolved {
                main_content: self.content.clone(),
                file_contents: std::collections::HashMap::new(),
                binary_contents: std::collections::HashMap::new(),
                template_content: std::collections::HashMap::new(),
                resolved_dependencies: vec![],
            },
            validation: AssetValidationStatus {
                is_valid: self.validation.as_ref().map(|v| v.valid).unwrap_or(false),
                validated_at: Utc::now(),
                errors: vec![],
                warnings: vec![],
                security_results: SecurityScanResults {
                    security_score: self
                        .validation
                        .as_ref()
                        .map(|v| v.security_score)
                        .unwrap_or(0),
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
            package_hash: self.hash.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            signature: None,
        }
    }

    /// Create LibraryAssetPackage from Catalog AssetPackage
    pub fn from_asset_package(package: &crate::assets::AssetPackage) -> Self {
        Self {
            id: Arc::from(package.package_hash.as_str()),
            name: package.spec.metadata.name.clone(),
            version: package.spec.metadata.version.clone(),
            description: package.spec.metadata.description.clone(),
            asset_type: package.spec.spec.asset_type.clone(),
            size: package.content.main_content.len() as u64,
            hash: package.package_hash.clone(),
            content: package.content.main_content.clone(),
            metadata: Some(PackageMetadata {
                name: Arc::from(package.spec.metadata.name.as_str()),
                version: Arc::from(package.spec.metadata.version.as_str()),
                description: package
                    .spec
                    .metadata
                    .description
                    .as_ref()
                    .map(|d| Arc::from(d.as_str())),
                author: package
                    .spec
                    .metadata
                    .author
                    .as_ref()
                    .map(|a| Arc::from(a.as_str())),
                license: package
                    .spec
                    .metadata
                    .license
                    .as_ref()
                    .map(|l| Arc::from(l.as_str())),
                tags: package
                    .spec
                    .metadata
                    .tags
                    .iter()
                    .map(|t| Arc::from(t.as_str()))
                    .collect(),
                keywords: package
                    .spec
                    .metadata
                    .keywords
                    .iter()
                    .map(|k| Arc::from(k.as_str()))
                    .collect(),
                created: package
                    .spec
                    .metadata
                    .created
                    .map(|t| t.timestamp())
                    .unwrap_or(0),
                modified: package
                    .spec
                    .metadata
                    .updated
                    .map(|t| t.timestamp())
                    .unwrap_or(0),
            }),
            spec: None,
            content_refs: None,
            validation: if package.validation.is_valid {
                Some(ValidationStatus {
                    valid: package.validation.is_valid,
                    security_score: package.validation.security_results.security_score,
                    validated_at: package.validation.validated_at.timestamp(),
                    expires_at: package.validation.validated_at.timestamp() + 86400,
                })
            } else {
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_requirements() {
        let runtime = RuntimeRequirements {
            runtime_type: "lua".to_string(),
            version: "5.4".to_string(),
            dependencies: vec!["luasocket".to_string()],
        };
        assert_eq!(runtime.runtime_type, "lua");
        assert_eq!(runtime.version, "5.4");
        assert_eq!(runtime.dependencies.len(), 1);
    }

    #[test]
    fn test_resource_requirements_default() {
        let resources = ResourceRequirements::default();
        assert_eq!(resources.cpu_millicores, 100);
        assert_eq!(resources.memory_mb, 128);
        assert!(!resources.gpu_required);
        assert_eq!(resources.timeout_seconds, 30);
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay_ms, 1000);
        assert!(policy.exponential_backoff);
    }
}
