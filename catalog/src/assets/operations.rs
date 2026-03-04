// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset package operations - loading, hashing, validation, and convenience accessors

use anyhow::Result;
use base64::Engine;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use super::registry::*;
use super::types::*;

impl AssetPackage {
    /// Create a new asset package from YAML specification
    pub async fn from_yaml<P: AsRef<Path>>(yaml_path: P) -> Result<Self> {
        let yaml_content = tokio::fs::read_to_string(yaml_path).await?;
        let spec: AssetSpec = serde_yaml::from_str(&yaml_content)?;

        let mut package = Self {
            spec: spec.clone(),
            content: AssetContentResolved {
                main_content: String::new(),
                file_contents: HashMap::new(),
                binary_contents: HashMap::new(),
                template_content: HashMap::new(),
                resolved_dependencies: Vec::new(),
            },
            validation: AssetValidationStatus {
                is_valid: false,
                validated_at: Utc::now(),
                errors: Vec::new(),
                warnings: Vec::new(),
                security_results: SecurityScanResults {
                    security_score: 0,
                    vulnerabilities: Vec::new(),
                    recommendations: Vec::new(),
                    scanned_at: Utc::now(),
                },
                dependency_results: DependencyValidationResults {
                    dependencies_valid: false,
                    total_dependencies: spec.spec.dependencies.len(),
                    valid_dependencies: 0,
                    invalid_dependencies: Vec::new(),
                    conflicts: Vec::new(),
                    validated_at: Utc::now(),
                },
            },
            package_hash: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            signature: None,
        };

        package.load_content().await?;
        package.compute_hash()?;

        Ok(package)
    }

    /// Load all content files referenced in the asset specification
    async fn load_content(&mut self) -> Result<()> {
        if !self.spec.spec.content.main.is_empty() {
            match tokio::fs::read_to_string(&self.spec.spec.content.main).await {
                Ok(content) => {
                    self.content.main_content = content;
                }
                Err(e) => {
                    self.validation.errors.push(ValidationError {
                        code: "MAIN_FILE_NOT_FOUND".to_string(),
                        message: format!(
                            "Main file '{}' not found: {}",
                            self.spec.spec.content.main, e
                        ),
                        file: Some(self.spec.spec.content.main.clone()),
                        line: None,
                        column: None,
                        severity: ErrorSeverity::Critical,
                    });
                }
            }
        }

        for file_path in &self.spec.spec.content.files {
            match tokio::fs::read_to_string(file_path).await {
                Ok(content) => {
                    self.content
                        .file_contents
                        .insert(file_path.clone(), content);
                }
                Err(e) => {
                    self.validation.errors.push(ValidationError {
                        code: "FILE_NOT_FOUND".to_string(),
                        message: format!("File '{file_path}' not found: {e}"),
                        file: Some(file_path.clone()),
                        line: None,
                        column: None,
                        severity: ErrorSeverity::Error,
                    });
                }
            }
        }

        if let Some(inline_content) = &self.spec.spec.content.inline {
            self.content.main_content = inline_content.clone();
        }

        for binary_asset in &self.spec.spec.content.binary {
            match base64::engine::general_purpose::STANDARD.decode(&binary_asset.content) {
                Ok(decoded) => {
                    self.content
                        .binary_contents
                        .insert(binary_asset.name.clone(), decoded);
                }
                Err(e) => {
                    self.validation.errors.push(ValidationError {
                        code: "BINARY_DECODE_ERROR".to_string(),
                        message: format!(
                            "Failed to decode binary asset '{}': {}",
                            binary_asset.name, e
                        ),
                        file: Some(binary_asset.name.clone()),
                        line: None,
                        column: None,
                        severity: ErrorSeverity::Error,
                    });
                }
            }
        }

        Ok(())
    }

    /// Compute package hash for integrity verification (BLAKE3)
    fn compute_hash(&mut self) -> Result<()> {
        let mut hasher = blake3::Hasher::new();
        let spec_json = serde_json::to_string(&self.spec)?;
        hasher.update(spec_json.as_bytes());
        hasher.update(self.content.main_content.as_bytes());

        for (path, content) in &self.content.file_contents {
            hasher.update(path.as_bytes());
            hasher.update(content.as_bytes());
        }

        for (name, content) in &self.content.binary_contents {
            hasher.update(name.as_bytes());
            hasher.update(content);
        }

        let result = hasher.finalize();
        self.package_hash = result.to_hex().to_string();

        Ok(())
    }

    /// Verify package integrity against stored hash
    pub fn verify_integrity(&self) -> Result<bool> {
        let mut temp_package = self.clone();
        temp_package.compute_hash()?;
        Ok(temp_package.package_hash == self.package_hash)
    }

    /// Get asset package unique identifier
    pub fn get_package_id(&self) -> AssetPackageId {
        Uuid::new_v5(&Uuid::NAMESPACE_OID, self.package_hash.as_bytes())
    }

    /// Check if asset package is valid for execution
    pub fn is_execution_ready(&self) -> bool {
        self.validation.is_valid
            && self
                .validation
                .errors
                .iter()
                .all(|e| !matches!(e.severity, ErrorSeverity::Critical | ErrorSeverity::Error))
            && self.validation.dependency_results.dependencies_valid
            && self.validation.security_results.security_score >= 70
    }

    /// Get human-readable summary of the asset package
    pub fn get_summary(&self) -> String {
        format!(
            "{} v{}\n  Type: {}\n  Files: {} main + {} additional + {} binary\n  Dependencies: {}\n  Valid: {}\n  Security Score: {}",
            self.spec.metadata.name,
            self.spec.metadata.version,
            self.spec.spec.asset_type,
            if self.content.main_content.is_empty() { "none" } else { "present" },
            self.content.file_contents.len(),
            self.content.binary_contents.len(),
            self.spec.spec.dependencies.len(),
            if self.validation.is_valid { "yes" } else { "no" },
            self.validation.security_results.security_score
        )
    }

    // Convenience methods for backward compatibility

    pub fn id(&self) -> &str {
        &self.spec.metadata.name
    }
    pub fn version(&self) -> &str {
        &self.spec.metadata.version
    }
    pub fn asset_type(&self) -> &str {
        &self.spec.spec.asset_type
    }
    pub fn metadata(&self) -> &PackageSpecMetadata {
        &self.spec.metadata
    }
    pub fn description(&self) -> Option<&str> {
        self.spec.metadata.description.as_deref()
    }
    pub fn tags(&self) -> &[String] {
        &self.spec.metadata.tags
    }
    pub fn author(&self) -> Option<&str> {
        self.spec.metadata.author.as_deref()
    }
    pub fn license(&self) -> Option<&str> {
        self.spec.metadata.license.as_deref()
    }
    pub fn dependencies(&self) -> &[AssetDependency] {
        &self.spec.spec.dependencies
    }
    pub fn security(&self) -> &AssetSecurity {
        &self.spec.spec.security
    }
    pub fn resources(&self) -> &AssetResources {
        &self.spec.spec.resources
    }
    pub fn execution(&self) -> &AssetExecution {
        &self.spec.spec.execution
    }
    pub fn is_valid(&self) -> bool {
        self.validation.is_valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_asset_package_creation() {
        let temp_dir = TempDir::new().expect("test");
        let yaml_path = temp_dir.path().join("test_asset.yaml");

        let yaml_content = r#"
apiVersion: "catalog.v1"
kind: "Asset"
metadata:
  name: "test-asset"
  version: "1.0.0"
  tags: ["test", "example"]
  keywords: []
  description: "Test asset for validation"
spec:
  type: "lua-script"
  content:
    main: ""
    files: []
    binary: []
    templates: []
  security:
    state_proof_required: false
    certificate_pinning: false
    hash_validation: "blake3"
    sandbox_level: "standard"
    allowed_syscalls: []
    network_access:
      enabled: false
      allowed_domains: []
      allowed_ports: []
      require_tls: true
    file_access:
      level: "read_only"
      allowed_paths: []
      denied_paths: []
      allow_temp: false
    permissions: []
  resources:
    cpu_limit: "1000m"
    memory_limit: "1Gi"
    execution_timeout: "30s"
    gpu_required: false
    hardware_requirements: []
  execution:
    delegation_strategy: "nearest_node"
    minimum_state_proof: 1
    retry_policy: "none"
    priority: "normal"
    timeout_config:
      execution: "30s"
      network: "10s"
      io: "5s"
    scheduling:
      timing: "immediate"
      allocation_strategy: "best_fit"
      node_affinity: []
      anti_affinity: []
  dependencies: []
  environment: {}
"#;

        fs::write(&yaml_path, yaml_content).expect("test");

        let package = AssetPackage::from_yaml(&yaml_path).await.expect("test");

        assert_eq!(package.spec.metadata.name, "test-asset");
        assert_eq!(package.spec.metadata.version, "1.0.0");
        assert_eq!(package.spec.spec.asset_type, "lua-script");
        assert!(!package.package_hash.is_empty());
    }

    #[test]
    fn test_package_hash_computation() {
        let mut package = AssetPackage {
            spec: AssetSpec {
                api_version: "catalog.v1".to_string(),
                kind: "Asset".to_string(),
                metadata: PackageSpecMetadata {
                    name: "test".to_string(),
                    version: "1.0.0".to_string(),
                    tags: vec!["test".to_string()],
                    description: None,
                    author: None,
                    license: None,
                    homepage: None,
                    repository: None,
                    download_count: 0,
                    featured: false,
                    keywords: vec![],
                    created: None,
                    updated: None,
                },
                spec: AssetSpecification {
                    asset_type: "test".to_string(),
                    content: AssetContent {
                        main: "".to_string(),
                        files: vec![],
                        inline: None,
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
                        memory_limit: "1Gi".to_string(),
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
                main_content: "test content".to_string(),
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
                    security_score: 85,
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
        };

        package.compute_hash().expect("test");
        assert!(!package.package_hash.is_empty());
        assert!(package.verify_integrity().expect("test"));
    }
}
